//! Adapters - Infrastructure Implementations
//!
//! This crate contains the implementations of the ports defined in hodei-ports.

pub mod bus;
pub mod event_bus;
pub mod postgres;
pub mod redb;
pub mod repositories;
pub mod worker_client;

pub use crate::bus::{InMemoryBus, InMemoryBusBuilder};
pub use crate::repositories::{
    InMemoryJobRepository, InMemoryPipelineRepository, InMemoryWorkerRepository,
};
pub use crate::worker_client::MockWorkerClient;

// PostgreSQL implementations
pub use crate::postgres::{
    PostgreSqlJobRepository, PostgreSqlPipelineRepository, PostgreSqlWorkerRepository,
};

// Redb (embedded) implementations
pub use crate::redb::{RedbJobRepository, RedbPipelineRepository, RedbWorkerRepository};
