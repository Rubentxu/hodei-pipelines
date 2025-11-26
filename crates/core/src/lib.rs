//! Domain Core - Business Logic and Shared Types
//!
//! This crate contains domain entities, value objects, business logic,
//! and all shared types consolidated to avoid duplication.

pub mod circuit_breaker;
pub mod correlation;
pub mod domain_services;
pub mod error;
pub mod events;
pub mod health_checks;
pub mod job;
pub mod job_definitions;
pub mod job_specifications;
pub mod mappers;
pub mod pipeline;
pub mod projections;
pub mod security;
pub mod specifications;
pub mod worker;
pub mod worker_messages;

pub use crate::error::DomainError;
pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::{Uuid, uuid};

// Re-export all types for easy importing
pub use crate::correlation::{CorrelationId, TraceContext};
pub use crate::health_checks::{HealthCheck, HealthStatus};
pub use crate::job::Job;
pub use crate::job_definitions::{ExecResult, JobId, JobSpec, JobState, ResourceQuota};
pub use crate::pipeline::{Pipeline, PipelineId, PipelineStatus};
pub use crate::worker::Worker;
pub use crate::worker_messages::{
    RuntimeSpec, WorkerCapabilities, WorkerId, WorkerMessage, WorkerState, WorkerStateMessage,
    WorkerStatus,
};

/// Tenant identifier for multi-tenancy support
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(String);

impl TenantId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TenantId {
    fn from(s: String) -> Self {
        TenantId::new(s)
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Domain result type
pub type Result<T> = std::result::Result<T, DomainError>;
