//! Execution Coordination Services

use crate::shared_kernel::DomainResult;

/// Execution Coordinator Service
///
/// Orchestrates job execution across providers
pub struct ExecutionCoordinator;

impl ExecutionCoordinator {
    pub fn new() -> Self {
        Self
    }

    pub async fn coordinate_execution(
        &self,
        job: &super::super::job_execution::Job,
        provider: &super::super::provider_management::Provider,
    ) -> DomainResult<String> {
        Ok(format!(
            "Job {} will be executed by provider {}",
            job.id.to_string(),
            provider.name
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_execution::{Job, JobSpec};
    use crate::provider_management::entities::ProviderStatus;
    use crate::provider_management::value_objects::ProviderConfig;
    use crate::provider_management::Provider;
    use crate::shared_kernel::{JobId, ProviderCapabilities, ProviderId, ProviderType};

    #[tokio::test]
    async fn test_coordinate_execution() {
        let coordinator = ExecutionCoordinator::new();

        let job_id = JobId::new("job-123".to_string());
        let spec = crate::job_execution::value_objects::JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string()],
            vec![],
        );
        let job = crate::job_execution::Job::new(job_id, spec).unwrap();

        let provider = Provider::new(
            ProviderId::new("provider-1".to_string()),
            "Docker Provider".to_string(),
            ProviderType::Docker,
            crate::shared_kernel::ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["docker".to_string()],
                memory_limit_mb: Some(4096),
                cpu_limit: Some(2.0),
            },
            crate::provider_management::value_objects::ProviderConfig::new(
                "http://localhost:2375".to_string(),
            ),
        );

        let result = coordinator
            .coordinate_execution(&job, &provider)
            .await
            .unwrap();

        assert!(result.contains("Job job-123"));
        assert!(result.contains("provider Docker Provider"));
    }
}
