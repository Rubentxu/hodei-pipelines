//! PostgreSQL Job Repository
//!
//! Production-ready implementation for persisting and retrieving jobs.

use async_trait::async_trait;
use hodei_core::{DomainError, JobId, JobSpec, JobState, Result};
use hodei_ports::JobRepository;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// PostgreSQL Job Repository
#[derive(Debug)]
pub struct PostgreSqlJobRepository {
    pool: PgPool,
}

impl PostgreSqlJobRepository {
    /// Create a new PostgreSQL job repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Initialize database schema for jobs
    pub async fn init_schema(&self) -> Result<(), DomainError> {
        info!("Initializing job schema");

        // Create jobs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                job_id UUID PRIMARY KEY,
                spec JSONB NOT NULL,
                state TEXT NOT NULL,
                worker_id UUID NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create jobs table: {}", e)))?;

        // Create index for faster lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_jobs_state
            ON jobs(state)
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create index: {}", e)))?;

        info!("Job schema initialized successfully");
        Ok(())
    }
}

#[async_trait]
impl JobRepository for PostgreSqlJobRepository {
    async fn create_job(&self, job_spec: JobSpec) -> Result<JobId, DomainError> {
        let job_id = JobId::new();

        sqlx::query(
            r#"
            INSERT INTO jobs (job_id, spec, state, updated_at)
            VALUES ($1, $2, $3, NOW())
        "#,
        )
        .bind(job_id.as_uuid())
        .bind(serde_json::to_value(&job_spec).unwrap_or_default())
        .bind(JobState::PENDING.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create job: {}", e)))?;

        Ok(job_id)
    }

    async fn get_job(&self, job_id: &JobId) -> Result<Option<hodei_core::job::Job>, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT job_id, spec, state, worker_id
            FROM jobs
            WHERE job_id = $1
        "#,
        )
        .bind(job_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        if let Some(row) = row {
            let spec: JobSpec = serde_json::from_value(row.get("spec")).map_err(|e| {
                DomainError::Infrastructure(format!("Failed to deserialize job spec: {}", e))
            })?;
            let state = JobState::from_str(row.get("state")).unwrap_or(JobState::PENDING);

            Ok(Some(hodei_core::job::Job {
                id: job_id.clone(),
                spec,
                state,
                worker_id: row.get("worker_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_job_state(&self, job_id: &JobId, state: JobState) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET state = $1,
                updated_at = NOW()
            WHERE job_id = $2
        "#,
        )
        .bind(state.as_str())
        .bind(job_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to update job state: {}", e)))?;

        Ok(())
    }

    async fn delete_job(&self, job_id: &JobId) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            DELETE FROM jobs WHERE job_id = $1
        "#,
        )
        .bind(job_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to delete job: {}", e)))?;

        Ok(())
    }

    async fn list_jobs(
        &self,
        _filter: Option<&hodei_ports::JobFilter>,
    ) -> Result<Vec<hodei_core::job::Job>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT job_id, spec, state, worker_id, created_at, updated_at
            FROM jobs
            ORDER BY created_at DESC
            LIMIT 1000
        "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to list jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let spec: JobSpec = serde_json::from_value(row.get("spec")).map_err(|e| {
                DomainError::Infrastructure(format!("Failed to deserialize job spec: {}", e))
            })?;
            let state = JobState::from_str(row.get("state")).unwrap_or(JobState::PENDING);
            let job_id = JobId::from_uuid(row.get("job_id"));

            jobs.push(hodei_core::job::Job {
                id: job_id,
                spec,
                state,
                worker_id: row.get("worker_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(jobs)
    }
}
