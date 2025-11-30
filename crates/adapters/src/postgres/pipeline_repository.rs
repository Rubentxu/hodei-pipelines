//! PostgreSQL Pipeline Repository
//!
//! Production-ready implementation for persisting and retrieving pipelines.

use async_trait::async_trait;
use hodei_pipelines_core::{DomainError, Pipeline, PipelineId, Result};
use hodei_pipelines_ports::PipelineRepository;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::info;

/// Default timeout for pipeline steps (5 minutes in milliseconds)
#[allow(dead_code)]
const DEFAULT_TIMEOUT_MS: u64 = 300000;

/// PostgreSQL Pipeline Repository
#[derive(Debug)]
pub struct PostgreSqlPipelineRepository {
    pool: PgPool,
}

impl PostgreSqlPipelineRepository {
    /// Create a new PostgreSQL pipeline repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Initialize database schema for pipelines
    pub async fn init_schema(&self) -> Result<()> {
        info!("Initializing pipeline schema");

        // Create pipelines table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pipelines (
                pipeline_id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NULL,
                status TEXT NOT NULL,
                variables JSONB NOT NULL DEFAULT '{}'::jsonb,
                workflow_definition JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                tenant_id TEXT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create pipelines table: {}", e))
        })?;

        // Create pipeline_steps table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pipeline_steps (
                step_id UUID PRIMARY KEY,
                pipeline_id UUID NOT NULL REFERENCES pipelines(pipeline_id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                job_spec JSONB NOT NULL,
                timeout_ms BIGINT NOT NULL DEFAULT 300000,
                depends_on JSONB NOT NULL DEFAULT '[]'::jsonb,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create pipeline_steps table: {}", e))
        })?;

        // Create indexes
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pipelines_name
            ON pipelines(name)
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create index: {}", e)))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pipeline_steps_pipeline_id
            ON pipeline_steps(pipeline_id)
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create index: {}", e)))?;

        info!("Pipeline schema initialized successfully");
        Ok(())
    }

    /// Deserialize a PipelineStep from a SQL row
    ///
    /// This method can handle both regular queries and JOIN queries with column aliases
    fn deserialize_pipeline_step_from_row(
        &self,
        step_row: &sqlx::postgres::PgRow,
    ) -> Result<hodei_pipelines_core::pipeline::PipelineStep> {
        // Handle both "name" (regular query) and "step_name" (JOIN query) column names
        let step_name = step_row
            .try_get::<String, _>("step_name")
            .unwrap_or_else(|_| step_row.get::<String, _>("name"));

        let job_spec: hodei_pipelines_core::job::JobSpec =
            serde_json::from_value(step_row.get("job_spec")).map_err(|e| {
                DomainError::Validation(format!("Failed to deserialize job spec: {}", e))
            })?;

        let depends_on: Vec<hodei_pipelines_core::pipeline::PipelineStepId> =
            serde_json::from_value(step_row.get("depends_on")).map_err(|e| {
                DomainError::Validation(format!("Failed to deserialize dependencies: {}", e))
            })?;

        Ok(hodei_pipelines_core::pipeline::PipelineStep {
            id: hodei_pipelines_core::pipeline::PipelineStepId::from_uuid(step_row.get("step_id")),
            name: step_name,
            job_spec,
            depends_on,
            timeout_ms: step_row.get::<i64, _>("timeout_ms") as u64,
        })
    }

    /// Deserialize a Pipeline from SQL rows (main query + steps)
    fn deserialize_pipeline_from_rows(
        &self,
        pipeline_row: &sqlx::postgres::PgRow,
        step_rows: Vec<sqlx::postgres::PgRow>,
    ) -> Result<Pipeline> {
        let steps = step_rows
            .into_iter()
            .map(|row| self.deserialize_pipeline_step_from_row(&row))
            .collect::<Result<Vec<_>>>()?;

        let variables: serde_json::Value = pipeline_row.get("variables");
        let variables =
            serde_json::from_value::<HashMap<String, String>>(variables).map_err(|e| {
                DomainError::Validation(format!("Failed to deserialize variables: {}", e))
            })?;

        Ok(Pipeline {
            id: PipelineId::from_uuid(pipeline_row.get("pipeline_id")),
            name: pipeline_row.get("name"),
            description: pipeline_row.get("description"),
            status: hodei_pipelines_core::pipeline::PipelineStatus::from_str(
                pipeline_row.get("status"),
            )
            .map_err(|_| DomainError::Validation("Invalid pipeline status".to_string()))?,
            steps,
            variables,
            created_at: pipeline_row.get("created_at"),
            updated_at: pipeline_row.get("updated_at"),
            tenant_id: pipeline_row.get("tenant_id"),
            workflow_definition: pipeline_row.get("workflow_definition"),
        })
    }
}

#[async_trait]
impl PipelineRepository for PostgreSqlPipelineRepository {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<()> {
        // Save or update pipeline without transaction
        sqlx::query(
            r#"
            INSERT INTO pipelines (
                pipeline_id, name, description, status, variables,
                workflow_definition, tenant_id, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (pipeline_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                status = EXCLUDED.status,
                variables = EXCLUDED.variables,
                workflow_definition = EXCLUDED.workflow_definition,
                tenant_id = EXCLUDED.tenant_id,
                updated_at = NOW()
        "#,
        )
        .bind(pipeline.id.as_uuid())
        .bind(&pipeline.name)
        .bind(pipeline.description.as_deref())
        .bind(pipeline.status.as_str())
        .bind(serde_json::to_value(&pipeline.variables).unwrap_or_default())
        .bind(&pipeline.workflow_definition)
        .bind(pipeline.tenant_id.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to save pipeline: {}", e)))?;

        // Delete existing steps
        sqlx::query(
            r#"
            DELETE FROM pipeline_steps WHERE pipeline_id = $1
        "#,
        )
        .bind(pipeline.id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to delete existing pipeline steps: {}", e))
        })?;

        // Insert all pipeline steps one by one
        for step in &pipeline.steps {
            sqlx::query(
                r#"
                INSERT INTO pipeline_steps (
                    step_id, pipeline_id, name, job_spec, timeout_ms, depends_on, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
            "#,
            )
            .bind(step.id.as_uuid())
            .bind(pipeline.id.as_uuid())
            .bind(&step.name)
            .bind(serde_json::to_value(&step.job_spec).unwrap_or_default())
            .bind(step.timeout_ms as i64)
            .bind(serde_json::to_value(&step.depends_on).unwrap_or_default())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!("Failed to save pipeline step: {}", e))
            })?;
        }

        Ok(())
    }

    async fn get_pipeline(&self, pipeline_id: &PipelineId) -> Result<Option<Pipeline>> {
        let pipeline_row = sqlx::query(r#"
            SELECT pipeline_id, name, description, status, variables, workflow_definition, created_at, updated_at, tenant_id
            FROM pipelines
            WHERE pipeline_id = $1
        "#)
        .bind(pipeline_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get pipeline: {}", e)))?;

        if let Some(row) = pipeline_row {
            // Get pipeline steps
            let step_rows = sqlx::query(
                r#"
                SELECT step_id, name, job_spec, timeout_ms, depends_on
                FROM pipeline_steps
                WHERE pipeline_id = $1
                ORDER BY created_at
            "#,
            )
            .bind(pipeline_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!("Failed to get pipeline steps: {}", e))
            })?;

            let pipeline = self.deserialize_pipeline_from_rows(&row, step_rows)?;
            Ok(Some(pipeline))
        } else {
            Ok(None)
        }
    }

    async fn get_all_pipelines(&self) -> Result<Vec<Pipeline>> {
        // Get all pipelines with their steps in a single query using LEFT JOIN
        let pipeline_rows = sqlx::query(
            r#"
            SELECT p.pipeline_id, p.name, p.description, p.status, p.variables,
                   p.workflow_definition, p.created_at, p.updated_at, p.tenant_id,
                   s.step_id, s.name as step_name, s.job_spec, s.timeout_ms, s.depends_on
            FROM pipelines p
            LEFT JOIN pipeline_steps s ON p.pipeline_id = s.pipeline_id
            ORDER BY p.created_at DESC, s.created_at
        "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get all pipelines: {}", e)))?;

        // Group pipeline steps by pipeline_id
        let mut pipelines_map: std::collections::HashMap<
            PipelineId,
            (sqlx::postgres::PgRow, Vec<sqlx::postgres::PgRow>),
        > = std::collections::HashMap::new();

        for row in pipeline_rows {
            let pipeline_id = PipelineId::from_uuid(row.get("pipeline_id"));
            if let Some((_, step_rows)) = pipelines_map.get_mut(&pipeline_id) {
                step_rows.push(row);
            } else {
                pipelines_map.insert(pipeline_id, (row, Vec::new()));
            }
        }

        // Deserialize each pipeline
        let mut pipelines = Vec::new();
        for (_, (pipeline_row, step_rows)) in pipelines_map {
            let step_rows_sorted = step_rows;
            let pipeline = self.deserialize_pipeline_from_rows(&pipeline_row, step_rows_sorted)?;
            pipelines.push(pipeline);
        }

        Ok(pipelines)
    }

    async fn delete_pipeline(&self, pipeline_id: &PipelineId) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM pipelines WHERE pipeline_id = $1
        "#,
        )
        .bind(pipeline_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to delete pipeline: {}", e)))?;

        Ok(())
    }
}
