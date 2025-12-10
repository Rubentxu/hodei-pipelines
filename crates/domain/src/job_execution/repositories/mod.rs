//! Job Repository Port
//!
//! Repository interface for persisting and retrieving jobs

use super::entities::Job;
use crate::shared_kernel::DomainResult;

/// Repository port for Job aggregate
#[async_trait::async_trait]
pub trait JobRepository: Send + Sync {
    /// Saves a job to the repository
    async fn save(&self, job: &Job) -> DomainResult<()>;

    /// Finds a job by its ID
    async fn find_by_id(&self, id: &crate::shared_kernel::JobId) -> DomainResult<Option<Job>>;

    /// Lists all jobs
    async fn list(&self) -> DomainResult<Vec<Job>>;

    /// Deletes a job
    async fn delete(&self, id: &crate::shared_kernel::JobId) -> DomainResult<()>;
}
