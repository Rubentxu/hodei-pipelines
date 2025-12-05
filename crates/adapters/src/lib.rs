//! Adapters - Infrastructure Implementations
//!
//! This crate contains the implementations of the ports defined in hodei-pipelines-ports.

pub mod bus;
pub mod cached_repository;
pub mod config;
pub mod event_bus;
pub mod extractors;
pub mod identity_access;
pub mod observability;
pub mod orchestrator;
pub mod pipeline_execution;
pub mod postgres;
pub mod redb;
pub mod resource_governance;
pub mod scheduling;
pub mod security;
pub mod websocket;

pub use crate::bus::config::{EventBusConfig, EventBusType, NatsConfig};
pub use crate::bus::{EventBusFactory, InMemoryBus, InMemoryBusBuilder};
pub use crate::orchestrator::FacadeOrchestratorAdapter;
pub use crate::redb::RedbJobRepository;
pub use crate::resource_governance::RedbResourcePoolRepository;
pub use crate::scheduling::docker_provider::DockerProvider;
pub use crate::scheduling::kubernetes_provider::KubernetesProvider;
pub use crate::scheduling::provider_factory::DefaultProviderFactory;
pub use crate::scheduling::worker_client::{GrpcWorkerClient, HttpWorkerClient};
pub use crate::scheduling::worker_registration::{RegistrationConfig, WorkerRegistrationAdapter};
pub use crate::websocket::websocket_handler;

// Re-export types from hodei-pipelines-ports
pub use hodei_pipelines_ports::scheduling::worker_provider::{ProviderConfig, ProviderType};

// PostgreSQL implementations
pub use crate::postgres::{
    PostgreSqlJobRepository, PostgreSqlPipelineExecutionRepository, PostgreSqlPipelineRepository,
    PostgreSqlWorkerRepository, functional_tests,
};

// RBAC repositories
// RBAC repositories
pub use crate::identity_access::postgresql_rbac_repositories::{
    PostgreSqlPermissionRepository, PostgreSqlRoleRepository,
};
pub use crate::identity_access::rbac_repositories::{
    InMemoryPermissionRepository, InMemoryRoleRepository,
};

// Metrics TSDB repository
// Metrics TSDB repository
pub use crate::observability::metrics_persistence_service::{
    MetricsPersistenceConfig, MetricsPersistenceService,
};
pub use crate::observability::metrics_timeseries_repository::MetricsTimeseriesRepository;
