//! Job Specification validation rules using the Specification Pattern
//!
//! This module provides reusable, composable validation rules for JobSpec,
//! Worker, and other domain entities using the Specification Pattern.

use crate::specifications::Specification;
use crate::DomainError;
use hodei_shared_types::{JobSpec, JobId, Worker, ResourceQuota, WorkerCapabilities};
use std::fmt;

/// Specification that validates JobSpec name is not empty
pub struct JobNameNotEmptySpec;

impl Specification<JobSpec> for JobNameNotEmptySpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        !candidate.name.trim().is_empty()
    }
}

impl fmt::Display for JobNameNotEmptySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job name must not be empty")
    }
}

/// Specification that validates JobSpec image is not empty
pub struct JobImageNotEmptySpec;

impl Specification<JobSpec> for JobImageNotEmptySpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        !candidate.image.trim().is_empty()
    }
}

impl fmt::Display for JobImageNotEmptySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job image must not be empty")
    }
}

/// Specification that validates JobSpec command is not empty
pub struct JobCommandNotEmptySpec;

impl Specification<JobSpec> for JobCommandNotEmptySpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        !candidate.command.is_empty()
    }
}

impl fmt::Display for JobCommandNotEmptySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job command must not be empty")
    }
}

/// Specification that validates JobSpec timeout is greater than 0
pub struct JobTimeoutPositiveSpec;

impl Specification<JobSpec> for JobTimeoutPositiveSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        candidate.timeout_ms > 0
    }
}

impl fmt::Display for JobTimeoutPositiveSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job timeout must be greater than 0")
    }
}

/// Specification that validates JobSpec retries are within acceptable range
pub struct JobRetriesValidSpec {
    pub max_retries: u8,
}

impl JobRetriesValidSpec {
    pub fn new(max_retries: u8) -> Self {
        Self { max_retries }
    }
}

impl Specification<JobSpec> for JobRetriesValidSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        candidate.retries <= self.max_retries
    }
}

impl fmt::Display for JobRetriesValidSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Job retries must not exceed {}", self.max_retries)
    }
}

/// Composite specification for complete JobSpec validation
pub struct ValidJobSpec;

impl ValidJobSpec {
    pub fn new() -> Self {
        Self
    }

    /// Get all validation specifications for a JobSpec
    pub fn get_specifications(&self) -> Vec<Box<dyn Specification<JobSpec> + Send + Sync>> {
        vec![
            Box::new(JobNameNotEmptySpec),
            Box::new(JobImageNotEmptySpec),
            Box::new(JobCommandNotEmptySpec),
            Box::new(JobTimeoutPositiveSpec),
            Box::new(JobRetriesValidSpec { max_retries: 5 }),
        ]
    }
}

impl Default for ValidJobSpec {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<JobSpec> for ValidJobSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        let specs = self.get_specifications();
        specs.iter().all(|spec| spec.is_satisfied_by(candidate))
    }
}

impl fmt::Display for ValidJobSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JobSpec must be valid")
    }
}

/// Validate JobSpec using specification pattern
pub fn validate_job_spec(job_spec: &JobSpec) -> Result<(), DomainError> {
    let validator = ValidJobSpec::new();
    if validator.is_satisfied_by(job_spec) {
        Ok(())
    } else {
        Err(DomainError::Validation(
            "JobSpec does not satisfy validation specifications".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_job_spec() -> JobSpec {
        JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota {
                cpu_m: 1000,
                memory_mb: 1024,
                gpu: None,
            },
            timeout_ms: 300000,
            retries: 0,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        }
    }

    #[test]
    fn test_job_name_not_empty_spec() {
        let spec = JobNameNotEmptySpec;
        let mut job = create_test_job_spec();
        assert!(spec.is_satisfied_by(&job));

        job.name = "".to_string();
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_job_image_not_empty_spec() {
        let spec = JobImageNotEmptySpec;
        let mut job = create_test_job_spec();
        assert!(spec.is_satisfied_by(&job));

        job.image = "".to_string();
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_job_command_not_empty_spec() {
        let spec = JobCommandNotEmptySpec;
        let mut job = create_test_job_spec();
        assert!(spec.is_satisfied_by(&job));

        job.command = vec![];
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_job_timeout_positive_spec() {
        let spec = JobTimeoutPositiveSpec;
        let mut job = create_test_job_spec();
        assert!(spec.is_satisfied_by(&job));

        job.timeout_ms = 0;
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_valid_job_spec() {
        let validator = ValidJobSpec::new();
        let job = create_test_job_spec();
        assert!(validator.is_satisfied_by(&job));
    }

    #[test]
    fn test_composite_spec() {
        // Test AND composition
        let spec = JobNameNotEmptySpec.and(JobImageNotEmptySpec);
        let mut job = create_test_job_spec();
        assert!(spec.is_satisfied_by(&job));

        job.name = "".to_string();
        assert!(!spec.is_satisfied_by(&job));
    }
}
