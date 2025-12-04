//! Worker Provisioner Port
//!
//! This module defines the port (trait) for provisioning ephemeral workers
//! for pipeline execution with resource-aware scheduling.

use async_trait::async_trait;
use hodei_pipelines_domain::{Pipeline, Worker, WorkerId};
use serde::{Deserialize, Serialize};

use crate::scheduling::worker_template::WorkerTemplate;

/// Worker allocation request for pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAllocationRequest {
    /// Pipeline being executed
    pub pipeline: Pipeline,

    /// Calculated resource requirements
    pub resource_requirements: ResourceRequirements,

    /// Tenant ID for multi-tenancy
    pub tenant_id: Option<String>,

    /// Priority of the execution
    pub priority: u8,

    /// Labels for pool selection
    pub required_labels: Vec<(String, String)>,

    /// Preferred labels (optional)
    pub preferred_labels: Vec<(String, String)>,
}

/// Resource requirements for pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements in millicores (1000m = 1 core)
    pub cpu_millicores: u64,

    /// Memory requirements in MB
    pub memory_mb: u64,

    /// GPU requirements (optional)
    pub gpu_count: Option<u32>,

    /// Storage requirements in GB (optional)
    pub storage_gb: Option<u64>,

    /// Estimated execution duration in seconds
    pub estimated_duration_secs: u64,

    /// Maximum number of retry attempts
    pub max_retries: u8,
}

/// Worker provisioner error
#[derive(thiserror::Error, Debug)]
pub enum WorkerProvisionerError {
    #[error("Provisioner error: {0}")]
    Provisioner(String),

    #[error("Resource allocation failed: {0}")]
    ResourceAllocation(String),

    #[error("Worker provisioning failed: {0}")]
    WorkerProvisioning(String),

    #[error("Timeout waiting for worker: {0}")]
    Timeout(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Result type for worker provisioner operations
pub type WorkerProvisionerResult<T> = Result<T, WorkerProvisionerError>;

/// Worker provisioner port trait
#[async_trait]
pub trait WorkerProvisioner: Send + Sync + std::fmt::Debug {
    /// Provision an ephemeral worker for pipeline execution
    async fn provision_worker_for_pipeline(
        &self,
        request: WorkerAllocationRequest,
    ) -> WorkerProvisionerResult<ProvisionedWorker>;

    /// Calculate resource requirements from a pipeline
    async fn calculate_resource_requirements(
        &self,
        pipeline: &Pipeline,
    ) -> WorkerProvisionerResult<ResourceRequirements>;

    /// Create worker template from pipeline and resource requirements
    async fn create_worker_template(
        &self,
        pipeline: &Pipeline,
        requirements: &ResourceRequirements,
    ) -> WorkerProvisionerResult<WorkerTemplate>;
}

/// A provisioned worker ready for pipeline execution
#[derive(Debug, Clone)]
pub struct ProvisionedWorker {
    /// The worker entity
    pub worker: Worker,

    /// Worker ID
    pub worker_id: WorkerId,

    /// Worker template used for provisioning
    pub template: WorkerTemplate,

    /// Resource pool where worker was provisioned
    pub pool_id: Option<String>,

    /// Allocation metadata
    pub allocation_metadata: AllocationMetadata,
}

/// Allocation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationMetadata {
    /// When the worker was allocated
    pub allocated_at: chrono::DateTime<chrono::Utc>,

    /// Estimated cost per hour (if available)
    pub estimated_cost_per_hour: Option<f64>,

    /// Auto-cleanup timeout in seconds
    pub auto_cleanup_seconds: Option<u64>,

    /// Resource pool constraints
    pub pool_constraints: Vec<String>,
}