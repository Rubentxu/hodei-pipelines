//! Test helpers

use hodei_pipelines_adapters::{
    bus::InMemoryBus, config::AppConfig, PostgreSqlJobRepository,
    PostgreSqlPermissionRepository, PostgreSqlPipelineExecutionRepository,
    PostgreSqlPipelineRepository, PostgreSqlRoleRepository, PostgreSqlWorkerRepository,
    MetricsPersistenceConfig, MetricsPersistenceService, MetricsTimeseriesRepository,
};
use hodei_pipelines_modules::{ConcreteOrchestrator, PipelineExecutionConfig};
use hodei_server::bootstrap::ServerComponents;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

/// Create a complete ServerComponents instance for testing
pub fn create_test_server_components() -> ServerComponents {
    // Create a dummy PostgreSQL connection pool for tests
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
        .unwrap_or_else(|_| PgPoolOptions::new().max_connections(5).connect_lazy("sqlite::memory:").unwrap());

    let event_bus = Arc::new(InMemoryBus::new(100));

    let orchestrator = Arc::new(
        ConcreteOrchestrator::new(
            Arc::new(PostgreSqlPipelineExecutionRepository::new(pool.clone())),
            Arc::new(PostgreSqlJobRepository::new(pool.clone())),
            Arc::new(PostgreSqlPipelineRepository::new(pool.clone())),
            event_bus.clone(),
            PipelineExecutionConfig::default(),
        )
        .expect("Failed to create test orchestrator"),
    );

    let metrics_repository = Arc::new(MetricsTimeseriesRepository::new(pool.clone()));
    let metrics_persistence_service = Arc::new(MetricsPersistenceService::new(
        MetricsPersistenceConfig::default(),
        metrics_repository.clone(),
    ));

    ServerComponents {
        config: AppConfig::default(),
        event_subscriber: event_bus.clone(),
        event_publisher: event_bus,
        orchestrator: orchestrator.clone(),
        pipeline_service: orchestrator,
        role_repository: Arc::new(PostgreSqlRoleRepository::new(pool.clone())),
        permission_repository: Arc::new(PostgreSqlPermissionRepository::new(pool.clone())),
        worker_repository: Arc::new(PostgreSqlWorkerRepository::new(pool.clone())),
        metrics_repository,
        metrics_persistence_service,
        concrete_orchestrator: Arc::new(
            ConcreteOrchestrator::new(
                Arc::new(PostgreSqlPipelineExecutionRepository::new(pool.clone())),
                Arc::new(PostgreSqlJobRepository::new(pool.clone())),
                Arc::new(PostgreSqlPipelineRepository::new(pool.clone())),
                Arc::new(InMemoryBus::new(100)),
                PipelineExecutionConfig::default(),
            )
            .expect("Failed to create test concrete orchestrator"),
        ),
        scheduler: Arc::new(PostgreSqlWorkerRepository::new(pool)),
        status: "test",
    }
}
