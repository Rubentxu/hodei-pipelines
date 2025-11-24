//! Hodei Jobs Server - Monolithic Modular Architecture with OpenAPI Documentation
//!
//! # API Documentation
//!
//! Interactive API documentation is available at: http://localhost:8080/api/docs
//!
//! OpenAPI specification: http://localhost:8080/api/openapi.json

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
    InMemoryPipelineRepository, InMemoryWorkerRepository, ProviderConfig, ProviderType,
};
use hodei_core::{Job, JobId, Worker, WorkerId};
use hodei_modules::{OrchestratorModule, SchedulerModule, WorkerManagementService};
use hodei_shared_types::{JobSpec, ResourceQuota, WorkerCapabilities};
use serde_json::{Value, json};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

// API types
mod api_docs;
use api_docs::{
    CreateDynamicWorkerRequest, CreateDynamicWorkerResponse, CreateJobRequest,
    CreateProviderRequest, DynamicWorkerStatusResponse, HealthResponse, JobResponse,
    ListDynamicWorkersResponse, ListProvidersResponse, MessageResponse,
    ProviderCapabilitiesResponse, ProviderInfo, ProviderResponse, ProviderTypeDto,
    RegisterWorkerRequest,
};

use utoipa::{IntoParams, ToSchema};

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
    worker_management: Arc<WorkerManagementService>,
    metrics: MetricsRegistry,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Hodei Jobs Server");
    info!("📚 API Documentation: http://localhost:8080/api/docs");
    info!("🔗 OpenAPI Spec: http://localhost:8080/api/openapi.json");

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

    // Create Worker Management Service
    let worker_management =
        hodei_modules::worker_management::create_default_worker_management_service()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to initialize worker management: {}", e);
                anyhow::anyhow!("Failed to initialize worker management: {}", e)
            })?;

    scheduler.clone().start().await?;

    let app_state = AppState {
        scheduler: scheduler.clone(),
        orchestrator: orchestrator.clone(),
        worker_management: Arc::new(worker_management),
        metrics: metrics.clone(),
    };

    // Handler functions
    let health_handler = {
        || async {
            Json(HealthResponse {
                status: "healthy".to_string(),
                service: "hodei-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                architecture: "monolithic_modular".to_string(),
            })
        }
    };

    let create_job_handler = {
        let orchestrator = orchestrator.clone();
        move |State(_): State<AppState>, Json(payload): Json<CreateJobRequest>| async move {
            let job_spec = JobSpec {
                name: payload.name,
                image: payload.image,
                command: payload.command,
                resources: payload.resources,
                timeout_ms: payload.timeout_ms,
                retries: payload.retries,
                env: payload.env,
                secret_refs: payload.secret_refs,
            };

            match orchestrator.create_job(job_spec).await {
                Ok(job) => Ok(Json(JobResponse {
                    id: job.id.to_string(),
                    name: job.name().to_string(),
                    spec: job.spec.as_ref().clone(),
                    state: job.state.as_str().to_string(),
                    created_at: job.created_at,
                    updated_at: job.updated_at,
                    started_at: job.started_at,
                    completed_at: job.completed_at,
                    result: Some(job.result),
                })),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    };

    let get_job_handler = {
        let orchestrator = orchestrator.clone();
        move |State(_): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>| async move {
            let job_id =
                JobId::from(uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?);

            match orchestrator.get_job(&job_id).await {
                Ok(Some(job)) => Ok(Json(JobResponse {
                    id: job.id.to_string(),
                    name: job.name().to_string(),
                    spec: job.spec.as_ref().clone(),
                    state: job.state.as_str().to_string(),
                    created_at: job.created_at,
                    updated_at: job.updated_at,
                    started_at: job.started_at,
                    completed_at: job.completed_at,
                    result: Some(job.result),
                })),
                Ok(None) => Err(StatusCode::NOT_FOUND),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    };

    let cancel_job_handler = {
        let orchestrator = orchestrator.clone();
        move |State(_): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>| async move {
            let job_id =
                JobId::from(uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?);

            match orchestrator.cancel_job(&job_id).await {
                Ok(_) => Ok(Json(MessageResponse {
                    message: "Job cancelled successfully".to_string(),
                })),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    };

    let register_worker_handler = {
        let scheduler = scheduler.clone();
        move |State(_): State<AppState>, Json(payload): Json<RegisterWorkerRequest>| async move {
            let capabilities = WorkerCapabilities::new(payload.cpu_cores, payload.memory_gb * 1024);
            let worker = Worker::new(WorkerId::new(), payload.name, capabilities);

            match scheduler.register_worker(worker).await {
                Ok(_) => Ok(Json(MessageResponse {
                    message: "Worker registered successfully".to_string(),
                })),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    };

    let worker_heartbeat_handler = {
        let scheduler = scheduler.clone();
        move |State(_): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>| async move {
            let worker_id = WorkerId::from_uuid(
                uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?,
            );

            match scheduler.process_heartbeat(&worker_id, None).await {
                Ok(_) => Ok(Json(MessageResponse {
                    message: "Heartbeat processed successfully".to_string(),
                })),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    };

    let get_metrics_handler = {
        move |State(state): State<AppState>| async move {
            match state.metrics.gather() {
                Ok(metrics) => Ok(metrics),
                Err(e) => {
                    tracing::error!("Failed to gather metrics: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    };

    let create_dynamic_worker_handler = {
        let worker_management = Arc::clone(&app_state.worker_management);
        move |State(_): State<AppState>, Json(payload): Json<CreateDynamicWorkerRequest>| async move {
            // Parse provider type
            let provider_type = match payload.provider_type.to_lowercase().as_str() {
                "docker" => ProviderType::Docker,
                "kubernetes" | "k8s" => ProviderType::Kubernetes,
                _ => {
                    return Err(StatusCode::BAD_REQUEST);
                }
            };

            // Build provider config with custom options
            let mut config = match provider_type {
                ProviderType::Docker => {
                    ProviderConfig::docker(format!("worker-{}", uuid::Uuid::new_v4()))
                }
                ProviderType::Kubernetes => {
                    ProviderConfig::kubernetes(format!("worker-{}", uuid::Uuid::new_v4()))
                }
            };

            // Set custom image if provided, otherwise use default from payload
            if let Some(image) = payload.custom_image.or(Some(payload.image)) {
                config = config.with_image(image);
            }

            // Set custom pod template if provided (Kubernetes only)
            if provider_type == ProviderType::Kubernetes {
                if let Some(template) = payload.custom_pod_template {
                    config = config.with_pod_template(template);
                }
            }

            // Set namespace if provided (Kubernetes only)
            if let Some(namespace) = payload.namespace {
                config = config.with_namespace(namespace);
            }

            match worker_management
                .provision_worker_with_config(config, payload.cpu_cores, payload.memory_mb)
                .await
            {
                Ok(worker) => Ok(Json(CreateDynamicWorkerResponse {
                    worker_id: worker.id.to_string(),
                    container_id: None, // Could extract from metadata
                    state: "starting".to_string(),
                    message: "Worker provisioned successfully".to_string(),
                })),
                Err(e) => {
                    tracing::error!("Failed to provision worker: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    };

    let get_dynamic_worker_status_handler = {
        let worker_management = Arc::clone(&app_state.worker_management);
        move |State(_): State<AppState>,
              axum::extract::Path(worker_id): axum::extract::Path<String>| async move {
            let worker_id_uuid =
                uuid::Uuid::parse_str(&worker_id).map_err(|_| StatusCode::BAD_REQUEST)?;

            let worker_id = hodei_core::WorkerId::from_uuid(worker_id_uuid);

            match worker_management.get_worker_status(&worker_id).await {
                Ok(status) => Ok(Json(DynamicWorkerStatusResponse {
                    worker_id: worker_id.to_string(),
                    state: status.as_str().to_string(),
                    container_id: None,
                    created_at: chrono::Utc::now(),
                })),
                Err(_) => Err(StatusCode::NOT_FOUND),
            }
        }
    };

    let list_dynamic_workers_handler = {
        let worker_management = Arc::clone(&app_state.worker_management);
        move |State(_): State<AppState>| async move {
            match worker_management.list_workers().await {
                Ok(worker_ids) => {
                    let mut workers = Vec::new();
                    for worker_id in worker_ids {
                        match worker_management.get_worker_status(&worker_id).await {
                            Ok(status) => {
                                workers.push(DynamicWorkerStatusResponse {
                                    worker_id: worker_id.to_string(),
                                    state: status.as_str().to_string(),
                                    container_id: None,
                                    created_at: chrono::Utc::now(),
                                });
                            }
                            Err(_) => {
                                // Skip workers with errors
                                workers.push(DynamicWorkerStatusResponse {
                                    worker_id: worker_id.to_string(),
                                    state: "unknown".to_string(),
                                    container_id: None,
                                    created_at: chrono::Utc::now(),
                                });
                            }
                        }
                    }

                    Ok(Json(ListDynamicWorkersResponse { workers }))
                }
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    };

    let get_provider_capabilities_handler = {
        let worker_management = Arc::clone(&app_state.worker_management);
        move |State(_): State<AppState>| async move {
            match worker_management.get_provider_capabilities().await {
                Ok(capabilities) => Ok(Json(ProviderCapabilitiesResponse {
                    provider_type: "docker".to_string(), // From provider
                    name: "docker-provider".to_string(),
                    capabilities: api_docs::ProviderCapabilitiesInfo {
                        supports_auto_scaling: capabilities.supports_auto_scaling,
                        supports_health_checks: capabilities.supports_health_checks,
                        supports_volumes: capabilities.supports_volumes,
                        max_workers: capabilities.max_workers,
                        estimated_provision_time_ms: capabilities.estimated_provision_time_ms,
                    },
                })),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    };

    let stop_dynamic_worker_handler = {
        let worker_management = Arc::clone(&app_state.worker_management);
        move |State(_): State<AppState>,
              axum::extract::Path(worker_id): axum::extract::Path<String>| async move {
            let worker_id_uuid =
                uuid::Uuid::parse_str(&worker_id).map_err(|_| StatusCode::BAD_REQUEST)?;

            let worker_id = hodei_core::WorkerId::from_uuid(worker_id_uuid);

            match worker_management.stop_worker(&worker_id, true).await {
                Ok(_) => Ok(Json(api_docs::MessageResponse {
                    message: "Worker stopped successfully".to_string(),
                })),
                Err(_) => Err(StatusCode::NOT_FOUND),
            }
        }
    };

    let delete_dynamic_worker_handler = {
        let worker_management = Arc::clone(&app_state.worker_management);
        move |State(_): State<AppState>,
              axum::extract::Path(worker_id): axum::extract::Path<String>| async move {
            let worker_id_uuid =
                uuid::Uuid::parse_str(&worker_id).map_err(|_| StatusCode::BAD_REQUEST)?;

            let worker_id = hodei_core::WorkerId::from_uuid(worker_id_uuid);

            match worker_management.delete_worker(&worker_id).await {
                Ok(_) => Ok(Json(api_docs::MessageResponse {
                    message: "Worker deleted successfully".to_string(),
                })),
                Err(_) => Err(StatusCode::NOT_FOUND),
            }
        }
    };

    let list_providers_handler = {
        move |State(_): State<AppState>| async move {
            // For now, return the built-in providers
            // In a real implementation, this would query a repository
            let providers = vec![
                ProviderInfo {
                    provider_type: "docker".to_string(),
                    name: "docker-provider".to_string(),
                    status: "active".to_string(),
                },
                ProviderInfo {
                    provider_type: "kubernetes".to_string(),
                    name: "kubernetes-provider".to_string(),
                    status: "active".to_string(),
                },
            ];

            Ok::<axum::Json<ListProvidersResponse>, StatusCode>(Json(ListProvidersResponse {
                providers,
            }))
        }
    };

    let create_provider_handler = {
        let worker_management = Arc::clone(&app_state.worker_management);
        move |State(_): State<AppState>, Json(payload): Json<CreateProviderRequest>| async move {
            // Build provider config
            let provider_type = match payload.provider_type {
                ProviderTypeDto::Docker => ProviderType::Docker,
                ProviderTypeDto::Kubernetes => ProviderType::Kubernetes,
            };

            let mut config = match provider_type {
                ProviderType::Docker => ProviderConfig::docker(payload.name.clone()),
                ProviderType::Kubernetes => ProviderConfig::kubernetes(payload.name.clone()),
            };

            // Set custom options
            if let Some(ref image) = payload.custom_image {
                config = config.with_image(image.clone());
            }

            if let Some(ref template) = payload.custom_pod_template {
                config = config.with_pod_template(template.clone());
            }

            if let Some(ref namespace) = payload.namespace {
                config = config.with_namespace(namespace.clone());
            }

            if let Some(ref docker_host) = payload.docker_host {
                config = config.with_docker_host(docker_host.clone());
            }

            // In a real implementation, this would register the provider
            // For now, just return the provider info
            let response = ProviderResponse {
                provider_type: provider_type.as_str().to_string(),
                name: payload.name,
                namespace: payload.namespace,
                custom_image: payload.custom_image,
                status: "active".to_string(),
                created_at: chrono::Utc::now(),
            };

            Ok::<axum::Json<ProviderResponse>, StatusCode>(Json(response))
        }
    };

    let app = Router::new()
        // API Documentation
        .route("/api/openapi.json", get(|| async {
            use serde_json::json;

            Json(json!({
                "openapi": "3.0.0",
                "info": {
                    "title": "Hodei Jobs API",
                    "version": "1.0.0",
                    "description": "API para gestionar jobs, workers y pipelines en el sistema Hodei Jobs"
                },
                "paths": {},
                "components": {
                    "schemas": {}
                }
            }))
        }))
        .route("/health", get(health_handler))
        .route("/api/v1/jobs", post(create_job_handler))
        .route("/api/v1/jobs/{id}", get(get_job_handler))
        .route("/api/v1/jobs/{id}/cancel", post(cancel_job_handler))
        .route("/api/v1/workers", post(register_worker_handler))
        .route(
            "/api/v1/workers/{id}/heartbeat",
            post(worker_heartbeat_handler),
        )
        .route("/api/v1/metrics", get(get_metrics_handler))
        .route("/api/v1/dynamic-workers", post(create_dynamic_worker_handler))
        .route("/api/v1/dynamic-workers", get(list_dynamic_workers_handler))
        .route(
            "/api/v1/dynamic-workers/{id}",
            get(get_dynamic_worker_status_handler),
        )
        .route(
            "/api/v1/dynamic-workers/{id}/stop",
            post(stop_dynamic_worker_handler),
        )
        .route(
            "/api/v1/dynamic-workers/{id}",
            axum::routing::delete(delete_dynamic_worker_handler),
        )
        .route("/api/v1/providers/capabilities", get(get_provider_capabilities_handler))
        .route("/api/v1/providers", get(list_providers_handler))
        .route("/api/v1/providers", post(create_provider_handler))
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
