//! Domain Core - Pure Business Logic
//!
//! This crate contains the domain entities, value objects, and business logic
//! following Domain-Driven Design principles. It has NO dependencies on
//! infrastructure, ports, or external libraries.

pub mod error;
pub mod job;
pub mod pipeline;
pub mod worker;

pub use crate::error::{DomainError, Result};
pub use crate::job::{ExecResult, Job, JobId, JobSpec, JobState, ResourceQuota};
pub use crate::pipeline::{Pipeline, PipelineId, PipelineStatus};
pub use crate::worker::{Worker, WorkerCapabilities, WorkerId, WorkerStatus};

// Re-export commonly used types
pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::{Uuid, uuid};

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
