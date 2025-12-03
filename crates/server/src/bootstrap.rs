//! Server Bootstrap - Production Initialization

use hodei_pipelines_adapters::{
    InMemoryBus, MetricsPersistenceConfig, MetricsPersistenceService, MetricsTimeseriesRepository,
    PostgreSqlJobRepository, PostgreSqlPermissionRepository, PostgreSqlPipelineExecutionRepository,
    PostgreSqlPipelineRepository, PostgreSqlRoleRepository, PostgreSqlWorkerRepository,
    config::AppConfig,
};
use hodei_pipelines_modules::{
    ConcreteOrchestrator, PipelineExecutionConfig, PipelineExecutionService, PipelineService,
};
use hodei_pipelines_ports::{EventSubscriber, SchedulerPort};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Configuration error: {0}")]
    Config(#[from] hodei_pipelines_adapters::config::ConfigError),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, BootstrapError>;

#[derive(Clone)]
pub struct ServerComponents {
    pub config: AppConfig,
    pub event_subscriber: Arc<dyn EventSubscriber>,
    pub event_publisher: Arc<dyn hodei_pipelines_ports::EventPublisher>,
    pub orchestrator: Arc<dyn PipelineExecutionService + Send + Sync>,
    pub pipeline_service: Arc<dyn PipelineService + Send + Sync>,
    pub role_repository: Arc<PostgreSqlRoleRepository>,
    pub permission_repository: Arc<PostgreSqlPermissionRepository>,
    pub worker_repository: Arc<PostgreSqlWorkerRepository>,
    pub metrics_repository: Arc<MetricsTimeseriesRepository>,
    pub metrics_persistence_service: Arc<MetricsPersistenceService>,
    pub concrete_orchestrator: Arc<ConcreteOrchestrator>,
    pub scheduler: Arc<dyn SchedulerPort + Send + Sync>,
    #[allow(dead_code)]
    pub status: &'static str,
}

pub async fn initialize_server() -> Result<ServerComponents> {
    info!("🚀 Initializing Hodei Pipelines Server for Production");

    let config = AppConfig::load().map_err(|e| {
        error!("❌ Failed to load configuration: {}", e);
        BootstrapError::Config(e)
    })?;
    info!("✅ Configuration loaded successfully");

    let event_bus = Arc::new(InMemoryBus::new(1000));
    let event_subscriber: Arc<dyn EventSubscriber> = event_bus.clone();
    let event_publisher: Arc<dyn hodei_pipelines_ports::EventPublisher> = event_bus.clone();
    info!("✅ Event Bus initialized");

    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await
        .map_err(|e| {
            error!("❌ Failed to connect to PostgreSQL: {}", e);
            BootstrapError::General(anyhow::anyhow!("Failed to connect to PostgreSQL: {}", e))
        })?;
    info!("✅ PostgreSQL connection pool initialized");

    // Initialize repositories with migration file configuration
    let migrations_path = config.database.migrations_path.clone();
    let execution_repo = PostgreSqlPipelineExecutionRepository::new(
        pool.clone(),
        migrations_path.clone(),
        config.database.pipeline_execution_migration_file.clone(),
    );
    let job_repo = PostgreSqlJobRepository::new(
        pool.clone(),
        migrations_path.clone(),
        config.database.job_migration_file.clone(),
    );
    let pipeline_repo = PostgreSqlPipelineRepository::new(
        pool.clone(),
        migrations_path.clone(),
        config.database.pipeline_migration_file.clone(),
    );
    let worker_repo = Arc::new(PostgreSqlWorkerRepository::new(
        pool.clone(),
        migrations_path,
        config.database.worker_migration_file.clone(),
    ));

    let role_repository = Arc::new(PostgreSqlRoleRepository::new(pool.clone()));
    let permission_repository = Arc::new(PostgreSqlPermissionRepository::new(pool.clone()));

    // Initialize database schemas for all repositories
    info!("🗄️  Initializing database schemas...");
    execution_repo.init_schema().await.map_err(|e| {
        error!("❌ Failed to initialize pipeline execution schema: {}", e);
        BootstrapError::General(anyhow::anyhow!(
            "Failed to initialize pipeline execution schema: {}",
            e
        ))
    })?;
    info!("   ✅ Pipeline execution schema initialized");

    job_repo.init_schema().await.map_err(|e| {
        error!("❌ Failed to initialize job schema: {}", e);
        BootstrapError::General(anyhow::anyhow!("Failed to initialize job schema: {}", e))
    })?;
    info!("   ✅ Job schema initialized");

    pipeline_repo.init_schema().await.map_err(|e| {
        error!("❌ Failed to initialize pipeline schema: {}", e);
        BootstrapError::General(anyhow::anyhow!(
            "Failed to initialize pipeline schema: {}",
            e
        ))
    })?;
    info!("   ✅ Pipeline schema initialized");

    worker_repo.init_schema().await.map_err(|e| {
        error!("❌ Failed to initialize worker schema: {}", e);
        BootstrapError::General(anyhow::anyhow!("Failed to initialize worker schema: {}", e))
    })?;
    info!("   ✅ Worker schema initialized");

    info!("✅ All dynamic schema repositories initialized");

    let metrics_repository = Arc::new(MetricsTimeseriesRepository::new(pool.clone()));
    info!("✅ Metrics TSDB Repository initialized");

    let metrics_config = MetricsPersistenceConfig::default();
    let metrics_persistence_service = Arc::new(MetricsPersistenceService::new(
        metrics_config,
        metrics_repository.clone(),
    ));
    metrics_persistence_service.start().await.map_err(|e| {
        error!("❌ Failed to start Metrics Persistence Service: {}", e);
        BootstrapError::General(anyhow::anyhow!(
            "Failed to start Metrics Persistence Service: {}",
            e
        ))
    })?;
    info!("✅ Metrics Persistence Service initialized");

    info!("🎯 Initializing ConcreteOrchestrator...");
    let orchestrator_config = PipelineExecutionConfig {
        max_concurrent_steps: 10,
        max_retry_attempts: 3,
        step_timeout_secs: 3600,
        cleanup_interval_secs: 300,
    };

    let event_bus_for_orchestrator = InMemoryBus::new(1000);

    let concrete_orchestrator = Arc::new(
        ConcreteOrchestrator::new(
            Arc::new(execution_repo.clone()),
            Arc::new(job_repo.clone()),
            Arc::new(pipeline_repo.clone()),
            Arc::new(event_bus_for_orchestrator),
            orchestrator_config,
        )
        .map_err(|e| {
            error!("❌ Failed to initialize ConcreteOrchestrator: {}", e);
            BootstrapError::General(anyhow::anyhow!("Failed to initialize orchestrator: {}", e))
        })?,
    );

    let pipeline_service: Arc<dyn PipelineService + Send + Sync> = concrete_orchestrator.clone();
    let orchestrator_trait: Arc<dyn PipelineExecutionService + Send + Sync> =
        concrete_orchestrator.clone();

    info!("✅ ConcreteOrchestrator initialized");

    // Create scheduler using concrete implementation
    // Initialize Worker Client (using lazy connection for now, as agents connect to us)
    // We use a dummy endpoint because SchedulerModule will prioritize the agent stream (send_job_to_worker)
    // and only use this client for fallback or legacy workers.
    let dummy_channel = tonic::transport::Endpoint::from_static("http://0.0.0.0:0").connect_lazy();
    let worker_client = Arc::new(hodei_pipelines_adapters::GrpcWorkerClient::new(
        dummy_channel,
        std::time::Duration::from_secs(5),
    ));

    // Initialize Scheduler Module
    info!("📅 Initializing Scheduler...");
    let scheduler_config = hodei_pipelines_modules::scheduler::SchedulerConfig::default();

    let unified_repo = Arc::new(crate::unified_repository::UnifiedRepository::new(
        Arc::new(job_repo.clone()),
        Arc::new(pipeline_repo.clone()),
        role_repository.clone(), // Assuming role_repository is available and Clone
    ));

    let scheduler_module = Arc::new(hodei_pipelines_modules::scheduler::SchedulerModule::new(
        unified_repo,
        event_bus.clone(),
        worker_client,
        worker_repo.clone(),
        scheduler_config,
    ));

    // Start Scheduler Loop
    scheduler_module.start().await.map_err(|e| {
        error!("❌ Failed to start Scheduler Module: {}", e);
        BootstrapError::General(anyhow::anyhow!("Failed to start Scheduler Module: {}", e))
    })?;

    let scheduler: Arc<dyn SchedulerPort + Send + Sync> = scheduler_module;
    info!("✅ Scheduler initialized and started");

    // Initialize and Start Log Persistence Service
    info!("💾 Initializing Log Persistence Service...");
    let log_persistence = hodei_pipelines_modules::log_persistence::LogPersistenceService::new(
        Arc::new(execution_repo.clone()),
        event_bus.clone(),
        config.logging.retention_limit,
    );

    log_persistence.start().await.map_err(|e| {
        error!("❌ Failed to start Log Persistence Service: {}", e);
        BootstrapError::General(anyhow::anyhow!(
            "Failed to start Log Persistence Service: {}",
            e
        ))
    })?;
    info!("✅ Log Persistence Service initialized and started");

    info!("✨ Server bootstrap completed successfully");
    info!("📊 Status: ready");
    info!(
        "🌐 Ready to accept connections on {}:{}",
        config.server.host, config.server.port
    );

    Ok(ServerComponents {
        config,
        event_subscriber,
        event_publisher,
        orchestrator: orchestrator_trait,
        pipeline_service,
        role_repository,
        permission_repository,
        worker_repository: worker_repo,
        metrics_repository,
        metrics_persistence_service,
        concrete_orchestrator,
        scheduler,
        status: "ready",
    })
}

pub fn log_config_summary(config: &AppConfig) {
    info!("📋 Configuration Summary:");
    info!(
        "   Database: {} (max_conn: {})",
        mask_url(&config.database.url),
        config.database.max_connections
    );
    info!("   Server: {}:{}", config.server.host, config.server.port);
}

fn mask_url(url: &str) -> String {
    if let Some(pos) = url.find("://") {
        let (protocol, rest) = url.split_at(pos + 3);
        if let Some(at_pos) = rest.find('@') {
            let (creds, host) = rest.split_at(at_pos);
            if let Some(colon_pos) = creds.find(':') {
                let (user, _) = creds.split_at(colon_pos);
                return format!("{}****:****@{}", protocol, user);
            }
            return format!("{}****@{}", protocol, host);
        }
    }
    url.to_string()
}
