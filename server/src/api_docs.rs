//! API Documentation using OpenAPI 3.0 with utoipa
//!
//! This module provides comprehensive API documentation for the Hodei Pipelines API.
//! Access the interactive Swagger UI at: http://localhost:8080/api/docs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use utoipa::ToSchema; // Disabled for compilation

// Re-export shared types for API documentation
pub use hodei_core::{JobSpec, ResourceQuota, WorkerCapabilities};

/// Health check response
#[derive(Serialize, Deserialize)]
pub struct HealthResponse {
    /// Status of the service
    pub status: String,
    /// Name of the service
    pub service: String,
    /// Version of the service
    pub version: String,
    /// Architecture type
    pub architecture: String,
}

/// Job specification for creating new jobs
#[derive(Serialize, Deserialize)]
pub struct CreateJobRequest {
    /// Name of the job
    pub name: String,
    /// Docker image to use
    pub image: String,
    /// Command to execute
    pub command: Vec<String>,
    /// Resource requirements
    pub resources: ResourceQuota,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Number of retries on failure
    pub retries: u8,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// References to secrets
    pub secret_refs: Vec<String>,
}

/// Job response
#[derive(Serialize, Deserialize)]
pub struct JobResponse {
    /// Unique job identifier
    pub id: String,
    /// Job name
    pub name: String,
    /// Job specification
    pub spec: JobSpec,
    /// Current state
    pub state: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Start timestamp (null if not started)
    pub started_at: Option<DateTime<Utc>>,
    /// Completion timestamp (null if not completed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Job result (null if not completed)
    pub result: Option<serde_json::Value>,
}

/// Response wrapper for jobs
#[derive(Serialize, Deserialize)]
pub struct JobListResponse {
    /// List of jobs
    pub jobs: Vec<JobResponse>,
}

/// Register worker request
#[derive(Serialize, Deserialize)]
pub struct RegisterWorkerRequest {
    /// Worker name
    pub name: String,
    /// Number of CPU cores
    pub cpu_cores: u32,
    /// Memory in GB
    pub memory_gb: u64,
}

/// Worker response
#[derive(Serialize, Deserialize)]
pub struct WorkerResponse {
    /// Unique worker identifier
    pub id: String,
    /// Worker name
    pub name: String,
    /// Current status
    pub status: String,
    /// Worker capabilities
    pub capabilities: WorkerCapabilities,
    /// Last heartbeat timestamp
    pub last_heartbeat: DateTime<Utc>,
}

/// Generic success message
#[derive(Serialize, Deserialize)]
pub struct MessageResponse {
    /// Success message
    pub message: String,
}

/// Generic error response
#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Optional details
    pub details: Option<String>,
}

/// Create dynamic worker request
#[derive(Serialize, Deserialize)]
pub struct CreateDynamicWorkerRequest {
    /// Infrastructure provider type
    pub provider_type: String,
    /// Kubernetes namespace (if using K8s provider)
    pub namespace: Option<String>,
    /// Docker image to use for the worker
    pub image: String,
    /// Number of CPU cores to allocate
    pub cpu_cores: u32,
    /// Memory in MB to allocate
    pub memory_mb: u64,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// Labels to attach to the worker
    pub labels: Option<HashMap<String, String>>,
    /// Custom image (overrides default)
    pub custom_image: Option<String>,
    /// Custom Kubernetes Pod template (YAML or JSON as String, only for K8s)
    pub custom_pod_template: Option<String>,
}

/// Create dynamic worker response
#[derive(Serialize, Deserialize)]
pub struct CreateDynamicWorkerResponse {
    /// Unique worker identifier
    pub worker_id: String,
    /// Container ID (Docker)
    pub container_id: Option<String>,
    /// Current state
    pub state: String,
    /// Status message
    pub message: String,
}

/// Dynamic worker status response
#[derive(Serialize, Deserialize)]
pub struct DynamicWorkerStatusResponse {
    /// Unique worker identifier
    pub worker_id: String,
    /// Current state
    pub state: String,
    /// Container ID (if applicable)
    pub container_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// List dynamic workers response
#[derive(Serialize, Deserialize)]
pub struct ListDynamicWorkersResponse {
    /// List of dynamic workers
    pub workers: Vec<DynamicWorkerStatusResponse>,
}

/// Provider capabilities response
#[derive(Serialize, Deserialize)]
pub struct ProviderCapabilitiesResponse {
    /// Provider type
    pub provider_type: String,
    /// Provider name
    pub name: String,
    /// Provider capabilities
    pub capabilities: ProviderCapabilitiesInfo,
}

#[derive(Serialize, Deserialize)]
pub struct ProviderCapabilitiesInfo {
    /// Supports auto-scaling
    pub supports_auto_scaling: bool,
    /// Supports health checks
    pub supports_health_checks: bool,
    /// Supports volumes
    pub supports_volumes: bool,
    /// Maximum number of workers (null if unlimited)
    pub max_workers: Option<u32>,
    /// Estimated provisioning time in milliseconds
    pub estimated_provision_time_ms: u64,
}

/// Provider type enumeration
#[derive(Serialize, Deserialize)]
pub enum ProviderTypeDto {
    Docker,
    Kubernetes,
}

/// Provider info
#[derive(Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider type
    pub provider_type: String,
    /// Provider name
    pub name: String,
    /// Provider status
    pub status: String,
}

/// List providers response
#[derive(Serialize, Deserialize)]
pub struct ListProvidersResponse {
    /// List of available providers
    pub providers: Vec<ProviderInfo>,
}

/// Create provider request
#[derive(Serialize, Deserialize)]
pub struct CreateProviderRequest {
    /// Provider type
    pub provider_type: ProviderTypeDto,
    /// Provider name
    pub name: String,
    /// Namespace (for Kubernetes)
    pub namespace: Option<String>,
    /// Docker host (for Docker)
    pub docker_host: Option<String>,
    /// Custom image (overrides default)
    pub custom_image: Option<String>,
    /// Custom K8s Pod template (for Kubernetes, YAML or JSON as String)
    pub custom_pod_template: Option<String>,
}

/// Provider response
#[derive(Serialize, Deserialize)]
pub struct ProviderResponse {
    /// Provider type
    pub provider_type: String,
    /// Provider name
    pub name: String,
    /// Namespace
    pub namespace: Option<String>,
    /// Custom image
    pub custom_image: Option<String>,
    /// Provider status
    pub status: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// EPIC-09: Tenant Management Schemas
// ============================================================================

/// Create tenant request (EPIC-09)
#[derive(Serialize, Deserialize)]
pub struct CreateTenantRequest {
    /// Tenant name
    pub name: String,
    /// Tenant admin email
    pub email: String,
}

/// Update tenant request (EPIC-09)
#[derive(Serialize, Deserialize)]
pub struct UpdateTenantRequest {
    /// Tenant name
    pub name: String,
    /// Tenant admin email
    pub email: String,
}

/// Tenant response (EPIC-09)
#[derive(Serialize, Deserialize)]
pub struct TenantResponse {
    /// Unique tenant identifier
    pub id: String,
    /// Tenant name
    pub name: String,
    /// Tenant admin email
    pub email: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Quota response (EPIC-09)
#[derive(Serialize, Deserialize)]
pub struct QuotaResponse {
    /// CPU allocation in millicores
    pub cpu_m: u64,
    /// Memory allocation in MB
    pub memory_mb: u64,
    /// Maximum concurrent jobs
    pub max_concurrent_jobs: u32,
    /// Current resource usage
    pub current_usage: QuotaUsage,
}

/// Quota usage (EPIC-09)
#[derive(Serialize, Deserialize)]
pub struct QuotaUsage {
    /// Currently used CPU in millicores
    pub cpu_m: u64,
    /// Currently used memory in MB
    pub memory_mb: u64,
    /// Currently active jobs
    pub active_jobs: u32,
}

/// List tenants response (EPIC-09)
#[derive(Serialize, Deserialize)]
pub struct ListTenantsResponse {
    /// List of tenants
    pub tenants: Vec<TenantResponse>,
}
