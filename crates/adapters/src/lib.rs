//! Adapters - Infrastructure Implementations
//!
//! This crate contains the implementations of the ports defined in hodei-ports.

pub mod bus;
pub mod docker_provider;
pub mod event_bus;
pub mod kubernetes_provider;
pub mod kubernetes_provider_tests;
pub mod postgres;
pub mod provider_factory;
pub mod redb;
pub mod repositories;
pub mod security;
pub mod worker_client;
pub mod worker_registration;

pub use crate::bus::{InMemoryBus, InMemoryBusBuilder};
pub use crate::docker_provider::DockerProvider;
pub use crate::kubernetes_provider::KubernetesProvider;
pub use crate::provider_factory::DefaultProviderFactory;
pub use crate::repositories::{
    InMemoryJobRepository, InMemoryPipelineRepository, InMemoryWorkerRepository,
};
pub use crate::worker_client::{GrpcWorkerClient, HttpWorkerClient};
pub use crate::worker_registration::{RegistrationConfig, WorkerRegistrationAdapter};

// Re-export types from hodei-ports
pub use hodei_ports::worker_provider::{ProviderConfig, ProviderType};

// PostgreSQL implementations
pub use crate::postgres::{
    PostgreSqlJobRepository, PostgreSqlPipelineRepository, PostgreSqlWorkerRepository,
};

// Redb (embedded) implementations
pub use crate::redb::{RedbJobRepository, RedbPipelineRepository, RedbWorkerRepository};
