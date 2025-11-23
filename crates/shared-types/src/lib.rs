//! Shared types and definitions for the Hodei CI/CD distributed system
//!
//! This crate contains common types, IDs, value objects, and error types
//! used across all bounded contexts in the system.

pub mod correlation;
pub mod error;
pub mod health_checks;
pub mod job_definitions;
pub mod worker_messages;

pub use crate::error::DomainError;
pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::{Uuid, uuid};

// Re-export all types for easy importing
pub use crate::correlation::{CorrelationId, TraceContext};
pub use crate::health_checks::{HealthCheck, HealthStatus};
pub use crate::job_definitions::{ExecResult, JobId, JobSpec, JobState, ResourceQuota};
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
