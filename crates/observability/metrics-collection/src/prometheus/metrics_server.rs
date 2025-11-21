//! Prometheus Metrics Server
//!
//! HTTP server that exposes /metrics endpoint for Prometheus scraping.

use super::MetricsCollector;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Server bind failed: {0}")]
    BindFailed(String),

    #[error("Server start failed: {0}")]
    StartFailed(String),

    #[error("Metrics gathering failed: {0}")]
    MetricsGatheringFailed(String),
}

/// Metrics server state
#[derive(Clone)]
struct AppState {
    collector: Arc<MetricsCollector>,
}

/// Prometheus metrics server
pub struct MetricsServer {
    host: String,
    port: u16,
    collector: Arc<MetricsCollector>,
}

impl MetricsServer {
    /// Create a new metrics server
    pub fn new(host: &str, port: u16) -> Result<Self, ServerError> {
        Ok(Self {
            host: host.to_string(),
            port,
            collector: Arc::new(MetricsCollector::new()),
        })
    }

    /// Create a new metrics server with custom collector
    pub fn with_collector(
        host: &str,
        port: u16,
        collector: MetricsCollector,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            host: host.to_string(),
            port,
            collector: Arc::new(collector),
        })
    }

    /// Get the metrics collector
    pub fn get_collector(&self) -> Arc<MetricsCollector> {
        Arc::clone(&self.collector)
    }

    /// Start the metrics server
    pub async fn start(self) -> Result<(), ServerError> {
        let app_state = AppState {
            collector: Arc::clone(&self.collector),
        };

        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/health", get(health_handler))
            .with_state(app_state);

        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ServerError::BindFailed(format!("Failed to bind to {}: {}", addr, e)))?;

        tracing::info!("Prometheus metrics server listening on {}", addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| ServerError::StartFailed(e.to_string()))?;

        Ok(())
    }
}

/// Handler for /metrics endpoint
async fn metrics_handler(State(state): State<AppState>) -> Response {
    match state.collector.gather() {
        Ok(metrics) => (StatusCode::OK, metrics).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to gather metrics: {}", e),
        )
            .into_response(),
    }
}

/// Handler for /health endpoint
async fn health_handler() -> Response {
    (StatusCode::OK, "OK").into_response()
}
