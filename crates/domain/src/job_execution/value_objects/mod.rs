//! Value Objects for Job Execution
//!
//! Immutable value objects that represent job specifications and execution contexts

use crate::shared_kernel::{JobResult, ProviderId, ResourceRequirements};
use serde::{Deserialize, Serialize};

/// Job specification value object
///
/// Contains all the information needed to execute a job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSpec {
    pub name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub resources: Option<ResourceRequirements>,
}

impl JobSpec {
    /// Creates a new job specification
    pub fn new(name: String, command: Vec<String>, args: Vec<String>) -> Self {
        Self {
            name,
            command,
            args,
            env: std::collections::HashMap::new(),
            resources: None,
        }
    }
}

/// Execution context value object
///
/// Tracks the execution of a job on a specific provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub job_id: crate::shared_kernel::JobId,
    pub provider_id: ProviderId,
    pub execution_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub result: Option<JobResult>,
}

impl ExecutionContext {
    /// Creates a new execution context
    pub fn new(
        job_id: crate::shared_kernel::JobId,
        provider_id: ProviderId,
        execution_id: String,
    ) -> Self {
        Self {
            job_id,
            provider_id,
            execution_id,
            started_at: chrono::Utc::now(),
            result: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_job_spec_creation() {
        let spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string()],
            vec!["hello".to_string()],
        );

        assert_eq!(spec.name, "test-job");
        assert_eq!(spec.command, vec!["echo"]);
        assert_eq!(spec.args, vec!["hello"]);
        assert!(spec.env.is_empty());
        assert!(spec.resources.is_none());
    }

    #[test]
    fn test_job_spec_with_all_fields() {
        let mut spec = JobSpec::new(
            "test-job".to_string(),
            vec!["python".to_string(), "script.py".to_string()],
            vec![],
        );

        spec.env.insert("ENV_VAR".to_string(), "value".to_string());
        spec.resources = Some(crate::shared_kernel::ResourceRequirements {
            memory_mb: Some(1024),
            cpu_cores: Some(2.0),
            timeout_seconds: Some(300),
        });

        assert_eq!(spec.name, "test-job");
        assert_eq!(spec.command, vec!["python", "script.py"]);
        assert_eq!(spec.args, Vec::<String>::new());
        assert_eq!(spec.env.get("ENV_VAR"), Some(&"value".to_string()));
        assert!(spec.resources.is_some());

        let resources = spec.resources.unwrap();
        assert_eq!(resources.memory_mb, Some(1024));
        assert_eq!(resources.cpu_cores, Some(2.0));
        assert_eq!(resources.timeout_seconds, Some(300));
    }
}
