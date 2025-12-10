//! HTTP Routes
//!
//! Defines the API routes for the application

use axum::routing::{post, get, delete, put};
use axum::Router;
use super::handlers::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Job routes
        .route("/api/v1/jobs", post(super::handlers::create_job_handler))
        .route("/api/v1/jobs", get(super::handlers::list_jobs_handler))
        .route("/api/v1/jobs/:job_id", get(super::handlers::get_job_handler))
        .route("/api/v1/jobs/:job_id/execute", post(super::handlers::execute_job_handler))
        .route("/api/v1/jobs/:job_id/cancel", delete(super::handlers::cancel_job_handler))

        // Provider routes
        .route("/api/v1/providers", post(super::handlers::register_provider_handler))
        .route("/api/v1/providers", get(super::handlers::list_providers_handler))
        .route("/api/v1/providers/:provider_id", get(super::handlers::get_provider_handler))
        .route("/api/v1/providers/:provider_id/activate", post(super::handlers::activate_provider_handler))
        .route("/api/v1/providers/:provider_id/deactivate", post(super::handlers::deactivate_provider_handler))
        .route("/api/v1/providers/:provider_id", delete(super::handlers::delete_provider_handler))

        // Health check
        .route("/health", get(super::handlers::health_check_handler))
}
