//! Provider Adapters
//!
//! Infrastructure adapters for different provider implementations

use domain::job_execution::JobSpec;
use domain::shared_kernel::ProviderWorker;
use domain::DomainError;
use domain::shared_kernel::{
    DomainResult, ExecutionStatus, JobId, JobResult, ProviderCapabilities, ProviderId,
};

/// Docker Provider Adapter
///
/// Implements the ProviderWorker trait for Docker containers
pub struct DockerProviderAdapter {
    provider_id: ProviderId,
    docker_socket: Option<String>,
}

impl DockerProviderAdapter {
    /// Create a new Docker provider adapter
    pub fn new(provider_id: ProviderId) -> Self {
        Self {
            provider_id,
            docker_socket: None,
        }
    }

    /// Set custom Docker socket path
    pub fn with_socket(mut self, socket_path: String) -> Self {
        self.docker_socket = Some(socket_path);
        self
    }
}

#[async_trait::async_trait]
impl ProviderWorker for DockerProviderAdapter {
    async fn submit_job(&self, job_id: &JobId, spec: &JobSpec) -> DomainResult<String> {
        // TODO: Implement actual Docker container creation
        // For now, return a mock execution ID
        let execution_id = format!("docker-exec-{}", job_id.to_string());

        // Validate job spec
        if spec.command.is_empty() {
            return Err(DomainError::Validation(
                "Job spec must contain at least one command".to_string(),
            ));
        }

        Ok(execution_id)
    }

    async fn get_execution_status(&self, execution_id: &str) -> DomainResult<ExecutionStatus> {
        // TODO: Check Docker API for actual status
        // For now, return Running as placeholder
        Ok(ExecutionStatus::Running)
    }

    async fn get_job_result(&self, execution_id: &str) -> DomainResult<Option<JobResult>> {
        // TODO: Get logs and exit code from Docker API
        // For now, return a mock result
        Ok(Some(JobResult {
            success: true,
            output: Some("Job completed".to_string()),
            error: None,
            execution_time_ms: 1000,
        }))
    }

    async fn cancel_job(&self, execution_id: &str) -> DomainResult<()> {
        // TODO: Stop Docker container
        Ok(())
    }

    async fn get_capabilities(&self) -> DomainResult<ProviderCapabilities> {
        Ok(ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["docker".to_string()],
            memory_limit_mb: Some(4096),
            cpu_limit: Some(2.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::shared_kernel::{JobId, ProviderId};

    #[tokio::test]
    async fn test_submit_job_with_valid_spec() {
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let job_id = JobId::new("job-123".to_string());
        let spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string(), "hello".to_string()],
            vec![],
        );

        let result = adapter.submit_job(&job_id, &spec).await.unwrap();
        assert!(result.starts_with("docker-exec-job-123"));
    }

    #[tokio::test]
    async fn test_submit_job_with_empty_commands() {
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let job_id = JobId::new("job-456".to_string());
        let spec = JobSpec::new("test-job".to_string(), vec![], vec![]);

        let result = adapter.submit_job(&job_id, &spec).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_execution_status() {
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let status = adapter.get_execution_status("docker-exec-job-123").await.unwrap();
        assert_eq!(status, ExecutionStatus::Running);
    }

    #[tokio::test]
    async fn test_get_job_result() {
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let result = adapter.get_job_result("docker-exec-job-123").await.unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let result = adapter.cancel_job("docker-exec-job-123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_capabilities() {
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let capabilities = adapter.get_capabilities().await.unwrap();
        assert_eq!(capabilities.max_concurrent_jobs, 10);
        assert!(capabilities.supported_job_types.contains(&"docker".to_string()));
    }

    #[tokio::test]
    async fn test_docker_provider_with_custom_socket() {
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let adapter = DockerProviderAdapter::new(provider_id)
            .with_socket("/var/run/docker-custom.sock".to_string());

        let capabilities = adapter.get_capabilities().await.unwrap();
        assert_eq!(capabilities.max_concurrent_jobs, 10);
    }
}
