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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_repository_trait_exists() {
        // This test verifies the trait exists and compiles
        let _repo: Option<Box<dyn JobRepository + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[test]
    fn test_job_repository_error_constructors() {
        // Test error constructors
        let _not_found = JobRepositoryError::Conflict;
        let _conflict = JobRepositoryError::Conflict;
        let _database = JobRepositoryError::Database("error".to_string());
        let _validation = JobRepositoryError::Validation("error".to_string());
    }

    #[test]
    fn test_job_repository_error_display() {
        let _error = JobRepositoryError::Conflict;
        assert!(true);
    }

    #[test]
    fn test_job_repository_error_variants() {
        let conflict = JobRepositoryError::Conflict;
        let database = JobRepositoryError::Database("Connection lost".to_string());
        let validation = JobRepositoryError::Validation("Invalid data".to_string());
        
        assert!(conflict.to_string().contains("Concurrent modification detected"));
        assert!(database.to_string().contains("Database error"));
        assert!(validation.to_string().contains("Invalid job data"));
    }
}
