//! Worker Provider Port
//!
//! This module defines the port (trait) for worker infrastructure providers
//! that handle dynamic worker provisioning.

use async_trait::async_trait;
use hodei_core::{Worker, WorkerId};
use serde::{Deserialize, Serialize};

/// Provider type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    Docker,
    Kubernetes,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::Docker => "docker",
            ProviderType::Kubernetes => "kubernetes",
        }
    }
}

/// Provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_auto_scaling: bool,
    pub supports_health_checks: bool,
    pub supports_volumes: bool,
    pub max_workers: Option<u32>,
    pub estimated_provision_time_ms: u64,
}

/// Provider error
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Provider port trait
#[async_trait]
pub trait WorkerProvider: Send + Sync + std::fmt::Debug {
    fn provider_type(&self) -> ProviderType;
    fn name(&self) -> &str;
    async fn capabilities(&self) -> Result<ProviderCapabilities, ProviderError>;

    async fn create_worker(
        &self,
        worker_id: WorkerId,
        config: ProviderConfig,
    ) -> Result<Worker, ProviderError>;

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<hodei_core::WorkerStatus, ProviderError>;

    async fn stop_worker(&self, worker_id: &WorkerId, graceful: bool) -> Result<(), ProviderError>;

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    async fn list_workers(&self) -> Result<Vec<WorkerId>, ProviderError>;
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub name: String,
    pub namespace: Option<String>,
    pub docker_host: Option<String>,
    pub kube_config: Option<String>,

    /// Custom Docker image to use (e.g., "hwp-agent:latest")
    pub custom_image: Option<String>,

    /// Custom Kubernetes Pod manifest (YAML or JSON as String)
    pub custom_pod_template: Option<String>,
}

impl ProviderConfig {
    pub fn docker(name: String) -> Self {
        Self {
            provider_type: ProviderType::Docker,
            name,
            namespace: None,
            docker_host: None,
            kube_config: None,
            custom_image: None,
            custom_pod_template: None,
        }
    }

    pub fn kubernetes(name: String) -> Self {
        Self {
            provider_type: ProviderType::Kubernetes,
            name,
            namespace: Some("default".to_string()),
            docker_host: None,
            kube_config: None,
            custom_image: None,
            custom_pod_template: None,
        }
    }

    /// Set custom Docker image (overrides default HWP Agent image)
    pub fn with_image(mut self, image: String) -> Self {
        self.custom_image = Some(image);
        self
    }

    /// Set custom Kubernetes Pod template (YAML or JSON)
    pub fn with_pod_template(mut self, template: String) -> Self {
        self.custom_pod_template = Some(template);
        self
    }

    /// Set Kubernetes namespace
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Set Docker host
    pub fn with_docker_host(mut self, docker_host: String) -> Self {
        self.docker_host = Some(docker_host);
        self
    }
}

/// Provider factory trait - implemented in hodei-adapters
#[async_trait]
pub trait ProviderFactoryTrait: Send + Sync {
    async fn create_provider(
        &self,
        config: ProviderConfig,
    ) -> Result<Box<dyn WorkerProvider>, ProviderError>;
}
