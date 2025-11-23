//! Job Domain Entity
//!
//! This module contains the Job aggregate root and related value objects.

use crate::{DomainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Job identifier - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

    pub fn from_legacy(id: &str) -> Self {
        Self(Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()))
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Resource requirements - Value Object
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

    pub fn fits_within(&self, available: &ResourceQuota) -> bool {
        self.cpu_m <= available.cpu_m && self.memory_mb <= available.memory_mb
    }
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self::new(1000, 512) // 1 CPU core, 512MB RAM
    }
}

/// Job state - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct JobState(String);

impl JobState {
    pub const PENDING: &'static str = "PENDING";
    pub const SCHEDULED: &'static str = "SCHEDULED";
    pub const RUNNING: &'static str = "RUNNING";
    pub const SUCCESS: &'static str = "SUCCESS";
    pub const FAILED: &'static str = "FAILED";
    pub const CANCELLED: &'static str = "CANCELLED";

    pub fn new(state: String) -> Result<Self> {
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

impl From<String> for JobState {
    fn from(s: String) -> Self {
        Self::new(s).expect("valid state")
    }
}

/// Job specification - Value Object
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
    pub fn validate(&self) -> Result<()> {
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

    pub fn builder() -> JobSpecBuilder {
        JobSpecBuilder::new()
    }
}

/// Job specification builder
#[derive(Debug, Default)]
pub struct JobSpecBuilder {
    name: Option<String>,
    image: Option<String>,
    command: Option<Vec<String>>,
    resources: Option<ResourceQuota>,
    timeout_ms: Option<u64>,
    retries: Option<u8>,
    env: Option<HashMap<String, String>>,
    secret_refs: Option<Vec<String>>,
}

impl JobSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }

    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    pub fn resources(mut self, resources: ResourceQuota) -> Self {
        self.resources = Some(resources);
        self
    }

    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn retries(mut self, retries: u8) -> Self {
        self.retries = Some(retries);
        self
    }

    pub fn env(mut self, env: HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }

    pub fn secret_refs(mut self, secret_refs: Vec<String>) -> Self {
        self.secret_refs = Some(secret_refs);
        self
    }

    pub fn build(self) -> Result<JobSpec> {
        let spec = JobSpec {
            name: self.name.unwrap_or_default(),
            image: self.image.unwrap_or_default(),
            command: self.command.unwrap_or_default(),
            resources: self.resources.unwrap_or_default(),
            timeout_ms: self.timeout_ms.unwrap_or(30000),
            retries: self.retries.unwrap_or(3),
            env: self.env.unwrap_or_default(),
            secret_refs: self.secret_refs.unwrap_or_default(),
        };

        spec.validate()?;
        Ok(spec)
    }
}

/// Job aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub name: String,
    pub description: Option<String>,
    pub spec: JobSpec,
    pub state: JobState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tenant_id: Option<String>,
    pub result: serde_json::Value,
}

impl Job {
    pub fn new(id: JobId, spec: JobSpec) -> Result<Self> {
        spec.validate()?;

        let now = chrono::Utc::now();
        Ok(Self {
            id,
            name: spec.name.clone(),
            description: None,
            spec,
            state: JobState::new(JobState::PENDING.to_string())?,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            tenant_id: None,
            result: serde_json::Value::Null,
        })
    }

    pub fn schedule(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::SCHEDULED.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::RUNNING.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn complete(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::SUCCESS.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn fail(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::FAILED.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<()> {
        let new_state = JobState::new(JobState::CANCELLED.to_string())?;

        if !self.state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: new_state.as_str().to_string(),
            });
        }

        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn is_pending(&self) -> bool {
        self.state.as_str() == JobState::PENDING
    }

    pub fn is_running(&self) -> bool {
        self.state.as_str() == JobState::RUNNING
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

/// Job execution result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub duration_ms: u64,
}

impl ExecResult {
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            stdout: None,
            stderr: None,
            duration_ms: 0,
        }
    }

    pub fn failure(exit_code: i32) -> Self {
        Self {
            exit_code,
            stdout: None,
            stderr: None,
            duration_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_id_new_generates_unique() {
        let id1 = JobId::new();
        let id2 = JobId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_job_spec_validation() {
        let spec = JobSpec {
            name: "test".to_string(),
            image: "ubuntu:22.04".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_job_state_transition() {
        let pending = JobState::new(JobState::PENDING.to_string()).unwrap();
        let scheduled = JobState::new(JobState::SCHEDULED.to_string()).unwrap();
        let running = JobState::new(JobState::RUNNING.to_string()).unwrap();
        let success = JobState::new(JobState::SUCCESS.to_string()).unwrap();

        assert!(pending.can_transition_to(&scheduled));
        assert!(scheduled.can_transition_to(&running));
        assert!(running.can_transition_to(&success));
        assert!(!pending.can_transition_to(&running));
    }

    #[test]
    fn test_job_lifecycle() {
        let spec = JobSpec {
            name: "test".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let mut job = Job::new(JobId::new(), spec).unwrap();

        assert!(job.is_pending());

        job.schedule().unwrap();
        assert_eq!(job.state.as_str(), JobState::SCHEDULED);

        job.start().unwrap();
        assert!(job.is_running());

        job.complete().unwrap();
        assert!(job.is_terminal());
    }

    #[test]
    fn test_job_spec_builder() {
        let spec = JobSpec::builder()
            .name("test-job")
            .image("ubuntu:22.04")
            .command(vec!["echo".to_string()])
            .build()
            .unwrap();

        assert_eq!(spec.name, "test-job");
        assert_eq!(spec.image, "ubuntu:22.04");
    }
}
