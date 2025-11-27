//! PostgreSQL Pipeline Execution Repository
//!
//! Production-ready implementation for persisting and retrieving pipeline executions.

use async_trait::async_trait;
use hodei_core::{
    DomainError, Result,
    pipeline::PipelineStepId,
    pipeline_execution::{
        ExecutionId, ExecutionStatus, PipelineExecution, StepExecution, StepExecutionId,
        StepExecutionStatus,
    },
};
use hodei_ports::PipelineExecutionRepository;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// PostgreSQL Pipeline Execution Repository
#[derive(Debug)]
pub struct PostgreSqlPipelineExecutionRepository {
    pool: PgPool,
}

impl PostgreSqlPipelineExecutionRepository {
    /// Create a new PostgreSQL pipeline execution repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Initialize database schema for pipeline executions
    pub async fn init_schema(&self) -> Result<()> {
        info!("Initializing pipeline execution schema");

        // Create pipeline_executions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pipeline_executions (
                execution_id UUID PRIMARY KEY,
                pipeline_id UUID NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ NULL,
                variables JSONB NOT NULL DEFAULT '{}'::jsonb,
                tenant_id TEXT NULL,
                correlation_id TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!(
                "Failed to create pipeline_executions table: {}",
                e
            ))
        })?;

        // Create step_executions table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS step_executions (
                step_execution_id UUID PRIMARY KEY,
                execution_id UUID NOT NULL REFERENCES pipeline_executions(execution_id) ON DELETE CASCADE,
                step_id UUID NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMPTZ NULL,
                completed_at TIMESTAMPTZ NULL,
                retry_count INT NOT NULL DEFAULT 0,
                error_message TEXT NULL,
                logs TEXT NOT NULL DEFAULT '',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        "#)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create step_executions table: {}", e)))?;

        // Create index for faster lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_pipeline_executions_pipeline_id
            ON pipeline_executions(pipeline_id)
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create index: {}", e)))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_step_executions_execution_id
            ON step_executions(execution_id)
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create index: {}", e)))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_step_executions_step_id
            ON step_executions(step_id)
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create index: {}", e)))?;

        info!("Pipeline execution schema initialized successfully");
        Ok(())
    }
}

#[async_trait]
impl PipelineExecutionRepository for PostgreSqlPipelineExecutionRepository {
    async fn save_execution(&self, execution: &PipelineExecution) -> Result<()> {
        debug!("Saving pipeline execution: {}", execution.id);

        // Save pipeline execution
        sqlx::query(
            r#"
            INSERT INTO pipeline_executions (
                execution_id, pipeline_id, status, started_at, completed_at,
                variables, tenant_id, correlation_id, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT (execution_id) DO UPDATE SET
                status = EXCLUDED.status,
                completed_at = EXCLUDED.completed_at,
                variables = EXCLUDED.variables,
                updated_at = NOW()
        "#,
        )
        .bind(execution.id.as_uuid())
        .bind(execution.pipeline_id.as_uuid())
        .bind(execution.status.as_str())
        .bind(execution.started_at)
        .bind(execution.completed_at)
        .bind(serde_json::to_value(&execution.variables).unwrap_or_default())
        .bind(execution.tenant_id.as_deref())
        .bind(execution.correlation_id.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to save pipeline execution: {}", e))
        })?;

        // Save step executions
        for step in &execution.steps {
            sqlx::query(
                r#"
                INSERT INTO step_executions (
                    step_execution_id, execution_id, step_id, status,
                    started_at, completed_at, retry_count, error_message, logs, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
                ON CONFLICT (step_execution_id) DO UPDATE SET
                    status = EXCLUDED.status,
                    started_at = EXCLUDED.started_at,
                    completed_at = EXCLUDED.completed_at,
                    retry_count = EXCLUDED.retry_count,
                    error_message = EXCLUDED.error_message,
                    logs = EXCLUDED.logs,
                    updated_at = NOW()
            "#,
            )
            .bind(step.step_execution_id.as_uuid())
            .bind(execution.id.as_uuid())
            .bind(step.step_id.as_uuid())
            .bind(step.status.as_str())
            .bind(step.started_at)
            .bind(step.completed_at)
            .bind(step.retry_count as i32)
            .bind(step.error_message.as_deref())
            .bind(step.logs.join("\n"))
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!("Failed to save step execution: {}", e))
            })?;
        }

        info!("Pipeline execution saved successfully: {}", execution.id);
        Ok(())
    }

    async fn get_execution(&self, execution_id: &ExecutionId) -> Result<Option<PipelineExecution>> {
        debug!("Getting pipeline execution: {}", execution_id);

        // Get pipeline execution
        let exec_row = sqlx::query(
            r#"
            SELECT execution_id, pipeline_id, status, started_at, completed_at,
                   variables, tenant_id, correlation_id
            FROM pipeline_executions
            WHERE execution_id = $1
        "#,
        )
        .bind(execution_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to get pipeline execution: {}", e))
        })?;

        if let Some(row) = exec_row {
            // Get step executions
            let step_rows = sqlx::query(
                r#"
                SELECT step_execution_id, step_id, status, started_at, completed_at,
                       retry_count, error_message, logs
                FROM step_executions
                WHERE execution_id = $1
                ORDER BY created_at
            "#,
            )
            .bind(execution_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!("Failed to get step executions: {}", e))
            })?;

            let steps = step_rows
                .into_iter()
                .map(|row| {
                    let logs_str: String = row.get("logs");
                    Ok(StepExecution {
                        step_execution_id: StepExecutionId::from_uuid(row.get("step_execution_id")),
                        step_id: PipelineStepId::from_uuid(row.get("step_id")),
                        status: StepExecutionStatus::from_str(row.get("status"))
                            .unwrap_or(StepExecutionStatus::PENDING),
                        started_at: row.get("started_at"),
                        completed_at: row.get("completed_at"),
                        retry_count: row.get::<i32, _>("retry_count") as u8,
                        error_message: row.get("error_message"),
                        logs: if logs_str.is_empty() {
                            Vec::new()
                        } else {
                            logs_str.split('\n').map(|s| s.to_string()).collect()
                        },
                    })
                })
                .collect::<Result<Vec<_>, DomainError>>()?;

            let variables: HashMap<String, serde_json::Value> = row.get("variables");
            let execution = PipelineExecution {
                id: ExecutionId::from_uuid(row.get("execution_id")),
                pipeline_id: hodei_core::PipelineId::from_uuid(row.get("pipeline_id")),
                status: ExecutionStatus::from_str(row.get("status"))
                    .unwrap_or(ExecutionStatus::PENDING),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                steps,
                variables: variables
                    .into_iter()
                    .map(|(k, v)| (k, v.as_str().unwrap_or_default().to_string()))
                    .collect(),
                tenant_id: row.get("tenant_id"),
                correlation_id: row.get("correlation_id"),
            };

            Ok(Some(execution))
        } else {
            Ok(None)
        }
    }

    async fn get_executions_by_pipeline(
        &self,
        pipeline_id: &hodei_core::PipelineId,
    ) -> Result<Vec<PipelineExecution>> {
        debug!("Getting pipeline executions for pipeline: {}", pipeline_id);

        let exec_rows = sqlx::query(
            r#"
            SELECT execution_id
            FROM pipeline_executions
            WHERE pipeline_id = $1
            ORDER BY started_at DESC
        "#,
        )
        .bind(pipeline_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to get pipeline executions: {}", e))
        })?;

        let mut executions = Vec::new();
        for row in exec_rows {
            let execution_id = ExecutionId::from_uuid(row.get("execution_id"));
            if let Some(execution) = self.get_execution(&execution_id).await? {
                executions.push(execution);
            }
        }

        Ok(executions)
    }

    async fn update_execution_status(
        &self,
        execution_id: &ExecutionId,
        status: ExecutionStatus,
    ) -> Result<()> {
        debug!(
            "Updating pipeline execution status: {} -> {}",
            execution_id, status
        );

        sqlx::query(r#"
            UPDATE pipeline_executions
            SET status = $1,
                completed_at = CASE WHEN $2 IN ('COMPLETED', 'FAILED', 'CANCELLED') THEN NOW() ELSE completed_at END,
                updated_at = NOW()
            WHERE execution_id = $3
        "#)
        .bind(status.as_str())
        .bind(status.as_str())
        .bind(execution_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to update execution status: {}", e)))?;

        Ok(())
    }

    async fn update_step_status(
        &self,
        execution_id: &ExecutionId,
        step_id: &PipelineStepId,
        status: StepExecutionStatus,
    ) -> Result<()> {
        debug!(
            "Updating step status: {} for execution {}",
            step_id, execution_id
        );

        sqlx::query(r#"
            UPDATE step_executions
            SET status = $1,
                started_at = CASE WHEN $1 = 'RUNNING' THEN NOW() ELSE started_at END,
                completed_at = CASE WHEN $1 IN ('COMPLETED', 'FAILED', 'SKIPPED') THEN NOW() ELSE completed_at END,
                updated_at = NOW()
            WHERE execution_id = $2 AND step_id = $3
        "#)
        .bind(status.as_str())
        .bind(execution_id.as_uuid())
        .bind(step_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to update step status: {}", e)))?;

        Ok(())
    }

    async fn update_step(&self, execution_id: &ExecutionId, step: &StepExecution) -> Result<()> {
        debug!(
            "Updating step: {} for execution {}",
            step.step_id, execution_id
        );

        sqlx::query(
            r#"
            UPDATE step_executions
            SET status = $1,
                started_at = $2,
                completed_at = $3,
                retry_count = $4,
                error_message = $5,
                logs = $6,
                updated_at = NOW()
            WHERE step_execution_id = $7
        "#,
        )
        .bind(step.status.as_str())
        .bind(step.started_at)
        .bind(step.completed_at)
        .bind(step.retry_count as i32)
        .bind(step.error_message.as_deref())
        .bind(step.logs.join("\n"))
        .bind(step.step_execution_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to update step: {}", e)))?;

        Ok(())
    }

    async fn delete_execution(&self, execution_id: &ExecutionId) -> Result<()> {
        debug!("Deleting pipeline execution: {}", execution_id);

        sqlx::query(
            r#"
            DELETE FROM pipeline_executions WHERE execution_id = $1
        "#,
        )
        .bind(execution_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to delete pipeline execution: {}", e))
        })?;

        Ok(())
    }

    async fn get_active_executions(&self) -> Result<Vec<PipelineExecution>> {
        debug!("Getting active pipeline executions");

        let exec_rows = sqlx::query(
            r#"
            SELECT execution_id
            FROM pipeline_executions
            WHERE status IN ('PENDING', 'RUNNING')
            ORDER BY started_at
        "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to get active executions: {}", e))
        })?;

        let mut executions = Vec::new();
        for row in exec_rows {
            let execution_id = ExecutionId::from_uuid(row.get("execution_id"));
            if let Some(execution) = self.get_execution(&execution_id).await? {
                executions.push(execution);
            }
        }

        Ok(executions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Connection, Executor, PgConnection};

    #[tokio::test]
    async fn test_repository_creation() {
        // This is a basic compilation test
        let _repo = PostgreSqlPipelineExecutionRepository {
            pool: PgPool::connect("postgres://postgres:postgres@localhost:5432/test")
                .await
                .unwrap(),
        };
    }
}
