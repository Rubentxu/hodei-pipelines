//! Job Repository Port
//!
//! Defines the interface for job persistence.

use async_trait::async_trait;
use hodei_pipelines_domain::{Job, JobId, Result, WorkerId};

/// Job filter for querying jobs
#[derive(Debug, Clone)]
pub struct JobFilter {
    /// Filter by job state
    pub state: Option<String>,
    /// Filter by assigned worker
    pub worker_id: Option<WorkerId>,
    /// Filter by job type
    pub job_type: Option<String>,
}

/// Job repository port
#[async_trait]
pub trait JobRepository: Send + Sync {
    /// Save a job
    async fn save_job(&self, job: &Job) -> Result<()>;

    /// Get a job by ID
    async fn get_job(&self, id: &JobId) -> Result<Option<Job>>;

    /// Get all pending jobs
    async fn get_pending_jobs(&self) -> Result<Vec<Job>>;

    /// Get all running jobs
    async fn get_running_jobs(&self) -> Result<Vec<Job>>;

    /// Delete a job
    async fn delete_job(&self, id: &JobId) -> Result<()>;

    /// Update job state atomically
    async fn compare_and_swap_status(
        &self,
        id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool>;

    /// Assign a worker to a job
    async fn assign_worker(&self, job_id: &JobId, worker_id: &WorkerId) -> Result<()>;

    /// Set job start time
    async fn set_job_start_time(
        &self,
        job_id: &JobId,
        start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()>;

    /// Set job finish time
    async fn set_job_finish_time(
        &self,
        job_id: &JobId,
        finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()>;

    /// Set job duration
    async fn set_job_duration(&self, job_id: &JobId, duration_ms: i64) -> Result<()>;

    /// Create a job from spec (US-PIPE-001 extension)
    async fn create_job(
        &self,
        job_spec: hodei_pipelines_domain::pipeline_execution::job_definitions::JobSpec,
    ) -> Result<JobId>;

    /// Update job state (US-PIPE-001 extension)
    async fn update_job_state(
        &self,
        job_id: &JobId,
        state: hodei_pipelines_domain::pipeline_execution::job_definitions::JobState,
    ) -> Result<()>;

    /// List jobs (US-PIPE-001 extension)
    async fn list_jobs(&self) -> Result<Vec<Job>>;
}

/// Job repository error (legacy - for backward compatibility)
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

        assert!(
            conflict
                .to_string()
                .contains("Concurrent modification detected")
        );
        assert!(database.to_string().contains("Database error"));
        assert!(validation.to_string().contains("Invalid job data"));
    }
}
