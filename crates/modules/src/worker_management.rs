//! Worker Management Module
//!
//! This module provides the application layer (use cases) for managing
//! dynamic workers across different infrastructure providers.

use async_trait::async_trait;
use hodei_adapters::DefaultProviderFactory;
use hodei_core::{Worker, WorkerId};
use hodei_ports::ProviderFactoryTrait;
use hodei_ports::worker_provider::{ProviderConfig, ProviderError, WorkerProvider};
use tracing::{info, warn};

/// Worker management service
#[derive(Debug)]
pub struct WorkerManagementService {
    provider: Box<dyn WorkerProvider + Send + Sync>,
}

impl WorkerManagementService {
    pub fn new(provider: Box<dyn WorkerProvider + Send + Sync>) -> Self {
        Self { provider }
    }

    /// Create a new dynamic worker
    pub async fn provision_worker(
        &self,
        image: String,
        cpu_cores: u32,
        memory_mb: u64,
    ) -> Result<Worker, WorkerManagementError> {
        let worker_id = WorkerId::new();
        let config = ProviderConfig::docker(format!("worker-{}", worker_id));

        info!(
            worker_id = %worker_id,
            image = %image,
            "Provisioning new worker"
        );

        let worker = self
            .provider
            .create_worker(worker_id.clone(), config)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(
            worker_id = %worker.id,
            container_id = ?worker.metadata.get("container_id"),
            "Worker provisioned successfully"
        );

        Ok(worker)
    }

    /// Stop a worker
    pub async fn stop_worker(
        &self,
        worker_id: &WorkerId,
        graceful: bool,
    ) -> Result<(), WorkerManagementError> {
        info!(worker_id = %worker_id, graceful = graceful, "Stopping worker");

        self.provider
            .stop_worker(worker_id, graceful)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(worker_id = %worker_id, "Worker stopped successfully");
        Ok(())
    }

    /// Delete a worker
    pub async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), WorkerManagementError> {
        info!(worker_id = %worker_id, "Deleting worker");

        self.provider
            .delete_worker(worker_id)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(worker_id = %worker_id, "Worker deleted successfully");
        Ok(())
    }

    /// Get worker status
    pub async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<hodei_shared_types::WorkerStatus, WorkerManagementError> {
        let status = self
            .provider
            .get_worker_status(worker_id)
            .await
            .map_err(WorkerManagementError::Provider)?;

        Ok(status)
    }

    /// List all workers
    pub async fn list_workers(&self) -> Result<Vec<WorkerId>, WorkerManagementError> {
        let workers = self
            .provider
            .list_workers()
            .await
            .map_err(WorkerManagementError::Provider)?;

        Ok(workers)
    }

    /// Get provider capabilities
    pub async fn get_provider_capabilities(
        &self,
    ) -> Result<hodei_ports::worker_provider::ProviderCapabilities, WorkerManagementError> {
        let capabilities = self
            .provider
            .capabilities()
            .await
            .map_err(WorkerManagementError::Provider)?;

        Ok(capabilities)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WorkerManagementError {
    #[error("Provider error: {0}")]
    Provider(ProviderError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl WorkerManagementError {
    pub fn internal<T: Into<String>>(msg: T) -> Self {
        Self::Internal(msg.into())
    }
}

/// Create a default worker management service with Docker provider
pub async fn create_default_worker_management_service()
-> Result<WorkerManagementService, WorkerManagementError> {
    let config = ProviderConfig::docker("docker-provider".to_string());
    let factory = DefaultProviderFactory::new();
    let provider = factory
        .create_provider(config)
        .await
        .map_err(WorkerManagementError::Provider)?;

    Ok(WorkerManagementService::new(provider))
}

/// Create a worker management service with Kubernetes provider
pub async fn create_kubernetes_worker_management_service(
    namespace: String,
) -> Result<WorkerManagementService, WorkerManagementError> {
    let mut config = ProviderConfig::kubernetes("k8s-provider".to_string());
    config.namespace = Some(namespace);

    let factory = DefaultProviderFactory::new();
    let provider = factory
        .create_provider(config)
        .await
        .map_err(WorkerManagementError::Provider)?;

    Ok(WorkerManagementService::new(provider))
}
