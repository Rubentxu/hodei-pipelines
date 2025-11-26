//! Job definition types and schemas

use crate::Uuid;
use crate::error::DomainError;
use crate::specifications::Specification;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Job identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
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

impl From<Uuid> for JobId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    /// Create a new ResourceQuota (legacy method without validation)
    pub fn new(cpu_m: u64, memory_mb: u64) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: None,
        }
    }

    /// Create a new ResourceQuota with validation
    ///
    /// # Errors
    /// Returns `crate::error::DomainError::Validation` if:
    /// - `cpu_m` is 0
    /// - `memory_mb` is 0
    pub fn create(cpu_m: u64, memory_mb: u64) -> crate::Result<Self> {
        if cpu_m == 0 {
            return Err(crate::error::DomainError::Validation(
                "CPU must be greater than 0 millicores".to_string(),
            ));
        }

        if memory_mb == 0 {
            return Err(crate::error::DomainError::Validation(
                "Memory must be greater than 0 MB".to_string(),
            ));
        }

        Ok(Self {
            cpu_m,
            memory_mb,
            gpu: None,
        })
    }

    /// Create ResourceQuota with GPU requirement
    ///
    /// # Errors
    /// Returns `crate::error::DomainError::Validation` if:
    /// - `cpu_m` is 0
    /// - `memory_mb` is 0
    /// - `gpu` is 0
    pub fn create_with_gpu(cpu_m: u64, memory_mb: u64, gpu: u8) -> crate::Result<Self> {
        if cpu_m == 0 {
            return Err(crate::error::DomainError::Validation(
                "CPU must be greater than 0 millicores".to_string(),
            ));
        }

        if memory_mb == 0 {
            return Err(crate::error::DomainError::Validation(
                "Memory must be greater than 0 MB".to_string(),
            ));
        }

        if gpu == 0 {
            return Err(crate::error::DomainError::Validation(
                "GPU must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            cpu_m,
            memory_mb,
            gpu: Some(gpu),
        })
    }
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            cpu_m: 1000,
            memory_mb: 1024,
            gpu: None,
        }
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
        use crate::job_specifications::ValidJobSpec;

        let spec = ValidJobSpec::new();
        if spec.is_satisfied_by(self) {
            Ok(())
        } else {
            Err(DomainError::Validation(
                "job specification does not meet validation requirements".to_string(),
            ))
        }
    }

    /// Create a new JobSpec builder
    pub fn builder(name: String, image: String) -> JobSpecBuilder {
        JobSpecBuilder::new(name, image)
    }

    /// Estimate memory size of JobSpec (in bytes)
    pub fn estimated_size(&self) -> usize {
        self.name.len()
            + self.image.len()
            + self.command.iter().map(|s| s.len()).sum::<usize>()
            + self
                .env
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>()
            + self.secret_refs.iter().map(|s| s.len()).sum::<usize>()
            + std::mem::size_of::<Self>()
    }
}

/// Builder for JobSpec
pub struct JobSpecBuilder {
    name: String,
    image: String,
    command: Vec<String>,
    resources: ResourceQuota,
    timeout_ms: u64,
    retries: u8,
    env: HashMap<String, String>,
    secret_refs: Vec<String>,
}

impl JobSpecBuilder {
    pub fn new(name: String, image: String) -> Self {
        Self {
            name,
            image,
            command: Vec::new(),
            resources: ResourceQuota::default(),
            timeout_ms: 300000, // 5 minutes default
            retries: 0,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        }
    }

    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = command;
        self
    }

    pub fn resources(mut self, resources: ResourceQuota) -> Self {
        self.resources = resources;
        self
    }

    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn retries(mut self, retries: u8) -> Self {
        self.retries = retries;
        self
    }

    pub fn env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn secret_refs(mut self, secret_refs: Vec<String>) -> Self {
        self.secret_refs = secret_refs;
        self
    }

    pub fn build(self) -> Result<JobSpec, DomainError> {
        let job_spec = JobSpec {
            name: self.name,
            image: self.image,
            command: self.command,
            resources: self.resources,
            timeout_ms: self.timeout_ms,
            retries: self.retries,
            env: self.env,
            secret_refs: self.secret_refs,
        };

        job_spec.validate()?;
        Ok(job_spec)
    }
}

/// Job state value object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
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

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.0.as_str(),
            Self::SUCCESS | Self::FAILED | Self::CANCELLED
        )
    }
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== TDD Tests: ResourceQuota Validation =====

    #[test]
    fn resource_quota_rejects_zero_cpu() {
        let result = ResourceQuota::create(0, 1024);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("CPU must be greater than 0"));
        }
    }

    #[test]
    fn resource_quota_rejects_zero_memory() {
        let result = ResourceQuota::create(1000, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Memory must be greater than 0"));
        }
    }

    #[test]
    fn resource_quota_rejects_zero_gpu() {
        let result = ResourceQuota::create_with_gpu(1000, 1024, 0);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("GPU must be greater than 0"));
        }
    }

    #[test]
    fn resource_quota_accepts_valid_values() {
        let quota = ResourceQuota::create(1000, 2048).unwrap();
        assert_eq!(quota.cpu_m, 1000);
        assert_eq!(quota.memory_mb, 2048);
        assert_eq!(quota.gpu, None);
    }

    #[test]
    fn resource_quota_with_gpu_accepts_valid_values() {
        let quota = ResourceQuota::create_with_gpu(2000, 4096, 1).unwrap();
        assert_eq!(quota.cpu_m, 2000);
        assert_eq!(quota.memory_mb, 4096);
        assert_eq!(quota.gpu, Some(1));
    }
}
