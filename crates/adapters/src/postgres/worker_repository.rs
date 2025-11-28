//! PostgreSQL Worker Repository
//!
//! Production-ready implementation for persisting and retrieving workers.

use async_trait::async_trait;
use hodei_core::{DomainError, Result, Worker, WorkerCapabilities, WorkerId, WorkerStatus};
use hodei_ports::WorkerRepository;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Default worker capabilities for new workers
const DEFAULT_WORKER_CAPABILITIES: (u32, u64) = (4, 8192);

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
    pub async fn init_schema(&self) -> Result<()> {
        info!("Initializing worker schema");

        // Create workers table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workers (
                worker_id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
                metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
                current_jobs JSONB NOT NULL DEFAULT '[]'::jsonb,
                last_heartbeat TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                tenant_id TEXT NULL,
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

    /// Deserialize a Worker from a SQL row
    fn deserialize_worker_from_row(&self, row: &sqlx::postgres::PgRow) -> Result<Worker> {
        let capabilities_json: serde_json::Value = row.get("capabilities");
        let capabilities = serde_json::from_value::<WorkerCapabilities>(capabilities_json)
            .map_err(|e| {
                DomainError::Validation(format!("Failed to deserialize capabilities: {}", e))
            })
            .unwrap_or_else(|_| {
                WorkerCapabilities::new(
                    DEFAULT_WORKER_CAPABILITIES.0,
                    DEFAULT_WORKER_CAPABILITIES.1,
                )
            });

        let metadata_json: serde_json::Value = row.get("metadata");
        let metadata = serde_json::from_value::<HashMap<String, String>>(metadata_json)
            .map_err(|e| DomainError::Validation(format!("Failed to deserialize metadata: {}", e)))
            .unwrap_or_default();

        let current_jobs_json: serde_json::Value = row.get("current_jobs");
        let current_jobs = serde_json::from_value::<Vec<Uuid>>(current_jobs_json)
            .map_err(|e| {
                DomainError::Validation(format!("Failed to deserialize current jobs: {}", e))
            })
            .unwrap_or_default();

        let status_str = row.get::<String, _>("status");
        let status = WorkerStatus::create_with_status(status_str);

        Ok(Worker {
            id: WorkerId::from_uuid(row.get("worker_id")),
            name: row.get("name"),
            status,
            capabilities,
            metadata,
            current_jobs,
            last_heartbeat: row.get("last_heartbeat"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            tenant_id: row.get("tenant_id"),
        })
    }
}

#[async_trait]
impl WorkerRepository for PostgreSqlWorkerRepository {
    async fn save_worker(&self, worker: &Worker) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO workers (
                worker_id, name, status, capabilities, metadata,
                current_jobs, last_heartbeat, tenant_id, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            ON CONFLICT (worker_id) DO UPDATE SET
                name = EXCLUDED.name,
                status = EXCLUDED.status,
                capabilities = EXCLUDED.capabilities,
                metadata = EXCLUDED.metadata,
                current_jobs = EXCLUDED.current_jobs,
                last_heartbeat = EXCLUDED.last_heartbeat,
                tenant_id = EXCLUDED.tenant_id,
                updated_at = NOW()
        "#,
        )
        .bind(worker.id.as_uuid())
        .bind(&worker.name)
        .bind(worker.status.as_str())
        .bind(serde_json::to_value(&worker.capabilities).unwrap_or_default())
        .bind(serde_json::to_value(&worker.metadata).unwrap_or_default())
        .bind(serde_json::to_value(&worker.current_jobs).unwrap_or_default())
        .bind(worker.last_heartbeat)
        .bind(&worker.tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to save worker: {}", e)))?;

        Ok(())
    }

    async fn get_worker(&self, worker_id: &WorkerId) -> Result<Option<Worker>> {
        let row = sqlx::query(
            r#"
            SELECT worker_id, name, status, capabilities, metadata,
                   current_jobs, last_heartbeat, tenant_id, created_at, updated_at
            FROM workers
            WHERE worker_id = $1
        "#,
        )
        .bind(worker_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get worker: {}", e)))?;

        if let Some(row) = row {
            Ok(Some(self.deserialize_worker_from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn get_all_workers(&self) -> Result<Vec<Worker>> {
        let rows = sqlx::query(
            r#"
            SELECT worker_id, name, status, capabilities, metadata,
                   current_jobs, last_heartbeat, tenant_id, created_at, updated_at
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
            workers.push(self.deserialize_worker_from_row(&row)?);
        }

        Ok(workers)
    }

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<()> {
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

    async fn update_last_seen(&self, worker_id: &WorkerId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE workers
            SET last_heartbeat = NOW(),
                updated_at = NOW()
            WHERE worker_id = $1
        "#,
        )
        .bind(worker_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to update last seen: {}", e)))?;

        Ok(())
    }

    async fn find_stale_workers(
        &self,
        threshold_duration: std::time::Duration,
    ) -> Result<Vec<Worker>> {
        let threshold_seconds = threshold_duration.as_secs();
        let threshold = chrono::Utc::now() - chrono::Duration::seconds(threshold_seconds as i64);
        let rows = sqlx::query(
            r#"
            SELECT worker_id, name, status, capabilities, metadata,
                   current_jobs, last_heartbeat, tenant_id, created_at, updated_at
            FROM workers
            WHERE last_heartbeat < $1
            ORDER BY last_heartbeat ASC
            LIMIT 1000
        "#,
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to find stale workers: {}", e)))?;

        let mut workers = Vec::new();
        for row in rows {
            workers.push(self.deserialize_worker_from_row(&row)?);
        }

        Ok(workers)
    }

    async fn update_worker_status(
        &self,
        worker_id: &WorkerId,
        status: hodei_core::worker_messages::WorkerStatus,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE workers
            SET status = $1,
                current_jobs = $2,
                last_heartbeat = $3,
                updated_at = NOW()
            WHERE worker_id = $4
        "#,
        )
        .bind(status.as_str())
        .bind(serde_json::to_value(&status.current_jobs).unwrap_or_default())
        .bind(chrono::DateTime::<chrono::Utc>::from(status.last_heartbeat))
        .bind(worker_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to update worker status: {}", e))
        })?;

        Ok(())
    }
}
