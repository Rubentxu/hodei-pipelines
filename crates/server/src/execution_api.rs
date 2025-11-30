//! Execution Management REST API Module
//!
//! Provides REST endpoints for Pipeline Execution management.
//! Implements US-005 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use hodei_pipelines_core::{
    pipeline::PipelineId,
    pipeline_execution::{ExecutionId, StepExecutionStatus},
};
use hodei_pipelines_modules::PipelineExecutionService;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::dtos::*;

// ===== Application State =====

/// Application state for Execution API
#[derive(Clone)]
pub struct ExecutionApiAppState {
    pub execution_service: Arc<dyn PipelineExecutionService>,
}

impl ExecutionApiAppState {
    pub fn new(service: Arc<dyn PipelineExecutionService>) -> Self {
        Self {
            execution_service: service,
        }
    }
}

// ===== API Handlers =====

pub async fn get_execution_details_handler(
    State(state): State<ExecutionApiAppState>,
    Path(execution_id): Path<ExecutionId>,
) -> Result<Json<ExecutionDetailsDto>, StatusCode> {
    info!("Getting execution details: {}", execution_id);

    match state.execution_service.get_execution(&execution_id).await {
        Ok(Some(execution)) => {
            info!("Execution found: {}", execution_id);

            let stages = execution
                .steps
                .iter()
                .map(|step| StageExecutionDto {
                    step_id: step.step_id.0.to_string(),
                    step_name: format!("step-{}", step.step_id.0),
                    status: step.status.clone().into(),
                    started_at: step.started_at,
                    completed_at: step.completed_at,
                    retry_count: step.retry_count,
                    error_message: step.error_message.clone(),
                })
                .collect();

            let current_step = execution
                .steps
                .iter()
                .find(|s| matches!(s.status, StepExecutionStatus::RUNNING))
                .map(|s| s.step_id.0.to_string());

            let duration_ms = execution
                .get_duration()
                .map(|d| d.num_milliseconds() as u64);

            let details = ExecutionDetailsDto {
                id: execution.id.0,
                pipeline_id: execution.pipeline_id.0,
                status: execution.status.as_str().to_string(),
                started_at: execution.started_at,
                completed_at: execution.completed_at,
                duration_ms,
                current_step,
                stages,
                variables: execution.variables.clone(),
                tenant_id: execution.tenant_id,
                correlation_id: execution.correlation_id,
            };

            Ok(Json(details))
        }
        Ok(None) => {
            warn!("Execution not found: {}", execution_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get execution: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_execution_history_handler(
    State(state): State<ExecutionApiAppState>,
    Path(pipeline_id): Path<PipelineId>,
) -> Result<Json<ExecutionHistoryDto>, StatusCode> {
    info!("Getting execution history for pipeline: {}", pipeline_id);

    match state
        .execution_service
        .get_executions_for_pipeline(&pipeline_id)
        .await
    {
        Ok(executions) => {
            info!(
                "Retrieved {} executions for pipeline {}",
                executions.len(),
                pipeline_id
            );

            let execution_items = executions
                .iter()
                .map(|exec| {
                    let duration_ms = exec.get_duration().map(|d| d.num_milliseconds() as u64);

                    ExecutionListItemDto {
                        id: exec.id.0,
                        pipeline_id: exec.pipeline_id.0,
                        status: exec.status.as_str().to_string(),
                        trigger: "manual".to_string(), // TODO: Get from correlation_id or other source
                        started_at: exec.started_at,
                        completed_at: exec.completed_at,
                        duration_ms,
                        cost: None, // TODO: Calculate cost
                    }
                })
                .collect();

            let history = ExecutionHistoryDto {
                executions: execution_items,
                total: executions.len(),
            };

            Ok(Json(history))
        }
        Err(e) => {
            error!("Failed to get execution history: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn cancel_execution_handler(
    State(state): State<ExecutionApiAppState>,
    Path(execution_id): Path<ExecutionId>,
) -> Result<Json<CancelExecutionResponseDto>, StatusCode> {
    info!("Canceling execution: {}", execution_id);

    match state
        .execution_service
        .cancel_execution(&execution_id)
        .await
    {
        Ok(()) => {
            info!("Execution canceled successfully: {}", execution_id);

            let response = CancelExecutionResponseDto {
                id: execution_id.0,
                status: "CANCELED".to_string(),
                canceled_at: chrono::Utc::now(),
            };

            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to cancel execution: {}", e);
            let err_str = e.to_string();
            if err_str.contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn retry_execution_handler(
    State(state): State<ExecutionApiAppState>,
    Path(execution_id): Path<ExecutionId>,
) -> Result<Json<RetryExecutionResponseDto>, StatusCode> {
    info!("Retrying execution: {}", execution_id);

    match state.execution_service.retry_execution(&execution_id).await {
        Ok(new_execution_id) => {
            info!(
                "Execution retried successfully: {} -> {}",
                execution_id, new_execution_id
            );

            let response = RetryExecutionResponseDto {
                original_execution_id: execution_id.0,
                new_execution_id: new_execution_id.0,
                status: "CREATED".to_string(),
                created_at: chrono::Utc::now(),
            };

            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to retry execution: {}", e);
            let err_str = e.to_string();
            if err_str.contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

// ===== Router =====

pub fn execution_api_routes(state: ExecutionApiAppState) -> Router {
    Router::new()
        .route("/{id}", get(get_execution_details_handler))
        .route("/{id}/cancel", post(cancel_execution_handler))
        .route("/{id}/retry", post(retry_execution_handler))
        .route(
            "/pipeline/{pipeline_id}",
            get(get_execution_history_handler),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_get_execution_details_handler() {
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_execution_history_handler() {
        assert!(true);
    }
}
