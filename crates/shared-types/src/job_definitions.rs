//! Job definition types and schemas

use crate::Uuid;
use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Job identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub Uuid);

impl JobId {
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

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource requirements for a job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub cpu_m: u64,      // CPU in millicores
    pub memory_mb: u64,  // Memory in MB
    pub gpu: Option<u8>, // Optional GPU requirement
}

impl ResourceQuota {
    pub fn new(cpu_m: u64, memory_mb: u64) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: None,
        }
    }

    pub fn with_gpu(mut self, gpu: u8) -> Self {
        self.gpu = Some(gpu);
        self
    }
}

/// Job specification (immutable value object)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub resources: ResourceQuota,
    pub timeout_ms: u64,
    pub retries: u8,
    pub env: HashMap<String, String>,
    pub secret_refs: Vec<String>,
}

impl JobSpec {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::Validation(
                "job name cannot be empty".to_string(),
            ));
        }

        if self.image.trim().is_empty() {
            return Err(DomainError::Validation("image cannot be empty".to_string()));
        }

        if self.command.is_empty() {
            return Err(DomainError::Validation(
                "command cannot be empty".to_string(),
            ));
        }

        if self.timeout_ms == 0 {
            return Err(DomainError::Validation(
                "timeout must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Job state value object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobState(String);

impl JobState {
    pub const PENDING: &'static str = "PENDING";
    pub const SCHEDULED: &'static str = "SCHEDULED";
    pub const RUNNING: &'static str = "RUNNING";
    pub const SUCCESS: &'static str = "SUCCESS";
    pub const FAILED: &'static str = "FAILED";
    pub const CANCELLED: &'static str = "CANCELLED";

    pub fn new(state: String) -> Result<Self, DomainError> {
        match state.as_str() {
            Self::PENDING
            | Self::SCHEDULED
            | Self::RUNNING
            | Self::SUCCESS
            | Self::FAILED
            | Self::CANCELLED => Ok(Self(state)),
            _ => Err(DomainError::Validation(format!(
                "invalid job state: {}",
                state
            ))),
        }
    }

    pub fn can_transition_to(&self, target: &Self) -> bool {
        match (self.0.as_str(), target.0.as_str()) {
            (Self::PENDING, Self::SCHEDULED) => true,
            (Self::PENDING, Self::CANCELLED) => true,
            (Self::SCHEDULED, Self::RUNNING) => true,
            (Self::SCHEDULED, Self::CANCELLED) => true,
            (Self::RUNNING, Self::SUCCESS) => true,
            (Self::RUNNING, Self::FAILED) => true,
            (Self::RUNNING, Self::CANCELLED) => true,
            (Self::FAILED, Self::PENDING) => true, // For retry
            (Self::FAILED, Self::CANCELLED) => true,
            _ => false,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for JobState {
    fn from(s: String) -> Self {
        Self::new(s).expect("valid state")
    }
}

/// Job execution result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}
