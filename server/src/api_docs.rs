//! API Documentation using OpenAPI 3.0 with utoipa
//!
//! This module provides comprehensive API documentation for the Hodei Pipelines API.
//! Access the interactive Swagger UI at: http://localhost:8080/api/docs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};

// Re-export shared types for API documentation
pub use hodei_shared_types::{JobSpec, ResourceQuota, WorkerCapabilities};

/// Health check response
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "status": "healthy",
    "service": "hodei-server",
    "version": "0.1.0",
    "architecture": "monolithic_modular"
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "process-data",
    "image": "ubuntu:latest",
    "command": ["echo", "Hello World"],
    "resources": {
        "cpu_m": 1000,
        "memory_mb": 2048
    },
    "timeout_ms": 300000,
    "retries": 3,
    "env": {
        "ENVIRONMENT": "production",
        "LOG_LEVEL": "info"
    },
    "secret_refs": ["database-password"]
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "name": "process-data",
    "spec": {
        "name": "process-data",
        "image": "ubuntu:latest",
        "command": ["echo", "Hello World"]
    },
    "state": "PENDING",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z",
    "started_at": null,
    "completed_at": null,
    "result": null
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
pub struct JobListResponse {
    /// List of jobs
    pub jobs: Vec<JobResponse>,
}

/// Register worker request
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "worker-01",
    "cpu_cores": 4,
    "memory_gb": 8
}))]
pub struct RegisterWorkerRequest {
    /// Worker name
    pub name: String,
    /// Number of CPU cores
    pub cpu_cores: u32,
    /// Memory in GB
    pub memory_gb: u64,
}

/// Worker response
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "name": "worker-01",
    "status": "IDLE",
    "capabilities": {
        "cpu_cores": 4,
        "memory_gb": 8
    },
    "last_heartbeat": "2024-01-01T00:00:00Z"
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "message": "Operation completed successfully"
}))]
pub struct MessageResponse {
    /// Success message
    pub message: String,
}

/// Generic error response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Optional details
    pub details: Option<String>,
}

/// Create dynamic worker request
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "provider_type": "docker",
    "namespace": "default",
    "image": "hwp-agent:latest",
    "cpu_cores": 4,
    "memory_mb": 8192,
    "env": {
        "HODEI_SERVER_GRPC_URL": "http://hodei-server:50051"
    },
    "labels": {
        "env": "production"
    },
    "custom_image": null,
    "custom_pod_template": null
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "worker_id": "550e8400-e29b-41d4-a716-446655440000",
    "container_id": "hodei-worker-550e8400",
    "state": "starting",
    "message": "Worker provisioned successfully"
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "worker_id": "550e8400-e29b-41d4-a716-446655440000",
    "state": "running",
    "container_id": "hodei-worker-550e8400",
    "created_at": "2024-01-01T00:00:00Z"
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ListDynamicWorkersResponse {
    /// List of dynamic workers
    pub workers: Vec<DynamicWorkerStatusResponse>,
}

/// Provider capabilities response
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "provider_type": "docker",
    "name": "docker-provider",
    "capabilities": {
        "supports_auto_scaling": true,
        "supports_health_checks": true,
        "supports_volumes": true,
        "max_workers": 100,
        "estimated_provision_time_ms": 5000
    }
}))]
pub struct ProviderCapabilitiesResponse {
    /// Provider type
    pub provider_type: String,
    /// Provider name
    pub name: String,
    /// Provider capabilities
    pub capabilities: ProviderCapabilitiesInfo,
}

#[derive(Serialize, Deserialize, ToSchema)]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!("docker"))]
pub enum ProviderTypeDto {
    Docker,
    Kubernetes,
}

/// Provider info
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "provider_type": "docker",
    "name": "docker-provider",
    "status": "active"
}))]
pub struct ProviderInfo {
    /// Provider type
    pub provider_type: String,
    /// Provider name
    pub name: String,
    /// Provider status
    pub status: String,
}

/// List providers response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ListProvidersResponse {
    /// List of available providers
    pub providers: Vec<ProviderInfo>,
}

/// Create provider request
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "provider_type": "docker",
    "name": "my-docker",
    "namespace": "default",
    "docker_host": "unix:///var/run/docker.sock",
    "custom_image": "hwp-agent:latest",
    "custom_pod_template": null
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "provider_type": "docker",
    "name": "my-docker",
    "namespace": "default",
    "custom_image": "hwp-agent:latest",
    "status": "active",
    "created_at": "2024-01-01T00:00:00Z"
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "tenant-a",
    "email": "admin@tenant-a.com"
}))]
pub struct CreateTenantRequest {
    /// Tenant name
    pub name: String,
    /// Tenant admin email
    pub email: String,
}

/// Update tenant request (EPIC-09)
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "tenant-a-updated",
    "email": "admin-updated@tenant-a.com"
}))]
pub struct UpdateTenantRequest {
    /// Tenant name
    pub name: String,
    /// Tenant admin email
    pub email: String,
}

/// Tenant response (EPIC-09)
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "name": "tenant-a",
    "email": "admin@tenant-a.com",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "cpu_m": 4000,
    "memory_mb": 8192,
    "max_concurrent_jobs": 10,
    "current_usage": {
        "cpu_m": 1500,
        "memory_mb": 2048,
        "active_jobs": 3
    }
}))]
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
#[derive(Serialize, Deserialize, ToSchema)]
pub struct QuotaUsage {
    /// Currently used CPU in millicores
    pub cpu_m: u64,
    /// Currently used memory in MB
    pub memory_mb: u64,
    /// Currently active jobs
    pub active_jobs: u32,
}

/// List tenants response (EPIC-09)
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ListTenantsResponse {
    /// List of tenants
    pub tenants: Vec<TenantResponse>,
}
