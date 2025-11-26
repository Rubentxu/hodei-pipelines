//! Worker-related message types for distributed communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Worker identifier
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

/// Worker state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerState {
    Creating,
    Available,
    Running,
    Unhealthy,
    Draining,
    Terminated,
    Failed { reason: String },
}

/// Worker status for tracking worker state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub worker_id: WorkerId,
    pub status: String,
    pub current_jobs: Vec<super::JobId>,
    pub last_heartbeat: std::time::SystemTime,
}

// For simplicity, WorkerStatus is stored as JSON in PostgreSQL
// No direct SQLx Type implementation needed

impl WorkerStatus {
    pub const IDLE: &'static str = "IDLE";
    pub const BUSY: &'static str = "BUSY";
    pub const OFFLINE: &'static str = "OFFLINE";
    pub const DRAINING: &'static str = "DRAINING";

    pub fn new(worker_id: WorkerId, status: String) -> Self {
        Self {
            worker_id,
            status,
            current_jobs: Vec::new(),
            last_heartbeat: std::time::SystemTime::now(),
        }
    }

    pub fn create_with_status(status: String) -> Self {
        Self {
            worker_id: WorkerId::new(),
            status,
            current_jobs: Vec::new(),
            last_heartbeat: std::time::SystemTime::now(),
        }
    }

    pub fn from_status_string(status: String) -> Self {
        Self::create_with_status(status)
    }

    pub fn as_str(&self) -> &str {
        &self.status
    }

    pub fn is_available(&self) -> bool {
        self.status == Self::IDLE
    }
}

/// Runtime specification for worker
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSpec {
    pub image: String,
    pub command: Option<Vec<String>>,
    pub resources: crate::job_definitions::ResourceQuota,
    pub env: std::collections::HashMap<String, String>,
    pub labels: std::collections::HashMap<String, String>,
}

impl RuntimeSpec {
    pub fn new(image: String) -> Self {
        Self {
            image,
            command: None,
            resources: crate::job_definitions::ResourceQuota::new(100, 512),
            env: std::collections::HashMap::new(),
            labels: std::collections::HashMap::new(),
        }
    }
}

/// Worker capabilities for matching
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerCapabilities {
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub gpu: Option<u8>,
    pub features: Vec<String>,
    pub labels: std::collections::HashMap<String, String>,
    pub max_concurrent_jobs: u32,
}

impl WorkerCapabilities {
    /// Create new WorkerCapabilities (legacy method without validation)
    pub fn new(cpu_cores: u32, memory_gb: u64) -> Self {
        Self {
            cpu_cores,
            memory_gb,
            gpu: None,
            features: Vec::new(),
            labels: std::collections::HashMap::new(),
            max_concurrent_jobs: 4,
        }
    }

    /// Create new WorkerCapabilities with validation
    ///
    /// # Errors
    /// Returns `crate::error::DomainError::Validation` if:
    /// - `cpu_cores` is 0
    /// - `memory_gb` is 0
    pub fn create(cpu_cores: u32, memory_gb: u64) -> crate::Result<Self> {
        if cpu_cores == 0 {
            return Err(crate::error::DomainError::Validation(
                "CPU cores must be greater than 0".to_string(),
            ));
        }

        if memory_gb == 0 {
            return Err(crate::error::DomainError::Validation(
                "Memory must be greater than 0 GB".to_string(),
            ));
        }

        Ok(Self {
            cpu_cores,
            memory_gb,
            gpu: None,
            features: Vec::new(),
            labels: std::collections::HashMap::new(),
            max_concurrent_jobs: 4, // Default value
        })
    }

    /// Create WorkerCapabilities with all parameters
    ///
    /// # Errors
    /// Returns `crate::error::DomainError::Validation` if any parameter is invalid
    pub fn create_with_concurrency(
        cpu_cores: u32,
        memory_gb: u64,
        max_concurrent_jobs: u32,
    ) -> crate::Result<Self> {
        if cpu_cores == 0 {
            return Err(crate::error::DomainError::Validation(
                "CPU cores must be greater than 0".to_string(),
            ));
        }

        if memory_gb == 0 {
            return Err(crate::error::DomainError::Validation(
                "Memory must be greater than 0 GB".to_string(),
            ));
        }

        if max_concurrent_jobs == 0 {
            return Err(crate::error::DomainError::Validation(
                "Max concurrent jobs must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            cpu_cores,
            memory_gb,
            gpu: None,
            features: Vec::new(),
            labels: std::collections::HashMap::new(),
            max_concurrent_jobs,
        })
    }
}

/// Worker message envelope for distributed communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMessage {
    pub correlation_id: crate::correlation::CorrelationId,
    pub worker_id: WorkerId,
    pub message_type: String,
    pub payload: serde_json::Value,
}

/// Worker state message for status updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStateMessage {
    pub worker_id: WorkerId,
    pub state: WorkerState,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== TDD Tests: WorkerCapabilities Validation =====

    #[test]
    fn worker_capabilities_rejects_zero_cpu() {
        let result = WorkerCapabilities::create(0, 8);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("CPU cores must be greater than 0"));
        }
    }

    #[test]
    fn worker_capabilities_rejects_zero_memory() {
        let result = WorkerCapabilities::create(4, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Memory must be greater than 0 GB"));
        }
    }

    #[test]
    fn worker_capabilities_rejects_zero_concurrent_jobs() {
        let result = WorkerCapabilities::create_with_concurrency(4, 8, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(
                e.to_string()
                    .contains("Max concurrent jobs must be greater than 0")
            );
        }
    }

    #[test]
    fn worker_capabilities_accepts_valid_values() {
        let caps = WorkerCapabilities::create(4, 16).unwrap();
        assert_eq!(caps.cpu_cores, 4);
        assert_eq!(caps.memory_gb, 16);
        assert_eq!(caps.max_concurrent_jobs, 4);
    }

    #[test]
    fn worker_capabilities_with_concurrency_accepts_valid_values() {
        let caps = WorkerCapabilities::create_with_concurrency(8, 32, 10).unwrap();
        assert_eq!(caps.cpu_cores, 8);
        assert_eq!(caps.memory_gb, 32);
        assert_eq!(caps.max_concurrent_jobs, 10);
    }
}
