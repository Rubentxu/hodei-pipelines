//! Orchestrator binary
//!
//! This binary provides the orchestrator service for managing jobs and pipelines.

use axum::{
    Router,
    extract::{Path, State},
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
    pipelines: HashMap<String, Value>,
    jobs: HashMap<String, Value>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let port = std::env::var("ORCHESTRATOR_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

    let app_state = SharedState::new(RwLock::new(AppState {
        pipelines: HashMap::new(),
        jobs: HashMap::new(),
    }));

    let app = Router::new()
        // Health check
        .route("/health", get(health_handler))
        // Pipeline endpoints
        .route("/api/v1/pipelines", post(create_pipeline))
        .route("/api/v1/pipelines", get(list_pipelines))
        .route("/api/v1/pipelines/:id", get(get_pipeline))
        // Job endpoints
        .route("/api/v1/jobs", post(create_job))
        .route("/api/v1/jobs", get(list_jobs))
        .route("/api/v1/jobs/:id", get(get_job))
        .route("/api/v1/jobs/:id/status", get(get_job_status))
        // Swagger UI
        .route("/swagger-ui", get(swagger_ui))
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
    tracing::info!("🚀 Orchestrator listening on http://localhost:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}

// Health check endpoint
async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "orchestrator",
        "version": "0.1.0"
    }))
}

// Pipeline handlers
async fn create_pipeline(
    State(state): State<SharedState>,
    Json(pipeline): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    let pipeline_id = Uuid::new_v4().to_string();

    let mut pipeline_with_id = pipeline;
    if let Some(obj) = pipeline_with_id.as_object_mut() {
        obj.insert("id".to_string(), json!(pipeline_id));
        obj.insert("created_at".to_string(), json!(chrono::Utc::now()));
        obj.insert("status".to_string(), json!("active"));
    }

    state
        .pipelines
        .insert(pipeline_id.clone(), pipeline_with_id.clone());

    Ok(Json(pipeline_with_id))
}

async fn list_pipelines(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    let pipelines: Vec<Value> = state.pipelines.values().cloned().collect();
    Json(json!(pipelines))
}

async fn get_pipeline(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if let Some(pipeline) = state.pipelines.get(&id) {
        Ok(Json(pipeline.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Job handlers
async fn create_job(
    State(state): State<SharedState>,
    Json(job): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut state = state.write().await;
    let job_id = Uuid::new_v4().to_string();

    let mut job_with_id = job;
    if let Some(obj) = job_with_id.as_object_mut() {
        obj.insert("id".to_string(), json!(job_id));
        obj.insert("created_at".to_string(), json!(chrono::Utc::now()));
        obj.insert("status".to_string(), json!("Pending"));
    }

    state.jobs.insert(job_id.clone(), job_with_id.clone());

    Ok(Json(job_with_id))
}

async fn list_jobs(State(state): State<SharedState>) -> Json<Value> {
    let state = state.read().await;
    let jobs: Vec<Value> = state.jobs.values().cloned().collect();
    Json(json!(jobs))
}

async fn get_job(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if let Some(job) = state.jobs.get(&id) {
        Ok(Json(job.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn get_job_status(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> Result<Json<Value>, StatusCode> {
    let state = state.read().await;
    if let Some(job) = state.jobs.get(&id) {
        let status = job.get("status").unwrap_or(&json!("Unknown")).clone();
        Ok(Json(json!({ "id": id, "status": status })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Swagger UI endpoint
async fn swagger_ui() -> Result<String, StatusCode> {
    Ok(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Orchestrator API - Swagger UI</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.0.0/swagger-ui.css" />
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5.0.0/swagger-ui-bundle.js"></script>
    <script>
        SwaggerUIBundle({
            url: '/openapi.json',
            dom_id: '#swagger-ui',
            presets: [
                SwaggerUIBundle.presets.apis,
                SwaggerUIBundle.presets.standalone
            ]
        });
    </script>
</body>
</html>
"#
    .to_string())
}

// OpenAPI spec endpoint
async fn openapi_spec() -> Json<Value> {
    Json(json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Orchestrator API",
            "version": "1.0.0",
            "description": "API for managing jobs and pipelines"
        },
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": {
                            "description": "Service is healthy",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "status": {"type": "string"},
                                            "service": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/pipelines": {
                "get": {
                    "summary": "List all pipelines",
                    "responses": {
                        "200": {
                            "description": "List of pipelines",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {"type": "object"}
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create a new pipeline",
                    "responses": {
                        "200": {
                            "description": "Pipeline created",
                            "content": {
                                "application/json": {
                                    "schema": {"type": "object"}
                                }
                            }
                        }
                    }
                }
            },
            "/api/v1/jobs": {
                "get": {
                    "summary": "List all jobs",
                    "responses": {
                        "200": {
                            "description": "List of jobs",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {"type": "object"}
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create a new job",
                    "responses": {
                        "200": {
                            "description": "Job created",
                            "content": {
                                "application/json": {
                                    "schema": {"type": "object"}
                                }
                            }
                        }
                    }
                }
            }
        }
    }))
}
