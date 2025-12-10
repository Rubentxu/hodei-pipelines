//! API Middleware
//!
//! Error handling, logging, CORS, and other middleware

use axum::{
    extract::Request,
    http::{HeaderValue, Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, instrument};

/// CORS middleware configuration
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
}

/// Trace middleware for request logging
#[instrument(name = "api_request", skip(req, next), fields(method, uri, status_code))]
pub async fn trace_requests(req: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status_code = response.status();

    info!(
        method = %method,
        uri = %uri,
        status_code = %status_code.as_u16(),
        duration = ?duration,
        "Request processed"
    );

    response
}

/// Error handling middleware
#[instrument(name = "error_handler", skip(req, next), fields(method, uri))]
pub async fn error_handler(req: Request, next: Next) -> Result<Response, StatusCode> {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    match next.run(req).await {
        response if response.status().is_server_error() => {
            tracing::error!(
                method = %method,
                uri = %uri,
                status_code = %response.status().as_u16(),
                duration = ?start.elapsed(),
                "Server error occurred"
            );
            Ok(response)
        }
        response if response.status().is_client_error() => {
            tracing::warn!(
                method = %method,
                uri = %uri,
                status_code = %response.status().as_u16(),
                duration = ?start.elapsed(),
                "Client error occurred"
            );
            Ok(response)
        }
        response => {
            tracing::info!(
                method = %method,
                uri = %uri,
                status_code = %response.status().as_u16(),
                duration = ?start.elapsed(),
                "Request completed"
            );
            Ok(response)
        }
    }
}

/// Request ID middleware
#[instrument(name = "request_id", skip(req, next))]
pub async fn add_request_id(req: Request, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();

    let mut response = next.run(req).await;

    response.headers_mut().insert(
        "X-Request-ID",
        HeaderValue::from_str(&request_id).unwrap(),
    );

    response
}

/// Rate limiting headers middleware
#[instrument(name = "rate_limit_headers", skip(req, next))]
pub async fn rate_limit_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;

    // Add rate limit headers (placeholder values)
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        HeaderValue::from_static("100"),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_static("99"),
    );
    response.headers_mut().insert(
        "X-RateLimit-Reset",
        HeaderValue::from_static("60"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};

    #[tokio::test]
    async fn test_cors_layer_creation() {
        let cors = cors_layer();
        assert!(cors.is_ok());
    }

    #[tokio::test]
    async fn test_trace_requests() {
        let app = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(trace_requests));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_request_id() {
        let app = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(add_request_id));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(response.headers().contains_key("X-Request-ID"));
        assert!(response.headers().get("X-Request-ID").is_some());
    }

    #[tokio::test]
    async fn test_rate_limit_headers() {
        let app = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(rate_limit_headers));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(response.headers().contains_key("X-RateLimit-Limit"));
        assert!(response.headers().contains_key("X-RateLimit-Remaining"));
        assert!(response.headers().contains_key("X-RateLimit-Reset"));
    }
}
