//! Shared Kernel - Common types shared across bounded contexts
//!
//! This module contains:
//! - Error types and DomainResult
//! - Value objects shared by all bounded contexts
//! - Core types (JobId, ProviderId, etc.)

pub mod error;
pub mod traits;
pub mod types;

pub use error::{DomainError, DomainResult};
pub use traits::{ProviderWorker, ProviderWorkerBuilder};
pub use types::*;
