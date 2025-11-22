//! Worker Lifecycle Manager binary
//!
//! This binary provides the worker lifecycle manager service.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
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
    workers: HashMap<String, Value>,
    executions: HashMap<String, Value>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("WORKER_MANAGER_PORT")
        .unwrap_or_else(|_| "8082".to_string())
        .parse::<u16>()?;

    let app_state = SharedState::new(RwLock::new(AppState {
        workers: HashMap::new(),
        executions: HashMap::new(),
    }));

    let app = Router::new()
        // Health check
        .route("/health", get(health_handler))
        // Worker lifecycle
        .route("/api/v1/workers", post(start_worker))
        .route("/api/v1/workers", get(list_workers))
        .route("/api/v1/workers/:id", get(get_worker))
        .route("/api/v1/workers/:id", delete(stop_worker))
        .route("/api/v1/workers/:id/status", get(get_worker_status))
        // Job execution
        .route("/api/v1/execute", post(execute_job))
        .route("/api/v1/executions", get(list_executions))
        .route("/api/v1/executions/:id", get(get_execution))
        .route("/api/v1/executions/:id/logs", get(get_execution_logs))
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
    tracing::info!("🚀 Worker Manager listening on http://localhost:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}

// Health check endpoint
async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "worker-manager",
        "version": "0.1.0"
    }))
}

// Worker lifecycle handlers
async fn start_worker(
    State(state): State<SharedState>,
    Json(worker_config): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    let worker_id = Uuid::new_v4().to_string();

    let mut worker_data = worker_config;
    if let Some(obj) = worker_data.as_object_mut() {
        obj.insert("id".to_string(), json!(worker_id));
        obj.insert("started_at".to_string(), json!(chrono::Utc::now()));
        obj.insert("status".to_string(), json!("running"));
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

async fn stop_worker(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    if let Some(worker) = state.workers.get_mut(&id) {
        if let Some(obj) = worker.as_object_mut() {
            obj.insert("stopped_at".to_string(), json!(chrono::Utc::now()));
            obj.insert("status".to_string(), json!("stopped"));
        }
        Ok(Json(worker.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_worker_status(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if let Some(worker) = state.workers.get(&id) {
        let status = worker.get("status").unwrap_or(&json!("unknown")).clone();
        Ok(Json(json!({ "id": id, "status": status })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Job execution handlers
async fn execute_job(
    State(state): State<SharedState>,
    Json(job): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    let execution_id = Uuid::new_v4().to_string();

    let mut execution = job;
    if let Some(obj) = execution.as_object_mut() {
        obj.insert("id".to_string(), json!(execution_id));
        obj.insert("started_at".to_string(), json!(chrono::Utc::now()));
        obj.insert("status".to_string(), json!("running"));
    }

    state
        .executions
        .insert(execution_id.clone(), execution.clone());

    // Simulate job execution
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    });

    Ok(Json(execution))
}

async fn list_executions(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    let executions: Vec<Value> = state.executions.values().cloned().collect();
    Json(json!(executions))
}

async fn get_execution(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if let Some(execution) = state.executions.get(&id) {
        Ok(Json(execution.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_execution_logs(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if state.executions.get(&id).is_some() {
        Ok(Json(json!({
            "execution_id": id,
            "logs": [
                {"timestamp": chrono::Utc::now(), "level": "info", "message": "Job started"},
                {"timestamp": chrono::Utc::now(), "level": "info", "message": "Job completed"}
            ]
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// OpenAPI spec endpoint
async fn openapi_spec() -> Json<Value> {
    Json(json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Worker Manager API",
            "version": "1.0.0",
            "description": "API for worker lifecycle and job execution management"
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
            "/api/v1/workers": {
                "get": {
                    "summary": "List all workers",
                    "responses": {
                        "200": {"description": "List of workers"}
                    }
                },
                "post": {
                    "summary": "Start a new worker",
                    "responses": {
                        "200": {"description": "Worker started"}
                    }
                }
            },
            "/api/v1/execute": {
                "post": {
                    "summary": "Execute a job",
                    "responses": {
                        "200": {"description": "Job execution started"}
                    }
                }
            }
        }
    }))
}
