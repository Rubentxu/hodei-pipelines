//! Centralized API Router
//!
//! This module provides a single point of entry for all API routes
//! Used by both the main server and integration tests

use axum::Router;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::bootstrap::ServerComponents;
use crate::execution_api::{ExecutionApiAppState, ExecutionServiceWrapper, execution_api_routes};
use crate::logs_api::{LogsApiAppState, MockLogService, logs_api_routes};
use crate::pipeline_api::{PipelineApiAppState, PipelineServiceWrapper, pipeline_api_routes};
use crate::resource_pool_crud::{ResourcePoolCrudAppState, resource_pool_crud_routes};
use crate::terminal::{TerminalAppState, terminal_routes};
use axum::routing::get;
use hodei_adapters::websocket_handler;

use hodei_core::{
    DomainError, ExecutionId, Pipeline, PipelineId, Result as CoreResult,
    pipeline_execution::PipelineExecution,
};
use hodei_modules::{
    CreatePipelineRequest, ExecutePipelineRequest, ListPipelinesFilter, UpdatePipelineRequest,
};

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

    // Initialize terminal state
    let terminal_state = TerminalAppState::default();

    info!("✅ Resource Pool CRUD routes initialized");
    info!("✅ Pipeline API routes initialized");
    info!("✅ Execution API routes initialized");
    info!("✅ Logs API routes initialized");
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
                // WebSocket Terminal Interactive (US-008)
                .nest("/terminal", terminal_routes().with_state(terminal_state))
                // Observability routes
                .nest("/observability", observability_routes()),
        )
        // Add OpenAPI documentation routes (US-10.5)
        .nest("/api/docs", create_openapi_docs_routes())
        // WebSocket route (US-009)
        .route(
            "/ws",
            get(websocket_handler).with_state(server_components.event_subscriber),
        )
}

/// Simple observability routes for testing
fn observability_routes() -> axum::Router {
    axum::Router::new().route(
        "/topology",
        axum::routing::get(|| async {
            (
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({
                    "nodes": [],
                    "edges": [],
                    "total_workers": 0
                })),
            )
        }),
    )
}

/// OpenAPI documentation routes
fn create_openapi_docs_routes() -> axum::Router {
    use axum::http::StatusCode;

    axum::Router::new()
        .route(
            "/openapi.json",
            axum::routing::get(|| async {
                (
                    StatusCode::OK,
                    r#"{"openapi":"3.0.0","info":{"title":"Hodei API","version":"1.0"},"paths":{}}"#.to_string()
                )
            }),
        )
        .route("/", axum::routing::get(|| async {
            (
                StatusCode::FOUND,
                [("Location", "/api/docs")]
            )
        }))
}
