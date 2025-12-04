//! API Documentation using OpenAPI 3.0 with utoipa
//!
//! This module provides comprehensive API documentation for the Hodei Pipelines API.
//! Access the interactive Swagger UI at: http://localhost:8080/swagger-ui/

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{OpenApi, ToSchema};

use crate::dtos::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::pipeline_execution::pipeline_api::create_pipeline_handler,
        crate::pipeline_execution::pipeline_api::list_pipelines_handler,
        crate::pipeline_execution::pipeline_api::get_pipeline_handler,
        crate::pipeline_execution::pipeline_api::update_pipeline_handler,
        crate::pipeline_execution::pipeline_api::delete_pipeline_handler,
        crate::pipeline_execution::pipeline_api::execute_pipeline_handler,
        crate::pipeline_execution::pipeline_api::get_pipeline_dag_handler,
        crate::pipeline_execution::pipeline_api::get_step_details_handler,
        crate::pipeline_execution::pipeline_api::get_execution_logs_handler,
        crate::resource_governance::api::list_pools_handler,
        crate::resource_governance::api::create_pool_handler,
        crate::resource_governance::api::get_pool_handler,
        crate::resource_governance::api::update_pool_put_handler,
        crate::resource_governance::api::update_pool_patch_handler,
        crate::resource_governance::api::delete_pool_handler,
        crate::resource_governance::api::get_pool_status_handler,
        // Observability APIs (partial implementation - Story 4)
        crate::observability::metrics_api::get_dashboard_metrics,
        crate::live_metrics_api::metrics_websocket_handler,
        crate::observability::logs_api::logs_stream_handler,
        crate::observability::logs_explorer_ui::logs_statistics_handler,
        crate::observability::traces_distributed_tracing::get_trace_handler,
        // Complete Observability APIs (Story 12)
        crate::observability::observability_api::get_service_health,
        crate::observability::observability_api::get_performance_metrics,
        crate::observability::observability_api::get_metrics,
        crate::observability::observability_api::get_error_events,
        crate::observability::observability_api::get_audit_logs,
        crate::observability::observability_api::get_trace_spans,
        crate::observability::observability_api::get_observability_config,
        crate::observability::observability_api::update_observability_config,
        crate::observability::observability_api::get_cluster_topology,
        // Cost Management APIs (Story 6)
        crate::resource_governance::cost_tracking_aggregation::get_cost_summary_handler,
        crate::resource_governance::cost_tracking_aggregation::get_cost_by_resource_handler,
        crate::resource_governance::cost_tracking_aggregation::get_cost_by_tenant_handler,
        crate::resource_governance::cost_tracking_aggregation::get_cost_trends_handler,
        // Budget Management APIs (Story 9)
        crate::resource_governance::budget_management::list_budgets_handler,
        crate::resource_governance::budget_management::get_budget_handler,
        crate::resource_governance::budget_management::create_budget_handler,
        crate::resource_governance::budget_management::update_budget_handler,
        crate::resource_governance::budget_management::delete_budget_handler,
        crate::resource_governance::budget_management::get_budget_usage_handler,
        crate::resource_governance::budget_management::get_budget_alerts_handler,
        crate::resource_governance::budget_management::check_budget_alerts_handler,
        // RBAC Management APIs (Story 8)
        crate::identity_access::rbac::login_handler,
        crate::identity_access::rbac::get_user_handler,
        crate::identity_access::rbac::list_users_handler,
        crate::identity_access::rbac::create_user_handler,
        crate::identity_access::rbac::update_user_handler,
        crate::identity_access::rbac::delete_user_handler,
        crate::identity_access::rbac::get_user_roles_handler,
        crate::identity_access::rbac::assign_role_handler,
        crate::identity_access::rbac::revoke_role_handler,
        crate::identity_access::rbac::check_permission_handler,
        hello_openapi,
    ),
    components(
        schemas(
            HealthResponse,
            CreateJobRequest,
            JobResponse,
            JobListResponse,
            RegisterWorkerRequest,
            WorkerResponse,
            MessageResponse,
            ErrorResponse,
            CreateDynamicWorkerRequest,
            CreateDynamicWorkerResponse,
            DynamicWorkerStatusResponse,
            ListDynamicWorkersResponse,
            ProviderCapabilitiesResponse,
            ProviderCapabilitiesInfo,
            ProviderTypeDto,
            ProviderInfo,
            ListProvidersResponse,
            CreateProviderRequest,
            ProviderResponse,
            CreateTenantRequest,
            UpdateTenantRequest,
            TenantResponse,
            QuotaResponse,
            QuotaUsage,
            ListTenantsResponse,
            // Pipeline DTOs
            crate::dtos::CreatePipelineRequestDto,
            crate::dtos::CreatePipelineStepRequestDto,
            crate::dtos::PipelineDto,
            crate::dtos::PipelineStepDto,
            crate::dtos::JobSpecDto,
            crate::dtos::ResourceQuotaDto,
            crate::dtos::ListPipelinesResponseDto,
            crate::dtos::PipelineSummaryDto,
            crate::dtos::ExecutePipelineRequestDto,
            crate::dtos::ExecutePipelineResponseDto,
            crate::dtos::DagNodeDto,
            crate::dtos::DagEdgeDto,
            crate::dtos::DagPositionDto,
            crate::dtos::DagStructureDto,
            crate::dtos::StepDetailsDto,
            crate::dtos::ExecutionLogsDto,
            crate::dtos::StepExecutionLogsDto,
            crate::dtos::LogEntryDto,
            // Resource Pool DTOs
            crate::dtos::CreatePoolRequestDto,
            crate::dtos::UpdatePoolRequestDto,
            crate::dtos::ResourcePoolResponseDto,
            crate::dtos::ResourcePoolConfigDto,
            crate::dtos::ResourcePoolStatusDto,
            crate::dtos::ResourcePoolTypeDto,
            // Observability Streaming DTOs
            // crate::live_metrics_api::LiveMetric,
            // crate::live_metrics_api::MetricType,
            // crate::live_metrics_api::ThresholdStatus,
            crate::observability::logs_api::LogEvent,
            crate::observability::logs_api::LogLevel,
            crate::observability::metrics_api::DashboardMetrics,
            crate::observability::metrics_api::DashboardMetricsRequest,
            crate::observability::logs_explorer_ui::LogStatistics,
            crate::observability::traces_distributed_tracing::Trace,
            crate::observability::traces_distributed_tracing::Span,
            crate::observability::traces_distributed_tracing::SpanLog,
            // Complete Observability DTOs (Story 12)
            crate::observability::observability_api::ObservabilityMetric,
            crate::observability::observability_api::ServiceHealth,
            crate::observability::observability_api::HealthStatus,
            crate::observability::observability_api::DependencyHealth,
            crate::observability::observability_api::TraceSpan,
            crate::observability::observability_api::SpanLog,
            crate::observability::observability_api::LogLevel,
            crate::observability::observability_api::PerformanceMetrics,
            crate::observability::observability_api::ErrorEvent,
            crate::observability::observability_api::ErrorSeverity,
            crate::observability::observability_api::AuditLog,
            crate::observability::observability_api::AuditOutcome,
            crate::observability::observability_api::ClusterTopology,
            crate::observability::observability_api::ClusterNode,
            crate::observability::observability_api::NodeType,
            crate::observability::observability_api::NodeCapabilities,
            crate::observability::observability_api::ClusterEdge,
            crate::observability::observability_api::EdgeType,
            crate::observability::observability_api::ObservabilityConfig,
            // Cost Management DTOs (Story 6)
            crate::resource_governance::cost_tracking_aggregation::CostSummary,
            crate::resource_governance::cost_tracking_aggregation::CostBreakdown,
            crate::resource_governance::cost_tracking_aggregation::TenantCostBreakdown,
            crate::resource_governance::cost_tracking_aggregation::CostTrend,
            crate::resource_governance::cost_optimization_recommendations::Recommendation,
            crate::resource_governance::cost_optimization_recommendations::OptimizationType,
            crate::resource_governance::cost_optimization_recommendations::ResourceType,
            // Budget Management DTOs (Story 9)
            crate::resource_governance::budget_management::Budget,
            crate::resource_governance::budget_management::BudgetPeriod,
            crate::resource_governance::budget_management::AlertThreshold,
            crate::resource_governance::budget_management::BudgetAlert,
            crate::resource_governance::budget_management::BudgetUsage,
            // RBAC Management DTOs (Story 8)
            crate::identity_access::rbac::Role,
            crate::identity_access::rbac::Permission,
            crate::identity_access::rbac::ResourceType,
            crate::identity_access::rbac::User,
            crate::identity_access::rbac::RoleAssignment,
            crate::identity_access::rbac::PermissionGrant,
            crate::identity_access::rbac::AccessDecision,
            crate::identity_access::rbac::AuthToken,
            crate::identity_access::rbac::Session,
            crate::identity_access::rbac::LoginRequest,
            crate::identity_access::rbac::RevokeRoleRequest,
            crate::identity_access::rbac::CheckPermissionRequest,
        )
    ),
    tags(
        (name = "pipelines", description = "Pipeline management endpoints"),
        (name = "executions", description = "Pipeline execution endpoints"),
        (name = "worker-pools", description = "Resource pool management endpoints"),
        (name = "system", description = "System status and health endpoints"),
        (name = "cost-management", description = "Cost management and budget tracking endpoints"),
        (name = "security", description = "Security vulnerability tracking and compliance endpoints"),
        (name = "auth", description = "Authentication and RBAC management endpoints"),
        (name = "observability", description = "Observability, monitoring, and cluster topology endpoints")
    )
)]
pub struct ApiDoc;

/// Health check response
#[derive(Serialize, Deserialize, ToSchema)]
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

#[utoipa::path(
    get,
    path = "/api/hello-openapi",
    responses(
        (status = 200, description = "Hello OpenAPI")
    )
)]
pub async fn hello_openapi() -> &'static str {
    "Hello OpenAPI"
}

/// Job specification for creating new jobs
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// Name of the job
    pub name: String,
    /// Docker image to use
    pub image: String,
    /// Command to execute
    pub command: Vec<String>,
    /// Resource requirements
    pub resources: ResourceQuotaDto,
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
pub struct JobResponse {
    /// Unique job identifier
    pub id: String,
    /// Job name
    pub name: String,
    /// Job specification
    pub spec: JobSpecDto,
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
pub struct WorkerResponse {
    /// Unique worker identifier
    pub id: String,
    /// Worker name
    pub name: String,
    /// Current status
    pub status: String,
    /// Worker capabilities
    pub capabilities: WorkerCapabilitiesDto,
    /// Last heartbeat timestamp
    pub last_heartbeat: DateTime<Utc>,
}

/// Generic success message
#[derive(Serialize, Deserialize, ToSchema)]
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
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Create dynamic worker request
#[derive(Serialize, Deserialize, ToSchema)]
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
pub enum ProviderTypeDto {
    Docker,
    Kubernetes,
}

/// Provider info
#[derive(Serialize, Deserialize, ToSchema)]
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
pub struct CreateTenantRequest {
    /// Tenant name
    pub name: String,
    /// Tenant admin email
    pub email: String,
}

/// Update tenant request (EPIC-09)
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateTenantRequest {
    /// Tenant name
    pub name: String,
    /// Tenant admin email
    pub email: String,
}

/// Tenant response (EPIC-09)
#[derive(Serialize, Deserialize, ToSchema)]
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
