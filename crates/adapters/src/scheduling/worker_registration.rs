//! Worker Registration Adapter
//!
//! This module implements the WorkerRegistrationPort using a SchedulerPort.
//! It provides automatic registration of workers with retry logic, timeouts,
//! and batch operations with controlled concurrency.

use async_trait::async_trait;
use hodei_pipelines_domain::Worker;
use hodei_pipelines_domain::WorkerId;
use hodei_pipelines_ports::{
    SchedulerPort, WorkerRegistrationError, WorkerRegistrationPort, scheduler_port::SchedulerError,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
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
/// 🔴 CRITICAL FIX: Added task registry to prevent memory leaks
#[derive(Debug, Clone)]
pub struct WorkerRegistrationAdapter<T>
where
    T: SchedulerPort + Clone + 'static,
{
    scheduler: T,
    config: RegistrationConfig,
    // 🔴 CRITICAL FIX: Track active batch tasks to prevent memory leaks
    active_batch_tasks:
        Arc<Mutex<std::collections::HashMap<String, Vec<tokio::task::AbortHandle>>>>,
}

impl<T> WorkerRegistrationAdapter<T>
where
    T: SchedulerPort + Clone + 'static,
{
    /// Create new adapter with scheduler client
    /// 🔴 CRITICAL FIX: Initialize task registry
    pub fn new(scheduler: T, config: RegistrationConfig) -> Self {
        Self {
            scheduler,
            config,
            active_batch_tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
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
    /// 🔴 CRITICAL FIX: Added timeout and task tracking to prevent memory leaks
    async fn register_workers_batch(
        &self,
        workers: Vec<Worker>,
    ) -> Vec<Result<(), WorkerRegistrationError>> {
        if workers.is_empty() {
            return Vec::new();
        }

        // 🔴 CRITICAL FIX: Generate unique batch ID for tracking
        let batch_id = format!("batch-{}", uuid::Uuid::new_v4());
        info!(batch_id = %batch_id, worker_count = workers.len(), "Starting worker batch registration");

        // Clone data for use in async blocks
        let scheduler = self.scheduler.clone();
        let config = self.config.clone();
        let task_registry = self.active_batch_tasks.clone();

        // Use a semaphore to limit concurrent registrations
        let semaphore =
            std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.batch_concurrency));
        let mut tasks = Vec::with_capacity(workers.len());

        for worker in workers {
            let semaphore = semaphore.clone();
            let scheduler = scheduler.clone();
            let config = config.clone();

            // 🔴 CRITICAL FIX: Spawn task and capture AbortHandle
            let handle = tokio::spawn(async move {
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
            });

            // 🔴 CRITICAL FIX: Track the AbortHandle
            let abort_handle = handle.abort_handle();

            // 🔴 CRITICAL FIX: Register task in the tracking map
            {
                let mut registry = task_registry.lock().await;
                registry
                    .entry(batch_id.clone())
                    .or_insert_with(Vec::new)
                    .push(abort_handle);
            }

            tasks.push(handle);
        }

        // 🔴 CRITICAL FIX: Wait for all tasks with overall batch timeout
        let batch_timeout = tokio::time::timeout(
            Duration::from_secs(300), // 5 minutes max for batch
            futures::future::join_all(tasks),
        );

        let mut results = Vec::new();
        let timeout_occurred = match batch_timeout.await {
            Ok(join_results) => {
                info!(batch_id = %batch_id, "Batch completed successfully");
                results = join_results;
                false
            }
            Err(_) => {
                warn!(batch_id = %batch_id, "Batch timeout reached, aborting all tasks");
                // Abort all tasks
                let registry = task_registry.lock().await;
                if let Some(handles) = registry.get(&batch_id) {
                    for handle in handles {
                        handle.abort();
                    }
                }
                // Wait briefly for tasks to be cancelled
                tokio::time::sleep(Duration::from_millis(100)).await;
                true
            }
        };

        // 🔴 CRITICAL FIX: Cleanup: Remove completed tasks from registry
        {
            let mut registry = task_registry.lock().await;
            registry.remove(&batch_id);
        }

        info!(batch_id = %batch_id, "Batch registration cleanup complete");

        // Extract results, handling any panics or cancellations
        results
            .into_iter()
            .map(|result| match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(_e) => {
                    if timeout_occurred {
                        Err(WorkerRegistrationError::registration_failed(
                            "Batch timeout".to_string(),
                        ))
                    } else {
                        Err(WorkerRegistrationError::internal(
                            "Task cancelled".to_string(),
                        ))
                    }
                }
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
        SchedulerError::Validation(msg) => {
            WorkerRegistrationError::internal(format!("Validation error: {}", msg))
        }
        SchedulerError::Config(msg) => {
            WorkerRegistrationError::internal(format!("Configuration error: {}", msg))
        }
        SchedulerError::NoEligibleWorkers => {
            WorkerRegistrationError::internal("No eligible workers found".to_string())
        }
        SchedulerError::JobRepository(msg) => {
            WorkerRegistrationError::internal(format!("Job repository error: {}", msg))
        }
        SchedulerError::WorkerRepository(msg) => {
            WorkerRegistrationError::internal(format!("Worker repository error: {}", msg))
        }
        SchedulerError::WorkerClient(msg) => {
            WorkerRegistrationError::internal(format!("Worker client error: {}", msg))
        }
        SchedulerError::EventBus(msg) => {
            WorkerRegistrationError::internal(format!("Event bus error: {}", msg))
        }
        SchedulerError::ClusterState(msg) => {
            WorkerRegistrationError::internal(format!("Cluster state error: {}", msg))
        }
        SchedulerError::Internal(msg) => WorkerRegistrationError::internal(msg),
    }
}
