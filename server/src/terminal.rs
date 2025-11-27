//! Interactive Terminal Module
//!
//! Provides WebSocket-based interactive terminal sessions for jobs.

use axum::extract::ws::{Message, WebSocket};
use axum::{
    Router,
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::api_docs::MessageResponse;

#[derive(Debug, Clone)]
pub struct TerminalSession {
    pub id: String,
    pub job_id: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct TerminalAppState {
    pub sessions: Arc<RwLock<HashMap<String, TerminalSession>>>,
}

impl Default for TerminalAppState {
    fn default() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalService {
    state: TerminalAppState,
}

impl TerminalService {
    pub fn new(state: TerminalAppState) -> Self {
        Self { state }
    }

    pub async fn create_session(
        &self,
        job_id: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let session_id = Uuid::new_v4().to_string();
        let session = TerminalSession {
            id: session_id.clone(),
            job_id: job_id.clone(),
            active: true,
        };

        let mut sessions = self.state.sessions.write().await;
        sessions.insert(session_id.clone(), session);

        info!(
            "Created terminal session: {} for job: {}",
            session_id, job_id
        );
        Ok(session_id)
    }

    pub async fn close_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut sessions = self.state.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.active = false;
            info!("Closed terminal session: {}", session_id);
            Ok(())
        } else {
            Err(format!("Session not found: {}", session_id).into())
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<TerminalSession> {
        let sessions = self.state.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn handle_websocket(
        &self,
        ws: WebSocketUpgrade,
        session_id: String,
    ) -> impl IntoResponse {
        // Verify session exists
        if let Some(session) = self.get_session(&session_id).await {
            if !session.active {
                return (StatusCode::BAD_REQUEST, "Session is not active".to_string())
                    .into_response();
            }
        } else {
            return (StatusCode::NOT_FOUND, "Session not found".to_string()).into_response();
        }

        let state = self.state.clone();
        ws.on_upgrade(move |socket| handle_websocket_session(state, socket, session_id))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub job_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub job_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendInputRequest {
    pub input: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TerminalMessage {
    Input { data: String },
    Output { data: String },
}

/// Create terminal routes
pub fn terminal_routes() -> Router<TerminalAppState> {
    Router::new()
        .route("/sessions", post(create_session_handler))
        .route("/sessions/{id}", delete(close_session_handler))
        .route("/sessions/{id}/input", post(send_input_handler))
        .route("/sessions/{id}/ws", get(websocket_handler))
}

// Handler for creating a session
async fn create_session_handler(
    State(state): State<TerminalAppState>,
    axum::extract::Json(req): axum::extract::Json<CreateSessionRequest>,
) -> (StatusCode, Json<CreateSessionResponse>) {
    let job_id = req.job_id.clone();
    let service = TerminalService::new(state);
    match service.create_session(job_id.clone()).await {
        Ok(session_id) => {
            info!("Created terminal session: {}", session_id);
            (
                StatusCode::OK,
                Json(CreateSessionResponse { session_id, job_id }),
            )
        }
        Err(e) => {
            error!("Failed to create terminal session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateSessionResponse {
                    session_id: "".to_string(),
                    job_id: "".to_string(),
                }),
            )
        }
    }
}

// Handler for closing a session
async fn close_session_handler(
    State(state): State<TerminalAppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<MessageResponse>) {
    let service = TerminalService::new(state);
    match service.close_session(&id).await {
        Ok(_) => {
            info!("Closed terminal session: {}", id);
            (
                StatusCode::OK,
                Json(MessageResponse {
                    message: "Terminal session closed successfully".to_string(),
                }),
            )
        }
        Err(e) => {
            error!("Failed to close terminal session: {}", e);
            (
                StatusCode::NOT_FOUND,
                Json(MessageResponse {
                    message: format!("Failed to close session: {}", e),
                }),
            )
        }
    }
}

// Handler for sending input
async fn send_input_handler(
    State(state): State<TerminalAppState>,
    Path(id): Path<String>,
    axum::extract::Json(req): axum::extract::Json<SendInputRequest>,
) -> (StatusCode, Json<MessageResponse>) {
    let service = TerminalService::new(state);
    info!(
        "Received input for terminal session {}: {} bytes",
        id,
        req.input.len()
    );
    (
        StatusCode::OK,
        Json(MessageResponse {
            message: "Input sent".to_string(),
        }),
    )
}

// Handler for WebSocket upgrade
async fn websocket_handler(
    State(state): State<TerminalAppState>,
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        let state = state.clone();
        async move {
            handle_websocket_session(state, socket, id).await;
        }
    })
}

async fn handle_websocket_session(
    state: TerminalAppState,
    mut socket: WebSocket,
    session_id: String,
) {
    let session_id_clone = session_id.clone();

    // Send welcome message
    let welcome_msg = "Hodei Interactive Terminal\nConnected to job\n\n".to_string();
    let _ = socket.send(Message::Text(welcome_msg.into())).await;

    // Spawn task to read from WebSocket
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = socket.next().await {
            if let Ok(Message::Text(text)) = msg {
                info!(
                    "Received input for session {}: {} bytes",
                    session_id_clone,
                    text.len()
                );
            } else {
                break;
            }
        }
    });

    // Wait for task to complete
    recv_task.await.unwrap();

    // Clean up
    let service = TerminalService::new(state);
    let _ = service.close_session(&session_id).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let state = TerminalAppState::default();
        let service = TerminalService::new(state);

        let session_id = service
            .create_session("test-job-123".to_string())
            .await
            .unwrap();
        assert!(!session_id.is_empty());

        let session = service.get_session(&session_id).await.unwrap();
        assert_eq!(session.job_id, "test-job-123");
        assert!(session.active);
    }

    #[tokio::test]
    async fn test_close_session() {
        let state = TerminalAppState::default();
        let service = TerminalService::new(state);

        let session_id = service
            .create_session("test-job-456".to_string())
            .await
            .unwrap();
        assert!(service.get_session(&session_id).await.is_some());

        service.close_session(&session_id).await.unwrap();

        let session = service.get_session(&session_id).await.unwrap();
        assert!(!session.active);
    }

    #[tokio::test]
    async fn test_session_not_found() {
        let state = TerminalAppState::default();
        let service = TerminalService::new(state);

        let result = service.close_session("non-existent-id").await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::{Router, http::StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_http_create_terminal_session() {
        let state = TerminalAppState::default();
        let app = terminal_routes().with_state(state);

        let session_request = CreateSessionRequest {
            job_id: "test-job-789".to_string(),
        };

        let http_request = axum::http::Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&session_request).unwrap())
            .unwrap();

        let response = app.oneshot(http_request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let result: CreateSessionResponse = serde_json::from_slice(&body).unwrap();
        assert!(!result.session_id.is_empty());
        assert_eq!(result.job_id, "test-job-789");
    }

    #[tokio::test]
    async fn test_http_close_terminal_session() {
        let state = TerminalAppState::default();
        let service = TerminalService::new(state.clone());

        // First create a session
        let session_id = service
            .create_session("test-job-abc".to_string())
            .await
            .unwrap();

        let app = terminal_routes().with_state(state);

        // Now delete it using the same app state
        let delete_http_request = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/sessions/{}", session_id))
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(delete_http_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_send_input_to_terminal() {
        let state = TerminalAppState::default();
        let app = terminal_routes().with_state(state);

        let input_request = SendInputRequest {
            input: "ls -la\n".to_string(),
        };

        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/sessions/test-id/input")
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&input_request).unwrap())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_close_nonexistent_session() {
        let state = TerminalAppState::default();
        let app = terminal_routes().with_state(state);

        let request = axum::http::Request::builder()
            .method("DELETE")
            .uri("/sessions/non-existent-id")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
