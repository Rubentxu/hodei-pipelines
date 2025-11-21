//! Worker-related message types for distributed communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Worker identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub cpu_m: u64,
    pub memory_mb: u64,
    pub gpu: Option<u8>,
    pub features: Vec<String>,
}
