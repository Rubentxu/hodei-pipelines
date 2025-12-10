//! API Layer - HTTP Server
//!
//! Axum-based HTTP API for the job orchestration system

pub mod handlers;
pub mod routes;
pub mod middleware;

// Re-export main server components
pub use routes::create_router;
pub use middleware::cors_layer;
