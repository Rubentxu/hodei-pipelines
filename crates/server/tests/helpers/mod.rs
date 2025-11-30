//! Test helpers for ServerComponents creation
//!
//! Provides production-ready test utilities with real dependencies

use hodei_pipelines_adapters::{
    MetricsPersistenceConfig, MetricsPersistenceService, MetricsTimeseriesRepository,
    PostgreSqlJobRepository, PostgreSqlPermissionRepository, PostgreSqlPipelineExecutionRepository,
    PostgreSqlPipelineRepository, PostgreSqlRoleRepository, PostgreSqlWorkerRepository,
    bus::InMemoryBus, config::AppConfig,
};
use hodei_pipelines_modules::{ConcreteOrchestrator, PipelineExecutionConfig};
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
