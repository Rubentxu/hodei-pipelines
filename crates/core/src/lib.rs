//! Domain Core - Pure Business Logic
//!
//! This crate contains the domain entities, value objects, and business logic
//! following Domain-Driven Design principles.
//!
//! # Shared Kernel Pattern
//! Value objects and common types are defined in the Shared Kernel
//! (hodei-shared-types crate) to avoid duplication across bounded contexts.

pub mod job;
pub mod pipeline;
pub mod security;
pub mod worker;

// Re-exports from Shared Kernel (shared-types crate)
pub use hodei_shared_types::{
    CorrelationId, DomainError, ExecResult, HealthCheck, JobId, JobSpec, JobState, ResourceQuota,
    RuntimeSpec, TenantId, TraceContext, WorkerId, WorkerMessage, WorkerState, WorkerStateMessage,
};

// Re-export domain types
pub use crate::job::Job;
pub use crate::pipeline::{Pipeline, PipelineId, PipelineStatus};
pub use crate::worker::Worker;

// Re-export commonly used types from Shared Kernel
pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::{Uuid, uuid};

// Domain result type
pub type Result<T> = std::result::Result<T, DomainError>;
