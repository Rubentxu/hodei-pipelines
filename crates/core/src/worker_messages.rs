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
