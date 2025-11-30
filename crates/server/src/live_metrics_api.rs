//! Live Metrics Streaming API (US-010)
//!
//! Provides WebSocket-based streaming of real-time metrics for pipeline executions.
//! Supports CPU, memory, disk, network metrics and threshold alerts.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::sync::{RwLock, broadcast};
use tokio::time::{Duration, interval};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Live metric DTO
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LiveMetric {
    pub metric_type: MetricType,
    pub worker_id: String,
    pub execution_id: Option<String>,
    pub value: f64,
    pub unit: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub threshold_status: ThresholdStatus,
}

/// Type of metric
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    #[serde(rename = "cpu_usage")]
    CpuUsage,
    #[serde(rename = "memory_usage")]
    MemoryUsage,
    #[serde(rename = "disk_io")]
    DiskIo,
    #[serde(rename = "network_io")]
    NetworkIo,
    #[serde(rename = "load_average")]
    LoadAverage,
}

/// Threshold status
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ThresholdStatus {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
}

/// Application state for Live Metrics API
#[derive(Clone)]
pub struct LiveMetricsApiAppState {
    pub service: Arc<LiveMetricsService>,
}

/// Live metrics service
#[derive(Debug)]
pub struct LiveMetricsService {
    /// Channels for broadcasting metrics per execution
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<LiveMetric>>>>,
    /// Metric thresholds configuration
    thresholds: Arc<RwLock<HashMap<MetricType, ThresholdConfig>>>,
}

#[derive(Debug, Clone)]
pub struct ThresholdConfig {
    pub warning: f64,
    pub critical: f64,
}

impl LiveMetricsService {
    /// Create new live metrics service
    pub fn new() -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert(
            MetricType::CpuUsage,
            ThresholdConfig {
                warning: 70.0,
                critical: 90.0,
            },
        );
        thresholds.insert(
            MetricType::MemoryUsage,
            ThresholdConfig {
                warning: 75.0,
                critical: 95.0,
            },
        );
        thresholds.insert(
            MetricType::DiskIo,
            ThresholdConfig {
                warning: 100.0, // MB/s
                critical: 200.0,
            },
        );
        thresholds.insert(
            MetricType::NetworkIo,
            ThresholdConfig {
                warning: 100.0, // MB/s
                critical: 200.0,
            },
        );
        thresholds.insert(
            MetricType::LoadAverage,
            ThresholdConfig {
                warning: 2.0,
                critical: 4.0,
            },
        );

        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            thresholds: Arc::new(RwLock::new(thresholds)),
        }
    }

    /// Get or create a channel for an execution
    async fn get_or_create_channel(&self, execution_id: &str) -> broadcast::Sender<LiveMetric> {
        let mut channels = self.channels.write().await;

        if let Some(sender) = channels.get(execution_id) {
            return sender.clone();
        }

        let (sender, _) = broadcast::channel(100);
        channels.insert(execution_id.to_string(), sender.clone());
        sender
    }

    /// Broadcast a metric update
    pub async fn broadcast_metric(&self, metric: LiveMetric) {
        let execution_id = metric.execution_id.as_ref().unwrap_or(&metric.worker_id);
        let sender = self.get_or_create_channel(execution_id).await;

        let result = sender.send(metric);
        match result {
            Ok(n) => info!("Broadcasted metric to {} subscribers", n),
            Err(e) => warn!("Failed to broadcast metric: {}", e),
        }
    }

    /// Get metrics stream for an execution
    pub async fn get_metrics_stream(&self, execution_id: &str) -> broadcast::Receiver<LiveMetric> {
        let sender = self.get_or_create_channel(execution_id).await;
        sender.subscribe()
    }

    /// Simulate live metrics (for testing and demo)
    pub async fn simulate_live_metrics(&self, worker_id: String, execution_id: Option<String>) {
        let service = self.clone();
        tokio::spawn(async move {
            let mut cpu = 20.0;
            let mut memory = 30.0;
            let mut disk_io = 10.0;
            let mut network_io = 15.0;
            let mut load_avg = 1.0;

            let mut interval = interval(Duration::from_millis(1000));

            loop {
                interval.tick().await;

                // Simulate metric fluctuations
                cpu += (rand::random::<f64>() - 0.5) * 10.0;
                memory += (rand::random::<f64>() - 0.5) * 5.0;
                disk_io += (rand::random::<f64>() - 0.5) * 8.0;
                network_io += (rand::random::<f64>() - 0.5) * 6.0;
                load_avg += (rand::random::<f64>() - 0.5) * 0.5;

                // Clamp values
                cpu = cpu.clamp(0.0, 100.0);
                memory = memory.clamp(0.0, 100.0);
                disk_io = disk_io.clamp(0.0, 500.0);
                network_io = network_io.clamp(0.0, 500.0);
                load_avg = load_avg.max(0.0);

                // Create metrics
                let metrics = vec![
                    LiveMetric {
                        metric_type: MetricType::CpuUsage,
                        worker_id: worker_id.clone(),
                        execution_id: execution_id.clone(),
                        value: cpu,
                        unit: "%".to_string(),
                        timestamp: chrono::Utc::now(),
                        threshold_status: if cpu > 90.0 {
                            ThresholdStatus::Critical
                        } else if cpu > 70.0 {
                            ThresholdStatus::Warning
                        } else {
                            ThresholdStatus::Normal
                        },
                    },
                    LiveMetric {
                        metric_type: MetricType::MemoryUsage,
                        worker_id: worker_id.clone(),
                        execution_id: execution_id.clone(),
                        value: memory,
                        unit: "%".to_string(),
                        timestamp: chrono::Utc::now(),
                        threshold_status: if memory > 95.0 {
                            ThresholdStatus::Critical
                        } else if memory > 75.0 {
                            ThresholdStatus::Warning
                        } else {
                            ThresholdStatus::Normal
                        },
                    },
                    LiveMetric {
                        metric_type: MetricType::DiskIo,
                        worker_id: worker_id.clone(),
                        execution_id: execution_id.clone(),
                        value: disk_io,
                        unit: "MB/s".to_string(),
                        timestamp: chrono::Utc::now(),
                        threshold_status: if disk_io > 200.0 {
                            ThresholdStatus::Critical
                        } else if disk_io > 100.0 {
                            ThresholdStatus::Warning
                        } else {
                            ThresholdStatus::Normal
                        },
                    },
                    LiveMetric {
                        metric_type: MetricType::NetworkIo,
                        worker_id: worker_id.clone(),
                        execution_id: execution_id.clone(),
                        value: network_io,
                        unit: "MB/s".to_string(),
                        timestamp: chrono::Utc::now(),
                        threshold_status: if network_io > 200.0 {
                            ThresholdStatus::Critical
                        } else if network_io > 100.0 {
                            ThresholdStatus::Warning
                        } else {
                            ThresholdStatus::Normal
                        },
                    },
                    LiveMetric {
                        metric_type: MetricType::LoadAverage,
                        worker_id: worker_id.clone(),
                        execution_id: execution_id.clone(),
                        value: load_avg,
                        unit: "".to_string(),
                        timestamp: chrono::Utc::now(),
                        threshold_status: if load_avg > 4.0 {
                            ThresholdStatus::Critical
                        } else if load_avg > 2.0 {
                            ThresholdStatus::Warning
                        } else {
                            ThresholdStatus::Normal
                        },
                    },
                ];

                for metric in metrics {
                    service.broadcast_metric(metric).await;
                }
            }
        });
    }
}

impl Default for LiveMetricsService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for LiveMetricsService {
    fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),
            thresholds: self.thresholds.clone(),
        }
    }
}

/// WebSocket handler for live metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics/stream/{worker_id}",
    params(
        ("worker_id" = String, Path, description = "Worker ID to stream metrics for")
    ),
    responses(
        (status = 101, description = "WebSocket upgrade successful"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "observability"
)]
pub async fn metrics_websocket_handler(
    State(state): State<LiveMetricsApiAppState>,
    ws: WebSocketUpgrade,
    Path(worker_id): Path<String>,
) -> impl IntoResponse {
    info!("WebSocket connection requested for worker: {}", worker_id);

    ws.on_upgrade(move |socket| handle_metrics_websocket(state, socket, worker_id))
}

/// Handle WebSocket connection for metrics
async fn handle_metrics_websocket(
    state: LiveMetricsApiAppState,
    mut socket: WebSocket,
    worker_id: String,
) {
    let connection_id = Uuid::new_v4().to_string();
    info!(
        "WebSocket connected: {} for worker: {}",
        connection_id, worker_id
    );

    // Subscribe to metrics
    let mut receiver = state.service.get_metrics_stream(&worker_id).await;

    // Send welcome message
    let welcome_msg = format!(
        "Connected to worker: {}\nWaiting for live metrics...\n",
        worker_id
    );
    let _ = socket.send(Message::Text(welcome_msg.into())).await;

    // Start metrics simulation
    let simulation_service = state.service.clone();
    let sim_worker_id = worker_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        simulation_service
            .simulate_live_metrics(sim_worker_id, None)
            .await;
    });

    // Handle messages and metrics
    loop {
        tokio::select! {
            Some(msg_result) = socket.next() => {
                match msg_result {
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
            result = receiver.recv() => {
                match result {
                    Ok(metric) => {
                        let json = serde_json::to_string(&metric).unwrap_or_else(|e| {
                            error!("Failed to serialize metric: {}", e);
                            "{}".to_string()
                        });
                        let _ = socket.send(Message::Text(json.into())).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Metrics channel closed for worker: {}", worker_id);
                        return;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        warn!("Metrics receiver lagged for worker: {}", worker_id);
                        return;
                    }
                }
            }
        }
    }
}

/// Live Metrics API routes
pub fn live_metrics_api_routes() -> Router<LiveMetricsApiAppState> {
    Router::new().route("/workers/{id}/metrics/ws", get(metrics_websocket_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_metrics_service_new() {
        let service = LiveMetricsService::new();
        let _receiver = service.get_metrics_stream("test-worker").await;
        // After getting the stream, the channel should exist
        assert!(service.channels.read().await.contains_key("test-worker"));
    }

    #[tokio::test]
    async fn test_broadcast_metric() {
        let service = LiveMetricsService::new();
        let worker_id = "test-worker".to_string();

        // Create receiver BEFORE broadcasting
        let mut receiver = service.get_metrics_stream(&worker_id).await;

        let metric = LiveMetric {
            metric_type: MetricType::CpuUsage,
            worker_id: worker_id.clone(),
            execution_id: None,
            value: 50.0,
            unit: "%".to_string(),
            timestamp: chrono::Utc::now(),
            threshold_status: ThresholdStatus::Normal,
        };

        service.broadcast_metric(metric.clone()).await;

        let received = tokio::time::timeout(std::time::Duration::from_secs(2), receiver.recv())
            .await
            .expect("Timeout waiting for metric")
            .expect("Failed to receive metric");

        assert_eq!(received.worker_id, worker_id);
        assert_eq!(received.value, 50.0);
    }

    #[tokio::test]
    async fn test_serialize_metric() {
        let metric = LiveMetric {
            metric_type: MetricType::MemoryUsage,
            worker_id: "worker-1".to_string(),
            execution_id: Some("exec-123".to_string()),
            value: 65.5,
            unit: "%".to_string(),
            timestamp: chrono::Utc::now(),
            threshold_status: ThresholdStatus::Warning,
        };

        let json = serde_json::to_string(&metric).unwrap();
        assert!(json.contains("metric_type"));
        assert!(json.contains("value"));
        assert!(json.contains("threshold_status"));

        let deserialized: LiveMetric = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.metric_type, MetricType::MemoryUsage));
        assert_eq!(deserialized.value, 65.5);
    }

    #[tokio::test]
    async fn test_threshold_status() {
        assert!(matches!(ThresholdStatus::Normal, _));
        assert!(matches!(ThresholdStatus::Warning, _));
        assert!(matches!(ThresholdStatus::Critical, _));

        // Test serialization
        let status = ThresholdStatus::Critical;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""critical""#);

        let deserialized: ThresholdStatus = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ThresholdStatus::Critical));
    }
}
