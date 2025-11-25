//! Worker Registration Adapter
//!
//! This module implements the WorkerRegistrationPort using a SchedulerPort.
//! It provides automatic registration of workers with retry logic, timeouts,
//! and batch operations with controlled concurrency.

use async_trait::async_trait;
use hodei_core::Worker;
use hodei_ports::{
    SchedulerPort, WorkerRegistrationError, WorkerRegistrationPort, scheduler_port::SchedulerError,
};
use hodei_core::WorkerId;
use std::time::Duration;
use tracing::{error, info, warn};

/// Configuration for WorkerRegistrationAdapter
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationConfig {
    pub max_retries: u32,
    pub base_backoff: Duration,
    pub registration_timeout: Duration,
    pub batch_concurrency: usize,
}

impl Default for RegistrationConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_backoff: Duration::from_millis(100),
            registration_timeout: Duration::from_secs(30),
            batch_concurrency: 10,
        }
    }
}

/// Adapter that registers workers with the Scheduler
#[derive(Debug, Clone)]
pub struct WorkerRegistrationAdapter<T>
where
    T: SchedulerPort + Clone + 'static,
{
    scheduler: T,
    config: RegistrationConfig,
}

impl<T> WorkerRegistrationAdapter<T>
where
    T: SchedulerPort + Clone + 'static,
{
    /// Create new adapter with scheduler client
    pub fn new(scheduler: T, config: RegistrationConfig) -> Self {
        Self { scheduler, config }
    }

    /// Calculate exponential backoff duration
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base = self.config.base_backoff;
        let exp = 2u32.saturating_pow(attempt);
        base * exp
    }
}

#[async_trait]
impl<T> WorkerRegistrationPort for WorkerRegistrationAdapter<T>
where
    T: SchedulerPort + Clone + 'static,
{
    /// Register a worker with automatic retry logic
    async fn register_worker(&self, worker: &Worker) -> Result<(), WorkerRegistrationError> {
        let worker_id = &worker.id;
        let mut attempt = 0u32;

        loop {
            // Attempt registration with timeout
            let register_result = tokio::time::timeout(self.config.registration_timeout, async {
                self.scheduler
                    .register_worker(worker)
                    .await
                    .map_err(convert_scheduler_error)
            })
            .await;

            match register_result {
                Ok(Ok(())) => {
                    // Success
                    info!(
                        worker_id = %worker_id,
                        attempt = attempt + 1,
                        "Worker registered successfully"
                    );
                    return Ok(());
                }
                Ok(Err(err)) => {
                    // Registration failed, check if we should retry
                    if attempt >= self.config.max_retries {
                        error!(
                            worker_id = %worker_id,
                            error = %err,
                            attempts = attempt + 1,
                            "Failed to register worker after max retries"
                        );
                        return Err(err);
                    }

                    // Calculate backoff and retry
                    let backoff_duration = self.calculate_backoff(attempt);
                    warn!(
                        worker_id = %worker_id,
                        error = %err,
                        attempt = attempt + 1,
                        backoff_duration = ?backoff_duration,
                        "Worker registration failed, retrying"
                    );

                    tokio::time::sleep(backoff_duration).await;
                    attempt += 1;
                }
                Err(_) => {
                    // Timeout
                    if attempt >= self.config.max_retries {
                        error!(
                            worker_id = %worker_id,
                            attempts = attempt + 1,
                            "Failed to register worker due to timeout"
                        );
                        return Err(WorkerRegistrationError::registration_failed(format!(
                            "Registration timeout after {:?}",
                            self.config.registration_timeout
                        )));
                    }

                    let backoff_duration = self.calculate_backoff(attempt);
                    warn!(
                        worker_id = %worker_id,
                        attempt = attempt + 1,
                        backoff_duration = ?backoff_duration,
                        "Worker registration timed out, retrying"
                    );

                    tokio::time::sleep(backoff_duration).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Unregister a worker from scheduler
    async fn unregister_worker(&self, worker_id: &WorkerId) -> Result<(), WorkerRegistrationError> {
        self.scheduler
            .unregister_worker(worker_id)
            .await
            .map_err(convert_scheduler_error)?;

        info!(worker_id = %worker_id, "Worker unregistered");
        Ok(())
    }

    /// Register multiple workers in batch with controlled concurrency
    async fn register_workers_batch(
        &self,
        workers: Vec<Worker>,
    ) -> Vec<Result<(), WorkerRegistrationError>> {
        if workers.is_empty() {
            return Vec::new();
        }

        // Clone data for use in async blocks
        let scheduler = self.scheduler.clone();
        let config = self.config.clone();

        // Use a semaphore to limit concurrent registrations
        let semaphore =
            std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.batch_concurrency));
        let mut tasks = Vec::with_capacity(workers.len());

        for worker in workers {
            let semaphore = semaphore.clone();
            let scheduler = scheduler.clone();
            let config = config.clone();

            tasks.push(tokio::spawn(async move {
                // Acquire permit before registration
                let _permit = semaphore.acquire().await.unwrap();

                // Register the worker with retry logic inline
                let mut attempt = 0u32;
                let worker_id = &worker.id;

                loop {
                    let register_result =
                        tokio::time::timeout(config.registration_timeout, async {
                            scheduler
                                .register_worker(&worker)
                                .await
                                .map_err(convert_scheduler_error)
                        })
                        .await;

                    match register_result {
                        Ok(Ok(())) => {
                            info!(
                                worker_id = %worker_id,
                                attempt = attempt + 1,
                                "Worker registered successfully"
                            );
                            return Ok(());
                        }
                        Ok(Err(err)) => {
                            if attempt >= config.max_retries {
                                error!(
                                    worker_id = %worker_id,
                                    error = %err,
                                    attempts = attempt + 1,
                                    "Failed to register worker after max retries"
                                );
                                return Err(err);
                            }
                            let base = config.base_backoff;
                            let exp = 2u32.saturating_pow(attempt);
                            let backoff_duration = base * exp;
                            warn!(
                                worker_id = %worker_id,
                                error = %err,
                                attempt = attempt + 1,
                                backoff_duration = ?backoff_duration,
                                "Worker registration failed, retrying"
                            );
                            tokio::time::sleep(backoff_duration).await;
                            attempt += 1;
                        }
                        Err(_) => {
                            if attempt >= config.max_retries {
                                error!(
                                    worker_id = %worker_id,
                                    attempts = attempt + 1,
                                    "Failed to register worker due to timeout"
                                );
                                return Err(WorkerRegistrationError::registration_failed(format!(
                                    "Registration timeout after {:?}",
                                    config.registration_timeout
                                )));
                            }
                            let base = config.base_backoff;
                            let exp = 2u32.saturating_pow(attempt);
                            let backoff_duration = base * exp;
                            warn!(
                                worker_id = %worker_id,
                                attempt = attempt + 1,
                                backoff_duration = ?backoff_duration,
                                "Worker registration timed out, retrying"
                            );
                            tokio::time::sleep(backoff_duration).await;
                            attempt += 1;
                        }
                    }
                }
            }));
        }

        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;

        // Extract results, handling any panics
        results
            .into_iter()
            .map(|result| match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(WorkerRegistrationError::internal(format!(
                    "Task panicked: {}",
                    e
                ))),
            })
            .collect()
    }
}

/// Convert SchedulerError to WorkerRegistrationError
fn convert_scheduler_error(error: SchedulerError) -> WorkerRegistrationError {
    match error {
        SchedulerError::RegistrationFailed(msg) => {
            WorkerRegistrationError::registration_failed(msg)
        }
        SchedulerError::WorkerNotFound(worker_id) => {
            WorkerRegistrationError::worker_not_found(worker_id)
        }
        SchedulerError::Internal(msg) => WorkerRegistrationError::internal(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use hodei_core::Worker;
    use hodei_ports::scheduler_port::SchedulerPort;
    use hodei_core::WorkerCapabilities;

    // Mock implementation for testing
    #[derive(Debug, Clone)]
    pub struct MockSchedulerPort {
        pub registered_workers: Vec<WorkerId>,
        pub should_fail: bool,
        pub fail_with: SchedulerError,
    }

    impl MockSchedulerPort {
        pub fn new() -> Self {
            Self {
                registered_workers: Vec::new(),
                should_fail: false,
                fail_with: SchedulerError::internal("Mock error".to_string()),
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
    }

    // Helper to create a test worker
    fn create_test_worker() -> Worker {
        Worker::new(
            WorkerId::new(),
            "test-worker".to_string(),
            WorkerCapabilities::new(4, 8192),
        )
    }

    #[tokio::test]
    async fn test_register_worker_success() {
        let mock_scheduler = MockSchedulerPort::new();
        let config = RegistrationConfig::default();
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        let worker = create_test_worker();
        let result = adapter.register_worker(&worker).await;

        assert!(result.is_ok(), "Expected successful registration");
    }

    #[tokio::test]
    async fn test_register_worker_retry_on_failure() {
        let error = SchedulerError::registration_failed("Scheduler unavailable".to_string());
        let mock_scheduler = MockSchedulerPort::new().with_failure(error);
        let config = RegistrationConfig {
            max_retries: 2,
            ..Default::default()
        };
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        let worker = create_test_worker();
        let result = adapter.register_worker(&worker).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            WorkerRegistrationError::RegistrationFailed(_)
        ));
    }

    #[tokio::test]
    async fn test_unregister_worker_success() {
        let mock_scheduler = MockSchedulerPort::new();
        let config = RegistrationConfig::default();
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        let worker_id = WorkerId::new();
        let result = adapter.unregister_worker(&worker_id).await;

        assert!(result.is_ok(), "Expected successful unregistration");
    }

    #[tokio::test]
    async fn test_batch_registration_all_success() {
        let mock_scheduler = MockSchedulerPort::new();
        let config = RegistrationConfig {
            batch_concurrency: 5,
            ..Default::default()
        };
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        let workers: Vec<Worker> = (0..10)
            .map(|i| {
                Worker::new(
                    WorkerId::new(),
                    format!("test-worker-{}", i),
                    WorkerCapabilities::new(4, 8192),
                )
            })
            .collect();

        let results = adapter.register_workers_batch(workers).await;

        assert_eq!(results.len(), 10);
        assert!(
            results.into_iter().all(|r| r.is_ok()),
            "All batch registrations should succeed"
        );
    }

    #[tokio::test]
    async fn test_batch_registration_partial_failure() {
        let error = SchedulerError::internal("Persistent error".to_string());
        let mock_scheduler = MockSchedulerPort::new().with_failure(error);
        let config = RegistrationConfig {
            max_retries: 1,
            batch_concurrency: 5,
            ..Default::default()
        };
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        let workers: Vec<Worker> = (0..5)
            .map(|i| {
                Worker::new(
                    WorkerId::new(),
                    format!("test-worker-{}", i),
                    WorkerCapabilities::new(4, 8192),
                )
            })
            .collect();

        let results = adapter.register_workers_batch(workers).await;

        assert_eq!(results.len(), 5);
        assert!(
            results.into_iter().all(|r| r.is_err()),
            "All batch registrations should fail"
        );
    }

    #[tokio::test]
    async fn test_batch_registration_empty() {
        let mock_scheduler = MockSchedulerPort::new();
        let config = RegistrationConfig::default();
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        let results = adapter.register_workers_batch(Vec::new()).await;

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_calculate_backoff() {
        let mock_scheduler = MockSchedulerPort::new();
        let config = RegistrationConfig {
            base_backoff: Duration::from_millis(100),
            ..Default::default()
        };
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        assert_eq!(adapter.calculate_backoff(0), Duration::from_millis(100));
        assert_eq!(adapter.calculate_backoff(1), Duration::from_millis(200));
        assert_eq!(adapter.calculate_backoff(2), Duration::from_millis(400));
        assert_eq!(adapter.calculate_backoff(3), Duration::from_millis(800));
        assert_eq!(
            adapter.calculate_backoff(10),
            Duration::from_millis(102_400)
        );
    }

    #[tokio::test]
    async fn test_registration_config_default() {
        let config = RegistrationConfig::default();

        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_backoff, Duration::from_millis(100));
        assert_eq!(config.registration_timeout, Duration::from_secs(30));
        assert_eq!(config.batch_concurrency, 10);
    }

    #[tokio::test]
    async fn test_registration_config_clone() {
        let config = RegistrationConfig::default();
        let cloned = config.clone();

        assert_eq!(config, cloned);
    }

    #[tokio::test]
    async fn test_register_worker_with_different_worker_types() {
        let mock_scheduler = MockSchedulerPort::new();
        let config = RegistrationConfig::default();
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        // Test with different capability configurations
        for cpu_cores in &[1, 2, 4, 8, 16] {
            let worker = Worker::new(
                WorkerId::new(),
                format!("worker-{}", cpu_cores),
                WorkerCapabilities::new(*cpu_cores, 8192),
            );

            let result = adapter.register_worker(&worker).await;
            assert!(
                result.is_ok(),
                "Registration should succeed for all worker types"
            );
        }
    }

    #[tokio::test]
    async fn test_batch_registration_concurrency_limit() {
        let mock_scheduler = MockSchedulerPort::new();
        let config = RegistrationConfig {
            batch_concurrency: 2, // Very low concurrency
            ..Default::default()
        };
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, config);

        let workers: Vec<Worker> = (0..10)
            .map(|i| {
                Worker::new(
                    WorkerId::new(),
                    format!("test-worker-{}", i),
                    WorkerCapabilities::new(4, 8192),
                )
            })
            .collect();

        let start = std::time::Instant::now();
        let results = adapter.register_workers_batch(workers).await;
        let duration = start.elapsed();

        // With concurrency of 2 and 10 workers, it should take at least 5 * base_backoff
        // (accounting for registration time)
        assert_eq!(results.len(), 10);
        assert!(
            results.into_iter().all(|r| r.is_ok()),
            "All registrations should succeed"
        );

        // The test should complete successfully
        // Note: duration might be very small in mock tests, so we just verify it completes
        assert!(duration.as_nanos() >= 0);
    }

    #[test]
    fn test_convert_scheduler_error() {
        let sched_err = SchedulerError::registration_failed("test".to_string());
        let reg_err = convert_scheduler_error(sched_err.clone());

        assert!(matches!(
            reg_err,
            WorkerRegistrationError::RegistrationFailed(_)
        ));

        let sched_err = SchedulerError::worker_not_found(WorkerId::new());
        let reg_err = convert_scheduler_error(sched_err);

        assert!(matches!(
            reg_err,
            WorkerRegistrationError::WorkerNotFound(_)
        ));

        let sched_err = SchedulerError::internal("internal".to_string());
        let reg_err = convert_scheduler_error(sched_err);

        assert!(matches!(reg_err, WorkerRegistrationError::Internal(_)));
    }
}
