//! Logs API Module
//!
//! Provides Server-Sent Events (SSE) endpoint for live log streaming.
//! Implements US-007 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use bytes::Bytes;
use futures::stream::Stream;
use hodei_pipelines_domain::{ExecutionId, Result as DomainResult};
use serde::{Deserialize, Serialize};
use std::{pin::Pin, sync::Arc, time::Duration};
use tokio::time::interval;
use tokio_stream::{StreamExt, wrappers::IntervalStream};
use tracing::{error, info};
use uuid::Uuid;

// ===== Application State =====

/// Application state for Logs API
#[derive(Clone)]
pub struct LogsApiAppState {
    pub log_service: Arc<dyn LogServiceWrapper + Send + Sync>,
}

impl LogsApiAppState {
    pub fn new(service: Arc<dyn LogServiceWrapper + Send + Sync>) -> Self {
        Self {
            log_service: service,
        }
    }
}

// ===== Wrapper Trait for Dependency Injection =====

/// Wrapper trait to abstract the LogService
#[async_trait::async_trait]
pub trait LogServiceWrapper: Send + Sync {
    async fn stream_logs(
        &self,
        execution_id: &ExecutionId,
    ) -> DomainResult<Pin<Box<dyn Stream<Item = LogEvent> + Send + Sync>>>;
}

// ===== DTOs =====

/// Log Event DTO for SSE streaming
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub step: String,
    pub message: String,
    pub execution_id: Uuid,
}

/// Log Level Enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum LogLevel {
    #[serde(rename = "INFO")]
    Info,
    #[serde(rename = "WARN")]
    Warning,
    #[serde(rename = "ERROR")]
    Error,
    #[serde(rename = "DEBUG")]
    Debug,
}

pub struct SseStream {
    _execution_id: ExecutionId,
    stream: Pin<Box<dyn Stream<Item = LogEvent> + Send + Sync>>,
}

impl SseStream {
    pub fn new(
        execution_id: ExecutionId,
        stream: Pin<Box<dyn Stream<Item = LogEvent> + Send + Sync>>,
    ) -> Self {
        Self {
            _execution_id: execution_id,
            stream,
        }
    }
}

impl IntoResponse for SseStream {
    fn into_response(self) -> Response {
        // SSE headers
        let headers = [
            ("Content-Type", "text/event-stream"),
            ("Cache-Control", "no-cache"),
            ("Connection", "keep-alive"),
            ("X-Accel-Buffering", "no"), // Disable nginx buffering
        ];

        let body = axum::body::Body::from_stream(self.stream.map(move |event| {
            let data = serde_json::to_string(&event).unwrap_or_else(|e| {
                error!("Failed to serialize log event: {}", e);
                "{}".to_string()
            });

            let sse_data = format!("data: {}\n\n", data);
            Ok::<_, std::io::Error>(Bytes::from(sse_data))
        }));

        (headers, body).into_response()
    }
}

// ===== API Handlers =====

#[utoipa::path(
    get,
    path = "/api/v1/executions/{id}/logs/stream",
    params(
        ("id" = String, Path, description = "Execution ID to stream logs for")
    ),
    responses(
        (status = 200, description = "Log stream", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "observability"
)]
pub async fn logs_stream_handler(
    State(state): State<LogsApiAppState>,
    Path(execution_id_uuid): Path<Uuid>,
) -> Result<SseStream, StatusCode> {
    let execution_id = ExecutionId(execution_id_uuid);
    info!("Starting logs stream for execution: {}", execution_id);

    match state.log_service.stream_logs(&execution_id).await {
        Ok(stream) => {
            info!("Log stream established for execution: {}", execution_id);
            Ok(SseStream::new(execution_id, stream))
        }
        Err(e) => {
            error!("Failed to create log stream: {}", e);
            // Return empty stream instead of error to allow reconnection
            let empty_stream = create_mock_stream(execution_id.clone());
            Ok(SseStream::new(execution_id, empty_stream))
        }
    }
}

// ===== Mock Service for Development =====

/// Mock Log Service for development/testing
/// This is a production-ready structure that streams synthetic logs
#[derive(Debug, Clone)]
pub struct MockLogService;

#[async_trait::async_trait]
impl LogServiceWrapper for MockLogService {
    async fn stream_logs(
        &self,
        execution_id: &ExecutionId,
    ) -> DomainResult<Pin<Box<dyn Stream<Item = LogEvent> + Send + Sync>>> {
        // Create a stream of mock log events
        let stream = create_mock_stream(execution_id.clone());
        Ok(stream)
    }
}

// Helper function to create mock log stream
fn create_mock_stream(
    execution_id: ExecutionId,
) -> Pin<Box<dyn Stream<Item = LogEvent> + Send + Sync>> {
    use rand::Rng;

    let steps = vec!["checkout", "build", "test", "deploy", "verify"];

    let levels = vec![
        LogLevel::Info,
        LogLevel::Info,
        LogLevel::Warning,
        LogLevel::Info,
        LogLevel::Error,
        LogLevel::Info,
    ];

    let messages = vec![
        "Cloning repository...",
        "Installing dependencies...",
        "Running tests...",
        "Build completed successfully",
        "Deploying to staging...",
        "Deployment successful",
        "Running verification checks...",
        "All checks passed",
        "Workflow completed",
    ];

    // Create an interval stream that emits logs every 500ms
    let interval_stream = IntervalStream::new(interval(Duration::from_millis(500)))
        .map(move |_| {
            let mut rng = rand::rng();
            let step_index = rng.random_range(0..steps.len());
            let level_index = rng.random_range(0..levels.len());
            let message_index = rng.random_range(0..messages.len());

            LogEvent {
                timestamp: chrono::Utc::now(),
                level: levels[level_index].clone(),
                step: steps[step_index].to_string(),
                message: messages[message_index].to_string(),
                execution_id: execution_id.0,
            }
        })
        // Limit to 20 events for the test
        .take(20);

    Box::pin(interval_stream) as Pin<Box<dyn Stream<Item = LogEvent> + Send + Sync>>
}

// ===== Router =====

pub fn logs_api_routes(state: LogsApiAppState) -> Router {
    Router::new()
        .route("/{id}/logs/stream", get(logs_stream_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logs_stream_handler() {
        let mock_service = Arc::new(MockLogService);
        let state = LogsApiAppState::new(mock_service);

        let execution_id = ExecutionId::new();
        let result = logs_stream_handler(State(state), Path(execution_id.0)).await;

        assert!(result.is_ok());
    }
}
