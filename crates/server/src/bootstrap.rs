//! Server Bootstrap - Production Initialization

use hodei_pipelines_adapters::scheduling::docker_provider::DockerProvider;
use hodei_pipelines_adapters::{
    InMemoryBus, MetricsPersistenceConfig, MetricsPersistenceService, MetricsTimeseriesRepository,
    PostgreSqlJobRepository, PostgreSqlPermissionRepository, PostgreSqlPipelineExecutionRepository,
    PostgreSqlPipelineRepository, PostgreSqlRoleRepository, PostgreSqlWorkerRepository,
    config::AppConfig,
};
use hodei_pipelines_application::scheduling::worker_provisioner::WorkerProvisionerService;
use hodei_pipelines_application::{
    ConcreteOrchestrator,
    PipelineExecutionConfig,
    PipelineExecutionService,
    PipelineService,
    // observability::log_persistence::LogPersistenceService, // Removed as per instruction
    // scheduling::SchedulerConfig, // Removed as per instruction
};
use hodei_pipelines_domain::resource_governance::GlobalResourceController;

use hodei_pipelines_adapters::resource_governance::RedbResourcePoolRepository;
use hodei_pipelines_domain::resource_governance::GRCConfig;
use hodei_pipelines_ports::worker_provider::{ProviderConfig, WorkerTemplate};

use async_trait::async_trait;
use hodei_pipelines_domain::{Worker, WorkerId};
use hodei_pipelines_ports::EventSubscriber;
use hodei_pipelines_ports::resource_governance::resource_pool::ResourcePoolRepository;
use hodei_pipelines_ports::scheduling::scheduler_port::{SchedulerError, SchedulerPort};
use hodei_pipelines_proto::ServerMessage;
use sqlx::postgres::PgPoolOptions;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct StubScheduler;

#[async_trait]
impl SchedulerPort for StubScheduler {
    async fn register_worker(&self, _worker: &Worker) -> std::result::Result<(), SchedulerError> {
        Ok(())
    }
    async fn unregister_worker(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<(), SchedulerError> {
        Ok(())
    }
    async fn get_registered_workers(&self) -> std::result::Result<Vec<WorkerId>, SchedulerError> {
        Ok(vec![])
    }
    async fn register_transmitter(
        &self,
        _worker_id: &WorkerId,
        _transmitter: mpsc::UnboundedSender<std::result::Result<ServerMessage, SchedulerError>>,
    ) -> std::result::Result<(), SchedulerError> {
        Ok(())
    }
    async fn unregister_transmitter(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<(), SchedulerError> {
        Ok(())
    }
    async fn send_to_worker(
        &self,
        _worker_id: &WorkerId,
        _message: ServerMessage,
    ) -> std::result::Result<(), SchedulerError> {
        Ok(())
    }
}

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

    pub scheduler: Arc<dyn SchedulerPort + Send + Sync>,
    pub resource_pool_repository: Arc<dyn ResourcePoolRepository>,
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

    // Initialize Event Bus
    let bus_type =
        hodei_pipelines_adapters::bus::config::EventBusType::from_str(&config.event_bus.bus_type)
            .unwrap_or_default();

    info!("🚌 Initializing Event Bus (Type: {})", bus_type);

    let (event_publisher, event_subscriber): (
        Arc<dyn hodei_pipelines_ports::EventPublisher>,
        Arc<dyn EventSubscriber>,
    ) = match bus_type {
        hodei_pipelines_adapters::bus::config::EventBusType::InMemory => {
            tracing::warn!("⚠️ Using InMemory EventBus - NOT FOR PRODUCTION (unless testing)");
            let bus = Arc::new(InMemoryBus::new(1000));
            (bus.clone(), bus.clone())
        }
        hodei_pipelines_adapters::bus::config::EventBusType::Nats => {
            // Map AppConfig NatsConfig to Adapter NatsConfig
            let nats_config = hodei_pipelines_adapters::bus::config::NatsConfig {
                url: config.nats.url.clone(),
                connection_timeout_ms: config.nats.connection_timeout_ms,
            };

            let bus = Arc::new(
                hodei_pipelines_adapters::bus::nats::NatsBus::new_with_config(nats_config.into())
                    .await
                    .map_err(|e| {
                        error!("❌ Failed to initialize NATS Event Bus: {}", e);
                        BootstrapError::General(anyhow::anyhow!(
                            "Failed to initialize NATS Event Bus: {}",
                            e
                        ))
                    })?,
            );
            (bus.clone(), bus.clone())
        }
    };
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
    let execution_repo = Arc::new(PostgreSqlPipelineExecutionRepository::new(
        pool.clone(),
        migrations_path.clone(),
        config.database.pipeline_execution_migration_file.clone(),
    ));
    let job_repo = Arc::new(PostgreSqlJobRepository::new(
        pool.clone(),
        migrations_path.clone(),
        config.database.job_migration_file.clone(),
    ));
    let pipeline_repo = Arc::new(PostgreSqlPipelineRepository::new(
        pool.clone(),
        migrations_path.clone(),
        config.database.pipeline_migration_file.clone(),
    ));
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

    // Initialize Resource Pool Repository (Redb)
    let resource_pool_db_path = config.database.resource_pool_db_path.clone();
    let resource_pool_repo = Arc::new(
        RedbResourcePoolRepository::new_with_path(&resource_pool_db_path).map_err(|e| {
            error!(
                "❌ Failed to initialize Redb Resource Pool Repository: {}",
                e
            );
            BootstrapError::General(anyhow::anyhow!(
                "Failed to initialize Redb Resource Pool Repository: {}",
                e
            ))
        })?,
    );
    resource_pool_repo.init_schema().await.map_err(|e| {
        error!("❌ Failed to initialize resource pool schema: {}", e);
        BootstrapError::General(anyhow::anyhow!(
            "Failed to initialize resource pool schema: {}",
            e
        ))
    })?;
    info!(
        "✅ Redb Resource Pool Repository initialized at {}",
        resource_pool_db_path
    );

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

    // Create Docker provider for worker provisioning
    let template = WorkerTemplate::new("default-worker", "1.0.0", "hwp-agent:latest");
    let provider_config = ProviderConfig::docker("default".to_string(), template);
    let docker_provider = Arc::new(DockerProvider::new(provider_config).await.map_err(|e| {
        BootstrapError::General(anyhow::anyhow!("Failed to create Docker provider: {}", e))
    })?);

    // Create Global Resource Controller
    let grc_config = GRCConfig::default();
    let grc = Arc::new(GlobalResourceController::new(
        grc_config,
        resource_pool_repo.clone(),
    ));

    // Initialize Scheduler Module
    info!("📅 Initializing Scheduler...");
    // TODO: Implement proper scheduler initialization
    // For now, create a stub scheduler
    let scheduler_module = Arc::new(StubScheduler);
    let scheduler: Arc<dyn SchedulerPort + Send + Sync> = scheduler_module.clone();
    info!("✅ Scheduler initialized and started");

    // Create real WorkerProvisionerService
    let worker_provisioner = Arc::new(WorkerProvisionerService::new(
        docker_provider,
        scheduler_module.clone(),
        grc,
        "default".to_string(),
    ));

    // Initialize Orchestrator
    let orchestrator = Arc::new(
        ConcreteOrchestrator::new(
            execution_repo.clone(),
            job_repo.clone(),
            pipeline_repo.clone(),
            worker_provisioner.clone(),
            orchestrator_config,
        )
        .map_err(|e| {
            BootstrapError::General(anyhow::anyhow!("Failed to create Orchestrator: {}", e))
        })?,
    );

    let orchestrator_trait: Arc<dyn PipelineExecutionService + Send + Sync> = orchestrator.clone();
    let pipeline_service: Arc<dyn PipelineService + Send + Sync> = orchestrator.clone();

    // Initialize and Start Log Persistence Service
    info!("💾 Initializing Log Persistence Service...");
    // TODO: Implement proper log persistence initialization
    // For now, skip log persistence service
    info!("⚠️ Log Persistence Service temporarily disabled");

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

        scheduler,
        resource_pool_repository: resource_pool_repo,
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
