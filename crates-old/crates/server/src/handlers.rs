//! HTTP Request Handlers
//!
//! This module contains all HTTP request handlers for the Hodei server.

use axum::Json;
use serde_json::json;

/// Health check endpoint
pub async fn health_check() -> String {
    tracing::info!("🔍 Health check requested");
    "ok".to_string()
}

/// Server status endpoint with detailed information
pub async fn server_status() -> Json<serde_json::Value> {
    tracing::info!("🔍 Server status requested");

    let status = json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "environment": "production",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "features": {
            "api_alignment": "EPIC-10 - Contract First",
            "resource_pools": "enabled",
            "observability": "enabled",
            "openapi_docs": "enabled"
        }
    });

    Json(status)
}
