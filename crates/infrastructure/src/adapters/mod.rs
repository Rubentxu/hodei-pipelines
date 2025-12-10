//! Provider Adapters
//!
//! Infrastructure adapters for different provider implementations

use domain::job_execution::JobSpec;
use domain::shared_kernel::ProviderWorker;
use domain::shared_kernel::{
    DomainResult, ExecutionStatus, JobId, JobResult, ProviderCapabilities, ProviderId,
};

pub struct DockerProviderAdapter {
    provider_id: ProviderId,
}

impl DockerProviderAdapter {
    pub fn new(provider_id: ProviderId) -> Self {
        Self { provider_id }
    }
}

#[async_trait::async_trait]
impl ProviderWorker for DockerProviderAdapter {
    async fn submit_job(&self, job_id: &JobId, _spec: &JobSpec) -> DomainResult<String> {
        Ok(format!("docker-exec-{}", job_id.to_string()))
    }

    async fn get_execution_status(&self, _execution_id: &str) -> DomainResult<ExecutionStatus> {
        Ok(ExecutionStatus::Running)
    }

    async fn get_job_result(&self, _execution_id: &str) -> DomainResult<Option<JobResult>> {
        Ok(Some(JobResult {
            success: true,
            output: Some("Job completed".to_string()),
            error: None,
            execution_time_ms: 1000,
        }))
    }

    async fn cancel_job(&self, _execution_id: &str) -> DomainResult<()> {
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
