//! Job Domain Entity
//!
//! This module contains the Job aggregate root that uses value objects
//! from the Shared Kernel (shared-types crate).
//!
//! NOTE: Value objects (JobId, JobState, JobSpec, ResourceQuota) are
//! defined in shared-types crate to avoid duplication across bounded contexts.

pub use hodei_shared_types::{DomainError, ExecResult, JobId, JobSpec, JobState, ResourceQuota};

use crate::Result;
use serde::{Deserialize, Serialize};

/// Job aggregate root
///
/// This entity encapsulates the business logic for job lifecycle management
/// and maintains consistency of job state transitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Create a new job with PENDING state
    ///
    /// # Errors
    /// Returns `DomainError::Validation` if the job spec is invalid
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

    /// Transition job to SCHEDULED state
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
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

    /// Transition job to RUNNING state
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
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
        self.started_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Transition job to SUCCESS state (terminal)
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
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
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Transition job to FAILED state
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
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
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Transition job to CANCELLED state (terminal)
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if transition is invalid
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
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Check if job is in PENDING state
    pub fn is_pending(&self) -> bool {
        self.state.as_str() == JobState::PENDING
    }

    /// Check if job is in RUNNING state
    pub fn is_running(&self) -> bool {
        self.state.as_str() == JobState::RUNNING
    }

    /// Check if job is in a terminal state (SUCCESS, FAILED, or CANCELLED)
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Retry a failed job by transitioning back to PENDING
    ///
    /// # Errors
    /// Returns `DomainError::InvalidStateTransition` if job is not in FAILED state
    pub fn retry(&mut self) -> Result<()> {
        if self.state.as_str() != JobState::FAILED {
            return Err(DomainError::InvalidStateTransition {
                from: self.state.as_str().to_string(),
                to: JobState::PENDING.to_string(),
            });
        }

        let new_state = JobState::new(JobState::PENDING.to_string())?;
        self.state = new_state;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Helper function to create a valid JobSpec
    fn create_valid_job_spec() -> JobSpec {
        JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        }
    }

    // ===== JobId Tests =====

    #[test]
    fn test_job_id_new_generates_unique() {
        let id1 = JobId::new();
        let id2 = JobId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_job_id_display() {
        let id = JobId::new();
        let id_str = format!("{}", id);
        assert!(!id_str.is_empty());
    }

    #[test]
    fn test_job_id_from_string() {
        let id = JobId::new();
        let id_str = id.to_string();
        // JobId doesn't implement FromStr, so we just check it can be formatted
        assert!(!id_str.is_empty());
    }

    // ===== JobState Transition Tests =====

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
    fn test_job_state_valid_transitions() {
        let pending = JobState::new(JobState::PENDING.to_string()).unwrap();
        let scheduled = JobState::new(JobState::SCHEDULED.to_string()).unwrap();
        let running = JobState::new(JobState::RUNNING.to_string()).unwrap();
        let success = JobState::new(JobState::SUCCESS.to_string()).unwrap();
        let failed = JobState::new(JobState::FAILED.to_string()).unwrap();
        let cancelled = JobState::new(JobState::CANCELLED.to_string()).unwrap();

        assert!(pending.can_transition_to(&scheduled));
        assert!(scheduled.can_transition_to(&running));
        assert!(running.can_transition_to(&success));
        assert!(running.can_transition_to(&failed));
        assert!(running.can_transition_to(&cancelled));
    }

    #[test]
    fn test_job_state_invalid_transitions() {
        let pending = JobState::new(JobState::PENDING.to_string()).unwrap();
        let running = JobState::new(JobState::RUNNING.to_string()).unwrap();
        let success = JobState::new(JobState::SUCCESS.to_string()).unwrap();

        assert!(!running.can_transition_to(&pending));
        assert!(!success.can_transition_to(&running));
        // Note: FAILED -> PENDING transition is allowed for retry logic
    }

    #[test]
    fn test_job_state_terminal_states() {
        let success = JobState::new(JobState::SUCCESS.to_string()).unwrap();
        let failed = JobState::new(JobState::FAILED.to_string()).unwrap();
        let cancelled = JobState::new(JobState::CANCELLED.to_string()).unwrap();
        let pending = JobState::new(JobState::PENDING.to_string()).unwrap();

        assert!(success.is_terminal());
        assert!(failed.is_terminal());
        assert!(cancelled.is_terminal());
        assert!(!pending.is_terminal());
    }

    // ===== Job Lifecycle Tests =====

    #[test]
    fn test_job_new_pending() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec).unwrap();

        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.is_pending());
        assert!(!job.is_running());
        assert!(!job.is_terminal());
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());
    }

    #[test]
    fn test_job_schedule_transition() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        assert!(job.schedule().is_ok());
        assert_eq!(job.state.as_str(), JobState::SCHEDULED);
        assert!(!job.is_pending());
        assert!(!job.is_terminal());
    }

    #[test]
    fn test_job_start_transition() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.schedule().unwrap();

        assert!(job.start().is_ok());
        assert_eq!(job.state.as_str(), JobState::RUNNING);
        assert!(job.is_running());
        assert!(job.started_at.is_some());
        assert!(!job.is_terminal());
    }

    #[test]
    fn test_job_complete_transition() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();

        assert!(job.complete().is_ok());
        assert_eq!(job.state.as_str(), JobState::SUCCESS);
        assert!(job.is_terminal());
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_fail_transition() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();

        assert!(job.fail().is_ok());
        assert_eq!(job.state.as_str(), JobState::FAILED);
        assert!(job.is_terminal());
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_cancel_transition() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        assert!(job.cancel().is_ok());
        assert_eq!(job.state.as_str(), JobState::CANCELLED);
        assert!(job.is_terminal());
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_retry() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();
        job.fail().unwrap();

        // Can retry from failed state
        assert!(job.retry().is_ok());
        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.is_pending());
        assert!(!job.is_terminal());
    }

    #[test]
    fn test_job_retry_from_invalid_state() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.schedule().unwrap();

        // Cannot retry from scheduled state
        assert!(job.retry().is_err());
    }

    #[test]
    fn test_job_invalid_state_transitions() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        // Cannot go directly from PENDING to RUNNING
        assert!(job.start().is_err());

        // Cannot complete from PENDING
        assert!(job.complete().is_err());

        // Cannot fail from PENDING
        assert!(job.fail().is_err());
    }

    #[test]
    fn test_job_state_consistency() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        // Complete valid transition path
        assert!(job.schedule().is_ok());
        assert!(job.start().is_ok());
        assert!(job.complete().is_ok());

        // Verify timestamps
        assert!(job.created_at <= job.updated_at);
        assert!(job.started_at.is_some());
        assert!(job.completed_at.is_some());
        assert!(job.started_at <= job.completed_at);
    }

    #[test]
    fn test_job_with_description() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.description = Some("Test description".to_string());

        assert_eq!(job.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_job_with_tenant() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();
        job.tenant_id = Some("tenant-123".to_string());

        assert_eq!(job.tenant_id, Some("tenant-123".to_string()));
    }

    #[test]
    fn test_job_full_lifecycle() {
        let spec = create_valid_job_spec();
        let mut job = Job::new(JobId::new(), spec).unwrap();

        // Full lifecycle: PENDING -> SCHEDULED -> RUNNING -> SUCCESS
        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.schedule().is_ok());
        assert_eq!(job.state.as_str(), JobState::SCHEDULED);
        assert!(job.start().is_ok());
        assert_eq!(job.state.as_str(), JobState::RUNNING);
        assert!(job.complete().is_ok());
        assert_eq!(job.state.as_str(), JobState::SUCCESS);
        assert!(job.is_terminal());
    }

    #[test]
    fn test_job_timeout_and_retries() {
        let mut spec = create_valid_job_spec();
        spec.timeout_ms = 60000;
        spec.retries = 2;

        let job = Job::new(JobId::new(), spec).unwrap();

        assert_eq!(job.spec.timeout_ms, 60000);
        assert_eq!(job.spec.retries, 2);
    }

    #[test]
    fn test_job_environment_variables() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec).unwrap();

        assert!(job.spec.env.is_empty());
    }

    #[test]
    fn test_job_secret_refs() {
        let spec = create_valid_job_spec();
        let job = Job::new(JobId::new(), spec).unwrap();

        assert!(job.spec.secret_refs.is_empty());
    }
}
