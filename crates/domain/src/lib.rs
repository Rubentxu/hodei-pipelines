//! Domain Core - Business Logic and Shared Types
//!
//! This crate contains domain entities, value objects, business logic,
//! and all shared types consolidated to avoid duplication.

pub mod identity_access;
pub mod observability;
pub mod pipeline_execution;
pub mod resource_governance;
pub mod scheduling;

// Shared kernel (cross-cutting concerns)
pub mod shared_kernel {
    pub mod circuit_breaker;
    pub mod correlation;
    pub mod error;
    pub mod events;
    pub mod mappers;
    pub mod projections;
    pub mod specifications;
}

pub use crate::shared_kernel::error::{
    DatabaseError, DomainError, EventBusError, JobError, PipelineError, RepositoryError,
    WorkerError,
};
pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::{Uuid, uuid};

// Re-export all types for easy importing
pub use crate::observability::value_objects::health_checks::{HealthCheck, HealthStatus};
pub use crate::pipeline_execution::entities::execution::{
    ExecutionId, ExecutionStatus, PipelineExecution, StepExecution, StepExecutionId,
    StepExecutionStatus,
};
pub use crate::pipeline_execution::entities::job::Job;
pub use crate::pipeline_execution::entities::pipeline::{Pipeline, PipelineId, PipelineStatus};
pub use crate::pipeline_execution::value_objects::job_definitions::{
    ExecResult, JobId, JobSpec, JobState, ResourceQuota,
};
pub use crate::resource_governance::domain_services::controller::{
    AllocationResult, GRCConfig, GRCMetrics, GlobalResourceController, PoolSelectionResult,
};
pub use crate::resource_governance::entities::domain::{
    ComputePool, CostConfig, PoolCapacity, PoolId, PoolStatus, ProviderType, RequestId,
    ResourcePoolConfig, ResourcePoolStatus, ResourceRequest, TenantQuota,
};
pub use crate::scheduling::entities::worker::Worker;
pub use crate::scheduling::value_objects::worker_messages::{
    RuntimeSpec, WorkerCapabilities, WorkerId, WorkerMessage, WorkerState, WorkerStateMessage,
    WorkerStatus,
};
pub use crate::shared_kernel::correlation::{CorrelationId, TraceContext};

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage_m: u64,
    pub memory_usage_mb: u64,
    pub active_jobs: u32,
    pub disk_read_mb: f64,
    pub disk_write_mb: f64,
    pub network_sent_mb: f64,
    pub network_received_mb: f64,
    pub gpu_utilization_percent: f64,
    pub timestamp: i64,
}

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
