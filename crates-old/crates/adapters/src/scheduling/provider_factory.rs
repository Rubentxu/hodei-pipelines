//! Provider-as-Worker Factory
//!
//! This module provides the concrete implementation of the ProviderFactoryTrait
//! that can create provider-as-workers for Docker, Kubernetes, and other providers.

use crate::{DockerProvider, KubernetesProvider};
use async_trait::async_trait;
use hodei_pipelines_ports::worker_provider::{
    ProviderConfig, ProviderError, ProviderFactoryTrait, ProviderType, ProviderWorker,
};

/// Default provider-as-worker factory implementation
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
    ) -> Result<Box<dyn ProviderWorker>, ProviderError> {
        match config.provider_type {
            ProviderType::Docker => {
                let provider = DockerProvider::new(config).await?;
                Ok(Box::new(provider) as Box<dyn ProviderWorker>)
            }

            ProviderType::Kubernetes => {
                let provider = KubernetesProvider::new(config).await?;
                Ok(Box::new(provider) as Box<dyn ProviderWorker>)
            }

            // TODO: Implement other providers
            ProviderType::Lambda => {
                unimplemented!("Lambda provider not yet implemented")
            }
            ProviderType::AzureVm => {
                unimplemented!("Azure VM provider not yet implemented")
            }
            ProviderType::GcpFunctions => {
                unimplemented!("GCP Functions provider not yet implemented")
            }
            ProviderType::Ec2 => {
                unimplemented!("EC2 provider not yet implemented")
            }
            ProviderType::ContainerInstance => {
                unimplemented!("Container Instance provider not yet implemented")
            }
            ProviderType::CloudRun => {
                unimplemented!("CloudRun provider not yet implemented")
            }
            ProviderType::BareMetal => {
                unimplemented!("Bare Metal provider not yet implemented")
            }
            ProviderType::Custom(_) => {
                unimplemented!("Custom provider not yet implemented")
            }
        }
    }
}
