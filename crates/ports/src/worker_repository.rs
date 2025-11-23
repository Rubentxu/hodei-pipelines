//! Worker Repository Port

use async_trait::async_trait;
use hodei_core::{Worker, WorkerId};

#[async_trait]
pub trait WorkerRepository: Send + Sync {
    async fn save_worker(&self, worker: &Worker) -> Result<(), WorkerRepositoryError>;
    async fn get_worker(&self, id: &WorkerId) -> Result<Option<Worker>, WorkerRepositoryError>;
    async fn get_all_workers(&self) -> Result<Vec<Worker>, WorkerRepositoryError>;
    async fn delete_worker(&self, id: &WorkerId) -> Result<(), WorkerRepositoryError>;
}

#[derive(thiserror::Error, Debug)]
pub enum WorkerRepositoryError {
    #[error("Worker not found: {0}")]
    NotFound(WorkerId),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Validation error: {0}")]
    Validation(String),
}
