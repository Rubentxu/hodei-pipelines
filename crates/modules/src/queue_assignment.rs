//! Queue Assignment Engine Module
//!
//! This module provides intelligent job assignment to workers across
//! multiple resource pools with priority-based queuing and scheduling policies.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use hodei_core::{JobId, Worker, WorkerId};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

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
    pub async fn enqueue(&self, job: QueuedJob) -> Result<bool, QueueError> {
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

/// Assignment engine
pub struct QueueAssignmentEngine {
    queues: HashMap<String, JobQueue>,
    pools: HashMap<String, Arc<dyn ResourcePool>>,
    scheduler_policy: SchedulingPolicy,
    stats: Arc<AssignmentStatistics>,
}

impl std::fmt::Debug for QueueAssignmentEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueueAssignmentEngine")
            .field("queues", &self.queues)
            .field("pool_ids", &self.pools.keys())
            .field("scheduler_policy", &self.scheduler_policy)
            .field("stats", &self.stats)
            .finish()
    }
}

/// Resource pool trait
#[async_trait]
pub trait ResourcePool {
    async fn allocate_worker(
        &self,
        requirements: &JobRequirements,
    ) -> Result<(WorkerId, Worker), ResourcePoolError>;
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

impl QueueAssignmentEngine {
    /// Create new assignment engine
    pub fn new() -> Self {
        Self::new_with_policy(SchedulingPolicy::FIFO)
    }

    /// Create new assignment engine with custom scheduling policy
    pub fn new_with_policy(scheduler_policy: SchedulingPolicy) -> Self {
        let mut queues = HashMap::new();

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
            pools: HashMap::new(),
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
        self.queues.clone()
    }

    /// Submit a job for assignment
    pub async fn submit_job(
        &self,
        request: AssignmentRequest,
    ) -> Result<AssignmentResult, QueueError> {
        let start_time = Instant::now();
        self.stats
            .total_assignments
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Validate priority
        if request.priority < 1 || request.priority > 10 {
            return Err(QueueError::InvalidPriority {
                priority: request.priority,
            });
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
    ) -> Result<Option<AssignmentResult>, QueueError> {
        if self.pools.is_empty() {
            return Ok(None);
        }

        // Try each pool to find a suitable worker
        for (_pool_id, pool) in &self.pools {
            match pool.allocate_worker(&request.requirements).await {
                Ok((worker_id, _worker)) => {
                    return Ok(Some(AssignmentResult::Assigned {
                        worker_id,
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
    ) -> Result<AssignmentResult, QueueError> {
        let queue = self
            .queues
            .get(queue_id)
            .ok_or_else(|| QueueError::QueueNotFound {
                queue_id: queue_id.to_string(),
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
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Process queued jobs (assign when workers become available)
    pub async fn process_queues(&self) -> Result<u32, QueueError> {
        let mut processed = 0;

        for (queue_id, queue) in &self.queues {
            // Dequeue jobs and try to assign them
            while let Some(job) = queue.dequeue().await {
                let job_id = job.job_id.clone();
                let queue_type = job.queue_type.clone();
                let requirements = job.requirements.clone();
                let tenant_id = job.tenant_id.clone();
                let max_wait_time = job.max_wait_time;

                let request = AssignmentRequest {
                    job_id: job_id.clone(),
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
        for (queue_id, queue) in &self.queues {
            metrics.insert(queue_id.clone(), queue.get_metrics());
        }
        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock resource pool for testing
    struct MockResourcePool {
        pool_id: String,
        available: bool,
    }

    #[async_trait]
    impl ResourcePool for MockResourcePool {
        async fn allocate_worker(
            &self,
            _requirements: &JobRequirements,
        ) -> Result<(WorkerId, Worker), ResourcePoolError> {
            if self.available {
                let worker_id = WorkerId::new();
                let worker = Worker::new(
                    worker_id.clone(),
                    format!("{}-worker", self.pool_id),
                    hodei_shared_types::WorkerCapabilities::new(4, 8192),
                );
                Ok((worker_id, worker))
            } else {
                Err(ResourcePoolError::NoWorkerAvailable)
            }
        }

        async fn get_status(&self) -> PoolStatus {
            PoolStatus {
                pool_id: self.pool_id.clone(),
                available_workers: if self.available { 1 } else { 0 },
                busy_workers: 0,
                total_workers: 1,
            }
        }

        fn pool_id(&self) -> &str {
            &self.pool_id
        }
    }

    #[tokio::test]
    async fn test_queue_creation() {
        let queue = JobQueue::new("test".to_string(), QueuePriority::Normal, 100);

        assert_eq!(queue.queue_id, "test");
        assert_eq!(queue.priority, QueuePriority::Normal);
        assert_eq!(queue.max_size, 100);
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_job_enqueue_dequeue() {
        let queue = JobQueue::new("test".to_string(), QueuePriority::Normal, 100);

        let job = QueuedJob {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 5,
            tenant_id: "test-tenant".to_string(),
            queue_type: QueueType::Default,
            submitted_at: Utc::now(),
            max_wait_time: Duration::from_secs(60),
        };

        assert!(queue.enqueue(job.clone()).await.unwrap());
        assert!(!queue.is_empty().await);
        assert_eq!(queue.size().await, 1);

        let dequeued = queue.dequeue().await.unwrap();
        assert_eq!(dequeued.job_id, job.job_id);
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_priority_queueing() {
        let queue = JobQueue::new("test".to_string(), QueuePriority::Normal, 100);

        // Enqueue jobs with different priorities
        let job1 = QueuedJob {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 3,
            tenant_id: "tenant1".to_string(),
            queue_type: QueueType::Default,
            submitted_at: Utc::now(),
            max_wait_time: Duration::from_secs(60),
        };

        let job2 = QueuedJob {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 8,
            tenant_id: "tenant2".to_string(),
            queue_type: QueueType::Default,
            submitted_at: Utc::now(),
            max_wait_time: Duration::from_secs(60),
        };

        let job3 = QueuedJob {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 5,
            tenant_id: "tenant3".to_string(),
            queue_type: QueueType::Default,
            submitted_at: Utc::now(),
            max_wait_time: Duration::from_secs(60),
        };

        queue.enqueue(job1.clone()).await.unwrap();
        queue.enqueue(job2.clone()).await.unwrap();
        queue.enqueue(job3.clone()).await.unwrap();

        // Dequeue and verify priority order (higher priority first)
        let first = queue.dequeue().await.unwrap();
        assert_eq!(first.priority, 8); // job2

        let second = queue.dequeue().await.unwrap();
        assert_eq!(second.priority, 5); // job3

        let third = queue.dequeue().await.unwrap();
        assert_eq!(third.priority, 3); // job1
    }

    #[tokio::test]
    async fn test_queue_overflow() {
        let queue = JobQueue::new("test".to_string(), QueuePriority::Normal, 2);

        let job = QueuedJob {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 5,
            tenant_id: "test-tenant".to_string(),
            queue_type: QueueType::Default,
            submitted_at: Utc::now(),
            max_wait_time: Duration::from_secs(60),
        };

        // Fill the queue
        assert!(queue.enqueue(job.clone()).await.unwrap());
        assert!(queue.enqueue(job.clone()).await.unwrap());

        // Try to add one more (should fail)
        assert!(!queue.enqueue(job.clone()).await.unwrap());

        // Verify overflow counter
        let metrics = queue.get_metrics();
        assert_eq!(metrics.overflow_count, 1);
    }

    #[tokio::test]
    async fn test_assignment_engine_creation() {
        let engine = QueueAssignmentEngine::new_with_policy(SchedulingPolicy::Priority);

        assert_eq!(engine.queues.len(), 3); // default, high-priority, low-priority
        assert!(engine.pools.is_empty());
    }

    #[tokio::test]
    async fn test_immediate_assignment() {
        let mut engine = QueueAssignmentEngine::new_with_policy(SchedulingPolicy::FIFO);

        // Register a mock pool
        let pool = Arc::new(MockResourcePool {
            pool_id: "test-pool".to_string(),
            available: true,
        });
        engine.register_pool(pool);

        let request = AssignmentRequest {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 5,
            tenant_id: "test-tenant".to_string(),
            queue_type: QueueType::Default,
            max_wait_time: Duration::from_secs(60),
        };

        let result = engine.submit_job(request).await.unwrap();

        match result {
            AssignmentResult::Assigned {
                worker_id,
                pool_id,
                assignment_time,
            } => {
                assert!(!worker_id.to_string().is_empty());
                assert_eq!(pool_id, "test-pool");
                assert!(assignment_time.as_millis() > 0);
            }
            _ => panic!("Expected AssignmentResult::Assigned"),
        }
    }

    #[tokio::test]
    async fn test_queueing_when_no_worker_available() {
        let mut engine = QueueAssignmentEngine::new_with_policy(SchedulingPolicy::FIFO);

        // Register a mock pool with no available workers
        let pool = Arc::new(MockResourcePool {
            pool_id: "test-pool".to_string(),
            available: false,
        });
        engine.register_pool(pool);

        let request = AssignmentRequest {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 5,
            tenant_id: "test-tenant".to_string(),
            queue_type: QueueType::Default,
            max_wait_time: Duration::from_secs(60),
        };

        let result = engine.submit_job(request).await.unwrap();

        match result {
            AssignmentResult::Queued {
                queue_id,
                position,
                estimated_wait,
            } => {
                assert_eq!(queue_id, "default");
                assert_eq!(position, 1);
                assert!(estimated_wait.as_millis() > 0);
            }
            _ => panic!("Expected AssignmentResult::Queued"),
        }
    }

    #[tokio::test]
    async fn test_invalid_priority() {
        let engine = QueueAssignmentEngine::new_with_policy(SchedulingPolicy::FIFO);

        let request = AssignmentRequest {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 15, // Invalid priority
            tenant_id: "test-tenant".to_string(),
            queue_type: QueueType::Default,
            max_wait_time: Duration::from_secs(60),
        };

        let result = engine.submit_job(request).await;

        assert!(matches!(result, Err(QueueError::InvalidPriority { .. })));
    }

    #[tokio::test]
    async fn test_statistics_tracking() {
        let mut engine = QueueAssignmentEngine::new_with_policy(SchedulingPolicy::FIFO);

        let pool = Arc::new(MockResourcePool {
            pool_id: "test-pool".to_string(),
            available: true,
        });
        engine.register_pool(pool);

        // Submit a job for immediate assignment
        let request = AssignmentRequest {
            job_id: JobId::new(),
            requirements: JobRequirements::default(),
            priority: 5,
            tenant_id: "test-tenant".to_string(),
            queue_type: QueueType::Default,
            max_wait_time: Duration::from_secs(60),
        };

        let _ = engine.submit_job(request).await.unwrap();

        let stats = engine.get_stats();
        assert_eq!(stats.total_assignments, 1);
        assert_eq!(stats.successful_assignments, 1);
    }
}
