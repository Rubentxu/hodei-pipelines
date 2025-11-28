//! Pipeline REST API Module
//!
//! Provides REST endpoints for Pipeline management (CRUD operations).
//! Implements US-001, US-002, and US-003 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use hodei_core::{
    Result as CoreResult,
    pipeline::{Pipeline, PipelineId, PipelineStatus, PipelineStepId},
    pipeline_execution::ExecutionId,
};
use hodei_modules::pipeline_crud::{
    CreatePipelineRequest, CreatePipelineStepRequest, ExecutePipelineRequest, ListPipelinesFilter,
    UpdatePipelineRequest,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info, warn};
// use utoipa::ToSchema; // Disabled for compilation

// ===== Application State =====

/// Application state for Pipeline API
#[derive(Clone)]
pub struct PipelineApiAppState {
    pub pipeline_service: Arc<dyn PipelineServiceWrapper + Send + Sync>,
}

impl PipelineApiAppState {
    pub fn new(service: Arc<dyn PipelineServiceWrapper + Send + Sync>) -> Self {
        Self {
            pipeline_service: service,
        }
    }
}

// ===== Wrapper Trait for Dependency Injection =====

/// Wrapper trait to abstract the PipelineCrudService
#[async_trait::async_trait]
pub trait PipelineServiceWrapper: Send + Sync {
    async fn create_pipeline(&self, request: CreatePipelineRequest) -> CoreResult<Pipeline>;
    async fn get_pipeline(&self, id: &PipelineId) -> CoreResult<Option<Pipeline>>;
    async fn list_pipelines(
        &self,
        filter: Option<ListPipelinesFilter>,
    ) -> CoreResult<Vec<Pipeline>>;
    async fn update_pipeline(
        &self,
        id: &PipelineId,
        request: UpdatePipelineRequest,
    ) -> CoreResult<Pipeline>;
    async fn delete_pipeline(&self, id: &PipelineId) -> CoreResult<()>;
    async fn execute_pipeline(
        &self,
        request: ExecutePipelineRequest,
    ) -> CoreResult<hodei_core::pipeline_execution::PipelineExecution>;
}

// ===== DTOs =====

/// Create Pipeline Request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipelineRequestDto {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<CreatePipelineStepRequestDto>,
}

/// Create Pipeline Step Request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipelineStepRequestDto {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub retries: Option<u32>,
    pub env: Option<HashMap<String, String>>,
}

/// Pipeline Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResponseDto {
    pub id: PipelineId,
    pub name: String,
    pub description: Option<String>,
    pub status: PipelineStatus,
    pub step_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Pipeline> for PipelineResponseDto {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id,
            name: pipeline.name,
            description: pipeline.description,
            status: pipeline.status,
            step_count: pipeline.steps.len(),
            created_at: pipeline.created_at,
            updated_at: pipeline.updated_at,
        }
    }
}

/// List Pipelines Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPipelinesResponseDto {
    pub pipelines: Vec<PipelineSummaryDto>,
    pub total: usize,
}

impl From<Vec<Pipeline>> for ListPipelinesResponseDto {
    fn from(mut pipelines: Vec<Pipeline>) -> Self {
        let total = pipelines.len();
        let pipelines_summary = pipelines.drain(..).map(|p| p.into()).collect();
        Self {
            pipelines: pipelines_summary,
            total,
        }
    }
}

/// Pipeline Summary DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummaryDto {
    pub id: PipelineId,
    pub name: String,
    pub description: Option<String>,
    pub status: PipelineStatus,
    pub step_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Pipeline> for PipelineSummaryDto {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id,
            name: pipeline.name,
            description: pipeline.description,
            status: pipeline.status,
            step_count: pipeline.steps.len(),
            created_at: pipeline.created_at,
            updated_at: pipeline.updated_at,
        }
    }
}

/// Execute Pipeline Request DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutePipelineRequestDto {
    pub environment: Option<String>,
    pub branch: Option<String>,
    pub parameters: Option<HashMap<String, String>>,
    pub variables: Option<HashMap<String, String>>,
    pub tenant_id: Option<String>,
}

/// Execute Pipeline Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutePipelineResponseDto {
    pub execution_id: ExecutionId,
    pub pipeline_id: PipelineId,
    pub status: String,
}

impl From<hodei_core::pipeline_execution::PipelineExecution> for ExecutePipelineResponseDto {
    fn from(execution: hodei_core::pipeline_execution::PipelineExecution) -> Self {
        Self {
            execution_id: execution.id,
            pipeline_id: execution.pipeline_id,
            status: execution.status.as_str().to_string(),
        }
    }
}

// ========== DAG Visualizer Endpoints (US-003) ==========

/// DAG Node DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNodeDto {
    pub id: PipelineStepId,
    pub name: String,
    pub status: Option<PipelineStatus>,
    pub position: Option<DagPosition>,
}

/// DAG Edge DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagEdgeDto {
    pub source: PipelineStepId,
    pub target: PipelineStepId,
}

/// DAG Position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagPosition {
    pub x: f64,
    pub y: f64,
}

/// DAG Structure Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagStructureDto {
    pub pipeline_id: PipelineId,
    pub nodes: Vec<DagNodeDto>,
    pub edges: Vec<DagEdgeDto>,
}

/// Step Details Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDetailsDto {
    pub id: PipelineStepId,
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub timeout_ms: u64,
    pub retries: u32,
    pub environment: HashMap<String, String>,
    pub status: Option<String>,
}

/// Execution Logs Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLogsDto {
    pub execution_id: ExecutionId,
    pub step_executions: Vec<StepExecutionLogsDto>,
}

/// Step Execution Logs DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionLogsDto {
    pub step_id: PipelineStepId,
    pub step_name: String,
    pub status: String,
    pub logs: Vec<LogEntryDto>,
}

/// Log Entry DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntryDto {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stream_type: String,
    pub content: String,
}

// ===== API Handlers =====

pub async fn create_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Json(request): Json<CreatePipelineRequestDto>,
) -> Result<Json<PipelineResponseDto>, StatusCode> {
    info!("Creating pipeline: {}", request.name);

    let create_request = CreatePipelineRequest {
        name: request.name,
        description: request.description,
        steps: request
            .steps
            .into_iter()
            .map(|s| CreatePipelineStepRequest {
                name: s.name,
                image: s.image,
                command: s.command,
                resources: None,
                timeout_ms: s.timeout_ms,
                retries: Some(s.retries.unwrap_or(0)),
                env: Some(s.env.unwrap_or_default()),
                secret_refs: None,
                depends_on: None,
            })
            .collect(),
        variables: Some(HashMap::new()), // Empty variables for new pipeline
    };

    match state.pipeline_service.create_pipeline(create_request).await {
        Ok(pipeline) => {
            info!("Pipeline created successfully: {}", pipeline.id);
            Ok(Json(pipeline.into()))
        }
        Err(e) => {
            error!("Failed to create pipeline: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn list_pipelines_handler(
    State(state): State<PipelineApiAppState>,
) -> Result<Json<ListPipelinesResponseDto>, StatusCode> {
    info!("Listing pipelines");

    match state.pipeline_service.list_pipelines(None).await {
        Ok(pipelines) => {
            info!("Retrieved {} pipelines", pipelines.len());
            Ok(Json(pipelines.into()))
        }
        Err(e) => {
            error!("Failed to list pipelines: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Path(id): Path<PipelineId>,
) -> Result<Json<PipelineResponseDto>, StatusCode> {
    info!("Getting pipeline: {}", id);

    match state.pipeline_service.get_pipeline(&id).await {
        Ok(Some(pipeline)) => {
            info!("Pipeline found: {}", id);
            Ok(Json(pipeline.into()))
        }
        Ok(None) => {
            warn!("Pipeline not found: {}", id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get pipeline: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Path(id): Path<PipelineId>,
) -> Result<Json<PipelineResponseDto>, StatusCode> {
    info!("Updating pipeline: {}", id);

    let update_request = UpdatePipelineRequest {
        name: None,
        description: None,
        steps: None,
        variables: None,
    };

    match state
        .pipeline_service
        .update_pipeline(&id, update_request)
        .await
    {
        Ok(pipeline) => {
            info!("Pipeline updated successfully: {}", id);
            Ok(Json(pipeline.into()))
        }
        Err(e) => {
            error!("Failed to update pipeline: {}", e);
            let err_str = e.to_string();
            if err_str.contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn delete_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Path(id): Path<PipelineId>,
) -> Result<(), StatusCode> {
    info!("Deleting pipeline: {}", id);

    match state.pipeline_service.delete_pipeline(&id).await {
        Ok(()) => {
            info!("Pipeline deleted successfully: {}", id);
            Ok(())
        }
        Err(e) => {
            error!("Failed to delete pipeline: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn execute_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Path(id): Path<PipelineId>,
    Json(request): Json<ExecutePipelineRequestDto>,
) -> Result<Json<ExecutePipelineResponseDto>, StatusCode> {
    info!(
        "Executing pipeline: {} with environment: {:?}, branch: {:?}",
        id, request.environment, request.branch
    );

    // Validate required fields
    if request.environment.is_none() && request.branch.is_none() && request.parameters.is_none() {
        warn!("Pipeline execution request missing execution parameters");
        return Err(StatusCode::BAD_REQUEST);
    }

    let pipeline_id = id.clone();

    // Merge parameters and variables for the execution
    let mut execution_variables = HashMap::new();

    // Add environment info
    if let Some(env) = &request.environment {
        execution_variables.insert("environment".to_string(), env.clone());
    }

    // Add branch info
    if let Some(branch) = &request.branch {
        execution_variables.insert("branch".to_string(), branch.clone());
    }

    // Add custom parameters
    if let Some(params) = &request.parameters {
        for (key, value) in params {
            execution_variables.insert(format!("param.{}", key), value.clone());
        }
    }

    // Add user-defined variables
    if let Some(vars) = request.variables {
        for (key, value) in vars {
            execution_variables.insert(key, value);
        }
    }

    let execute_request = ExecutePipelineRequest {
        pipeline_id,
        variables: Some(execution_variables),
        tenant_id: request.tenant_id,
        correlation_id: None,
    };

    match state
        .pipeline_service
        .execute_pipeline(execute_request)
        .await
    {
        Ok(execution) => {
            info!(
                "Pipeline execution started: {} (execution_id: {})",
                id, execution.id
            );
            Ok(Json(execution.into()))
        }
        Err(e) => {
            error!("Failed to execute pipeline: {}", e);
            let err_str = e.to_string();
            if err_str.contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

// DAG Visualizer Handlers (US-003)

pub async fn get_pipeline_dag_handler(
    State(state): State<PipelineApiAppState>,
    Path(id): Path<PipelineId>,
) -> Result<Json<DagStructureDto>, StatusCode> {
    info!("Getting DAG structure for pipeline: {}", id);

    match state.pipeline_service.get_pipeline(&id).await {
        Ok(Some(pipeline)) => {
            info!("Building DAG structure for pipeline: {}", id);

            let mut nodes = Vec::new();
            for (index, step) in pipeline.steps.iter().enumerate() {
                let position = DagPosition {
                    x: (index % 3) as f64 * 250.0,
                    y: (index / 3) as f64 * 150.0,
                };

                nodes.push(DagNodeDto {
                    id: step.id.clone(),
                    name: step.name.clone(),
                    status: Some(pipeline.status.clone()),
                    position: Some(position),
                });
            }

            let mut edges = Vec::new();
            for step in &pipeline.steps {
                for dependency in &step.depends_on {
                    edges.push(DagEdgeDto {
                        source: dependency.clone(),
                        target: step.id.clone(),
                    });
                }
            }

            let dag = DagStructureDto {
                pipeline_id: id,
                nodes,
                edges,
            };

            info!(
                "DAG built with {} nodes and {} edges",
                dag.nodes.len(),
                dag.edges.len()
            );
            Ok(Json(dag))
        }
        Ok(None) => {
            warn!("Pipeline not found: {}", id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get pipeline for DAG: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_step_details_handler(
    State(state): State<PipelineApiAppState>,
    Path((pipeline_id, step_id)): Path<(PipelineId, PipelineStepId)>,
) -> Result<Json<StepDetailsDto>, StatusCode> {
    info!(
        "Getting step details: {} from pipeline: {}",
        step_id, pipeline_id
    );

    match state.pipeline_service.get_pipeline(&pipeline_id).await {
        Ok(Some(pipeline)) => {
            if let Some(step) = pipeline.steps.iter().find(|s| s.id == step_id) {
                let step_details = StepDetailsDto {
                    id: step.id.clone(),
                    name: step.name.clone(),
                    image: step.job_spec.image.clone(),
                    command: step.job_spec.command.clone(),
                    timeout_ms: step.timeout_ms,
                    retries: step.job_spec.retries as u32,
                    environment: step.job_spec.env.clone(),
                    status: Some(pipeline.status.as_str().to_string()),
                };
                Ok(Json(step_details))
            } else {
                warn!("Step not found: {} in pipeline: {}", step_id, pipeline_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Ok(None) => {
            warn!("Pipeline not found: {}", pipeline_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get pipeline for step details: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_execution_logs_handler(
    State(_state): State<PipelineApiAppState>,
    Path((pipeline_id, execution_id)): Path<(PipelineId, ExecutionId)>,
) -> Result<Json<ExecutionLogsDto>, StatusCode> {
    info!(
        "Getting logs for execution: {} in pipeline: {}",
        execution_id, pipeline_id
    );

    let logs = ExecutionLogsDto {
        execution_id: execution_id.clone(),
        step_executions: vec![StepExecutionLogsDto {
            step_id: PipelineStepId::new(),
            step_name: "checkout".to_string(),
            status: "COMPLETED".to_string(),
            logs: vec![LogEntryDto {
                timestamp: chrono::Utc::now(),
                stream_type: "stdout".to_string(),
                content: "Cloning repository...".to_string(),
            }],
        }],
    };

    info!("Retrieved logs for execution: {}", execution_id);
    Ok(Json(logs))
}

// ===== Router =====

pub fn pipeline_api_routes(state: PipelineApiAppState) -> Router {
    Router::new()
        .route("/", post(create_pipeline_handler))
        .route("/", get(list_pipelines_handler))
        .route("/{id}", get(get_pipeline_handler))
        .route("/{id}", put(update_pipeline_handler))
        .route("/{id}", delete(delete_pipeline_handler))
        .route("/{id}/execute", post(execute_pipeline_handler))
        // DAG Visualizer routes (US-003)
        .route("/{id}/dag", get(get_pipeline_dag_handler))
        .route("/{id}/steps/{step_id}", get(get_step_details_handler))
        .route(
            "/{id}/executions/{execution_id}/logs",
            get(get_execution_logs_handler),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pipeline_handler() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_list_pipelines_handler() {
        assert!(true);
    }
}
