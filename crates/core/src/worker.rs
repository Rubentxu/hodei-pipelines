//! Worker Domain Entity
//!
//! This module contains the Worker aggregate root and related value objects.

pub use crate::worker_messages::{WorkerCapabilities, WorkerId, WorkerStatus};

use crate::Result;
use crate::error::DomainError;
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
    use std::collections::HashMap;

    // ===== WorkerId Tests =====

    #[test]
    fn test_worker_id_generation() {
        let id1 = WorkerId::new();
        let id2 = WorkerId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_worker_id_display() {
        let id = WorkerId::new();
        let id_str = format!("{}", id);
        assert!(!id_str.is_empty());
    }

    // ===== Worker Creation Tests =====

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
        assert!(worker.metadata.is_empty());
        assert!(worker.current_jobs.is_empty());
    }

    #[test]
    fn test_worker_with_tenant() {
        let worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        )
        .with_tenant_id("tenant-123".to_string());

        assert_eq!(worker.tenant_id, Some("tenant-123".to_string()));
    }

    #[test]
    fn test_worker_with_metadata() {
        let worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        )
        .with_metadata("key1".to_string(), "value1".to_string());

        assert_eq!(worker.metadata.get("key1"), Some(&"value1".to_string()));
    }

    // ===== Job Registration Tests =====

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
    fn test_worker_job_unregistration() {
        let mut capabilities = WorkerCapabilities::new(4, 8192);
        capabilities.max_concurrent_jobs = 2;

        let mut worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        let job_id = Uuid::new_v4();
        worker.register_job(job_id).unwrap();
        assert_eq!(worker.current_jobs.len(), 1);

        worker.unregister_job(&job_id);
        assert_eq!(worker.current_jobs.len(), 0);
        assert!(worker.is_available());
    }

    #[test]
    fn test_worker_job_registration_at_capacity() {
        let mut capabilities = WorkerCapabilities::new(4, 8192);
        capabilities.max_concurrent_jobs = 1;

        let mut worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        let job_id1 = Uuid::new_v4();
        assert!(worker.register_job(job_id1).is_ok());
        assert_eq!(worker.current_jobs.len(), 1);
        assert!(!worker.is_available());

        let job_id2 = Uuid::new_v4();
        assert!(worker.register_job(job_id2).is_err());
        assert_eq!(worker.current_jobs.len(), 1);
    }

    #[test]
    fn test_worker_register_multiple_jobs() {
        let mut capabilities = WorkerCapabilities::new(4, 8192);
        capabilities.max_concurrent_jobs = 5;

        let mut worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        for i in 0..5 {
            let job_id = Uuid::new_v4();
            assert!(worker.register_job(job_id).is_ok());
            assert_eq!(worker.current_jobs.len(), i + 1);
        }

        assert_eq!(worker.current_jobs.len(), 5);
        assert!(!worker.is_available());
    }

    // ===== Status Management Tests =====

    #[test]
    fn test_worker_status_update() {
        let mut worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        let new_status = WorkerStatus {
            worker_id: WorkerId::new(),
            status: "BUSY".to_string(),
            current_jobs: vec![],
            last_heartbeat: chrono::Utc::now().into(),
        };

        worker.update_status(new_status);
        assert_eq!(worker.status.as_str(), "BUSY");
    }

    #[test]
    fn test_worker_availability() {
        let mut capabilities = WorkerCapabilities::new(4, 8192);
        capabilities.max_concurrent_jobs = 2;

        let worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        assert!(worker.is_available());

        // Register jobs
        let job_id1 = Uuid::new_v4();
        let job_id2 = Uuid::new_v4();

        // This is a hack since we can't mutate in a test
        let mut worker = worker.clone();
        worker.register_job(job_id1).unwrap();
        assert!(worker.is_available());

        worker.register_job(job_id2).unwrap();
        assert!(!worker.is_available());
    }

    // ===== Heartbeat and Health Tests =====

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

    #[test]
    fn test_worker_health_with_recent_heartbeat() {
        let worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        // Heartbeat is recent
        assert!(worker.is_healthy());
    }

    #[test]
    fn test_worker_health_with_boundary_heartbeat() {
        let mut worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        // Exactly at 30 seconds - should be unhealthy
        worker.last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(30);
        assert!(!worker.is_healthy());

        // Just before 30 seconds - should be healthy
        worker.last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(29);
        assert!(worker.is_healthy());
    }

    // ===== Capabilities Tests =====

    #[test]
    fn test_worker_capabilities() {
        let mut capabilities = WorkerCapabilities::new(8, 8);
        capabilities.max_concurrent_jobs = 8;
        let worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        assert_eq!(worker.capabilities.max_concurrent_jobs, 8);
        assert_eq!(worker.capabilities.memory_gb, 8);
        assert_eq!(worker.capabilities.cpu_cores, 8);
    }

    #[test]
    fn test_worker_with_different_capabilities() {
        let mut capabilities1 = WorkerCapabilities::new(16, 8);
        capabilities1.max_concurrent_jobs = 16;
        let worker1 = Worker::new(WorkerId::new(), "cpu-worker".to_string(), capabilities1);

        let mut capabilities2 = WorkerCapabilities::new(4, 16);
        capabilities2.max_concurrent_jobs = 4;
        let worker2 = Worker::new(WorkerId::new(), "memory-worker".to_string(), capabilities2);

        assert_eq!(worker1.capabilities.max_concurrent_jobs, 16);
        assert_eq!(worker1.capabilities.cpu_cores, 16);
        assert_eq!(worker2.capabilities.memory_gb, 16);
        assert_eq!(worker2.capabilities.max_concurrent_jobs, 4);
    }

    // ===== Timestamp Tests =====

    #[test]
    fn test_worker_timestamps() {
        let worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        assert!(worker.created_at <= worker.updated_at);
        assert!(worker.updated_at <= worker.last_heartbeat);
    }

    #[test]
    fn test_worker_timestamp_consistency() {
        let worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        let created = worker.created_at;
        let updated = worker.updated_at;
        let heartbeat = worker.last_heartbeat;

        assert!(created <= updated);
        assert!(updated <= heartbeat);
    }

    // ===== Edge Cases =====

    #[test]
    fn test_worker_with_no_capacity() {
        let mut capabilities = WorkerCapabilities::new(4, 8192);
        capabilities.max_concurrent_jobs = 0;

        let mut worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        assert!(!worker.is_available());

        let job_id = Uuid::new_v4();
        assert!(worker.register_job(job_id).is_err());
    }

    #[test]
    fn test_worker_unregister_nonexistent_job() {
        let mut worker = Worker::new(
            WorkerId::new(),
            "worker-1".to_string(),
            WorkerCapabilities::new(4, 8192),
        );

        let fake_job_id = Uuid::new_v4();
        worker.unregister_job(&fake_job_id);
        assert_eq!(worker.current_jobs.len(), 0);
    }

    #[test]
    fn test_worker_unregister_does_not_affect_other_jobs() {
        let mut capabilities = WorkerCapabilities::new(4, 8192);
        capabilities.max_concurrent_jobs = 3;

        let mut worker = Worker::new(WorkerId::new(), "worker-1".to_string(), capabilities);

        let job_id1 = Uuid::new_v4();
        let job_id2 = Uuid::new_v4();
        let job_id3 = Uuid::new_v4();

        worker.register_job(job_id1).unwrap();
        worker.register_job(job_id2).unwrap();
        worker.register_job(job_id3).unwrap();

        assert_eq!(worker.current_jobs.len(), 3);

        worker.unregister_job(&job_id2);
        assert_eq!(worker.current_jobs.len(), 2);

        // Verify job_id1 and job_id3 are still there
        assert!(worker.current_jobs.contains(&job_id1));
        assert!(worker.current_jobs.contains(&job_id3));
        assert!(!worker.current_jobs.contains(&job_id2));
    }
}
