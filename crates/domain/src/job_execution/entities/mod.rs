//! Job Aggregate Root
//!
//! The Job entity is the aggregate root for job management.
//! It encapsulates the entire lifecycle of a job.

use super::value_objects::{ExecutionContext, JobSpec};
use crate::shared_kernel::{JobId, JobState, ProviderId};

/// Job aggregate root
///
/// Represents a job in the system with its current state and execution context
#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub spec: JobSpec,
    pub state: JobState,
    pub execution_context: Option<ExecutionContext>,
}

impl Job {
    /// Creates a new job in Pending state
    pub fn new(id: JobId, spec: JobSpec) -> Result<Self, crate::shared_kernel::DomainError> {
        if spec.name.trim().is_empty() {
            return Err(crate::shared_kernel::DomainError::Validation(
                "Job name cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            id,
            spec,
            state: JobState::Pending,
            execution_context: None,
        })
    }

    /// Transitions job to Running state
    pub fn submit_to_provider(&mut self, provider_id: ProviderId, context: ExecutionContext) {
        self.state = JobState::Running;
        self.execution_context = Some(context);
    }

    /// Marks job as completed
    pub fn complete(&mut self) {
        self.state = JobState::Completed;
    }

    /// Marks job as failed
    pub fn fail(&mut self) {
        self.state = JobState::Failed;
    }

    /// Cancels the job
    pub fn cancel(&mut self) {
        self.state = JobState::Cancelled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_execution::value_objects::JobSpec;
    use crate::shared_kernel::{JobId, JobState};

    #[test]
    fn test_create_job_with_valid_spec() {
        let job_id = JobId::new("job-123".to_string());
        let spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string()],
            vec!["hello".to_string()],
        );

        let job = Job::new(job_id.clone(), spec).unwrap();

        assert_eq!(job.id, job_id);
        assert_eq!(job.state, JobState::Pending);
        assert!(job.execution_context.is_none());
    }

    #[test]
    fn test_create_job_with_empty_name_fails() {
        let job_id = JobId::new("job-123".to_string());
        let spec = JobSpec::new(
            "".to_string(),
            vec!["echo".to_string()],
            vec!["hello".to_string()],
        );

        let result = Job::new(job_id, spec);

        assert!(result.is_err());
    }

    #[test]
    fn test_submit_to_provider_transitions_to_running() {
        let job_id = JobId::new("job-123".to_string());
        let spec = JobSpec::new("test-job".to_string(), vec!["echo".to_string()], vec![]);

        let mut job = Job::new(job_id, spec).unwrap();
        let provider_id = crate::shared_kernel::ProviderId::new("provider-1".to_string());
        let context = crate::job_execution::value_objects::ExecutionContext::new(
            JobId::new("job-123".to_string()),
            provider_id.clone(),
            "exec-123".to_string(),
        );

        job.submit_to_provider(provider_id, context);

        assert_eq!(job.state, JobState::Running);
        assert!(job.execution_context.is_some());
    }

    #[test]
    fn test_complete_transitions_to_completed() {
        let job_id = JobId::new("job-123".to_string());
        let spec = JobSpec::new("test-job".to_string(), vec!["echo".to_string()], vec![]);

        let mut job = Job::new(job_id, spec).unwrap();

        job.complete();

        assert_eq!(job.state, JobState::Completed);
    }

    #[test]
    fn test_fail_transitions_to_failed() {
        let job_id = JobId::new("job-123".to_string());
        let spec = JobSpec::new("test-job".to_string(), vec!["echo".to_string()], vec![]);

        let mut job = Job::new(job_id, spec).unwrap();

        job.fail();

        assert_eq!(job.state, JobState::Failed);
    }

    #[test]
    fn test_cancel_transitions_to_cancelled() {
        let job_id = JobId::new("job-123".to_string());
        let spec = JobSpec::new("test-job".to_string(), vec!["echo".to_string()], vec![]);

        let mut job = Job::new(job_id, spec).unwrap();

        job.cancel();

        assert_eq!(job.state, JobState::Cancelled);
    }
}
