//! Worker Domain Entity
//!
//! This module contains the Worker aggregate root and related value objects.

pub use hodei_shared_types::{WorkerCapabilities, WorkerId, WorkerStatus};

use crate::{DomainError, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Worker aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: WorkerId,
    pub name: String,
    pub status: WorkerStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tenant_id: Option<String>,
    pub capabilities: WorkerCapabilities,
    pub metadata: std::collections::HashMap<String, String>,
    pub current_jobs: Vec<Uuid>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

impl Worker {
    pub fn new(id: WorkerId, name: String, capabilities: WorkerCapabilities) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            status: WorkerStatus::create_with_status(WorkerStatus::IDLE.to_string()),
            created_at: now,
            updated_at: now,
            tenant_id: None,
            capabilities,
            metadata: std::collections::HashMap::new(),
            current_jobs: Vec::new(),
            last_heartbeat: now,
        }
    }

    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn update_status(&mut self, new_status: WorkerStatus) {
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
    }

    pub fn is_available(&self) -> bool {
        self.status.as_str() == WorkerStatus::IDLE
            && (self.current_jobs.len() as u32) < self.capabilities.max_concurrent_jobs
    }

    pub fn register_job(&mut self, job_id: Uuid) -> Result<()> {
        if (self.current_jobs.len() as u32) >= self.capabilities.max_concurrent_jobs {
            return Err(DomainError::Validation("Worker at capacity".to_string()));
        }
        self.current_jobs.push(job_id);
        Ok(())
    }

    pub fn unregister_job(&mut self, job_id: &Uuid) {
        self.current_jobs.retain(|id| id != job_id);
    }

    pub fn is_healthy(&self) -> bool {
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(self.last_heartbeat);
        diff.num_seconds() < 30
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_creation() {
        let worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        assert_eq!(worker.name, "worker-1");
        assert_eq!(worker.status.as_str(), WorkerStatus::IDLE);
        assert!(worker.is_available());
    }

    #[test]
    fn test_worker_job_registration() {
        let mut capabilities = WorkerCapabilities::new(4, 8192);
        capabilities.max_concurrent_jobs = 2;

        let mut worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        let job_id = Uuid::new_v4();
        assert!(worker.register_job(job_id).is_ok());
        assert_eq!(worker.current_jobs.len(), 1);
        assert!(worker.is_available()); // Still has capacity

        let job_id2 = Uuid::new_v4();
        assert!(worker.register_job(job_id2).is_ok());
        assert_eq!(worker.current_jobs.len(), 2);
        assert!(!worker.is_available()); // At capacity
    }

    #[test]
    fn test_worker_heartbeat() {
        let mut worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        assert!(worker.is_healthy());

        // Simulate old heartbeat
        worker.last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(60);
        assert!(!worker.is_healthy());
    }
}
