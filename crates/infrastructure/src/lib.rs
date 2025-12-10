//! Infrastructure Layer
//!
//! Contains adapters and repositories for external integrations

pub mod adapters;
pub mod repositories;

// Re-exports
pub use repositories::{InMemoryJobRepository, InMemoryProviderRepository};
