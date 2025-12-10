//! API Layer - HTTP Server
//!
//! Axum-based HTTP API for the job orchestration system

pub mod handlers;
pub mod routes;

// Re-export main server components
pub use routes::create_router;
