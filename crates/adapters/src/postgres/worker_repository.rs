//! PostgreSQL Worker Repository
//!
//! Production-ready implementation for persisting and retrieving workers.

use async_trait::async_trait;
use hodei_core::{DomainError, Result, Worker, WorkerId, WorkerStatus};
use hodei_ports::WorkerRepository;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::{error, info};
use uuid::Uuid;

/// PostgreSQL Worker Repository
#[derive(Debug)]
pub struct PostgreSqlWorkerRepository {
    pool: PgPool,
}

impl PostgreSqlWorkerRepository {
    /// Create a new PostgreSQL worker repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Initialize database schema for workers
    pub async fn init_schema(&self) -> Result<(), DomainError> {
        info!("Initializing worker schema");

        // Create workers table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workers (
                worker_id UUID PRIMARY KEY,
                status TEXT NOT NULL,
                worker_type TEXT NOT NULL,
                capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
                metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
                last_heartbeat TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create workers table: {}", e))
        })?;

        // Create index for faster lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_workers_status
            ON workers(status)
        "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to create index: {}", e)))?;

        info!("Worker schema initialized successfully");
        Ok(())
    }
}

#[async_trait]
impl WorkerRepository for PostgreSqlWorkerRepository {
    async fn save_worker(&self, worker: &Worker) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO workers (
                worker_id, status, worker_type, capabilities, metadata,
                last_heartbeat, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (worker_id) DO UPDATE SET
                status = EXCLUDED.status,
                worker_type = EXCLUDED.worker_type,
                capabilities = EXCLUDED.capabilities,
                metadata = EXCLUDED.metadata,
                last_heartbeat = EXCLUDED.last_heartbeat,
                updated_at = NOW()
        "#,
        )
        .bind(worker.id.as_uuid())
        .bind(worker.status.as_str())
        .bind(&worker.worker_type)
        .bind(serde_json::to_value(&worker.capabilities).unwrap_or_default())
        .bind(serde_json::to_value(&worker.metadata).unwrap_or_default())
        .bind(worker.last_heartbeat)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to save worker: {}", e)))?;

        Ok(())
    }

    async fn get_worker(&self, worker_id: &WorkerId) -> Result<Option<Worker>, DomainError> {
        let row = sqlx::query(r#"
            SELECT worker_id, status, worker_type, capabilities, metadata, last_heartbeat, created_at, updated_at
            FROM workers
            WHERE worker_id = $1
        "#)
        .bind(worker_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get worker: {}", e)))?;

        if let Some(row) = row {
            let capabilities: serde_json::Value = row.get("capabilities");
            let capabilities =
                hodei_core::worker_messages::WorkerCapabilities::from_json(&capabilities)
                    .unwrap_or_default();

            let metadata: serde_json::Value = row.get("metadata");

            Ok(Some(Worker {
                id: WorkerId::from_uuid(row.get("worker_id")),
                status: WorkerStatus::from_str(row.get("status")).unwrap_or(WorkerStatus::OFFLINE),
                worker_type: row.get("worker_type"),
                capabilities,
                metadata,
                last_heartbeat: row.get("last_heartbeat"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_all_workers(&self) -> Result<Vec<Worker>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT worker_id
            FROM workers
            ORDER BY created_at DESC
            LIMIT 1000
        "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get all workers: {}", e)))?;

        let mut workers = Vec::new();
        for row in rows {
            let worker_id = WorkerId::from_uuid(row.get("worker_id"));
            if let Some(worker) = self.get_worker(&worker_id).await? {
                workers.push(worker);
            }
        }

        Ok(workers)
    }

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            DELETE FROM workers WHERE worker_id = $1
        "#,
        )
        .bind(worker_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to delete worker: {}", e)))?;

        Ok(())
    }

    async fn update_worker_status(
        &self,
        worker_id: &WorkerId,
        status: WorkerStatus,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE workers
            SET status = $1,
                last_heartbeat = NOW(),
                updated_at = NOW()
            WHERE worker_id = $2
        "#,
        )
        .bind(status.as_str())
        .bind(worker_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to update worker status: {}", e))
        })?;

        Ok(())
    }
}
