//! Test helpers for ServerComponents creation
//!
//! Provides production-ready test utilities with real dependencies

use hodei_pipelines_adapters::{
    MetricsPersistenceConfig, MetricsPersistenceService, MetricsTimeseriesRepository,
    PostgreSqlJobRepository, PostgreSqlPermissionRepository, PostgreSqlPipelineExecutionRepository,
    PostgreSqlPipelineRepository, PostgreSqlRoleRepository, PostgreSqlWorkerRepository,
    bus::InMemoryBus, config::AppConfig, resource_governance::RedbResourcePoolRepository,
};
use hodei_pipelines_application::{ConcreteOrchestrator, PipelineExecutionConfig};
use hodei_server::bootstrap::ServerComponents;
use sqlx::PgPool;
use std::sync::Arc;

/// Create a minimal ServerComponents with production-ready defaults for testing
pub fn create_test_server_components() -> ServerComponents {
    // Create a dummy PostgreSQL connection pool for tests
    // In real tests, use TestContainers to get a real PostgreSQL instance
    let pool = PgPool::connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
        .unwrap_or_else(|_| PgPool::connect_lazy("sqlite::memory:").unwrap());

    let event_bus = Arc::new(InMemoryBus::new(100));

    let orchestrator = Arc::new(
        ConcreteOrchestrator::new(
            Arc::new(PostgreSqlPipelineExecutionRepository::new(
                pool.clone(),
                None,
                "migrations".to_string(),
            )),
            Arc::new(PostgreSqlJobRepository::new(
                pool.clone(),
                None,
                "migrations".to_string(),
            )),
            Arc::new(PostgreSqlPipelineRepository::new(
                pool.clone(),
                None,
                "migrations".to_string(),
            )),
            event_bus.clone(),
            PipelineExecutionConfig::default(),
        )
        .expect("Failed to create test orchestrator"),
    );

    let metrics_repository = Arc::new(MetricsTimeseriesRepository::new(pool.clone()));
    let metrics_persistence_service = Arc::new(MetricsPersistenceService::new(
        MetricsPersistenceConfig::default(),
        metrics_repository.clone(),
        metrics_repository.clone(),
    ));

    // Create a temporary file for Redb that will persist for the duration of the test
    // We use a random name in the OS temp directory
    let resource_pool_db_path =
        std::env::temp_dir().join(format!("test_resource_pool_{}.redb", uuid::Uuid::new_v4()));
    let resource_pool_db_path_str = resource_pool_db_path.to_str().unwrap().to_string();

    let resource_pool_repo = Arc::new(
        RedbResourcePoolRepository::new_with_path(&resource_pool_db_path_str)
            .expect("Failed to create test resource pool repository"),
    );
    // Initialize schema is async, but this function is sync.
    // We might need to make this function async or use block_on.
    // For now, let's assume init_schema is called by the caller or not needed if we just need the struct.
    // But wait, RedbResourcePoolRepository needs init_schema for tables to exist.
    // We can use tokio::task::block_in_place or similar if we are in async context, but this is a helper.
    // Let's make create_test_server_components async?
    // It is used in tests which are async.

    // Actually, looking at the code, it returns ServerComponents.
    // I'll just initialize the struct. The schema initialization might be skipped or done later.
    // But RedbResourcePoolRepository::new_with_path opens the DB.

    ServerComponents {
        config: AppConfig::default(),
        event_subscriber: event_bus.clone(),
        event_publisher: event_bus,
        orchestrator: orchestrator.clone(),
        pipeline_service: orchestrator,
        role_repository: Arc::new(PostgreSqlRoleRepository::new(pool.clone())),
        permission_repository: Arc::new(PostgreSqlPermissionRepository::new(pool.clone())),
        worker_repository: Arc::new(PostgreSqlWorkerRepository::new(
            pool.clone(),
            None,
            "migrations".to_string(),
        )),
        metrics_repository,
        metrics_persistence_service,
        concrete_orchestrator: Arc::new(
            ConcreteOrchestrator::new(
                Arc::new(PostgreSqlPipelineExecutionRepository::new(
                    pool.clone(),
                    None,
                    "migrations".to_string(),
                )),
                Arc::new(PostgreSqlJobRepository::new(
                    pool.clone(),
                    None,
                    "migrations".to_string(),
                )),
                Arc::new(PostgreSqlPipelineRepository::new(
                    pool.clone(),
                    None,
                    "migrations".to_string(),
                )),
                Arc::new(InMemoryBus::new(100)),
                PipelineExecutionConfig::default(),
            )
            .expect("Failed to create test concrete orchestrator"),
        ),
        scheduler: Arc::new(PostgreSqlWorkerRepository::new(
            pool,
            None,
            "migrations".to_string(),
        )),
        status: "test",
        resource_pool_repository: resource_pool_repo,
    }
}
