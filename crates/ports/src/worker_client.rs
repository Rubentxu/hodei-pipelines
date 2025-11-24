//! Worker Client Port
//!
//! Defines the interface for communicating with workers.

use async_trait::async_trait;
use hodei_shared_types::{JobId, WorkerId, WorkerStatus};
use serde::{Deserialize, Serialize};

/// Worker client port for communicating with worker agents
#[async_trait]
pub trait WorkerClient: Send + Sync {
    /// Assign a job to a worker
    async fn assign_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
        job_spec: &hodei_core::JobSpec,
    ) -> Result<(), WorkerClientError>;

    /// Cancel a running job
    async fn cancel_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<(), WorkerClientError>;

    /// Get worker status
    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<WorkerStatus, WorkerClientError>;

    /// Send heartbeat to worker
    async fn send_heartbeat(&self, worker_id: &WorkerId) -> Result<(), WorkerClientError>;
}

/// Worker client error
#[derive(thiserror::Error, Debug)]
pub enum WorkerClientError {
    #[error("Worker not found: {0}")]
    NotFound(WorkerId),

    #[error("Worker not available")]
    NotAvailable,

    #[error("Communication error: {0}")]
    Communication(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Connection error: {0}")]
    Connection(String),
}
