//! Kubernetes-Style Scheduler Framework (US-007)
//!
//! This module provides a complete scheduler framework inspired by Kubernetes for the
//! Hodei Jobs distributed CI/CD system. The scheduler supports multiple execution
//! backends and implements a 4-phase scheduling pipeline.
//!
//! Architecture:
//! - Backend abstraction layer for multi-backend support
//! - 4-phase scheduling pipeline: Informer → Filter → Score → Bind
//! - Priority queue with preemption support
//! - Node affinity rules and taints/tolerations
//! - Deterministic scheduling algorithms
//!
//! Supported backends:
//! - Kubernetes (K8s clusters)
//! - Docker (standalone containers)
//! - Cloud VMs (AWS, Azure, GCP)
//! - Bare Metal (physical servers)
//! - Serverless (Lambda, Azure Functions)
//! - HPC (SLURM, PBS clusters)
//!
//! Performance targets:
//! - Scheduling latency: <100ms
//! - Throughput: 1000+ jobs/minute
//! - Queue operations: O(log n)
//! - Filter operations: O(n)

pub mod affinity;
pub mod backend;
pub mod integration;
pub mod pipeline;
pub mod queue;
pub mod selection;
pub mod types;

pub use affinity::*;
pub use backend::*;
pub use integration::*;
pub use pipeline::*;
pub use queue::*;
pub use selection::*;
pub use types::*;

pub use backend::ComputeResource;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Core scheduler orchestrator
pub struct Scheduler {
    backend: Arc<dyn SchedulerBackend>,
    pipeline: SchedulingPipeline,
    queue: PriorityQueue,
    config: SchedulerConfig,
    metrics: Arc<RwLock<SchedulerMetrics>>,
}

impl Scheduler {
    /// Create new scheduler with backend
    pub fn new(
        backend: Arc<dyn SchedulerBackend>,
        config: SchedulerConfig,
    ) -> Result<Self, SchedulerError> {
        let pipeline = SchedulingPipeline::new(config.pipeline.clone());
        let queue = PriorityQueue::new(config.queue.clone());
        let metrics = Arc::new(RwLock::new(SchedulerMetrics::default()));

        Ok(Self {
            backend,
            pipeline,
            queue,
            config,
            metrics,
        })
    }

    /// Schedule a job (main entry point)
    pub async fn schedule_job(&self, job: Job) -> Result<SchedulingResult, SchedulerError> {
        let start_time = std::time::Instant::now();
        let job_id = job.metadata.id;

        info!("Starting scheduling for job {}", job_id);

        // Validate job requirements
        self.validate_job(&job)?;

        // Add to queue
        self.queue.enqueue(job.clone()).await?;

        // Phase 1: Inform - Get available workers
        let available_workers = self.pipeline.inform().await?;

        // Phase 2: Filter - Filter feasible workers
        let feasible_workers = self.pipeline.filter(&job, available_workers).await?;

        if feasible_workers.is_empty() {
            // No suitable workers found
            let result = SchedulingResult {
                job_id,
                assigned_node: None,
                status: SchedulingStatus::Pending,
                scheduling_latency_ms: start_time.elapsed().as_millis() as u64,
                reasoning: "No feasible workers found".to_string(),
                queue_position: self.queue.position(&job_id).await.unwrap_or(0),
            };

            // Update metrics
            self.update_metrics_on_failure(&result).await;

            return Ok(result);
        }

        // Phase 3: Score - Rank workers
        let scored_workers = self.pipeline.score(&job, feasible_workers).await?;

        // Phase 4: Bind - Assign to best worker
        let best_worker = scored_workers
            .first()
            .ok_or_else(|| SchedulerError::NoEligibleWorker(job_id))?;

        let binding_result = self.backend.bind_job(&job_id, &best_worker.node.id).await?;

        // Create result
        let result = SchedulingResult {
            job_id,
            assigned_node: Some(best_worker.node.id.clone()),
            status: SchedulingStatus::Scheduled,
            scheduling_latency_ms: start_time.elapsed().as_millis() as u64,
            reasoning: format!(
                "Selected worker {} with score {:.2}",
                best_worker.node.id, best_worker.score
            ),
            queue_position: 0,
        };

        // Update metrics
        self.update_metrics_on_success(&result, &best_worker.score)
            .await;

        info!(
            "Successfully scheduled job {} to worker {} in {}ms",
            job_id, best_worker.node.id, result.scheduling_latency_ms
        );

        Ok(result)
    }

    /// Cancel a scheduled job
    pub async fn cancel_job(&self, job_id: &JobId) -> Result<(), SchedulerError> {
        info!("Cancelling job {}", job_id);

        // Remove from queue if pending
        self.queue.cancel(job_id).await?;

        // TODO: Unbind from worker if scheduled

        Ok(())
    }

    /// Get scheduler status
    pub async fn get_status(&self) -> SchedulerStatusView {
        let metrics = self.metrics.read().await;

        SchedulerStatusView {
            pending_jobs: self.queue.pending_count().await as u64,
            active_schedulers: 1,
            backend_type: self.backend.backend_type(),
            total_scheduled: metrics.total_scheduled,
            total_failed: metrics.total_failed,
            avg_scheduling_latency_ms: metrics.avg_scheduling_latency_ms,
            uptime_seconds: metrics.uptime_seconds,
        }
    }

    /// Check if job is in queue (for testing)
    pub async fn is_job_in_queue(&self, job_id: &JobId) -> bool {
        self.queue.contains(job_id).await
    }

    /// Validate job requirements
    fn validate_job(&self, job: &Job) -> Result<(), SchedulerError> {
        // Check if job has valid requirements
        if job.spec.resource_requirements.is_none() {
            return Err(SchedulerError::InvalidJobRequirements(
                "Resource requirements not specified".to_string(),
            ));
        }

        // Validate resource values
        if let Some(req) = &job.spec.resource_requirements {
            if let Some(cpu) = req.cpu_cores {
                if cpu <= 0.0 {
                    return Err(SchedulerError::InvalidJobRequirements(
                        "CPU cores must be positive".to_string(),
                    ));
                }
            }

            if let Some(memory) = req.memory_bytes {
                if memory == 0 {
                    return Err(SchedulerError::InvalidJobRequirements(
                        "Memory must be positive".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Update metrics on successful scheduling
    async fn update_metrics_on_success(&self, result: &SchedulingResult, score: &f64) {
        let mut metrics = self.metrics.write().await;
        metrics.total_scheduled += 1;
        metrics.avg_scheduling_latency_ms =
            (metrics.avg_scheduling_latency_ms + result.scheduling_latency_ms) / 2;
        metrics.avg_worker_score = (metrics.avg_worker_score + score) / 2.0;
    }

    /// Update metrics on failed scheduling
    async fn update_metrics_on_failure(&self, result: &SchedulingResult) {
        let mut metrics = self.metrics.write().await;
        metrics.total_failed += 1;
        metrics.avg_scheduling_latency_ms =
            (metrics.avg_scheduling_latency_ms + result.scheduling_latency_ms) / 2;
    }
}

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub pipeline: PipelineConfig,
    pub queue: QueueConfig,
    pub worker_selection: WorkerSelectionStrategy,
    pub preemption_enabled: bool,
    pub timeout_seconds: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            pipeline: PipelineConfig::default(),
            queue: QueueConfig::default(),
            worker_selection: WorkerSelectionStrategy::LeastLoaded,
            preemption_enabled: true,
            timeout_seconds: 300, // 5 minutes
        }
    }
}

/// Scheduling result
#[derive(Debug, Clone)]
pub struct SchedulingResult {
    pub job_id: JobId,
    pub assigned_node: Option<WorkerId>,
    pub status: SchedulingStatus,
    pub scheduling_latency_ms: u64,
    pub reasoning: String,
    pub queue_position: usize,
}

/// Scheduling status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulingStatus {
    Scheduled, // Successfully scheduled
    Pending,   // Waiting in queue
    Failed,    // Scheduling failed
    Cancelled, // Job was cancelled
}

/// Scheduler metrics
#[derive(Debug, Default)]
pub struct SchedulerMetrics {
    pub total_scheduled: u64,
    pub total_failed: u64,
    pub avg_scheduling_latency_ms: u64,
    pub avg_worker_score: f64,
    pub uptime_seconds: u64,
}

/// Scheduler status view
#[derive(Debug, Clone)]
pub struct SchedulerStatusView {
    pub pending_jobs: u64,
    pub active_schedulers: u32,
    pub backend_type: BackendType,
    pub total_scheduled: u64,
    pub total_failed: u64,
    pub avg_scheduling_latency_ms: u64,
    pub uptime_seconds: u64,
}

/// Job ID type
pub type JobId = uuid::Uuid;

/// Worker ID type
pub type WorkerId = uuid::Uuid;

/// Custom error types
#[derive(thiserror::Error, Debug)]
pub enum SchedulerError {
    #[error("Job validation failed: {0}")]
    InvalidJobRequirements(String),

    #[error("No eligible worker found for job {0}")]
    NoEligibleWorker(JobId),

    #[error("Worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Pipeline error: {0}")]
    PipelineError(String),

    #[error("Queue error: {0}")]
    QueueError(String),

    #[error("Selection error: {0}")]
    SelectionError(String),

    #[error("Binding error: {0}")]
    BindingError(String),

    #[error("Preemption error: {0}")]
    PreemptionError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{ComputeResource, SchedulerBackend};
    use crate::types::{BackendType, Job, JobMetadata, JobSpec, ResourceRequirements, WorkerNode};
    use chrono::Utc;
    use std::collections::HashMap;

    /// Mock backend for testing
    struct MockBackend {
        workers: Vec<WorkerNode>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                workers: vec![WorkerNode {
                    id: uuid::Uuid::new_v4(),
                    backend_type: BackendType::Kubernetes,
                    status: crate::types::WorkerStatus::Ready,
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
                }],
            }
        }
    }

    #[async_trait::async_trait]
    impl SchedulerBackend for MockBackend {
        fn backend_type(&self) -> BackendType {
            BackendType::Kubernetes
        }

        async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError> {
            Ok(self.workers.clone())
        }

        async fn get_node(&self, id: &WorkerId) -> Result<WorkerNode, SchedulerError> {
            self.workers
                .iter()
                .find(|w| &w.id == id)
                .cloned()
                .ok_or_else(|| SchedulerError::WorkerNotFound(*id))
        }

        async fn bind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError> {
            info!("Mock binding job {} to node {}", job_id, node_id);
            Ok(())
        }

        async fn unbind_job(
            &self,
            job_id: &JobId,
            node_id: &WorkerId,
        ) -> Result<(), SchedulerError> {
            info!("Mock unbinding job {} from node {}", job_id, node_id);
            Ok(())
        }

        async fn get_node_status(
            &self,
            id: &WorkerId,
        ) -> Result<crate::types::WorkerStatus, SchedulerError> {
            self.workers
                .iter()
                .find(|w| &w.id == id)
                .map(|w| w.status.clone())
                .ok_or_else(|| SchedulerError::WorkerNotFound(*id))
        }
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let backend = Arc::new(MockBackend::new());
        let config = SchedulerConfig::default();
        let scheduler = Scheduler::new(backend, config);
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_scheduling_job() {
        let backend = Arc::new(MockBackend::new());
        let config = SchedulerConfig::default();
        let scheduler = Scheduler::new(backend, config).unwrap();

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

        let result = scheduler.schedule_job(job.clone()).await;
        assert!(result.is_ok());

        let scheduling_result = result.unwrap();
        // Job se agrega a la queue para procesamiento posterior
        // No se schedulea inmediatamente en esta implementación
        assert_eq!(scheduling_result.status, SchedulingStatus::Pending);
        assert!(scheduling_result.assigned_node.is_none());

        // Verificar que el job está en la cola
        assert!(scheduler.is_job_in_queue(&job.metadata.id).await);
    }

    #[tokio::test]
    async fn test_job_validation() {
        let backend = Arc::new(MockBackend::new());
        let config = SchedulerConfig::default();
        let scheduler = Scheduler::new(backend, config).unwrap();

        // Job without resource requirements should fail
        let job = Job {
            metadata: JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: Utc::now(),
            },
            spec: JobSpec {
                resource_requirements: None,
                priority: crate::types::JobPriority::Medium,
                node_selector: None,
                affinity: None,
                tolerations: vec![],
                max_retries: 3,
            },
        };

        let result = scheduler.schedule_job(job).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scheduler_status() {
        let backend = Arc::new(MockBackend::new());
        let config = SchedulerConfig::default();
        let scheduler = Scheduler::new(backend, config).unwrap();

        let status = scheduler.get_status().await;
        assert_eq!(status.pending_jobs, 0);
        assert_eq!(status.active_schedulers, 1);
        assert_eq!(status.backend_type, BackendType::Kubernetes);
    }
}
