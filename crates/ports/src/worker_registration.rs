//! Worker Registration Port
//!
//! This module defines the port (trait) for worker registration operations
//! that handle automatic registration of workers with the scheduler.

use async_trait::async_trait;
use hodei_pipelines_core::Worker;
use hodei_pipelines_core::WorkerId;

/// Worker registration port error
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
#[error("Worker registration error: {0}")]
pub enum WorkerRegistrationError {
    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl WorkerRegistrationError {
    pub fn registration_failed<T: Into<String>>(msg: T) -> Self {
        WorkerRegistrationError::RegistrationFailed(msg.into())
    }

    pub fn worker_not_found<T: Into<WorkerId>>(worker_id: T) -> Self {
        WorkerRegistrationError::WorkerNotFound(worker_id.into())
    }

    pub fn internal<T: Into<String>>(msg: T) -> Self {
        WorkerRegistrationError::Internal(msg.into())
    }
}

/// Worker registration port
#[async_trait]
pub trait WorkerRegistrationPort: Send + Sync {
    /// Register a worker with automatic retry logic
    ///
    /// # Arguments
    ///
    /// * `worker` - The worker to register
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `WorkerRegistrationError` on failure.
    async fn register_worker(&self, worker: &Worker) -> Result<(), WorkerRegistrationError>;

    /// Unregister a worker from the scheduler
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The ID of the worker to unregister
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `WorkerRegistrationError` on failure.
    async fn unregister_worker(&self, worker_id: &WorkerId) -> Result<(), WorkerRegistrationError>;

    /// Register multiple workers in batch with parallel execution
    ///
    /// # Arguments
    ///
    /// * `workers` - List of workers to register
    ///
    /// # Returns
    ///
    /// Returns a list of results, one per worker.
    async fn register_workers_batch(
        &self,
        workers: Vec<Worker>,
    ) -> Vec<Result<(), WorkerRegistrationError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== WorkerRegistrationError Tests =====

    #[test]
    fn test_worker_registration_error_display() {
        let error = WorkerRegistrationError::registration_failed("test error");
        let error_str = format!("{}", error);
        assert!(error_str.contains("Registration failed"));
        assert!(error_str.contains("test error"));

        let error = WorkerRegistrationError::worker_not_found(WorkerId::new());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Worker not found"));

        let error = WorkerRegistrationError::internal("internal error");
        let error_str = format!("{}", error);
        assert!(error_str.contains("Internal error"));
    }

    #[test]
    fn test_worker_registration_error_clone() {
        let error1 = WorkerRegistrationError::registration_failed("error1".to_string());
        let error2 = error1.clone();

        assert_eq!(error1, error2);

        let worker_id = WorkerId::new();
        let error3 = WorkerRegistrationError::worker_not_found(worker_id.clone());
        let error4 = error3.clone();
        assert_eq!(error3, error4);
    }

    #[test]
    fn test_worker_registration_error_partial_eq() {
        let error1 = WorkerRegistrationError::registration_failed("same".to_string());
        let error2 = WorkerRegistrationError::registration_failed("same".to_string());
        let error3 = WorkerRegistrationError::internal("different".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);

        let worker_id1 = WorkerId::new();
        let worker_id2 = WorkerId::new();
        let error4 = WorkerRegistrationError::worker_not_found(worker_id1.clone());
        let error5 = WorkerRegistrationError::worker_not_found(worker_id1.clone());
        let error6 = WorkerRegistrationError::worker_not_found(worker_id2.clone());

        assert_eq!(error4, error5);
        assert_ne!(error4, error6);
    }

    #[test]
    fn test_worker_registration_error_factory_methods() {
        let err = WorkerRegistrationError::registration_failed("test registration");
        assert!(matches!(
            err,
            WorkerRegistrationError::RegistrationFailed(_)
        ));

        let worker_id = WorkerId::new();
        let err = WorkerRegistrationError::worker_not_found(worker_id);
        assert!(matches!(err, WorkerRegistrationError::WorkerNotFound(_)));

        let err = WorkerRegistrationError::internal("test internal");
        assert!(matches!(err, WorkerRegistrationError::Internal(_)));
    }

    #[test]
    fn test_worker_registration_port_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Box<dyn WorkerRegistrationPort>>();
    }

    #[test]
    fn test_worker_registration_port_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Box<dyn WorkerRegistrationPort>>();
    }

    #[test]
    fn test_worker_registration_error_is_error_trait() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<WorkerRegistrationError>();
    }
}
