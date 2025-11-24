//! Worker mapper for database persistence
//!
//! This module provides the mapping layer between domain objects and database rows,
//! reducing Feature Envy in the repository adapters.

use crate::{Worker, WorkerId};
use chrono::{DateTime, Utc};
use hodei_shared_types::WorkerCapabilities;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Database row representation for Worker entity
#[derive(Debug, Clone)]
pub struct WorkerRow {
    pub id: WorkerId,
    pub name: String,
    pub status: String,
    pub capabilities_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tenant_id: Option<String>,
    pub metadata: Option<Value>,
    pub current_jobs: Vec<Uuid>,
}

/// Mapper trait for Worker entity
pub trait WorkerMapper {
    /// Convert domain Worker to database row
    fn to_row(&self, worker: &Worker) -> WorkerRow;

    /// Convert database row to domain Worker
    fn from_row(&self, row: WorkerRow) -> Result<Worker, String>;
}

/// SQLx-based Worker mapper implementation
pub struct SqlxWorkerMapper;

impl SqlxWorkerMapper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqlxWorkerMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerMapper for SqlxWorkerMapper {
    fn to_row(&self, worker: &Worker) -> WorkerRow {
        WorkerRow {
            id: worker.id.clone(),
            name: worker.name.clone(),
            status: worker.status.status.clone(),
            capabilities_json: serde_json::to_string(&worker.capabilities).ok(),
            created_at: worker.created_at,
            updated_at: worker.updated_at,
            tenant_id: worker.tenant_id.clone(),
            metadata: serde_json::to_value(&worker.metadata).ok(),
            current_jobs: worker.current_jobs.clone(),
        }
    }

    fn from_row(&self, row: WorkerRow) -> Result<Worker, String> {
        let capabilities = match row.capabilities_json {
            Some(json_str) => serde_json::from_str::<WorkerCapabilities>(&json_str)
                .map_err(|e| format!("Failed to deserialize capabilities: {}", e))?,
            None => WorkerCapabilities::new(1, 1024), // Default capabilities
        };

        let worker_status = hodei_shared_types::WorkerStatus {
            worker_id: row.id.clone(),
            status: row.status,
            current_jobs: row.current_jobs.into_iter().map(Into::into).collect(),
            last_heartbeat: chrono::Utc::now().into(),
        };

        Ok(Worker {
            id: row.id,
            name: row.name,
            status: worker_status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            tenant_id: row.tenant_id,
            capabilities,
            metadata: row
                .metadata
                .and_then(|v| serde_json::from_value::<Option<HashMap<String, String>>>(v).ok())
                .flatten()
                .unwrap_or_default(),
            current_jobs: Vec::new(),
            last_heartbeat: chrono::Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Worker;
    use hodei_shared_types::{WorkerCapabilities, WorkerId};

    fn create_test_worker() -> Worker {
        Worker {
            id: WorkerId::new(),
            name: "test-worker".to_string(),
            status: hodei_shared_types::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: Vec::new(),
                last_heartbeat: chrono::Utc::now().into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: Some("test-tenant".to_string()),
            capabilities: WorkerCapabilities::new(4, 4096),
            metadata: HashMap::new(),
            current_jobs: Vec::new(),
            last_heartbeat: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_to_row_from_row() {
        let mapper = SqlxWorkerMapper;
        let worker = create_test_worker();

        let row = mapper.to_row(&worker);
        assert_eq!(row.name, worker.name);
        assert_eq!(row.status, worker.status.status);

        let recovered_worker = mapper.from_row(row).unwrap();
        assert_eq!(recovered_worker.name, worker.name);
        assert_eq!(recovered_worker.capabilities.max_concurrent_jobs, 4);
    }
}
