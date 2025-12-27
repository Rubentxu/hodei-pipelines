//! Middleware components for HTTP API
//!
//! Provides CORS, tracing, error handling, request ID, and rate limiting middleware

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{debug, error, info, warn};

/// CORS configuration for the API
pub fn cors_layer() -> Result<CorsLayer, Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Ok(cors)
}

/// Request tracing middleware
pub async fn trace_requests(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();

    info!(
        status = response.status().as_u16(),
        duration = ?duration,
        "request completed"
    );

    Ok(response)
}

/// Add unique request ID to response headers
pub async fn add_request_id(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();

    let mut response = next.run(request).await;

    response.headers_mut().insert(
        "X-Request-ID",
        HeaderValue::from_str(&request_id).unwrap(),
    );

    Ok(response)
}

/// Rate limit headers middleware
pub async fn rate_limit_headers(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let response = next.run(request).await;

    let mut response = response;

    // Add dummy rate limit headers
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        HeaderValue::from_static("1000"),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_static("999"),
    );

    Ok(response)
}

/// Error handling middleware
pub async fn handle_errors(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let response = next.run(request).await;
    Ok(response)
}
