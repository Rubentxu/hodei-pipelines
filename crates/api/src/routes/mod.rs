//! HTTP Routes
//!
//! Defines the API routes for the application

use axum::routing::{post, get};
use axum::Router;

pub fn create_router() -> Router<()> {
    Router::new()
        .route("/jobs", post(super::handlers::create_job_handler))
        .route("/health", get(|| async { "ok" }))
}
