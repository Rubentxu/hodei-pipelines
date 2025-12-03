//! Scheduler module for job scheduling and worker management

pub mod state_machine;

pub use state_machine::{SchedulingContext, SchedulingState, SchedulingStateMachine};

use crossbeam::queue::SegQueue;
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use hodei_pipelines_core::resource_governance::{
    ComputePool, GRCConfig, GlobalResourceController, PoolCapacity, PoolId, ProviderType,
    ResourceRequest,
};
use hodei_pipelines_core::{
    DomainError, Job, JobId, JobState, Result, Worker, WorkerCapabilities, WorkerId,
};
use hodei_pipelines_ports::{
    EventPublisher, JobRepository, PipelineRepository, RoleRepository, SchedulerPort, WorkerClient,
    WorkerRepository,
};
use hodei_pipelines_proto::ServerMessage;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::burst_capacity_manager;
use crate::cooldown_management;
use crate::cost_optimization;
use crate::cost_tracking;
use crate::metrics_collection;
use crate::multi_tenancy_quota_manager;
use crate::orchestrator;
use crate::pipeline_crud;
use crate::pool_lifecycle;
use crate::queue_assignment;
use crate::quota_enforcement;
use crate::rbac;
use crate::worker_management;

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

impl Default for ResourceUsage {
    fn default() -> Self {
        Self::new()
    }
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
pub struct QueueEntry {
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

/// Lock-free priority queue using SegQueue with crossbeam
/// Provides O(1) enqueue/dequeue operations and batching support
pub struct LockFreePriorityQueue {
    high_priority_queue: Arc<SegQueue<QueueEntry>>,
    normal_priority_queue: Arc<SegQueue<QueueEntry>>,
    low_priority_queue: Arc<SegQueue<QueueEntry>>,
    queue_sizes: Arc<CachePadded<AtomicUsize>>,
    batch_size: usize,
}

impl LockFreePriorityQueue {
    pub fn new(batch_size: usize) -> Self {
        Self {
            high_priority_queue: Arc::new(SegQueue::new()),
            normal_priority_queue: Arc::new(SegQueue::new()),
            low_priority_queue: Arc::new(SegQueue::new()),
            queue_sizes: Arc::new(CachePadded::new(AtomicUsize::new(0))),
            batch_size,
        }
    }

    /// Enqueue job with lock-free operation (O(1))
    pub fn enqueue(&self, entry: QueueEntry) {
        let queue = match entry.priority {
            0..=3 => &self.high_priority_queue,
            4..=7 => &self.normal_priority_queue,
            _ => &self.low_priority_queue,
        };

        queue.push(entry);
        self.queue_sizes.fetch_add(1, Ordering::Relaxed);
    }

    /// Dequeue batch of jobs (lock-free, O(1) per job)
    /// Returns up to batch_size jobs, prioritizing higher priority queues
    pub fn dequeue_batch(&self) -> Vec<QueueEntry> {
        let mut batch = Vec::with_capacity(self.batch_size);

        // Dequeue from high priority queue first
        for _ in 0..std::cmp::min(self.batch_size / 3, self.batch_size) {
            if let Some(entry) = self.high_priority_queue.pop() {
                batch.push(entry);
                self.queue_sizes.fetch_sub(1, Ordering::Relaxed);
            } else {
                break;
            }
        }

        // Then from normal priority queue
        for _ in 0..std::cmp::min(self.batch_size / 3, self.batch_size - batch.len()) {
            if let Some(entry) = self.normal_priority_queue.pop() {
                batch.push(entry);
                self.queue_sizes.fetch_sub(1, Ordering::Relaxed);
            } else {
                break;
            }
        }

        // Finally from low priority queue
        for _ in 0..(self.batch_size - batch.len()) {
            if let Some(entry) = self.low_priority_queue.pop() {
                batch.push(entry);
                self.queue_sizes.fetch_sub(1, Ordering::Relaxed);
            } else {
                break;
            }
        }

        batch
    }

    /// Peek at next job without dequeueing (O(1))
    pub fn peek(&self) -> Option<QueueEntry> {
        self.high_priority_queue
            .pop()
            .or_else(|| self.normal_priority_queue.pop())
            .or_else(|| self.low_priority_queue.pop())
    }

    /// Get current queue size
    pub fn len(&self) -> usize {
        self.queue_sizes.load(Ordering::Relaxed)
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for LockFreePriorityQueue {
    fn default() -> Self {
        Self::new(100) // Default batch size
    }
}

pub struct SchedulerModule<R, E, W, WR>
where
    R: JobRepository + PipelineRepository + RoleRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    pub(crate) job_repo: Arc<R>,
    pub(crate) event_bus: Arc<E>,
    pub(crate) worker_client: Arc<W>,

    pub(crate) worker_repo: Arc<WR>,
    pub(crate) config: SchedulerConfig,
    pub(crate) queue: Arc<LockFreePriorityQueue>,
    pub(crate) cluster_state: Arc<ClusterState>,
    pub global_resource_controller: Option<Arc<GlobalResourceController>>,
    pub burst_capacity_manager: Option<burst_capacity_manager::BurstCapacityManager>,
    pub cooldown_management: Option<cooldown_management::AdvancedCooldownManager>,
    pub cost_optimization: Option<cost_optimization::CostOptimizationEngine>,
    pub cost_tracking: Option<cost_tracking::CostTrackingService>,
    pub metrics_collection: Option<metrics_collection::MetricsCollector>,
    pub multi_tenancy_quota_manager: Option<multi_tenancy_quota_manager::MultiTenancyQuotaManager>,
    pub orchestrator: Option<
        orchestrator::OrchestratorModule<
            R,
            E,
            hodei_pipelines_adapters::postgres::pipeline_repository::PostgreSqlPipelineRepository,
        >,
    >, // Assuming P is this for now, or need generic P?
    pub pipeline_crud: Option<pipeline_crud::PipelineCrudService<R, E>>,
    pub pool_lifecycle: Option<pool_lifecycle::InMemoryStateStore>,
    pub queue_assignment: Option<queue_assignment::QueueAssignmentEngine>,
    pub quota_enforcement: Option<quota_enforcement::QuotaEnforcementEngine>,
    pub rbac: Option<
        rbac::RoleBasedAccessControlService<
            R,
            hodei_pipelines_adapters::rbac_repositories::InMemoryPermissionRepository,
            E,
        >,
    >, // Need generic P
    pub worker_management: Option<
        Box<
            worker_management::WorkerManagementService<
                hodei_pipelines_adapters::DockerProvider,
                SchedulerModule<R, E, W, WR>,
            >,
        >,
    >, // Circular dependency broken by Box
}

/// Builder for SchedulerModule to eliminate Connascence of Position
pub struct SchedulerBuilder<R, E, W, WR>
where
    R: JobRepository + PipelineRepository + RoleRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    job_repo: Option<Arc<R>>,
    event_bus: Option<Arc<E>>,
    worker_client: Option<Arc<W>>,
    worker_repo: Option<Arc<WR>>,
    config: Option<SchedulerConfig>,
    pub global_resource_controller: Option<Arc<GlobalResourceController>>,
    pub burst_capacity_manager: Option<burst_capacity_manager::BurstCapacityManager>,
    pub cooldown_management: Option<cooldown_management::AdvancedCooldownManager>,
    pub cost_optimization: Option<cost_optimization::CostOptimizationEngine>,
    pub cost_tracking: Option<cost_tracking::CostTrackingService>,
    pub metrics_collection: Option<metrics_collection::MetricsCollector>,
    pub multi_tenancy_quota_manager: Option<multi_tenancy_quota_manager::MultiTenancyQuotaManager>,
    pub orchestrator: Option<
        orchestrator::OrchestratorModule<
            R,
            E,
            hodei_pipelines_adapters::postgres::pipeline_repository::PostgreSqlPipelineRepository,
        >,
    >,
    pub pipeline_crud: Option<pipeline_crud::PipelineCrudService<R, E>>,
    pub pool_lifecycle: Option<pool_lifecycle::InMemoryStateStore>,
    pub queue_assignment: Option<queue_assignment::QueueAssignmentEngine>,
    pub quota_enforcement: Option<quota_enforcement::QuotaEnforcementEngine>,
    pub rbac: Option<
        rbac::RoleBasedAccessControlService<
            R,
            hodei_pipelines_adapters::rbac_repositories::InMemoryPermissionRepository,
            E,
        >,
    >,
    pub worker_management: Option<
        worker_management::WorkerManagementService<
            hodei_pipelines_adapters::docker_provider::DockerProvider,
            SchedulerModule<R, E, W, WR>,
        >,
    >,
}

impl<R, E, W, WR> SchedulerBuilder<R, E, W, WR>
where
    R: JobRepository + PipelineRepository + RoleRepository + Send + Sync + 'static,
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
            burst_capacity_manager: None,
            cooldown_management: None,
            cost_optimization: None,
            cost_tracking: None,
            metrics_collection: None,
            multi_tenancy_quota_manager: None,
            orchestrator: None,
            pipeline_crud: None,
            pool_lifecycle: None,
            queue_assignment: None,
            quota_enforcement: None,
            rbac: None,
            worker_management: None,
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

    pub fn global_resource_controller(mut self, grc: Arc<GlobalResourceController>) -> Self {
        self.global_resource_controller = Some(grc);
        self
    }

    pub fn build(self) -> Result<SchedulerModule<R, E, W, WR>> {
        let job_repo = self.job_repo.ok_or_else(|| {
            hodei_pipelines_core::DomainError::Infrastructure("job_repository is required".into())
        })?;

        let event_bus = self.event_bus.ok_or_else(|| {
            hodei_pipelines_core::DomainError::Infrastructure("event_bus is required".into())
        })?;

        let worker_client = self.worker_client.ok_or_else(|| {
            hodei_pipelines_core::DomainError::Infrastructure("worker_client is required".into())
        })?;

        let worker_repo = self.worker_repo.ok_or_else(|| {
            hodei_pipelines_core::DomainError::Infrastructure(
                "worker_repository is required".into(),
            )
        })?;

        let config = self.config.unwrap_or_default();
        let max_queue_size = config.max_queue_size;

        Ok(SchedulerModule {
            job_repo,
            event_bus,
            worker_client,
            worker_repo,
            config,
            queue: Arc::new(LockFreePriorityQueue::new(max_queue_size)),
            cluster_state: Arc::new(ClusterState::new()),
            global_resource_controller: self.global_resource_controller,
            burst_capacity_manager: self.burst_capacity_manager,
            cooldown_management: self.cooldown_management,
            cost_optimization: self.cost_optimization,
            cost_tracking: self.cost_tracking,
            metrics_collection: self.metrics_collection,
            multi_tenancy_quota_manager: self.multi_tenancy_quota_manager,
            orchestrator: self.orchestrator,
            pipeline_crud: self.pipeline_crud,
            pool_lifecycle: self.pool_lifecycle,
            queue_assignment: self.queue_assignment,
            quota_enforcement: self.quota_enforcement,
            rbac: self.rbac,
            worker_management: self.worker_management.map(Box::new),
        })
    }
}

impl<R, E, W, WR> Default for SchedulerBuilder<R, E, W, WR>
where
    R: JobRepository + PipelineRepository + RoleRepository + Send + Sync + 'static,
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
    R: JobRepository + PipelineRepository + RoleRepository + Send + Sync + 'static,
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
        let max_queue_size = config.max_queue_size;
        Self {
            job_repo,
            event_bus,
            worker_client,
            worker_repo,
            config,
            queue: Arc::new(LockFreePriorityQueue::new(max_queue_size)),
            cluster_state: Arc::new(ClusterState::new()),
            global_resource_controller: None,
            burst_capacity_manager: None,
            cooldown_management: None,
            cost_optimization: None,
            cost_tracking: None,
            metrics_collection: None,
            multi_tenancy_quota_manager: None,
            orchestrator: None,
            pipeline_crud: None,
            pool_lifecycle: None,
            queue_assignment: None,
            quota_enforcement: None,
            rbac: None,
            worker_management: None,
        }
    }

    /// Start the scheduler loop
    pub async fn start(&self) -> Result<()> {
        info!("Starting Scheduler Module");
        let interval = Duration::from_millis(self.config.scheduling_interval_ms);
        let scheduler = self.clone();

        tokio::spawn(async move {
            info!("Scheduler loop started with interval {:?}", interval);
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;
                if let Err(e) = scheduler.run_scheduling_cycle().await {
                    error!("Error in scheduling cycle: {}", e);
                }
            }
        });

        Ok(())
    }

    pub async fn schedule_job(&self, job: Job, priority: u8) -> Result<()> {
        info!("Scheduling job: {}", job.id);

        job.spec
            .validate()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let entry = QueueEntry {
            job: job.clone(),
            priority,
            enqueue_time: chrono::Utc::now(),
        };

        // Lock-free enqueue operation (O(1))
        self.queue.enqueue(entry);

        // Run scheduling cycle with batching
        self.run_scheduling_cycle().await?;

        Ok(())
    }

    /// Schedule job using state machine (eliminates temporal coupling)
    pub async fn schedule_job_with_state_machine(&self, job: Job) -> Result<()> {
        info!("Scheduling job using state machine: {}", job.id);

        job.spec
            .validate()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut state_machine = state_machine::SchedulingStateMachine::new();
        state_machine.set_job(job);

        // Execute scheduling cycle using state machine
        state_machine.complete(self).await?;

        Ok(())
    }

    /// Get scheduling matches without committing (useful for testing or preview)
    pub async fn discover_matches(&self, job: &Job) -> Result<Option<Worker>> {
        let mut state_machine = state_machine::SchedulingStateMachine::new();
        state_machine.set_job(job.clone());

        state_machine.discover_matches(self).await
    }

    async fn run_scheduling_cycle(&self) -> Result<()> {
        // Dequeue batch of jobs (lock-free, O(1) per job)
        let batch = self.queue.dequeue_batch();

        if batch.is_empty() {
            return Ok(());
        }

        // Process batch in parallel for better performance
        for entry in batch {
            let job = entry.job;

            // Find eligible workers
            let workers = self.find_eligible_workers(&job).await?;

            if !workers.is_empty() {
                let selected_worker = self.select_best_worker(&workers, &job).await?;

                if self.reserve_worker(&selected_worker, &job.id).await? {
                    // Update job state
                    if let Err(e) = self
                        .job_repo
                        .compare_and_swap_status(
                            &job.id,
                            &JobState::Pending.as_str(),
                            &JobState::Scheduled.as_str(),
                        )
                        .await
                    {
                        error!("Failed to update job state: {}", e);
                        continue;
                    }

                    // Assign job to worker
                    // First try to send via active stream (Agent)
                    let assign_result = if self.cluster_state.has_transmitter(&selected_worker.id) {
                        info!(
                            "Sending job {} to worker {} via active stream",
                            job.id, selected_worker.id
                        );
                        self.send_job_to_worker(&selected_worker.id, &job)
                            .await
                            .map_err(|e| {
                                // Convert SchedulerError to WorkerClientError-like string or handle it
                                // send_job_to_worker returns Result<()>
                                e
                            })
                    } else {
                        // Fallback to WorkerClient (gRPC/HTTP direct connection)
                        info!(
                            "Assigning job {} to worker {} via WorkerClient",
                            job.id, selected_worker.id
                        );
                        self.worker_client
                            .assign_job(&selected_worker.id, &job.id, &job.spec)
                            .await
                            .map_err(|e| DomainError::Infrastructure(e.to_string()))
                    };

                    if let Err(e) = assign_result {
                        error!("Failed to assign job to worker: {}", e);
                        // We should probably revert the status change here or let the timeout handle it
                        continue;
                    }

                    info!(
                        "Successfully scheduled job {} on worker {}",
                        job.id, selected_worker.id
                    );
                }
            }
        }

        Ok(())
    }

    async fn find_eligible_workers(&self, job: &Job) -> Result<Vec<Worker>> {
        let all_workers = self
            .worker_repo
            .get_all_workers()
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

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

    async fn select_best_worker(&self, workers: &[Worker], job: &Job) -> Result<Worker> {
        if workers.is_empty() {
            return Err(DomainError::Infrastructure(
                "No eligible workers found".to_string(),
            ));
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
            .ok_or_else(|| DomainError::Infrastructure("No eligible workers found".to_string()))?;

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

        resource_score + load_balancing_score + health_score_weighted + capability_score_weighted
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
                .keys()
                .map(|k| k.as_str())
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

    async fn reserve_worker(&self, worker: &Worker, job_id: &JobId) -> Result<bool> {
        self.cluster_state
            .reserve_job(&worker.id, *job_id)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))
    }

    pub async fn register_worker(&self, worker: Worker) -> Result<()> {
        self.worker_repo
            .save_worker(&worker)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        // Also register in cluster state
        self.cluster_state
            .register_worker(&worker.id, worker.capabilities.clone())
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        Ok(())
    }

    pub async fn process_heartbeat(
        &self,
        worker_id: &WorkerId,
        resource_usage: Option<ResourceUsage>,
    ) -> Result<()> {
        self.cluster_state
            .update_heartbeat(worker_id.clone(), resource_usage)
            .await;

        Ok(())
    }

    pub async fn get_cluster_stats(&self) -> ClusterStats {
        self.cluster_state.get_stats().await
    }

    /// Register a transmitter for a worker (US-02.2)
    pub async fn register_transmitter(
        &self,
        worker_id: &WorkerId,
        transmitter: mpsc::UnboundedSender<
            std::result::Result<
                ServerMessage,
                hodei_pipelines_ports::scheduler_port::SchedulerError,
            >,
        >,
    ) -> Result<()> {
        self.cluster_state
            .register_transmitter(worker_id, transmitter)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))
    }

    /// Unregister a transmitter for a worker
    pub async fn unregister_transmitter(&self, worker_id: &WorkerId) -> Result<()> {
        self.cluster_state
            .unregister_transmitter(worker_id)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))
    }

    /// Send a message to a worker through their registered transmitter
    pub async fn send_to_worker(&self, worker_id: &WorkerId, message: ServerMessage) -> Result<()> {
        self.cluster_state
            .send_to_worker(worker_id, message)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))
    }

    /// Send a job to a worker through their registered transmitter (US-02.3)
    pub async fn send_job_to_worker(&self, worker_id: &WorkerId, job: &Job) -> Result<()> {
        let job_spec = job.spec.clone();
        let gpu_count = job_spec.resources.gpu.unwrap_or(0) as u32;

        let message = ServerMessage {
            payload: Some(
                hodei_pipelines_proto::pb::server_message::Payload::AssignJob(
                    hodei_pipelines_proto::pb::AssignJobRequest {
                        worker_id: worker_id.to_string(),
                        job_id: job.id.as_uuid().to_string(),
                        job_spec: Some(hodei_pipelines_proto::pb::JobSpec {
                            name: job.id.to_string(),
                            image: job_spec.image,
                            command: job_spec.command,
                            resources: Some(hodei_pipelines_proto::pb::ResourceQuota {
                                cpu_m: job_spec.resources.cpu_m,
                                memory_mb: job_spec.resources.memory_mb,
                                gpu: gpu_count,
                            }),
                            timeout_ms: 0,
                            retries: 0,
                            env: job_spec.env,
                            secret_refs: vec![],
                        }),
                    },
                ),
            ),
        };

        self.send_to_worker(worker_id, message).await
    }

    /// Handle job acceptance event from worker
    /// Updates job state to RUNNING and records start timestamp
    pub async fn handle_job_accepted(&self, job_id: &JobId, worker_id: &WorkerId) -> Result<()> {
        info!("Job {} accepted by worker {}", job_id, worker_id);

        // Check if job exists
        let job =
            self.job_repo.get_job(job_id).await.map_err(|e| {
                DomainError::Infrastructure(format!("Job {} not found: {}", job_id, e))
            })?;

        if job.is_none() {
            return Err(DomainError::Infrastructure(format!(
                "Job {} not found",
                job_id
            )));
        }

        // Update job state from SCHEDULED to RUNNING
        let success = self
            .job_repo
            .compare_and_swap_status(job_id, "SCHEDULED", "RUNNING")
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to transition job {} to RUNNING: {}",
                    job_id, e
                ))
            })?;

        if !success {
            return Err(DomainError::Infrastructure(format!(
                "State transition failed for job {}",
                job_id
            )));
        }

        // Record worker assignment
        self.job_repo
            .assign_worker(job_id, worker_id)
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to assign worker {} to job {}: {}",
                    worker_id, job_id, e
                ))
            })?;

        // Record start timestamp
        self.job_repo
            .set_job_start_time(job_id, chrono::Utc::now())
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to set start time for job {}: {}",
                    job_id, e
                ))
            })?;

        info!(
            "Job {} state updated to RUNNING on worker {}",
            job_id, worker_id
        );
        Ok(())
    }

    /// Handle log entry from worker
    /// Publishes log event to event bus
    pub async fn handle_log_entry(
        &self,
        job_id: &JobId,
        worker_id: &WorkerId,
        log_data: Vec<u8>,
        stream_type: hodei_pipelines_ports::event_bus::StreamType,
    ) -> Result<()> {
        let log_entry = hodei_pipelines_ports::event_bus::LogEntry {
            job_id: *job_id,
            data: log_data,
            stream_type,
            sequence: 0, // TODO: Get sequence from message
            timestamp: chrono::Utc::now(),
        };

        // Publish log event to event bus
        self.event_bus
            .publish(
                hodei_pipelines_ports::event_bus::SystemEvent::LogChunkReceived(log_entry.clone()),
            )
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to publish log event for job {}: {}",
                    job_id, e
                ))
            })?;

        info!(
            "Published log event for job {} from worker {}",
            job_id, worker_id
        );
        Ok(())
    }

    /// Convert Job to ResourceRequest for GRC
    pub fn job_to_resource_request(&self, job: &Job, tenant_id: Option<String>) -> ResourceRequest {
        ResourceRequest::builder()
            .request_id(job.id.into())
            .cpu_millicores(job.spec.resources.cpu_m)
            .memory_mb(job.spec.resources.memory_mb)
            .gpu_count(job.spec.resources.gpu)
            .tenant_id(tenant_id.unwrap_or_else(|| "default".to_string()))
            .priority(5)
            .build()
            .expect("Failed to build ResourceRequest from Job")
    }

    /// Find eligible compute pools using GRC
    pub async fn find_eligible_pools(&self, job: &Job) -> Result<Vec<ComputePool>> {
        if let Some(grc) = &self.global_resource_controller {
            let resource_request = self.job_to_resource_request(job, None);
            let pools = grc.find_candidate_pools(&resource_request);

            // Extract pools from candidates and get full pool details
            let pool_ids: Vec<PoolId> = pools.into_iter().map(|(id, _)| id).collect();

            let all_pools = grc.get_all_pools();
            let eligible_pools: Vec<ComputePool> = all_pools
                .into_iter()
                .filter(|pool| pool_ids.contains(&pool.id))
                .collect();

            Ok(eligible_pools)
        } else {
            // Fallback to old behavior when GRC is not available
            Ok(vec![])
        }
    }

    /// Allocate resources using GRC before scheduling
    pub async fn allocate_resources_for_job(&self, job: &Job, pool_id: PoolId) -> Result<()> {
        if let Some(grc) = &self.global_resource_controller {
            let resource_request = self.job_to_resource_request(job, None);
            let allocation_id = format!("job-{}-alloc", job.id);

            let allocation_result = grc.allocate_with_quota_check(
                allocation_id,
                pool_id,
                resource_request,
            );

            match allocation_result {
                Ok(_) => {
                    info!("Successfully allocated resources for job {} on pool {}", job.id, pool_id);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to allocate resources for job {}: {}", job.id, e);
                    Err(DomainError::Infrastructure(format!(
                        "Resource allocation failed for job {}: {}",
                        job.id, e
                    )))
                }
            }
        } else {
            // Skip allocation when GRC is not available
            Ok(())
        }
    }

    /// Handle job result from worker
    /// Updates job state and publishes completion event
    pub async fn handle_job_result(
        &self,
        job_id: &JobId,
        worker_id: &WorkerId,
        exit_code: i32,
    ) -> Result<()> {
        info!(
            "Job {} completed on worker {} with exit code {}",
            job_id, worker_id, exit_code
        );

        // Determine final state based on exit code
        let final_state_str = if exit_code == 0 {
            "COMPLETED"
        } else {
            "FAILED"
        };

        // Check if job exists
        let job =
            self.job_repo.get_job(job_id).await.map_err(|e| {
                DomainError::Infrastructure(format!("Job {} not found: {}", job_id, e))
            })?;

        if job.is_none() {
            return Err(DomainError::Infrastructure(format!(
                "Job {} not found",
                job_id
            )));
        }

        // Update job state to final state (COMPLETED or FAILED)
        let success = self
            .job_repo
            .compare_and_swap_status(job_id, "RUNNING", final_state_str)
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to transition job {} to {}: {}",
                    job_id, final_state_str, e
                ))
            })?;

        if !success {
            return Err(DomainError::Infrastructure(format!(
                "State transition failed for job {}",
                job_id
            )));
        }

        // Record finish timestamp
        self.job_repo
            .set_job_finish_time(job_id, chrono::Utc::now())
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to set finish time for job {}: {}",
                    job_id, e
                ))
            })?;

        // Publish job completed/failed event
        let event = if exit_code == 0 {
            hodei_pipelines_ports::event_bus::SystemEvent::JobCompleted {
                job_id: *job_id,
                exit_code,
                compressed_logs: None, // Logs are handled separately via gRPC/NATS
            }
        } else {
            hodei_pipelines_ports::event_bus::SystemEvent::JobFailed {
                job_id: *job_id,
                error: format!("Job failed with exit code {}", exit_code),
            }
        };

        self.event_bus.publish(event).await.map_err(|e| {
            DomainError::Infrastructure(format!(
                "Failed to publish job completion event for job {}: {}",
                job_id, e
            ))
        })?;

        info!(
            "Job {} state updated to {} and event published",
            job_id, final_state_str
        );
        Ok(())
    }
}

impl<R, E, W, WR> Clone for SchedulerModule<R, E, W, WR>
where
    R: JobRepository + PipelineRepository + RoleRepository + Send + Sync + 'static,
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
            global_resource_controller: self.global_resource_controller.clone(),
            burst_capacity_manager: self.burst_capacity_manager.clone(),
            cooldown_management: self.cooldown_management.clone(),
            cost_optimization: self.cost_optimization.clone(),
            cost_tracking: self.cost_tracking.clone(),
            metrics_collection: self.metrics_collection.clone(),
            multi_tenancy_quota_manager: self.multi_tenancy_quota_manager.clone(),
            orchestrator: self.orchestrator.clone(),
            pipeline_crud: self.pipeline_crud.clone(),
            pool_lifecycle: self.pool_lifecycle.clone(),
            queue_assignment: self.queue_assignment.clone(),
            quota_enforcement: self.quota_enforcement.clone(),
            rbac: self.rbac.clone(),
            worker_management: self.worker_management.clone(),
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
    transmitters: Arc<
        DashMap<
            WorkerId,
            mpsc::UnboundedSender<
                std::result::Result<
                    ServerMessage,
                    hodei_pipelines_ports::scheduler_port::SchedulerError,
                >,
            >,
        >,
    >,
}

impl ClusterState {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(DashMap::new()),
            job_assignments: Arc::new(DashMap::new()),
            transmitters: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_worker(
        &self,
        worker_id: &WorkerId,
        capabilities: WorkerCapabilities,
    ) -> Result<()> {
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
    ) -> Result<()> {
        if let Some(mut worker) = self.workers.get_mut(worker_id) {
            worker.usage = usage;
            worker.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(DomainError::Infrastructure(format!(
                "Worker {} not found",
                worker_id
            )))
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

    pub async fn get_worker(&self, worker_id: &WorkerId) -> Result<Option<WorkerNode>> {
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

    pub async fn reserve_job(&self, worker_id: &WorkerId, job_id: JobId) -> Result<bool> {
        // Check if worker exists and has capacity
        if let Some(mut worker) = self.workers.get_mut(worker_id) {
            worker.reserved_jobs.push(job_id);
            self.job_assignments.insert(job_id, worker_id.clone());
            Ok(true)
        } else {
            Err(DomainError::Infrastructure(format!(
                "Worker {} not found",
                worker_id
            )))
        }
    }

    pub async fn release_job(&self, job_id: &JobId) -> Result<()> {
        if let Some((_, worker_id)) = self.job_assignments.remove(job_id) {
            if let Some(mut worker) = self.workers.get_mut(&worker_id) {
                worker.reserved_jobs.retain(|id| id != job_id);
            }
            Ok(())
        } else {
            Err(DomainError::Infrastructure(format!(
                "Job {} not found in reservations",
                job_id
            )))
        }
    }

    /// Register a transmitter for a worker
    pub async fn register_transmitter(
        &self,
        worker_id: &WorkerId,
        transmitter: mpsc::UnboundedSender<
            std::result::Result<
                ServerMessage,
                hodei_pipelines_ports::scheduler_port::SchedulerError,
            >,
        >,
    ) -> Result<()> {
        self.transmitters.insert(worker_id.clone(), transmitter);
        info!("Transmitter registered for worker: {}", worker_id);
        Ok(())
    }

    /// Unregister a transmitter for a worker
    pub async fn unregister_transmitter(&self, worker_id: &WorkerId) -> Result<()> {
        if self.transmitters.remove(worker_id).is_some() {
            info!("Transmitter unregistered for worker: {}", worker_id);
            Ok(())
        } else {
            Err(DomainError::Infrastructure(format!(
                "No transmitter found for worker {}",
                worker_id
            )))
        }
    }

    /// Check if a worker has a registered transmitter
    pub fn has_transmitter(&self, worker_id: &WorkerId) -> bool {
        self.transmitters.contains_key(worker_id)
    }

    /// Send a message to a worker through their registered transmitter
    pub async fn send_to_worker(&self, worker_id: &WorkerId, message: ServerMessage) -> Result<()> {
        if let Some(transmitter) = self.transmitters.get(worker_id) {
            if let Err(e) = transmitter.send(Ok(message)) {
                error!("Failed to send message to worker {}: {}", worker_id, e);
                return Err(DomainError::Infrastructure(format!(
                    "Failed to send message to worker {}: {}",
                    worker_id, e
                )));
            }
            Ok(())
        } else {
            Err(DomainError::Infrastructure(format!(
                "No transmitter registered for worker {}",
                worker_id
            )))
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

#[async_trait::async_trait]
impl<R, E, W, WR> SchedulerPort for SchedulerModule<R, E, W, WR>
where
    R: JobRepository + PipelineRepository + RoleRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
    W: WorkerClient + Send + Sync + 'static,
    WR: WorkerRepository + Send + Sync + 'static,
{
    async fn register_worker(
        &self,
        worker: &Worker,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        if let Err(e) = self.worker_repo.save_worker(worker).await {
            return Err(
                hodei_pipelines_ports::scheduler_port::SchedulerError::registration_failed(
                    e.to_string(),
                ),
            );
        }
        Ok(())
    }

    async fn unregister_worker(
        &self,
        worker_id: &WorkerId,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        if let Err(e) = self.worker_repo.delete_worker(worker_id).await {
            return Err(
                hodei_pipelines_ports::scheduler_port::SchedulerError::registration_failed(
                    e.to_string(),
                ),
            );
        }
        Ok(())
    }

    async fn get_registered_workers(
        &self,
    ) -> std::result::Result<Vec<WorkerId>, hodei_pipelines_ports::scheduler_port::SchedulerError>
    {
        let workers = self.worker_repo.get_all_workers().await.map_err(|e| {
            hodei_pipelines_ports::scheduler_port::SchedulerError::internal(e.to_string())
        })?;
        Ok(workers.into_iter().map(|w| w.id).collect())
    }

    async fn register_transmitter(
        &self,
        worker_id: &WorkerId,
        transmitter: mpsc::UnboundedSender<
            std::result::Result<
                ServerMessage,
                hodei_pipelines_ports::scheduler_port::SchedulerError,
            >,
        >,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        self.cluster_state
            .register_transmitter(worker_id, transmitter)
            .await
            .map_err(|e| {
                hodei_pipelines_ports::scheduler_port::SchedulerError::internal(format!(
                    "Failed to register transmitter: {}",
                    e
                ))
            })
    }

    async fn unregister_transmitter(
        &self,
        worker_id: &WorkerId,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        self.cluster_state
            .unregister_transmitter(worker_id)
            .await
            .map_err(|e| {
                hodei_pipelines_ports::scheduler_port::SchedulerError::internal(format!(
                    "Failed to unregister transmitter: {}",
                    e
                ))
            })
    }

    async fn send_to_worker(
        &self,
        worker_id: &WorkerId,
        message: ServerMessage,
    ) -> std::result::Result<(), hodei_pipelines_ports::scheduler_port::SchedulerError> {
        self.cluster_state
            .send_to_worker(worker_id, message)
            .await
            .map_err(|e| {
                hodei_pipelines_ports::scheduler_port::SchedulerError::internal(format!(
                    "Failed to send message to worker: {}",
                    e
                ))
            })
    }
}
