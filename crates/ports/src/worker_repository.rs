//! Worker Repository Port

use async_trait::async_trait;
use hodei_pipelines_core::{Result, Worker, WorkerId};

#[async_trait]
pub trait WorkerRepository: Send + Sync {
    async fn save_worker(&self, worker: &Worker) -> Result<()>;
    async fn get_worker(&self, id: &WorkerId) -> Result<Option<Worker>>;
    async fn get_all_workers(&self) -> Result<Vec<Worker>>;
    async fn delete_worker(&self, id: &WorkerId) -> Result<()>;

    /// Update the last_seen timestamp for a worker (US-03.1: Heartbeat Processing)
    async fn update_last_seen(&self, id: &WorkerId) -> Result<()>;

    /// Find workers that haven't sent a heartbeat within the threshold duration
    async fn find_stale_workers(
        &self,
        threshold_duration: std::time::Duration,
    ) -> Result<Vec<Worker>>;

    /// Update worker status (US-PIPE-001 extension)
    async fn update_worker_status(
        &self,
        worker_id: &WorkerId,
        status: hodei_pipelines_core::WorkerStatus,
    ) -> Result<()>;
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

    #[tokio::test]
    async fn test_repository_traits_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn WorkerRepository + Send + Sync>>();
    }

    #[tokio::test]
    async fn test_update_last_seen_method_exists() {
        // Test that the trait has the update_last_seen method
        struct DummyRepo;
        #[async_trait::async_trait]
        impl WorkerRepository for DummyRepo {
            async fn save_worker(&self, _worker: &Worker) -> Result<()> {
                Ok(())
            }
            async fn get_worker(&self, _id: &WorkerId) -> Result<Option<Worker>> {
                Ok(None)
            }
            async fn get_all_workers(&self) -> Result<Vec<Worker>> {
                Ok(Vec::new())
            }
            async fn delete_worker(&self, _id: &WorkerId) -> Result<()> {
                Ok(())
            }
            async fn update_last_seen(&self, _id: &WorkerId) -> Result<()> {
                Ok(())
            }
            async fn find_stale_workers(
                &self,
                _threshold_duration: std::time::Duration,
            ) -> Result<Vec<Worker>> {
                Ok(Vec::new())
            }
            async fn update_worker_status(
                &self,
                _worker_id: &WorkerId,
                _status: hodei_pipelines_core::WorkerStatus,
            ) -> Result<()> {
                Ok(())
            }
        }

        let repo = DummyRepo;
        let worker_id = WorkerId::new();
        assert!(repo.update_last_seen(&worker_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_find_stale_workers_method_exists() {
        // Test that the trait has the find_stale_workers method
        struct DummyRepo;
        #[async_trait::async_trait]
        impl WorkerRepository for DummyRepo {
            async fn save_worker(&self, _worker: &Worker) -> Result<()> {
                Ok(())
            }
            async fn get_worker(&self, _id: &WorkerId) -> Result<Option<Worker>> {
                Ok(None)
            }
            async fn get_all_workers(&self) -> Result<Vec<Worker>> {
                Ok(Vec::new())
            }
            async fn delete_worker(&self, _id: &WorkerId) -> Result<()> {
                Ok(())
            }
            async fn update_last_seen(&self, _id: &WorkerId) -> Result<()> {
                Ok(())
            }
            async fn find_stale_workers(
                &self,
                _threshold_duration: std::time::Duration,
            ) -> Result<Vec<Worker>> {
                Ok(Vec::new())
            }
            async fn update_worker_status(
                &self,
                _worker_id: &WorkerId,
                _status: hodei_pipelines_core::WorkerStatus,
            ) -> Result<()> {
                Ok(())
            }
        }

        let repo = DummyRepo;
        let threshold = std::time::Duration::from_secs(30);
        assert!(repo.find_stale_workers(threshold).await.is_ok());
    }
}
