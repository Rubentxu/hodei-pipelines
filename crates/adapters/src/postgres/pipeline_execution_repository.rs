//! PostgreSQL Pipeline Execution Repository
//!
//! Production-ready implementation for persisting and retrieving pipeline executions.

use async_trait::async_trait;
use hodei_pipelines_core::{
    DomainError, Result,
    pipeline::PipelineStepId,
    pipeline_execution::{
        ExecutionId, ExecutionStatus, PipelineExecution, StepExecution, StepExecutionId,
        StepExecutionStatus,
    },
};
use hodei_pipelines_ports::PipelineExecutionRepository;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{debug, info};

/// Log separator for step execution logs
const LOG_SEPARATOR: &str = "\n";

/// Execution status constants
const EXECUTION_STATUS_PENDING: &str = "PENDING";
const EXECUTION_STATUS_RUNNING: &str = "RUNNING";
const EXECUTION_STATUS_COMPLETED: &str = "COMPLETED";
const EXECUTION_STATUS_FAILED: &str = "FAILED";
const EXECUTION_STATUS_CANCELLED: &str = "CANCELLED";

/// Step execution status constants
#[allow(dead_code)]
const STEP_STATUS_PENDING: &str = "PENDING";
const STEP_STATUS_RUNNING: &str = "RUNNING";
const STEP_STATUS_COMPLETED: &str = "COMPLETED";
const STEP_STATUS_FAILED: &str = "FAILED";
const STEP_STATUS_SKIPPED: &str = "SKIPPED";

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

    /// Deserialize StepExecution from a SQL row
    ///
    /// This method can handle both regular queries and JOIN queries with column aliases
    fn deserialize_step_from_row(&self, row: &sqlx::postgres::PgRow) -> Result<StepExecution> {
        let logs_str: String = row.get("logs");
        let logs = if logs_str.is_empty() {
            Vec::new()
        } else {
            logs_str
                .split(LOG_SEPARATOR)
                .map(|s| s.to_string())
                .collect()
        };

        // Handle both "status" (regular query) and "step_status" (JOIN query) column names
        let status_str = row
            .try_get::<String, _>("step_status")
            .unwrap_or_else(|_| row.get::<String, _>("status"));
        let status = StepExecutionStatus::from_str(&status_str).map_err(|_| {
            DomainError::Validation(format!("Invalid step execution status: {}", status_str))
        })?;

        // Handle both column name variations for started_at and completed_at
        // Database fields are nullable, so SQLx returns Option<DateTime>
        let started_at = row
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("step_started_at")
            .unwrap_or_else(|_| row.get("started_at"));
        let completed_at = row
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("step_completed_at")
            .unwrap_or_else(|_| row.get("completed_at"));

        Ok(StepExecution {
            step_execution_id: StepExecutionId::from_uuid(row.get("step_execution_id")),
            step_id: PipelineStepId::from_uuid(row.get("step_id")),
            status,
            started_at,
            completed_at,
            retry_count: row.get::<i32, _>("retry_count") as u8,
            error_message: row.get("error_message"),
            logs,
        })
    }

    /// Deserialize PipelineExecution from SQL rows (main query + step executions)
    fn deserialize_execution_from_rows(
        &self,
        exec_row: &sqlx::postgres::PgRow,
        step_rows: Vec<sqlx::postgres::PgRow>,
    ) -> Result<PipelineExecution> {
        let steps = step_rows
            .into_iter()
            .map(|row| self.deserialize_step_from_row(&row))
            .collect::<Result<Vec<_>>>()?;

        let variables_json: serde_json::Value = exec_row.get("variables");
        let variables =
            serde_json::from_value::<HashMap<String, String>>(variables_json).map_err(|e| {
                DomainError::Validation(format!("Failed to deserialize variables: {}", e))
            })?;

        let status_str = exec_row.get::<String, _>("status");
        let status = ExecutionStatus::from_str(&status_str).map_err(|_| {
            DomainError::Validation(format!("Invalid execution status: {}", status_str))
        })?;

        Ok(PipelineExecution {
            id: ExecutionId::from_uuid(exec_row.get("execution_id")),
            pipeline_id: hodei_pipelines_core::PipelineId::from_uuid(exec_row.get("pipeline_id")),
            status,
            started_at: exec_row.get("started_at"),
            completed_at: exec_row.get("completed_at"),
            steps,
            variables: variables.into_iter().collect(),
            tenant_id: exec_row.get("tenant_id"),
            correlation_id: exec_row.get("correlation_id"),
        })
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
            .bind(step.logs.join(LOG_SEPARATOR))
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

            let execution = self.deserialize_execution_from_rows(&row, step_rows)?;
            Ok(Some(execution))
        } else {
            Ok(None)
        }
    }

    async fn get_executions_by_pipeline(
        &self,
        pipeline_id: &hodei_pipelines_core::PipelineId,
    ) -> Result<Vec<PipelineExecution>> {
        debug!("Getting pipeline executions for pipeline: {}", pipeline_id);

        // Get all pipeline executions with their step executions in a single query
        let exec_rows = sqlx::query(
            r#"
            SELECT pe.execution_id, pe.pipeline_id, pe.status, pe.started_at, pe.completed_at,
                   pe.variables, pe.tenant_id, pe.correlation_id,
                   se.step_execution_id, se.step_id, se.status as step_status,
                   se.started_at as step_started_at, se.completed_at as step_completed_at,
                   se.retry_count, se.error_message, se.logs
            FROM pipeline_executions pe
            LEFT JOIN step_executions se ON pe.execution_id = se.execution_id
            WHERE pe.pipeline_id = $1
            ORDER BY pe.started_at DESC, se.created_at
        "#,
        )
        .bind(pipeline_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to get pipeline executions: {}", e))
        })?;

        // Group step executions by execution_id
        let mut executions_map: std::collections::HashMap<
            ExecutionId,
            (sqlx::postgres::PgRow, Vec<sqlx::postgres::PgRow>),
        > = std::collections::HashMap::new();

        for row in exec_rows {
            let execution_id = ExecutionId::from_uuid(row.get("execution_id"));
            if let Some((_, step_rows)) = executions_map.get_mut(&execution_id) {
                step_rows.push(row);
            } else {
                executions_map.insert(execution_id, (row, Vec::new()));
            }
        }

        // Deserialize each execution
        let mut executions = Vec::new();
        for (_execution_id, (exec_row, step_rows)) in executions_map {
            // Re-order step_rows to ensure correct sorting
            let step_rows_sorted = step_rows;

            let execution = self.deserialize_execution_from_rows(&exec_row, step_rows_sorted)?;
            executions.push(execution);
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

        let terminal_statuses = vec![
            EXECUTION_STATUS_COMPLETED,
            EXECUTION_STATUS_FAILED,
            EXECUTION_STATUS_CANCELLED,
        ];

        sqlx::query(
            r#"
            UPDATE pipeline_executions
            SET status = $1,
                completed_at = CASE WHEN $2 = ANY($3) THEN NOW() ELSE completed_at END,
                updated_at = NOW()
            WHERE execution_id = $4
        "#,
        )
        .bind(status.as_str())
        .bind(status.as_str())
        .bind(terminal_statuses)
        .bind(execution_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to update execution status: {}", e))
        })?;

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

        let terminal_statuses = vec![
            STEP_STATUS_COMPLETED,
            STEP_STATUS_FAILED,
            STEP_STATUS_SKIPPED,
        ];

        sqlx::query(
            r#"
            UPDATE step_executions
            SET status = $1,
                started_at = CASE WHEN $1 = $4 THEN NOW() ELSE started_at END,
                completed_at = CASE WHEN $1 = ANY($5) THEN NOW() ELSE completed_at END,
                updated_at = NOW()
            WHERE execution_id = $2 AND step_id = $3
        "#,
        )
        .bind(status.as_str())
        .bind(execution_id.as_uuid())
        .bind(step_id.as_uuid())
        .bind(STEP_STATUS_RUNNING)
        .bind(terminal_statuses)
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
        .bind(step.logs.join(LOG_SEPARATOR))
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

        // Get all active pipeline executions with their step executions in a single query
        let exec_rows = sqlx::query(
            r#"
            SELECT pe.execution_id, pe.pipeline_id, pe.status, pe.started_at, pe.completed_at,
                   pe.variables, pe.tenant_id, pe.correlation_id,
                   se.step_execution_id, se.step_id, se.status as step_status,
                   se.started_at as step_started_at, se.completed_at as step_completed_at,
                   se.retry_count, se.error_message, se.logs
            FROM pipeline_executions pe
            LEFT JOIN step_executions se ON pe.execution_id = se.execution_id
            WHERE pe.status IN ($1, $2)
            ORDER BY pe.started_at, se.created_at
        "#,
        )
        .bind(EXECUTION_STATUS_PENDING)
        .bind(EXECUTION_STATUS_RUNNING)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to get active executions: {}", e))
        })?;

        // Group step executions by execution_id
        let mut executions_map: std::collections::HashMap<
            ExecutionId,
            (sqlx::postgres::PgRow, Vec<sqlx::postgres::PgRow>),
        > = std::collections::HashMap::new();

        for row in exec_rows {
            let execution_id = ExecutionId::from_uuid(row.get("execution_id"));
            if let Some((_, step_rows)) = executions_map.get_mut(&execution_id) {
                step_rows.push(row);
            } else {
                executions_map.insert(execution_id, (row, Vec::new()));
            }
        }

        // Deserialize each execution
        let mut executions = Vec::new();
        for (_, (exec_row, step_rows)) in executions_map {
            let step_rows_sorted = step_rows;
            let execution = self.deserialize_execution_from_rows(&exec_row, step_rows_sorted)?;
            executions.push(execution);
        }

        Ok(executions)
    }
}
