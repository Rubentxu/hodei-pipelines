//! Scheduler binary
//!
//! This binary provides the scheduler service for managing job scheduling.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

type SharedState = Arc<RwLock<AppState>>;

#[derive(Clone)]
struct AppState {
    scheduled_jobs: HashMap<String, Value>,
    workers: HashMap<String, Value>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("SCHEDULER_PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse::<u16>()?;

    let app_state = SharedState::new(RwLock::new(AppState {
        scheduled_jobs: HashMap::new(),
        workers: HashMap::new(),
    }));

    let app = Router::new()
        // Health check
        .route("/health", get(health_handler))
        // Scheduling endpoints
        .route("/api/v1/schedule", post(schedule_job))
        .route("/api/v1/scheduled", get(list_scheduled_jobs))
        .route("/api/v1/scheduled/:id", get(get_scheduled_job))
        // Worker management
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/workers", post(register_worker))
        .route("/api/v1/workers/:id", get(get_worker))
        .route("/api/v1/workers/:id/heartbeat", post(worker_heartbeat))
        // OpenAPI spec
        .route("/openapi.json", get(openapi_spec))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("🚀 Scheduler listening on http://localhost:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}

// Health check endpoint
async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "scheduler",
        "version": "0.1.0"
    }))
}

// Scheduling handlers
async fn schedule_job(
    State(state): State<SharedState>,
    Json(job): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    let schedule_id = Uuid::new_v4().to_string();

    let mut scheduled_job = job;
    if let Some(obj) = scheduled_job.as_object_mut() {
        obj.insert("id".to_string(), json!(schedule_id));
        obj.insert("scheduled_at".to_string(), json!(chrono::Utc::now()));
        obj.insert("status".to_string(), json!("scheduled"));
    }

    state
        .scheduled_jobs
        .insert(schedule_id.clone(), scheduled_job.clone());

    Ok(Json(scheduled_job))
}

async fn list_scheduled_jobs(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    let jobs: Vec<Value> = state.scheduled_jobs.values().cloned().collect();
    Json(json!(jobs))
}

async fn get_scheduled_job(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if let Some(job) = state.scheduled_jobs.get(&id) {
        Ok(Json(job.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Worker management handlers
async fn register_worker(
    State(state): State<SharedState>,
    Json(worker): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    let worker_id = Uuid::new_v4().to_string();

    let mut worker_data = worker;
    if let Some(obj) = worker_data.as_object_mut() {
        obj.insert("id".to_string(), json!(worker_id));
        obj.insert("registered_at".to_string(), json!(chrono::Utc::now()));
        obj.insert("status".to_string(), json!("online"));
    }

    state.workers.insert(worker_id.clone(), worker_data.clone());

    Ok(Json(worker_data))
}

async fn list_workers(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    let workers: Vec<Value> = state.workers.values().cloned().collect();
    Json(json!(workers))
}

async fn get_worker(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if let Some(worker) = state.workers.get(&id) {
        Ok(Json(worker.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn worker_heartbeat(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    if let Some(worker) = state.workers.get_mut(&id) {
        if let Some(obj) = worker.as_object_mut() {
            obj.insert("last_heartbeat".to_string(), json!(chrono::Utc::now()));
            obj.insert("status".to_string(), json!("online"));
        }
        Ok(Json(worker.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// OpenAPI spec endpoint
async fn openapi_spec() -> Json<Value> {
    Json(json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Scheduler API",
            "version": "1.0.0",
            "description": "API for job scheduling and worker management"
        },
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {
                            "description": "Service is healthy"
                        }
                    }
                }
            },
            "/api/v1/schedule": {
                "post": {
                    "summary": "Schedule a job",
                    "responses": {
                        "200": {"description": "Job scheduled"}
                    }
                }
            },
            "/api/v1/workers": {
                "get": {
                    "summary": "List all workers",
                    "responses": {
                        "200": {"description": "List of workers"}
                    }
                },
                "post": {
                    "summary": "Register a worker",
                    "responses": {
                        "200": {"description": "Worker registered"}
                    }
                }
            }
        }
    }))
}
