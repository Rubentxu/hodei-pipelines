//! Worker Domain Entity
//!
//! This module contains the Worker aggregate root and related value objects.

use crate::{DomainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Worker identifier - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct WorkerId(pub Uuid);

impl WorkerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WorkerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Worker capabilities - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerCapabilities {
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub gpu: Option<u8>,
    pub labels: HashMap<String, String>,
    pub architectures: Vec<String>,
    pub max_concurrent_jobs: u32,
}

impl WorkerCapabilities {
    pub fn new(cpu_cores: u32, memory_gb: u64) -> Self {
        Self {
            cpu_cores,
            memory_gb,
            gpu: None,
            labels: HashMap::new(),
            architectures: vec!["amd64".to_string()],
            max_concurrent_jobs: 1,
        }
    }

    pub fn with_gpu(mut self, gpu: u8) -> Self {
        self.gpu = Some(gpu);
        self
    }

    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    pub fn has_label(&self, key: &str, value: &str) -> bool {
        self.labels.get(key) == Some(&value.to_string())
    }

    pub fn can_run(&self, required_cpu: u32, required_memory_mb: u64) -> bool {
        self.cpu_cores >= required_cpu && (self.memory_gb * 1024) >= required_memory_mb
    }
}

/// Worker status - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct WorkerStatus(String);

impl WorkerStatus {
    pub const IDLE: &'static str = "IDLE";
    pub const BUSY: &'static str = "BUSY";
    pub const OFFLINE: &'static str = "OFFLINE";
    pub const DRAINING: &'static str = "DRAINING";

    pub fn new(status: String) -> Result<Self> {
        match status.as_str() {
            Self::IDLE | Self::BUSY | Self::OFFLINE | Self::DRAINING => Ok(Self(status)),
            _ => Err(DomainError::Validation(format!(
                "invalid worker status: {}",
                status
            ))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_available(&self) -> bool {
        self.0 == Self::IDLE
    }
}

impl From<String> for WorkerStatus {
    fn from(s: String) -> Self {
        Self::new(s).expect("valid status")
    }
}

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
            status: WorkerStatus::new(WorkerStatus::IDLE.to_string()).unwrap(),
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
