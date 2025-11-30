//! Minimal Hodei Server - Core functionality only
//!
//! This is a minimal server implementation focusing on core job orchestration

use axum::{Router, http::StatusCode, response::Json, routing::get};
use serde_json::{Value, json};
use std::sync::Arc;

use hodei_pipelines_core::{Job, JobId, Worker, WorkerId};
use hodei_pipelines_modules::{OrchestratorModule, SchedulerModule};
use hodei_pipelines_ports::SchedulerPort;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber;

// Health check endpoint
async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "hodei-server",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Hodei Server...");

    // Build application
    let app = Router::new()
        .route("/health", get(health))
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request, _| {
                    tracing::info!(
                        "{} {} {:?}",
                        request.method(),
                        request.uri(),
                        request.version()
                    );
                })
                .on_response(|response, latency, _| {
                    tracing::info!("Response: {} latency: {:?}", response.status(), latency);
                }),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Server listening on http://0.0.0.0:8080");
    info!("Health check: http://0.0.0.0:8080/health");

    axum::serve(listener, app).await?;

    Ok(())
}
