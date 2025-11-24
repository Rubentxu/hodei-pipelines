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

    #[test]
    fn test_job_id_new_generates_unique() {
        let id1 = JobId::new();
        let id2 = JobId::new();
        assert_ne!(id1, id2);
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
}
