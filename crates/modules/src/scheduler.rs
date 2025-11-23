use dashmap::DashMap;
use hodei_core::{Job, JobId, Worker, WorkerCapabilities, WorkerId};
use hodei_ports::{
    EventPublisher, JobRepository, JobRepositoryError, WorkerClient, WorkerRepository,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_queue_size: usize,
    pub scheduling_interval_ms: u64,
    pub worker_heartbeat_timeout_ms: u64,
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
    tenant_id: Option<String>,
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
    job_repo: Arc<R>,
    event_bus: Arc<E>,
    worker_client: Arc<W>,
    worker_repo: Arc<WR>,
    config: SchedulerConfig,
    queue: Arc<RwLock<std::collections::BinaryHeap<QueueEntry>>>,
    cluster_state: Arc<ClusterState>,
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
            tenant_id: None,
        };

        self.queue.write().await.push(entry);
        self.run_scheduling_cycle().await?;

        Ok(())
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
        _job: &Job,
    ) -> Result<Worker, SchedulerError> {
        if workers.is_empty() {
            return Err(SchedulerError::NoEligibleWorkers);
        }

        Ok(workers[0].clone())
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
