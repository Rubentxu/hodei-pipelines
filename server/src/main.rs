//! Hodei Jobs Server - Monolithic Modular Architecture

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use std::sync::Arc;

use hodei_adapters::{
    GrpcWorkerClient, HttpWorkerClient, InMemoryBus, InMemoryJobRepository,
    InMemoryPipelineRepository, InMemoryWorkerRepository,
};
use hodei_core::{JobSpec, Worker};
use hodei_modules::{OrchestratorModule, SchedulerModule};
use hodei_shared_types::WorkerCapabilities;
use serde_json::{Value, json};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

mod metrics;
use metrics::MetricsRegistry;

mod auth;
mod grpc;

#[derive(Clone)]
struct AppState {
    scheduler: Arc<
        SchedulerModule<
            InMemoryJobRepository,
            InMemoryBus,
            GrpcWorkerClient,
            InMemoryWorkerRepository,
        >,
    >,
    orchestrator:
        Arc<OrchestratorModule<InMemoryJobRepository, InMemoryBus, InMemoryPipelineRepository>>,
    metrics: MetricsRegistry,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Hodei Jobs Server");

    let port = std::env::var("HODEI_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

    // Initialize DI container
    let job_repo = Arc::new(InMemoryJobRepository::new());
    let worker_repo = Arc::new(InMemoryWorkerRepository::new());
    let pipeline_repo = Arc::new(InMemoryPipelineRepository::new());
    let event_bus = Arc::new(InMemoryBus::new(10_000));
    let worker_client = Arc::new(GrpcWorkerClient::new(
        tonic::transport::Channel::from_static("http://localhost:8080")
            .connect()
            .await?,
        std::time::Duration::from_secs(30),
    ));
    let metrics = MetricsRegistry::new().expect("Failed to initialize metrics registry");

    // Create modules
    let scheduler = Arc::new(SchedulerModule::new(
        job_repo.clone(),
        event_bus.clone(),
        worker_client.clone(),
        worker_repo.clone(),
        hodei_modules::SchedulerConfig {
            max_queue_size: 10000,
            scheduling_interval_ms: 100,
            worker_heartbeat_timeout_ms: 30000,
        },
    ));

    let orchestrator = Arc::new(OrchestratorModule::new(
        job_repo,
        event_bus,
        pipeline_repo,
        hodei_modules::OrchestratorConfig {
            max_concurrent_jobs: 1000,
            default_timeout_ms: 300000,
        },
    ));

    scheduler.clone().start().await?;

    let app_state = AppState {
        scheduler: scheduler.clone(),
        orchestrator: orchestrator.clone(),
        metrics: metrics.clone(),
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/jobs", post(create_job))
        .route("/api/v1/jobs/{id}", get(get_job))
        .route("/api/v1/jobs/{id}/cancel", post(cancel_job))
        .route("/api/v1/workers", post(register_worker))
        .route("/api/v1/workers/{id}/heartbeat", post(worker_heartbeat))
        .route("/api/v1/metrics", get(get_metrics))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("📡 Server listening on http://localhost:{}", port);
    info!("🏗️  Architecture: Monolithic Modular (Hexagonal)");

    // Start gRPC server
    let grpc_port = std::env::var("HODEI_GRPC_PORT")
        .unwrap_or_else(|_| "50051".to_string())
        .parse::<u16>()?;
    let grpc_addr = format!("0.0.0.0:{}", grpc_port).parse()?;

    let hwp_service = grpc::HwpService::new(scheduler.clone());

    // JWT Config
    let jwt_secret = std::env::var("HODEI_JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let jwt_config = hodei_adapters::security::JwtConfig {
        secret: jwt_secret,
        expiration_seconds: 3600,
    };
    let token_service = Arc::new(hodei_adapters::security::JwtTokenService::new(jwt_config));
    let auth_interceptor = auth::AuthInterceptor::new(token_service);

    // mTLS Config
    let cert_path = std::env::var("HODEI_TLS_CERT_PATH").unwrap_or_default();
    let key_path = std::env::var("HODEI_TLS_KEY_PATH").unwrap_or_default();
    let ca_path = std::env::var("HODEI_TLS_CA_PATH").unwrap_or_default();

    let mut builder = tonic::transport::Server::builder();

    if !cert_path.is_empty() && !key_path.is_empty() && !ca_path.is_empty() {
        info!("Configuring mTLS for gRPC server");
        let cert = std::fs::read_to_string(cert_path)?;
        let key = std::fs::read_to_string(key_path)?;
        let ca = std::fs::read_to_string(ca_path)?;

        let identity = tonic::transport::Identity::from_pem(cert, key);
        let client_ca = tonic::transport::Certificate::from_pem(ca);

        builder = builder.tls_config(
            tonic::transport::ServerTlsConfig::new()
                .identity(identity)
                .client_ca_root(client_ca),
        )?;
    }

    info!("📡 gRPC Server listening on {}", grpc_addr);

    tokio::spawn(async move {
        if let Err(e) = builder
            .add_service(hwp_proto::WorkerServiceServer::with_interceptor(
                hwp_service,
                auth_interceptor,
            ))
            .serve(grpc_addr)
            .await
        {
            tracing::error!("gRPC Server failed: {}", e);
        }
    });

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "service": "hodei-server",
        "version": env!("CARGO_PKG_VERSION"),
        "architecture": "monolithic_modular",
    }))
}

async fn create_job(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let job_spec: JobSpec = match serde_json::from_value(payload) {
        Ok(spec) => spec,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.orchestrator.create_job(job_spec).await {
        Ok(job) => Ok(Json(json!({ "job": job }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_job(
    State(state): State<AppState>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let job_id = hodei_core::JobId::new();

    match state.orchestrator.get_job(&job_id).await {
        Ok(Some(job)) => Ok(Json(json!({ "job": job }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn cancel_job(
    State(state): State<AppState>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let job_id = hodei_core::JobId::new();

    match state.orchestrator.cancel_job(&job_id).await {
        Ok(_) => Ok(Json(json!({ "message": "Job cancelled" }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn register_worker(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("worker");
    let cpu_cores = payload
        .get("cpu_cores")
        .and_then(|v| v.as_u64())
        .unwrap_or(4) as u32;
    let memory_gb = payload
        .get("memory_gb")
        .and_then(|v| v.as_u64())
        .unwrap_or(8192);

    let capabilities = WorkerCapabilities::new(cpu_cores, memory_gb);
    let worker = Worker::new(hodei_core::WorkerId::new(), name.to_string(), capabilities);

    match state.scheduler.register_worker(worker).await {
        Ok(_) => Ok(Json(json!({ "message": "Worker registered" }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn worker_heartbeat(
    State(state): State<AppState>,
    axum::extract::Path(_id): axum::extract::Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let worker_id = hodei_core::WorkerId::new();

    // For now, we don't have resource usage from the endpoint
    // In a real implementation, this would come from the request body
    let resource_usage = None;

    match state
        .scheduler
        .process_heartbeat(&worker_id, resource_usage)
        .await
    {
        Ok(_) => Ok(Json(json!({ "message": "Heartbeat processed" }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_metrics(State(state): State<AppState>) -> Result<String, StatusCode> {
    match state.metrics.gather() {
        Ok(metrics) => Ok(metrics),
        Err(e) => {
            tracing::error!("Failed to gather metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
