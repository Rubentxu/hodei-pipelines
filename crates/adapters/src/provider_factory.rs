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

