use axum::response::{IntoResponse, Json};
use axum::routing::{get, put};
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

/// Observability metric
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ObservabilityMetric {
    pub metric_name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub labels: HashMap<String, String>,
    pub source: String,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServiceHealth {
    pub service_name: String,
    pub status: HealthStatus,
    pub uptime: u64,
    pub last_check: DateTime<Utc>,
    pub dependencies: Vec<DependencyHealth>,
}

/// Health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Dependency health
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DependencyHealth {
    pub name: String,
    pub status: HealthStatus,
    pub response_time_ms: f64,
    pub last_success: DateTime<Utc>,
}

/// Tracing span
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TraceSpan {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: f64,
    pub tags: HashMap<String, String>,
    pub logs: Vec<SpanLog>,
}

/// Span log entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SpanLog {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub fields: HashMap<String, String>,
}

/// Log level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerformanceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_io_bytes_per_sec: u64,
    pub active_connections: u32,
    pub request_rate_per_sec: f64,
    pub average_response_time_ms: f64,
    pub error_rate_percent: f64,
}

/// Error tracking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorEvent {
    pub id: String,
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub severity: ErrorSeverity,
    pub service: String,
    pub user_id: Option<String>,
    pub request_id: Option<String>,
}

/// Error severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub action: String,
    pub resource: String,
    pub outcome: AuditOutcome,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub details: HashMap<String, String>,
}

/// Audit outcome
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    Forbidden,
}

/// Cluster topology structures (US-10.3)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterTopology {
    pub nodes: Vec<ClusterNode>,
    pub edges: Vec<ClusterEdge>,
    pub total_workers: u32,
    pub active_workers: u32,
    pub timestamp: DateTime<Utc>,
}

/// Individual node in the cluster
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterNode {
    pub id: String,
    pub node_type: NodeType,
    pub name: String,
    pub status: HealthStatus,
    pub capabilities: NodeCapabilities,
    pub metadata: HashMap<String, String>,
}

/// Type of cluster node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum NodeType {
    #[serde(rename_all = "snake_case")]
    ControlPlane,
    Worker,
    Storage,
    LoadBalancer,
}

/// Node capabilities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeCapabilities {
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub storage_gb: u64,
    pub gpu_count: Option<u32>,
    pub network_bandwidth_mbps: u64,
}

/// Connection/relationship between nodes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterEdge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
    pub latency_ms: f64,
    pub bandwidth_mbps: u64,
}

/// Type of relationship between nodes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum EdgeType {
    #[serde(rename_all = "snake_case")]
    Network,
    Storage,
    Control,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ObservabilityConfig {
    pub enabled: bool,
    pub sampling_rate: f64,
    pub metrics_retention_days: u32,
    pub traces_retention_days: u32,
    pub logs_retention_days: u32,
    pub enable_performance_monitoring: bool,
    pub enable_error_tracking: bool,
    pub enable_audit_logging: bool,
    pub external_tracing_endpoint: Option<String>,
    pub external_metrics_endpoint: Option<String>,
}

/// Service for observability API
#[derive(Debug)]
pub struct ObservabilityApiService {
    /// Configuration
    config: Arc<RwLock<ObservabilityConfig>>,
    /// Health status
    health_status: Arc<RwLock<ServiceHealth>>,
    /// Recent metrics
    metrics: Arc<RwLock<Vec<ObservabilityMetric>>>,
    /// Recent error events
    error_events: Arc<RwLock<Vec<ErrorEvent>>>,
    /// Audit logs
    audit_logs: Arc<RwLock<Vec<AuditLog>>>,
}

impl ObservabilityApiService {
    /// Create new observability API service
    pub fn new() -> Self {
        let default_config = ObservabilityConfig {
            enabled: true,
            sampling_rate: 1.0,
            metrics_retention_days: 30,
            traces_retention_days: 7,
            logs_retention_days: 30,
            enable_performance_monitoring: true,
            enable_error_tracking: true,
            enable_audit_logging: true,
            external_tracing_endpoint: None,
            external_metrics_endpoint: None,
        };

        let health = ServiceHealth {
            service_name: "hodei-server".to_string(),
            status: HealthStatus::Healthy,
            uptime: 3600,
            last_check: Utc::now(),
            dependencies: vec![],
        };

        Self {
            config: Arc::new(RwLock::new(default_config)),
            health_status: Arc::new(RwLock::new(health)),
            metrics: Arc::new(RwLock::new(Vec::new())),
            error_events: Arc::new(RwLock::new(Vec::new())),
            audit_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get service health
    pub async fn get_service_health(&self) -> ServiceHealth {
        let health = self.health_status.read().await;
        health.clone()
    }

    /// Update service health
    pub async fn update_service_health(&self, health: ServiceHealth) {
        let mut health_lock = self.health_status.write().await;
        *health_lock = health;
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        // Mock implementation - would collect real metrics in production
        PerformanceMetrics {
            cpu_usage_percent: 45.2,
            memory_usage_bytes: 1024 * 1024 * 512,
            memory_usage_percent: 60.0,
            disk_usage_percent: 35.5,
            network_io_bytes_per_sec: 1024 * 1024,
            active_connections: 25,
            request_rate_per_sec: 150.0,
            average_response_time_ms: 85.0,
            error_rate_percent: 0.5,
        }
    }

    /// Get recent metrics
    pub async fn get_metrics(&self, limit: Option<usize>) -> Vec<ObservabilityMetric> {
        let metrics = self.metrics.read().await;
        let limit = limit.unwrap_or(100);
        metrics.iter().rev().take(limit).cloned().collect()
    }

    /// Record metric
    pub async fn record_metric(&self, metric: ObservabilityMetric) {
        // Get retention policy before acquiring write lock
        let max_metrics = {
            let config = self.config.read().await;
            config.metrics_retention_days as usize * 24 * 60 // Assuming 1 metric per minute
        };

        let mut metrics = self.metrics.write().await;
        metrics.push(metric);

        // Keep only the most recent metrics based on retention policy
        let current_len = metrics.len();
        if current_len > max_metrics {
            let to_remove = current_len - max_metrics;
            metrics.drain(0..to_remove);
        }
    }

    /// Get error events
    pub async fn get_error_events(&self, limit: Option<usize>) -> Vec<ErrorEvent> {
        let errors = self.error_events.read().await;
        let limit = limit.unwrap_or(100);
        errors.iter().rev().take(limit).cloned().collect()
    }

    /// Record error event
    pub async fn record_error(&self, error: ErrorEvent) {
        // Get max errors before acquiring write lock
        let max_errors = 1000;

        let mut errors = self.error_events.write().await;
        errors.push(error);

        // Keep only recent errors
        let current_len = errors.len();
        if current_len > max_errors {
            let to_remove = current_len - max_errors;
            errors.drain(0..to_remove);
        }
    }

    /// Get audit logs
    pub async fn get_audit_logs(&self, limit: Option<usize>) -> Vec<AuditLog> {
        let logs = self.audit_logs.read().await;
        let limit = limit.unwrap_or(100);
        logs.iter().rev().take(limit).cloned().collect()
    }

    /// Record audit log
    pub async fn record_audit_log(&self, log: AuditLog) {
        // Get max logs before acquiring write lock
        let max_logs = 10000;

        let mut logs = self.audit_logs.write().await;
        logs.push(log);

        // Keep only recent logs
        let current_len = logs.len();
        if current_len > max_logs {
            let to_remove = current_len - max_logs;
            logs.drain(0..to_remove);
        }
    }

    /// Get trace spans
    pub async fn get_trace_spans(&self, trace_id: &str) -> Vec<TraceSpan> {
        // Mock implementation - would query tracing system in production
        vec![TraceSpan {
            span_id: "span-123".to_string(),
            trace_id: trace_id.to_string(),
            parent_span_id: None,
            operation_name: "test_operation".to_string(),
            start_time: Utc::now() - chrono::Duration::milliseconds(100),
            end_time: Utc::now(),
            duration_ms: 100.0,
            tags: HashMap::new(),
            logs: vec![],
        }]
    }

    /// Get configuration
    pub async fn get_config(&self) -> ObservabilityConfig {
        let config = self.config.read().await;
        config.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, config: ObservabilityConfig) {
        let mut config_lock = self.config.write().await;
        *config_lock = config;
    }

    /// Get cluster topology (US-10.3)
    pub async fn get_cluster_topology(&self) -> ClusterTopology {
        // Mock implementation - would collect real topology data in production
        let nodes = vec![
            ClusterNode {
                id: "control-plane-1".to_string(),
                node_type: NodeType::ControlPlane,
                name: "Control Plane Node".to_string(),
                status: HealthStatus::Healthy,
                capabilities: NodeCapabilities {
                    cpu_cores: 4,
                    memory_gb: 16,
                    storage_gb: 100,
                    gpu_count: None,
                    network_bandwidth_mbps: 10000,
                },
                metadata: HashMap::from([
                    ("region".to_string(), "us-west-1".to_string()),
                    ("zone".to_string(), "a".to_string()),
                ]),
            },
            ClusterNode {
                id: "worker-1".to_string(),
                node_type: NodeType::Worker,
                name: "Worker Node 1".to_string(),
                status: HealthStatus::Healthy,
                capabilities: NodeCapabilities {
                    cpu_cores: 16,
                    memory_gb: 64,
                    storage_gb: 500,
                    gpu_count: Some(1),
                    network_bandwidth_mbps: 10000,
                },
                metadata: HashMap::from([
                    ("region".to_string(), "us-west-1".to_string()),
                    ("zone".to_string(), "b".to_string()),
                ]),
            },
            ClusterNode {
                id: "worker-2".to_string(),
                node_type: NodeType::Worker,
                name: "Worker Node 2".to_string(),
                status: HealthStatus::Healthy,
                capabilities: NodeCapabilities {
                    cpu_cores: 16,
                    memory_gb: 64,
                    storage_gb: 500,
                    gpu_count: Some(2),
                    network_bandwidth_mbps: 10000,
                },
                metadata: HashMap::from([
                    ("region".to_string(), "us-west-1".to_string()),
                    ("zone".to_string(), "c".to_string()),
                ]),
            },
            ClusterNode {
                id: "storage-1".to_string(),
                node_type: NodeType::Storage,
                name: "Storage Node".to_string(),
                status: HealthStatus::Healthy,
                capabilities: NodeCapabilities {
                    cpu_cores: 8,
                    memory_gb: 32,
                    storage_gb: 10000,
                    gpu_count: None,
                    network_bandwidth_mbps: 25000,
                },
                metadata: HashMap::from([
                    ("region".to_string(), "us-west-1".to_string()),
                    ("zone".to_string(), "a".to_string()),
                ]),
            },
        ];

        let edges = vec![
            ClusterEdge {
                source: "control-plane-1".to_string(),
                target: "worker-1".to_string(),
                edge_type: EdgeType::Control,
                latency_ms: 0.5,
                bandwidth_mbps: 10000,
            },
            ClusterEdge {
                source: "control-plane-1".to_string(),
                target: "worker-2".to_string(),
                edge_type: EdgeType::Control,
                latency_ms: 0.7,
                bandwidth_mbps: 10000,
            },
            ClusterEdge {
                source: "worker-1".to_string(),
                target: "storage-1".to_string(),
                edge_type: EdgeType::Storage,
                latency_ms: 1.2,
                bandwidth_mbps: 10000,
            },
            ClusterEdge {
                source: "worker-2".to_string(),
                target: "storage-1".to_string(),
                edge_type: EdgeType::Storage,
                latency_ms: 1.1,
                bandwidth_mbps: 10000,
            },
        ];

        let total_workers = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Worker))
            .count() as u32;

        let active_workers = nodes
            .iter()
            .filter(|n| {
                matches!(n.node_type, NodeType::Worker) && matches!(n.status, HealthStatus::Healthy)
            })
            .count() as u32;

        ClusterTopology {
            nodes,
            edges,
            total_workers,
            active_workers,
            timestamp: Utc::now(),
        }
    }
}

/// Get service health
#[utoipa::path(
    get,
    path = "/api/v1/observability/health",
    responses(
        (status = 200, description = "Service health retrieved successfully", body = ServiceHealth)
    ),
    tag = "Observability API"
)]
pub async fn get_service_health(
    State(state): State<ObservabilityApiAppState>,
) -> Result<Json<ServiceHealth>, StatusCode> {
    let health = state.service.get_service_health().await;
    Ok(Json(health))
}

/// Get performance metrics
#[utoipa::path(
    get,
    path = "/api/v1/observability/performance",
    responses(
        (status = 200, description = "Performance metrics retrieved successfully", body = PerformanceMetrics)
    ),
    tag = "Observability API"
)]
pub async fn get_performance_metrics(
    State(state): State<ObservabilityApiAppState>,
) -> Result<Json<PerformanceMetrics>, StatusCode> {
    let metrics = state.service.get_performance_metrics().await;
    Ok(Json(metrics))
}

/// Get metrics
#[utoipa::path(
    get,
    path = "/api/v1/observability/metrics",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of metrics to return")
    ),
    responses(
        (status = 200, description = "Metrics retrieved successfully", body = Vec<ObservabilityMetric>)
    ),
    tag = "Observability API"
)]
pub async fn get_metrics(
    State(state): State<ObservabilityApiAppState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<ObservabilityMetric>>, StatusCode> {
    let limit = params.get("limit").and_then(|s| s.parse().ok());
    let metrics = state.service.get_metrics(limit).await;
    Ok(Json(metrics))
}

/// Get error events
#[utoipa::path(
    get,
    path = "/api/v1/observability/errors",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of errors to return")
    ),
    responses(
        (status = 200, description = "Error events retrieved successfully", body = Vec<ErrorEvent>)
    ),
    tag = "Observability API"
)]
pub async fn get_error_events(
    State(state): State<ObservabilityApiAppState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<ErrorEvent>>, StatusCode> {
    let limit = params.get("limit").and_then(|s| s.parse().ok());
    let errors = state.service.get_error_events(limit).await;
    Ok(Json(errors))
}

/// Get audit logs
#[utoipa::path(
    get,
    path = "/api/v1/observability/audit",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of logs to return")
    ),
    responses(
        (status = 200, description = "Audit logs retrieved successfully", body = Vec<AuditLog>)
    ),
    tag = "Observability API"
)]
pub async fn get_audit_logs(
    State(state): State<ObservabilityApiAppState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<AuditLog>>, StatusCode> {
    let limit = params.get("limit").and_then(|s| s.parse().ok());
    let logs = state.service.get_audit_logs(limit).await;
    Ok(Json(logs))
}

/// Get trace spans
#[utoipa::path(
    get,
    path = "/api/v1/observability/traces/{trace_id}",
    params(
        ("trace_id" = String, Path, description = "Trace ID")
    ),
    responses(
        (status = 200, description = "Trace spans retrieved successfully", body = Vec<TraceSpan>)
    ),
    tag = "Observability API"
)]
pub async fn get_trace_spans(
    Path(trace_id): Path<String>,
    State(state): State<ObservabilityApiAppState>,
) -> Result<Json<Vec<TraceSpan>>, StatusCode> {
    let spans = state.service.get_trace_spans(&trace_id).await;
    Ok(Json(spans))
}

/// Get observability configuration
#[utoipa::path(
    get,
    path = "/api/v1/observability/config",
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = ObservabilityConfig)
    ),
    tag = "Observability API"
)]
pub async fn get_observability_config(
    State(state): State<ObservabilityApiAppState>,
) -> Result<Json<ObservabilityConfig>, StatusCode> {
    let config = state.service.get_config().await;
    Ok(Json(config))
}

/// Update observability configuration
#[utoipa::path(
    put,
    path = "/api/v1/observability/config",
    request_body = ObservabilityConfig,
    responses(
        (status = 200, description = "Configuration updated successfully"),
        (status = 400, description = "Invalid configuration")
    ),
    tag = "Observability API"
)]
pub async fn update_observability_config(
    State(state): State<ObservabilityApiAppState>,
    Json(config): Json<ObservabilityConfig>,
) -> Result<Json<String>, StatusCode> {
    state.service.update_config(config).await;
    Ok(Json("Configuration updated successfully".to_string()))
}

/// Get cluster topology (US-10.3)
#[utoipa::path(
    get,
    path = "/api/v1/observability/topology",
    responses(
        (status = 200, description = "Cluster topology retrieved successfully", body = ClusterTopology)
    ),
    tag = "Observability API"
)]
pub async fn get_cluster_topology(
    State(state): State<ObservabilityApiAppState>,
) -> Result<Json<ClusterTopology>, StatusCode> {
    let topology = state.service.get_cluster_topology().await;
    Ok(Json(topology))
}

/// Application state for Observability API
#[derive(Clone)]
pub struct ObservabilityApiAppState {
    pub service: Arc<ObservabilityApiService>,
}

/// Observability API routes
pub fn observability_api_routes() -> Router<ObservabilityApiAppState> {
    Router::new()
        .route("/observability/health", get(get_service_health))
        .route("/observability/performance", get(get_performance_metrics))
        .route("/observability/metrics", get(get_metrics))
        .route("/observability/errors", get(get_error_events))
        .route("/observability/audit", get(get_audit_logs))
        .route("/observability/traces/{trace_id}", get(get_trace_spans))
        .route("/observability/config", get(get_observability_config))
        .route("/observability/config", put(update_observability_config))
        // Cluster topology endpoint (US-10.3)
        .route("/observability/topology", get(get_cluster_topology))
        .route("/observability/metrics/stream", get(metrics_stream_handler))
        .route("/observability/logs/stream", get(logs_stream_handler))
        .route(
            "/observability/topology/stream",
            get(topology_stream_handler),
        )
        .route(
            "/observability/sla/violations/stream",
            get(sla_violations_stream_handler),
        )
        .route("/observability/sla/violations", get(get_sla_violations))
}

/// Get SLA violations
#[utoipa::path(
    get,
    path = "/api/v1/observability/sla/violations",
    responses(
        (status = 200, description = "SLA violations list", body = String)
    ),
    tag = "Observability API"
)]
pub async fn get_sla_violations(
    State(_state): State<ObservabilityApiAppState>,
) -> impl IntoResponse {
    let violations = vec![serde_json::json!({
        "id": "sla-1",
        "message": "High latency detected",
        "severity": "warning",
        "timestamp": Utc::now()
    })];
    Json(serde_json::json!({
        "violations": violations,
        "total": 1,
        "criticalCount": 0,
        "warningCount": 1
    }))
}
/// Stream metrics (SSE) - TODO: Implement with WebSocket
#[utoipa::path(
    get,
    path = "/api/v1/observability/metrics/stream",
    responses(
        (status = 200, description = "Metrics stream", body = String)
    ),
    tag = "Observability API"
)]
pub async fn metrics_stream_handler(
    State(_state): State<ObservabilityApiAppState>,
) -> impl IntoResponse {
    Json(json!({
        "status": "not_implemented",
        "message": "SSE stream not implemented yet, use WebSocket instead"
    }))
}

/// Stream logs (SSE) - TODO: Implement with WebSocket
#[utoipa::path(
    get,
    path = "/api/v1/observability/logs/stream",
    responses(
        (status = 200, description = "Logs stream", body = String)
    ),
    tag = "Observability API"
)]
pub async fn logs_stream_handler(
    State(_state): State<ObservabilityApiAppState>,
) -> impl IntoResponse {
    Json(json!({
        "status": "not_implemented",
        "message": "SSE stream not implemented yet, use WebSocket instead"
    }))
}

/// Stream topology (SSE) - TODO: Implement with WebSocket
#[utoipa::path(
    get,
    path = "/api/v1/observability/topology/stream",
    responses(
        (status = 200, description = "Topology stream", body = String)
    ),
    tag = "Observability API"
)]
pub async fn topology_stream_handler(
    State(_state): State<ObservabilityApiAppState>,
) -> impl IntoResponse {
    Json(json!({
        "status": "not_implemented",
        "message": "SSE stream not implemented yet, use WebSocket instead"
    }))
}

/// Stream SLA violations (SSE) - TODO: Implement with WebSocket
#[utoipa::path(
    get,
    path = "/api/v1/observability/sla/violations/stream",
    responses(
        (status = 200, description = "SLA violations stream", body = String)
    ),
    tag = "Observability API"
)]
pub async fn sla_violations_stream_handler(
    State(_state): State<ObservabilityApiAppState>,
) -> impl IntoResponse {
    Json(json!({
        "status": "not_implemented",
        "message": "SSE stream not implemented yet, use WebSocket instead"
    }))
}
