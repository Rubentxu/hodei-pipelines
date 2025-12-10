//! Provider-as-Worker Port
//!
//! This module defines the port (trait) for provider workers that execute jobs.
//! Following the "Provider-as-Worker" pattern where each provider IS a worker.

use async_trait::async_trait;
use hodei_pipelines_domain::WorkerId;
use serde::{Deserialize, Serialize};

pub use crate::scheduling::worker_template::{
    EnvVar, ResourceRequirements, ToolSpec, VolumeMount, WorkerTemplate,
};

/// Provider-as-Worker type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    Lambda,            // AWS Lambda - serverless execution
    Kubernetes,        // Kubernetes cluster - pod-based execution
    Docker,            // Docker daemon - container-based execution
    AzureVm,           // Azure Virtual Machines
    GcpFunctions,      // Google Cloud Functions
    Ec2,               // AWS EC2
    ContainerInstance, // AWS Fargate / Azure Container Instances
    CloudRun,          // Google Cloud Run
    BareMetal,         // Physical servers
    Custom(String),    // Extensible
}

impl ProviderType {
    pub fn as_str(&self) -> &str {
        match self {
            ProviderType::Lambda => "lambda",
            ProviderType::Kubernetes => "kubernetes",
            ProviderType::Docker => "docker",
            ProviderType::AzureVm => "azure_vm",
            ProviderType::GcpFunctions => "gcp_functions",
            ProviderType::Ec2 => "ec2",
            ProviderType::ContainerInstance => "container_instance",
            ProviderType::CloudRun => "cloud_run",
            ProviderType::BareMetal => "bare_metal",
            ProviderType::Custom(name) => name,
        }
    }
}

/// Provider-as-Worker capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub max_cpu_cores: Option<u32>,
    pub max_memory_gb: Option<u64>,
    pub supports_gpu: bool,
    pub supported_runtimes: Vec<String>,
    pub supported_architectures: Vec<String>,
    pub max_execution_time: Option<u64>, // in seconds
    pub supports_ephemeral_storage: bool,
    pub regions: Vec<String>,
    pub cost_per_execution: Option<f64>,
    pub max_concurrent_jobs: u32,
}

impl ProviderCapabilities {
    pub fn new() -> Self {
        Self {
            max_cpu_cores: None,
            max_memory_gb: None,
            supports_gpu: false,
            supported_runtimes: Vec::new(),
            supported_architectures: Vec::new(),
            max_execution_time: None,
            supports_ephemeral_storage: false,
            regions: Vec::new(),
            cost_per_execution: None,
            max_concurrent_jobs: 10,
        }
    }

    pub fn with_concurrency(mut self, max_concurrent: u32) -> Self {
        self.max_concurrent_jobs = max_concurrent;
        self
    }
}

/// Provider-as-Worker error
#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Provider is unhealthy: {0}")]
    Unhealthy(String),
}

/// Execution status for jobs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Submitted,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Execution context for tracking job execution on a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub job_id: WorkerId,
    pub provider_id: ProviderId,
    pub provider_execution_id: String,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ExecutionStatus,
    pub result: Option<JobResult>,
    pub metadata: serde_json::Value,
}

impl ExecutionContext {
    pub fn new(job_id: WorkerId, provider_id: ProviderId, provider_execution_id: String) -> Self {
        Self {
            job_id,
            provider_id,
            provider_execution_id,
            submitted_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            status: ExecutionStatus::Submitted,
            result: None,
            metadata: serde_json::Value::Null,
        }
    }
}

/// Job result from provider execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResult {
    Success,
    Failed { exit_code: i32 },
    Cancelled,
    Timeout,
    Error { message: String },
}

/// Provider identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(pub uuid::Uuid);

impl ProviderId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for ProviderId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Provider-as-Worker port trait
/// Each provider IS a worker - they execute jobs directly
#[async_trait]
pub trait ProviderWorker: Send + Sync + std::fmt::Debug {
    /// Get provider ID
    fn provider_id(&self) -> ProviderId;

    /// Get provider type
    fn provider_type(&self) -> ProviderType;

    /// Get provider name
    fn name(&self) -> &str;

    /// Get provider capabilities
    fn capabilities(&self) -> &ProviderCapabilities;

    /// Check if provider can execute this job
    fn can_execute(&self, spec: &JobSpec) -> bool;

    /// Submit job for execution (async)
    async fn submit_job(&self, job: &JobSpec) -> Result<ExecutionContext, ProviderError>;

    /// Check job status
    async fn check_status(
        &self,
        context: &ExecutionContext,
    ) -> Result<ExecutionStatus, ProviderError>;

    /// Wait for job completion
    async fn wait_for_completion(
        &self,
        context: &ExecutionContext,
        timeout: std::time::Duration,
    ) -> Result<JobResult, ProviderError>;

    /// Get job logs
    async fn get_logs(&self, context: &ExecutionContext) -> Result<Vec<LogEntry>, ProviderError>;

    /// Cancel job
    async fn cancel(&self, context: &ExecutionContext) -> Result<(), ProviderError>;

    /// Health check
    async fn health_check(&self) -> Result<bool, ProviderError>;

    /// Estimate cost for job execution
    fn estimate_cost(&self, spec: &JobSpec) -> Option<f64>;
}

/// Job specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub id: WorkerId,
    pub image: String,
    pub command: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub resources: ResourceRequirements,
    pub timeout_ms: u64,
    pub labels: std::collections::HashMap<String, String>,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub job_id: WorkerId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: String,
    pub stream_type: LogStreamType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogStreamType {
    Stdout,
    Stderr,
}

impl LogEntry {
    pub fn new(
        job_id: WorkerId,
        message: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            job_id,
            message,
            timestamp,
            stream_type: LogStreamType::Stdout,
        }
    }
}

/// Provider-as-Worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub name: String,
    pub provider_id: ProviderId,

    /// Provider-specific configuration
    pub namespace: Option<String>,
    pub docker_host: Option<String>,
    pub kube_config: Option<String>,
    pub region: Option<String>,
    pub credentials: Option<std::collections::HashMap<String, String>>,

    /// Default worker template for jobs
    pub default_template: Option<WorkerTemplate>,

    /// Provider-specific settings
    pub settings: std::collections::HashMap<String, String>,
}

impl ProviderConfig {
    pub fn new(provider_type: ProviderType, name: String) -> Self {
        Self {
            provider_type,
            name,
            provider_id: ProviderId::new(),
            namespace: None,
            docker_host: None,
            kube_config: None,
            region: None,
            credentials: None,
            default_template: None,
            settings: std::collections::HashMap::new(),
        }
    }

    pub fn docker(name: String) -> Self {
        let mut config = Self::new(ProviderType::Docker, name);
        config.docker_host = Some("unix:///var/run/docker.sock".to_string());
        config
    }

    pub fn kubernetes(name: String) -> Self {
        let mut config = Self::new(ProviderType::Kubernetes, name);
        config.namespace = Some("default".to_string());
        config
    }

    pub fn lambda(name: String, region: String) -> Self {
        let mut config = Self::new(ProviderType::Lambda, name);
        config.region = Some(region);
        config
    }

    /// Set region for cloud providers
    pub fn with_region(mut self, region: String) -> Self {
        self.region = Some(region);
        self
    }

    /// Set default template
    pub fn with_template(mut self, template: WorkerTemplate) -> Self {
        self.default_template = Some(template);
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

    /// Add a credential
    pub fn with_credential(mut self, key: String, value: String) -> Self {
        if self.credentials.is_none() {
            self.credentials = Some(std::collections::HashMap::new());
        }
        self.credentials.as_mut().unwrap().insert(key, value);
        self
    }

    /// Add a setting
    pub fn with_setting(mut self, key: String, value: String) -> Self {
        self.settings.insert(key, value);
        self
    }
}

/// Provider factory trait - implemented in hodei-pipelines-adapters
#[async_trait]
pub trait ProviderFactoryTrait: Send + Sync {
    async fn create_provider(
        &self,
        config: ProviderConfig,
    ) -> Result<Box<dyn ProviderWorker>, ProviderError>;
}
