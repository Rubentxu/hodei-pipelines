//! Scheduler Port
//!
//! This module defines the port (trait) for integrating worker management
//! with the scheduler system. This enables decoupled registration of
//! dynamically provisioned workers without depending on concrete scheduler
//! implementations.

use async_trait::async_trait;
use hodei_pipelines_domain::Worker;
use hodei_pipelines_domain::WorkerId;
use hodei_pipelines_proto::ServerMessage;
use tokio::sync::mpsc;

/// Scheduler port error
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
#[error("Scheduler error: {0}")]
pub enum SchedulerError {
    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("No eligible workers found")]
    NoEligibleWorkers,

    #[error("Job repository error: {0}")]
    JobRepository(String),

    #[error("Worker repository error: {0}")]
    WorkerRepository(String),

    #[error("Worker client error: {0}")]
    WorkerClient(String),

    #[error("Event bus error: {0}")]
    EventBus(String),

    #[error("Cluster state error: {0}")]
    ClusterState(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl SchedulerError {
    pub fn registration_failed<T: Into<String>>(msg: T) -> Self {
        SchedulerError::RegistrationFailed(msg.into())
    }

    pub fn worker_not_found<T: Into<WorkerId>>(worker_id: T) -> Self {
        SchedulerError::WorkerNotFound(worker_id.into())
    }

    pub fn validation_failed<T: Into<String>>(msg: T) -> Self {
        SchedulerError::Validation(msg.into())
    }

    pub fn internal<T: Into<String>>(msg: T) -> Self {
        SchedulerError::Internal(msg.into())
    }
}

/// Scheduler port
#[async_trait]
pub trait SchedulerPort: Send + Sync {
    /// Register a dynamically provisioned worker
    ///
    /// # Arguments
    ///
    /// * `worker` - The worker to register with the scheduler
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SchedulerError` on failure.
    async fn register_worker(&self, worker: &Worker) -> Result<(), SchedulerError>;

    /// Unregister a worker (e.g., when stopping/deleting)
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The ID of the worker to unregister
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SchedulerError` on failure.
    async fn unregister_worker(&self, worker_id: &WorkerId) -> Result<(), SchedulerError>;

    /// Get list of registered workers
    ///
    /// # Returns
    ///
    /// Returns a list of worker IDs on success, or a `SchedulerError` on failure.
    async fn get_registered_workers(&self) -> Result<Vec<WorkerId>, SchedulerError>;

    /// Register a transmitter (mpsc channel) for a worker
    ///
    /// This allows the scheduler to send messages to the worker through the transmitter.
    /// The transmitter is typically used for gRPC streaming communication.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The ID of the worker
    /// * `transmitter` - The mpsc sender to register
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SchedulerError` on failure.
    async fn register_transmitter(
        &self,
        worker_id: &WorkerId,
        transmitter: mpsc::UnboundedSender<Result<ServerMessage, SchedulerError>>,
    ) -> Result<(), SchedulerError>;

    /// Unregister a transmitter for a worker
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The ID of the worker
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SchedulerError` on failure.
    async fn unregister_transmitter(&self, worker_id: &WorkerId) -> Result<(), SchedulerError>;

    /// Send a message to a worker through their registered transmitter
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The ID of the worker
    /// * `message` - The message to send
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `SchedulerError` on failure.
    async fn send_to_worker(
        &self,
        worker_id: &WorkerId,
        message: ServerMessage,
    ) -> Result<(), SchedulerError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    // ===== SchedulerError Tests =====

    #[test]
    fn test_scheduler_error_display_trait() {
        let error = SchedulerError::registration_failed("test error");
        let error_str = format!("{}", error);
        assert!(error_str.contains("Registration failed"));
        assert!(error_str.contains("test error"));

        let error = SchedulerError::worker_not_found(WorkerId::new());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Worker not found"));

        let error = SchedulerError::internal("internal error");
        let error_str = format!("{}", error);
        assert!(error_str.contains("Internal error"));
    }

    #[test]
    fn test_scheduler_error_debug_trait() {
        let error = SchedulerError::registration_failed("test error");
        let debug_str = format!("{:?}", error);
        // thiserror uses a different debug format
        assert!(debug_str.contains("RegistrationFailed"));
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_scheduler_error_send_trait() {
        fn assert_send<T: Send>() {}
        assert_send::<SchedulerError>();
    }

    #[test]
    fn test_scheduler_error_sync_trait() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<SchedulerError>();
    }

    #[test]
    fn test_scheduler_error_clone() {
        let error1 = SchedulerError::registration_failed("error1".to_string());
        let error2 = error1.clone();

        assert_eq!(error1, error2);

        let worker_id = WorkerId::new();
        let error3 = SchedulerError::worker_not_found(worker_id.clone());
        let error4 = error3.clone();
        assert_eq!(error3, error4);
    }

    #[test]
    fn test_scheduler_error_partial_eq() {
        let error1 = SchedulerError::registration_failed("same".to_string());
        let error2 = SchedulerError::registration_failed("same".to_string());
        let error3 = SchedulerError::internal("different".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);

        let worker_id1 = WorkerId::new();
        let worker_id2 = WorkerId::new();
        let error4 = SchedulerError::worker_not_found(worker_id1.clone());
        let error5 = SchedulerError::worker_not_found(worker_id1.clone());
        let error6 = SchedulerError::worker_not_found(worker_id2.clone());

        assert_eq!(error4, error5);
        assert_ne!(error4, error6);
    }

    #[test]
    fn test_scheduler_error_equality_with_different_variants() {
        let err1 = SchedulerError::registration_failed("msg".to_string());
        let err2 = SchedulerError::internal("msg".to_string());

        assert_ne!(err1, err2);
    }

    #[test]
    fn test_scheduler_error_factory_methods() {
        let err = SchedulerError::registration_failed("test registration");
        assert!(matches!(err, SchedulerError::RegistrationFailed(_)));

        let worker_id = WorkerId::new();
        let err = SchedulerError::worker_not_found(worker_id);
        assert!(matches!(err, SchedulerError::WorkerNotFound(_)));

        let err = SchedulerError::internal("test internal");
        assert!(matches!(err, SchedulerError::Internal(_)));
    }

    #[test]
    fn test_scheduler_error_with_different_string_types() {
        let err1 = SchedulerError::registration_failed("static string");
        assert!(matches!(err1, SchedulerError::RegistrationFailed(_)));

        let err2 = SchedulerError::registration_failed(String::from("dynamic string"));
        assert!(matches!(err2, SchedulerError::RegistrationFailed(_)));

        let err3 = SchedulerError::internal(format!("formatted {}", "string"));
        assert!(matches!(err3, SchedulerError::Internal(_)));
    }

    // ===== Mock Implementation for Testing =====

    /// Mock implementation for testing
    #[derive(Debug, Clone)]
    pub struct MockSchedulerPort {
        pub registered_workers: Vec<WorkerId>,
        pub registered_transmitters: std::collections::HashMap<
            WorkerId,
            mpsc::UnboundedSender<Result<ServerMessage, SchedulerError>>,
        >,
        pub should_fail: bool,
        pub fail_with: SchedulerError,
        #[allow(dead_code)]
        pub sent_messages: Vec<(WorkerId, ServerMessage)>,
    }

    impl MockSchedulerPort {
        pub fn new() -> Self {
            Self {
                registered_workers: Vec::new(),
                registered_transmitters: std::collections::HashMap::new(),
                should_fail: false,
                fail_with: SchedulerError::internal("Mock error".to_string()),
                sent_messages: Vec::new(),
            }
        }

        pub fn with_worker(mut self, worker_id: WorkerId) -> Self {
            self.registered_workers.push(worker_id);
            self
        }

        pub fn with_failure(mut self, error: SchedulerError) -> Self {
            self.should_fail = true;
            self.fail_with = error;
            self
        }

        pub fn with_transmitter(
            mut self,
            worker_id: WorkerId,
            transmitter: mpsc::UnboundedSender<Result<ServerMessage, SchedulerError>>,
        ) -> Self {
            self.registered_transmitters.insert(worker_id, transmitter);
            self
        }
    }

    #[async_trait]
    impl SchedulerPort for MockSchedulerPort {
        async fn register_worker(&self, _worker: &Worker) -> Result<(), SchedulerError> {
            if self.should_fail {
                return Err(self.fail_with.clone());
            }
            Ok(())
        }

        async fn unregister_worker(&self, _worker_id: &WorkerId) -> Result<(), SchedulerError> {
            if self.should_fail {
                return Err(self.fail_with.clone());
            }
            Ok(())
        }

        async fn get_registered_workers(&self) -> Result<Vec<WorkerId>, SchedulerError> {
            if self.should_fail {
                return Err(self.fail_with.clone());
            }
            Ok(self.registered_workers.clone())
        }

        async fn register_transmitter(
            &self,
            _worker_id: &WorkerId,
            _transmitter: mpsc::UnboundedSender<Result<ServerMessage, SchedulerError>>,
        ) -> Result<(), SchedulerError> {
            if self.should_fail {
                return Err(self.fail_with.clone());
            }
            Ok(())
        }

        async fn unregister_transmitter(
            &self,
            _worker_id: &WorkerId,
        ) -> Result<(), SchedulerError> {
            if self.should_fail {
                return Err(self.fail_with.clone());
            }
            Ok(())
        }

        async fn send_to_worker(
            &self,
            _worker_id: &WorkerId,
            _message: ServerMessage,
        ) -> Result<(), SchedulerError> {
            if self.should_fail {
                return Err(self.fail_with.clone());
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_register_worker_success() {
        let mock = MockSchedulerPort::new();
        let worker = Worker::new(
            WorkerId::new(),
            "test-worker".to_string(),
            hodei_pipelines_domain::WorkerCapabilities::new(4, 8192),
        );

        let result = mock.register_worker(&worker).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_register_worker_failure() {
        let error = SchedulerError::registration_failed("Test error".to_string());
        let mock = MockSchedulerPort::new().with_failure(error.clone());
        let worker = Worker::new(
            WorkerId::new(),
            "test-worker".to_string(),
            hodei_pipelines_domain::WorkerCapabilities::new(4, 8192),
        );

        let result = mock.register_worker(&worker).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), error);
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_unregister_worker_success() {
        let mock = MockSchedulerPort::new();
        let worker_id = WorkerId::new();

        let result = mock.unregister_worker(&worker_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_get_registered_workers_success() {
        let worker1 = WorkerId::new();
        let worker2 = WorkerId::new();
        let mock = MockSchedulerPort::new()
            .with_worker(worker1.clone())
            .with_worker(worker2.clone());

        let result = mock.get_registered_workers().await;
        assert!(result.is_ok());

        let workers = result.unwrap();
        assert_eq!(workers.len(), 2);
        assert!(workers.contains(&worker1));
        assert!(workers.contains(&worker2));
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_get_registered_workers_empty() {
        let mock = MockSchedulerPort::new();

        let result = mock.get_registered_workers().await;
        assert!(result.is_ok());

        let workers = result.unwrap();
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_dyn_trait_object() {
        let mock = MockSchedulerPort::new()
            .with_worker(WorkerId::new())
            .with_worker(WorkerId::new());

        // Test that we can create a trait object
        let boxed: Box<dyn SchedulerPort> = Box::new(mock);
        assert!(
            boxed
                .register_worker(&Worker::new(
                    WorkerId::new(),
                    "test".to_string(),
                    hodei_pipelines_domain::WorkerCapabilities::new(4, 8192),
                ))
                .await
                .is_ok()
        );

        let workers = boxed.get_registered_workers().await.unwrap();
        assert_eq!(workers.len(), 2);
    }

    // ===== Transmitter Tests =====

    #[tokio::test]
    async fn test_mock_scheduler_port_register_transmitter_success() {
        let mock = MockSchedulerPort::new();
        let (tx, _rx) = mpsc::unbounded_channel::<Result<ServerMessage, SchedulerError>>();
        let worker_id = WorkerId::new();

        let result = mock.register_transmitter(&worker_id, tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_register_transmitter_failure() {
        let error = SchedulerError::registration_failed("Test error".to_string());
        let mock = MockSchedulerPort::new().with_failure(error.clone());
        let (tx, _rx) = mpsc::unbounded_channel::<Result<ServerMessage, SchedulerError>>();
        let worker_id = WorkerId::new();

        let result = mock.register_transmitter(&worker_id, tx).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), error);
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_unregister_transmitter_success() {
        let mock = MockSchedulerPort::new();
        let worker_id = WorkerId::new();

        let result = mock.unregister_transmitter(&worker_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_unregister_transmitter_failure() {
        let error = SchedulerError::internal("Test error".to_string());
        let mock = MockSchedulerPort::new().with_failure(error.clone());
        let worker_id = WorkerId::new();

        let result = mock.unregister_transmitter(&worker_id).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), error);
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_send_to_worker_success() {
        let mock = MockSchedulerPort::new();
        let worker_id = WorkerId::new();
        let message = ServerMessage { payload: None };

        let result = mock.send_to_worker(&worker_id, message.clone()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_scheduler_port_send_to_worker_failure() {
        let error = SchedulerError::worker_not_found(WorkerId::new());
        let mock = MockSchedulerPort::new().with_failure(error.clone());
        let worker_id = WorkerId::new();
        let message = ServerMessage { payload: None };

        let result = mock.send_to_worker(&worker_id, message).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), error);
    }

    #[tokio::test]
    async fn test_scheduler_port_with_transmitter_builder() {
        let (tx, _rx) = mpsc::unbounded_channel::<Result<ServerMessage, SchedulerError>>();
        let worker_id = WorkerId::new();
        let worker_id_clone = worker_id.clone();
        let mock = MockSchedulerPort::new()
            .with_worker(worker_id_clone.clone())
            .with_transmitter(worker_id_clone, tx);

        assert!(mock.registered_transmitters.contains_key(&worker_id));
    }

    #[tokio::test]
    async fn test_transmitter_methods_with_trait_object() {
        let mock = MockSchedulerPort::new();
        let (tx, _rx) = mpsc::unbounded_channel::<Result<ServerMessage, SchedulerError>>();
        let worker_id = WorkerId::new();

        let boxed: Box<dyn SchedulerPort> = Box::new(mock);

        // Test register_transmitter through trait object
        assert!(boxed.register_transmitter(&worker_id, tx).await.is_ok());

        // Test unregister_transmitter through trait object
        assert!(boxed.unregister_transmitter(&worker_id).await.is_ok());

        // Test send_to_worker through trait object
        let message = ServerMessage { payload: None };
        assert!(boxed.send_to_worker(&worker_id, message).await.is_ok());
    }

    #[test]
    fn test_scheduler_port_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Box<dyn SchedulerPort>>();
    }

    #[test]
    fn test_scheduler_port_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Box<dyn SchedulerPort>>();
    }

    #[test]
    fn test_scheduler_error_is_error_trait() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<SchedulerError>();
    }

    #[test]
    fn test_scheduler_error_has_source() {
        let error = SchedulerError::internal("test error");
        // All our errors have the inner string as source
        assert!(error.source().is_none());
    }

    // ===== Trait Object Tests =====

    #[test]
    fn test_scheduler_port_trait_object_safe() {
        // This test ensures the trait is object-safe
        fn takes_scheduler_port(_port: Box<dyn SchedulerPort>) {}
        let mock = MockSchedulerPort::new();
        takes_scheduler_port(Box::new(mock));
    }

    #[tokio::test]
    async fn test_scheduler_port_multiple_implementations() {
        // Test that we can have multiple implementations
        struct DummyScheduler;
        #[async_trait]
        impl SchedulerPort for DummyScheduler {
            async fn register_worker(&self, _worker: &Worker) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn unregister_worker(&self, _worker_id: &WorkerId) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn get_registered_workers(&self) -> Result<Vec<WorkerId>, SchedulerError> {
                Ok(vec![])
            }

            async fn register_transmitter(
                &self,
                _worker_id: &WorkerId,
                _transmitter: mpsc::UnboundedSender<Result<ServerMessage, SchedulerError>>,
            ) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn unregister_transmitter(
                &self,
                _worker_id: &WorkerId,
            ) -> Result<(), SchedulerError> {
                Ok(())
            }

            async fn send_to_worker(
                &self,
                _worker_id: &WorkerId,
                _message: ServerMessage,
            ) -> Result<(), SchedulerError> {
                Ok(())
            }
        }

        let dummy = DummyScheduler;
        assert!(
            dummy
                .register_worker(&Worker::new(
                    WorkerId::new(),
                    "test".to_string(),
                    hodei_pipelines_domain::WorkerCapabilities::new(4, 8192),
                ))
                .await
                .is_ok()
        );
    }

    // ===== Edge Cases =====

    #[test]
    fn test_scheduler_error_empty_message() {
        let err1 = SchedulerError::registration_failed("".to_string());
        let err2 = SchedulerError::internal("".to_string());
        let worker_id = WorkerId::new();
        let err3 = SchedulerError::worker_not_found(worker_id);

        assert_eq!(err1, err1.clone());
        assert_eq!(err2, err2.clone());
        assert_eq!(err3, err3.clone());
    }

    #[test]
    fn test_scheduler_error_long_message() {
        let long_message = "a".repeat(1000);
        let err = SchedulerError::internal(long_message.clone());
        assert_eq!(err.to_string().len() > 1000, true);
    }

    #[test]
    fn test_mock_scheduler_port_clone_consistency() {
        let original = MockSchedulerPort::new()
            .with_worker(WorkerId::new())
            .with_failure(SchedulerError::internal("test".to_string()));

        let cloned = original.clone();

        // Clone should preserve the data
        assert_eq!(
            cloned.registered_workers.len(),
            original.registered_workers.len()
        );
        assert_eq!(cloned.should_fail, original.should_fail);
        assert_eq!(cloned.fail_with, original.fail_with);
    }
}
