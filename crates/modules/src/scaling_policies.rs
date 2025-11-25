//! Scaling Policies Module
//!
//! This module provides intelligent scaling policies for dynamic resource pools
//! with queue depth, CPU utilization, and custom metrics-based scaling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use hodei_core::WorkerId;
use hodei_core::{ResourceQuota, WorkerStatus};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Scaling decision
#[derive(Debug, Clone)]
pub struct ScalingDecision {
    pub pool_id: String,
    pub action: ScalingAction,
    pub target_size: u32,
    pub reason: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub cooldown_remaining: Option<Duration>,
}

/// Scaling actions
#[derive(Debug, Clone, PartialEq)]
pub enum ScalingAction {
    ScaleUp(u32),   // Scale up by N workers
    ScaleDown(u32), // Scale down by N workers
    NoOp,           // No action needed
}

/// Scaling context
#[derive(Debug, Clone)]
pub struct ScalingContext {
    pub pool_id: String,
    pub current_size: u32,
    pub pending_jobs: u32,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub active_workers: u32,
    pub idle_workers: u32,
    pub timestamp: chrono::DateTime<Utc>,
}

/// Metrics snapshot for scaling decisions
#[derive(Debug, Clone)]
pub struct ScalingMetrics {
    pub timestamp: chrono::DateTime<Utc>,
    pub pending_jobs: u32,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub worker_count: u32,
    pub utilization_history: VecDeque<ScalingMetrics>,
}

impl ScalingMetrics {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            pending_jobs: 0,
            cpu_utilization: 0.0,
            memory_utilization: 0.0,
            worker_count: 0,
            utilization_history: VecDeque::with_capacity(60), // Keep 60 samples
        }
    }

    pub fn average_cpu_utilization(&self, window_size: usize) -> f64 {
        let samples: Vec<_> = self
            .utilization_history
            .iter()
            .rev()
            .take(window_size)
            .collect();
        let sum: f64 = samples.iter().map(|m| m.cpu_utilization).sum();
        let count = samples.len() as f64;
        if count > 0.0 {
            sum / count
        } else {
            self.cpu_utilization
        }
    }
}

use std::collections::VecDeque;

/// Queue depth-based scaling policy
#[derive(Debug, Clone)]
pub struct QueueDepthScalingPolicy {
    pub policy_id: String,
    pub scale_up_threshold: u32,
    pub scale_down_threshold: u32,
    pub min_pending_for_scale_up: u32,
    pub max_workers: u32,
    pub min_workers: u32,
}

impl QueueDepthScalingPolicy {
    pub fn new(
        policy_id: String,
        scale_up_threshold: u32,
        scale_down_threshold: u32,
        min_workers: u32,
        max_workers: u32,
    ) -> Self {
        Self {
            policy_id,
            scale_up_threshold,
            scale_down_threshold,
            min_pending_for_scale_up: scale_up_threshold,
            max_workers,
            min_workers,
        }
    }

    pub async fn evaluate(&self, context: &ScalingContext) -> ScalingAction {
        if context.pending_jobs >= self.scale_up_threshold
            && context.pending_jobs >= self.min_pending_for_scale_up
        {
            // Calculate scale-up amount based on backlog
            let backlog = context.pending_jobs - self.scale_up_threshold;
            let scale_up_by = (backlog / self.scale_up_threshold + 1).min(10); // Max 10 at a time
            let target_size = (context.current_size + scale_up_by).min(self.max_workers);

            ScalingAction::ScaleUp(target_size.saturating_sub(context.current_size))
        } else if context.pending_jobs <= self.scale_down_threshold
            && context.active_workers > self.min_workers
        {
            // Scale down proportionally
            let excess = context.active_workers - self.min_workers;
            let scale_down_by = (excess / 2).min(5); // Conservative scale down
            ScalingAction::ScaleDown(scale_down_by.max(1))
        } else {
            ScalingAction::NoOp
        }
    }
}

/// CPU utilization-based scaling policy
#[derive(Debug, Clone)]
pub struct CpuUtilizationScalingPolicy {
    pub policy_id: String,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub min_cpu_utilization: f64,
    pub max_cpu_utilization: f64,
    pub min_workers: u32,
    pub max_workers: u32,
    pub evaluation_window: usize,
}

impl CpuUtilizationScalingPolicy {
    pub fn new(
        policy_id: String,
        scale_up_threshold: f64,
        scale_down_threshold: f64,
        min_workers: u32,
        max_workers: u32,
        evaluation_window: usize,
    ) -> Self {
        Self {
            policy_id,
            scale_up_threshold,
            scale_down_threshold,
            min_cpu_utilization: scale_up_threshold * 0.5,
            max_cpu_utilization: scale_up_threshold,
            min_workers,
            max_workers,
            evaluation_window,
        }
    }

    pub async fn evaluate(&self, context: &ScalingContext) -> ScalingAction {
        // For this implementation, we'll use the current CPU utilization
        // In a real implementation, you'd use the average over the evaluation window
        let cpu_usage = context.cpu_utilization;

        if cpu_usage >= self.scale_up_threshold && context.current_size < self.max_workers {
            // Scale up based on CPU pressure
            let utilization_ratio = cpu_usage / self.scale_up_threshold;
            let scale_up_by = (utilization_ratio * 2.0).ceil() as u32; // Scale based on pressure
            let target_size = (context.current_size + scale_up_by).min(self.max_workers);

            ScalingAction::ScaleUp(target_size.saturating_sub(context.current_size))
        } else if cpu_usage <= self.scale_down_threshold
            && context.active_workers > self.min_workers
        {
            // Scale down if CPU is under-utilized
            let scale_down_by = ((self.scale_up_threshold - cpu_usage) / self.scale_up_threshold
                * 2.0)
                .ceil() as u32;
            ScalingAction::ScaleDown(scale_down_by.max(1))
        } else {
            ScalingAction::NoOp
        }
    }
}

/// Custom scaling policy based on multiple metrics
#[derive(Debug, Clone)]
pub struct CustomScalingPolicy {
    pub policy_id: String,
    pub rules: Vec<ScalingRule>,
    pub min_workers: u32,
    pub max_workers: u32,
}

#[derive(Debug, Clone)]
pub struct ScalingRule {
    pub name: String,
    pub condition: ScalingCondition,
    pub action: ScalingAction,
}

/// Scaling condition
#[derive(Debug, Clone)]
pub enum ScalingCondition {
    PendingJobsGreaterThan(u32),
    PendingJobsLessThan(u32),
    CpuUtilizationGreaterThan(f64),
    CpuUtilizationLessThan(f64),
    MemoryUtilizationGreaterThan(f64),
    MemoryUtilizationLessThan(f64),
    ActiveWorkersGreaterThan(u32),
    ActiveWorkersLessThan(u32),
}

impl CustomScalingPolicy {
    pub fn new(policy_id: String, min_workers: u32, max_workers: u32) -> Self {
        Self {
            policy_id,
            rules: Vec::new(),
            min_workers,
            max_workers,
        }
    }

    pub fn add_rule(&mut self, rule: ScalingRule) {
        self.rules.push(rule);
    }

    pub async fn evaluate(&self, context: &ScalingContext) -> ScalingAction {
        // Evaluate rules in order
        for rule in &self.rules {
            if self.evaluate_condition(&rule.condition, context) {
                return match &rule.action {
                    ScalingAction::ScaleUp(by) => {
                        let target = (context.current_size + by).min(self.max_workers);
                        ScalingAction::ScaleUp(target.saturating_sub(context.current_size))
                    }
                    ScalingAction::ScaleDown(by) => {
                        let target = context.current_size.saturating_sub(*by);
                        if target >= self.min_workers {
                            ScalingAction::ScaleDown(*by)
                        } else {
                            ScalingAction::NoOp
                        }
                    }
                    ScalingAction::NoOp => ScalingAction::NoOp,
                };
            }
        }

        ScalingAction::NoOp
    }

    fn evaluate_condition(&self, condition: &ScalingCondition, context: &ScalingContext) -> bool {
        match condition {
            ScalingCondition::PendingJobsGreaterThan(threshold) => {
                context.pending_jobs > *threshold
            }
            ScalingCondition::PendingJobsLessThan(threshold) => context.pending_jobs < *threshold,
            ScalingCondition::CpuUtilizationGreaterThan(threshold) => {
                context.cpu_utilization > *threshold
            }
            ScalingCondition::CpuUtilizationLessThan(threshold) => {
                context.cpu_utilization < *threshold
            }
            ScalingCondition::MemoryUtilizationGreaterThan(threshold) => {
                context.memory_utilization > *threshold
            }
            ScalingCondition::MemoryUtilizationLessThan(threshold) => {
                context.memory_utilization < *threshold
            }
            ScalingCondition::ActiveWorkersGreaterThan(threshold) => {
                context.active_workers > *threshold
            }
            ScalingCondition::ActiveWorkersLessThan(threshold) => {
                context.active_workers < *threshold
            }
        }
    }
}

/// Cooldown manager to prevent oscillation
#[derive(Debug, Clone)]
pub struct CooldownManager {
    pub cooldown_duration: Duration,
    pub last_scaling: HashMap<String, Instant>,
}

impl CooldownManager {
    pub fn new(cooldown_duration: Duration) -> Self {
        Self {
            cooldown_duration,
            last_scaling: HashMap::new(),
        }
    }

    pub fn is_in_cooldown(&self, pool_id: &str) -> bool {
        if let Some(last_time) = self.last_scaling.get(pool_id) {
            last_time.elapsed() < self.cooldown_duration
        } else {
            false
        }
    }

    pub fn record_scaling(&mut self, pool_id: &str) {
        self.last_scaling
            .insert(pool_id.to_string(), Instant::now());
    }

    pub fn remaining_cooldown(&self, pool_id: &str) -> Option<Duration> {
        if let Some(last_time) = self.last_scaling.get(pool_id) {
            let elapsed = last_time.elapsed();
            if elapsed < self.cooldown_duration {
                Some(self.cooldown_duration - elapsed)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Policy enum to unify different policy types
#[derive(Debug, Clone)]
pub enum ScalingPolicyEnum {
    QueueDepth(QueueDepthScalingPolicy),
    CpuUtilization(CpuUtilizationScalingPolicy),
    Custom(CustomScalingPolicy),
}

impl ScalingPolicyEnum {
    pub async fn evaluate(&self, context: &ScalingContext) -> ScalingAction {
        match self {
            ScalingPolicyEnum::QueueDepth(policy) => policy.evaluate(context).await,
            ScalingPolicyEnum::CpuUtilization(policy) => policy.evaluate(context).await,
            ScalingPolicyEnum::Custom(policy) => policy.evaluate(context).await,
        }
    }

    pub fn policy_id(&self) -> &str {
        match self {
            ScalingPolicyEnum::QueueDepth(policy) => &policy.policy_id,
            ScalingPolicyEnum::CpuUtilization(policy) => &policy.policy_id,
            ScalingPolicyEnum::Custom(policy) => &policy.policy_id,
        }
    }
}

/// Scaling engine
#[derive(Debug)]
pub struct ScalingEngine {
    pub policies: HashMap<String, ScalingPolicyEnum>,
    pub cooldown_manager: Arc<RwLock<CooldownManager>>,
    pub metrics: Arc<ScalingMetrics>,
}

impl ScalingEngine {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            cooldown_manager: Arc::new(RwLock::new(CooldownManager::new(Duration::from_secs(60)))),
            metrics: Arc::new(ScalingMetrics::new()),
        }
    }

    pub async fn add_policy(&mut self, policy: ScalingPolicyEnum) {
        self.policies.insert(policy.policy_id().to_string(), policy);
    }

    pub async fn set_cooldown_duration(&mut self, duration: Duration) {
        let mut manager = self.cooldown_manager.write().await;
        manager.cooldown_duration = duration;
    }

    pub async fn evaluate_scaling(&self, context: &ScalingContext) -> Option<ScalingDecision> {
        // Check cooldown
        {
            let manager = self.cooldown_manager.read().await;
            if manager.is_in_cooldown(&context.pool_id) {
                return Some(ScalingDecision {
                    pool_id: context.pool_id.clone(),
                    action: ScalingAction::NoOp,
                    target_size: context.current_size,
                    reason: format!(
                        "In cooldown: {:?} remaining",
                        manager.remaining_cooldown(&context.pool_id)
                    ),
                    timestamp: Utc::now(),
                    cooldown_remaining: manager.remaining_cooldown(&context.pool_id),
                });
            }
        }

        // Evaluate all policies
        let mut decisions = Vec::new();
        for policy in self.policies.values() {
            let action = policy.evaluate(context).await;
            if action != ScalingAction::NoOp {
                let action_clone = action.clone();
                decisions.push(ScalingDecision {
                    pool_id: context.pool_id.clone(),
                    action,
                    target_size: match action_clone {
                        ScalingAction::ScaleUp(by) => context.current_size + by,
                        ScalingAction::ScaleDown(by) => context.current_size.saturating_sub(by),
                        ScalingAction::NoOp => context.current_size,
                    },
                    reason: format!("Policy '{}'", policy.policy_id()),
                    timestamp: Utc::now(),
                    cooldown_remaining: None,
                });
            }
        }

        // Choose the most aggressive action (scale up wins over scale down)
        let decision = if decisions.is_empty() {
            ScalingDecision {
                pool_id: context.pool_id.clone(),
                action: ScalingAction::NoOp,
                target_size: context.current_size,
                reason: "No scaling needed".to_string(),
                timestamp: Utc::now(),
                cooldown_remaining: None,
            }
        } else {
            // Prioritize scale up over scale down
            let has_scale_up = decisions
                .iter()
                .any(|d| matches!(d.action, ScalingAction::ScaleUp(_)));
            if has_scale_up {
                decisions
                    .into_iter()
                    .find(|d| matches!(d.action, ScalingAction::ScaleUp(_)))
                    .unwrap()
            } else {
                decisions.into_iter().next().unwrap()
            }
        };

        // Record scaling decision if it's an actual scaling action
        if !matches!(decision.action, ScalingAction::NoOp) {
            let mut manager = self.cooldown_manager.write().await;
            manager.record_scaling(&context.pool_id);
        }

        Some(decision)
    }

    pub async fn update_metrics(
        &self,
        pool_id: &str,
        pending_jobs: u32,
        cpu_util: f64,
        mem_util: f64,
        worker_count: u32,
    ) {
        let mut metrics = self.metrics.clone();
        let mut_metrics = Arc::make_mut(&mut metrics);
        mut_metrics.timestamp = Utc::now();
        mut_metrics.pending_jobs = pending_jobs;
        mut_metrics.cpu_utilization = cpu_util;
        mut_metrics.memory_utilization = mem_util;
        mut_metrics.worker_count = worker_count;

        mut_metrics.utilization_history.push_back(ScalingMetrics {
            timestamp: Utc::now(),
            pending_jobs,
            cpu_utilization: cpu_util,
            memory_utilization: mem_util,
            worker_count,
            utilization_history: VecDeque::new(),
        });

        // Keep only the last 60 samples
        while mut_metrics.utilization_history.len() > 60 {
            mut_metrics.utilization_history.pop_front();
        }
    }

    pub async fn get_cooldown_status(&self, pool_id: &str) -> Option<Duration> {
        let manager = self.cooldown_manager.read().await;
        manager.remaining_cooldown(pool_id)
    }
}

impl Default for ScalingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors
#[derive(Error, Debug)]
pub enum ScalingError {
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Invalid scale operation: {0}")]
    InvalidScaleOperation(String),

    #[error("Cooldown violation: {0}")]
    CooldownViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_depth_scaling_up() {
        let policy = QueueDepthScalingPolicy::new(
            "queue-depth".to_string(),
            10,  // scale up threshold
            3,   // scale down threshold
            2,   // min workers
            100, // max workers
        );

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 5,
            pending_jobs: 15,
            cpu_utilization: 50.0,
            memory_utilization: 60.0,
            active_workers: 5,
            idle_workers: 2,
            timestamp: Utc::now(),
        };

        let action = policy.evaluate(&context).await;
        assert!(matches!(action, ScalingAction::ScaleUp(1)));
    }

    #[tokio::test]
    async fn test_queue_depth_scaling_down() {
        let policy = QueueDepthScalingPolicy::new("queue-depth".to_string(), 10, 3, 2, 100);

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 10,
            pending_jobs: 1,
            cpu_utilization: 30.0,
            memory_utilization: 40.0,
            active_workers: 3,
            idle_workers: 7,
            timestamp: Utc::now(),
        };

        let action = policy.evaluate(&context).await;
        assert!(matches!(action, ScalingAction::ScaleDown(1)));
    }

    #[tokio::test]
    async fn test_queue_depth_no_scaling() {
        let policy = QueueDepthScalingPolicy::new("queue-depth".to_string(), 10, 3, 2, 100);

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 10,
            pending_jobs: 5,
            cpu_utilization: 50.0,
            memory_utilization: 60.0,
            active_workers: 8,
            idle_workers: 2,
            timestamp: Utc::now(),
        };

        let action = policy.evaluate(&context).await;
        assert!(matches!(action, ScalingAction::NoOp));
    }

    #[tokio::test]
    async fn test_cpu_utilization_scaling() {
        let policy = CpuUtilizationScalingPolicy::new(
            "cpu-util".to_string(),
            80.0, // scale up threshold
            30.0, // scale down threshold
            2,    // min workers
            100,  // max workers
            5,    // evaluation window
        );

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 5,
            pending_jobs: 3,
            cpu_utilization: 85.0,
            memory_utilization: 70.0,
            active_workers: 5,
            idle_workers: 2,
            timestamp: Utc::now(),
        };

        let action = policy.evaluate(&context).await;
        assert!(matches!(action, ScalingAction::ScaleUp(_)));
    }

    #[tokio::test]
    async fn test_custom_scaling_policy() {
        let mut policy = CustomScalingPolicy::new(
            "custom".to_string(),
            2,   // min workers
            100, // max workers
        );

        policy.add_rule(ScalingRule {
            name: "scale up on high load".to_string(),
            condition: ScalingCondition::PendingJobsGreaterThan(20),
            action: ScalingAction::ScaleUp(5),
        });

        policy.add_rule(ScalingRule {
            name: "scale down on low load".to_string(),
            condition: ScalingCondition::PendingJobsLessThan(2),
            action: ScalingAction::ScaleDown(3),
        });

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 10,
            pending_jobs: 25,
            cpu_utilization: 60.0,
            memory_utilization: 50.0,
            active_workers: 8,
            idle_workers: 2,
            timestamp: Utc::now(),
        };

        let action = policy.evaluate(&context).await;
        assert!(matches!(action, ScalingAction::ScaleUp(5)));
    }

    #[tokio::test]
    async fn test_cooldown_manager() {
        let mut manager = CooldownManager::new(Duration::from_secs(10));

        // Initially not in cooldown
        assert!(!manager.is_in_cooldown("pool1"));

        // Record scaling
        manager.record_scaling("pool1");

        // Immediately after, should be in cooldown
        assert!(manager.is_in_cooldown("pool1"));

        // Check remaining cooldown
        let remaining = manager.remaining_cooldown("pool1");
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_scaling_engine() {
        let mut engine = ScalingEngine::new();

        let queue_policy = ScalingPolicyEnum::QueueDepth(QueueDepthScalingPolicy::new(
            "queue-depth".to_string(),
            10,
            3,
            2,
            100,
        ));

        engine.add_policy(queue_policy).await;

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 5,
            pending_jobs: 15,
            cpu_utilization: 50.0,
            memory_utilization: 60.0,
            active_workers: 5,
            idle_workers: 2,
            timestamp: Utc::now(),
        };

        let decision = engine.evaluate_scaling(&context).await;
        assert!(decision.is_some());

        let decision = decision.unwrap();
        assert!(matches!(decision.action, ScalingAction::ScaleUp(_)));
        assert_eq!(decision.pool_id, "test-pool");

        // Should be in cooldown now
        let cooldown = engine.get_cooldown_status("test-pool").await;
        assert!(cooldown.is_some());
    }

    #[tokio::test]
    async fn test_scaling_engine_cooldown_prevents_scaling() {
        let mut engine = ScalingEngine::new();
        engine
            .set_cooldown_duration(Duration::from_millis(100))
            .await;

        let queue_policy = ScalingPolicyEnum::QueueDepth(QueueDepthScalingPolicy::new(
            "queue-depth".to_string(),
            10,
            3,
            2,
            100,
        ));

        engine.add_policy(queue_policy).await;

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 5,
            pending_jobs: 15,
            cpu_utilization: 50.0,
            memory_utilization: 60.0,
            active_workers: 5,
            idle_workers: 2,
            timestamp: Utc::now(),
        };

        // First evaluation - should scale
        let decision = engine.evaluate_scaling(&context).await;
        assert!(decision.is_some());
        assert!(matches!(
            decision.unwrap().action,
            ScalingAction::ScaleUp(_)
        ));

        // Second evaluation immediately after - should be in cooldown
        let decision = engine.evaluate_scaling(&context).await;
        assert!(decision.is_some());
        assert!(matches!(decision.unwrap().action, ScalingAction::NoOp));
    }

    #[tokio::test]
    async fn test_scaling_metrics_update() {
        let engine = ScalingEngine::new();

        // Should not panic - metrics are updated internally
        engine.update_metrics("pool1", 10, 75.0, 60.0, 8).await;

        // Test passes if no panic occurred
    }

    #[tokio::test]
    async fn test_multiple_policies_prioritize_scale_up() {
        let mut engine = ScalingEngine::new();

        // Add a CPU policy that wants to scale up
        let cpu_policy = ScalingPolicyEnum::CpuUtilization(CpuUtilizationScalingPolicy::new(
            "cpu".to_string(),
            80.0,
            30.0,
            2,
            100,
            5,
        ));

        // Add a queue policy that wants to scale down
        let queue_policy = ScalingPolicyEnum::QueueDepth(QueueDepthScalingPolicy::new(
            "queue".to_string(),
            10,
            3,
            2,
            100,
        ));

        engine.add_policy(cpu_policy).await;
        engine.add_policy(queue_policy).await;

        let context = ScalingContext {
            pool_id: "test-pool".to_string(),
            current_size: 5,
            pending_jobs: 1,
            cpu_utilization: 85.0, // High CPU, wants to scale up
            memory_utilization: 60.0,
            active_workers: 5,
            idle_workers: 0,
            timestamp: Utc::now(),
        };

        let decision = engine.evaluate_scaling(&context).await.unwrap();
        // Should prioritize scale up from CPU policy
        assert!(matches!(decision.action, ScalingAction::ScaleUp(_)));
    }
}
