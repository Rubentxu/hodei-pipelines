//! Worker Client Port
//!
//! Defines the interface for communicating with workers.

use async_trait::async_trait;
use hodei_core::JobSpec;
use hodei_core::{JobId, WorkerId, WorkerStatus};

/// Worker client port for communicating with worker agents
#[async_trait]
pub trait WorkerClient: Send + Sync {
    /// Assign a job to a worker
    async fn assign_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
        job_spec: &JobSpec,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_client_trait_exists() {
        // This test verifies the trait exists and compiles
        let _client: Option<Box<dyn WorkerClient + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[test]
    fn test_worker_client_error_constructors() {
        // Test error constructors
        let _not_available = WorkerClientError::NotAvailable;
        let _communication = WorkerClientError::Communication("error".to_string());
        let _timeout = WorkerClientError::Timeout("error".to_string());
        let _configuration = WorkerClientError::Configuration("error".to_string());
        let _connection = WorkerClientError::Connection("error".to_string());
    }

    #[test]
    fn test_worker_client_error_display() {
        let not_available = WorkerClientError::NotAvailable;
        let communication = WorkerClientError::Communication("Network error".to_string());
        let timeout = WorkerClientError::Timeout("Request timed out".to_string());

        assert!(not_available.to_string().contains("Worker not available"));
        assert!(communication.to_string().contains("Communication error"));
        assert!(timeout.to_string().contains("Timeout"));
    }
}
