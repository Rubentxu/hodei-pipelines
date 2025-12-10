//! Shared traits for Provider-as-Worker pattern
//!
//! Defines the unified interface implemented by all providers that act as workers

use crate::shared_kernel::{
    DomainResult, ExecutionStatus, JobId, JobResult, ProviderCapabilities, ProviderId,
};

/// Unified trait for all providers that act as workers
///
/// Each provider that executes jobs must implement this trait.
/// This enables polymorphism and decoupling between domain and infrastructure.
#[async_trait::async_trait]
pub trait ProviderWorker: Send + Sync {
    /// Submits a job for execution on this provider
    ///
    /// # Arguments
    /// * `job_id` - Unique ID of the job to execute
    /// * `spec` - Job specification with all necessary parameters
    ///
    /// # Returns
    /// Unique execution ID for tracking the job execution
    async fn submit_job(
        &self,
        job_id: &JobId,
        spec: &crate::job_execution::JobSpec,
    ) -> DomainResult<String>;

    /// Gets the current execution status of a job
    ///
    /// # Arguments
    /// * `execution_id` - Execution ID returned by `submit_job`
    ///
    /// # Returns
    /// Current status of the execution
    async fn get_execution_status(&self, execution_id: &str) -> DomainResult<ExecutionStatus>;

    /// Gets the result of a completed job
    ///
    /// # Arguments
    /// * `execution_id` - Execution ID returned by `submit_job`
    ///
    /// # Returns
    /// Result of the execution if available
    async fn get_job_result(&self, execution_id: &str) -> DomainResult<Option<JobResult>>;

    /// Cancels a running job
    ///
    /// # Arguments
    /// * `execution_id` - Execution ID returned by `submit_job`
    ///
    /// # Returns
    /// `()` if cancellation was successful
    async fn cancel_job(&self, execution_id: &str) -> DomainResult<()>;

    /// Gets the current capabilities of the provider
    ///
    /// # Returns
    /// Information about available resources, supported job types, etc.
    async fn get_capabilities(&self) -> DomainResult<ProviderCapabilities>;
}

/// Builder trait for creating provider instances with configuration
#[async_trait::async_trait]
pub trait ProviderWorkerBuilder: Send + Sync {
    /// Type of provider built by this builder
    type Worker: ProviderWorker;

    /// Sets the provider ID
    fn with_provider_id(self, provider_id: ProviderId) -> Self;

    /// Builds the provider instance
    async fn build(&self) -> DomainResult<Self::Worker>;
}
