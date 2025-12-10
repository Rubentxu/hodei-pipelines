//! Core types shared across all bounded contexts
//!
//! Contains primitive value objects and enums that are fundamental
//! to the domain model

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Unique identifier for a job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobId(pub String);

impl JobId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

impl FromStr for JobId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderId(pub String);

impl ProviderId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of provider (Docker, Kubernetes, Lambda, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    Docker,
    Kubernetes,
    Lambda,
    AzureVM,
    GCPFunctions,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Docker => write!(f, "Docker"),
            ProviderType::Kubernetes => write!(f, "Kubernetes"),
            ProviderType::Lambda => write!(f, "Lambda"),
            ProviderType::AzureVM => write!(f, "AzureVM"),
            ProviderType::GCPFunctions => write!(f, "GCPFunctions"),
        }
    }
}

/// Current state of a job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobState::Pending => write!(f, "Pending"),
            JobState::Running => write!(f, "Running"),
            JobState::Completed => write!(f, "Completed"),
            JobState::Failed => write!(f, "Failed"),
            JobState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Status of a job execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Queued => write!(f, "Queued"),
            ExecutionStatus::Running => write!(f, "Running"),
            ExecutionStatus::Succeeded => write!(f, "Succeeded"),
            ExecutionStatus::Failed => write!(f, "Failed"),
            ExecutionStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Result of a completed job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub max_concurrent_jobs: u32,
    pub supported_job_types: Vec<String>,
    pub memory_limit_mb: Option<u64>,
    pub cpu_limit: Option<f64>,
}

impl PartialEq for ProviderCapabilities {
    fn eq(&self, other: &Self) -> bool {
        self.max_concurrent_jobs == other.max_concurrent_jobs
            && self.supported_job_types == other.supported_job_types
            && self.memory_limit_mb == other.memory_limit_mb
            && self.cpu_limit.map(|x| (x * 1000.0).round() / 1000.0)
                == other.cpu_limit.map(|x| (x * 1000.0).round() / 1000.0)
    }
}

impl Eq for ProviderCapabilities {}

/// Resource requirements for a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub memory_mb: Option<u64>,
    pub cpu_cores: Option<f64>,
    pub timeout_seconds: Option<u64>,
}

impl PartialEq for ResourceRequirements {
    fn eq(&self, other: &Self) -> bool {
        self.memory_mb == other.memory_mb
            && self.cpu_cores.map(|x| (x * 1000.0).round() / 1000.0)
                == other.cpu_cores.map(|x| (x * 1000.0).round() / 1000.0)
            && self.timeout_seconds == other.timeout_seconds
    }
}

impl Eq for ResourceRequirements {}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    #[test]
    fn test_job_id_creation() {
        let job_id = JobId::new("job-123".to_string());
        assert_eq!(job_id.to_string(), "job-123");
    }

    #[test]
    fn test_job_id_from_str() {
        let job_id = JobId::from_str("job-123").unwrap();
        assert_eq!(job_id.to_string(), "job-123");
    }

    #[test]
    fn test_job_id_display() {
        let job_id = JobId::new("job-456".to_string());
        assert_eq!(format!("{}", job_id), "job-456");
    }

    #[test]
    fn test_provider_id_creation() {
        let provider_id = ProviderId::new("provider-1".to_string());
        assert_eq!(provider_id.to_string(), "provider-1");
    }

    #[test]
    fn test_provider_id_display() {
        let provider_id = ProviderId::new("provider-2".to_string());
        assert_eq!(format!("{}", provider_id), "provider-2");
    }

    #[test]
    fn test_job_state_variants() {
        assert_eq!(format!("{}", JobState::Pending), "Pending");
        assert_eq!(format!("{}", JobState::Running), "Running");
        assert_eq!(format!("{}", JobState::Completed), "Completed");
        assert_eq!(format!("{}", JobState::Failed), "Failed");
        assert_eq!(format!("{}", JobState::Cancelled), "Cancelled");
    }

    #[test]
    fn test_execution_status_variants() {
        assert_eq!(format!("{}", ExecutionStatus::Queued), "Queued");
        assert_eq!(format!("{}", ExecutionStatus::Running), "Running");
        assert_eq!(format!("{}", ExecutionStatus::Succeeded), "Succeeded");
        assert_eq!(format!("{}", ExecutionStatus::Failed), "Failed");
        assert_eq!(format!("{}", ExecutionStatus::Cancelled), "Cancelled");
    }

    #[test]
    fn test_provider_type_variants() {
        assert_eq!(format!("{}", ProviderType::Docker), "Docker");
        assert_eq!(format!("{}", ProviderType::Kubernetes), "Kubernetes");
        assert_eq!(format!("{}", ProviderType::Lambda), "Lambda");
        assert_eq!(format!("{}", ProviderType::AzureVM), "AzureVM");
        assert_eq!(format!("{}", ProviderType::GCPFunctions), "GCPFunctions");
    }

    #[test]
    fn test_job_result_creation() {
        let result = JobResult {
            success: true,
            output: Some("Success".to_string()),
            error: None,
            execution_time_ms: 1500,
        };

        assert!(result.success);
        assert_eq!(result.output, Some("Success".to_string()));
        assert_eq!(result.execution_time_ms, 1500);
    }

    #[test]
    fn test_provider_capabilities_creation() {
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["docker".to_string(), "k8s".to_string()],
            memory_limit_mb: Some(4096),
            cpu_limit: Some(2.0),
        };

        assert_eq!(capabilities.max_concurrent_jobs, 10);
        assert_eq!(capabilities.supported_job_types.len(), 2);
        assert_eq!(capabilities.memory_limit_mb, Some(4096));
        assert_eq!(capabilities.cpu_limit, Some(2.0));
    }

    #[test]
    fn test_resource_requirements_creation() {
        let requirements = ResourceRequirements {
            memory_mb: Some(1024),
            cpu_cores: Some(2.0),
            timeout_seconds: Some(300),
        };

        assert_eq!(requirements.memory_mb, Some(1024));
        assert_eq!(requirements.cpu_cores, Some(2.0));
        assert_eq!(requirements.timeout_seconds, Some(300));
    }

    #[test]
    fn test_job_result_equality_with_floating_point() {
        let result1 = JobResult {
            success: true,
            output: Some("Test".to_string()),
            error: None,
            execution_time_ms: 1000,
        };

        let result2 = JobResult {
            success: true,
            output: Some("Test".to_string()),
            error: None,
            execution_time_ms: 1000,
        };

        assert_eq!(result1, result2);
    }
}
