//! Hodei Pipelines Server - Minimal Implementation
//!

use axum::{Router, routing::get};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Hodei Pipelines Server");
    info!("📚 API Documentation: http://localhost:8080/api/docs");
    info!("🔗 OpenAPI Spec: http://localhost:8080/api/openapi.json");

    let port = std::env::var("HODEI_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

    // TODO: Implement with PostgreSQL repositories
    info!("⚠️  Server initialized - repositories not yet configured");

    // Set up HTTP server routes
    info!("🌐 Setting up HTTP routes");

    let app = Router::new().route("/api/health", get(health_check)).route(
        "/api/docs",
        get(|| async { "API Documentation - Under Development" }),
    );

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("✅ Server listening on http://0.0.0.0:{}", port);
    info!("📖 API Documentation: http://0.0.0.0:{}/api/docs", port);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> String {
    "ok".to_string()
}
