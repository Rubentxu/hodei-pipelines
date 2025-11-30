//! Centralized API Router
//!
//! This module provides a single point of entry for all API routes
//! Used by both the main server and integration tests

use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::alerting_system::{AlertingApiAppState, AlertingService, alerting_api_routes};
use crate::audit_logs_compliance::{
    AuditLogsComplianceApiAppState, AuditLogsComplianceService, audit_logs_compliance_api_routes,
};
use crate::bootstrap::ServerComponents;
use crate::budget_management::{
    BudgetManagementApiAppState, BudgetManagementService, budget_management_api_routes,
};
use crate::cost_optimization_recommendations::{
    CostOptimizationApiAppState, CostOptimizationService, cost_optimization_api_routes,
};
use crate::cost_tracking_aggregation::{
    CostTrackingApiAppState, CostTrackingService, cost_tracking_api_routes,
};
use crate::execution_api::{ExecutionApiAppState, ExecutionServiceWrapper, execution_api_routes};
use crate::live_metrics_api::{
    LiveMetricsApiAppState, LiveMetricsService, live_metrics_api_routes,
};
use crate::logs_api::{LogsApiAppState, MockLogService, logs_api_routes};
use crate::logs_explorer_ui::{
    LogsExplorerApiAppState, LogsExplorerService, logs_explorer_api_routes,
};
use crate::metrics_api::{
    DashboardMetricsApiAppState, DashboardMetricsService, dashboard_metrics_api_routes,
};
use crate::observability_api::{
    ObservabilityApiAppState, ObservabilityApiService, observability_api_routes,
};
use crate::pipeline_api::{PipelineApiAppState, PipelineServiceWrapper, pipeline_api_routes};
use crate::rbac::{RbacApiAppState, RbacService, rbac_api_routes};
use crate::realtime_status_api::{
    RealtimeStatusApiAppState, RealtimeStatusService, realtime_status_api_routes,
};
use crate::resource_pool_crud::{ResourcePoolCrudAppState, resource_pool_crud_routes};
use crate::security_vulnerability_tracking::{
    SecurityVulnerabilityApiAppState, SecurityVulnerabilityService,
    security_vulnerability_api_routes,
};
use crate::terminal::{TerminalAppState, terminal_routes};
use crate::traces_distributed_tracing::{TracesApiAppState, TracesService, traces_api_routes};
use axum::routing::get;
use hodei_pipelines_adapters::websocket_handler;

use crate::api_docs::{ApiDoc, hello_openapi};
use hodei_pipelines_core::{
    DomainError, ExecutionId, Pipeline, PipelineId, Result as CoreResult,
    pipeline_execution::PipelineExecution,
};
use hodei_pipelines_modules::{
    CreatePipelineRequest, ExecutePipelineRequest, ListPipelinesFilter, UpdatePipelineRequest,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Mock Pipeline Service for development/testing
#[derive(Debug, Clone)]
pub struct MockPipelineService;

/// Mock Execution Service for development/testing
#[derive(Debug, Clone)]
pub struct MockExecutionService;

#[async_trait::async_trait]
impl PipelineServiceWrapper for MockPipelineService {
    async fn create_pipeline(&self, _request: CreatePipelineRequest) -> CoreResult<Pipeline> {
        Err(DomainError::Infrastructure(
            "Pipeline CRUD service not yet initialized in production mode".to_string(),
        ))
    }

    async fn get_pipeline(&self, _id: &PipelineId) -> CoreResult<Option<Pipeline>> {
        Ok(None)
    }

    async fn list_pipelines(
        &self,
        _filter: Option<ListPipelinesFilter>,
    ) -> CoreResult<Vec<Pipeline>> {
        Ok(vec![])
    }

    async fn update_pipeline(
        &self,
        _id: &PipelineId,
        _request: UpdatePipelineRequest,
    ) -> CoreResult<Pipeline> {
        Err(DomainError::Infrastructure(
            "Pipeline CRUD service not yet initialized in production mode".to_string(),
        ))
    }

    async fn delete_pipeline(&self, _id: &PipelineId) -> CoreResult<()> {
        Err(DomainError::Infrastructure(
            "Pipeline CRUD service not yet initialized in production mode".to_string(),
        ))
    }

    async fn execute_pipeline(
        &self,
        _request: ExecutePipelineRequest,
    ) -> CoreResult<PipelineExecution> {
        Err(DomainError::Infrastructure(
            "Pipeline CRUD service not yet initialized in production mode".to_string(),
        ))
    }
}

#[async_trait::async_trait]
impl ExecutionServiceWrapper for MockExecutionService {
    async fn get_execution(&self, _id: &ExecutionId) -> CoreResult<Option<PipelineExecution>> {
        Ok(None)
    }

    async fn get_executions_for_pipeline(
        &self,
        _pipeline_id: &PipelineId,
    ) -> CoreResult<Vec<PipelineExecution>> {
        Ok(vec![])
    }

    async fn cancel_execution(&self, _id: &ExecutionId) -> CoreResult<()> {
        Ok(())
    }

    async fn retry_execution(&self, _id: &ExecutionId) -> CoreResult<ExecutionId> {
        Ok(ExecutionId::new())
    }
}

/// Create centralized API router
pub fn create_api_router(server_components: ServerComponents) -> axum::Router {
    let doc = ApiDoc::openapi();
    println!(
        "DEBUG: ApiDoc JSON: {}",
        doc.to_pretty_json().unwrap_or_default()
    );
    info!("🔧 Setting up centralized API routes...");

    // Initialize resource pool state
    let resource_pool_state = ResourcePoolCrudAppState {
        pools: Arc::new(RwLock::new(HashMap::new())),
        pool_statuses: Arc::new(RwLock::new(HashMap::new())),
    };

    // Initialize pipeline state - with mock service for now
    let mock_pipeline_service = Arc::new(MockPipelineService);
    let pipeline_state = PipelineApiAppState::new(mock_pipeline_service);

    // Initialize execution state - with mock service for now
    let mock_execution_service = Arc::new(MockExecutionService);
    let execution_state = ExecutionApiAppState::new(mock_execution_service);

    // Initialize logs state - with mock service for now
    let mock_log_service = Arc::new(MockLogService);
    let logs_state = LogsApiAppState::new(mock_log_service);

    // Initialize dashboard metrics state
    let dashboard_metrics_service = Arc::new(DashboardMetricsService::new());
    let dashboard_metrics_state = DashboardMetricsApiAppState {
        service: dashboard_metrics_service,
    };

    // Initialize real-time status updates state
    let realtime_status_service = Arc::new(RealtimeStatusService::new());
    let realtime_status_state = RealtimeStatusApiAppState {
        service: realtime_status_service,
    };

    // Initialize live metrics state
    let live_metrics_service = Arc::new(LiveMetricsService::new());
    let live_metrics_state = LiveMetricsApiAppState {
        service: live_metrics_service,
    };

    // Initialize logs explorer state
    let logs_explorer_service = Arc::new(LogsExplorerService::new());
    let logs_explorer_state = LogsExplorerApiAppState {
        service: logs_explorer_service,
    };

    // Initialize traces service
    let traces_service = Arc::new(TracesService::new());
    let traces_state = TracesApiAppState {
        service: traces_service,
    };

    // Initialize alerting service
    let alerting_service = Arc::new(AlertingService::new());
    let alerting_state = AlertingApiAppState {
        service: alerting_service,
    };

    // Initialize cost tracking service
    let cost_tracking_service = Arc::new(CostTrackingService::new());
    let cost_tracking_state = CostTrackingApiAppState {
        service: cost_tracking_service,
    };

    // Initialize cost optimization service
    let cost_optimization_service = Arc::new(CostOptimizationService::new());
    let cost_optimization_state = CostOptimizationApiAppState {
        service: cost_optimization_service,
    };

    // Initialize budget management service
    let budget_management_service = Arc::new(BudgetManagementService::new());
    let budget_management_state = BudgetManagementApiAppState {
        service: budget_management_service,
    };

    // Initialize security & vulnerability tracking service
    let security_vulnerability_service = Arc::new(SecurityVulnerabilityService::new());
    let security_vulnerability_state = SecurityVulnerabilityApiAppState {
        service: security_vulnerability_service,
    };

    // Initialize RBAC service
    let rbac_service = Arc::new(RbacService::new());
    let rbac_state = RbacApiAppState {
        service: rbac_service,
    };

    // Initialize audit logs & compliance service
    let audit_logs_compliance_service = Arc::new(AuditLogsComplianceService::new());
    let audit_logs_compliance_state = AuditLogsComplianceApiAppState {
        service: audit_logs_compliance_service,
    };

    // Initialize observability service
    let observability_service = Arc::new(ObservabilityApiService::new());
    let observability_state = ObservabilityApiAppState {
        service: observability_service,
    };

    // Initialize terminal state
    let terminal_state = TerminalAppState::default();

    info!("✅ Resource Pool CRUD routes initialized");
    info!("✅ Pipeline API routes initialized");
    info!("✅ Execution API routes initialized");
    info!("✅ Logs API routes initialized");
    info!("✅ Logs Explorer API routes initialized");
    info!("✅ Traces API routes initialized");
    info!("✅ Alerting System API routes initialized");
    info!("✅ Cost Tracking & Aggregation API routes initialized");
    info!("✅ Cost Optimization Recommendations API routes initialized");
    info!("✅ Budget Management & Alerts API routes initialized");
    info!("✅ Security Score & Vulnerability Tracking API routes initialized");
    info!("✅ RBAC API routes initialized");
    info!("✅ Audit Logs & Compliance API routes initialized");
    info!("✅ Dashboard Metrics API routes initialized");
    info!("✅ Realtime Status API routes initialized");
    info!("✅ Live Metrics API routes initialized");
    info!("✅ Observability API routes initialized");
    info!("✅ Terminal API routes initialized");

    // Create main router with all routes
    Router::new()
        // Basic health and status endpoints
        .route(
            "/api/health",
            axum::routing::get(|| async { (axum::http::StatusCode::OK, "ok") }),
        )
        .route(
            "/api/server/status",
            axum::routing::get(|| async {
                (
                    axum::http::StatusCode::OK,
                    axum::Json(serde_json::json!({
                        "status": "running",
                        "version": env!("CARGO_PKG_VERSION"),
                        "environment": "production"
                    })),
                )
            }),
        )
        // API v1 routes
        .nest(
            "/api/v1",
            Router::new()
                // Pipeline CRUD routes (US-001, US-002, US-003, US-004)
                .nest("/pipelines", pipeline_api_routes(pipeline_state))
                // Execution Management routes (US-005) and Live Logs SSE (US-007)
                .nest("/executions", execution_api_routes(execution_state))
                .nest("/executions", logs_api_routes(logs_state))
                // Resource Pool CRUD routes (EPIC-10: US-10.1, US-10.2)
                .nest(
                    "/worker-pools",
                    resource_pool_crud_routes().with_state(resource_pool_state),
                )
                // Dashboard Metrics API (US-011)
                .nest(
                    "/metrics",
                    dashboard_metrics_api_routes().with_state(dashboard_metrics_state),
                )
                // Real-time Status Updates via WebSocket (US-009)
                .merge(realtime_status_api_routes().with_state(realtime_status_state))
                // Live Metrics Streaming (US-010)
                .merge(live_metrics_api_routes().with_state(live_metrics_state))
                // Logs Explorer UI (US-012)
                .merge(logs_explorer_api_routes().with_state(logs_explorer_state))
                // Traces & Distributed Tracing (US-013)
                .merge(traces_api_routes().with_state(traces_state))
                // Alerting System (US-014)
                .merge(alerting_api_routes().with_state(alerting_state))
                // Cost Management APIs (US-015, US-016, US-017) - nested under /api/v1/costs
                .nest(
                    "/api/v1/costs",
                    Router::new()
                        .merge(cost_tracking_api_routes().with_state(cost_tracking_state))
                        .merge(cost_optimization_api_routes().with_state(cost_optimization_state))
                        .merge(budget_management_api_routes().with_state(budget_management_state)),
                )
                // Security APIs (US-018) - nested under /api/v1/security
                .nest(
                    "/api/v1/security",
                    security_vulnerability_api_routes().with_state(security_vulnerability_state),
                )
                // RBAC APIs (US-019) - nested under /api/v1/auth
                .nest("/api/v1/auth", rbac_api_routes().with_state(rbac_state))
                // Audit Logs & Compliance Reporting (US-020)
                .merge(audit_logs_compliance_api_routes().with_state(audit_logs_compliance_state))
                // WebSocket Terminal Interactive (US-008)
                .nest("/terminal", terminal_routes().with_state(terminal_state))
                // Complete Observability API routes (Story 12)
                .merge(observability_api_routes().with_state(observability_state)),
        )
        // Add OpenAPI documentation routes (US-10.5)
        .route("/api/hello-openapi", get(hello_openapi))
        .merge(SwaggerUi::new("/swagger-ui").url("/api/docs/spec.json", ApiDoc::openapi()))
        // Generic WebSocket route
        .route(
            "/ws",
            get(websocket_handler).with_state(server_components.event_subscriber),
        )
}
