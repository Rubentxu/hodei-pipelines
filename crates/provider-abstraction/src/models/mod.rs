//! Models module
//!
//! This module contains all data types for worker provider abstraction.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::traits::WorkerProvider;
pub use hodei_shared_types::worker_messages::WorkerId;

/// Provider types supported by the abstraction layer
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderType {
    Kubernetes,
    Docker,
    AwsEcs,
    AzureContainerInstances,
    Custom(String),
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Kubernetes => write!(f, "kubernetes"),
            ProviderType::Docker => write!(f, "docker"),
            ProviderType::AwsEcs => write!(f, "aws-ecs"),
            ProviderType::AzureContainerInstances => write!(f, "azure-container-instances"),
            ProviderType::Custom(name) => write!(f, "custom-{}", name),
        }
    }
}

/// Provider configuration
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub connection_string: String,
    pub credentials: Option<ProviderCredentials>,
    pub options: HashMap<String, String>,
}

/// Provider credentials
#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub certificate_path: Option<String>,
}

/// Worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub worker_id: WorkerId,
    pub image: String,
    pub resources: ResourceRequirements,
    pub environment: HashMap<String, String>,
    pub secrets: Vec<SecretReference>,
    pub health_checks: Vec<HealthCheckConfig>,
    pub scaling_config: ScalingConfiguration,
}

impl WorkerConfig {
    pub fn validate(&self) -> Result<(), ProviderError> {
        if self.image.is_empty() {
            return Err(ProviderError::ConfigurationError(
                "image cannot be empty".to_string(),
            ));
        }

        if self.resources.cpu_cores < 0.1 {
            return Err(ProviderError::ConfigurationError(
                "cpu_cores must be at least 0.1".to_string(),
            ));
        }

        Ok(())
    }
}

/// Resource requirements for workers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub ephemeral_storage_bytes: Option<u64>,
}

/// Reference to a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretReference {
    pub name: String,
    pub mount_path: String,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub path: String,
    pub port: u16,
    pub interval: Duration,
    pub timeout: Duration,
    pub initial_delay: Duration,
}

/// Scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfiguration {
    pub min_replicas: usize,
    pub max_replicas: usize,
    pub target_cpu_utilization: Option<u32>,
}

/// Worker status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub worker_id: WorkerId,
    pub state: WorkerState,
    pub started_at: Option<SystemTime>,
    pub last_health_check: Option<SystemTime>,
    pub resource_usage: ResourceUsage,
}

/// Worker state enum
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerState {
    Creating,
    Starting,
    Running,
    Degraded,
    Recovering,
    Stopping,
    Stopped,
    Failed,
}

impl std::fmt::Display for WorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerState::Creating => write!(f, "Creating"),
            WorkerState::Starting => write!(f, "Starting"),
            WorkerState::Running => write!(f, "Running"),
            WorkerState::Degraded => write!(f, "Degraded"),
            WorkerState::Recovering => write!(f, "Recovering"),
            WorkerState::Stopping => write!(f, "Stopping"),
            WorkerState::Stopped => write!(f, "Stopped"),
            WorkerState::Failed => write!(f, "Failed"),
        }
    }
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
}

/// Provider capabilities
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    pub auto_scaling: bool,
    pub health_checks: bool,
    pub volumes: bool,
    pub config_maps: bool,
    pub secrets: bool,
    pub network_policies: bool,
    pub multi_cluster: bool,
}

impl ProviderCapabilities {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Worker metadata for tracking
#[derive(Debug, Clone, Default)]
pub struct WorkerMetadata {
    pub created_at: Option<SystemTime>,
    pub labels: HashMap<String, String>,
}

impl WorkerMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }
}

/// Handle to an existing worker
#[derive(Debug, Clone)]
pub struct WorkerHandle {
    pub worker_id: WorkerId,
    pub provider_type: ProviderType,
    pub metadata: WorkerMetadata,
}

impl WorkerHandle {
    pub fn new(worker_id: WorkerId, provider_type: ProviderType) -> Self {
        Self {
            worker_id,
            provider_type,
            metadata: WorkerMetadata::new(),
        }
    }
}

/// Core error types for worker provider abstraction
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("provider not initialized: {0}")]
    NotInitialized(String),

    #[error("provider initialization failed: {0}")]
    InitializationFailed(String),

    #[error("worker operation failed: {0}")]
    WorkerOperationFailed(String),

    #[error("capability not supported: {0}")]
    CapabilityNotSupported(String),

    #[error("configuration error: {0}")]
    ConfigurationError(String),

    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("provider operation timeout")]
    Timeout,

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Mock provider implementation for testing
pub struct MockWorkerProvider {
    provider_type: ProviderType,
    workers: dashmap::DashMap<WorkerId, WorkerStatus>,
}

impl MockWorkerProvider {
    pub fn new(provider_type: ProviderType) -> Self {
        Self {
            provider_type,
            workers: dashmap::DashMap::new(),
        }
    }
}

#[async_trait]
impl WorkerProvider for MockWorkerProvider {
    async fn create_worker(&self, config: &WorkerConfig) -> Result<WorkerHandle, ProviderError> {
        let status = WorkerStatus {
            worker_id: config.worker_id.clone(),
            state: WorkerState::Creating,
            started_at: None,
            last_health_check: None,
            resource_usage: ResourceUsage {
                cpu_cores: 0.0,
                memory_bytes: 0,
                disk_bytes: 0,
            },
        };

        self.workers.insert(config.worker_id.clone(), status);

        Ok(WorkerHandle::new(
            config.worker_id.clone(),
            self.provider_type.clone(),
        ))
    }

    async fn start_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        if let Some(mut status) = self.workers.get_mut(worker_id) {
            status.state = WorkerState::Starting;
        }
        Ok(())
    }

    async fn stop_worker(
        &self,
        worker_id: &WorkerId,
        _graceful: bool,
    ) -> Result<(), ProviderError> {
        if let Some(mut status) = self.workers.get_mut(worker_id) {
            status.state = WorkerState::Stopping;
        }
        Ok(())
    }

    async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        self.workers.remove(worker_id);
        Ok(())
    }

    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerStatus, ProviderError> {
        self.workers
            .get(worker_id)
            .map(|status| status.clone())
            .ok_or_else(|| {
                ProviderError::WorkerOperationFailed(format!("Worker {:?} not found", worker_id))
            })
    }

    async fn get_capabilities(&self) -> Result<ProviderCapabilities, ProviderError> {
        Ok(ProviderCapabilities::new())
    }

    async fn scale_workers(
        &self,
        _worker_type: &str,
        _target_count: usize,
    ) -> Result<Vec<WorkerId>, ProviderError> {
        Ok(Vec::new())
    }

    fn get_provider_type(&self) -> ProviderType {
        self.provider_type.clone()
    }
}
