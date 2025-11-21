//! Provider traits module
//!
//! This module contains all the trait definitions for worker providers.

use crate::ProviderError;
use crate::models::{ProviderCapabilities, ProviderType, WorkerConfig, WorkerHandle, WorkerStatus};
use async_trait::async_trait;
use hodei_shared_types::worker_messages::WorkerId;

/// Core worker provider trait
#[async_trait]
pub trait WorkerProvider: Send + Sync {
    /// Create a new worker
    async fn create_worker(&self, config: &WorkerConfig) -> Result<WorkerHandle, ProviderError>;

    /// Start an existing worker
    async fn start_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Stop a running worker gracefully
    async fn stop_worker(&self, worker_id: &WorkerId, graceful: bool) -> Result<(), ProviderError>;

    /// Delete a worker completely
    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Get worker status
    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerStatus, ProviderError>;

    /// Get provider capabilities
    async fn get_capabilities(&self) -> Result<ProviderCapabilities, ProviderError>;

    /// Scale workers up or down
    async fn scale_workers(
        &self,
        worker_type: &str,
        target_count: usize,
    ) -> Result<Vec<WorkerId>, ProviderError>;

    /// Get provider type
    fn get_provider_type(&self) -> ProviderType;
}
