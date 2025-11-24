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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_repository_trait_exists() {
        // This test verifies the trait exists and compiles
        let _repo: Option<Box<dyn WorkerRepository + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[test]
    fn test_worker_repository_error_constructors() {
        // Test error constructors
        let _not_found = WorkerRepositoryError::Database("error".to_string());
        let _database = WorkerRepositoryError::Database("error".to_string());
        let _serialization = WorkerRepositoryError::Serialization("error".to_string());
        let _validation = WorkerRepositoryError::Validation("error".to_string());
    }

    #[test]
    fn test_worker_repository_error_display() {
        let database = WorkerRepositoryError::Database("Connection error".to_string());
        let serialization = WorkerRepositoryError::Serialization("Serialize failed".to_string());
        let validation = WorkerRepositoryError::Validation("Invalid data".to_string());
        
        assert!(database.to_string().contains("Database error"));
        assert!(serialization.to_string().contains("Serialization error"));
        assert!(validation.to_string().contains("Validation error"));
    }
}
