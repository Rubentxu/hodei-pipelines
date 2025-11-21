//! Job Entity implementation
//!
//! This module contains the Job aggregate root with state management
//! and business logic according to DDD principles.

use hodei_shared_types::job_definitions::{ExecResult, JobId, JobSpec, JobState};
use hodei_shared_types::{CorrelationId, DateTime, DomainError, TenantId, Utc};

/// Job aggregate root
#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub spec: JobSpec,
    pub state: JobState,
    pub attempts: u8,
    pub created_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub correlation_id: CorrelationId,
    pub tenant_id: TenantId,
}

#[derive(Debug)]
pub enum JobEvent {
    JobRequested {
        job_id: JobId,
        spec: JobSpec,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
    },
    JobScheduled {
        job_id: JobId,
    },
    JobStarted {
        job_id: JobId,
    },
    JobCompleted {
        job_id: JobId,
        result: ExecResult,
    },
    JobFailed {
        job_id: JobId,
        error: String,
        retryable: bool,
    },
    JobCancelled {
        job_id: JobId,
    },
}

impl Job {
    /// Create a new Job with PENDING state
    pub fn create(
        spec: JobSpec,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
    ) -> Result<Self, DomainError> {
        // Validate spec before creation
        spec.validate()?;

        Ok(Self {
            id: JobId::new(),
            spec,
            state: JobState::from(JobState::PENDING.to_string()),
            attempts: 1,
            created_at: Some(Utc::now()),
            started_at: None,
            completed_at: None,
            correlation_id,
            tenant_id,
        })
    }

    /// Transition job to SCHEDULED state
    pub fn schedule(&mut self) -> Result<Vec<JobEvent>, DomainError> {
        self.ensure_transition_valid(&JobState::from(JobState::SCHEDULED.to_string()))?;
        self.state = JobState::from(JobState::SCHEDULED.to_string());

        Ok(vec![JobEvent::JobScheduled {
            job_id: self.id.clone(),
        }])
    }

    /// Transition job to RUNNING state
    pub fn start(&mut self) -> Result<Vec<JobEvent>, DomainError> {
        self.ensure_transition_valid(&JobState::from(JobState::RUNNING.to_string()))?;
        self.state = JobState::from(JobState::RUNNING.to_string());
        self.started_at = Some(Utc::now());

        Ok(vec![JobEvent::JobStarted {
            job_id: self.id.clone(),
        }])
    }

    /// Transition job to SUCCESS state
    pub fn complete(&mut self) -> Result<Vec<JobEvent>, DomainError> {
        self.ensure_transition_valid(&JobState::from(JobState::SUCCESS.to_string()))?;
        self.state = JobState::from(JobState::SUCCESS.to_string());
        self.completed_at = Some(Utc::now());

        let result = ExecResult {
            exit_code: 0,
            stdout: None,
            stderr: None,
        };

        Ok(vec![JobEvent::JobCompleted {
            job_id: self.id.clone(),
            result,
        }])
    }

    /// Transition job to FAILED state
    pub fn fail(&mut self, error: String, retryable: bool) -> Result<Vec<JobEvent>, DomainError> {
        self.ensure_transition_valid(&JobState::from(JobState::FAILED.to_string()))?;
        self.state = JobState::from(JobState::FAILED.to_string());
        self.completed_at = Some(Utc::now());

        Ok(vec![JobEvent::JobFailed {
            job_id: self.id.clone(),
            error,
            retryable,
        }])
    }

    /// Transition job to CANCELLED state
    pub fn cancel(&mut self) -> Result<Vec<JobEvent>, DomainError> {
        self.ensure_transition_valid(&JobState::from(JobState::CANCELLED.to_string()))?;
        self.state = JobState::from(JobState::CANCELLED.to_string());

        Ok(vec![JobEvent::JobCancelled {
            job_id: self.id.clone(),
        }])
    }

    /// Retry a failed job (transition to PENDING)
    pub fn retry(&mut self) -> Result<Vec<JobEvent>, DomainError> {
        // Check max retries
        if self.attempts >= self.spec.retries {
            return Err(DomainError::Validation(format!(
                "max retries ({}) exceeded",
                self.spec.retries
            )));
        }

        self.ensure_transition_valid(&JobState::from(JobState::PENDING.to_string()))?;
        self.state = JobState::from(JobState::PENDING.to_string());
        self.attempts += 1;

        Ok(vec![JobEvent::JobRequested {
            job_id: self.id.clone(),
            spec: self.spec.clone(),
            correlation_id: self.correlation_id.clone(),
            tenant_id: self.tenant_id.clone(),
        }])
    }

    /// Manual state transition with validation
    pub fn transition_to(&mut self, target_state: &JobState) -> Result<Vec<JobEvent>, DomainError> {
        self.ensure_transition_valid(target_state)?;
        self.state = target_state.clone();

        match target_state.as_str() {
            JobState::RUNNING => self.started_at = Some(Utc::now()),
            JobState::SUCCESS | JobState::FAILED | JobState::CANCELLED => {
                self.completed_at = Some(Utc::now())
            }
            _ => {}
        }

        Ok(vec![])
    }

    /// Validate state transition
    fn ensure_transition_valid(&self, target: &JobState) -> Result<(), DomainError> {
        if !self.state.can_transition_to(target) {
            return Err(DomainError::invalid_state_transition(
                self.state.as_str(),
                target.as_str(),
            ));
        }
        Ok(())
    }
}
