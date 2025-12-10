//! Infrastructure Layer
//!
//! Contains adapters and repositories for external integrations

pub mod adapters;
pub mod database;
pub mod event_bus;
pub mod pooling;
pub mod repositories;

// Re-exports
pub use database::{DatabaseConfig, DatabasePool, PostgresJobRepository, PostgresProviderRepository};
pub use event_bus::{InMemoryEventStore, NatsConfig, NatsEventPublisher};
pub use pooling::{PgBouncerConfig, PgBouncerPool, PoolStats};
pub use repositories::{InMemoryJobRepository, InMemoryProviderRepository};
