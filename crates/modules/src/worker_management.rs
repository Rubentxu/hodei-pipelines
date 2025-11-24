//! Worker Management Module
//!
//! This module provides the application layer (use cases) for managing
//! dynamic workers across different infrastructure providers.

use async_trait::async_trait;
use hodei_adapters::{DefaultProviderFactory, RegistrationConfig, WorkerRegistrationAdapter};
use hodei_core::{Worker, WorkerId};
use hodei_ports::ProviderFactoryTrait;
use hodei_ports::scheduler_port::SchedulerPort;
use hodei_ports::worker_provider::{ProviderConfig, ProviderError, WorkerProvider};
use hodei_ports::{WorkerRegistrationError, WorkerRegistrationPort};
use tracing::{error, info, warn};

/// Configuration for worker management service
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerManagementConfig {
    pub registration_enabled: bool,
    pub registration_max_retries: u32,
}

impl Default for WorkerManagementConfig {
    fn default() -> Self {
        Self {
            registration_enabled: true,
            registration_max_retries: 3,
        }
    }
}

/// Worker management service
#[derive(Debug)]
pub struct WorkerManagementService<P, S>
where
    P: WorkerProvider + Send + Sync,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    provider: Box<P>,
    registration_adapter: Option<WorkerRegistrationAdapter<S>>,
    config: WorkerManagementConfig,
}

impl<P, S> WorkerManagementService<P, S>
where
    P: WorkerProvider + Send + Sync + Clone + 'static,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    /// Create new service without registration (backwards compatible)
    pub fn new(provider: P, config: WorkerManagementConfig) -> Self {
        Self {
            provider: Box::new(provider),
            registration_adapter: None,
            config,
        }
    }

    /// Create new service with registration adapter
    pub fn new_with_registration(
        provider: P,
        registration_adapter: WorkerRegistrationAdapter<S>,
        config: WorkerManagementConfig,
    ) -> Self {
        Self {
            provider: Box::new(provider),
            registration_adapter: Some(registration_adapter),
            config,
        }
    }

    /// Create a new dynamic worker with default Docker provider
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

        // Attempt registration if enabled
        if let (Some(adapter), true) =
            (&self.registration_adapter, self.config.registration_enabled)
        {
            if let Err(error) = adapter.register_worker(&worker).await {
                warn!(
                    worker_id = %worker.id,
                    error = %error,
                    "Worker provisioned but registration failed"
                );
            } else {
                info!(worker_id = %worker.id, "Worker registered successfully");
            }
        }

        Ok(worker)
    }

    /// Create a new dynamic worker with custom configuration
    pub async fn provision_worker_with_config(
        &self,
        mut config: ProviderConfig,
        cpu_cores: u32,
        memory_mb: u64,
    ) -> Result<Worker, WorkerManagementError> {
        let worker_id = WorkerId::new();

        info!(
            worker_id = %worker_id,
            provider_type = %config.provider_type.as_str(),
            "Provisioning new worker with custom config"
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

        // Attempt registration if enabled
        if let (Some(adapter), true) =
            (&self.registration_adapter, self.config.registration_enabled)
        {
            if let Err(error) = adapter.register_worker(&worker).await {
                warn!(
                    worker_id = %worker.id,
                    error = %error,
                    "Worker provisioned but registration failed"
                );
            } else {
                info!(worker_id = %worker.id, "Worker registered successfully");
            }
        }

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
pub async fn create_default_worker_management_service<P, S>(
    provider: P,
) -> Result<WorkerManagementService<P, S>, WorkerManagementError>
where
    P: WorkerProvider + Send + Sync + Clone + 'static,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    Ok(WorkerManagementService::new(
        provider,
        WorkerManagementConfig::default(),
    ))
}

/// Create a worker management service with Kubernetes provider
pub async fn create_kubernetes_worker_management_service<P, S>(
    provider: P,
) -> Result<WorkerManagementService<P, S>, WorkerManagementError>
where
    P: WorkerProvider + Send + Sync + Clone + 'static,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    Ok(WorkerManagementService::new(
        provider,
        WorkerManagementConfig::default(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use hodei_ports::worker_provider::ProviderCapabilities;
    use hodei_shared_types::WorkerCapabilities;

    // Mock implementation for testing
    #[derive(Debug, Clone)]
    pub struct MockWorkerProvider {
        pub workers: Vec<Worker>,
        pub should_fail: bool,
    }

    impl MockWorkerProvider {
        pub fn new() -> Self {
            Self {
                workers: Vec::new(),
                should_fail: false,
            }
        }

        pub fn with_worker(mut self, worker: Worker) -> Self {
            self.workers.push(worker);
            self
        }

        pub fn with_failure(mut self, should_fail: bool) -> Self {
            self.should_fail = should_fail;
            self
        }
    }

    #[async_trait]
    impl WorkerProvider for MockWorkerProvider {
        fn provider_type(&self) -> hodei_ports::worker_provider::ProviderType {
            hodei_ports::worker_provider::ProviderType::Docker
        }

        fn name(&self) -> &str {
            "mock-provider"
        }

        async fn capabilities(&self) -> Result<ProviderCapabilities, ProviderError> {
            Ok(ProviderCapabilities {
                supports_auto_scaling: true,
                supports_health_checks: true,
                supports_volumes: false,
                max_workers: Some(1000),
                estimated_provision_time_ms: 1000,
            })
        }

        async fn create_worker(
            &self,
            worker_id: WorkerId,
            _config: ProviderConfig,
        ) -> Result<Worker, ProviderError> {
            if self.should_fail {
                return Err(ProviderError::Provider("Mock error".to_string()));
            }

            let worker_name = format!("worker-{}", worker_id);
            Ok(Worker::new(
                worker_id,
                worker_name,
                WorkerCapabilities::new(4, 8192),
            ))
        }

        async fn get_worker_status(
            &self,
            worker_id: &WorkerId,
        ) -> Result<hodei_shared_types::WorkerStatus, ProviderError> {
            Ok(hodei_shared_types::WorkerStatus::create_with_status(
                "IDLE".to_string(),
            ))
        }

        async fn stop_worker(
            &self,
            _worker_id: &WorkerId,
            _graceful: bool,
        ) -> Result<(), ProviderError> {
            Ok(())
        }

        async fn delete_worker(&self, _worker_id: &WorkerId) -> Result<(), ProviderError> {
            Ok(())
        }

        async fn list_workers(&self) -> Result<Vec<WorkerId>, ProviderError> {
            Ok(self.workers.iter().map(|w| w.id.clone()).collect())
        }
    }

    // Mock scheduler for testing
    #[derive(Debug, Clone)]
    pub struct MockSchedulerPort;

    #[async_trait]
    impl SchedulerPort for MockSchedulerPort {
        async fn register_worker(
            &self,
            _worker: &Worker,
        ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
            Ok(())
        }

        async fn unregister_worker(
            &self,
            _worker_id: &WorkerId,
        ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
            Ok(())
        }

        async fn get_registered_workers(
            &self,
        ) -> Result<Vec<WorkerId>, hodei_ports::scheduler_port::SchedulerError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_provision_worker_with_registration() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service =
            WorkerManagementService::new_with_registration(mock_provider, adapter, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        assert!(result.is_ok(), "Expected successful provision");
    }

    #[tokio::test]
    async fn test_provision_worker_registration_failure_not_rollback() {
        let mock_provider = MockWorkerProvider::new().with_failure(true);
        let config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service =
            WorkerManagementService::new_with_registration(mock_provider, adapter, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        // Provisioning succeeds even if registration fails
        assert!(result.is_ok(), "Provisioning should succeed");
    }

    #[tokio::test]
    async fn test_provision_worker_without_registration() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig::default();

        let service: WorkerManagementService<MockWorkerProvider, MockSchedulerPort> =
            WorkerManagementService::new(mock_provider, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        assert!(result.is_ok(), "Expected successful provision");
    }

    #[tokio::test]
    async fn test_provision_worker_with_config_registration() {
        let mock_provider = MockWorkerProvider::new();
        let mut config = ProviderConfig::docker("test-provider".to_string());
        let worker_management_config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service = WorkerManagementService::new_with_registration(
            mock_provider,
            adapter,
            worker_management_config,
        );

        let result = service.provision_worker_with_config(config, 4, 8192).await;

        assert!(result.is_ok(), "Expected successful provision with config");
    }

    #[tokio::test]
    async fn test_provision_worker_registration_disabled() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig {
            registration_enabled: false,
            registration_max_retries: 0,
        };

        let service: WorkerManagementService<MockWorkerProvider, MockSchedulerPort> =
            WorkerManagementService::new(mock_provider, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        assert!(
            result.is_ok(),
            "Expected successful provision without registration"
        );
    }

    #[tokio::test]
    async fn test_worker_management_config_default() {
        let config = WorkerManagementConfig::default();

        assert_eq!(config.registration_enabled, true);
        assert_eq!(config.registration_max_retries, 3);
    }

    #[tokio::test]
    async fn test_worker_management_config_clone() {
        let config = WorkerManagementConfig::default();
        let cloned = config.clone();

        assert_eq!(config, cloned);
    }

    #[tokio::test]
    async fn test_worker_management_all_operations() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service =
            WorkerManagementService::new_with_registration(mock_provider.clone(), adapter, config);

        // Provision worker
        let worker = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await
            .unwrap();

        // Stop worker
        let result = service.stop_worker(&worker.id, true).await;
        assert!(result.is_ok(), "Expected successful stop");

        // Delete worker
        let result = service.delete_worker(&worker.id).await;
        assert!(result.is_ok(), "Expected successful delete");

        // Get provider capabilities
        let capabilities = service.get_provider_capabilities().await.unwrap();
        assert_eq!(capabilities.supports_auto_scaling, true);
    }
}
