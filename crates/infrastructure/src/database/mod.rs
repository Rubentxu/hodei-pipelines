//! Database module
//!
//! PostgreSQL repository implementations

pub mod postgres;

// Re-exports
pub use postgres::{
    DatabaseConfig, DatabasePool, PostgresJobRepository, PostgresProviderRepository,
};
