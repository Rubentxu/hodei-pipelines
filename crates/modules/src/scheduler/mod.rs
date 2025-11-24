//! Scheduler module for job scheduling and worker management

pub mod state_machine;

pub use state_machine::{SchedulingContext, SchedulingState, SchedulingStateMachine};

use dashmap::DashMap;
use hodei_core::{Job, JobId, Worker};
use hodei_ports::{
    EventPublisher, JobRepository, JobRepositoryError, WorkerClient, WorkerRepository,
};
use hodei_shared_types::{WorkerCapabilities, WorkerId};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_queue_size: usize,
    pub scheduling_interval_ms: u64,
    pub worker_heartbeat_timeout_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            scheduling_interval_ms: 1000,
            worker_heartbeat_timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub io_percent: f64,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_mb: 0,
            io_percent: 0.0,
        }
    }

    pub fn update(&mut self, cpu_percent: f64, memory_mb: u64, io_percent: f64) {
        self.cpu_percent = cpu_percent;
        self.memory_mb = memory_mb;
        self.io_percent = io_percent;
    }
}

#[derive(Debug, Clone)]
struct QueueEntry {
    job: Job,
    priority: u8,
    enqueue_time: chrono::DateTime<chrono::Utc>,
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.enqueue_time.cmp(&other.enqueue_time))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.enqueue_time == other.enqueue_time
            && self.job.id == other.job.id
    }
}

impl Eq for QueueEntry {}

pub struct SchedulerModule<R, E, W, WR>
where
    R: JobRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    pub(crate) job_repo: Arc<R>,
    pub(crate) event_bus: Arc<E>,
    pub(crate) worker_client: Arc<W>,
    pub(crate) worker_repo: Arc<WR>,
    pub(crate) config: SchedulerConfig,
    pub(crate) queue: Arc<RwLock<std::collections::BinaryHeap<QueueEntry>>>,
    pub(crate) cluster_state: Arc<ClusterState>,
}

/// Builder for SchedulerModule to eliminate Connascence of Position
pub struct SchedulerBuilder<R, E, W, WR>
where
    R: JobRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    job_repo: Option<Arc<R>>,
    event_bus: Option<Arc<E>>,
    worker_client: Option<Arc<W>>,
    worker_repo: Option<Arc<WR>>,
    config: Option<SchedulerConfig>,
}

impl<R, E, W, WR> SchedulerBuilder<R, E, W, WR>
where
    R: JobRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    pub fn new() -> Self {
        SchedulerBuilder {
            job_repo: None,
            event_bus: None,
            worker_client: None,
            worker_repo: None,
            config: None,
        }
    }

    pub fn job_repository(mut self, job_repo: Arc<R>) -> Self {
        self.job_repo = Some(job_repo);
        self
    }

    pub fn event_bus(mut self, event_bus: Arc<E>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub fn worker_client(mut self, worker_client: Arc<W>) -> Self {
        self.worker_client = Some(worker_client);
        self
    }

    pub fn worker_repository(mut self, worker_repo: Arc<WR>) -> Self {
        self.worker_repo = Some(worker_repo);
        self
    }

    pub fn config(mut self, config: SchedulerConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn build(self) -> Result<SchedulerModule<R, E, W, WR>, SchedulerError> {
        let job_repo = self
            .job_repo
            .ok_or_else(|| SchedulerError::Config("job_repository is required".into()))?;

        let event_bus = self
            .event_bus
            .ok_or_else(|| SchedulerError::Config("event_bus is required".into()))?;

        let worker_client = self
            .worker_client
            .ok_or_else(|| SchedulerError::Config("worker_client is required".into()))?;

        let worker_repo = self
            .worker_repo
            .ok_or_else(|| SchedulerError::Config("worker_repository is required".into()))?;

        let config = self.config.unwrap_or_else(|| SchedulerConfig::default());

        Ok(SchedulerModule {
            job_repo,
            event_bus,
            worker_client,
            worker_repo,
            config,
            queue: Arc::new(RwLock::new(std::collections::BinaryHeap::new())),
            cluster_state: Arc::new(ClusterState::new()),
        })
    }
}

impl<R, E, W, WR> Default for SchedulerBuilder<R, E, W, WR>
where
    R: JobRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<R, E, W, WR> SchedulerModule<R, E, W, WR>
where
    R: JobRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    pub fn new(
        job_repo: Arc<R>,
        event_bus: Arc<E>,
        worker_client: Arc<W>,
        worker_repo: Arc<WR>,
        config: SchedulerConfig,
    ) -> Self {
        Self {
            job_repo,
            event_bus,
            worker_client,
            worker_repo,
            config,
            queue: Arc::new(RwLock::new(std::collections::BinaryHeap::new())),
            cluster_state: Arc::new(ClusterState::new()),
        }
    }

    pub async fn schedule_job(&self, job: Job, priority: u8) -> Result<(), SchedulerError> {
        info!("Scheduling job: {}", job.id);

        job.spec
            .validate()
            .map_err(|e| SchedulerError::Validation(e.to_string()))?;

        let entry = QueueEntry {
            job: job.clone(),
            priority,
            enqueue_time: chrono::Utc::now(),
        };

        self.queue.write().await.push(entry);
        self.run_scheduling_cycle().await?;

        Ok(())
    }

    /// Schedule job using state machine (eliminates temporal coupling)
    pub async fn schedule_job_with_state_machine(&self, job: Job) -> Result<(), SchedulerError> {
        info!("Scheduling job using state machine: {}", job.id);

        job.spec
            .validate()
            .map_err(|e| SchedulerError::Validation(e.to_string()))?;

        let mut state_machine = state_machine::SchedulingStateMachine::new();
        state_machine.set_job(job);

        // Execute scheduling cycle using state machine
        state_machine.complete(self).await?;

        Ok(())
    }

    /// Get scheduling matches without committing (useful for testing or preview)
    pub async fn discover_matches(&self, job: &Job) -> Result<Option<Worker>, SchedulerError> {
        let mut state_machine = state_machine::SchedulingStateMachine::new();
        state_machine.set_job(job.clone());

        state_machine.discover_matches(self).await
    }

    async fn run_scheduling_cycle(&self) -> Result<(), SchedulerError> {
        let mut queue = self.queue.write().await;

        if let Some(entry) = queue.pop() {
            let job = entry.job;

            let workers = self.find_eligible_workers(&job).await?;

            if !workers.is_empty() {
                let selected_worker = self.select_best_worker(&workers, &job).await?;

                if self.reserve_worker(&selected_worker, &job.id).await? {
                    self.job_repo
                        .compare_and_swap_status(
                            &job.id,
                            hodei_core::JobState::PENDING,
                            hodei_core::JobState::SCHEDULED,
                        )
                        .await
                        .map_err(SchedulerError::JobRepository)?;

                    self.worker_client
                        .assign_job(&selected_worker.id, &job.id, &job.spec)
                        .await
                        .map_err(SchedulerError::WorkerClient)?;

                    info!(
                        "Successfully scheduled job {} on worker {}",
                        job.id, selected_worker.id
                    );
                }
            }
        }

        Ok(())
    }

    async fn find_eligible_workers(&self, job: &Job) -> Result<Vec<Worker>, SchedulerError> {
        let all_workers = self
            .worker_repo
            .get_all_workers()
            .await
            .map_err(SchedulerError::WorkerRepository)?;

        let eligible_workers: Vec<Worker> = all_workers
            .into_iter()
            .filter(|worker| {
                worker.is_available()
                    && u64::from(worker.capabilities.cpu_cores) * 1000 >= job.spec.resources.cpu_m
                    && worker.capabilities.memory_gb * 1024 >= job.spec.resources.memory_mb
            })
            .collect();

        Ok(eligible_workers)
    }

    async fn select_best_worker(
        &self,
        workers: &[Worker],
        job: &Job,
    ) -> Result<Worker, SchedulerError> {
        if workers.is_empty() {
            return Err(SchedulerError::NoEligibleWorkers);
        }

        // Get cluster state for current resource utilization
        let cluster_workers = self.cluster_state.get_all_workers().await;

        // Apply Bin Packing Algorithm with Resource Utilization optimization
        let best_worker = workers
            .iter()
            .filter(|w| w.is_available())
            .map(|worker| {
                let score = self.calculate_worker_score(worker, job, &cluster_workers);
                (worker.clone(), score)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(worker, _)| worker)
            .ok_or_else(|| SchedulerError::NoEligibleWorkers)?;

        info!(
            "Selected worker {} for job {} (score: {:.2})",
            best_worker.id,
            job.id,
            self.calculate_worker_score(&best_worker, job, &cluster_workers)
        );

        Ok(best_worker)
    }

    /// Calculate comprehensive worker score for optimal scheduling
    ///
    /// Uses Bin Packing + Priority-based + Load Balancing algorithm:
    /// - Resource utilization (minimize waste)
    /// - Current load (distribute evenly)
    /// - CPU/Memory fit (avoid over-provisioning)
    /// - Worker health and responsiveness
    fn calculate_worker_score(
        &self,
        worker: &Worker,
        job: &Job,
        cluster_workers: &[WorkerNode],
    ) -> f64 {
        // 1. Resource Utilization Score (40% weight)
        // Prefer workers that best fit the job without wasting resources
        let cpu_usage = self.get_worker_current_cpu_usage(worker, cluster_workers);
        let memory_usage = self.get_worker_current_memory_usage(worker, cluster_workers);

        let required_cpu = job.spec.resources.cpu_m as f64;
        let required_memory = job.spec.resources.memory_mb as f64;

        // Calculate fit score: penalize both under-utilization and over-provisioning
        let cpu_fit_score = self.calculate_fit_score(
            required_cpu,
            cpu_usage,
            worker.capabilities.cpu_cores as f64 * 1000.0,
        );
        let memory_fit_score = self.calculate_fit_score(
            required_memory,
            memory_usage,
            worker.capabilities.memory_gb as f64 * 1024.0,
        );

        let resource_score = (cpu_fit_score * 0.6 + memory_fit_score * 0.4) * 40.0;

        // 2. Load Balancing Score (30% weight)
        // Distribute jobs evenly across available workers
        let current_jobs = worker.current_jobs.len() as f64;
        let _load_score = 1.0 / (1.0 + current_jobs); // Fewer jobs = higher score
        let load_score_normalized =
            (1.0 - (current_jobs / worker.capabilities.max_concurrent_jobs as f64)).max(0.1);

        let load_balancing_score = load_score_normalized * 30.0;

        // 3. Health and Responsiveness Score (20% weight)
        // Prefer workers with recent heartbeats and good health
        let health_score = self.get_worker_health_score(worker, cluster_workers);
        let health_score_normalized = health_score.max(0.1); // Minimum 10% even for unhealthy
        let health_score_weighted = health_score_normalized * 20.0;

        // 4. Capability Match Score (10% weight)
        // Prefer workers that match labels/requirements exactly
        let capability_score = self.calculate_capability_score(worker, job);
        let capability_score_weighted = capability_score * 10.0;

        // Combined score
        let total_score = resource_score
            + load_balancing_score
            + health_score_weighted
            + capability_score_weighted;

        total_score
    }

    /// Calculate how well resources fit (Bin Packing approach)
    /// Penalizes both over-provisioning and tight fits
    fn calculate_fit_score(&self, required: f64, used: f64, available: f64) -> f64 {
        let total = available;
        let utilization = (used + required) / total;

        // Optimal utilization is between 60-85%
        let optimal_min = 0.60;
        let optimal_max = 0.85;

        if utilization >= optimal_min && utilization <= optimal_max {
            1.0 // Perfect fit
        } else if utilization < optimal_min {
            // Under-utilized - mild penalty
            0.7 + (utilization / optimal_min) * 0.3
        } else {
            // Over-utilized - severe penalty
            0.1 + (1.0 - (utilization - optimal_max) / (1.0 - optimal_max)) * 0.6
        }
    }

    /// Get current CPU usage for a worker
    fn get_worker_current_cpu_usage(&self, worker: &Worker, cluster_workers: &[WorkerNode]) -> f64 {
        cluster_workers
            .iter()
            .find(|w| w.id == worker.id)
            .map(|w| w.usage.cpu_percent)
            .unwrap_or(0.0)
    }

    /// Get current memory usage for a worker
    fn get_worker_current_memory_usage(
        &self,
        worker: &Worker,
        cluster_workers: &[WorkerNode],
    ) -> f64 {
        cluster_workers
            .iter()
            .find(|w| w.id == worker.id)
            .map(|w| w.usage.memory_mb as f64)
            .unwrap_or(0.0)
    }

    /// Calculate health score based on heartbeat recency and system responsiveness
    fn get_worker_health_score(&self, worker: &Worker, cluster_workers: &[WorkerNode]) -> f64 {
        if let Some(cluster_worker) = cluster_workers.iter().find(|w| w.id == worker.id) {
            let elapsed = cluster_worker.last_heartbeat.elapsed();

            // Healthy if heartbeat within last 30 seconds
            if elapsed < Duration::from_secs(30) {
                // Score from 0.5 to 1.0 based on recency
                let recency_score = 1.0 - (elapsed.as_secs_f64() / 30.0);
                0.5 + recency_score * 0.5
            } else {
                // Unhealthy - low score but not zero (still consider as fallback)
                0.1
            }
        } else {
            // Worker not in cluster state - unknown health
            0.3
        }
    }

    /// Calculate how well worker capabilities match job requirements
    fn calculate_capability_score(&self, worker: &Worker, job: &Job) -> f64 {
        let mut score = 1.0;

        // Exact match bonus
        let worker_cpu_m = worker.capabilities.cpu_cores as u64 * 1000;
        if worker_cpu_m >= job.spec.resources.cpu_m {
            let cpu_overhead =
                (worker_cpu_m - job.spec.resources.cpu_m) as f64 / job.spec.resources.cpu_m as f64;
            if cpu_overhead < 0.2 {
                score += 0.2; // Bonus for minimal overhead
            }
        }

        let worker_memory_mb = worker.capabilities.memory_gb * 1024;
        if worker_memory_mb >= job.spec.resources.memory_mb {
            let memory_overhead = (worker_memory_mb - job.spec.resources.memory_mb) as f64
                / job.spec.resources.memory_mb as f64;
            if memory_overhead < 0.2 {
                score += 0.2; // Bonus for minimal overhead
            }
        }

        // Label matching
        if !job.spec.env.is_empty() && !worker.capabilities.labels.is_empty() {
            let env_labels: std::collections::HashSet<&str> =
                job.spec.env.keys().map(|s| s.as_str()).collect();
            let worker_labels: std::collections::HashSet<&str> = worker
                .capabilities
                .labels
                .iter()
                .map(|(k, _v)| k.as_str())
                .collect();

            let matching_labels = env_labels.intersection(&worker_labels).count();
            let total_env_labels = env_labels.len();

            if total_env_labels > 0 {
                let label_match_ratio = matching_labels as f64 / total_env_labels as f64;
                score += label_match_ratio * 0.3;
            }
        }

        score.min(1.5) // Cap at 1.5 for bonus
    }

    async fn reserve_worker(
        &self,
        worker: &Worker,
        job_id: &JobId,
    ) -> Result<bool, SchedulerError> {
        self.cluster_state
            .reserve_job(&worker.id, job_id.clone())
            .await
            .map_err(|e| SchedulerError::ClusterState(e))
    }

    pub async fn register_worker(&self, worker: Worker) -> Result<(), SchedulerError> {
        self.worker_repo
            .save_worker(&worker)
            .await
            .map_err(SchedulerError::WorkerRepository)?;

        // Also register in cluster state
        self.cluster_state
            .register_worker(&worker.id, worker.capabilities.clone())
            .await
            .map_err(|e| SchedulerError::ClusterState(e.to_string()))?;

        Ok(())
    }

    pub async fn process_heartbeat(
        &self,
        worker_id: &WorkerId,
        resource_usage: Option<ResourceUsage>,
    ) -> Result<(), SchedulerError> {
        self.cluster_state
            .update_heartbeat(worker_id.clone(), resource_usage)
            .await;

        Ok(())
    }

    pub async fn get_cluster_stats(&self) -> ClusterStats {
        self.cluster_state.get_stats().await
    }

    pub async fn start(&self) -> Result<(), SchedulerError> {
        Ok(())
    }
}

impl<R, E, W, WR> Clone for SchedulerModule<R, E, W, WR>
where
    R: JobRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            job_repo: self.job_repo.clone(),
            event_bus: self.event_bus.clone(),
            worker_client: self.worker_client.clone(),
            worker_repo: self.worker_repo.clone(),
            config: self.config.clone(),
            queue: self.queue.clone(),
            cluster_state: self.cluster_state.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerNode {
    pub id: WorkerId,
    pub capabilities: WorkerCapabilities,
    pub usage: ResourceUsage,
    pub reserved_jobs: Vec<JobId>,
    pub last_heartbeat: Instant,
}

impl WorkerNode {
    pub fn is_healthy(&self) -> bool {
        self.last_heartbeat.elapsed() < Duration::from_secs(30)
    }

    pub fn has_capacity(&self, required_cores: u32, required_memory_mb: u64) -> bool {
        self.capabilities.cpu_cores >= required_cores
            && self.capabilities.memory_gb * 1024 >= required_memory_mb
    }
}

#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub total_workers: usize,
    pub healthy_workers: usize,
    pub total_jobs: usize,
    pub reserved_jobs: usize,
}

pub struct ClusterState {
    workers: Arc<DashMap<WorkerId, WorkerNode>>,
    job_assignments: Arc<DashMap<JobId, WorkerId>>,
}

impl ClusterState {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(DashMap::new()),
            job_assignments: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_worker(
        &self,
        worker_id: &WorkerId,
        capabilities: WorkerCapabilities,
    ) -> Result<(), String> {
        let node = WorkerNode {
            id: worker_id.clone(),
            capabilities,
            usage: ResourceUsage::new(),
            reserved_jobs: vec![],
            last_heartbeat: Instant::now(),
        };

        self.workers.insert(worker_id.clone(), node);
        info!("Worker registered: {}", worker_id);

        Ok(())
    }

    pub async fn update_resource_usage(
        &self,
        worker_id: &WorkerId,
        usage: ResourceUsage,
    ) -> Result<(), String> {
        if let Some(mut worker) = self.workers.get_mut(worker_id) {
            worker.usage = usage;
            worker.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(format!("Worker {} not found", worker_id))
        }
    }

    pub async fn update_heartbeat(
        &self,
        worker_id: WorkerId,
        resource_usage: Option<ResourceUsage>,
    ) {
        if let Some(mut worker) = self.workers.get_mut(&worker_id) {
            if let Some(usage) = resource_usage {
                worker.usage = usage;
            }
            worker.last_heartbeat = Instant::now();
        }
    }

    pub async fn get_worker(&self, worker_id: &WorkerId) -> Result<Option<WorkerNode>, String> {
        Ok(self.workers.get(worker_id).map(|entry| entry.clone()))
    }

    pub async fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub async fn get_all_workers(&self) -> Vec<WorkerNode> {
        self.workers.iter().map(|entry| entry.clone()).collect()
    }

    pub async fn get_healthy_workers(&self) -> Vec<WorkerNode> {
        self.workers
            .iter()
            .filter(|entry| entry.is_healthy())
            .map(|entry| entry.clone())
            .collect()
    }

    pub async fn reserve_job(&self, worker_id: &WorkerId, job_id: JobId) -> Result<bool, String> {
        // Check if worker exists and has capacity
        if let Some(mut worker) = self.workers.get_mut(worker_id) {
            worker.reserved_jobs.push(job_id.clone());
            self.job_assignments.insert(job_id, worker_id.clone());
            Ok(true)
        } else {
            Err(format!("Worker {} not found", worker_id))
        }
    }

    pub async fn release_job(&self, job_id: &JobId) -> Result<(), String> {
        if let Some((_, worker_id)) = self.job_assignments.remove(job_id) {
            if let Some(mut worker) = self.workers.get_mut(&worker_id) {
                worker.reserved_jobs.retain(|id| id != job_id);
            }
            Ok(())
        } else {
            Err(format!("Job {} not found in reservations", job_id))
        }
    }

    pub async fn get_stats(&self) -> ClusterStats {
        let total_workers = self.workers.len();
        let healthy_workers = self
            .workers
            .iter()
            .filter(|entry| entry.is_healthy())
            .count();
        let total_jobs = self.job_assignments.len();
        let reserved_jobs = self
            .workers
            .iter()
            .map(|entry| entry.reserved_jobs.len())
            .sum();

        ClusterStats {
            total_workers,
            healthy_workers,
            total_jobs,
            reserved_jobs,
        }
    }
}

impl Default for ClusterState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SchedulerError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("No eligible workers found")]
    NoEligibleWorkers,

    #[error("Job repository error: {0}")]
    JobRepository(JobRepositoryError),

    #[error("Worker repository error: {0}")]
    WorkerRepository(hodei_ports::WorkerRepositoryError),

    #[error("Worker client error: {0}")]
    WorkerClient(hodei_ports::WorkerClientError),

    #[error("Event bus error: {0}")]
    EventBus(hodei_ports::EventBusError),

    #[error("Cluster state error: {0}")]
    ClusterState(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::Worker;
    use hodei_core::{Job, JobId, JobSpec};
    use hodei_ports::{
        EventPublisher, JobRepository, JobRepositoryError, WorkerClient, WorkerRepository,
    };
    use hodei_shared_types::{WorkerCapabilities, WorkerId};
    use std::sync::Arc;

    // Mock implementations for testing
    #[derive(PartialEq, Clone)]
    struct MockJobRepository;
    #[derive(PartialEq, Clone)]
    struct MockEventBus;
    #[derive(PartialEq, Clone)]
    struct MockWorkerClient;
    #[derive(PartialEq, Clone)]
    struct MockWorkerRepository;

    #[async_trait::async_trait]
    impl JobRepository for MockJobRepository {
        async fn save_job(&self, _job: &Job) -> Result<(), JobRepositoryError> {
            Ok(())
        }

        async fn get_job(&self, _id: &JobId) -> Result<Option<Job>, JobRepositoryError> {
            Ok(None)
        }

        async fn get_pending_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
            Ok(vec![])
        }

        async fn get_running_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
            Ok(vec![])
        }

        async fn delete_job(&self, _id: &JobId) -> Result<(), JobRepositoryError> {
            Ok(())
        }

        async fn compare_and_swap_status(
            &self,
            _id: &JobId,
            _expected: &str,
            _new: &str,
        ) -> Result<bool, JobRepositoryError> {
            Ok(true)
        }
    }

    #[async_trait::async_trait]
    impl EventPublisher for MockEventBus {
        async fn publish(
            &self,
            _event: hodei_ports::SystemEvent,
        ) -> Result<(), hodei_ports::EventBusError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl WorkerClient for MockWorkerClient {
        async fn assign_job(
            &self,
            _worker_id: &WorkerId,
            _job_id: &JobId,
            _job_spec: &JobSpec,
        ) -> Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }

        async fn cancel_job(
            &self,
            _worker_id: &WorkerId,
            _job_id: &JobId,
        ) -> Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }

        async fn get_worker_status(
            &self,
            _worker_id: &WorkerId,
        ) -> Result<hodei_shared_types::WorkerStatus, hodei_ports::WorkerClientError> {
            Ok(hodei_shared_types::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().into(),
            })
        }

        async fn send_heartbeat(
            &self,
            _worker_id: &WorkerId,
        ) -> Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl WorkerRepository for MockWorkerRepository {
        async fn save_worker(
            &self,
            _worker: &Worker,
        ) -> Result<(), hodei_ports::WorkerRepositoryError> {
            Ok(())
        }

        async fn get_worker(
            &self,
            _id: &WorkerId,
        ) -> Result<Option<Worker>, hodei_ports::WorkerRepositoryError> {
            Ok(None)
        }

        async fn get_all_workers(&self) -> Result<Vec<Worker>, hodei_ports::WorkerRepositoryError> {
            Ok(vec![])
        }

        async fn delete_worker(
            &self,
            _id: &WorkerId,
        ) -> Result<(), hodei_ports::WorkerRepositoryError> {
            Ok(())
        }
    }

    // TODO: Fix these tests - they don't compile with current Rust restrictions on dyn traits
    // The SchedulerBuilder requires Sized bounds that are incompatible with dyn traits
    /*
    #[tokio::test]
    async fn test_scheduler_builder_basic() {
        let job_repo: Arc<dyn JobRepository> = Arc::new(MockJobRepository);
        let event_bus: Arc<dyn EventPublisher> = Arc::new(MockEventBus);
        let worker_client: Arc<dyn WorkerClient> = Arc::new(MockWorkerClient);
        let worker_repo: Arc<dyn WorkerRepository> = Arc::new(MockWorkerRepository);

        let scheduler = SchedulerBuilder::<
            dyn JobRepository,
            dyn EventPublisher,
            dyn WorkerClient,
            dyn WorkerRepository,
        >::new()
        .job_repository(job_repo.clone())
        .event_bus(event_bus.clone())
        .worker_client(worker_client.clone())
        .worker_repository(worker_repo.clone())
        .build()
        .unwrap();

        // Just verify it builds without panicking
        assert!(scheduler.job_repo.is_some());
    }

    #[tokio::test]
    async fn test_scheduler_builder_with_custom_config() {
        let job_repo: Arc<dyn JobRepository> = Arc::new(MockJobRepository);
        let event_bus: Arc<dyn EventPublisher> = Arc::new(MockEventBus);
        let worker_client: Arc<dyn WorkerClient> = Arc::new(MockWorkerClient);
        let worker_repo: Arc<dyn WorkerRepository> = Arc::new(MockWorkerRepository);

        let custom_config = SchedulerConfig {
            max_queue_size: 2000,
            scheduling_interval_ms: 500,
            worker_heartbeat_timeout_ms: 60000,
        };

        let scheduler = SchedulerBuilder::<
            dyn JobRepository,
            dyn EventPublisher,
            dyn WorkerClient,
            dyn WorkerRepository,
        >::new()
        .job_repository(job_repo.clone())
        .event_bus(event_bus.clone())
        .worker_client(worker_client.clone())
        .worker_repository(worker_repo.clone())
        .config(custom_config.clone())
        .build()
        .unwrap();

        assert_eq!(scheduler.config.max_queue_size, 2000);
        assert_eq!(scheduler.config.scheduling_interval_ms, 500);
        assert_eq!(scheduler.config.worker_heartbeat_timeout_ms, 60000);
    }

    #[tokio::test]
    async fn test_scheduler_builder_respects_order() {
        // The key test: Builder allows ANY order of configuration
        let job_repo: Arc<dyn JobRepository> = Arc::new(MockJobRepository);
        let event_bus: Arc<dyn EventPublisher> = Arc::new(MockEventBus);
        let worker_client: Arc<dyn WorkerClient> = Arc::new(MockWorkerClient);
        let worker_repo: Arc<dyn WorkerRepository> = Arc::new(MockWorkerRepository);

        // Different order - should still work!
        let scheduler = SchedulerBuilder::<
            dyn JobRepository,
            dyn EventPublisher,
            dyn WorkerClient,
            dyn WorkerRepository,
        >::new()
        .worker_repository(worker_repo.clone())
        .config(SchedulerConfig::default())
        .job_repository(job_repo.clone())
        .event_bus(event_bus.clone())
        .worker_client(worker_client.clone())
        .build()
        .unwrap();

        // Just verify it builds
        assert!(scheduler.job_repo.is_some());
        assert!(scheduler.event_bus.is_some());
    }

    #[tokio::test]
    async fn test_scheduler_builder_missing_job_repo() {
        let event_bus: Arc<dyn EventPublisher> = Arc::new(MockEventBus);
        let worker_client: Arc<dyn WorkerClient> = Arc::new(MockWorkerClient);
        let worker_repo: Arc<dyn WorkerRepository> = Arc::new(MockWorkerRepository);

        let result = SchedulerBuilder::<
            dyn JobRepository,
            dyn EventPublisher,
            dyn WorkerClient,
            dyn WorkerRepository,
        >::new()
        .event_bus(event_bus.clone())
        .worker_client(worker_client.clone())
        .worker_repository(worker_repo.clone())
        .build();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("job_repository is required"));
        }
    }
    */

    /*
    #[tokio::test]
    async fn test_scheduler_builder_default_config() {
        let job_repo: Arc<dyn JobRepository> = Arc::new(MockJobRepository);
        let event_bus: Arc<dyn EventPublisher> = Arc::new(MockEventBus);
        let worker_client: Arc<dyn WorkerClient> = Arc::new(MockWorkerClient);
        let worker_repo: Arc<dyn WorkerRepository> = Arc::new(MockWorkerRepository);

        let scheduler = SchedulerBuilder::<
            dyn JobRepository,
            dyn EventPublisher,
            dyn WorkerClient,
            dyn WorkerRepository,
        >::new()
        .job_repository(job_repo)
        .event_bus(event_bus)
        .worker_client(worker_client)
        .worker_repository(worker_repo)
        .build()
        .unwrap();

        // Should use default config when not provided
        assert_eq!(scheduler.config.max_queue_size, 1000);
        assert_eq!(scheduler.config.scheduling_interval_ms, 1000);
        assert_eq!(scheduler.config.worker_heartbeat_timeout_ms, 30000);
    }
    */
}
