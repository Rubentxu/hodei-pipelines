//! PostgreSQL Repository Implementations
//!
//! Production-grade persistence using PostgreSQL with async SQLx driver.

use async_trait::async_trait;
use futures::future::join_all;
use hodei_core::{Job, JobId, Pipeline, PipelineId, Worker, pipeline::PipelineStepId};
use hodei_core::{WorkerCapabilities, WorkerId};
use hodei_ports::{
    JobRepository, JobRepositoryError, PipelineRepository, PipelineRepositoryError,
    WorkerRepository, WorkerRepositoryError,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Workflow Definition structure for JSON deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowDefinitionJson {
    steps: Option<Vec<WorkflowStepJson>>,
    variables: Option<HashMap<String, String>>,
}

/// Workflow Step structure for JSON deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowStepJson {
    name: String,
    job_spec: hodei_core::job::JobSpec,
    depends_on: Option<Vec<String>>,
    timeout_ms: Option<u64>,
}

/// PostgreSQL-backed job repository
pub struct PostgreSqlJobRepository {
    pool: Arc<Pool<Postgres>>,
}

impl PostgreSqlJobRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Initialize database schema with performance indexes
    pub async fn init(&self) -> Result<(), JobRepositoryError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                spec JSONB NOT NULL,
                state TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                started_at TIMESTAMPTZ,
                completed_at TIMESTAMPTZ,
                tenant_id TEXT,
                result JSONB
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| JobRepositoryError::Database(format!("Failed to create jobs table: {}", e)))?;

        // Performance indexes for common query patterns
        let index_queries = vec![
            // Single column index for simple lookups
            "CREATE INDEX IF NOT EXISTS idx_jobs_state ON jobs(state)",
            // Composite index for pending/running jobs ordered by priority (created_at)
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_jobs_state_created ON jobs(state, created_at)",
            // Composite index for tenant isolation
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_jobs_tenant_state ON jobs(tenant_id, state) WHERE tenant_id IS NOT NULL",
            // Composite index for job completion tracking
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_jobs_state_completed ON jobs(state, completed_at) WHERE completed_at IS NOT NULL",
            // Full-text search GIN index for efficient text search across name, description, and command
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_jobs_fulltext ON jobs USING GIN (to_tsvector('english', name || ' ' || COALESCE(description, '') || ' ' || (spec->>'command')))",
        ];

        for query in index_queries {
            sqlx::query(query).execute(&*self.pool).await.map_err(|e| {
                JobRepositoryError::Database(format!("Failed to create index: {}", e))
            })?;
        }

        info!("PostgreSQL job repository initialized with performance indexes");
        Ok(())
    }
}

#[async_trait]
impl JobRepository for PostgreSqlJobRepository {
    async fn save_job(&self, job: &Job) -> Result<(), JobRepositoryError> {
        let query = r#"
            INSERT INTO jobs (
                id, name, description, spec, state, created_at, updated_at,
                started_at, completed_at, tenant_id, result
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                spec = EXCLUDED.spec,
                state = EXCLUDED.state,
                updated_at = EXCLUDED.updated_at,
                started_at = EXCLUDED.started_at,
                completed_at = EXCLUDED.completed_at,
                result = EXCLUDED.result
        "#;

        let description: Option<String> = job.description.as_deref().map(|s| s.to_string());
        let tenant_id: Option<&str> = job.tenant_id.as_deref();

        sqlx::query(query)
            .bind(&job.id)
            .bind(&job.name)
            .bind(description)
            .bind(serde_json::to_value(&job.spec).ok())
            .bind(&job.state)
            .bind(job.created_at)
            .bind(job.updated_at)
            .bind(job.started_at)
            .bind(job.completed_at)
            .bind(tenant_id)
            .bind(serde_json::to_value(&job.result).ok())
            .execute(&*self.pool)
            .await
            .map_err(|e| JobRepositoryError::Database(format!("Failed to save job: {}", e)))?;

        info!("Saved job to PostgreSQL: {}", job.id);
        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<Job>, JobRepositoryError> {
        match sqlx::query("SELECT * FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
        {
            Ok(Some(row)) => {
                let result: Option<serde_json::Value> = row.get("result");
                let state_str: String = row.get("state");
                let spec_json: Option<serde_json::Value> = row.get("spec");
                let name: String = row.get("name");
                let description: Option<String> = row.get("description");
                let tenant_id: Option<String> = row.get("tenant_id");

                let job = Job {
                    id: row.get("id"),
                    name,
                    description,
                    spec: spec_json
                        .and_then(|v| serde_json::from_value::<hodei_core::JobSpec>(v).ok())
                        .unwrap_or_else(|| hodei_core::JobSpec {
                            name: "unknown".to_string(),
                            image: "unknown".to_string(),
                            command: vec![],
                            resources: hodei_core::ResourceQuota::default(),
                            timeout_ms: 30000,
                            retries: 3,
                            env: std::collections::HashMap::new(),
                            secret_refs: vec![],
                        }),
                    state: hodei_core::JobState::new(state_str)
                        .map_err(|e| JobRepositoryError::Validation(e.to_string()))?,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    tenant_id,
                    result: result.unwrap_or_default(),
                };
                Ok(Some(job))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JobRepositoryError::Database(format!(
                "Failed to fetch job: {}",
                e
            ))),
        }
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
        let rows =
            sqlx::query("SELECT * FROM jobs WHERE state = 'PENDING' ORDER BY created_at DESC")
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to fetch pending jobs: {}", e))
                })?;

        debug!("Fetched {} pending jobs from database", rows.len());

        // Use parallel processing for row conversion (performance optimization)
        let jobs: Vec<Job> = join_all(rows.into_iter().map(|row| self.row_to_job(row)))
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        Ok(jobs)
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
        let rows = sqlx::query("SELECT * FROM jobs WHERE state = 'RUNNING'")
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                JobRepositoryError::Database(format!("Failed to fetch running jobs: {}", e))
            })?;

        let jobs: Vec<Job> = rows
            .into_iter()
            .map(|row| {
                let result: Option<serde_json::Value> = row.get("result");
                let state_str: String = row.get("state");
                let spec_json: Option<serde_json::Value> = row.get("spec");
                let name: String = row.get("name");
                let description: Option<String> = row.get("description");
                let tenant_id: Option<String> = row.get("tenant_id");

                Job {
                    id: row.get("id"),
                    name,
                    description,
                    spec: spec_json
                        .and_then(|v| serde_json::from_value::<hodei_core::JobSpec>(v).ok())
                        .unwrap_or_else(|| hodei_core::JobSpec {
                            name: "unknown".to_string(),
                            image: "unknown".to_string(),
                            command: vec![],
                            resources: hodei_core::ResourceQuota::default(),
                            timeout_ms: 30000,
                            retries: 3,
                            env: std::collections::HashMap::new(),
                            secret_refs: vec![],
                        }),
                    state: hodei_core::JobState::new(state_str).unwrap_or_else(|_| {
                        hodei_core::JobState::new("RUNNING".to_string()).unwrap()
                    }),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    tenant_id,
                    result: result.unwrap_or_default(),
                }
            })
            .collect();

        Ok(jobs)
    }

    async fn delete_job(&self, id: &JobId) -> Result<(), JobRepositoryError> {
        let query = "DELETE FROM jobs WHERE id = $1";

        sqlx::query(query)
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| JobRepositoryError::Database(format!("Failed to delete job: {}", e)))?;

        Ok(())
    }

    async fn compare_and_swap_status(
        &self,
        id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool, JobRepositoryError> {
        let query = r#"
            UPDATE jobs
            SET state = $1, updated_at = $2
            WHERE id = $3 AND state = $4
            RETURNING id
        "#;

        let updated_at = chrono::Utc::now();

        let result = sqlx::query(query)
            .bind(new_state)
            .bind(updated_at)
            .bind(id)
            .bind(expected_state)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| {
                JobRepositoryError::Database(format!(
                    "Failed to compare and swap job status: {}",
                    e
                ))
            })?;

        Ok(result.is_some())
    }

    async fn assign_worker(
        &self,
        _job_id: &JobId,
        _worker_id: &WorkerId,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement worker assignment in postgres
        Ok(())
    }

    async fn set_job_start_time(
        &self,
        _job_id: &JobId,
        _start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement start time tracking in postgres
        Ok(())
    }

    async fn set_job_finish_time(
        &self,
        _job_id: &JobId,
        _finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement finish time tracking in postgres
        Ok(())
    }

    async fn set_job_duration(
        &self,
        _job_id: &JobId,
        _duration_ms: i64,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement duration tracking in postgres
        Ok(())
    }
}

impl PostgreSqlJobRepository {
    /// Helper method to convert a database row to Job (avoids code duplication)
    async fn row_to_job(&self, row: sqlx::postgres::PgRow) -> Result<Job, JobRepositoryError> {
        let result: Option<serde_json::Value> = row.get("result");
        let state_str: String = row.get("state");
        let spec_json: Option<serde_json::Value> = row.get("spec");
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let tenant_id: Option<String> = row.get("tenant_id");

        Ok(Job {
            id: row.get("id"),
            name,
            description,
            spec: spec_json
                .and_then(|v| serde_json::from_value::<hodei_core::JobSpec>(v).ok())
                .unwrap_or_else(|| hodei_core::JobSpec {
                    name: "unknown".to_string(),
                    image: "unknown".to_string(),
                    command: vec![],
                    resources: hodei_core::ResourceQuota::default(),
                    timeout_ms: 30000,
                    retries: 3,
                    env: std::collections::HashMap::new(),
                    secret_refs: vec![],
                }),
            state: hodei_core::JobState::new(state_str)
                .map_err(|e| JobRepositoryError::Validation(e.to_string()))?,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            tenant_id,
            result: result.unwrap_or_default(),
        })
    }
}

/// PostgreSQL-backed worker repository
pub struct PostgreSqlWorkerRepository {
    pool: Arc<Pool<Postgres>>,
}

impl PostgreSqlWorkerRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Initialize database schema
    pub async fn init(&self) -> Result<(), WorkerRepositoryError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workers (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                last_heartbeat TIMESTAMPTZ,
                tenant_id TEXT,
                metadata JSONB,
                capabilities JSONB
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to create workers table: {}", e))
        })?;

        // Add last_heartbeat column if it doesn't exist (for existing databases)
        sqlx::query("ALTER TABLE workers ADD COLUMN IF NOT EXISTS last_heartbeat TIMESTAMPTZ")
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!(
                    "Failed to add last_heartbeat column: {}",
                    e
                ))
            })?;

        // Add capabilities column if it doesn't exist (for existing databases)
        sqlx::query("ALTER TABLE workers ADD COLUMN IF NOT EXISTS capabilities JSONB")
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to add capabilities column: {}", e))
            })?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS worker_capabilities (
                id SERIAL PRIMARY KEY,
                worker_id UUID NOT NULL REFERENCES workers(id) ON DELETE CASCADE,
                capability_type TEXT NOT NULL,
                capability_value TEXT NOT NULL
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            WorkerRepositoryError::Database(format!(
                "Failed to create worker capabilities table: {}",
                e
            ))
        })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_workers_status ON workers(status)")
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!(
                    "Failed to create workers status index: {}",
                    e
                ))
            })?;

        info!("PostgreSQL worker repository initialized");
        Ok(())
    }
}

#[async_trait]
impl WorkerRepository for PostgreSqlWorkerRepository {
    async fn save_worker(&self, worker: &Worker) -> Result<(), WorkerRepositoryError> {
        // Insert/update worker
        let query = r#"
            INSERT INTO workers (
                id, name, status, created_at, updated_at, tenant_id, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at,
                metadata = EXCLUDED.metadata
        "#;

        sqlx::query(query)
            .bind(&worker.id)
            .bind(&worker.name)
            .bind(&worker.status.status)
            .bind(worker.created_at)
            .bind(worker.updated_at)
            .bind(worker.tenant_id.as_deref())
            .bind(serde_json::to_value(&worker.metadata).ok())
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to save worker: {}", e))
            })?;

        // Delete old capabilities
        sqlx::query("DELETE FROM worker_capabilities WHERE worker_id = $1")
            .bind(&worker.id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!(
                    "Failed to delete old worker capabilities: {}",
                    e
                ))
            })?;

        // Store capabilities as JSONB
        let capabilities_json = serde_json::to_string(&worker.capabilities).map_err(|e| {
            WorkerRepositoryError::Serialization(format!("Failed to serialize capabilities: {}", e))
        })?;

        sqlx::query("UPDATE workers SET capabilities = $1 WHERE id = $2")
            .bind(&capabilities_json)
            .bind(&worker.id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!(
                    "Failed to update worker capabilities: {}",
                    e
                ))
            })?;

        info!("Saved worker to PostgreSQL: {}", worker.id);
        Ok(())
    }

    async fn get_worker(&self, id: &WorkerId) -> Result<Option<Worker>, WorkerRepositoryError> {
        match sqlx::query("SELECT * FROM workers WHERE id = $1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
        {
            Ok(Some(row)) => {
                let worker = self.row_to_worker(row).await?;
                Ok(Some(worker))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(WorkerRepositoryError::Database(format!(
                "Failed to fetch worker: {}",
                e
            ))),
        }
    }

    async fn get_all_workers(&self) -> Result<Vec<Worker>, WorkerRepositoryError> {
        let query = "SELECT * FROM workers";

        let rows = sqlx::query(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to fetch workers: {}", e))
            })?;

        let workers: Vec<Worker> =
            futures::future::join_all(rows.into_iter().map(|row| self.row_to_worker(row)))
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;

        Ok(workers)
    }

    async fn delete_worker(&self, id: &WorkerId) -> Result<(), WorkerRepositoryError> {
        let query = "DELETE FROM workers WHERE id = $1";

        sqlx::query(query)
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to delete worker: {}", e))
            })?;

        Ok(())
    }

    async fn update_last_seen(&self, id: &WorkerId) -> Result<(), WorkerRepositoryError> {
        let now = chrono::Utc::now();
        let query = "UPDATE workers SET last_heartbeat = $1, updated_at = $2 WHERE id = $3";

        sqlx::query(query)
            .bind(now)
            .bind(now)
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to update last_seen: {}", e))
            })?;

        info!("Updated last_seen for worker: {}", id);
        Ok(())
    }

    async fn find_stale_workers(
        &self,
        threshold_duration: std::time::Duration,
    ) -> Result<Vec<Worker>, WorkerRepositoryError> {
        let threshold_time =
            chrono::Utc::now() - chrono::Duration::from_std(threshold_duration).unwrap_or_default();

        let query =
            "SELECT * FROM workers WHERE last_heartbeat IS NOT NULL AND last_heartbeat < $1";

        let rows = sqlx::query(query)
            .bind(threshold_time)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to find stale workers: {}", e))
            })?;

        let workers: Vec<Worker> =
            futures::future::join_all(rows.into_iter().map(|row| self.row_to_worker(row)))
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;

        info!(
            "Found {} stale workers (threshold: {} seconds)",
            workers.len(),
            threshold_duration.as_secs()
        );

        Ok(workers)
    }
}

impl PostgreSqlWorkerRepository {
    async fn row_to_worker(
        &self,
        row: sqlx::postgres::PgRow,
    ) -> Result<Worker, WorkerRepositoryError> {
        let id: WorkerId = row.get("id");

        // Fetch capabilities as JSONB
        let capabilities_json: Option<String> = row.get("capabilities");
        let capabilities = match capabilities_json {
            Some(json_str) => {
                serde_json::from_str::<WorkerCapabilities>(&json_str).map_err(|e| {
                    WorkerRepositoryError::Serialization(format!(
                        "Failed to deserialize capabilities: {}",
                        e
                    ))
                })?
            }
            None => WorkerCapabilities::new(1, 1024), // Default capabilities
        };

        // Fetch last_heartbeat from database (can be NULL for old records)
        let last_heartbeat: Option<chrono::DateTime<chrono::Utc>> = row.get("last_heartbeat");

        let metadata: Option<serde_json::Value> = row.get("metadata");
        let status_string: String = row.get("status");
        let current_jobs_uuids: Vec<Uuid> = row.get("current_jobs");

        let worker_status = hodei_core::WorkerStatus {
            worker_id: id.clone(),
            status: status_string,
            current_jobs: current_jobs_uuids.into_iter().map(Into::into).collect(),
            last_heartbeat: last_heartbeat.unwrap_or_else(|| chrono::Utc::now()).into(),
        };

        Ok(Worker {
            id,
            name: row.get("name"),
            status: worker_status,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            tenant_id: row.get("tenant_id"),
            capabilities,
            metadata: metadata
                .and_then(|v| {
                    serde_json::from_value::<Option<std::collections::HashMap<String, String>>>(v)
                        .ok()
                })
                .flatten()
                .unwrap_or_default(),
            current_jobs: Vec::new(),
            last_heartbeat: last_heartbeat.unwrap_or_else(|| chrono::Utc::now()),
        })
    }
}

/// PostgreSQL-backed pipeline repository
pub struct PostgreSqlPipelineRepository {
    pool: Arc<Pool<Postgres>>,
}

impl PostgreSqlPipelineRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Initialize database schema
    pub async fn init(&self) -> Result<(), PipelineRepositoryError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pipelines (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                tenant_id TEXT,
                workflow_definition JSONB
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to create pipelines table: {}", e))
        })?;

        info!("PostgreSQL pipeline repository initialized");
        Ok(())
    }
}

#[async_trait]
impl PipelineRepository for PostgreSqlPipelineRepository {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<(), PipelineRepositoryError> {
        // Ensure workflow_definition is synchronized with steps and variables
        let workflow_json = WorkflowDefinitionJson {
            steps: Some(
                pipeline
                    .steps
                    .iter()
                    .map(|step| WorkflowStepJson {
                        name: step.name.clone(),
                        job_spec: step.job_spec.clone(),
                        depends_on: Some(step.depends_on.iter().map(|id| id.to_string()).collect()),
                        timeout_ms: Some(step.timeout_ms),
                    })
                    .collect(),
            ),
            variables: Some(pipeline.variables.clone()),
        };

        let query = r#"
            INSERT INTO pipelines (
                id, name, description, created_at, updated_at, tenant_id, workflow_definition
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                updated_at = EXCLUDED.updated_at,
                workflow_definition = EXCLUDED.workflow_definition
        "#;

        sqlx::query(query)
            .bind(&pipeline.id)
            .bind(&pipeline.name)
            .bind(pipeline.description.as_deref())
            .bind(pipeline.created_at)
            .bind(pipeline.updated_at)
            .bind(pipeline.tenant_id.as_deref())
            .bind(serde_json::to_value(&workflow_json).ok())
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to save pipeline: {}", e))
            })?;

        info!("Saved pipeline to PostgreSQL: {}", pipeline.id);
        Ok(())
    }

    async fn get_pipeline(
        &self,
        id: &PipelineId,
    ) -> Result<Option<Pipeline>, PipelineRepositoryError> {
        let query = "SELECT * FROM pipelines WHERE id = $1";

        match sqlx::query(query)
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
        {
            Ok(Some(row)) => {
                let workflow_def: Option<serde_json::Value> = row.get("workflow_definition");

                // Deserialize workflow_definition to extract steps and variables
                let (steps, variables) = if let Some(workflow_json) = &workflow_def {
                    match serde_json::from_value::<WorkflowDefinitionJson>(workflow_json.clone()) {
                        Ok(workflow) => {
                            let steps = workflow.steps.map_or(vec![], |steps_json| {
                                steps_json
                                    .into_iter()
                                    .map(|step_json| hodei_core::pipeline::PipelineStep {
                                        id: PipelineStepId::new(),
                                        name: step_json.name,
                                        job_spec: step_json.job_spec,
                                        depends_on: step_json
                                            .depends_on
                                            .unwrap_or_default()
                                            .into_iter()
                                            .map(|s| {
                                                PipelineStepId::from_uuid(
                                                    s.parse().unwrap_or_else(|_| Uuid::new_v4()),
                                                )
                                            })
                                            .collect(),
                                        timeout_ms: step_json.timeout_ms.unwrap_or(300000),
                                    })
                                    .collect()
                            });

                            let variables = workflow.variables.unwrap_or_default();
                            (steps, variables)
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to deserialize workflow_definition for pipeline {}: {}. Using empty steps and variables.",
                                row.get::<String, &str>("id"),
                                e
                            );
                            (vec![], HashMap::new())
                        }
                    }
                } else {
                    (vec![], HashMap::new())
                };

                let pipeline = Pipeline {
                    id: row.get("id"),
                    name: row.get("name"),
                    description: row.get("description"),
                    steps,
                    status: hodei_core::PipelineStatus::PENDING,
                    variables,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    tenant_id: row.get("tenant_id"),
                    workflow_definition: workflow_def.unwrap_or_default(),
                };
                Ok(Some(pipeline))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(PipelineRepositoryError::Database(format!(
                "Failed to fetch pipeline: {}",
                e
            ))),
        }
    }

    async fn delete_pipeline(&self, id: &PipelineId) -> Result<(), PipelineRepositoryError> {
        let query = "DELETE FROM pipelines WHERE id = $1";

        sqlx::query(query)
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to delete pipeline: {}", e))
            })?;

        Ok(())
    }

    async fn get_all_pipelines(&self) -> Result<Vec<Pipeline>, PipelineRepositoryError> {
        let query = "SELECT * FROM pipelines ORDER BY created_at DESC";

        let rows = sqlx::query(query)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to fetch all pipelines: {}", e))
            })?;

        let mut pipelines = Vec::new();

        for row in rows {
            let workflow_def: Option<serde_json::Value> = row.get("workflow_definition");

            // Deserialize workflow_definition to extract steps and variables
            let (steps, variables) = if let Some(workflow_json) = &workflow_def {
                match serde_json::from_value::<WorkflowDefinitionJson>(workflow_json.clone()) {
                    Ok(workflow) => {
                        let steps = workflow.steps.map_or(vec![], |steps_json| {
                            steps_json
                                .into_iter()
                                .map(|step_json| hodei_core::pipeline::PipelineStep {
                                    id: PipelineStepId::new(),
                                    name: step_json.name,
                                    job_spec: step_json.job_spec,
                                    depends_on: step_json
                                        .depends_on
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|s| {
                                            PipelineStepId::from_uuid(
                                                s.parse().unwrap_or_else(|_| Uuid::new_v4()),
                                            )
                                        })
                                        .collect(),
                                    timeout_ms: step_json.timeout_ms.unwrap_or(300000),
                                })
                                .collect()
                        });

                        let variables = workflow.variables.unwrap_or_default();
                        (steps, variables)
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to deserialize workflow_definition for pipeline {}: {}. Using empty steps and variables.",
                            row.get::<String, &str>("id"),
                            e
                        );
                        (vec![], HashMap::new())
                    }
                }
            } else {
                (vec![], HashMap::new())
            };

            // Parse status from string to PipelineStatus enum
            let status_str: String = row.get("status");
            let status = hodei_core::PipelineStatus::from_str(&status_str)
                .unwrap_or_else(|_| hodei_core::PipelineStatus::PENDING);

            let pipeline = Pipeline {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
                steps,
                status,
                variables,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                tenant_id: row.get("tenant_id"),
                workflow_definition: workflow_def.unwrap_or_else(|| serde_json::Value::Null),
            };

            pipelines.push(pipeline);
        }

        info!("Retrieved {} pipelines from PostgreSQL", pipelines.len());
        Ok(pipelines)
    }
}
