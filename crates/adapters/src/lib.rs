//! Adapters - Infrastructure Implementations
//!
//! This crate contains the implementations of the ports defined in hodei-ports.

pub mod bus;
pub mod cached_repository;
pub mod docker_provider;
pub mod event_bus;
pub mod extractors;
pub mod kubernetes_provider;
pub mod kubernetes_provider_tests;
pub mod postgres;
pub mod provider_factory;
pub mod rbac_repositories;
pub mod redb;
pub mod security;
pub mod worker_client;
pub mod worker_registration;

pub use crate::bus::config::{EventBusConfig, EventBusType, NatsConfig};
pub use crate::bus::{EventBusFactory, InMemoryBus, InMemoryBusBuilder};
pub use crate::cached_repository::{CacheStats, CachedJobRepository};
pub use crate::docker_provider::DockerProvider;
pub use crate::kubernetes_provider::KubernetesProvider;
pub use crate::provider_factory::DefaultProviderFactory;
pub use crate::worker_client::{GrpcWorkerClient, HttpWorkerClient};
pub use crate::worker_registration::{RegistrationConfig, WorkerRegistrationAdapter};

// Re-export types from hodei-ports
pub use hodei_ports::worker_provider::{ProviderConfig, ProviderType};

// PostgreSQL implementations
pub use crate::postgres::{
    PostgreSqlJobRepository, PostgreSqlPipelineExecutionRepository, PostgreSqlPipelineRepository,
    PostgreSqlWorkerRepository, functional_tests,
};

// RBAC repositories
pub use crate::rbac_repositories::{InMemoryPermissionRepository, InMemoryRoleRepository};
