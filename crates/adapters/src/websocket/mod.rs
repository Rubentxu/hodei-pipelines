//! WebSocket Adapter
//!
//! Handles real-time communication with clients via WebSockets.
//! Subscribes to the EventBus and broadcasts relevant events to connected clients.

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};

use hodei_pipelines_ports::{EventSubscriber, SystemEvent};
use std::sync::Arc;
use tracing::{error, info, warn};

/// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(event_subscriber): State<Arc<dyn EventSubscriber>>,
) -> impl IntoResponse {
    info!("WebSocket upgrade request received");
    ws.on_upgrade(move |socket| handle_socket(socket, event_subscriber))
}

async fn handle_socket(mut socket: WebSocket, event_subscriber: Arc<dyn EventSubscriber>) {
    info!("WebSocket client connected");

    // Subscribe to system events
    let mut rx = match event_subscriber.subscribe().await {
        Ok(rx) => rx,
        Err(e) => {
            error!("Failed to subscribe to event bus: {}", e);
            let _ = socket
                .send(Message::Text(format!("Error: {}", e).into()))
                .await;
            return;
        }
    };

    loop {
        tokio::select! {
            // Receive event from EventBus
            event_result = rx.recv() => {
                match event_result {
                    Ok(event) => {
                        // Filter events relevant for the frontend
                        if let Some(json_msg) = serialize_event_for_client(&event)
                            && let Err(e) = socket.send(Message::Text(json_msg.into())).await
                        {
                            warn!("Failed to send message to WebSocket client: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error receiving event from bus: {}", e);
                        break;
                    }
                }
            }

            // Receive message from WebSocket client (e.g., ping/pong or commands)
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Close(_) => {
                                info!("WebSocket client disconnected");
                                break;
                            }
                            Message::Ping(_) => {
                                // Axum handles pong automatically, but we can log if needed
                            }
                            _ => {
                                // Ignore other messages for now
                            }
                        }
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket client error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket client stream ended");
                        break;
                    }
                }
            }
        }
    }
}

/// Serializes system events into JSON for the client.
/// Returns None if the event should not be sent to the client.
fn serialize_event_for_client(event: &SystemEvent) -> Option<String> {
    match event {
        SystemEvent::JobCreated(_)
        | SystemEvent::JobScheduled { .. }
        | SystemEvent::JobStarted { .. }
        | SystemEvent::JobCompleted { .. }
        | SystemEvent::JobFailed { .. }
        | SystemEvent::WorkerHeartbeat { .. }
        | SystemEvent::PipelineCreated(_)
        | SystemEvent::PipelineStarted { .. }
        | SystemEvent::PipelineCompleted { .. }
        | SystemEvent::PipelineExecutionStarted { .. }
        | SystemEvent::LogChunkReceived(_) => serde_json::to_string(event).ok(),
        // Filter out internal events if any (e.g., WorkerHeartbeat might be too noisy, but useful for now)
        _ => serde_json::to_string(event).ok(),
    }
}
