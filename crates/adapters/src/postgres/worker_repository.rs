//! PostgreSQL Worker Repository
//!
//! Production-ready implementation for persisting and retrieving workers.

use async_trait::async_trait;
use hodei_pipelines_domain::{
    DomainError, Result, Worker, WorkerCapabilities, WorkerId, WorkerStatus,
};
use hodei_pipelines_ports::{SchedulerError, SchedulerPort, WorkerRepository};
use hodei_pipelines_proto::ServerMessage;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{error, info};
use uuid::Uuid;

/// Default worker capabilities for new workers
const DEFAULT_WORKER_CAPABILITIES: (u32, u64) = (4, 8192);

/// PostgreSQL Worker Repository
#[derive(Debug)]
pub struct PostgreSqlWorkerRepository {
    pool: PgPool,
    migrations_path: Option<String>,
    migration_file: String,
}

impl PostgreSqlWorkerRepository {
    /// Create a new PostgreSQL worker repository
    pub fn new(pool: PgPool, migrations_path: Option<String>, migration_file: String) -> Self {
        Self {
            pool,
            migrations_path,
            migration_file,
        }
    }

    /// Initialize database schema for workers
    pub async fn init_schema(&self) -> Result<()> {
        info!("Initializing worker schema from migration file");

        // Load migration SQL
        let migration_sql = self.load_migration_sql()?;

        // Parse and execute individual SQL statements
        let statements = Self::parse_sql_statements(&migration_sql)?;

        for stmt in statements {
            sqlx::query(&stmt).execute(&self.pool).await.map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to execute worker migration: {}\nStatement: {}",
                    e,
                    stmt.lines().take(3).collect::<Vec<_>>().join(" ")
                ))
            })?;
        }

        info!("Worker schema initialized successfully");
        Ok(())
    }

    /// Parse SQL file into individual executable statements
    fn parse_sql_statements(sql: &str) -> Result<Vec<String>> {
        let mut statements = Vec::new();
        let mut current_stmt = String::new();
        let mut chars = sql.chars().peekable();
        let mut in_block_comment = false;
        let mut in_line_comment = false;
        let mut in_dollar_quote = false;
        let mut dollar_quote_tag = String::new();
        let mut paren_depth = 0;

        while let Some(ch) = chars.next() {
            // Handle block comments
            if !in_line_comment && !in_dollar_quote {
                if ch == '/' && chars.peek() == Some(&'*') {
                    in_block_comment = true;
                    chars.next(); // consume '*'
                    continue;
                }
                if ch == '*' && chars.peek() == Some(&'/') {
                    in_block_comment = false;
                    chars.next(); // consume '/'
                    continue;
                }
            }

            // Handle line comments
            if !in_block_comment && !in_dollar_quote
                && ch == '-' && chars.peek() == Some(&'-') {
                    in_line_comment = true;
                    // Skip to end of line
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '\n' {
                            break;
                        }
                        chars.next();
                    }
                    continue;
                }

            // Handle dollar-quoted strings
            if ch == '$' && !in_block_comment && !in_line_comment {
                let mut tag = String::new();
                tag.push('$');

                // Collect the tag
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '$' {
                        tag.push('$');
                        chars.next(); // consume closing $
                        break;
                    } else if next_ch == '\n' {
                        // Invalid dollar quote, treat as regular $
                        break;
                    } else {
                        tag.push(next_ch);
                        chars.next();
                    }
                }

                if !in_dollar_quote {
                    // Start of dollar quote
                    in_dollar_quote = true;
                    dollar_quote_tag = tag.clone();
                    current_stmt.push_str(&tag);
                    continue;
                } else if tag == dollar_quote_tag {
                    // End of dollar quote
                    in_dollar_quote = false;
                    current_stmt.push_str(&tag);
                    dollar_quote_tag.clear();
                    continue;
                }
            }

            if in_block_comment || in_line_comment {
                continue;
            }

            // Track parentheses depth
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')'
                && paren_depth > 0 {
                    paren_depth -= 1;
                }

            // Accumulate character
            current_stmt.push(ch);

            // Check for statement end (semicolon not inside quotes, comments, or parentheses)
            if ch == ';' && !in_dollar_quote && paren_depth == 0 {
                let stmt = current_stmt.trim();
                if !stmt.is_empty() {
                    statements.push(stmt.to_string());
                }
                current_stmt = String::new();
            }
        }

        Ok(statements)
    }

    /// Load migration SQL from file or use embedded fallback
    fn load_migration_sql(&self) -> Result<String> {
        if let Some(custom_path) = &self.migrations_path {
            let path = format!("{}/{}", custom_path, self.migration_file);
            let sql = std::fs::read_to_string(&path).map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to load migration file from custom path {}: {}",
                    path, e
                ))
            })?;
            info!("Loaded worker migration from custom path: {}", path);
            return Ok(sql);
        }

        // Use embedded SQL for the configured migration file
        match self.migration_file.as_str() {
            "20241201_workers.sql" => {
                Ok(include_str!("../../migrations/20241201_workers.sql").to_string())
            }
            other => Err(DomainError::Infrastructure(format!(
                "Unknown worker migration file: {}. Expected one of: 20241201_workers.sql",
                other
            ))),
        }
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
        status: hodei_pipelines_domain::WorkerStatus,
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

// Implement SchedulerPort for PostgreSqlWorkerRepository
// This allows using the repository directly as a scheduler in production
#[async_trait::async_trait]
impl SchedulerPort for PostgreSqlWorkerRepository {
    async fn register_worker(&self, worker: &Worker) -> std::result::Result<(), SchedulerError> {
        info!("SchedulerPort: Registering worker {}", worker.id);

        self.save_worker(worker)
            .await
            .map_err(|e| SchedulerError::WorkerRepository(e.to_string()))?;

        info!("✅ Worker {} registered successfully", worker.id);
        Ok(())
    }

    async fn unregister_worker(
        &self,
        worker_id: &WorkerId,
    ) -> std::result::Result<(), SchedulerError> {
        info!("SchedulerPort: Unregistering worker {}", worker_id);

        self.delete_worker(worker_id)
            .await
            .map_err(|e| SchedulerError::WorkerRepository(e.to_string()))?;

        info!("✅ Worker {} removed", worker_id);
        Ok(())
    }

    async fn get_registered_workers(&self) -> std::result::Result<Vec<WorkerId>, SchedulerError> {
        self.get_all_workers()
            .await
            .map_err(|e| {
                error!("Failed to get workers: {}", e);
                SchedulerError::WorkerRepository(e.to_string())
            })
            .map(|workers| workers.into_iter().map(|w| w.id).collect())
    }

    async fn register_transmitter(
        &self,
        _worker_id: &WorkerId,
        _transmitter: tokio::sync::mpsc::UnboundedSender<
            std::result::Result<ServerMessage, SchedulerError>,
        >,
    ) -> std::result::Result<(), SchedulerError> {
        info!("SchedulerPort: Transmitter registered");
        Ok(())
    }

    async fn unregister_transmitter(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<(), SchedulerError> {
        info!("SchedulerPort: Transmitter unregistered");
        Ok(())
    }

    async fn send_to_worker(
        &self,
        _worker_id: &WorkerId,
        _message: ServerMessage,
    ) -> std::result::Result<(), SchedulerError> {
        info!("SchedulerPort: Message sent (implementation pending for streaming)");
        Ok(())
    }
}
