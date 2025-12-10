//! HTTP Handlers
//!
//! Request handlers for the API endpoints

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateJobRequest {
    pub name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
}

#[derive(Serialize)]
pub struct CreateJobResponse {
    pub job_id: String,
    pub status: String,
}

pub async fn create_job_handler(
    State(state): State<()>,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<CreateJobResponse>, StatusCode> {
    // TODO: Implement job creation
    Ok(Json(CreateJobResponse {
        job_id: "todo".to_string(),
        status: "pending".to_string(),
    }))
}
