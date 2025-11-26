//! QueueManager Domain Service
//!
//! This module implements the QueueManager domain service which orchestrates
//! job queueing operations using Weighted Fair Queuing (WFQ) algorithm.

use crate::queueing::{QueueState, QueuedJob};
use std::collections::HashMap;

/// Queueing policy trait - Strategy pattern for queue algorithms
pub trait QueueingPolicy: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
}

/// WFQ Policy implementation
#[derive(Debug)]
pub struct WFQPolicy {
    pub name: String,
    pub tenant_weights: HashMap<String, f64>,
    pub fairness_threshold: f64,
    pub min_weight: f64,
    pub max_weight: f64,
}

impl WFQPolicy {
    pub fn new(
        name: String,
        tenant_weights: HashMap<String, f64>,
        fairness_threshold: f64,
        min_weight: f64,
        max_weight: f64,
    ) -> Self {
        Self {
            name,
            tenant_weights,
            fairness_threshold,
            min_weight,
            max_weight,
        }
    }

    pub fn get_tenant_weight(&self, tenant_id: &str) -> f64 {
        self.tenant_weights.get(tenant_id).copied().unwrap_or(1.0)
    }
}

impl QueueingPolicy for WFQPolicy {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Domain Service for managing job queues with fair sharing using WFQ
/// Implements Weighted Fair Queuing algorithm for fair job scheduling
pub struct QueueManager {
    /// WFQ policy for tenant weight management
    wfq_policy: WFQPolicy,

    /// Pending jobs queue
    pending_jobs: HashMap<String, QueuedJob>,

    /// Current queue state for policy decisions
    queue_state: QueueState,

    /// Global virtual time for WFQ scheduling
    global_virtual_time: f64,
}

impl QueueManager {
    /// Create a new QueueManager with WFQ policy
    pub fn new_with_wfq(max_capacity: usize, tenant_weights: HashMap<String, f64>) -> Self {
        let wfq_policy = WFQPolicy::new(
            "wfq".to_string(),
            tenant_weights,
            2.0,  // fairness_threshold
            0.1,  // min_weight
            10.0, // max_weight
        );

        Self {
            wfq_policy,
            pending_jobs: HashMap::new(),
            queue_state: QueueState::new(max_capacity),
            global_virtual_time: 0.0,
        }
    }

    /// Enqueue a job with WFQ priority calculation
    pub fn enqueue(
        &mut self,
        job_id: String,
        tenant_id: String,
        job_weight: f64,
    ) -> std::result::Result<(), QueueManagerError> {
        // Check capacity
        if self.is_full() {
            return Err(QueueManagerError::CapacityExceeded {
                current: self.queue_state.current_size,
                max: self.queue_state.max_capacity,
            });
        }

        // Get tenant weight from WFQ policy
        let tenant_weight = self.wfq_policy.get_tenant_weight(&tenant_id);

        // Calculate priority based on job weight and tenant weight
        let priority = ((job_weight * tenant_weight) * 10.0).min(10.0).max(1.0) as u8;

        // Calculate virtual finish time for WFQ
        let virtual_finish_time = self.global_virtual_time + (job_weight / tenant_weight);

        // Create queued job
        let mut queued_job = QueuedJob::new(job_id.clone(), tenant_id, priority, job_weight);
        queued_job.virtual_finish_time = virtual_finish_time;

        // Enqueue job
        self.pending_jobs.insert(job_id, queued_job);
        self.queue_state.current_size += 1;

        Ok(())
    }

    /// Dequeue next job based on WFQ algorithm (smallest virtual finish time)
    pub fn dequeue(&mut self) -> std::result::Result<Option<String>, QueueManagerError> {
        if self.pending_jobs.is_empty() {
            return Ok(None);
        }

        // Update global virtual time
        self.global_virtual_time += 1.0;

        // WFQ: select job with smallest virtual finish time
        let mut best_job_id: Option<String> = None;
        let mut best_virtual_time = f64::MAX;

        for (job_id, job) in &self.pending_jobs {
            if job.virtual_finish_time < best_virtual_time {
                best_virtual_time = job.virtual_finish_time;
                best_job_id = Some(job_id.clone());
            }
        }

        // Remove selected job from queue
        if let Some(job_id) = best_job_id {
            self.pending_jobs.remove(&job_id);
            self.queue_state.current_size -= 1;
            Ok(Some(job_id))
        } else {
            Ok(None)
        }
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.queue_state.is_at_capacity()
    }

    /// Get pending jobs count
    pub fn len(&self) -> usize {
        self.pending_jobs.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.pending_jobs.is_empty()
    }

    /// Get current queue status
    pub fn get_status(&self) -> QueueStatus {
        QueueStatus {
            current_size: self.queue_state.current_size,
            max_capacity: self.queue_state.max_capacity,
            utilization: if self.queue_state.max_capacity > 0 {
                self.queue_state.current_size as f64 / self.queue_state.max_capacity as f64
            } else {
                0.0
            },
        }
    }
}

/// Queue status information
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub current_size: usize,
    pub max_capacity: usize,
    pub utilization: f64,
}

/// Error types for QueueManager
#[derive(thiserror::Error, Debug)]
pub enum QueueManagerError {
    #[error("Queue capacity exceeded: {current}/{max}")]
    CapacityExceeded { current: usize, max: usize },

    #[error("Invalid job weight: {weight}")]
    InvalidWeight { weight: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_queue_manager_with_wfq_policy() {
        let mut tenant_weights = HashMap::new();
        tenant_weights.insert("tenant-a".to_string(), 2.0);
        tenant_weights.insert("tenant-b".to_string(), 1.0);

        let mut queue = QueueManager::new_with_wfq(10, tenant_weights);

        // Enqueue jobs from different tenants
        assert!(
            queue
                .enqueue("job-1".to_string(), "tenant-a".to_string(), 1.0)
                .is_ok()
        );
        assert!(
            queue
                .enqueue("job-2".to_string(), "tenant-b".to_string(), 1.0)
                .is_ok()
        );
        assert!(
            queue
                .enqueue("job-3".to_string(), "tenant-a".to_string(), 1.0)
                .is_ok()
        );

        assert_eq!(queue.len(), 3);
        assert!(!queue.is_full());
    }

    #[test]
    fn test_enqueue_dequeue_lifecycle() {
        let mut tenant_weights = HashMap::new();
        tenant_weights.insert("tenant-1".to_string(), 1.0);

        let mut queue = QueueManager::new_with_wfq(10, tenant_weights);

        // Enqueue job
        assert!(
            queue
                .enqueue("job-1".to_string(), "tenant-1".to_string(), 1.0)
                .is_ok()
        );
        assert_eq!(queue.len(), 1);

        // Dequeue job
        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued, Some("job-1".to_string()));
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_capacity_limit() {
        let tenant_weights = HashMap::new();
        let mut queue = QueueManager::new_with_wfq(2, tenant_weights);

        // Fill queue to capacity
        assert!(
            queue
                .enqueue("job-1".to_string(), "tenant-1".to_string(), 1.0)
                .is_ok()
        );
        assert!(
            queue
                .enqueue("job-2".to_string(), "tenant-1".to_string(), 1.0)
                .is_ok()
        );
        assert_eq!(queue.len(), 2);

        // Next enqueue should fail
        assert!(
            queue
                .enqueue("job-3".to_string(), "tenant-1".to_string(), 1.0)
                .is_err()
        );
    }

    #[test]
    fn test_queue_status() {
        let tenant_weights = HashMap::new();
        let mut queue = QueueManager::new_with_wfq(10, tenant_weights);

        queue
            .enqueue("job-1".to_string(), "tenant-1".to_string(), 1.0)
            .unwrap();

        let status = queue.get_status();
        assert_eq!(status.current_size, 1);
        assert_eq!(status.max_capacity, 10);
        assert!(status.utilization > 0.0);
        assert!(status.utilization <= 1.0);
    }

    #[test]
    fn test_wfq_priority_calculation() {
        let mut tenant_weights = HashMap::new();
        tenant_weights.insert("high-priority-tenant".to_string(), 3.0);
        tenant_weights.insert("low-priority-tenant".to_string(), 1.0);

        let mut queue = QueueManager::new_with_wfq(10, tenant_weights);

        // Higher weight tenant should get higher priority
        assert!(
            queue
                .enqueue("job-1".to_string(), "high-priority-tenant".to_string(), 1.0)
                .is_ok()
        );
        assert!(
            queue
                .enqueue("job-2".to_string(), "low-priority-tenant".to_string(), 1.0)
                .is_ok()
        );

        // Dequeue should return job with smaller virtual finish time
        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued, Some("job-1".to_string())); // High priority tenant job first
    }
}
