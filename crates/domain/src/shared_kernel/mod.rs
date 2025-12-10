//! Shared Kernel - Common types shared across bounded contexts
//!
//! This module contains:
//! - Error types and DomainResult
//! - Value objects shared by all bounded contexts
//! - Core types (JobId, ProviderId, etc.)
//! - Domain events for inter-component communication

pub mod error;
pub mod events;
pub mod traits;
pub mod types;

pub use error::{DomainError, DomainResult};
pub use events::{DomainEvent, EventError, EventPublisher, EventResult, EventStore, EventSubscriber};
pub use traits::{ProviderWorker, ProviderWorkerBuilder};
pub use types::*;
