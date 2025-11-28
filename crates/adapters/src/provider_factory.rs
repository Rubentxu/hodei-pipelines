//! Provider Factory
//!
//! This module provides the concrete implementation of the ProviderFactoryTrait
//! that can create worker providers for Docker and Kubernetes.

use crate::{DockerProvider, KubernetesProvider};
use async_trait::async_trait;
use hodei_ports::worker_provider::{
    ProviderConfig, ProviderError, ProviderFactoryTrait, WorkerProvider,
};

/// Default provider factory implementation
pub struct DefaultProviderFactory;

impl Default for DefaultProviderFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultProviderFactory {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProviderFactoryTrait for DefaultProviderFactory {
    async fn create_provider(
        &self,
        config: ProviderConfig,
    ) -> Result<Box<dyn WorkerProvider>, ProviderError> {
        match config.provider_type {
            hodei_ports::worker_provider::ProviderType::Docker => {
                let provider = DockerProvider::new(config).await?;
                Ok(Box::new(provider) as Box<dyn WorkerProvider>)
            }

            hodei_ports::worker_provider::ProviderType::Kubernetes => {
                let provider = KubernetesProvider::new(config).await?;
                Ok(Box::new(provider) as Box<dyn WorkerProvider>)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_ports::worker_provider::ProviderType;

    #[tokio::test]
    async fn test_create_docker_provider() {
        let factory = DefaultProviderFactory::new();
        let config = ProviderConfig::docker("test-docker".to_string());

        let result = factory.create_provider(config).await;
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.provider_type(), ProviderType::Docker);
        assert_eq!(provider.name(), "test-docker");
    }

    #[tokio::test]
    async fn test_create_kubernetes_provider() {
        let factory = DefaultProviderFactory::new();
        let mut config = ProviderConfig::kubernetes("test-k8s".to_string());
        config.namespace = Some("default".to_string());

        // Test may fail if no Kubernetes cluster is available
        let result = factory.create_provider(config.clone()).await;

        // Allow both success (cluster available) and failure (no cluster)
        if result.is_ok() {
            let provider = result.unwrap();
            assert_eq!(provider.provider_type(), ProviderType::Kubernetes);
            assert_eq!(provider.name(), "test-k8s");
        } else {
            // Expected when running tests without a K8s cluster
            println!("Skipping K8s test - no cluster available");
        }
    }
}
