//! Scheduling Pipeline Module
//!
//! Implements the 4-phase Kubernetes-style scheduling pipeline:
//! 1. INFORM: Watch and discover available workers
//! 2. FILTER: Apply constraints and filter feasible workers
//! 3. SCORE: Rank workers based on scoring criteria
//! 4. BIND: Assign job to best worker

use crate::backend::SchedulerBackend;
use crate::types::*;
use crate::{JobId, SchedulerError, WorkerId};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// Scheduling pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub inform_interval_ms: u64,
    pub filter_enabled: bool,
    pub score_enabled: bool,
    pub max_candidates: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            inform_interval_ms: 1000, // 1 second
            filter_enabled: true,
            score_enabled: true,
            max_candidates: 100, // Limit to prevent performance issues
        }
    }
}

/// Main scheduling pipeline
pub struct SchedulingPipeline {
    backend: Arc<dyn SchedulerBackend>,
    config: PipelineConfig,
    informer: Informer,
}

impl SchedulingPipeline {
    /// Create new scheduling pipeline
    pub fn new(config: PipelineConfig) -> Self {
        Self {
            backend: Arc::new(crate::backend::KubernetesBackend::new()),
            config: config.clone(),
            informer: Informer::new(config.inform_interval_ms),
        }
    }

    /// Set backend (used for testing)
    pub fn set_backend(&mut self, backend: Arc<dyn SchedulerBackend>) {
        self.backend = backend;
    }

    /// Phase 1: Inform - Get available workers
    pub async fn inform(&self) -> Result<Vec<WorkerNode>, SchedulerError> {
        debug!("Phase 1: Inform - discovering available workers");

        let workers = self.backend.list_nodes().await?;

        // Filter only available workers
        let available_workers: Vec<_> = workers
            .into_iter()
            .filter(|w| w.status.is_available())
            .collect();

        debug!(
            "Phase 1: Found {} available workers",
            available_workers.len()
        );

        Ok(available_workers)
    }

    /// Phase 2: Filter - Apply constraints and filter feasible workers
    pub async fn filter(
        &self,
        job: &Job,
        workers: Vec<WorkerNode>,
    ) -> Result<Vec<WorkerNode>, SchedulerError> {
        debug!("Phase 2: Filter - filtering feasible workers");

        if !self.config.filter_enabled {
            debug!("Phase 2: Filtering disabled, returning all workers");
            return Ok(workers);
        }

        let total_workers = workers.len();
        let mut feasible_workers = Vec::new();

        for worker in workers {
            if self.worker_matches_job(&worker, job) {
                feasible_workers.push(worker);
            }

            // Limit candidates to prevent performance issues
            if feasible_workers.len() >= self.config.max_candidates {
                debug!(
                    "Phase 2: Reached max candidates limit ({})",
                    self.config.max_candidates
                );
                break;
            }
        }

        debug!(
            "Phase 2: Filtered to {} feasible workers out of {}",
            feasible_workers.len(),
            total_workers
        );

        Ok(feasible_workers)
    }

    /// Phase 3: Score - Rank workers based on scoring criteria
    pub async fn score(
        &self,
        job: &Job,
        workers: Vec<WorkerNode>,
    ) -> Result<Vec<ScoredWorker>, SchedulerError> {
        debug!("Phase 3: Score - ranking workers");

        if !self.config.score_enabled {
            debug!("Phase 3: Scoring disabled, assigning equal scores");
            return Ok(workers
                .into_iter()
                .map(|w| ScoredWorker::new(w, 50.0))
                .collect());
        }

        let scoring_weights = ScoringWeights::default();

        let mut scored_workers: Vec<_> = workers
            .into_iter()
            .map(|worker| {
                let score = worker.calculate_score(job, &scoring_weights);
                ScoredWorker::new(worker, score)
            })
            .collect();

        // Sort by score descending
        scored_workers.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!(
            "Phase 3: Scored {} workers, best score: {:.2}",
            scored_workers.len(),
            scored_workers.first().map(|s| s.score).unwrap_or(0.0)
        );

        Ok(scored_workers)
    }

    /// Phase 4: Bind - Job is already assigned in schedule_job
    /// This is a placeholder for the binding phase
    pub async fn bind(&self, job_id: &JobId, worker_id: &WorkerId) -> Result<(), SchedulerError> {
        debug!(
            "Phase 4: Bind - binding job {} to worker {}",
            job_id, worker_id
        );

        // Actual binding is done by the backend in the main scheduler
        // This is here for completeness of the pipeline
        info!("Job {} bound to worker {}", job_id, worker_id);

        Ok(())
    }

    /// Check if worker matches job requirements
    fn worker_matches_job(&self, worker: &WorkerNode, job: &Job) -> bool {
        // Check resources
        if let Some(req) = &job.spec.resource_requirements {
            if !worker.resources.has_resources(req) {
                return false;
            }
        }

        // Check node selector
        if let Some(selector) = &job.spec.node_selector {
            for (key, value) in &selector.labels {
                match worker.labels.get(key) {
                    Some(node_value) => {
                        if node_value != value {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
        }

        // Check taints and tolerations
        for taint in &worker.taints {
            let has_matching_toleration = job.spec.tolerations.iter().any(|tol| {
                tol.key == taint.key && tol.value == taint.value && tol.effect == taint.effect
            });

            // If there's no matching toleration and effect is NoSchedule, reject
            if !has_matching_toleration && taint.effect == TaintEffect::NoSchedule {
                return false;
            }
        }

        // Check affinity rules (simplified)
        if let Some(affinity) = &job.spec.affinity {
            // In production: implement full affinity matching logic
            // For now: assume all nodes match
        }

        true
    }
}

/// Informer component for watching cluster state
#[derive(Debug)]
pub struct Informer {
    interval_ms: u64,
    last_sync: Arc<std::sync::Mutex<std::time::Instant>>,
}

impl Informer {
    pub fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            last_sync: Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
        }
    }

    /// Check if informer should sync
    pub fn should_sync(&self) -> bool {
        let last_sync = self.last_sync.lock().unwrap();
        last_sync.elapsed().as_millis() >= self.interval_ms as u128
    }

    /// Update last sync time
    pub fn update_sync(&self) {
        let mut last_sync = self.last_sync.lock().unwrap();
        *last_sync = std::time::Instant::now();
    }
}

/// Filter stage of the pipeline
#[derive(Debug)]
pub struct FilterStage {
    enabled: bool,
}

impl FilterStage {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Apply filters to workers
    pub fn apply(&self, job: &Job, workers: Vec<WorkerNode>) -> Vec<WorkerNode> {
        if !self.enabled {
            return workers;
        }

        workers
            .into_iter()
            .filter(|w| self.passes_filters(w, job))
            .collect()
    }

    /// Check if worker passes all filters
    fn passes_filters(&self, worker: &WorkerNode, job: &Job) -> bool {
        // Resource filter
        if let Some(req) = &job.spec.resource_requirements {
            if !worker.resources.has_resources(req) {
                return false;
            }
        }

        // Node selector filter
        if let Some(selector) = &job.spec.node_selector {
            for (key, value) in &selector.labels {
                match worker.labels.get(key) {
                    Some(node_value) => {
                        if node_value != value {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
        }

        // Taint/toleration filter
        for taint in &worker.taints {
            let has_matching_toleration = job.spec.tolerations.iter().any(|tol| {
                tol.key == taint.key && tol.value == taint.value && tol.effect == taint.effect
            });

            if !has_matching_toleration && taint.effect == TaintEffect::NoSchedule {
                return false;
            }
        }

        true
    }
}

/// Scoring stage of the pipeline
#[derive(Debug)]
pub struct ScoreStage {
    weights: ScoringWeights,
}

impl ScoreStage {
    pub fn new(weights: ScoringWeights) -> Self {
        Self { weights }
    }

    /// Score workers
    pub fn apply(&self, job: &Job, workers: Vec<WorkerNode>) -> Vec<ScoredWorker> {
        workers
            .into_iter()
            .map(|worker| {
                let score = worker.calculate_score(job, &self.weights);
                ScoredWorker::new(worker, score)
            })
            .collect()
    }
}

/// Bind stage of the pipeline
pub struct BindStage {
    backend: Arc<dyn SchedulerBackend>,
}

impl BindStage {
    pub fn new(backend: Arc<dyn SchedulerBackend>) -> Self {
        Self { backend }
    }

    /// Bind job to worker
    pub async fn bind(&self, job_id: &JobId, worker_id: &WorkerId) -> Result<(), SchedulerError> {
        self.backend.bind_job(job_id, worker_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{ComputeResource, SchedulerBackend};
    use crate::types::WorkerNode;
    use std::collections::HashMap;

    struct MockBackend {
        workers: Vec<WorkerNode>,
    }

    impl MockBackend {
        fn new(workers: Vec<WorkerNode>) -> Self {
            Self { workers }
        }
    }

    #[async_trait::async_trait]
    impl SchedulerBackend for MockBackend {
        fn backend_type(&self) -> crate::types::BackendType {
            crate::types::BackendType::Kubernetes
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
            tracing::info!("Mock binding job {} to node {}", job_id, node_id);
            Ok(())
        }

        async fn unbind_job(
            &self,
            job_id: &JobId,
            node_id: &WorkerId,
        ) -> Result<(), SchedulerError> {
            tracing::info!("Mock unbinding job {} from node {}", job_id, node_id);
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
    async fn test_pipeline_inform() {
        let workers = vec![WorkerNode {
            id: uuid::Uuid::new_v4(),
            backend_type: crate::types::BackendType::Kubernetes,
            status: crate::types::WorkerStatus::Ready,
            resources: ComputeResource {
                cpu_cores: 8.0,
                memory_bytes: 16_000_000_000,
                gpu_count: 2,
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
        }];

        let backend = Arc::new(MockBackend::new(workers));
        let config = PipelineConfig::default();
        let mut pipeline = SchedulingPipeline::new(config);
        pipeline.set_backend(backend);

        let available_workers = pipeline.inform().await.unwrap();
        assert_eq!(available_workers.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_filter() {
        let workers = vec![
            WorkerNode {
                id: uuid::Uuid::new_v4(),
                backend_type: crate::types::BackendType::Kubernetes,
                status: crate::types::WorkerStatus::Ready,
                resources: ComputeResource {
                    cpu_cores: 8.0,
                    memory_bytes: 16_000_000_000,
                    gpu_count: 2,
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
                backend_type: crate::types::BackendType::Kubernetes,
                status: crate::types::WorkerStatus::Offline,
                resources: ComputeResource {
                    cpu_cores: 8.0,
                    memory_bytes: 16_000_000_000,
                    gpu_count: 2,
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
        ];

        let backend = Arc::new(MockBackend::new(workers));
        let config = PipelineConfig::default();
        let mut pipeline = SchedulingPipeline::new(config);
        pipeline.set_backend(backend);

        let job = crate::types::Job {
            metadata: crate::types::JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            spec: crate::types::JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(4.0),
                    memory_bytes: Some(8_000_000_000),
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

        let available_workers = pipeline.inform().await.unwrap();
        let filtered_workers = pipeline.filter(&job, available_workers).await.unwrap();

        // Only 1 worker should be available (Ready status)
        assert_eq!(filtered_workers.len(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_score() {
        let workers = vec![WorkerNode {
            id: uuid::Uuid::new_v4(),
            backend_type: crate::types::BackendType::Kubernetes,
            status: crate::types::WorkerStatus::Ready,
            resources: ComputeResource {
                cpu_cores: 8.0,
                memory_bytes: 16_000_000_000,
                gpu_count: 2,
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
        }];

        let backend = Arc::new(MockBackend::new(workers));
        let config = PipelineConfig::default();
        let mut pipeline = SchedulingPipeline::new(config);
        pipeline.set_backend(backend);

        let job = crate::types::Job {
            metadata: crate::types::JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            spec: crate::types::JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(4.0),
                    memory_bytes: Some(8_000_000_000),
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

        let available_workers = pipeline.inform().await.unwrap();
        let filtered_workers = pipeline.filter(&job, available_workers).await.unwrap();
        let scored_workers = pipeline.score(&job, filtered_workers).await.unwrap();

        assert_eq!(scored_workers.len(), 1);
        assert!(scored_workers[0].score > 0.0);
    }
}
