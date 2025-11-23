//! Job Repository Port
//!
//! Defines the interface for job persistence.

use async_trait::async_trait;
use hodei_core::{Job, JobId};

/// Job repository port
#[async_trait]
pub trait JobRepository: Send + Sync {
    /// Save a job
    async fn save_job(&self, job: &Job) -> Result<(), JobRepositoryError>;

    /// Get a job by ID
    async fn get_job(&self, id: &JobId) -> Result<Option<Job>, JobRepositoryError>;

    /// Get all pending jobs
    async fn get_pending_jobs(&self) -> Result<Vec<Job>, JobRepositoryError>;

    /// Get all running jobs
    async fn get_running_jobs(&self) -> Result<Vec<Job>, JobRepositoryError>;

    /// Delete a job
    async fn delete_job(&self, id: &JobId) -> Result<(), JobRepositoryError>;

    /// Update job state atomically
    async fn compare_and_swap_status(
        &self,
        id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool, JobRepositoryError>;
}

/// Job repository error
#[derive(thiserror::Error, Debug)]
pub enum JobRepositoryError {
    #[error("Job not found: {0}")]
    NotFound(JobId),

    #[error("Concurrent modification detected")]
    Conflict,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid job data: {0}")]
    Validation(String),
}
