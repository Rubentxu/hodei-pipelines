//! Worker Lifecycle Management (US-005)
//!
//! This module provides comprehensive worker lifecycle management including:
//! - Worker state transitions (AVAILABLE, RUNNING, UNHEALTHY, DRAINING)
//! - Health monitoring with heartbeat-based detection
//! - Auto-recovery mechanisms
//! - Capability matching for optimal job scheduling

use hodei_shared_types::{CorrelationId, TenantId, Uuid};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{Instant, interval};

/// Worker state representing lifecycle stage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerState {
    /// Worker is available for job assignment
    AVAILABLE,
    /// Worker is currently executing jobs
    RUNNING,
    /// Worker is unhealthy and not accepting new jobs
    UNHEALTHY,
    /// Worker is draining and completing existing jobs
    DRAINING,
}

impl WorkerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerState::AVAILABLE => "AVAILABLE",
            WorkerState::RUNNING => "RUNNING",
            WorkerState::UNHEALTHY => "UNHEALTHY",
            WorkerState::DRAINING => "DRAINING",
        }
    }

    pub fn can_accept_jobs(&self) -> bool {
        matches!(self, WorkerState::AVAILABLE | WorkerState::RUNNING)
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self, WorkerState::AVAILABLE | WorkerState::RUNNING)
    }
}

/// Worker capabilities (e.g., CPU type, GPU, special hardware)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerCapabilities {
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub has_gpu: bool,
    pub gpu_count: Option<u8>,
    pub specialized_hardware: Vec<String>,
    pub container_runtime: String,
}

impl Default for WorkerCapabilities {
    fn default() -> Self {
        Self {
            cpu_cores: 4,
            memory_gb: 8,
            has_gpu: false,
            gpu_count: None,
            specialized_hardware: vec![],
            container_runtime: "docker".to_string(),
        }
    }
}

/// Worker information and state
#[derive(Debug, Clone)]
pub struct Worker {
    pub id: Uuid,
    pub state: WorkerState,
    pub capabilities: WorkerCapabilities,
    pub load: f64,
    pub jobs_running: usize,
    pub max_jobs: usize,
    pub last_heartbeat: Instant,
    pub failure_count: u32,
    pub tenant_id: Option<TenantId>,
}

impl Worker {
    pub fn new(
        id: Uuid,
        capabilities: WorkerCapabilities,
        max_jobs: usize,
        tenant_id: Option<TenantId>,
    ) -> Self {
        Self {
            id,
            state: WorkerState::AVAILABLE,
            capabilities,
            load: 0.0,
            jobs_running: 0,
            max_jobs,
            last_heartbeat: Instant::now(),
            failure_count: 0,
            tenant_id,
        }
    }

    pub fn can_accept_job(&self) -> bool {
        self.state.can_accept_jobs() && self.jobs_running < self.max_jobs && self.state.is_healthy()
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
        self.failure_count = 0;
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        if self.failure_count >= 3 {
            self.state = WorkerState::UNHEALTHY;
        }
    }
}

/// Worker Manager for lifecycle operations
pub struct WorkerManager {
    workers: Arc<Mutex<HashMap<Uuid, Worker>>>,
    coordinator: Option<Arc<hodei_coordinator::JobCoordinator>>,
    health_check_interval: Duration,
    heartbeat_timeout: Duration,
}

impl WorkerManager {
    pub fn new(
        coordinator: Option<Arc<hodei_coordinator::JobCoordinator>>,
        health_check_interval: Duration,
        heartbeat_timeout: Duration,
    ) -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
            coordinator,
            health_check_interval,
            heartbeat_timeout,
        }
    }

    /// Register a new worker
    pub fn register_worker(&self, worker: Worker) -> Result<(), WorkerLifecycleError> {
        let worker_id = worker.id;
        let mut workers = self.workers.lock();

        if workers.contains_key(&worker_id) {
            return Err(WorkerLifecycleError::WorkerAlreadyExists);
        }

        workers.insert(worker_id, worker);
        tracing::info!("Worker registered: {}", worker_id);

        Ok(())
    }

    /// Deregister a worker (graceful shutdown)
    pub async fn deregister_worker(&self, worker_id: Uuid) -> Result<(), WorkerLifecycleError> {
        let mut workers = self.workers.lock();

        if let Some(mut worker) = workers.remove(&worker_id) {
            if worker.jobs_running > 0 {
                worker.state = WorkerState::DRAINING;
                workers.insert(worker_id, worker);
                tracing::info!("Worker {} entered draining state", worker_id);

                // Wait for jobs to complete (simplified - in real impl would track completion)
                tokio::time::sleep(Duration::from_secs(30)).await;
                workers.remove(&worker_id);
            }

            tracing::info!("Worker deregistered: {}", worker_id);
        }

        Ok(())
    }

    /// Process worker heartbeat
    pub fn handle_heartbeat(
        &self,
        worker_id: Uuid,
        load: f64,
        jobs_running: usize,
    ) -> Result<(), WorkerLifecycleError> {
        let mut workers = self.workers.lock();

        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.update_heartbeat();
            worker.load = load;
            worker.jobs_running = jobs_running;

            if worker.state == WorkerState::UNHEALTHY && worker.failure_count < 3 {
                worker.state = WorkerState::AVAILABLE;
                tracing::info!("Worker {} recovered from unhealthy state", worker_id);
            }

            return Ok(());
        }

        Err(WorkerLifecycleError::WorkerNotFound)
    }

    /// Find available workers with matching capabilities
    pub fn find_suitable_workers(
        &self,
        required_cpus: u32,
        required_memory_gb: u64,
        needs_gpu: bool,
        max_jobs_per_worker: usize,
    ) -> Vec<Uuid> {
        let workers = self.workers.lock();

        let mut suitable_workers: Vec<_> = workers
            .values()
            .filter(|w| {
                w.can_accept_job()
                    && w.capabilities.cpu_cores >= required_cpus
                    && w.capabilities.memory_gb >= required_memory_gb
                    && (!needs_gpu || w.capabilities.has_gpu)
                    && w.jobs_running < max_jobs_per_worker
            })
            .cloned()
            .collect();

        // Sort by load (ascending) and job count (ascending)
        suitable_workers.sort_by(|a, b| {
            a.load
                .partial_cmp(&b.load)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.jobs_running.cmp(&b.jobs_running))
        });

        suitable_workers.into_iter().map(|w| w.id).collect()
    }

    /// Mark worker as unhealthy
    pub fn mark_unhealthy(&self, worker_id: Uuid) {
        let mut workers = self.workers.lock();
        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.state = WorkerState::UNHEALTHY;
            worker.record_failure();
            tracing::warn!(
                "Worker {} marked as unhealthy (failures: {})",
                worker_id,
                worker.failure_count
            );
        }
    }

    /// Get worker status
    pub fn get_worker_status(
        &self,
        worker_id: Uuid,
    ) -> Result<WorkerStatusResponse, WorkerLifecycleError> {
        let workers = self.workers.lock();

        if let Some(worker) = workers.get(&worker_id) {
            Ok(WorkerStatusResponse {
                worker_id: worker.id,
                state: worker.state.as_str().to_string(),
                load: worker.load,
                jobs_running: worker.jobs_running,
                max_jobs: worker.max_jobs,
                last_heartbeat: Some(worker.last_heartbeat.elapsed().as_secs()),
                capabilities: worker.capabilities.clone(),
                failure_count: worker.failure_count,
            })
        } else {
            Err(WorkerLifecycleError::WorkerNotFound)
        }
    }

    /// Start health monitoring background task
    pub fn start_health_monitoring(&self) {
        let workers = Arc::clone(&self.workers);
        let heartbeat_timeout = self.heartbeat_timeout;
        let coordinator = self.coordinator.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                let mut failed_workers = vec![];
                {
                    let workers_guard = workers.lock();
                    let now = Instant::now();

                    for (worker_id, worker) in workers_guard.iter() {
                        if now.duration_since(worker.last_heartbeat) > heartbeat_timeout {
                            failed_workers.push(*worker_id);
                        }
                    }
                }

                // Process failures outside the lock
                for worker_id in failed_workers {
                    tracing::warn!("Worker {} heartbeat timeout detected", worker_id);

                    // Mark as unhealthy
                    {
                        let mut workers_guard = workers.lock();
                        if let Some(worker) = workers_guard.get_mut(&worker_id) {
                            worker.state = WorkerState::UNHEALTHY;
                            worker.record_failure();
                        }
                    }

                    // Notify coordinator if available
                    if let Some(coordinator) = &coordinator {
                        coordinator.handle_worker_failure(worker_id);
                    }
                }
            }
        });
    }

    /// Get all workers summary
    pub fn get_workers_summary(&self) -> Vec<WorkerSummary> {
        let workers = self.workers.lock();

        workers
            .values()
            .map(|w| WorkerSummary {
                worker_id: w.id,
                state: w.state.as_str().to_string(),
                load: w.load,
                jobs_running: w.jobs_running,
                has_gpu: w.capabilities.has_gpu,
            })
            .collect()
    }
}

/// Worker status response
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerStatusResponse {
    pub worker_id: Uuid,
    pub state: String,
    pub load: f64,
    pub jobs_running: usize,
    pub max_jobs: usize,
    pub last_heartbeat: Option<u64>,
    pub capabilities: WorkerCapabilities,
    pub failure_count: u32,
}

/// Worker summary for listing
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerSummary {
    pub worker_id: Uuid,
    pub state: String,
    pub load: f64,
    pub jobs_running: usize,
    pub has_gpu: bool,
}

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum WorkerLifecycleError {
    #[error("worker not found")]
    WorkerNotFound,

    #[error("worker already exists")]
    WorkerAlreadyExists,

    #[error("invalid worker state transition")]
    InvalidStateTransition,

    #[error("worker is unhealthy")]
    WorkerUnhealthy,

    #[error("worker at capacity")]
    WorkerAtCapacity,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_worker_creation() {
        let worker = Worker::new(Uuid::new_v4(), WorkerCapabilities::default(), 10, None);

        assert_eq!(worker.state, WorkerState::AVAILABLE);
        assert_eq!(worker.jobs_running, 0);
        assert_eq!(worker.max_jobs, 10);
        assert!(worker.can_accept_job());
    }

    #[test]
    fn test_worker_state_transitions() {
        let mut worker = Worker::new(Uuid::new_v4(), WorkerCapabilities::default(), 10, None);

        // Start job
        worker.jobs_running = 5;
        assert!(worker.can_accept_job());

        // Too many jobs
        worker.jobs_running = 10;
        assert!(!worker.can_accept_job());

        // Unhealthy
        worker.state = WorkerState::UNHEALTHY;
        assert!(!worker.can_accept_job());
    }

    #[test]
    fn test_capability_matching() {
        let manager = WorkerManager::new(None, Duration::from_secs(30), Duration::from_secs(10));

        // Register workers with different capabilities
        manager
            .register_worker(Worker::new(
                Uuid::new_v4(),
                WorkerCapabilities {
                    cpu_cores: 8,
                    memory_gb: 16,
                    has_gpu: true,
                    gpu_count: Some(1),
                    specialized_hardware: vec!["nvme".to_string()],
                    container_runtime: "docker".to_string(),
                },
                5,
                None,
            ))
            .unwrap();

        // Find suitable workers
        let suitable = manager.find_suitable_workers(4, 8, true, 5);
        assert_eq!(suitable.len(), 1);
    }

    #[tokio::test]
    async fn test_heartbeat_handling() {
        let manager = WorkerManager::new(None, Duration::from_secs(30), Duration::from_secs(10));

        let worker_id = Uuid::new_v4();
        manager
            .register_worker(Worker::new(
                worker_id,
                WorkerCapabilities::default(),
                10,
                None,
            ))
            .unwrap();

        // Send heartbeat
        assert!(manager.handle_heartbeat(worker_id, 0.5, 2).is_ok());

        // Verify status
        let status = manager.get_worker_status(worker_id).unwrap();
        assert_eq!(status.state, "AVAILABLE");
        assert_eq!(status.load, 0.5);
        assert_eq!(status.jobs_running, 2);
    }
}
