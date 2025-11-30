//! Adapters - Infrastructure Implementations
//!
//! This crate contains the implementations of the ports defined in hodei-pipelines-ports.

pub mod bus;
pub mod cached_repository;
pub mod config;
pub mod docker_provider;
pub mod event_bus;
pub mod extractors;
pub mod kubernetes_provider;
pub mod kubernetes_provider_tests;
pub mod metrics;
pub mod metrics_persistence_service;
pub mod postgres;
pub mod metrics_timeseries_repository;
pub mod postgresql_rbac_repositories;
pub mod provider_factory;
pub mod rbac_repositories;
pub mod redb;
pub mod security;
pub mod websocket;
pub mod worker_client;
pub mod worker_registration;

pub use crate::bus::config::{EventBusConfig, EventBusType, NatsConfig};
pub use crate::bus::{EventBusFactory, InMemoryBus, InMemoryBusBuilder};
pub use crate::cached_repository::{CacheStats, CachedJobRepository};
pub use crate::docker_provider::DockerProvider;
pub use crate::kubernetes_provider::KubernetesProvider;
pub use crate::provider_factory::DefaultProviderFactory;
pub use crate::websocket::websocket_handler;
pub use crate::worker_client::{GrpcWorkerClient, HttpWorkerClient};
pub use crate::worker_registration::{RegistrationConfig, WorkerRegistrationAdapter};

// Re-export types from hodei-pipelines-ports
pub use hodei_pipelines_ports::worker_provider::{ProviderConfig, ProviderType};

// PostgreSQL implementations
pub use crate::postgres::{
    PostgreSqlJobRepository, PostgreSqlPipelineExecutionRepository, PostgreSqlPipelineRepository,
    PostgreSqlWorkerRepository, functional_tests,
};

// RBAC repositories
pub use crate::postgresql_rbac_repositories::{
    PostgreSqlPermissionRepository, PostgreSqlRoleRepository,
};
pub use crate::rbac_repositories::{InMemoryPermissionRepository, InMemoryRoleRepository};

// Metrics TSDB repository
pub use crate::metrics_timeseries_repository::MetricsTimeseriesRepository;
pub use crate::metrics_persistence_service::{MetricsPersistenceConfig, MetricsPersistenceService};
