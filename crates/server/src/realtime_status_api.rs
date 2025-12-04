//! Real-time Status Updates API (US-009)
//!
//! Provides WebSocket-based real-time status updates for pipeline executions.
//! Supports broadcasting status changes to connected clients.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use futures::StreamExt;
use hodei_pipelines_domain::pipeline_execution::entities::execution::{
    ExecutionId, ExecutionStatus,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Real-time status update DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatusUpdate {
    pub execution_id: ExecutionId,
    pub status: ExecutionStatus,
    pub current_stage: Option<String>,
    pub progress: u8, // 0-100
    pub message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cost: Option<f64>,
    pub duration: Option<u64>, // in seconds
}

/// Application state for Real-time Status API
#[derive(Clone)]
pub struct RealtimeStatusApiAppState {
    pub service: Arc<RealtimeStatusService>,
}

/// Real-time status update service
#[derive(Debug)]
pub struct RealtimeStatusService {
    /// Channels for broadcasting status updates per execution
    channels: Arc<RwLock<HashMap<ExecutionId, broadcast::Sender<ExecutionStatusUpdate>>>>,
    /// Active WebSocket connections
    connections: Arc<RwLock<HashMap<String, broadcast::Receiver<ExecutionStatusUpdate>>>>,
}

impl RealtimeStatusService {
    /// Create new realtime status service
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a channel for an execution
    async fn get_or_create_channel(
        &self,
        execution_id: &ExecutionId,
    ) -> broadcast::Sender<ExecutionStatusUpdate> {
        let mut channels = self.channels.write().await;

        if let Some(sender) = channels.get(execution_id) {
            return sender.clone();
        }

        let (sender, _) = broadcast::channel(100);
        channels.insert(execution_id.clone(), sender.clone());
        sender
    }

    /// Broadcast a status update
    pub async fn broadcast_status_update(&self, update: ExecutionStatusUpdate) {
        let execution_id = update.execution_id.clone();
        let sender = self.get_or_create_channel(&execution_id).await;

        let result = sender.send(update);
        match result {
            Ok(n) => info!("Broadcasted status update to {} subscribers", n),
            Err(e) => warn!("Failed to broadcast status update: {}", e),
        }
    }

    /// Get status updates stream for an execution
    pub async fn get_status_updates(
        &self,
        execution_id: &ExecutionId,
    ) -> broadcast::Receiver<ExecutionStatusUpdate> {
        let sender = self.get_or_create_channel(execution_id).await;
        sender.subscribe()
    }

    /// Simulate status updates (for testing and demo purposes)
    pub async fn simulate_status_updates(&self, execution_id: ExecutionId) {
        let service = self.clone();
        tokio::spawn(async move {
            let stages = vec![
                ("pending", 0),
                ("checkout", 20),
                ("build", 40),
                ("test", 60),
                ("deploy", 80),
                ("verify", 100),
            ];

            for (stage, progress) in stages {
                let update = ExecutionStatusUpdate {
                    execution_id: execution_id.clone(),
                    status: if progress == 100 {
                        ExecutionStatus::COMPLETED
                    } else {
                        ExecutionStatus::RUNNING
                    },
                    current_stage: Some(stage.to_string()),
                    progress,
                    message: Some(format!("Executing {} stage", stage)),
                    timestamp: chrono::Utc::now(),
                    cost: Some(0.10 * progress as f64),
                    duration: Some(progress as u64 * 2),
                };

                service.broadcast_status_update(update).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
        });
    }
}

impl Default for RealtimeStatusService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RealtimeStatusService {
    fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),
            connections: self.connections.clone(),
        }
    }
}

/// WebSocket handler for real-time status updates
pub async fn status_websocket_handler(
    State(state): State<RealtimeStatusApiAppState>,
    ws: WebSocketUpgrade,
    Path(execution_id): Path<ExecutionId>,
) -> impl IntoResponse {
    info!(
        "WebSocket connection requested for execution: {}",
        execution_id
    );

    ws.on_upgrade(move |socket| handle_status_websocket(state, socket, execution_id))
}

/// Handle WebSocket connection for status updates
async fn handle_status_websocket(
    state: RealtimeStatusApiAppState,
    mut socket: WebSocket,
    execution_id: ExecutionId,
) {
    let connection_id = Uuid::new_v4().to_string();
    info!(
        "WebSocket connected: {} for execution: {}",
        connection_id, execution_id
    );

    // Subscribe to status updates
    let mut receiver = state.service.get_status_updates(&execution_id).await;

    // Send welcome message
    let welcome_msg = format!(
        "Connected to execution: {}\nWaiting for status updates...\n",
        execution_id
    );
    let _ = socket.send(Message::Text(welcome_msg.into())).await;

    // Start simulation if no updates are coming
    let simulation_service = state.service.clone();
    let sim_execution_id = execution_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        simulation_service
            .simulate_status_updates(sim_execution_id)
            .await;
    });

    // Handle messages and status updates
    loop {
        tokio::select! {
            // Receive WebSocket messages
            Some(msg) = socket.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        info!("Received message from client {}: {}", connection_id, text);
                        let echo_msg = format!("Echo: {}", text);
                        let _ = socket.send(Message::Text(echo_msg.into())).await;
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket closed by client: {}", connection_id);
                        return;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        return;
                    }
                }
            }
            // Receive status updates
            result = receiver.recv() => {
                match result {
                    Ok(update) => {
                        let json = serde_json::to_string(&update).unwrap_or_else(|e| {
                            error!("Failed to serialize status update: {}", e);
                            "{}".to_string()
                        });
                        let _ = socket.send(Message::Text(json.into())).await;
                    }
                    Err(e) => {
                        match e {
                            broadcast::error::RecvError::Closed => {
                                info!("Status channel closed for execution: {}", execution_id);
                                return;
                            }
                            broadcast::error::RecvError::Lagged(_) => {
                                warn!("Status receiver lagged for execution: {}", execution_id);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Realtime Status API routes
pub fn realtime_status_api_routes() -> Router<RealtimeStatusApiAppState> {
    Router::new().route("/executions/{id}/ws", get(status_websocket_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_pipelines_domain::pipeline_execution::entities::execution::ExecutionStatus;

    #[tokio::test]
    async fn test_realtime_status_service_new() {
        let service = RealtimeStatusService::new();
        let execution_id = ExecutionId::new();

        let _receiver = service.get_status_updates(&execution_id).await;
        // After getting the stream, the channel should exist
        assert!(service.channels.read().await.contains_key(&execution_id));
    }

    #[tokio::test]
    async fn test_broadcast_status_update() {
        let service = RealtimeStatusService::new();
        let execution_id = ExecutionId::new();

        // Create receiver BEFORE broadcasting
        let mut receiver = service.get_status_updates(&execution_id).await;

        let update = ExecutionStatusUpdate {
            execution_id: execution_id.clone(),
            status: ExecutionStatus::RUNNING,
            current_stage: Some("test".to_string()),
            progress: 50,
            message: Some("Running tests".to_string()),
            timestamp: chrono::Utc::now(),
            cost: Some(0.25),
            duration: Some(120),
        };

        service.broadcast_status_update(update.clone()).await;
        let received = tokio::time::timeout(std::time::Duration::from_secs(2), receiver.recv())
            .await
            .expect("Timeout waiting for status update")
            .expect("Failed to receive status update");

        assert_eq!(received.execution_id, execution_id);
        assert_eq!(received.status, ExecutionStatus::RUNNING);
        assert_eq!(received.progress, 50);
    }

    #[tokio::test]
    async fn test_serialize_status_update() {
        let update = ExecutionStatusUpdate {
            execution_id: ExecutionId::new(),
            status: ExecutionStatus::RUNNING,
            current_stage: Some("build".to_string()),
            progress: 75,
            message: Some("Building application".to_string()),
            timestamp: chrono::Utc::now(),
            cost: Some(0.50),
            duration: Some(180),
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("execution_id"));
        assert!(json.contains("status"));
        assert!(json.contains("progress"));

        let deserialized: ExecutionStatusUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, ExecutionStatus::RUNNING);
        assert_eq!(deserialized.progress, 75);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let service = RealtimeStatusService::new();
        let execution_id = ExecutionId::new();

        let update = ExecutionStatusUpdate {
            execution_id: execution_id.clone(),
            status: ExecutionStatus::COMPLETED,
            current_stage: Some("complete".to_string()),
            progress: 100,
            message: Some("Execution completed".to_string()),
            timestamp: chrono::Utc::now(),
            cost: Some(1.00),
            duration: Some(300),
        };

        // Subscribe multiple receivers
        let mut receiver1 = service.get_status_updates(&execution_id).await;
        let mut receiver2 = service.get_status_updates(&execution_id).await;
        let mut receiver3 = service.get_status_updates(&execution_id).await;

        // Broadcast update
        service.broadcast_status_update(update.clone()).await;

        // All receivers should get the update
        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();
        let received3 = receiver3.recv().await.unwrap();

        assert_eq!(received1.execution_id, execution_id);
        assert_eq!(received2.execution_id, execution_id);
        assert_eq!(received3.execution_id, execution_id);
    }
}
