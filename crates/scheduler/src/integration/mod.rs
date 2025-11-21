//! Scheduler and Worker Lifecycle Integration (US-012)
//!
//! This module provides integration between the scheduler framework and the worker
//! lifecycle management system, enabling:
//! - Real-time worker state synchronization with scheduler
//! - Event-driven scheduling based on worker lifecycle events
//! - Automatic job rescheduling on worker failures
//! - Coordinated preemption and job cleanup
//! - Health monitoring and recovery

use crate::backend::SchedulerBackend;
use crate::types::{Job, WorkerNode, WorkerStatus};
use crate::{JobId, WorkerId};
use hodei_worker_lifecycle::WorkerManager;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Integration coordinator between scheduler and worker lifecycle
pub struct SchedulerWorkerIntegration {
    scheduler_backend: Arc<dyn SchedulerBackend>,
    worker_manager: Arc<WorkerManager>,
    event_handlers: Arc<Mutex<Vec<Box<dyn EventHandler>>>>,
    job_to_worker_map: Arc<Mutex<HashMap<JobId, WorkerId>>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl SchedulerWorkerIntegration {
    /// Create new integration coordinator
    pub fn new(
        scheduler_backend: Arc<dyn SchedulerBackend>,
        worker_manager: Arc<WorkerManager>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            scheduler_backend,
            worker_manager,
            event_handlers: Arc::new(Mutex::new(Vec::new())),
            job_to_worker_map: Arc::new(Mutex::new(HashMap::new())),
            shutdown_tx,
        }
    }

    /// Start the integration coordinator
    pub async fn start(&self) -> Result<(), IntegrationError> {
        info!("Starting scheduler-worker integration");

        // Start monitoring worker lifecycle events
        self.spawn_event_monitoring().await;

        // Register event handlers
        self.register_default_handlers().await;

        info!("Scheduler-worker integration started successfully");

        Ok(())
    }

    /// Stop the integration coordinator
    pub async fn stop(&self) -> Result<(), IntegrationError> {
        info!("Stopping scheduler-worker integration");

        // Notify all tasks to shutdown
        let _ = self.shutdown_tx.send(());

        info!("Scheduler-worker integration stopped");

        Ok(())
    }

    /// Handle worker registration event
    pub async fn handle_worker_registered(
        &self,
        worker_id: WorkerId,
    ) -> Result<(), IntegrationError> {
        info!("Worker registered event: {}", worker_id);

        // Update backend with worker info
        let worker_info = self
            .worker_manager
            .get_worker_status(worker_id)
            .map_err(|e| IntegrationError::WorkerLifecycleError(e.to_string()))?;

        // Trigger rescheduling of pending jobs if we now have capacity
        self.trigger_pending_job_retry().await;

        Ok(())
    }

    /// Handle worker heartbeat event
    pub async fn handle_worker_heartbeat(
        &self,
        worker_id: WorkerId,
        load: f64,
        jobs_running: usize,
    ) -> Result<(), IntegrationError> {
        // Update worker manager with heartbeat
        self.worker_manager
            .handle_heartbeat(worker_id, load, jobs_running)
            .map_err(|e| IntegrationError::WorkerLifecycleError(e.to_string()))?;

        // Check if we need to reschedule jobs based on load
        if load > 0.9 {
            warn!("Worker {} has high load ({:.2}%)", worker_id, load * 100.0);
            // Could trigger load balancing here
        }

        Ok(())
    }

    /// Handle worker failure event
    pub async fn handle_worker_failed(&self, worker_id: WorkerId) -> Result<(), IntegrationError> {
        error!("Worker failed event: {}", worker_id);

        // Get jobs running on this worker
        let jobs_to_reschedule = {
            let map = self.job_to_worker_map.lock();
            map.iter()
                .filter_map(|(job_id, w_id)| {
                    if *w_id == worker_id {
                        Some(*job_id)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        // Remove worker from job mapping
        {
            let mut map = self.job_to_worker_map.lock();
            map.retain(|_, w_id| *w_id != worker_id);
        }

        // Reschedule jobs on different workers
        for job_id in jobs_to_reschedule {
            info!(
                "Rescheduling job {} from failed worker {}",
                job_id, worker_id
            );
            // In production: get job details from scheduler queue and reschedule
            // For now: just log the event
        }

        Ok(())
    }

    /// Handle worker deregistration event
    pub async fn handle_worker_deregistered(
        &self,
        worker_id: WorkerId,
    ) -> Result<(), IntegrationError> {
        info!("Worker deregistered event: {}", worker_id);

        // Handle graceful shutdown
        self.handle_worker_failed(worker_id).await?;

        Ok(())
    }

    /// Notify scheduler that job was bound to worker
    pub fn notify_job_bound(&self, job_id: JobId, worker_id: WorkerId) {
        let mut map = self.job_to_worker_map.lock();
        map.insert(job_id, worker_id);
        info!("Job {} bound to worker {}", job_id, worker_id);
    }

    /// Notify scheduler that job completed on worker
    pub fn notify_job_completed(&self, job_id: JobId, worker_id: WorkerId) {
        let mut map = self.job_to_worker_map.lock();
        map.remove(&job_id);
        info!("Job {} completed on worker {}", job_id, worker_id);
    }

    /// Get worker status from lifecycle manager
    pub async fn get_worker_status(
        &self,
        worker_id: WorkerId,
    ) -> Result<WorkerStatusView, IntegrationError> {
        let status = self
            .worker_manager
            .get_worker_status(worker_id)
            .map_err(|e| IntegrationError::WorkerLifecycleError(e.to_string()))?;

        // Map to scheduler view
        let scheduler_status = match status.state.as_str() {
            "AVAILABLE" => WorkerStatus::Ready,
            "RUNNING" => WorkerStatus::Running,
            "UNHEALTHY" => WorkerStatus::Failed,
            "DRAINING" => WorkerStatus::Draining,
            _ => WorkerStatus::Offline,
        };

        Ok(WorkerStatusView {
            worker_id,
            state: scheduler_status,
            load: status.load,
            jobs_running: status.jobs_running,
            max_jobs: status.max_jobs,
            has_gpu: status.capabilities.has_gpu,
            backend_type: self.scheduler_backend.backend_type(),
        })
    }

    /// Get all workers summary
    pub async fn get_workers_summary(&self) -> Vec<WorkerSummary> {
        self.worker_manager
            .get_workers_summary()
            .into_iter()
            .map(|w| WorkerSummary {
                worker_id: w.worker_id,
                state: w.state,
                load: w.load,
                jobs_running: w.jobs_running,
                has_gpu: w.has_gpu,
            })
            .collect()
    }

    /// Start background monitoring tasks
    async fn spawn_event_monitoring(&self) {
        let worker_manager = Arc::clone(&self.worker_manager);
        let scheduler_backend = Arc::clone(&self.scheduler_backend);
        let shutdown_rx = self.shutdown_tx.subscribe();

        // Monitor worker health and sync with backend
        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                        // Periodic health check and sync
                        let workers = worker_manager.get_workers_summary();

                        // Sync with backend (simplified - in production would do incremental sync)
                        for worker_summary in workers {
                            let backend_status = match worker_summary.state.as_str() {
                                "AVAILABLE" => WorkerStatus::Ready,
                                "RUNNING" => WorkerStatus::Running,
                                "DRAINING" => WorkerStatus::Draining,
                                _ => WorkerStatus::Offline,
                            };

                            // Update backend if needed (in production: only if changed)
                            // scheduler_backend.update_worker_status(worker_summary.worker_id, backend_status).await;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });
    }

    /// Register default event handlers
    async fn register_default_handlers(&self) {
        let mut handlers = self.event_handlers.lock();
        handlers.push(Box::new(LoadBalancingHandler::new()));
        handlers.push(Box::new(PreemptionHandler::new()));
        handlers.push(Box::new(MetricsHandler::new()));
    }

    /// Trigger retry of pending jobs
    async fn trigger_pending_job_retry(&self) {
        info!("Triggering pending job retry due to worker availability change");
        // In production: would notify scheduler to re-evaluate pending jobs
    }

    /// Find workers for job using both scheduler and lifecycle manager
    pub async fn find_suitable_workers(
        &self,
        job: &Job,
    ) -> Result<Vec<WorkerNode>, IntegrationError> {
        // Get workers from backend
        let all_workers = self.scheduler_backend.list_nodes().await?;

        // Filter using lifecycle manager state
        let lifecycle_workers = self.worker_manager.get_workers_summary();
        let lifecycle_worker_ids: std::collections::HashSet<_> =
            lifecycle_workers.iter().map(|w| w.worker_id).collect();

        let suitable_workers: Vec<WorkerNode> = all_workers
            .into_iter()
            .filter(|worker| {
                // Check if worker is in lifecycle manager
                lifecycle_worker_ids.contains(&worker.id)
                    && worker.status == WorkerStatus::Ready
                    && worker.matches_requirements(job)
            })
            .collect();

        Ok(suitable_workers)
    }

    /// Handle job binding to worker
    pub async fn bind_job_to_worker(
        &self,
        job_id: &JobId,
        worker_id: &WorkerId,
    ) -> Result<(), IntegrationError> {
        // Record binding
        self.notify_job_bound(*job_id, *worker_id);

        // Notify backend
        self.scheduler_backend.bind_job(job_id, worker_id).await?;

        // Update worker state if needed
        // Note: Worker state is managed by WorkerManager, not scheduler

        Ok(())
    }

    /// Handle job unbinding from worker
    pub async fn unbind_job_from_worker(
        &self,
        job_id: &JobId,
        worker_id: &WorkerId,
    ) -> Result<(), IntegrationError> {
        // Remove binding
        {
            let mut map = self.job_to_worker_map.lock();
            map.remove(job_id);
        }

        // Notify backend
        self.scheduler_backend.unbind_job(job_id, worker_id).await?;

        Ok(())
    }

    /// Get job to worker mapping
    pub fn get_job_to_worker_map(&self) -> HashMap<JobId, WorkerId> {
        let map = self.job_to_worker_map.lock();
        map.clone()
    }

    /// Check if job is currently bound to a worker
    pub fn is_job_bound(&self, job_id: &JobId) -> bool {
        let map = self.job_to_worker_map.lock();
        map.contains_key(job_id)
    }

    /// Get worker that job is bound to
    pub fn get_job_worker(&self, job_id: &JobId) -> Option<WorkerId> {
        let map = self.job_to_worker_map.lock();
        map.get(job_id).copied()
    }
}

/// Event handler trait for integration events
pub trait EventHandler: Send + Sync {
    fn handle_worker_registered(&self, worker_id: WorkerId);
    fn handle_worker_failed(&self, worker_id: WorkerId);
    fn handle_worker_deregistered(&self, worker_id: WorkerId);
}

/// Load balancing event handler
struct LoadBalancingHandler;

impl LoadBalancingHandler {
    fn new() -> Self {
        Self
    }
}

impl EventHandler for LoadBalancingHandler {
    fn handle_worker_registered(&self, worker_id: WorkerId) {
        info!("[LoadBalancer] New worker registered: {}", worker_id);
    }

    fn handle_worker_failed(&self, worker_id: WorkerId) {
        error!("[LoadBalancer] Worker failed: {}", worker_id);
    }

    fn handle_worker_deregistered(&self, worker_id: WorkerId) {
        info!("[LoadBalancer] Worker deregistered: {}", worker_id);
    }
}

/// Preemption event handler
struct PreemptionHandler;

impl PreemptionHandler {
    fn new() -> Self {
        Self
    }
}

impl EventHandler for PreemptionHandler {
    fn handle_worker_registered(&self, worker_id: WorkerId) {
        info!("[Preemption] Worker registered: {}", worker_id);
    }

    fn handle_worker_failed(&self, worker_id: WorkerId) {
        warn!("[Preemption] Worker failed: {}", worker_id);
    }

    fn handle_worker_deregistered(&self, worker_id: WorkerId) {
        info!("[Preemption] Worker deregistered: {}", worker_id);
    }
}

/// Metrics event handler
struct MetricsHandler;

impl MetricsHandler {
    fn new() -> Self {
        Self
    }
}

impl EventHandler for MetricsHandler {
    fn handle_worker_registered(&self, worker_id: WorkerId) {
        info!("[Metrics] Worker registered: {}", worker_id);
    }

    fn handle_worker_failed(&self, worker_id: WorkerId) {
        warn!("[Metrics] Worker failed: {}", worker_id);
    }

    fn handle_worker_deregistered(&self, worker_id: WorkerId) {
        info!("[Metrics] Worker deregistered: {}", worker_id);
    }
}

/// Worker status view for integration
#[derive(Debug, Clone)]
pub struct WorkerStatusView {
    pub worker_id: WorkerId,
    pub state: WorkerStatus,
    pub load: f64,
    pub jobs_running: usize,
    pub max_jobs: usize,
    pub has_gpu: bool,
    pub backend_type: crate::types::BackendType,
}

/// Worker summary for listing
#[derive(Debug, Clone)]
pub struct WorkerSummary {
    pub worker_id: WorkerId,
    pub state: String,
    pub load: f64,
    pub jobs_running: usize,
    pub has_gpu: bool,
}

/// Integration error types
#[derive(thiserror::Error, Debug)]
pub enum IntegrationError {
    #[error("scheduler error: {0}")]
    SchedulerError(#[from] crate::SchedulerError),

    #[error("worker lifecycle error: {0}")]
    WorkerLifecycleError(String),

    #[error("worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("job not bound")]
    JobNotBound,

    #[error("integration error: {0}")]
    Integration(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{ComputeResource, SchedulerBackend};
    use crate::types::{BackendType, Job, JobMetadata, JobSpec, ResourceRequirements, WorkerNode};
    use chrono::Utc;
    use hodei_worker_lifecycle::{Worker, WorkerCapabilities, WorkerManager};
    use std::collections::HashMap;
    use tokio::time::Duration;

    /// Mock backend for testing
    struct MockIntegrationBackend {
        workers: Vec<WorkerNode>,
    }

    impl MockIntegrationBackend {
        fn new() -> Self {
            Self {
                workers: vec![
                    WorkerNode {
                        id: uuid::Uuid::new_v4(),
                        backend_type: BackendType::Kubernetes,
                        status: WorkerStatus::Ready,
                        resources: ComputeResource {
                            cpu_cores: 8.0,
                            memory_bytes: 16_000_000_000,
                            gpu_count: 0,
                        },
                        labels: HashMap::new(),
                        taints: vec![],
                        backend_specific: crate::types::BackendSpecific::Kubernetes(
                            crate::types::KubernetesNodeSpecific {
                                node_name: "node-1".to_string(),
                                namespace: "default".to_string(),
                            },
                        ),
                        location: crate::types::NodeLocation::default(),
                    },
                    WorkerNode {
                        id: uuid::Uuid::new_v4(),
                        backend_type: BackendType::Kubernetes,
                        status: WorkerStatus::Ready,
                        resources: ComputeResource {
                            cpu_cores: 4.0,
                            memory_bytes: 8_000_000_000,
                            gpu_count: 0,
                        },
                        labels: HashMap::new(),
                        taints: vec![],
                        backend_specific: crate::types::BackendSpecific::Kubernetes(
                            crate::types::KubernetesNodeSpecific {
                                node_name: "node-2".to_string(),
                                namespace: "default".to_string(),
                            },
                        ),
                        location: crate::types::NodeLocation::default(),
                    },
                ],
            }
        }
    }

    #[async_trait::async_trait]
    impl SchedulerBackend for MockIntegrationBackend {
        fn backend_type(&self) -> BackendType {
            BackendType::Kubernetes
        }

        async fn list_nodes(&self) -> Result<Vec<WorkerNode>, crate::SchedulerError> {
            Ok(self.workers.clone())
        }

        async fn get_node(&self, id: &WorkerId) -> Result<WorkerNode, crate::SchedulerError> {
            self.workers
                .iter()
                .find(|w| &w.id == id)
                .cloned()
                .ok_or_else(|| crate::SchedulerError::WorkerNotFound(*id))
        }

        async fn bind_job(
            &self,
            _job_id: &JobId,
            _node_id: &WorkerId,
        ) -> Result<(), crate::SchedulerError> {
            Ok(())
        }

        async fn unbind_job(
            &self,
            _job_id: &JobId,
            _node_id: &WorkerId,
        ) -> Result<(), crate::SchedulerError> {
            Ok(())
        }

        async fn get_node_status(
            &self,
            id: &WorkerId,
        ) -> Result<WorkerStatus, crate::SchedulerError> {
            self.workers
                .iter()
                .find(|w| &w.id == id)
                .map(|w| w.status.clone())
                .ok_or_else(|| crate::SchedulerError::WorkerNotFound(*id))
        }
    }

    #[tokio::test]
    async fn test_integration_creation() {
        let backend = Arc::new(MockIntegrationBackend::new());
        let worker_manager = Arc::new(WorkerManager::new(
            None,
            Duration::from_secs(30),
            Duration::from_secs(10),
        ));

        let integration = SchedulerWorkerIntegration::new(backend, worker_manager);

        assert!(integration.start().await.is_ok());
        assert!(integration.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_worker_registration() {
        let backend = Arc::new(MockIntegrationBackend::new());
        let worker_manager = Arc::new(WorkerManager::new(
            None,
            Duration::from_secs(30),
            Duration::from_secs(10),
        ));

        let integration = SchedulerWorkerIntegration::new(backend, worker_manager.clone());
        integration.start().await.unwrap();

        let worker_id = uuid::Uuid::new_v4();

        // Register worker in lifecycle manager
        worker_manager
            .register_worker(Worker::new(
                worker_id,
                WorkerCapabilities::default(),
                10,
                None,
            ))
            .unwrap();

        // Handle registration event
        assert!(
            integration
                .handle_worker_registered(worker_id)
                .await
                .is_ok()
        );

        // Check status
        let status = integration.get_worker_status(worker_id).await;
        assert!(status.is_ok());

        integration.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_job_binding_tracking() {
        let backend = Arc::new(MockIntegrationBackend::new());
        let worker_manager = Arc::new(WorkerManager::new(
            None,
            Duration::from_secs(30),
            Duration::from_secs(10),
        ));

        let integration = SchedulerWorkerIntegration::new(backend, worker_manager);
        integration.start().await.unwrap();

        let job_id = uuid::Uuid::new_v4();
        let worker_id = uuid::Uuid::new_v4();

        // Notify binding
        integration.notify_job_bound(job_id, worker_id);

        // Check if job is bound
        assert!(integration.is_job_bound(&job_id));
        assert_eq!(integration.get_job_worker(&job_id), Some(worker_id));

        // Check map
        let map = integration.get_job_to_worker_map();
        assert_eq!(map.get(&job_id), Some(&worker_id));

        // Notify completion
        integration.notify_job_completed(job_id, worker_id);

        // Check if job is no longer bound
        assert!(!integration.is_job_bound(&job_id));
        assert_eq!(integration.get_job_worker(&job_id), None);

        integration.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_find_suitable_workers() {
        let backend = Arc::new(MockIntegrationBackend::new());
        let worker_manager = Arc::new(WorkerManager::new(
            None,
            Duration::from_secs(30),
            Duration::from_secs(10),
        ));

        // Get backend workers before moving backend
        let backend_workers = backend.list_nodes().await.unwrap();

        let integration = SchedulerWorkerIntegration::new(backend, worker_manager.clone());
        integration.start().await.unwrap();

        // Register backend workers in the lifecycle manager
        for worker_node in backend_workers {
            worker_manager
                .register_worker(Worker::new(
                    worker_node.id,
                    WorkerCapabilities::default(),
                    10,
                    None,
                ))
                .unwrap();
        }

        let job = Job {
            metadata: JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: Utc::now(),
            },
            spec: JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(2.0),
                    memory_bytes: Some(4_000_000_000),
                    gpu_count: None,
                    ephemeral_storage: None,
                }),
                priority: crate::types::JobPriority::Medium,
                node_selector: None,
                affinity: None,
                tolerations: vec![],
                max_retries: 3,
            },
        };

        // Find suitable workers
        let suitable = integration.find_suitable_workers(&job).await;
        assert!(suitable.is_ok());

        // Should return all workers as they meet requirements
        let workers = suitable.unwrap();
        assert!(!workers.is_empty());

        integration.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_heartbeat() {
        let backend = Arc::new(MockIntegrationBackend::new());
        let worker_manager = Arc::new(WorkerManager::new(
            None,
            Duration::from_secs(30),
            Duration::from_secs(10),
        ));

        let integration = SchedulerWorkerIntegration::new(backend, worker_manager.clone());
        integration.start().await.unwrap();

        let worker_id = uuid::Uuid::new_v4();

        // Register worker
        worker_manager
            .register_worker(Worker::new(
                worker_id,
                WorkerCapabilities::default(),
                10,
                None,
            ))
            .unwrap();

        // Send heartbeat
        assert!(
            integration
                .handle_worker_heartbeat(worker_id, 0.5, 2)
                .await
                .is_ok()
        );

        integration.stop().await.unwrap();
    }
}
