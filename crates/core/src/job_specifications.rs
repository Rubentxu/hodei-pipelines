//! JobSpec validation specifications using the Specification Pattern

use crate::job_definitions::JobSpec;
use crate::specifications::{
    AndSpec, Specification, SpecificationResult, SpecificationResultBuilder,
};
use std::fmt;

/// Specification to ensure job name is not empty
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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
pub struct ValidJobSpec {
    pub name: JobNameNotEmptySpec,
    pub image: JobImageNotEmptySpec,
    pub command: JobCommandNotEmptySpec,
    pub timeout: JobTimeoutPositiveSpec,
}

impl ValidJobSpec {
    pub fn new() -> Self {
        Self {
            name: JobNameNotEmptySpec::new(),
            image: JobImageNotEmptySpec::new(),
            command: JobCommandNotEmptySpec::new(),
            timeout: JobTimeoutPositiveSpec::new(),
        }
    }

    /// Create a composite specification using AND logic
    pub fn with_all() -> impl Specification<JobSpec> {
        JobNameNotEmptySpec::new()
            .and(JobImageNotEmptySpec::new())
            .and(JobCommandNotEmptySpec::new())
            .and(JobTimeoutPositiveSpec::new())
    }

    /// Create a composite specification using custom composition
    #[allow(clippy::type_complexity)]
    pub fn composed() -> AndSpec<
        AndSpec<
            AndSpec<JobNameNotEmptySpec, JobImageNotEmptySpec, JobSpec>,
            JobCommandNotEmptySpec,
            JobSpec,
        >,
        JobTimeoutPositiveSpec,
        JobSpec,
    > {
        JobNameNotEmptySpec::new()
            .and(JobImageNotEmptySpec::new())
            .and(JobCommandNotEmptySpec::new())
            .and(JobTimeoutPositiveSpec::new())
    }
}

impl Default for ValidJobSpec {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<JobSpec> for ValidJobSpec {
    fn is_satisfied_by(&self, candidate: &JobSpec) -> bool {
        self.name.is_satisfied_by(candidate)
            && self.image.is_satisfied_by(candidate)
            && self.command.is_satisfied_by(candidate)
            && self.timeout.is_satisfied_by(candidate)
    }
}

impl fmt::Display for ValidJobSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ValidJobSpec (composable)")
    }
}

/// Validate JobSpec and return detailed SpecificationResult
pub fn validate_job_spec(candidate: &JobSpec) -> SpecificationResult {
    use crate::specifications::Specification;

    let mut result = SpecificationResultBuilder::new(candidate);

    let specs = ValidJobSpec::new();

    // Validate name using specification
    if !specs.name.is_satisfied_by(candidate) {
        result.add_error(format!("{}", specs.name));
    }

    // Validate image using specification
    if !specs.image.is_satisfied_by(candidate) {
        result.add_error(format!("{}", specs.image));
    }

    // Validate command using specification
    if !specs.command.is_satisfied_by(candidate) {
        result.add_error(format!("{}", specs.command));
    }

    // Validate timeout using specification
    if !specs.timeout.is_satisfied_by(candidate) {
        result.add_error(format!("{}", specs.timeout));
    }

    result.build()
}

/// Validate JobSpec using composable specifications
pub fn validate_job_spec_composable(candidate: &JobSpec) -> SpecificationResult {
    let mut result = SpecificationResultBuilder::new(candidate);

    let spec = ValidJobSpec::with_all();

    // Validate all at once using composable spec
    if !spec.is_satisfied_by(candidate) {
        // Check each component individually for detailed error reporting
        if !JobNameNotEmptySpec::new().is_satisfied_by(candidate) {
            result.add_error(format!("{}", JobNameNotEmptySpec::new()));
        }
        if !JobImageNotEmptySpec::new().is_satisfied_by(candidate) {
            result.add_error(format!("{}", JobImageNotEmptySpec::new()));
        }
        if !JobCommandNotEmptySpec::new().is_satisfied_by(candidate) {
            result.add_error(format!("{}", JobCommandNotEmptySpec::new()));
        }
        if !JobTimeoutPositiveSpec::new().is_satisfied_by(candidate) {
            result.add_error(format!("{}", JobTimeoutPositiveSpec::new()));
        }
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
        let spec = ValidJobSpec::new();
        let job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));
    }

    // ===== TDD Tests: Composable Specifications =====

    #[test]
    fn test_composable_spec_with_all() {
        let spec = ValidJobSpec::with_all();
        let job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_composable_spec_composed() {
        let spec = ValidJobSpec::composed();
        let job = create_valid_job_spec();

        assert!(spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_and_operator_for_custom_specs() {
        // Create a custom specification combining name and image validation
        let custom_spec = JobNameNotEmptySpec::new().and(JobImageNotEmptySpec::new());

        let mut job = create_valid_job_spec();
        assert!(custom_spec.is_satisfied_by(&job));

        // Test with empty name
        job.name = "".to_string();
        assert!(!custom_spec.is_satisfied_by(&job));

        // Test with empty image
        job.name = "test-job".to_string();
        job.image = "".to_string();
        assert!(!custom_spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_or_operator_for_alternative_specs() {
        // Either a valid command OR a timeout > 5 minutes
        let alternative_spec = JobCommandNotEmptySpec::new().or(JobTimeoutPositiveSpec::new());

        let mut job = create_valid_job_spec();
        assert!(alternative_spec.is_satisfied_by(&job));

        // Remove command but keep timeout > 300000ms
        job.command = Vec::new();
        job.timeout_ms = 400000;
        assert!(alternative_spec.is_satisfied_by(&job));

        // Both empty - should fail
        job.timeout_ms = 0;
        assert!(!alternative_spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_not_operator_for_negation() {
        // Command must NOT be empty (positive test)
        let has_command = JobCommandNotEmptySpec::new();
        let no_command = has_command.clone().not();

        let mut job = create_valid_job_spec();
        assert!(has_command.is_satisfied_by(&job));
        assert!(!no_command.is_satisfied_by(&job));

        // Empty command
        job.command = Vec::new();
        assert!(!has_command.is_satisfied_by(&job));
        assert!(no_command.is_satisfied_by(&job));
    }

    #[test]
    fn test_complex_composition_with_and_or_not() {
        // More realistic: name is valid AND (command exists OR timeout > 0)
        let complex_spec = JobNameNotEmptySpec::new()
            .and(JobCommandNotEmptySpec::new().or(JobTimeoutPositiveSpec::new()));

        let mut job = create_valid_job_spec();
        job.timeout_ms = 300000; // Exactly 5 minutes
        assert!(complex_spec.is_satisfied_by(&job));

        // No command but sufficient timeout
        job.command = Vec::new();
        job.timeout_ms = 400000;
        assert!(complex_spec.is_satisfied_by(&job));

        // Both missing
        job.timeout_ms = 0;
        assert!(!complex_spec.is_satisfied_by(&job));
    }

    #[test]
    fn test_validate_job_spec_accumulates_errors() {
        let invalid_job = JobSpec {
            name: "".to_string(),
            image: "".to_string(),
            command: Vec::new(),
            resources: ResourceQuota::default(),
            timeout_ms: 0,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: Vec::new(),
        };

        let result = validate_job_spec(&invalid_job);
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 4); // All fields should have errors
        assert!(result.errors.iter().any(|e| e.contains("name")));
        assert!(result.errors.iter().any(|e| e.contains("image")));
        assert!(result.errors.iter().any(|e| e.contains("command")));
        assert!(result.errors.iter().any(|e| e.contains("timeout")));
    }

    #[test]
    fn test_validate_job_spec_composable() {
        let invalid_job = JobSpec {
            name: "".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 0,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: Vec::new(),
        };

        let result = validate_job_spec_composable(&invalid_job);
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 2); // Name and timeout
    }

    #[test]
    fn test_valid_job_passes_all_validations() {
        let valid_job = create_valid_job_spec();

        let result = validate_job_spec(&valid_job);
        assert!(result.is_valid());

        let result_composable = validate_job_spec_composable(&valid_job);
        assert!(result_composable.is_valid());
    }

    #[test]
    fn test_specifications_are_reusable() {
        let name_spec = JobNameNotEmptySpec::new();
        let image_spec = JobImageNotEmptySpec::new();

        let job1 = JobSpec {
            name: "valid-name".to_string(),
            image: "valid-image".to_string(),
            command: vec!["cmd".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 1000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job2 = JobSpec {
            name: "".to_string(),
            image: "".to_string(),
            command: vec!["cmd".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 1000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: Vec::new(),
        };

        // Same spec instances can validate multiple jobs
        assert!(name_spec.is_satisfied_by(&job1));
        assert!(!name_spec.is_satisfied_by(&job2));

        assert!(image_spec.is_satisfied_by(&job1));
        assert!(!image_spec.is_satisfied_by(&job2));
    }

    #[test]
    fn test_default_valid_job_spec() {
        let default_job = JobSpec {
            name: "default-job".to_string(),
            image: "busybox:latest".to_string(),
            command: vec!["sh".to_string(), "-c".to_string(), "echo hello".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: Vec::new(),
        };

        let spec = ValidJobSpec::new();
        assert!(spec.is_satisfied_by(&default_job));
    }
}
