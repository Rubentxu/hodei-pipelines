//! JobSpec validation specifications using the Specification Pattern

use crate::job_definitions::JobSpec;
use crate::specifications::{Specification, SpecificationResult, SpecificationResultBuilder};
use std::fmt;

/// Specification to ensure job name is not empty
#[derive(Debug, Clone)]
pub struct JobNameNotEmptySpec;

impl JobNameNotEmptySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JobNameNotEmptySpec {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Specification to ensure job image is not empty
#[derive(Debug, Clone)]
pub struct JobImageNotEmptySpec;

impl JobImageNotEmptySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JobImageNotEmptySpec {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Specification to ensure job command is not empty
#[derive(Debug, Clone)]
pub struct JobCommandNotEmptySpec;

impl JobCommandNotEmptySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JobCommandNotEmptySpec {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Specification to ensure job timeout is positive
#[derive(Debug, Clone)]
pub struct JobTimeoutPositiveSpec;

impl JobTimeoutPositiveSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JobTimeoutPositiveSpec {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Composite specification for complete JobSpec validation
pub struct ValidJobSpec;

impl ValidJobSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValidJobSpec {
    fn default() -> Self {
        Self::new()
    }
}

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

impl fmt::Display for ValidJobSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ValidJobSpec")
    }
}

/// Validate JobSpec and return detailed SpecificationResult
pub fn validate_job_spec(candidate: &JobSpec) -> SpecificationResult {
    let mut result = SpecificationResultBuilder::new(candidate);

    // Validate name
    if candidate.name.trim().is_empty() {
        result.add_error("Job name cannot be empty");
    }

    // Validate image
    if candidate.image.trim().is_empty() {
        result.add_error("Job image cannot be empty");
    }

    // Validate command
    if candidate.command.is_empty() {
        result.add_error("Job command cannot be empty");
    }

    // Validate timeout
    if candidate.timeout_ms <= 0 {
        result.add_error("Job timeout must be greater than 0");
    }

    result.build()
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
