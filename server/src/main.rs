//! Hodei Pipelines Server - Production Bootstrap
//!

use axum::{Router, routing::get};
use tracing::{info, instrument};

mod bootstrap;

use crate::bootstrap::{initialize_server, log_config_summary};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Hodei Pipelines Server");
    info!("========================================");

    // Initialize all server components
    let server_components = initialize_server().await.map_err(|e| {
        tracing::error!("❌ Failed to initialize server: {}", e);
        e
    })?;

    // Log configuration summary
    log_config_summary(&server_components.config);

    info!("========================================");
    info!("🌐 Setting up HTTP routes...");

    // Set up HTTP server routes
    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/server/status", get(server_status));

    let port = server_components.config.server.port;
    let host = server_components.config.server.host.clone();

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("✅ Server listening on http://{}:{}", host, port);
    info!("📚 API Documentation: http://{}:{}/api/docs", host, port);
    info!("💡 Health Check: http://{}:{}/api/health", host, port);
    info!(
        "🔍 Server Status: http://{}:{}/api/server/status",
        host, port
    );

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
#[instrument]
async fn health_check() -> String {
    info!("🔍 Health check requested");
    "ok".to_string()
}

/// Server status endpoint with detailed information
#[instrument]
async fn server_status() -> axum::Json<serde_json::Value> {
    info!("🔍 Server status requested");

    let status = serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "environment": "production",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    axum::Json(status)
}
