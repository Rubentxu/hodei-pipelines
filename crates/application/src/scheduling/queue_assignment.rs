//! Queue Assignment Engine Module
//!
//! This module provides intelligent job assignment to workers across
//! multiple resource pools with priority-based queuing and scheduling policies.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use hodei_pipelines_domain::{JobId, Result, Worker, WorkerId};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Job requirements
#[derive(Debug, Clone)]
pub struct JobRequirements {
    pub min_cpu_cores: u32,
    pub min_memory_mb: u64,
    pub required_features: Vec<String>,
}

/// Default job requirements
impl Default for JobRequirements {
    fn default() -> Self {
        Self {
            min_cpu_cores: 1,
            min_memory_mb: 1024,
            required_features: Vec::new(),
        }
    }
}

/// Queue priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Queue type
#[derive(Debug, Clone)]
pub enum QueueType {
    Default,
    HighPriority,
    LowPriority,
    Bulk,
    Urgent,
}

/// Queued job representation
#[derive(Debug, Clone)]
pub struct QueuedJob {
    pub job_id: JobId,
    pub requirements: JobRequirements,
    pub priority: u8, // 1-10, higher = more priority
    pub tenant_id: String,
    pub queue_type: QueueType,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub max_wait_time: Duration,
}

/// Assignment request
#[derive(Debug, Clone)]
pub struct AssignmentRequest {
    pub job_id: JobId,
    pub requirements: JobRequirements,
    pub priority: u8, // 1-10
    pub tenant_id: String,
    pub queue_type: QueueType,
    pub max_wait_time: Duration,
}

/// Assignment result
#[derive(Debug, Clone)]
pub enum AssignmentResult {
    Assigned {
        worker_id: WorkerId,
        pool_id: String,
        assignment_time: Duration,
    },
    Queued {
        queue_id: String,
        position: usize,
        estimated_wait: Duration,
    },
    Failed {
        reason: String,
        can_retry: bool,
    },
}

/// Assignment statistics
#[derive(Debug, Clone)]
pub struct AssignmentStats {
    pub total_assignments: u64,
    pub successful_assignments: u64,
    pub queued_jobs: u64,
    pub failed_assignments: u64,
    pub average_assignment_time_ms: f64,
    pub average_queue_wait_time_ms: f64,
}

/// Job queue with priority support
#[derive(Debug, Clone)]
pub struct JobQueue {
    pub queue_id: String,
    pub priority: QueuePriority,
    pub jobs: Arc<RwLock<VecDeque<QueuedJob>>>,
    pub max_size: usize,
    pub metrics: QueueMetrics,
}

/// Queue metrics
#[derive(Debug)]
pub struct QueueMetrics {
    pub current_size: std::sync::atomic::AtomicU64,
    pub total_enqueued: std::sync::atomic::AtomicU64,
    pub total_dequeued: std::sync::atomic::AtomicU64,
    pub average_wait_time_ms: std::sync::atomic::AtomicU64,
    pub max_wait_time_ms: std::sync::atomic::AtomicU64,
    pub overflow_count: std::sync::atomic::AtomicU64,
}

impl Clone for QueueMetrics {
    fn clone(&self) -> Self {
        Self {
            current_size: std::sync::atomic::AtomicU64::new(
                self.current_size.load(std::sync::atomic::Ordering::Relaxed),
            ),
            total_enqueued: std::sync::atomic::AtomicU64::new(
                self.total_enqueued
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            total_dequeued: std::sync::atomic::AtomicU64::new(
                self.total_dequeued
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            average_wait_time_ms: std::sync::atomic::AtomicU64::new(
                self.average_wait_time_ms
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            max_wait_time_ms: std::sync::atomic::AtomicU64::new(
                self.max_wait_time_ms
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            overflow_count: std::sync::atomic::AtomicU64::new(
                self.overflow_count
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

impl JobQueue {
    pub fn new(queue_id: String, priority: QueuePriority, max_size: usize) -> Self {
        Self {
            queue_id,
            priority,
            jobs: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            metrics: QueueMetrics::new(),
        }
    }

    /// Enqueue a job (returns false if queue is full)
    pub async fn enqueue(&self, job: QueuedJob) -> Result<bool> {
        let mut jobs = self.jobs.write().await;

        // Check queue size limit
        if jobs.len() >= self.max_size {
            self.metrics
                .overflow_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(false);
        }

        // Insert job based on priority (higher priority first for same priority)
        let mut inserted = false;
        let mut insert_index = 0;

        for (index, queued_job) in jobs.iter().enumerate() {
            if job.priority > queued_job.priority {
                insert_index = index;
                inserted = true;
                break;
            }
            insert_index = index + 1;
        }

        if inserted {
            jobs.insert(insert_index, job);
        } else {
            jobs.push_back(job);
        }

        self.metrics
            .current_size
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .total_enqueued
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(true)
    }

    /// Dequeue the highest priority job
    pub async fn dequeue(&self) -> Option<QueuedJob> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.pop_front() {
            self.metrics
                .current_size
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            self.metrics
                .total_dequeued
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(job)
        } else {
            None
        }
    }

    /// Get current queue size
    pub async fn size(&self) -> usize {
        self.jobs.read().await.len()
    }

    /// Get queue length (alias for size)
    pub async fn get_length(&self) -> u32 {
        self.size().await as u32
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.jobs.read().await.is_empty()
    }

    /// Get queue metrics snapshot
    pub fn get_metrics(&self) -> QueueMetricsSnapshot {
        QueueMetricsSnapshot {
            current_size: self
                .metrics
                .current_size
                .load(std::sync::atomic::Ordering::Relaxed),
            total_enqueued: self
                .metrics
                .total_enqueued
                .load(std::sync::atomic::Ordering::Relaxed),
            total_dequeued: self
                .metrics
                .total_dequeued
                .load(std::sync::atomic::Ordering::Relaxed),
            average_wait_time_ms: self
                .metrics
                .average_wait_time_ms
                .load(std::sync::atomic::Ordering::Relaxed),
            max_wait_time_ms: self
                .metrics
                .max_wait_time_ms
                .load(std::sync::atomic::Ordering::Relaxed),
            overflow_count: self
                .metrics
                .overflow_count
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl Default for QueueMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueMetrics {
    pub fn new() -> Self {
        Self {
            current_size: std::sync::atomic::AtomicU64::new(0),
            total_enqueued: std::sync::atomic::AtomicU64::new(0),
            total_dequeued: std::sync::atomic::AtomicU64::new(0),
            average_wait_time_ms: std::sync::atomic::AtomicU64::new(0),
            max_wait_time_ms: std::sync::atomic::AtomicU64::new(0),
            overflow_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

/// Queue metrics snapshot
#[derive(Debug, Clone)]
pub struct QueueMetricsSnapshot {
    pub current_size: u64,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub average_wait_time_ms: u64,
    pub max_wait_time_ms: u64,
    pub overflow_count: u64,
}

/// Queue error types
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue overflow: {queue_id} is full")]
    QueueOverflow { queue_id: String },

    #[error("Queue full: {queue_id} has reached capacity {capacity}")]
    QueueFull { queue_id: String, capacity: usize },

    #[error("Queue not found: {queue_id}")]
    QueueNotFound { queue_id: String },

    #[error("Invalid priority: {priority}, must be between 1 and 10")]
    InvalidPriority { priority: u8 },

    #[error("Job not found in queue: {job_id}")]
    JobNotFound { job_id: JobId },

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Scheduling policies
#[derive(Debug, Clone)]
pub enum SchedulingPolicy {
    FIFO,                  // First In, First Out
    Priority,              // Higher priority first
    FairShare,             // Round-robin per tenant
    EarliestDeadlineFirst, // EDF for time-sensitive jobs
}

use dashmap::DashMap;

/// Assignment engine
#[derive(Clone)]
pub struct QueueAssignmentEngine {
    queues: Arc<DashMap<String, JobQueue>>,
    pools: Arc<DashMap<String, Arc<dyn ResourcePool>>>,
    scheduler_policy: SchedulingPolicy,
    stats: Arc<AssignmentStatistics>,
}

impl std::fmt::Debug for QueueAssignmentEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueAssignmentEngine")
            .field("queues", &self.queues)
            .field(
                "pool_ids",
                &self
                    .pools
                    .iter()
                    .map(|entry| entry.key().clone())
                    .collect::<Vec<_>>(),
            )
            .field("scheduler_policy", &self.scheduler_policy)
            .field("stats", &self.stats)
            .finish()
    }
}

/// Resource pool trait
#[async_trait]
pub trait ResourcePool: Send + Sync {
    async fn allocate_worker(&self, requirements: &JobRequirements) -> Result<Worker>;
    async fn get_status(&self) -> PoolStatus;
    fn pool_id(&self) -> &str;
}

/// Resource pool error
#[derive(Debug, thiserror::Error)]
pub enum ResourcePoolError {
    #[error("No suitable worker available")]
    NoWorkerAvailable,

    #[error("Requirements not met: {0}")]
    RequirementsNotMet(String),

    #[error("Pool error: {0}")]
    PoolError(String),
}

impl From<ResourcePoolError> for hodei_pipelines_domain::DomainError {
    fn from(error: ResourcePoolError) -> Self {
        hodei_pipelines_domain::DomainError::Infrastructure(error.to_string())
    }
}

/// Pool status
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub pool_id: String,
    pub available_workers: u32,
    pub busy_workers: u32,
    pub total_workers: u32,
}

/// Assignment statistics
#[derive(Debug)]
pub struct AssignmentStatistics {
    pub total_assignments: std::sync::atomic::AtomicU64,
    pub successful_assignments: std::sync::atomic::AtomicU64,
    pub queued_jobs: std::sync::atomic::AtomicU64,
    pub failed_assignments: std::sync::atomic::AtomicU64,
    pub total_assignment_time_ms: std::sync::atomic::AtomicU64,
    pub total_queue_wait_time_ms: std::sync::atomic::AtomicU64,
}

impl Clone for AssignmentStatistics {
    fn clone(&self) -> Self {
        Self {
            total_assignments: std::sync::atomic::AtomicU64::new(
                self.total_assignments
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            successful_assignments: std::sync::atomic::AtomicU64::new(
                self.successful_assignments
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            queued_jobs: std::sync::atomic::AtomicU64::new(
                self.queued_jobs.load(std::sync::atomic::Ordering::Relaxed),
            ),
            failed_assignments: std::sync::atomic::AtomicU64::new(
                self.failed_assignments
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            total_assignment_time_ms: std::sync::atomic::AtomicU64::new(
                self.total_assignment_time_ms
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            total_queue_wait_time_ms: std::sync::atomic::AtomicU64::new(
                self.total_queue_wait_time_ms
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }
}

impl Default for AssignmentStatistics {
    fn default() -> Self {
        Self::new()
    }
}

impl AssignmentStatistics {
    pub fn new() -> Self {
        Self {
            total_assignments: std::sync::atomic::AtomicU64::new(0),
            successful_assignments: std::sync::atomic::AtomicU64::new(0),
            queued_jobs: std::sync::atomic::AtomicU64::new(0),
            failed_assignments: std::sync::atomic::AtomicU64::new(0),
            total_assignment_time_ms: std::sync::atomic::AtomicU64::new(0),
            total_queue_wait_time_ms: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn get_stats(&self) -> AssignmentStats {
        let total = self
            .total_assignments
            .load(std::sync::atomic::Ordering::Relaxed);
        let successful = self
            .successful_assignments
            .load(std::sync::atomic::Ordering::Relaxed);
        let queued = self.queued_jobs.load(std::sync::atomic::Ordering::Relaxed);
        let failed = self
            .failed_assignments
            .load(std::sync::atomic::Ordering::Relaxed);
        let assignment_time_sum = self
            .total_assignment_time_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        let queue_wait_sum = self
            .total_queue_wait_time_ms
            .load(std::sync::atomic::Ordering::Relaxed);

        AssignmentStats {
            total_assignments: total,
            successful_assignments: successful,
            queued_jobs: queued,
            failed_assignments: failed,
            average_assignment_time_ms: if total > 0 {
                assignment_time_sum as f64 / total as f64
            } else {
                0.0
            },
            average_queue_wait_time_ms: if total > 0 {
                queue_wait_sum as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

impl Default for QueueAssignmentEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueAssignmentEngine {
    /// Create new assignment engine
    pub fn new() -> Self {
        Self::new_with_policy(SchedulingPolicy::FIFO)
    }

    /// Create new assignment engine with custom scheduling policy
    pub fn new_with_policy(scheduler_policy: SchedulingPolicy) -> Self {
        let queues = Arc::new(DashMap::new());

        // Create default queues
        queues.insert(
            "default".to_string(),
            JobQueue::new("default".to_string(), QueuePriority::Normal, 1000),
        );
        queues.insert(
            "high-priority".to_string(),
            JobQueue::new("high-priority".to_string(), QueuePriority::High, 100),
        );
        queues.insert(
            "low-priority".to_string(),
            JobQueue::new("low-priority".to_string(), QueuePriority::Low, 1000),
        );

        Self {
            queues,
            pools: Arc::new(DashMap::new()),
            scheduler_policy,
            stats: Arc::new(AssignmentStatistics::new()),
        }
    }

    /// Register a resource pool
    pub fn register_pool(&mut self, pool: Arc<dyn ResourcePool>) {
        let pool_id = pool.pool_id().to_string();
        self.pools.insert(pool_id, pool);
    }

    /// Get all registered queues
    pub async fn get_queues(&self) -> HashMap<String, JobQueue> {
        self.queues
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Submit a job for assignment
    pub async fn submit_job(&self, request: AssignmentRequest) -> Result<AssignmentResult> {
        let start_time = Instant::now();
        self.stats
            .total_assignments
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Validate priority
        if request.priority < 1 || request.priority > 10 {
            return Err(QueueError::InvalidPriority {
                priority: request.priority,
            }
            .into());
        }

        // Determine queue ID based on queue type
        let queue_id = self.get_queue_id(&request.queue_type, request.priority);

        // Try to assign to a pool immediately
        let assignment_result = self.try_assign_immediate(&request).await;

        match assignment_result {
            Ok(Some(result)) => {
                // Successful immediate assignment
                self.stats
                    .successful_assignments
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                let elapsed = start_time.elapsed();
                self.stats.total_assignment_time_ms.fetch_add(
                    elapsed.as_millis() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );

                Ok(result)
            }
            Ok(None) => {
                // No immediate assignment possible, queue the job
                self.queue_job(request, &queue_id).await
            }
            Err(e) => {
                // Assignment failed
                self.stats
                    .failed_assignments
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// Try to assign job immediately to a pool
    async fn try_assign_immediate(
        &self,
        request: &AssignmentRequest,
    ) -> Result<Option<AssignmentResult>> {
        if self.pools.is_empty() {
            return Ok(None);
        }

        // Try each pool to find a suitable worker
        for pool in self.pools.iter() {
            match pool.allocate_worker(&request.requirements).await {
                Ok(worker) => {
                    return Ok(Some(AssignmentResult::Assigned {
                        worker_id: worker.id,
                        pool_id: pool.pool_id().to_string(),
                        assignment_time: Duration::from_millis(1),
                    }));
                }
                Err(_) => {
                    // Try next pool
                    continue;
                }
            }
        }

        // No suitable worker found in any pool
        Ok(None)
    }

    /// Queue a job for later assignment
    async fn queue_job(
        &self,
        request: AssignmentRequest,
        queue_id: &str,
    ) -> Result<AssignmentResult> {
        let queue = self.queues.get(queue_id).ok_or_else(|| {
            hodei_pipelines_domain::DomainError::NotFound(format!("Queue not found: {}", queue_id))
        })?;

        let queued_job = QueuedJob {
            job_id: request.job_id,
            requirements: request.requirements,
            priority: request.priority,
            tenant_id: request.tenant_id,
            queue_type: request.queue_type,
            submitted_at: Utc::now(),
            max_wait_time: request.max_wait_time,
        };

        match queue.enqueue(queued_job).await {
            Ok(true) => {
                self.stats
                    .queued_jobs
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                let position = queue.size().await;
                Ok(AssignmentResult::Queued {
                    queue_id: queue_id.to_string(),
                    position,
                    estimated_wait: Duration::from_secs(1),
                })
            }
            Ok(false) => {
                // Queue overflow
                Err(QueueError::QueueOverflow {
                    queue_id: queue_id.to_string(),
                }
                .into())
            }
            Err(e) => Err(e),
        }
    }

    /// Process queued jobs (assign when workers become available)
    pub async fn process_queues(&self) -> Result<u32> {
        let mut processed = 0;

        for entry in self.queues.iter() {
            let queue_id = entry.key();
            let queue = entry.value();
            // Dequeue jobs and try to assign them
            while let Some(job) = queue.dequeue().await {
                let job_id = job.job_id;
                let queue_type = job.queue_type.clone();
                let requirements = job.requirements.clone();
                let tenant_id = job.tenant_id.clone();
                let max_wait_time = job.max_wait_time;

                let request = AssignmentRequest {
                    job_id,
                    requirements,
                    priority: job.priority,
                    tenant_id,
                    queue_type: queue_type.clone(),
                    max_wait_time,
                };

                match self.try_assign_immediate(&request).await {
                    Ok(Some(_assignment)) => {
                        processed += 1;
                        self.stats
                            .successful_assignments
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    Ok(None) => {
                        // Re-queue the job (no worker available)
                        let _ = queue.enqueue(job).await;
                        break;
                    }
                    Err(e) => {
                        error!(queue_id, job_id = %job_id, error = %e, "Assignment failed");
                        self.stats
                            .failed_assignments
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }

        Ok(processed)
    }

    /// Get queue ID based on queue type and priority
    fn get_queue_id(&self, queue_type: &QueueType, priority: u8) -> String {
        match queue_type {
            QueueType::Default => "default".to_string(),
            QueueType::HighPriority => {
                if priority >= 8 {
                    "high-priority".to_string()
                } else {
                    "default".to_string()
                }
            }
            QueueType::LowPriority => "low-priority".to_string(),
            QueueType::Bulk => "low-priority".to_string(),
            QueueType::Urgent => "high-priority".to_string(),
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> AssignmentStats {
        self.stats.get_stats()
    }

    /// Get queue metrics
    pub async fn get_queue_metrics(&self) -> HashMap<String, QueueMetricsSnapshot> {
        let mut metrics = HashMap::new();
        for entry in self.queues.iter() {
            metrics.insert(entry.key().clone(), entry.value().get_metrics());
        }
        metrics
    }
}

/// Dead letter queue for failed jobs
#[derive(Debug)]
pub struct DeadLetterQueue {
    pub queue_id: String,
    pub failed_jobs: Arc<RwLock<VecDeque<QueuedJob>>>,
    pub max_size: usize,
    pub metrics: QueueMetrics,
}

impl DeadLetterQueue {
    pub fn new(queue_id: String, max_size: usize) -> Self {
        Self {
            queue_id,
            failed_jobs: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            metrics: QueueMetrics::new(),
        }
    }

    /// Add a failed job to the dead letter queue
    pub async fn add_failed_job(&self, job: QueuedJob) -> Result<()> {
        let mut jobs = self.failed_jobs.write().await;

        // Check capacity
        if jobs.len() >= self.max_size {
            // Remove oldest job to make space
            jobs.pop_front();
        }

        jobs.push_back(job);
        self.metrics
            .total_enqueued
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Get failed jobs
    pub async fn get_failed_jobs(&self) -> Vec<QueuedJob> {
        self.failed_jobs.read().await.clone().into_iter().collect()
    }

    /// Clear all failed jobs
    pub async fn clear(&self) {
        let mut jobs = self.failed_jobs.write().await;
        jobs.clear();
    }
}

/// FIFO Standard Queue implementation
#[derive(Debug)]
pub struct FIFOStandardQueue {
    pub queue_id: String,
    pub capacity: usize,
    pub fifo_queue: Arc<RwLock<VecDeque<(QueuedJob, Instant)>>>,
    pub dead_letter_queue: DeadLetterQueue,
    pub timeout_check_interval: Duration,
    pub is_running: Arc<RwLock<bool>>,
    pub metrics: QueueMetrics,
}

impl FIFOStandardQueue {
    pub fn new(queue_id: String, capacity: usize, max_dead_letter_size: usize) -> Self {
        let dlq_id = format!("{}-dlq", queue_id);
        Self {
            queue_id,
            capacity,
            fifo_queue: Arc::new(RwLock::new(VecDeque::new())),
            dead_letter_queue: DeadLetterQueue::new(dlq_id, max_dead_letter_size),
            timeout_check_interval: Duration::from_secs(5),
            is_running: Arc::new(RwLock::new(true)),
            metrics: QueueMetrics::new(),
        }
    }

    /// Enqueue a job (FIFO ordering)
    pub async fn enqueue(&self, job: QueuedJob) -> Result<()> {
        let mut queue = self.fifo_queue.write().await;

        // Check capacity
        if queue.len() >= self.capacity {
            return Err(QueueError::QueueFull {
                queue_id: self.queue_id.clone(),
                capacity: self.capacity,
            }
            .into());
        }

        // Add job with timestamp for FIFO ordering
        queue.push_back((job, Instant::now()));

        self.metrics
            .current_size
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics
            .total_enqueued
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Dequeue the oldest job (FIFO)
    pub async fn dequeue(&self) -> Option<QueuedJob> {
        let mut queue = self.fifo_queue.write().await;

        if let Some((job, _)) = queue.pop_front() {
            self.metrics
                .current_size
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            self.metrics
                .total_dequeued
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Update wait time metrics
            let wait_time = Instant::now().elapsed();
            self.metrics.average_wait_time_ms.store(
                wait_time.as_millis() as u64,
                std::sync::atomic::Ordering::Relaxed,
            );

            Some(job)
        } else {
            None
        }
    }

    /// Check for timed-out jobs and move them to dead letter queue
    pub async fn check_timeouts(&self) -> Result<u32> {
        let mut timed_out_count = 0;
        let mut queue = self.fifo_queue.write().await;
        let mut dead_letter_jobs = Vec::new();

        // Find timed-out jobs
        let now = Instant::now();
        let mut remaining_queue = VecDeque::new();

        while let Some((job, enqueue_time)) = queue.pop_front() {
            let wait_time = now.duration_since(enqueue_time);

            if wait_time > job.max_wait_time {
                // Job has timed out
                dead_letter_jobs.push(job);
                timed_out_count += 1;
                self.metrics
                    .total_dequeued
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            } else {
                // Keep job in queue
                remaining_queue.push_back((job, enqueue_time));
            }
        }

        // Put non-timed-out jobs back
        *queue = remaining_queue;

        // Move timed-out jobs to dead letter queue
        for job in dead_letter_jobs {
            if let Err(e) = self.dead_letter_queue.add_failed_job(job).await {
                error!("Failed to add job to dead letter queue: {}", e);
            }
        }

        Ok(timed_out_count)
    }

    /// Start timeout monitoring task
    pub async fn start_timeout_monitor(&self) {
        let is_running = self.is_running.clone();
        let _fifo_queue = self.fifo_queue.clone();
        let _dead_letter_queue = self.dead_letter_queue.clone();
        let queue_id = self.queue_id.clone();
        let check_interval = self.timeout_check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);

            while *is_running.read().await {
                interval.tick().await;

                // Check for timeouts (simplified implementation)
                // In a real implementation, we'd iterate through the queue
                // For now, we'll just log that we're checking
                info!(queue_id = %queue_id, "Checking for timed-out jobs");
            }
        });
    }

    /// Stop timeout monitoring
    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;
    }

    /// Get current queue size
    pub async fn size(&self) -> usize {
        self.fifo_queue.read().await.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.fifo_queue.read().await.is_empty()
    }

    /// Get queue metrics snapshot
    pub fn get_metrics(&self) -> QueueMetricsSnapshot {
        QueueMetricsSnapshot {
            current_size: self
                .metrics
                .current_size
                .load(std::sync::atomic::Ordering::Relaxed),
            total_enqueued: self
                .metrics
                .total_enqueued
                .load(std::sync::atomic::Ordering::Relaxed),
            total_dequeued: self
                .metrics
                .total_dequeued
                .load(std::sync::atomic::Ordering::Relaxed),
            average_wait_time_ms: self
                .metrics
                .average_wait_time_ms
                .load(std::sync::atomic::Ordering::Relaxed),
            max_wait_time_ms: self
                .metrics
                .max_wait_time_ms
                .load(std::sync::atomic::Ordering::Relaxed),
            overflow_count: self
                .metrics
                .overflow_count
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Get dead letter queue
    pub fn dead_letter_queue(&self) -> &DeadLetterQueue {
        &self.dead_letter_queue
    }
}

impl Drop for FIFOStandardQueue {
    fn drop(&mut self) {
        // Signal timeout monitor to stop
        let is_running = self.is_running.clone();
        tokio::spawn(async move {
            let mut running = is_running.write().await;
            *running = false;
        });
    }
}

impl Clone for DeadLetterQueue {
    fn clone(&self) -> Self {
        Self {
            queue_id: self.queue_id.clone(),
            failed_jobs: Arc::new(RwLock::new(VecDeque::new())),
            max_size: self.max_size,
            metrics: QueueMetrics {
                current_size: std::sync::atomic::AtomicU64::new(
                    self.metrics
                        .current_size
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
                total_enqueued: std::sync::atomic::AtomicU64::new(
                    self.metrics
                        .total_enqueued
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
                total_dequeued: std::sync::atomic::AtomicU64::new(
                    self.metrics
                        .total_dequeued
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
                average_wait_time_ms: std::sync::atomic::AtomicU64::new(
                    self.metrics
                        .average_wait_time_ms
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
                max_wait_time_ms: std::sync::atomic::AtomicU64::new(
                    self.metrics
                        .max_wait_time_ms
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
                overflow_count: std::sync::atomic::AtomicU64::new(
                    self.metrics
                        .overflow_count
                        .load(std::sync::atomic::Ordering::Relaxed),
                ),
            },
        }
    }
}

// Convert QueueError to DomainError
impl From<QueueError> for hodei_pipelines_domain::DomainError {
    fn from(err: QueueError) -> Self {
        hodei_pipelines_domain::DomainError::Other(err.to_string())
    }
}
