//! Worker Provider Abstraction Layer (US-018)
//!
//! This module provides a unified abstraction for worker providers
//! supporting multiple execution backends (Kubernetes, Docker, AWS ECS, etc.).
//!
//! Features:
//! - Provider abstraction with trait-based interfaces
//! - Provider factory for automatic instantiation
//! - Capability detection and feature matrix
//! - Multi-provider coordination
//! - Error handling with retry and circuit breaker patterns
//! - Performance optimization with connection pooling

pub mod factory;
pub mod models;
pub mod traits;

pub use factory::ProviderFactory;
pub use models::*;
pub use traits::WorkerProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_provider_factory_creation() {
        // RED: Test that we can create a provider factory
        let factory = ProviderFactory::new();

        // GREEN: Just check the factory was created
        assert!(factory.is_ok());
    }

    #[tokio::test]
    async fn test_provider_creation_without_config() {
        // RED: Test that provider creation fails without configuration
        let factory = ProviderFactory::new().unwrap();
        let result = factory
            .create_provider(ProviderType::Kubernetes, None)
            .await;

        // GREEN: Check it returns the expected error type
        assert!(matches!(result, Err(ProviderError::ConfigurationError(_))));
    }

    #[tokio::test]
    async fn test_provider_creation_with_config() {
        // RED: Test that we can create a provider with configuration
        let factory = ProviderFactory::new().unwrap();
        let config = ProviderConfig {
            provider_type: ProviderType::Kubernetes,
            connection_string: "test".to_string(),
            credentials: None,
            options: HashMap::new(),
        };

        let result = factory
            .create_provider(ProviderType::Kubernetes, Some(config))
            .await;

        // GREEN: Check provider was created successfully
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.get_provider_type(), ProviderType::Kubernetes);
    }

    #[tokio::test]
    async fn test_mock_worker_provider_operations() {
        // RED: Test basic worker operations on mock provider
        let provider = MockWorkerProvider::new(ProviderType::Kubernetes);

        // Create a test worker config
        use hodei_shared_types::worker_messages::WorkerId;

        let worker_id = WorkerId::new();
        let worker_config = WorkerConfig {
            worker_id: worker_id.clone(),
            image: "test-image:latest".to_string(),
            resources: ResourceRequirements {
                cpu_cores: 1.0,
                memory_bytes: 1024 * 1024 * 1024,
                ephemeral_storage_bytes: None,
            },
            environment: HashMap::new(),
            secrets: Vec::new(),
            health_checks: Vec::new(),
            scaling_config: ScalingConfiguration {
                min_replicas: 1,
                max_replicas: 5,
                target_cpu_utilization: Some(70),
            },
        };

        // Create worker
        let handle = provider.create_worker(&worker_config).await.unwrap();
        assert_eq!(handle.worker_id, worker_id);

        // Get status
        let status = provider.get_worker_status(&worker_id).await.unwrap();
        assert_eq!(status.state, WorkerState::Creating);

        // Stop worker
        let result = provider.stop_worker(&worker_id, true).await;
        assert!(result.is_ok());

        // Delete worker
        let result = provider.delete_worker(&worker_id).await;
        assert!(result.is_ok());

        // Verify worker is deleted
        let status_result = provider.get_worker_status(&worker_id).await;
        assert!(status_result.is_err());
    }

    #[tokio::test]
    async fn test_provider_capabilities() {
        // RED: Test that provider capabilities can be retrieved
        let provider = MockWorkerProvider::new(ProviderType::Kubernetes);

        // GREEN: Check capabilities are returned
        let capabilities = provider.get_capabilities().await.unwrap();
        assert_eq!(capabilities.auto_scaling, false);
        assert_eq!(capabilities.health_checks, false);
    }

    #[tokio::test]
    async fn test_worker_config_validation() {
        // RED: Test worker config validation
        use hodei_shared_types::worker_messages::WorkerId;

        let worker_id = WorkerId::new();

        // Valid config should pass
        let valid_config = WorkerConfig {
            worker_id: worker_id.clone(),
            image: "test-image:latest".to_string(),
            resources: ResourceRequirements {
                cpu_cores: 1.0,
                memory_bytes: 1024 * 1024 * 1024,
                ephemeral_storage_bytes: None,
            },
            environment: HashMap::new(),
            secrets: Vec::new(),
            health_checks: Vec::new(),
            scaling_config: ScalingConfiguration {
                min_replicas: 1,
                max_replicas: 5,
                target_cpu_utilization: Some(70),
            },
        };
        assert!(valid_config.validate().is_ok());

        // Invalid config (empty image) should fail
        let invalid_config = WorkerConfig {
            worker_id,
            image: "".to_string(),
            resources: ResourceRequirements {
                cpu_cores: 1.0,
                memory_bytes: 1024 * 1024 * 1024,
                ephemeral_storage_bytes: None,
            },
            environment: HashMap::new(),
            secrets: Vec::new(),
            health_checks: Vec::new(),
            scaling_config: ScalingConfiguration {
                min_replicas: 1,
                max_replicas: 5,
                target_cpu_utilization: Some(70),
            },
        };
        assert!(invalid_config.validate().is_err());

        // Invalid config (too low CPU) should fail
        let invalid_config2 = WorkerConfig {
            worker_id: WorkerId::new(),
            image: "test-image:latest".to_string(),
            resources: ResourceRequirements {
                cpu_cores: 0.01,
                memory_bytes: 1024 * 1024 * 1024,
                ephemeral_storage_bytes: None,
            },
            environment: HashMap::new(),
            secrets: Vec::new(),
            health_checks: Vec::new(),
            scaling_config: ScalingConfiguration {
                min_replicas: 1,
                max_replicas: 5,
                target_cpu_utilization: Some(70),
            },
        };
        assert!(invalid_config2.validate().is_err());
    }

    #[tokio::test]
    async fn test_provider_display_implementations() {
        // RED: Test Display implementations for enums
        assert_eq!(format!("{}", ProviderType::Kubernetes), "kubernetes");
        assert_eq!(format!("{}", ProviderType::Docker), "docker");
        assert_eq!(format!("{}", ProviderType::AwsEcs), "aws-ecs");
        assert_eq!(
            format!("{}", ProviderType::AzureContainerInstances),
            "azure-container-instances"
        );
        assert_eq!(
            format!("{}", ProviderType::Custom("custom-provider".to_string())),
            "custom-custom-provider"
        );

        assert_eq!(format!("{}", WorkerState::Creating), "Creating");
        assert_eq!(format!("{}", WorkerState::Running), "Running");
        assert_eq!(format!("{}", WorkerState::Failed), "Failed");
    }

    #[tokio::test]
    async fn test_provider_error_types() {
        // RED: Test all provider error types
        let error1 = ProviderError::ConfigurationError("test error".to_string());
        assert!(matches!(error1, ProviderError::ConfigurationError(_)));

        let error2 = ProviderError::WorkerOperationFailed("op failed".to_string());
        assert!(matches!(error2, ProviderError::WorkerOperationFailed(_)));

        let error3 = ProviderError::CapabilityNotSupported("cap not supported".to_string());
        assert!(matches!(error3, ProviderError::CapabilityNotSupported(_)));

        let error4 = ProviderError::Timeout;
        assert!(matches!(error4, ProviderError::Timeout));
    }

    #[tokio::test]
    async fn test_worker_metadata() {
        // RED: Test WorkerMetadata functionality
        let metadata = WorkerMetadata::new()
            .with_label("env".to_string(), "test".to_string())
            .with_label("version".to_string(), "1.0".to_string());

        assert_eq!(metadata.labels.get("env").unwrap(), &"test".to_string());
        assert_eq!(metadata.labels.get("version").unwrap(), &"1.0".to_string());
    }

    #[tokio::test]
    async fn test_worker_handle() {
        // RED: Test WorkerHandle creation
        use hodei_shared_types::worker_messages::WorkerId;

        let worker_id = WorkerId::new();
        let handle = WorkerHandle::new(worker_id.clone(), ProviderType::Kubernetes);

        assert_eq!(handle.worker_id, worker_id);
        assert_eq!(handle.provider_type, ProviderType::Kubernetes);
        assert_eq!(handle.metadata.labels.len(), 0);
    }

    #[tokio::test]
    async fn test_provider_capabilities_creation() {
        // RED: Test ProviderCapabilities creation
        let caps = ProviderCapabilities::new();
        assert_eq!(caps.auto_scaling, false);
        assert_eq!(caps.health_checks, false);

        let caps2 = ProviderCapabilities {
            auto_scaling: true,
            health_checks: true,
            volumes: true,
            config_maps: true,
            secrets: true,
            network_policies: false,
            multi_cluster: false,
        };

        assert_eq!(caps2.auto_scaling, true);
        assert_eq!(caps2.health_checks, true);
        assert_eq!(caps2.network_policies, false);
    }

    #[tokio::test]
    async fn test_scale_workers() {
        // RED: Test scale_workers operation
        let provider = MockWorkerProvider::new(ProviderType::Kubernetes);

        // Scale operation should succeed but return empty vector (no workers)
        let result = provider.scale_workers("test-worker", 5).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_provider_factory_detect_best_provider() {
        // RED: Test best provider detection
        let factory = ProviderFactory::new().unwrap();
        let best_provider = factory.detect_best_provider().await.unwrap();

        assert_eq!(best_provider, ProviderType::Kubernetes);
    }

    #[tokio::test]
    async fn test_unsupported_provider_types() {
        // RED: Test that unsupported providers return errors
        let factory = ProviderFactory::new().unwrap();

        let aws_result = factory
            .create_provider(
                ProviderType::AwsEcs,
                Some(ProviderConfig {
                    provider_type: ProviderType::AwsEcs,
                    connection_string: "test".to_string(),
                    credentials: None,
                    options: HashMap::new(),
                }),
            )
            .await;

        assert!(matches!(
            aws_result,
            Err(ProviderError::ConfigurationError(_))
        ));

        let azure_result = factory
            .create_provider(
                ProviderType::AzureContainerInstances,
                Some(ProviderConfig {
                    provider_type: ProviderType::AzureContainerInstances,
                    connection_string: "test".to_string(),
                    credentials: None,
                    options: HashMap::new(),
                }),
            )
            .await;

        assert!(matches!(
            azure_result,
            Err(ProviderError::ConfigurationError(_))
        ));
    }
}
