//! Hodei Pipelines Server - Monolithic Modular Architecture with OpenAPI Documentation
//!
//! # API Documentation
//!
//! Interactive API documentation is available at: http://localhost:8080/api/docs
//!
//! OpenAPI specification: http://localhost:8080/api/openapi.json

use async_trait::async_trait;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use std::sync::Arc;
use tokio::sync::RwLock;

use hodei_adapters::{
    DockerProvider, GrpcWorkerClient, HttpWorkerClient, InMemoryBus, InMemoryJobRepository,
    InMemoryPipelineRepository, InMemoryWorkerRepository, ProviderConfig, ProviderType,
    WorkerRegistrationAdapter,
};
use hodei_core::{Job, JobId, Worker, WorkerId};
use hodei_core::{JobSpec, ResourceQuota, WorkerCapabilities};
use hodei_modules::{
    OrchestratorModule, SchedulerModule, WorkerManagementConfig, WorkerManagementService,
    multi_tenancy_quota_manager::MultiTenancyQuotaManager,
};
use hodei_ports::worker_provider::WorkerProvider;
use hodei_ports::{ProviderFactoryTrait, SchedulerPort};
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

mod metrics;
use metrics::MetricsRegistry;

mod auth;
mod grpc;

// Tenant Management module (EPIC-09)
mod tenant_management;
use tenant_management::{TenantAppState, TenantManagementService, tenant_routes};

// Resource Quotas module (US-09.1.2)
mod resource_quotas;
use resource_quotas::{ResourceQuotasAppState, ResourceQuotasService, resource_quotas_routes};

// Quota Enforcement module (US-09.1.3)
mod quota_enforcement;
use quota_enforcement::{
    QuotaEnforcementAppState, QuotaEnforcementService, quota_enforcement_routes,
};

// Burst Capacity module (US-09.1.4)
mod burst_capacity;
use burst_capacity::{BurstCapacityAppState, BurstCapacityService, burst_capacity_routes};

// Job Prioritization module (US-09.2.1)
mod job_prioritization;
use job_prioritization::{
    JobPrioritizationAppState, JobPrioritizationService, job_prioritization_routes,
};

// WFQ Integration module (US-09.2.2) - TODO: Fix handler signatures
// mod wfq_integration;
// use wfq_integration::{WFQIntegrationAppState, WFQIntegrationService, wfq_integration_routes};

// SLA Tracking module (US-09.2.3)
mod sla_tracking;
use sla_tracking::{SLATrackingAppState, SLATrackingService, sla_tracking_routes};

// Queue Status module (US-09.2.4)
mod queue_status;
use queue_status::{QueueStatusAppState, queue_status_routes};

// Resource Pool CRUD module (US-09.3.1)
mod resource_pool_crud;
use resource_pool_crud::{ResourcePoolCrudAppState, resource_pool_crud_routes};

// Static Pool Management module (US-09.3.2)
mod static_pool_management;
use static_pool_management::{StaticPoolManagementAppState, static_pool_management_routes};

// Dynamic Pool Management module (US-09.3.3)
mod dynamic_pool_management;
use dynamic_pool_management::{DynamicPoolManagementAppState, dynamic_pool_management_routes};

// Pool Lifecycle module (US-09.3.4)
mod pool_lifecycle;
use pool_lifecycle::{PoolLifecycleAppState, pool_lifecycle_routes};

// Scaling Policies module (US-09.4.1)
mod scaling_policies;
use scaling_policies::{ScalingPoliciesAppState, scaling_policies_routes};

// Scaling Triggers module (US-09.4.2)
mod scaling_triggers;
use scaling_triggers::{ScalingTriggersAppState, scaling_triggers_routes};

// Cooldown Management module (US-09.4.3)
mod cooldown_management;
use cooldown_management::{CooldownsAppState, cooldowns_routes};

// Scaling History module (US-09.4.4)
mod scaling_history;
use scaling_history::{ScalingHistoryAppState, scaling_history_routes};

// Resource Pool Metrics module (US-09.5.1)
mod resource_pool_metrics;
use resource_pool_metrics::{ResourcePoolMetricsAppState, resource_pool_metrics_routes};

// Cost Optimization module (US-09.5.2)
mod cost_optimization;
use cost_optimization::{CostOptimizationAppState, cost_optimization_routes};

// Cost Reports module (US-09.5.3)
mod cost_reports;
use cost_reports::{CostReportsAppState, cost_reports_routes};

// Historical Metrics module (US-09.5.4)
mod historical_metrics;
use historical_metrics::{HistoricalMetricsAppState, historical_metrics_routes};

// Prometheus Integration module (US-09.6.1)
mod prometheus_integration;
use prometheus_integration::{PrometheusIntegrationAppState, prometheus_integration_routes};

// Grafana Dashboards module (US-09.6.2)
mod grafana_dashboards;
use grafana_dashboards::{GrafanaDashboardsAppState, grafana_dashboards_routes};

// Alerting Rules module (US-09.6.3)
mod alerting_rules;
use alerting_rules::{AlertingRulesAppState, alerting_rules_routes};

// Observability API module (US-09.6.4)
mod observability_api;
use observability_api::{ObservabilityApiAppState, observability_api_routes};

// Define a concrete type for WorkerManagementService
// For now, we'll use a mock scheduler port
#[derive(Clone)]
struct MockSchedulerPort;

#[async_trait::async_trait]
impl SchedulerPort for MockSchedulerPort {
    async fn register_worker(
        &self,
        _worker: &Worker,
    ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn unregister_worker(
        &self,
        _worker_id: &WorkerId,
    ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn get_registered_workers(
        &self,
    ) -> Result<Vec<WorkerId>, hodei_ports::scheduler_port::SchedulerError> {
        Ok(Vec::new())
    }
}

type ConcreteWorkerManagementService = WorkerManagementService<DockerProvider, MockSchedulerPort>;

#[derive(Clone)]
struct AppState {
    scheduler: Arc<dyn SchedulerPort>,
    worker_management: Arc<ConcreteWorkerManagementService>,
    metrics: MetricsRegistry,
    // EPIC-09: Tenant management
    tenant_app_state: TenantAppState,
    // US-09.1.2: Resource Quotas
    resource_quotas_app_state: ResourceQuotasAppState,
    // US-09.1.3: Quota Enforcement
    quota_enforcement_app_state: QuotaEnforcementAppState,
    // US-09.1.4: Burst Capacity
    burst_capacity_app_state: BurstCapacityAppState,
    // US-09.2.1: Job Prioritization
    job_prioritization_app_state: JobPrioritizationAppState,
    // US-09.2.2: WFQ Integration - TODO: Fix handler signatures
    // wfq_integration_app_state: WFQIntegrationAppState,
    // US-09.2.3: SLA Tracking
    sla_tracking_app_state: SLATrackingAppState,
    // US-09.2.4: Queue Status
    queue_status_app_state: QueueStatusAppState,
    // US-09.3.1: Resource Pool CRUD
    resource_pool_crud_app_state: ResourcePoolCrudAppState,
    // US-09.3.2: Static Pool Management
    static_pool_management_app_state: StaticPoolManagementAppState,
    // US-09.3.3: Dynamic Pool Management
    dynamic_pool_management_app_state: DynamicPoolManagementAppState,
    // US-09.3.4: Pool Lifecycle
    pool_lifecycle_app_state: PoolLifecycleAppState,
    // US-09.4.1: Scaling Policies
    scaling_policies_app_state: ScalingPoliciesAppState,
    // US-09.4.2: Scaling Triggers
    scaling_triggers_app_state: ScalingTriggersAppState,
    // US-09.4.3: Cooldown Management
    cooldown_app_state: CooldownsAppState,
    // US-09.4.4: Scaling History
    scaling_history_app_state: ScalingHistoryAppState,
    // US-09.5.1: Resource Pool Metrics
    resource_pool_metrics_app_state: ResourcePoolMetricsAppState,
    // US-09.5.2: Cost Optimization
    cost_optimization_app_state: CostOptimizationAppState,
    // US-09.5.3: Cost Reports
    cost_reports_app_state: CostReportsAppState,
    // US-09.5.4: Historical Metrics
    historical_metrics_app_state: HistoricalMetricsAppState,
    // US-09.6.1: Prometheus Integration
    prometheus_integration_app_state: PrometheusIntegrationAppState,
    // US-09.6.2: Grafana Dashboards
    grafana_dashboards_app_state: GrafanaDashboardsAppState,
    // US-09.6.3: Alerting Rules
    alerting_rules_app_state: AlertingRulesAppState,
    // US-09.6.4: Observability API
    observability_api_app_state: ObservabilityApiAppState,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Hodei Pipelines Server");
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
    )) as Arc<dyn SchedulerPort>;

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
    let config = WorkerManagementConfig {
        registration_enabled: true,
        registration_max_retries: 3,
    };

    // Create a Docker provider for now (in production, could use other providers)
    let provider_config = ProviderConfig::docker("default".to_string());
    let provider = DockerProvider::new(provider_config)
        .await
        .expect("Failed to create Docker provider");

    let worker_management = Arc::new(WorkerManagementService::new(provider, config));

    // Create Tenant Management Service (EPIC-09)
    let quota_manager_config =
        hodei_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
    let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
    let tenant_service = TenantManagementService::new(quota_manager.clone());
    let tenant_app_state = TenantAppState { tenant_service };

    // Create tenant routes before moving into AppState
    let tenant_router = tenant_routes().with_state(tenant_app_state.clone());

    // Create resource quotas service
    let resource_quotas_service = ResourceQuotasService::new(quota_manager.clone());
    let resource_quotas_app_state = ResourceQuotasAppState {
        service: resource_quotas_service,
    };

    // Create quota enforcement service
    let policy = hodei_modules::quota_enforcement::EnforcementPolicy {
        strict_mode: false,
        queue_on_violation: true,
        preemption_enabled: false,
        grace_period: std::time::Duration::from_secs(30),
        enforcement_delay: std::time::Duration::from_secs(5),
        max_queue_size: 100,
        enable_burst_enforcement: true,
    };
    let quota_enforcement_service = QuotaEnforcementService::new(quota_manager.clone(), policy);
    let quota_enforcement_app_state = QuotaEnforcementAppState {
        service: quota_enforcement_service,
    };

    // Create burst capacity service
    let burst_config = hodei_modules::burst_capacity_manager::BurstCapacityConfig {
        enabled: true,
        default_multiplier: 1.5,
        max_burst_duration: std::time::Duration::from_secs(3600),
        burst_cooldown: std::time::Duration::from_secs(600),
        global_burst_pool_ratio: 0.2,
        max_concurrent_bursts: 100,
        burst_cost_multiplier: 2.0,
        enable_burst_queuing: true,
    };
    let burst_capacity_service = BurstCapacityService::new(quota_manager.clone(), burst_config);
    let burst_capacity_app_state = BurstCapacityAppState {
        service: burst_capacity_service,
    };

    // Create job prioritization service
    let sla_tracker = Arc::new(hodei_modules::sla_tracking::SLATracker::new());
    let prioritization_engine =
        hodei_modules::queue_prioritization::QueuePrioritizationEngine::new(sla_tracker.clone());
    let job_prioritization_service = JobPrioritizationService::new(prioritization_engine);
    let job_prioritization_app_state = JobPrioritizationAppState {
        service: job_prioritization_service,
    };

    // Create SLA tracking service
    let sla_tracking_service =
        SLATrackingService::new(hodei_modules::sla_tracking::SLATracker::new());
    let sla_tracking_app_state = SLATrackingAppState {
        service: sla_tracking_service,
    };

    // Create queue status service
    let wfq_engine = Arc::new(RwLock::new(
        hodei_modules::weighted_fair_queuing::WeightedFairQueueingEngine::new(
            hodei_modules::weighted_fair_queuing::WFQConfig::default(),
        ),
    ));
    let prioritization_engine_for_status = Arc::new(RwLock::new(
        hodei_modules::queue_prioritization::QueuePrioritizationEngine::new(sla_tracker.clone()),
    ));
    let queue_status_app_state = QueueStatusAppState {
        prioritization_engine: prioritization_engine_for_status,
        wfq_engine,
    };

    // Create resource pool CRUD service
    let pools = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let pool_statuses = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let resource_pool_crud_app_state = ResourcePoolCrudAppState {
        pools,
        pool_statuses,
    };

    // Create static pool management service
    let static_pools = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let static_pool_configs = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let static_pool_management_app_state = StaticPoolManagementAppState {
        static_pools,
        pool_configs: static_pool_configs,
    };

    // Create dynamic pool management service
    let dynamic_pools = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let dynamic_pool_configs = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let dynamic_scaling_history = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let dynamic_pool_management_app_state = DynamicPoolManagementAppState {
        dynamic_pools,
        pool_configs: dynamic_pool_configs,
        scaling_history: dynamic_scaling_history,
    };

    // Create pool lifecycle service
    let pool_lifecycle_service = Arc::new(pool_lifecycle::PoolLifecycleService::new());
    let pool_lifecycle_app_state = PoolLifecycleAppState {
        service: pool_lifecycle_service,
    };

    // Create scaling policies service
    let scaling_policies_service = Arc::new(scaling_policies::ScalingPoliciesService::new());
    let scaling_policies_app_state = ScalingPoliciesAppState {
        service: scaling_policies_service,
    };

    // Create scaling triggers service
    let scaling_triggers_service = Arc::new(scaling_triggers::ScalingTriggersService::new());
    let scaling_triggers_app_state = ScalingTriggersAppState {
        service: scaling_triggers_service,
    };

    // Create cooldown management service
    let cooldown_service = Arc::new(cooldown_management::CooldownsService::new());
    let cooldown_app_state = CooldownsAppState {
        service: cooldown_service,
    };

    // Create scaling history service
    let scaling_history_service = Arc::new(scaling_history::ScalingHistoryService::new());
    let scaling_history_app_state = ScalingHistoryAppState {
        service: scaling_history_service,
    };

    // Create resource pool metrics service
    let resource_pool_metrics_service =
        Arc::new(resource_pool_metrics::ResourcePoolMetricsService::new());
    let resource_pool_metrics_app_state = ResourcePoolMetricsAppState {
        service: resource_pool_metrics_service,
    };

    // Create cost optimization service
    let cost_optimization_service = Arc::new(cost_optimization::CostOptimizationService::new());
    let cost_optimization_app_state = CostOptimizationAppState {
        service: cost_optimization_service,
    };

    // Create cost reports service
    let cost_reports_service = Arc::new(cost_reports::CostReportsService::new());
    let cost_reports_app_state = CostReportsAppState {
        service: cost_reports_service,
    };

    // Create historical metrics service
    let historical_metrics_service = Arc::new(historical_metrics::HistoricalMetricsService::new());
    let historical_metrics_app_state = HistoricalMetricsAppState {
        service: historical_metrics_service,
    };

    // Create Prometheus integration service
    let prometheus_integration_service =
        Arc::new(prometheus_integration::PrometheusIntegrationService::new());
    let prometheus_integration_app_state = PrometheusIntegrationAppState {
        service: prometheus_integration_service,
    };

    // Create Grafana dashboards service
    let grafana_dashboards_service = Arc::new(grafana_dashboards::GrafanaDashboardsService::new());
    let grafana_dashboards_app_state = GrafanaDashboardsAppState {
        service: grafana_dashboards_service,
    };

    // Create alerting rules service
    let alerting_rules_service = Arc::new(alerting_rules::AlertingRulesService::new());
    let alerting_rules_app_state = AlertingRulesAppState {
        service: alerting_rules_service,
    };

    // Create observability API service
    let observability_api_service = Arc::new(observability_api::ObservabilityApiService::new());
    let observability_api_app_state = ObservabilityApiAppState {
        service: observability_api_service,
    };

    // Create WFQ integration service - TODO: Fix handler signatures
    // let wfq_config = hodei_modules::weighted_fair_queuing::WFQConfig {
    //     enable_virtual_time: true,
    //     min_weight: 0.1,
    //     max_weight: 10.0,
    //     default_strategy: hodei_modules::weighted_fair_queuing::WeightStrategy::BillingTier,
    //     starvation_threshold: 0.5,
    //     weight_update_interval: std::time::Duration::from_secs(60),
    //     default_packet_size: 1000,
    //     enable_dynamic_weights: true,
    //     starvation_window: std::time::Duration::from_secs(300),
    //     fair_share_window: std::time::Duration::from_secs(3600),
    // };
    // let wfq_engine =
    //     hodei_modules::weighted_fair_queuing::WeightedFairQueueingEngine::new(wfq_config);
    // let wfq_integration_service = WFQIntegrationService::new(wfq_engine);
    // let wfq_integration_app_state = WFQIntegrationAppState {
    //     service: wfq_integration_service,
    // };

    // Create routes that will be nested
    let resource_quotas_router =
        resource_quotas_routes().with_state(resource_quotas_app_state.clone());
    let quota_enforcement_router =
        quota_enforcement_routes().with_state(quota_enforcement_app_state.clone());
    let burst_capacity_router =
        burst_capacity_routes().with_state(burst_capacity_app_state.clone());
    let job_prioritization_router =
        job_prioritization_routes().with_state(job_prioritization_app_state.clone());
    let sla_tracking_router = sla_tracking_routes().with_state(sla_tracking_app_state.clone());
    let queue_status_router = queue_status_routes().with_state(queue_status_app_state.clone());
    let resource_pool_crud_router =
        resource_pool_crud_routes().with_state(resource_pool_crud_app_state.clone());
    let static_pool_management_router =
        static_pool_management_routes().with_state(static_pool_management_app_state.clone());
    let dynamic_pool_management_router =
        dynamic_pool_management_routes().with_state(dynamic_pool_management_app_state.clone());
    let pool_lifecycle_router =
        pool_lifecycle_routes().with_state(pool_lifecycle_app_state.clone());
    let scaling_policies_router =
        scaling_policies_routes().with_state(scaling_policies_app_state.clone());
    let scaling_triggers_router =
        scaling_triggers_routes().with_state(scaling_triggers_app_state.clone());
    let cooldown_router = cooldowns_routes().with_state(cooldown_app_state.clone());
    let scaling_history_router =
        scaling_history_routes().with_state(scaling_history_app_state.clone());
    let resource_pool_metrics_router =
        resource_pool_metrics_routes().with_state(resource_pool_metrics_app_state.clone());
    let cost_optimization_router =
        cost_optimization_routes().with_state(cost_optimization_app_state.clone());
    let cost_reports_router = cost_reports_routes().with_state(cost_reports_app_state.clone());
    let historical_metrics_router =
        historical_metrics_routes().with_state(historical_metrics_app_state.clone());
    let prometheus_integration_router =
        prometheus_integration_routes().with_state(prometheus_integration_app_state.clone());
    let grafana_dashboards_router =
        grafana_dashboards_routes().with_state(grafana_dashboards_app_state.clone());
    let alerting_rules_router =
        alerting_rules_routes().with_state(alerting_rules_app_state.clone());
    let observability_api_router =
        observability_api_routes().with_state(observability_api_app_state.clone());
    // let wfq_integration_router =
    //     wfq_integration_routes().with_state(wfq_integration_app_state.clone());

    let app_state = AppState {
        scheduler: scheduler.clone(),
        worker_management: worker_management.clone(),
        metrics: metrics.clone(),
        tenant_app_state,
        resource_quotas_app_state,
        quota_enforcement_app_state,
        burst_capacity_app_state,
        job_prioritization_app_state,
        // wfq_integration_app_state,
        sla_tracking_app_state,
        queue_status_app_state,
        resource_pool_crud_app_state,
        static_pool_management_app_state,
        dynamic_pool_management_app_state,
        pool_lifecycle_app_state,
        scaling_policies_app_state,
        scaling_triggers_app_state,
        cooldown_app_state,
        scaling_history_app_state,
        resource_pool_metrics_app_state,
        cost_optimization_app_state,
        cost_reports_app_state,
        historical_metrics_app_state,
        prometheus_integration_app_state,
        grafana_dashboards_app_state,
        alerting_rules_app_state,
        observability_api_app_state,
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
                    spec: job.spec.clone(),
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
                    spec: job.spec.clone(),
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

            match scheduler.register_worker(&worker).await {
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
            let _worker_id = WorkerId::from_uuid(uuid::Uuid::parse_str(&id).map_err(|_| {
                axum::response::Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body("Invalid UUID".into())
                    .unwrap()
            })?);

            // TODO: Implement heartbeat processing
            Ok::<Json<MessageResponse>, axum::response::Response>(Json(MessageResponse {
                message: "Heartbeat processed successfully".to_string(),
            }))
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
        let worker_management: Arc<ConcreteWorkerManagementService> =
            Arc::clone(&app_state.worker_management);
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
        let worker_management: Arc<ConcreteWorkerManagementService> =
            Arc::clone(&app_state.worker_management);
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
        let worker_management: Arc<ConcreteWorkerManagementService> =
            Arc::clone(&app_state.worker_management);
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
        let worker_management: Arc<ConcreteWorkerManagementService> =
            Arc::clone(&app_state.worker_management);
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
        let worker_management: Arc<ConcreteWorkerManagementService> =
            Arc::clone(&app_state.worker_management);
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
        let worker_management: Arc<ConcreteWorkerManagementService> =
            Arc::clone(&app_state.worker_management);
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
        let worker_management: Arc<ConcreteWorkerManagementService> =
            Arc::clone(&app_state.worker_management);
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
                    "title": "Hodei Pipelines API",
                    "version": "1.0.0",
                    "description": "API para gestionar jobs, workers y pipelines en el sistema Hodei Pipelines"
                },
                "paths": {},
                "components": {
                    "schemas": {}
                }
            }))
        }))
        .route("/health", get(health_handler))
        .with_state(app_state.clone())
        // Main API routes
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
        // EPIC-09: Tenant Management Routes
        .nest("/api/v1", tenant_router)
        // US-09.1.2: Resource Quotas Routes
        .nest("/api/v1", resource_quotas_router)
        // US-09.1.3: Quota Enforcement Routes
        .nest("/api/v1", quota_enforcement_router)
        // US-09.1.4: Burst Capacity Routes
        .nest("/api/v1", burst_capacity_router)
        // US-09.2.1: Job Prioritization Routes
        .nest("/api/v1", job_prioritization_router)
        // US-09.2.2: WFQ Integration Routes - TODO: Fix handler signatures
        // .nest("/api/v1", wfq_integration_router)
        // US-09.2.3: SLA Tracking Routes
        .nest("/api/v1", sla_tracking_router)
        // US-09.2.4: Queue Status Routes
        .nest("/api/v1", queue_status_router)
        // US-09.3.1: Resource Pool CRUD Routes
        .nest("/api/v1", resource_pool_crud_router)
        // US-09.3.2: Static Pool Management Routes
        .nest("/api/v1", static_pool_management_router)
        // US-09.3.3: Dynamic Pool Management Routes
        .nest("/api/v1", dynamic_pool_management_router)
        // US-09.3.4: Pool Lifecycle Routes
        .nest("/api/v1", pool_lifecycle_router)
        // US-09.4.1: Scaling Policies Routes
        .nest("/api/v1", scaling_policies_router)
        // US-09.4.2: Scaling Triggers Routes
        .nest("/api/v1", scaling_triggers_router)
        // US-09.4.3: Cooldown Management Routes
        .nest("/api/v1", cooldown_router)
        // US-09.4.4: Scaling History Routes
        .nest("/api/v1", scaling_history_router)
        // US-09.5.1: Resource Pool Metrics Routes
        .nest("/api/v1", resource_pool_metrics_router)
        // US-09.5.2: Cost Optimization Routes
        .nest("/api/v1", cost_optimization_router)
        // US-09.5.3: Cost Reports Routes
        .nest("/api/v1", cost_reports_router)
        // US-09.5.4: Historical Metrics Routes
        .nest("/api/v1", historical_metrics_router)
        // US-09.6.1: Prometheus Integration Routes
        .nest("/api/v1", prometheus_integration_router)
        // US-09.6.2: Grafana Dashboards Routes
        .nest("/api/v1", grafana_dashboards_router)
        // US-09.6.3: Alerting Rules Routes
        .nest("/api/v1", alerting_rules_router)
        // US-09.6.4: Observability API Routes
        .nest("/api/v1", observability_api_router)
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
