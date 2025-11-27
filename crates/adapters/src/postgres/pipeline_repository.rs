//! PostgreSQL Pipeline Repository
//!
//! Production-ready implementation for persisting and retrieving pipelines.

use async_trait::async_trait;
use hodei_core::{DomainError, Pipeline, PipelineId, Result};
use hodei_ports::{PipelineRepository, PipelineRepositoryError};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::{error, info};
use uuid::Uuid;

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
}

#[async_trait]
impl PipelineRepository for PostgreSqlPipelineRepository {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<()> {
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

        // Save pipeline steps
        for step in &pipeline.steps {
            sqlx::query(
                r#"
                INSERT INTO pipeline_steps (
                    step_id, pipeline_id, name, job_spec, timeout_ms, depends_on, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
                ON CONFLICT (step_id) DO UPDATE SET
                    name = EXCLUDED.name,
                    job_spec = EXCLUDED.job_spec,
                    timeout_ms = EXCLUDED.timeout_ms,
                    depends_on = EXCLUDED.depends_on,
                    updated_at = NOW()
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

            let mut steps = Vec::new();
            for step_row in step_rows {
                let job_spec: hodei_core::job::JobSpec =
                    serde_json::from_value(step_row.get("job_spec")).map_err(|e| {
                        DomainError::Validation(format!("Failed to deserialize job spec: {}", e))
                    })?;

                let depends_on: Vec<hodei_core::pipeline::PipelineStepId> =
                    serde_json::from_value(step_row.get("depends_on")).map_err(|e| {
                        DomainError::Validation(format!(
                            "Failed to deserialize dependencies: {}",
                            e
                        ))
                    })?;

                steps.push(hodei_core::pipeline::PipelineStep {
                    id: hodei_core::pipeline::PipelineStepId::from_uuid(step_row.get("step_id")),
                    name: step_row.get("name"),
                    job_spec,
                    depends_on,
                    timeout_ms: step_row.get::<i64, _>("timeout_ms") as u64,
                });
            }

            let variables: serde_json::Value = row.get("variables");
            let variables: std::collections::HashMap<String, String> = variables
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect()
                })
                .unwrap_or_default();

            Ok(Some(Pipeline {
                id: PipelineId::from_uuid(row.get("pipeline_id")),
                name: row.get("name"),
                description: row.get("description"),
                status: hodei_core::pipeline::PipelineStatus::from_str(row.get("status"))
                    .unwrap_or(hodei_core::pipeline::PipelineStatus::PENDING),
                steps,
                variables,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                tenant_id: row.get("tenant_id"),
                workflow_definition: row.get("workflow_definition"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_all_pipelines(&self) -> Result<Vec<Pipeline>> {
        let pipeline_rows = sqlx::query(
            r#"
            SELECT pipeline_id
            FROM pipelines
            ORDER BY created_at DESC
            LIMIT 1000
        "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get all pipelines: {}", e)))?;

        let mut pipelines = Vec::new();
        for row in pipeline_rows {
            let pipeline_id = PipelineId::from_uuid(row.get("pipeline_id"));
            if let Some(pipeline) = self.get_pipeline(&pipeline_id).await? {
                pipelines.push(pipeline);
            }
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
