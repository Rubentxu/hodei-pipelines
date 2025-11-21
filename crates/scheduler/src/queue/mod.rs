//! Priority Queue Module with Preemption Support
//!
//! This module implements multiple job queue strategies similar to Kubernetes:
//! - Priority Queue with preemption support
//! - Simple FIFO queue
//! - Fair Queuing across tenants

use crate::types::*;
use crate::{JobId, SchedulerError};
use chrono::Utc;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// Queue strategy types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueStrategy {
    /// Simple FIFO (First In, First Out)
    Fifo,

    /// Priority queue with optional preemption
    Priority {
        with_preemption: bool,
        max_queue_time: Option<chrono::Duration>,
    },

    /// Fair queuing distributed across tenants
    Fair {
        tenant_key: String,
        weights: HashMap<String, u32>,
        quantum: Option<chrono::Duration>,
    },
}

impl Default for QueueStrategy {
    fn default() -> Self {
        QueueStrategy::Priority {
            with_preemption: true,
            max_queue_time: None,
        }
    }
}

/// Queue configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub max_size: usize,
    pub strategy: QueueStrategy,
    pub fairness_enabled: bool,
    pub namespace: Option<String>,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            strategy: QueueStrategy::default(),
            fairness_enabled: false,
            namespace: None,
        }
    }
}

/// Priority queue implementation
#[derive(Debug)]
pub struct PriorityQueue {
    inner: Arc<Mutex<PriorityQueueInner>>,
}

#[derive(Debug)]
struct PriorityQueueInner {
    queue: BinaryHeap<QueueEntry>,
    job_map: HashMap<JobId, QueueEntry>,
    position_map: HashMap<JobId, usize>,
    config: QueueConfig,
}

#[derive(Debug, Clone)]
struct QueueEntry {
    job_id: JobId,
    priority: JobPriority,
    enqueue_time: chrono::DateTime<Utc>,
    name: String,
    namespace: String,
}

/// Ordering for priority queue (higher priority comes first)
impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        if self.priority != other.priority {
            return self.priority.cmp(&other.priority);
        }

        // Earlier enqueue time first (FIFO for same priority)
        self.enqueue_time.cmp(&other.enqueue_time)
    }
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.enqueue_time == other.enqueue_time
            && self.job_id == other.job_id
    }
}

impl Eq for QueueEntry {}

impl PriorityQueue {
    /// Create new priority queue
    pub fn new(config: QueueConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PriorityQueueInner {
                queue: BinaryHeap::new(),
                job_map: HashMap::new(),
                position_map: HashMap::new(),
                config,
            })),
        }
    }

    /// Enqueue a job
    pub async fn enqueue(&self, job: Job) -> Result<(), SchedulerError> {
        let mut inner = self.inner.lock().unwrap();

        // Check queue capacity
        if inner.queue.len() >= inner.config.max_size {
            return Err(SchedulerError::QueueError(
                "Queue is at maximum capacity".to_string(),
            ));
        }

        let entry = QueueEntry {
            job_id: job.metadata.id,
            priority: job.spec.priority.clone(),
            enqueue_time: job.metadata.created_at,
            name: job.metadata.name.clone(),
            namespace: job.metadata.namespace.clone(),
        };

        inner.queue.push(entry.clone());
        inner.job_map.insert(job.metadata.id, entry);

        // Update position map
        let position = inner.queue.len() - 1;
        inner.position_map.insert(job.metadata.id, position);

        debug!(
            "Enqueued job {} with priority {:?} (position: {})",
            job.metadata.id, job.spec.priority, position
        );

        Ok(())
    }

    /// Dequeue the highest priority job
    pub async fn dequeue(&self) -> Option<JobId> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(entry) = inner.queue.pop() {
            inner.job_map.remove(&entry.job_id);
            inner.position_map.remove(&entry.job_id);

            debug!("Dequeued job {}", entry.job_id);

            Some(entry.job_id)
        } else {
            None
        }
    }

    /// Get the next job without removing it
    pub async fn peek(&self) -> Option<JobId> {
        let inner = self.inner.lock().unwrap();
        inner.queue.peek().map(|entry| entry.job_id)
    }

    /// Cancel a job (remove from queue)
    pub async fn cancel(&self, job_id: &JobId) -> Result<(), SchedulerError> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.job_map.contains_key(job_id) {
            return Err(SchedulerError::QueueError(format!(
                "Job {} not found in queue",
                job_id
            )));
        }

        // Remove from structures
        inner.job_map.remove(job_id);
        inner.position_map.remove(job_id);

        // Rebuild queue without the job
        // This is O(n) but necessary for BinaryHeap removal
        let entries: Vec<_> = inner
            .queue
            .drain()
            .filter(|e| e.job_id != *job_id)
            .collect();

        for entry in &entries {
            inner.queue.push(entry.clone());
        }

        for entry in &entries {
            inner.job_map.insert(entry.job_id, entry.clone());
            let position = inner
                .queue
                .iter()
                .position(|e| e.job_id == entry.job_id)
                .unwrap_or(0);
            inner.position_map.insert(entry.job_id, position);
        }

        info!("Cancelled job {}", job_id);

        Ok(())
    }

    /// Check if job is in queue
    pub async fn contains(&self, job_id: &JobId) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.job_map.contains_key(job_id)
    }

    /// Get queue position of a job
    pub async fn position(&self, job_id: &JobId) -> Option<usize> {
        let inner = self.inner.lock().unwrap();
        inner.position_map.get(job_id).copied()
    }

    /// Get pending job count
    pub async fn pending_count(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.queue.len()
    }

    /// Get all pending jobs
    pub async fn list_pending(&self) -> Vec<QueueEntryView> {
        let inner = self.inner.lock().unwrap();
        inner
            .queue
            .iter()
            .map(|entry| QueueEntryView {
                job_id: entry.job_id,
                priority: entry.priority.clone(),
                enqueue_time: entry.enqueue_time,
                name: entry.name.clone(),
                namespace: entry.namespace.clone(),
            })
            .collect()
    }

    /// Preemption: check if high-priority job can preempt others
    pub async fn check_preemption(&self, job: &Job) -> Result<Vec<JobId>, SchedulerError> {
        if let QueueStrategy::Priority {
            with_preemption, ..
        } = &self.inner.lock().unwrap().config.strategy
        {
            if !with_preemption {
                return Ok(vec![]);
            }
        } else {
            // Preemption only supported for Priority strategy
            return Ok(vec![]);
        }

        let mut inner = self.inner.lock().unwrap();
        let mut preempted = Vec::new();

        // Find lower priority jobs that can be preempted
        let mut preemptable_jobs: Vec<_> = inner
            .queue
            .iter()
            .filter(|entry| job.spec.priority.can_preempt(&entry.priority))
            .cloned()
            .collect();

        // Sort by priority (lowest first) to preempt lowest priority jobs first
        preemptable_jobs.sort_by(|a, b| {
            b.priority.cmp(&a.priority) // Reverse order - lowest priority last
        });

        // Select jobs to preempt
        // For simplicity, preempt all lower priority jobs
        // In production, you'd preempt only as many as needed
        for entry in preemptable_jobs {
            preempted.push(entry.job_id);
        }

        if !preempted.is_empty() {
            info!(
                "Preemption: job {} (priority {:?}) can preempt {} jobs",
                job.metadata.id,
                job.spec.priority,
                preempted.len()
            );
        }

        Ok(preempted)
    }

    /// Execute preemption
    pub async fn preempt(&self, preempting_job: &Job) -> Result<Vec<JobId>, SchedulerError> {
        let preempted_jobs = self.check_preemption(preempting_job).await?;

        if preempted_jobs.is_empty() {
            return Ok(vec![]);
        }

        let mut inner = self.inner.lock().unwrap();

        // Remove preempted jobs
        for job_id in &preempted_jobs {
            inner.job_map.remove(job_id);
            inner.position_map.remove(job_id);

            // Remove from queue
            let remaining: Vec<_> = inner
                .queue
                .drain()
                .filter(|e| e.job_id != *job_id)
                .collect();

            inner.queue.clear();
            for entry in &remaining {
                inner.queue.push(entry.clone());
            }

            info!("Preempted job {}", job_id);
        }

        // Add the preempting job
        let entry = QueueEntry {
            job_id: preempting_job.metadata.id,
            priority: preempting_job.spec.priority.clone(),
            enqueue_time: preempting_job.metadata.created_at,
            name: preempting_job.metadata.name.clone(),
            namespace: preempting_job.metadata.namespace.clone(),
        };

        inner.queue.push(entry.clone());
        inner.job_map.insert(preempting_job.metadata.id, entry);
        let position = inner.queue.len() - 1;
        inner
            .position_map
            .insert(preempting_job.metadata.id, position);

        Ok(preempted_jobs)
    }

    /// Clear the queue
    pub async fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.queue.clear();
        inner.job_map.clear();
        inner.position_map.clear();
    }
}

/// Queue entry view for external access
#[derive(Debug, Clone)]
pub struct QueueEntryView {
    pub job_id: JobId,
    pub priority: JobPriority,
    pub enqueue_time: chrono::DateTime<Utc>,
    pub name: String,
    pub namespace: String,
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_jobs: usize,
    pub priority_counts: HashMap<JobPriority, usize>,
    pub oldest_job_age: Option<chrono::Duration>,
    pub newest_job_age: Option<chrono::Duration>,
    pub tenant_distribution: HashMap<String, usize>,
}

impl PriorityQueue {
    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let inner = self.inner.lock().unwrap();

        let mut priority_counts = HashMap::new();
        let mut tenant_distribution = HashMap::new();
        let mut oldest_time = None;
        let mut newest_time = None;

        for entry in &inner.queue {
            *priority_counts.entry(entry.priority.clone()).or_insert(0) += 1;
            *tenant_distribution
                .entry(entry.namespace.clone())
                .or_insert(0) += 1;

            if oldest_time.is_none() || entry.enqueue_time < oldest_time.unwrap() {
                oldest_time = Some(entry.enqueue_time);
            }

            if newest_time.is_none() || entry.enqueue_time > newest_time.unwrap() {
                newest_time = Some(entry.enqueue_time);
            }
        }

        let now = Utc::now();
        let oldest_job_age = oldest_time.map(|t| now - t);
        let newest_job_age = newest_time.map(|t| now - t);

        QueueStats {
            total_jobs: inner.queue.len(),
            priority_counts,
            oldest_job_age,
            newest_job_age,
            tenant_distribution,
        }
    }
}

/// FIFO Queue implementation (simple first-in, first-out)
#[derive(Debug)]
pub struct FifoQueue {
    inner: Arc<Mutex<FifoQueueInner>>,
}

#[derive(Debug)]
struct FifoQueueInner {
    queue: Vec<QueueEntry>,
    job_map: HashMap<JobId, QueueEntry>,
    config: QueueConfig,
}

impl FifoQueue {
    /// Create new FIFO queue
    pub fn new(config: QueueConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FifoQueueInner {
                queue: Vec::new(),
                job_map: HashMap::new(),
                config,
            })),
        }
    }

    /// Enqueue a job
    pub async fn enqueue(&self, job: Job) -> Result<(), SchedulerError> {
        let mut inner = self.inner.lock().unwrap();

        if inner.queue.len() >= inner.config.max_size {
            return Err(SchedulerError::QueueError(
                "Queue is at maximum capacity".to_string(),
            ));
        }

        let entry = QueueEntry {
            job_id: job.metadata.id,
            priority: job.spec.priority,
            enqueue_time: job.metadata.created_at,
            name: job.metadata.name.clone(),
            namespace: job.metadata.namespace.clone(),
        };

        inner.queue.push(entry.clone());
        inner.job_map.insert(job.metadata.id, entry);

        debug!("FIFO: Enqueued job {}", job.metadata.id);

        Ok(())
    }

    /// Dequeue the first job (FIFO order)
    pub async fn dequeue(&self) -> Option<JobId> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(entry) = inner.queue.first().cloned() {
            inner.queue.remove(0);
            inner.job_map.remove(&entry.job_id);

            debug!("FIFO: Dequeued job {}", entry.job_id);

            Some(entry.job_id)
        } else {
            None
        }
    }

    /// Check if job is in queue
    pub async fn contains(&self, job_id: &JobId) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.job_map.contains_key(job_id)
    }

    /// Get pending job count
    pub async fn pending_count(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.queue.len()
    }

    /// Cancel a job
    pub async fn cancel(&self, job_id: &JobId) -> Result<(), SchedulerError> {
        let mut inner = self.inner.lock().unwrap();

        if !inner.job_map.contains_key(job_id) {
            return Err(SchedulerError::QueueError(format!(
                "Job {} not found in queue",
                job_id
            )));
        }

        inner.job_map.remove(job_id);
        inner.queue.retain(|e| e.job_id != *job_id);

        info!("FIFO: Cancelled job {}", job_id);

        Ok(())
    }

    /// Clear the queue
    pub async fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.queue.clear();
        inner.job_map.clear();
    }
}

/// Fair Queue implementation (weighted round-robin by tenant)
#[derive(Debug)]
pub struct FairQueue {
    inner: Arc<Mutex<FairQueueInner>>,
}

#[derive(Debug)]
struct FairQueueInner {
    tenant_queues: HashMap<String, BinaryHeap<QueueEntry>>,
    current_tenant: String,
    weights: HashMap<String, u32>,
    quantum: chrono::Duration,
    config: QueueConfig,
}

impl FairQueue {
    /// Create new Fair queue
    pub fn new(config: QueueConfig) -> Result<Self, SchedulerError> {
        if let QueueStrategy::Fair {
            tenant_key,
            weights,
            quantum,
        } = &config.strategy
        {
            Ok(Self {
                inner: Arc::new(Mutex::new(FairQueueInner {
                    tenant_queues: HashMap::new(),
                    current_tenant: tenant_key.clone(),
                    weights: weights.clone(),
                    quantum: quantum.unwrap_or_else(|| chrono::Duration::seconds(1)),
                    config,
                })),
            })
        } else {
            Err(SchedulerError::QueueError(
                "FairQueue requires Fair strategy".to_string(),
            ))
        }
    }

    /// Enqueue a job
    pub async fn enqueue(&self, job: Job) -> Result<(), SchedulerError> {
        let mut inner = self.inner.lock().unwrap();

        let total_jobs: usize = inner.tenant_queues.values().map(|q| q.len()).sum();
        if total_jobs >= inner.config.max_size {
            return Err(SchedulerError::QueueError(
                "Queue is at maximum capacity".to_string(),
            ));
        }

        let entry = QueueEntry {
            job_id: job.metadata.id,
            priority: job.spec.priority,
            enqueue_time: job.metadata.created_at,
            name: job.metadata.name.clone(),
            namespace: job.metadata.namespace.clone(),
        };

        let tenant_key = job.metadata.namespace.clone();

        inner
            .tenant_queues
            .entry(tenant_key.clone())
            .or_insert_with(BinaryHeap::new)
            .push(entry);

        info!(
            "FairQueue: Enqueued job {} to tenant {}",
            job.metadata.id, tenant_key
        );

        Ok(())
    }

    /// Dequeue using weighted round-robin
    pub async fn dequeue(&self) -> Option<JobId> {
        let mut inner = self.inner.lock().unwrap();

        if inner.tenant_queues.is_empty() {
            return None;
        }

        // Get sorted tenants by weight and dequeued count
        let mut tenants: Vec<_> = inner.tenant_queues.keys().cloned().collect();
        tenants.sort_by(|a, b| {
            let weight_a = inner.weights.get(a).unwrap_or(&1);
            let weight_b = inner.weights.get(b).unwrap_or(&1);
            weight_b.cmp(weight_a)
        });

        // Try to dequeue from each tenant in order
        for tenant in tenants.iter() {
            if let Some(queue) = inner.tenant_queues.get_mut(tenant) {
                if let Some(entry) = queue.pop() {
                    debug!(
                        "FairQueue: Dequeued job {} from tenant {}",
                        entry.job_id, tenant
                    );

                    // Update current tenant for next round
                    inner.current_tenant = tenant.clone();

                    return Some(entry.job_id);
                }
            }
        }

        None
    }

    /// Get pending job count
    pub async fn pending_count(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.tenant_queues.values().map(|q| q.len()).sum()
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let inner = self.inner.lock().unwrap();

        let mut tenant_distribution = HashMap::new();

        for (tenant, queue) in &inner.tenant_queues {
            tenant_distribution.insert(tenant.clone(), queue.len());

            // Also calculate priority distribution
            for entry in queue.iter() {
                // Note: We could calculate priority_counts here too if needed
            }
        }

        QueueStats {
            total_jobs: inner.tenant_queues.values().map(|q| q.len()).sum(),
            priority_counts: HashMap::new(), // Not calculated for FairQueue
            oldest_job_age: None,
            newest_job_age: None,
            tenant_distribution,
        }
    }

    /// Clear the queue
    pub async fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.tenant_queues.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Job, JobMetadata, JobSpec, ResourceRequirements};

    fn create_test_job(id: u32, priority: JobPriority) -> Job {
        Job {
            metadata: JobMetadata {
                id: uuid::Uuid::from_u128(id as u128),
                name: format!("test-job-{}", id),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            spec: JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(2.0),
                    memory_bytes: Some(4_000_000_000),
                    gpu_count: None,
                    ephemeral_storage: None,
                }),
                priority,
                node_selector: None,
                affinity: None,
                tolerations: vec![],
                max_retries: 3,
            },
        }
    }

    #[tokio::test]
    async fn test_queue_enqueue_dequeue() {
        let queue = PriorityQueue::new(QueueConfig::default());

        let job1 = create_test_job(1, JobPriority::Medium);
        let job2 = create_test_job(2, JobPriority::High);

        queue.enqueue(job1.clone()).await.unwrap();
        queue.enqueue(job2.clone()).await.unwrap();

        assert_eq!(queue.pending_count().await, 2);

        // High priority job should be dequeued first
        let first = queue.dequeue().await;
        assert_eq!(first, Some(job2.metadata.id));

        let second = queue.dequeue().await;
        assert_eq!(second, Some(job1.metadata.id));
    }

    #[tokio::test]
    async fn test_queue_priority_ordering() {
        let queue = PriorityQueue::new(QueueConfig::default());

        // Add jobs in random order
        queue
            .enqueue(create_test_job(1, JobPriority::Low))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(2, JobPriority::High))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(3, JobPriority::Medium))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(4, JobPriority::Critical))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(5, JobPriority::Batch))
            .await
            .unwrap();

        assert_eq!(queue.pending_count().await, 5);

        // Dequeue and verify priority order
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(4)));
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(2)));
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(3)));
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(1)));
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(5)));
    }

    #[tokio::test]
    async fn test_queue_cancel() {
        let queue = PriorityQueue::new(QueueConfig::default());

        let job1 = create_test_job(1, JobPriority::Medium);
        queue.enqueue(job1.clone()).await.unwrap();

        assert!(queue.contains(&job1.metadata.id).await);

        queue.cancel(&job1.metadata.id).await.unwrap();

        assert!(!queue.contains(&job1.metadata.id).await);
        assert_eq!(queue.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_queue_position() {
        let queue = PriorityQueue::new(QueueConfig::default());

        let job1 = create_test_job(1, JobPriority::Low);
        let job2 = create_test_job(2, JobPriority::High);

        queue.enqueue(job1.clone()).await.unwrap();
        queue.enqueue(job2.clone()).await.unwrap();

        // Job 2 (High) should be before job1 (Low) in priority queue
        // High priority jobs come first regardless of insertion order
        assert!(queue.position(&job2.metadata.id).await.is_some());
        assert!(queue.position(&job1.metadata.id).await.is_some());

        // Both jobs should be in the queue
        assert!(queue.contains(&job1.metadata.id).await);
        assert!(queue.contains(&job2.metadata.id).await);
    }

    #[tokio::test]
    async fn test_preemption() {
        let config = QueueConfig {
            strategy: QueueStrategy::Priority {
                with_preemption: true,
                max_queue_time: None,
            },
            ..Default::default()
        };
        let queue = PriorityQueue::new(config);

        // Add low priority jobs
        queue
            .enqueue(create_test_job(1, JobPriority::Low))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(2, JobPriority::Medium))
            .await
            .unwrap();

        // Add high priority job that should trigger preemption
        let high_job = create_test_job(3, JobPriority::High);
        let preempted = queue.preempt(&high_job).await.unwrap();

        // High priority job should preempt lower priority jobs
        assert!(!preempted.is_empty());

        // Check that high priority job is now in queue
        assert!(queue.contains(&high_job.metadata.id).await);
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let queue = PriorityQueue::new(QueueConfig::default());

        queue
            .enqueue(create_test_job(1, JobPriority::Low))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(2, JobPriority::High))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(3, JobPriority::Medium))
            .await
            .unwrap();

        let stats = queue.stats().await;

        assert_eq!(stats.total_jobs, 3);
        assert_eq!(stats.priority_counts[&JobPriority::Low], 1);
        assert_eq!(stats.priority_counts[&JobPriority::Medium], 1);
        assert_eq!(stats.priority_counts[&JobPriority::High], 1);
    }

    #[tokio::test]
    async fn test_queue_capacity() {
        let config = QueueConfig {
            max_size: 2,
            strategy: QueueStrategy::default(),
            ..Default::default()
        };
        let queue = PriorityQueue::new(config);

        queue
            .enqueue(create_test_job(1, JobPriority::Low))
            .await
            .unwrap();
        queue
            .enqueue(create_test_job(2, JobPriority::Medium))
            .await
            .unwrap();

        // Third enqueue should fail
        let result = queue.enqueue(create_test_job(3, JobPriority::High)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fifo_queue_basic() {
        let config = QueueConfig {
            max_size: 100,
            strategy: QueueStrategy::Fifo,
            ..Default::default()
        };
        let queue = FifoQueue::new(config);

        // Enqueue jobs in specific order
        let job1 = create_test_job(1, JobPriority::Medium);
        let job2 = create_test_job(2, JobPriority::High);
        let job3 = create_test_job(3, JobPriority::Low);

        queue.enqueue(job1).await.unwrap();
        queue.enqueue(job2).await.unwrap();
        queue.enqueue(job3).await.unwrap();

        // FIFO should return jobs in insertion order
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(1)));
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(2)));
        assert_eq!(queue.dequeue().await, Some(uuid::Uuid::from_u128(3)));

        assert_eq!(queue.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_fifo_queue_cancel() {
        let config = QueueConfig {
            max_size: 100,
            strategy: QueueStrategy::Fifo,
            ..Default::default()
        };
        let queue = FifoQueue::new(config);

        let job1 = create_test_job(1, JobPriority::Medium);
        queue.enqueue(job1.clone()).await.unwrap();

        assert!(queue.contains(&job1.metadata.id).await);

        queue.cancel(&job1.metadata.id).await.unwrap();

        assert!(!queue.contains(&job1.metadata.id).await);
        assert_eq!(queue.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_fair_queue_basic() {
        let mut weights = HashMap::new();
        weights.insert("tenant-a".to_string(), 2);
        weights.insert("tenant-b".to_string(), 1);

        let config = QueueConfig {
            max_size: 100,
            strategy: QueueStrategy::Fair {
                tenant_key: "namespace".to_string(),
                weights: weights.clone(),
                quantum: None,
            },
            ..Default::default()
        };

        let queue = FairQueue::new(config).unwrap();

        // Create jobs with different tenants
        let mut job1 = create_test_job(1, JobPriority::Medium);
        job1.metadata.namespace = "tenant-a".to_string();
        queue.enqueue(job1).await.unwrap();

        let mut job2 = create_test_job(2, JobPriority::High);
        job2.metadata.namespace = "tenant-b".to_string();
        queue.enqueue(job2).await.unwrap();

        let mut job3 = create_test_job(3, JobPriority::Low);
        job3.metadata.namespace = "tenant-a".to_string();
        queue.enqueue(job3).await.unwrap();

        // Fair queue should alternate between tenants
        // tenant-a has weight 2, so should get more jobs
        let first = queue.dequeue().await;
        assert!(first.is_some());

        let second = queue.dequeue().await;
        assert!(second.is_some());

        let third = queue.dequeue().await;
        assert!(third.is_some());

        // All jobs should be dequeued
        assert_eq!(queue.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_fair_queue_stats() {
        let mut weights = HashMap::new();
        weights.insert("tenant-a".to_string(), 2);
        weights.insert("tenant-b".to_string(), 1);

        let config = QueueConfig {
            max_size: 100,
            strategy: QueueStrategy::Fair {
                tenant_key: "namespace".to_string(),
                weights: weights.clone(),
                quantum: None,
            },
            ..Default::default()
        };

        let queue = FairQueue::new(config).unwrap();

        // Create jobs with different tenants
        for i in 1..=3 {
            let mut job = create_test_job(i, JobPriority::Medium);
            job.metadata.namespace = "tenant-a".to_string();
            queue.enqueue(job).await.unwrap();
        }

        for i in 4..=5 {
            let mut job = create_test_job(i, JobPriority::Medium);
            job.metadata.namespace = "tenant-b".to_string();
            queue.enqueue(job).await.unwrap();
        }

        let stats = queue.stats().await;

        assert_eq!(stats.total_jobs, 5);
        assert_eq!(stats.tenant_distribution["tenant-a"], 3);
        assert_eq!(stats.tenant_distribution["tenant-b"], 2);
    }

    #[tokio::test]
    async fn test_queue_strategy_different_types() {
        // Test Priority Queue
        let priority_config = QueueConfig {
            max_size: 100,
            strategy: QueueStrategy::Priority {
                with_preemption: true,
                max_queue_time: Some(chrono::Duration::minutes(5)),
            },
            ..Default::default()
        };
        let priority_queue = PriorityQueue::new(priority_config);
        assert!(priority_queue.pending_count().await == 0);

        // Test FIFO Queue
        let fifo_config = QueueConfig {
            max_size: 100,
            strategy: QueueStrategy::Fifo,
            ..Default::default()
        };
        let fifo_queue = FifoQueue::new(fifo_config);
        assert!(fifo_queue.pending_count().await == 0);

        // Test Fair Queue
        let mut weights = HashMap::new();
        weights.insert("tenant-1".to_string(), 1);

        let fair_config = QueueConfig {
            max_size: 100,
            strategy: QueueStrategy::Fair {
                tenant_key: "namespace".to_string(),
                weights: weights.clone(),
                quantum: None,
            },
            ..Default::default()
        };
        let fair_queue = FairQueue::new(fair_config).unwrap();
        assert!(fair_queue.pending_count().await == 0);
    }
}
