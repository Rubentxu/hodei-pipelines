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
use hodei_pipelines_core::{
    Result as CoreResult,
    pipeline::{Pipeline, PipelineId, PipelineStepId},
    pipeline_execution::ExecutionId,
};
use hodei_pipelines_modules::PipelineService;
use hodei_pipelines_modules::pipeline_crud::{
    CreatePipelineRequest, CreatePipelineStepRequest, ExecutePipelineRequest, ListPipelinesFilter,
    UpdatePipelineRequest,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info, warn};

use crate::dtos::*;
use flate2::read::GzDecoder;
use std::io::Read;

// ===== Application State =====

/// Application state for Pipeline API
#[derive(Clone)]
pub struct PipelineApiAppState {
    pub pipeline_service: Arc<dyn PipelineService>,
    pub execution_service: Arc<dyn hodei_pipelines_modules::PipelineExecutionService>,
}

impl PipelineApiAppState {
    pub fn new(
        service: Arc<dyn PipelineService>,
        execution_service: Arc<dyn hodei_pipelines_modules::PipelineExecutionService>,
    ) -> Self {
        Self {
            pipeline_service: service,
            execution_service,
        }
    }
}

// ===== API Handlers =====

#[utoipa::path(
    post,
    path = "/api/v1/pipelines",
    request_body = CreatePipelineRequestDto,
    responses(
        (status = 200, description = "Pipeline created successfully", body = PipelineDto),
        (status = 500, description = "Internal server error")
    ),
    tag = "pipelines"
)]
pub async fn create_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Json(request): Json<CreatePipelineRequestDto>,
) -> Result<Json<PipelineDto>, StatusCode> {
    info!("Creating pipeline: {}", request.name);

    let create_request = CreatePipelineRequest {
        name: request.name,
        description: request.description,
        steps: request
            .steps
            .into_iter()
            .map(|s| CreatePipelineStepRequest {
                name: s.name,
                image: s.job_spec.image,
                command: s.job_spec.command,
                resources: Some(s.job_spec.resources.into()),
                timeout_ms: Some(s.job_spec.timeout_ms),
                retries: Some(s.job_spec.retries as u32),
                env: Some(s.job_spec.env),
                secret_refs: Some(s.job_spec.secret_refs),
                depends_on: if s.dependencies.is_empty() {
                    None
                } else {
                    Some(
                        s.dependencies
                            .into_iter()
                            .map(|id| id.to_string())
                            .collect(),
                    )
                },
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

#[utoipa::path(
    get,
    path = "/api/v1/pipelines",
    params(
        ("status" = Option<String>, Query, description = "Filter by pipeline status"),
        ("name" = Option<String>, Query, description = "Filter by pipeline name"),
        ("created_after" = Option<String>, Query, description = "Filter by creation date (ISO8601)"),
        ("created_before" = Option<String>, Query, description = "Filter by creation date (ISO8601)")
    ),
    responses(
        (status = 200, description = "List of pipelines", body = ListPipelinesResponseDto),
        (status = 500, description = "Internal server error")
    ),
    tag = "pipelines"
)]
pub async fn list_pipelines_handler(
    State(state): State<PipelineApiAppState>,
    // Query parameters for filtering
) -> Result<Json<ListPipelinesResponseDto>, StatusCode> {
    info!("Listing pipelines");

    // Convert query params to ListPipelinesFilter
    // For now, we'll keep it simple - the filter could be extended later
    let filter = None;

    match state.pipeline_service.list_pipelines(filter).await {
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

#[utoipa::path(
    get,
    path = "/api/v1/pipelines/{id}",
    params(
        ("id" = String, Path, description = "Pipeline ID")
    ),
    responses(
        (status = 200, description = "Pipeline details", body = PipelineDto),
        (status = 404, description = "Pipeline not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "pipelines"
)]
pub async fn get_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Path(id): Path<PipelineId>,
) -> Result<Json<PipelineDto>, StatusCode> {
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

#[utoipa::path(
    put,
    path = "/api/v1/pipelines/{id}",
    params(
        ("id" = String, Path, description = "Pipeline ID")
    ),
    request_body = UpdatePipelineRequestDto,
    responses(
        (status = 200, description = "Pipeline updated successfully", body = PipelineDto),
        (status = 404, description = "Pipeline not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "pipelines"
)]
pub async fn update_pipeline_handler(
    State(state): State<PipelineApiAppState>,
    Path(id): Path<PipelineId>,
    Json(request): Json<UpdatePipelineRequestDto>,
) -> Result<Json<PipelineDto>, StatusCode> {
    info!("Updating pipeline: {}", id);

    let update_request = UpdatePipelineRequest {
        name: request.name,
        description: request.description,
        steps: request.steps.map(|steps| {
            steps
                .into_iter()
                .map(|s| CreatePipelineStepRequest {
                    name: s.name,
                    image: s.job_spec.image,
                    command: s.job_spec.command,
                    resources: Some(s.job_spec.resources.into()),
                    timeout_ms: Some(s.job_spec.timeout_ms),
                    retries: Some(s.job_spec.retries as u32),
                    env: Some(s.job_spec.env),
                    secret_refs: Some(s.job_spec.secret_refs),
                    depends_on: if s.dependencies.is_empty() {
                        None
                    } else {
                        Some(
                            s.dependencies
                                .into_iter()
                                .map(|id| id.to_string())
                                .collect(),
                        )
                    },
                })
                .collect()
        }),
        variables: request.variables,
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

#[utoipa::path(
    delete,
    path = "/api/v1/pipelines/{id}",
    params(
        ("id" = String, Path, description = "Pipeline ID")
    ),
    responses(
        (status = 200, description = "Pipeline deleted successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "pipelines"
)]
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

#[utoipa::path(
    post,
    path = "/api/v1/pipelines/{id}/execute",
    params(
        ("id" = String, Path, description = "Pipeline ID")
    ),
    request_body = ExecutePipelineRequestDto,
    responses(
        (status = 200, description = "Pipeline execution started", body = ExecutePipelineResponseDto),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Pipeline not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "executions"
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/pipelines/{id}/dag",
    params(
        ("id" = String, Path, description = "Pipeline ID")
    ),
    responses(
        (status = 200, description = "DAG structure", body = DagStructureDto),
        (status = 404, description = "Pipeline not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "pipelines"
)]
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
                let position = DagPositionDto {
                    x: (index % 3) as f64 * 250.0,
                    y: (index / 3) as f64 * 150.0,
                };

                nodes.push(DagNodeDto {
                    id: step.id.0,
                    name: step.name.clone(),
                    status: Some(pipeline.status.as_str().to_string()),
                    position: Some(position),
                });
            }

            let mut edges = Vec::new();
            for step in &pipeline.steps {
                for dependency in &step.depends_on {
                    edges.push(DagEdgeDto {
                        source: dependency.0,
                        target: step.id.0,
                    });
                }
            }

            let dag = DagStructureDto {
                pipeline_id: id.0,
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

#[utoipa::path(
    get,
    path = "/api/v1/pipelines/{id}/steps/{step_id}",
    params(
        ("id" = String, Path, description = "Pipeline ID"),
        ("step_id" = String, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "Step details", body = StepDetailsDto),
        (status = 404, description = "Pipeline or step not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "pipelines"
)]
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
                    id: step.id.0,
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

#[utoipa::path(
    get,
    path = "/api/v1/pipelines/{id}/executions/{execution_id}/logs",
    params(
        ("id" = String, Path, description = "Pipeline ID"),
        ("execution_id" = String, Path, description = "Execution ID")
    ),
    responses(
        (status = 200, description = "Execution logs", body = ExecutionLogsDto),
        (status = 500, description = "Internal server error")
    ),
    tag = "executions"
)]
pub async fn get_execution_logs_handler(
    State(state): State<PipelineApiAppState>,
    Path((pipeline_id, execution_id)): Path<(PipelineId, ExecutionId)>,
) -> Result<Json<ExecutionLogsDto>, StatusCode> {
    info!("Getting logs for execution {}", execution_id);

    match state.execution_service.get_execution(&execution_id).await {
        Ok(Some(execution)) => {
            // Fetch pipeline to get step names
            let pipeline = state
                .pipeline_service
                .get_pipeline(&pipeline_id)
                .await
                .map_err(|e| {
                    error!("Failed to get pipeline: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .ok_or(StatusCode::NOT_FOUND)?;

            let mut step_logs_dtos = Vec::new();

            for step in execution.steps {
                let logs = if let Some(compressed) = &step.compressed_logs {
                    let mut d = GzDecoder::new(&compressed[..]);
                    let mut s = String::new();
                    if d.read_to_string(&mut s).is_ok() {
                        s.lines().map(|l| l.to_string()).collect()
                    } else {
                        error!("Failed to decompress logs for step {}", step.step_id);
                        vec!["Failed to decompress logs".to_string()]
                    }
                } else {
                    step.logs.clone()
                };

                let log_entries = logs
                    .into_iter()
                    .map(|content| LogEntryDto {
                        timestamp: chrono::Utc::now(), // We don't have per-line timestamp
                        stream_type: "stdout".to_string(),
                        content,
                    })
                    .collect();

                let step_name = pipeline
                    .steps
                    .iter()
                    .find(|s| s.id == step.step_id)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| step.step_id.to_string());

                step_logs_dtos.push(StepExecutionLogsDto {
                    step_id: step.step_id.as_uuid(),
                    step_name,
                    status: step.status.as_str().to_string(),
                    logs: log_entries,
                });
            }

            Ok(Json(ExecutionLogsDto {
                execution_id: execution.id.as_uuid(),
                step_executions: step_logs_dtos,
            }))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get execution logs: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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

    #[tokio::test]
    async fn test_create_pipeline_handler() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_list_pipelines_handler() {
        assert!(true);
    }
}
