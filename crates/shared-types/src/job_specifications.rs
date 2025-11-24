//! JobSpec validation specifications

use super::specifications::Specification;
use crate::job_definitions::JobSpec;

/// Specification to ensure job name is not empty
pub struct JobNameNotEmptySpec;

impl Specification<JobSpec> for JobNameNotEmptySpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        !candidate.name.trim().is_empty()
    }
}

/// Specification to ensure job image is not empty
pub struct JobImageNotEmptySpec;

impl Specification<JobSpec> for JobImageNotEmptySpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        !candidate.image.trim().is_empty()
    }
}

/// Specification to ensure job command is not empty
pub struct JobCommandNotEmptySpec;

impl Specification<JobSpec> for JobCommandNotEmptySpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        !candidate.command.is_empty()
    }
}

/// Specification to ensure job timeout is positive
pub struct JobTimeoutPositiveSpec;

impl Specification<JobSpec> for JobTimeoutPositiveSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        candidate.timeout_ms > 0
    }
}

/// Composite specification for valid JobSpec
pub struct ValidJobSpec;

impl Specification<JobSpec> for ValidJobSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        let name_spec = JobNameNotEmptySpec;
        let image_spec = JobImageNotEmptySpec;
        let command_spec = JobCommandNotEmptySpec;
        let timeout_spec = JobTimeoutPositiveSpec;

        name_spec.is_satisfied_by(candidate)
            && image_spec.is_satisfied_by(candidate)
            && command_spec.is_satisfied_by(candidate)
            && timeout_spec.is_satisfied_by(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_definitions::ResourceQuota;

    fn create_valid_job_spec() -> JobSpec {
        JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: Vec::new(),
        }
    }

    #[test]
    fn test_name_not_empty_spec() {
        let spec = JobNameNotEmptySpec;
        let mut job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));

        job.name = "".to_string();
        assert!(!spec.is_satisfied_by(&job));

        job.name = "   ".to_string();
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_image_not_empty_spec() {
        let spec = JobImageNotEmptySpec;
        let mut job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));

        job.image = "".to_string();
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_command_not_empty_spec() {
        let spec = JobCommandNotEmptySpec;
        let mut job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));

        job.command = Vec::new();
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_timeout_positive_spec() {
        let spec = JobTimeoutPositiveSpec;
        let mut job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));

        job.timeout_ms = 0;
        assert!(!spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_valid_job_spec() {
        let spec = ValidJobSpec;
        let job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));
    }
}
