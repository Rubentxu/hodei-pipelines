//! Queue Scaling Integration Module
//!
//! This module integrates dynamic pools with job queues to enable
//! automatic scaling based on queue depth and job submission events.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info};

use super::queue_assignment::{QueueAssignmentEngine, QueueType};
use super::scaling_policies::{ScalingAction, ScalingContext, ScalingDecision, ScalingEngine};

/// Queue scaling event
#[derive(Debug, Clone)]
pub enum QueueScalingEvent {
    JobSubmitted {
        queue_type: QueueType,
        priority: u8,
        tenant_id: String,
        job_id: String,
    },
    JobAssigned {
        queue_type: QueueType,
        job_id: String,
        worker_id: String,
    },
    JobCompleted {
        queue_type: QueueType,
        job_id: String,
    },
    JobCancelled {
        queue_type: QueueType,
        job_id: String,
    },
}

/// Queue scaling statistics
#[derive(Debug, Clone)]
pub struct QueueScalingStats {
    pub total_jobs_submitted: u64,
    pub total_jobs_assigned: u64,
    pub total_jobs_completed: u64,
    pub total_jobs_cancelled: u64,
    pub current_queue_depth: HashMap<String, u32>,
    pub last_scaling_trigger: Option<Instant>,
    pub scaling_decisions_made: u64,
}

impl Default for QueueScalingStats {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueScalingStats {
    pub fn new() -> Self {
        Self {
            total_jobs_submitted: 0,
            total_jobs_assigned: 0,
            total_jobs_completed: 0,
            total_jobs_cancelled: 0,
            current_queue_depth: HashMap::new(),
            last_scaling_trigger: None,
            scaling_decisions_made: 0,
        }
    }

    pub fn update_queue_depth(&mut self, queue_id: &str, depth: u32) {
        self.current_queue_depth.insert(queue_id.to_string(), depth);
    }
}

/// Queue scaling configuration
#[derive(Debug, Clone)]
pub struct QueueScalingConfig {
    pub min_queue_depth_for_scaling: u32,
    pub cooldown_between_scaling: Duration,
    pub max_scaling_decisions_per_minute: u32,
}

impl Default for QueueScalingConfig {
    fn default() -> Self {
        Self {
            min_queue_depth_for_scaling: 5,
            cooldown_between_scaling: Duration::from_secs(60),
            max_scaling_decisions_per_minute: 10,
        }
    }
}

/// Queue Scaling Integration Service
pub struct QueueScalingIntegration {
    pub assignment_engine: Arc<RwLock<QueueAssignmentEngine>>,
    pub scaling_engine: Arc<RwLock<ScalingEngine>>,
    pub config: QueueScalingConfig,
    pub stats: Arc<RwLock<QueueScalingStats>>,
    pub last_scaling_decisions: Arc<RwLock<HashMap<String, Instant>>>,
}

impl QueueScalingIntegration {
    pub fn new(
        assignment_engine: Arc<RwLock<QueueAssignmentEngine>>,
        scaling_engine: Arc<RwLock<ScalingEngine>>,
        config: QueueScalingConfig,
    ) -> Self {
        Self {
            assignment_engine,
            scaling_engine,
            config,
            stats: Arc::new(RwLock::new(QueueScalingStats::new())),
            last_scaling_decisions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if scaling is needed based on queue depth
    pub async fn check_and_scale(&self) -> Result<Vec<ScalingDecision>, QueueScalingError> {
        let queues = {
            let engine = self.assignment_engine.read().await;
            engine.get_queues().await
        };

        let mut decisions = Vec::new();

        // Check each queue
        for (queue_id, queue) in queues {
            let depth = queue.get_length().await;

            // Update stats
            {
                let mut stats = self.stats.write().await;
                stats.update_queue_depth(&queue_id, depth);
            }

            // Check cooldown
            let now = Instant::now();
            let mut last_decisions = self.last_scaling_decisions.write().await;

            if let Some(last_time) = last_decisions.get(&queue_id)
                && now.duration_since(*last_time) < self.config.cooldown_between_scaling {
                    continue;
                }

            // Evaluate scaling decision
            let context = ScalingContext {
                pool_id: queue_id.clone(),
                current_size: 5, // This would come from actual pool state
                pending_jobs: depth,
                cpu_utilization: 50.0,
                memory_utilization: 50.0,
                active_workers: 3,
                idle_workers: 2,
                timestamp: Utc::now(),
            };

            let decision = {
                let engine = self.scaling_engine.read().await;
                engine.evaluate_scaling(&context).await
            };

            if let Some(decision) = decision {
                if !matches!(decision.action, ScalingAction::NoOp) {
                    info!(
                        queue_id = queue_id,
                        action = ?decision.action,
                        reason = decision.reason,
                        "Queue depth triggered scaling"
                    );

                    // Record decision
                    last_decisions.insert(queue_id, now);

                    let mut stats = self.stats.write().await;
                    stats.last_scaling_trigger = Some(now);
                    stats.scaling_decisions_made += 1;
                }

                decisions.push(decision);
            }
        }

        Ok(decisions)
    }

    /// Handle queue scaling events
    pub async fn handle_event(&self, event: QueueScalingEvent) {
        match event {
            QueueScalingEvent::JobSubmitted {
                queue_type, job_id, ..
            } => {
                info!(queue_type = ?queue_type, job_id, "Job submitted event");
                let mut stats = self.stats.write().await;
                stats.total_jobs_submitted += 1;
            }
            QueueScalingEvent::JobAssigned {
                queue_type, job_id, ..
            } => {
                info!(queue_type = ?queue_type, job_id, "Job assigned event");
                let mut stats = self.stats.write().await;
                stats.total_jobs_assigned += 1;
            }
            QueueScalingEvent::JobCompleted { queue_type, job_id } => {
                info!(queue_type = ?queue_type, job_id, "Job completed event");
                let mut stats = self.stats.write().await;
                stats.total_jobs_completed += 1;
            }
            QueueScalingEvent::JobCancelled { queue_type, job_id } => {
                info!(queue_type = ?queue_type, job_id, "Job cancelled event");
                let mut stats = self.stats.write().await;
                stats.total_jobs_cancelled += 1;
            }
        }
    }

    /// Get current queue scaling statistics
    pub async fn get_stats(&self) -> QueueScalingStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get queue depth for a specific queue
    pub async fn get_queue_depth(&self, queue_id: &str) -> u32 {
        let stats = self.stats.read().await;
        stats
            .current_queue_depth
            .get(queue_id)
            .copied()
            .unwrap_or(0)
    }

    /// Get maximum queue depth across all queues
    pub async fn get_max_queue_depth(&self) -> u32 {
        let stats = self.stats.read().await;
        stats
            .current_queue_depth
            .values()
            .copied()
            .max()
            .unwrap_or(0)
    }
}

/// Errors
#[derive(Error, Debug)]
pub enum QueueScalingError {
    #[error("Scaling operation failed: {0}")]
    ScalingFailed(String),

    #[error("Queue not found: {0}")]
    QueueNotFound(String),
}

