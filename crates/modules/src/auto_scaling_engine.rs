//! Auto-Scaling Policy Engine Module
//!
//! This module provides intelligent auto-scaling for dynamic resource pools
//! based on metrics, predictions, and configurable policies.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Auto-scaling policy
#[derive(Debug, Clone)]
pub struct AutoScalingPolicy {
    pub name: String,
    pub pool_id: String,
    pub triggers: Vec<ScalingTrigger>,
    pub strategy: ScalingStrategy,
    pub constraints: ScalingConstraints,
    pub enabled: bool,
    pub priority: u32, // Higher priority policies are evaluated first
}

impl AutoScalingPolicy {
    pub fn new(
        name: String,
        pool_id: String,
        strategy: ScalingStrategy,
        constraints: ScalingConstraints,
    ) -> Self {
        Self {
            name,
            pool_id,
            triggers: Vec::new(),
            strategy,
            constraints,
            enabled: true,
            priority: 100,
        }
    }

    pub fn add_trigger(&mut self, trigger: ScalingTrigger) {
        self.triggers.push(trigger);
    }
}

/// Scaling trigger types
#[derive(Debug, Clone)]
pub enum ScalingTrigger {
    QueueLength {
        threshold: u32,
        direction: ScaleDirection,
        scale_by: u32,
    },
    CpuUtilization {
        threshold: f64, // 0.0 to 100.0
        direction: ScaleDirection,
        scale_by: u32,
    },
    JobArrivalRate {
        threshold: f64, // jobs per minute
        direction: ScaleDirection,
        scale_by: u32,
    },
    MemoryUtilization {
        threshold: f64, // 0.0 to 100.0
        direction: ScaleDirection,
        scale_by: u32,
    },
    TimeBased {
        cron: String, // Cron expression for scheduled scaling
        action: ScaleAction,
    },
    Custom {
        metric_name: String,
        threshold: f64,
        direction: ScaleDirection,
        scale_by: u32,
    },
}

/// Scale direction
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleDirection {
    ScaleUp,
    ScaleDown,
}

/// Scale action
#[derive(Debug, Clone)]
pub struct ScaleAction {
    pub direction: ScaleDirection,
    pub target_size: Option<u32>,
    pub scale_by: Option<u32>,
}

/// Scaling strategy
#[derive(Debug, Clone)]
pub enum ScalingStrategy {
    Conservative,  // Gradual scaling (small increments)
    Aggressive,    // Faster scaling (large increments)
    Predictive,    // Scale before demand hits
    CostOptimized, // Balance cost and performance
    Custom {
        scale_up_increment: u32,
        scale_down_increment: u32,
        cooldown_period: Duration,
    },
}

/// Scaling constraints
#[derive(Debug, Clone)]
pub struct ScalingConstraints {
    pub min_workers: u32,
    pub max_workers: u32,
    pub default_cooldown: Duration,
    pub max_scale_up_per_minute: u32,
    pub max_scale_down_per_minute: u32,
}

impl ScalingConstraints {
    pub fn new(min_workers: u32, max_workers: u32) -> Self {
        Self {
            min_workers,
            max_workers,
            default_cooldown: Duration::from_secs(60),
            max_scale_up_per_minute: 10,
            max_scale_down_per_minute: 5,
        }
    }

    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.default_cooldown = cooldown;
        self
    }

    pub fn with_rate_limits(mut self, max_up: u32, max_down: u32) -> Self {
        self.max_scale_up_per_minute = max_up;
        self.max_scale_down_per_minute = max_down;
        self
    }
}

/// Historical metrics for prediction
#[derive(Debug, Clone)]
pub struct HistoricalMetric {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub metric_type: String,
}

/// Prediction result
#[derive(Debug, Clone)]
pub struct PredictionResult {
    pub metric_name: String,
    pub predicted_value: f64,
    pub confidence: f64, // 0.0 to 1.0
    pub time_horizon: Duration,
    pub generated_at: DateTime<Utc>,
}

/// Metrics snapshot
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub pool_id: String,
    pub timestamp: DateTime<Utc>,
    pub queue_length: u32,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub active_workers: u32,
    pub idle_workers: u32,
    pub job_arrival_rate: f64, // jobs per minute
    pub custom_metrics: HashMap<String, f64>,
}

impl MetricsSnapshot {
    pub fn new(pool_id: String) -> Self {
        Self {
            pool_id,
            timestamp: Utc::now(),
            queue_length: 0,
            cpu_utilization: 0.0,
            memory_utilization: 0.0,
            active_workers: 0,
            idle_workers: 0,
            job_arrival_rate: 0.0,
            custom_metrics: HashMap::new(),
        }
    }

    pub fn with_values(
        mut self,
        queue_length: u32,
        cpu_utilization: f64,
        active_workers: u32,
    ) -> Self {
        self.queue_length = queue_length;
        self.cpu_utilization = cpu_utilization;
        self.active_workers = active_workers;
        self
    }
}

/// Scaling decision
#[derive(Debug, Clone)]
pub struct ScalingDecision {
    pub policy_name: String,
    pub pool_id: String,
    pub action: ScaleAction,
    pub reason: String,
    pub triggered_by: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub cooldown_until: Option<DateTime<Utc>>,
}

/// Evaluation context
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    pub current_metrics: MetricsSnapshot,
    pub historical_metrics: Vec<HistoricalMetric>,
    pub previous_decisions: Vec<ScalingDecision>,
    pub active_policies: Vec<String>,
}

/// Prediction engine (simplified)
#[derive(Debug)]
pub struct PredictionEngine {
    pub history: Arc<RwLock<HashMap<String, Vec<HistoricalMetric>>>>,
}

impl Default for PredictionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PredictionEngine {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_metric(&self, pool_id: &str, metric: HistoricalMetric) {
        let mut history = self.history.write().await;
        history
            .entry(pool_id.to_string())
            .or_insert_with(Vec::new)
            .push(metric);

        // Keep only last 1000 samples
        if let Some(metrics) = history.get_mut(pool_id)
            && metrics.len() > 1000 {
                metrics.drain(0..metrics.len() - 1000);
            }
    }

    pub async fn predict(
        &self,
        pool_id: &str,
        metric_name: &str,
        time_horizon: Duration,
    ) -> Option<PredictionResult> {
        let history = self.history.read().await;
        let metrics = history.get(pool_id)?;

        // Simple linear regression prediction (simplified)
        if metrics.len() < 2 {
            return None;
        }

        let recent_metrics: Vec<_> = metrics
            .iter()
            .filter(|m| m.metric_type == metric_name)
            .rev()
            .take(10)
            .collect();

        if recent_metrics.is_empty() {
            return None;
        }

        // Calculate average trend
        let values: Vec<f64> = recent_metrics.iter().map(|m| m.value).collect();
        let avg_value = values.iter().copied().sum::<f64>() / values.len() as f64;

        Some(PredictionResult {
            metric_name: metric_name.to_string(),
            predicted_value: avg_value,
            confidence: 0.7, // Simplified confidence
            time_horizon,
            generated_at: Utc::now(),
        })
    }

    pub async fn detect_anomaly(
        &self,
        pool_id: &str,
        metric_name: &str,
        current_value: f64,
    ) -> bool {
        let history = self.history.read().await;
        let metrics = match history.get(pool_id) {
            Some(m) => m,
            None => return false,
        };

        let metric_values: Vec<f64> = metrics
            .iter()
            .filter(|m| m.metric_type == metric_name)
            .map(|m| m.value)
            .collect();

        if metric_values.len() < 10 {
            return false;
        }

        // Calculate mean and standard deviation
        let mean = metric_values.iter().sum::<f64>() / metric_values.len() as f64;
        let variance = metric_values
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / metric_values.len() as f64;
        let std_dev = variance.sqrt();

        // Detect anomaly if value is > 3 standard deviations from mean
        if std_dev == 0.0 {
            return false;
        }

        let z_score = (current_value - mean).abs() / std_dev;
        z_score > 3.0
    }
}

/// Auto-scaling policy engine
#[derive(Debug)]
pub struct AutoScalingPolicyEngine {
    pub policies: Arc<RwLock<HashMap<String, AutoScalingPolicy>>>,
    pub last_decisions: Arc<RwLock<HashMap<String, Instant>>>,
    pub prediction_engine: Arc<PredictionEngine>,
    pub enabled: Arc<RwLock<bool>>,
}

impl AutoScalingPolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            last_decisions: Arc::new(RwLock::new(HashMap::new())),
            prediction_engine: Arc::new(PredictionEngine::new()),
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Add a policy
    pub async fn add_policy(&self, policy: AutoScalingPolicy) {
        let policy_name = policy.name.clone();
        let mut policies = self.policies.write().await;
        policies.insert(policy_name.clone(), policy);
        info!(policy_name = policy_name, "Auto-scaling policy added");
    }

    /// Remove a policy
    pub async fn remove_policy(&self, name: &str) -> Option<AutoScalingPolicy> {
        let mut policies = self.policies.write().await;
        let removed = policies.remove(name);
        if removed.is_some() {
            info!(policy_name = name, "Auto-scaling policy removed");
        }
        removed
    }

    /// Enable/disable engine
    pub async fn set_enabled(&self, enabled: bool) {
        let mut engine_enabled = self.enabled.write().await;
        *engine_enabled = enabled;
        info!(enabled, "Auto-scaling engine state changed");
    }

    /// Check if engine is enabled
    pub async fn is_enabled(&self) -> bool {
        let enabled = self.enabled.read().await;
        *enabled
    }

    /// Evaluate policies for a pool
    pub async fn evaluate_policies(&self, context: EvaluationContext) -> Vec<ScalingDecision> {
        if !self.is_enabled().await {
            return Vec::new();
        }

        let policies = self.policies.read().await;
        let mut decisions = Vec::new();

        // Filter policies for this pool
        let pool_policies: Vec<_> = policies
            .values()
            .filter(|p| p.pool_id == context.current_metrics.pool_id && p.enabled)
            .collect();

        // Sort by priority (higher first)
        let mut sorted_policies = pool_policies.clone();
        sorted_policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        for policy in sorted_policies {
            // Check cooldown
            let now = Instant::now();
            let last_decisions = self.last_decisions.read().await;
            if let Some(last_time) = last_decisions.get(&policy.name) {
                let cooldown = policy.constraints.default_cooldown;
                if now.duration_since(*last_time) < cooldown {
                    continue;
                }
            }
            drop(last_decisions);

            // Evaluate triggers
            let triggered_actions = self.evaluate_triggers(policy, &context).await;

            for action in triggered_actions {
                // Apply constraints
                let constrained_action = self.apply_constraints(policy, action, &context);

                if constrained_action.direction != ScaleDirection::ScaleUp
                    && constrained_action.direction != ScaleDirection::ScaleDown
                {
                    continue;
                }

                // Record decision
                let mut last_decisions = self.last_decisions.write().await;
                last_decisions.insert(policy.name.clone(), now);

                decisions.push(ScalingDecision {
                    policy_name: policy.name.clone(),
                    pool_id: policy.pool_id.clone(),
                    action: constrained_action,
                    reason: format!("Policy '{}' triggered", policy.name),
                    triggered_by: policy.triggers.iter().map(|t| format!("{:?}", t)).collect(),
                    timestamp: Utc::now(),
                    cooldown_until: Some(
                        Utc::now()
                            + chrono::Duration::from_std(policy.constraints.default_cooldown)
                                .unwrap_or_default(),
                    ),
                });
            }
        }

        decisions
    }

    /// Evaluate triggers for a policy
    async fn evaluate_triggers(
        &self,
        policy: &AutoScalingPolicy,
        context: &EvaluationContext,
    ) -> Vec<ScaleAction> {
        let mut actions = Vec::new();
        let metrics = &context.current_metrics;

        for trigger in &policy.triggers {
            match trigger {
                ScalingTrigger::QueueLength {
                    threshold,
                    direction,
                    scale_by,
                } => {
                    if metrics.queue_length > *threshold && *direction == ScaleDirection::ScaleUp
                        || metrics.queue_length <= *threshold
                            && *direction == ScaleDirection::ScaleDown
                    {
                        actions.push(ScaleAction {
                            direction: direction.clone(),
                            target_size: None,
                            scale_by: Some(*scale_by),
                        });
                    }
                }
                ScalingTrigger::CpuUtilization {
                    threshold,
                    direction,
                    scale_by,
                } => {
                    if metrics.cpu_utilization > *threshold && *direction == ScaleDirection::ScaleUp
                        || metrics.cpu_utilization <= *threshold
                            && *direction == ScaleDirection::ScaleDown
                    {
                        actions.push(ScaleAction {
                            direction: direction.clone(),
                            target_size: None,
                            scale_by: Some(*scale_by),
                        });
                    }
                }
                ScalingTrigger::JobArrivalRate {
                    threshold,
                    direction,
                    scale_by,
                } => {
                    if metrics.job_arrival_rate > *threshold
                        && *direction == ScaleDirection::ScaleUp
                        || metrics.job_arrival_rate <= *threshold
                            && *direction == ScaleDirection::ScaleDown
                    {
                        actions.push(ScaleAction {
                            direction: direction.clone(),
                            target_size: None,
                            scale_by: Some(*scale_by),
                        });
                    }
                }
                ScalingTrigger::MemoryUtilization {
                    threshold,
                    direction,
                    scale_by,
                } => {
                    if metrics.memory_utilization > *threshold
                        && *direction == ScaleDirection::ScaleUp
                        || metrics.memory_utilization <= *threshold
                            && *direction == ScaleDirection::ScaleDown
                    {
                        actions.push(ScaleAction {
                            direction: direction.clone(),
                            target_size: None,
                            scale_by: Some(*scale_by),
                        });
                    }
                }
                ScalingTrigger::Custom {
                    metric_name,
                    threshold,
                    direction,
                    scale_by,
                } => {
                    if let Some(value) = metrics.custom_metrics.get(metric_name)
                        && (*value > *threshold && *direction == ScaleDirection::ScaleUp
                            || *value <= *threshold && *direction == ScaleDirection::ScaleDown)
                        {
                            actions.push(ScaleAction {
                                direction: direction.clone(),
                                target_size: None,
                                scale_by: Some(*scale_by),
                            });
                        }
                }
                ScalingTrigger::TimeBased { .. } => {
                    // Time-based triggers would be evaluated by a scheduler
                    // For now, skip them in this implementation
                }
            }
        }

        actions
    }

    /// Apply constraints to a scaling action
    fn apply_constraints(
        &self,
        policy: &AutoScalingPolicy,
        mut action: ScaleAction,
        context: &EvaluationContext,
    ) -> ScaleAction {
        let current_size = context.current_metrics.active_workers;
        let constraints = &policy.constraints;

        // Ensure we don't exceed max or go below min
        match action.direction {
            ScaleDirection::ScaleUp => {
                if let Some(scale_by) = action.scale_by {
                    let target = (current_size + scale_by).min(constraints.max_workers);
                    action.scale_by = Some(target.saturating_sub(current_size));
                }
                if let Some(target_size) = action.target_size {
                    action.target_size = Some(target_size.min(constraints.max_workers));
                }
            }
            ScaleDirection::ScaleDown => {
                if let Some(scale_by) = action.scale_by {
                    let target = current_size.saturating_sub(scale_by);
                    action.scale_by =
                        Some(current_size.saturating_sub(target.min(constraints.min_workers)));
                }
                if let Some(target_size) = action.target_size {
                    action.target_size = Some(target_size.max(constraints.min_workers));
                }
            }
        }

        // Apply strategy modifiers
        match &policy.strategy {
            ScalingStrategy::Conservative => {
                if let Some(scale_by) = &mut action.scale_by {
                    *scale_by = (*scale_by).min(2);
                }
            }
            ScalingStrategy::Aggressive => {
                // No modification needed, aggressive can scale large amounts
            }
            ScalingStrategy::Predictive => {
                // Predictive could increase scale-up to be proactive
                if let ScaleDirection::ScaleUp = action.direction
                    && let Some(scale_by) = &mut action.scale_by {
                        *scale_by = (*scale_by * 2).min(20);
                    }
            }
            ScalingStrategy::CostOptimized => {
                // Conservative on scale-up, slightly more aggressive on scale-down
                if let ScaleDirection::ScaleUp = action.direction
                    && let Some(scale_by) = &mut action.scale_by {
                        *scale_by = (*scale_by).min(3);
                    }
            }
            ScalingStrategy::Custom {
                scale_up_increment,
                scale_down_increment,
                ..
            } => match action.direction {
                ScaleDirection::ScaleUp => {
                    if action.scale_by.is_none() {
                        action.scale_by = Some(*scale_up_increment);
                    }
                }
                ScaleDirection::ScaleDown => {
                    if action.scale_by.is_none() {
                        action.scale_by = Some(*scale_down_increment);
                    }
                }
            },
        }

        action
    }

    /// Get active policies for a pool
    pub async fn get_policies_for_pool(&self, pool_id: &str) -> Vec<AutoScalingPolicy> {
        let policies = self.policies.read().await;
        policies
            .values()
            .filter(|p| p.pool_id == pool_id && p.enabled)
            .cloned()
            .collect()
    }

    /// Enable/disable a policy
    pub async fn set_policy_enabled(&self, name: &str, enabled: bool) {
        let mut policies = self.policies.write().await;
        if let Some(policy) = policies.get_mut(name) {
            policy.enabled = enabled;
            info!(policy_name = name, enabled, "Policy state changed");
        }
    }
}

impl Default for AutoScalingPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors
#[derive(Error, Debug)]
pub enum AutoScalingError {
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Invalid constraint: {0}")]
    InvalidConstraint(String),

    #[error("Evaluation failed: {0}")]
    EvaluationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let engine = AutoScalingPolicyEngine::new();
        assert!(engine.is_enabled().await);
    }

    #[tokio::test]
    async fn test_add_and_remove_policy() {
        let engine = AutoScalingPolicyEngine::new();

        let mut policy = AutoScalingPolicy::new(
            "test-policy".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );

        policy.add_trigger(ScalingTrigger::QueueLength {
            threshold: 10,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
        });

        engine.add_policy(policy.clone()).await;

        let policies = engine.get_policies_for_pool("pool-1").await;
        assert_eq!(policies.len(), 1);

        let removed = engine.remove_policy("test-policy").await;
        assert!(removed.is_some());

        let policies = engine.get_policies_for_pool("pool-1").await;
        assert_eq!(policies.len(), 0);
    }

    #[tokio::test]
    async fn test_queue_length_trigger_scale_up() {
        let engine = AutoScalingPolicyEngine::new();

        let mut policy = AutoScalingPolicy::new(
            "queue-policy".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );

        policy.add_trigger(ScalingTrigger::QueueLength {
            threshold: 10,
            direction: ScaleDirection::ScaleUp,
            scale_by: 5,
        });

        engine.add_policy(policy).await;

        let metrics = MetricsSnapshot::new("pool-1".to_string()).with_values(15, 50.0, 5); // Queue length > threshold

        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context).await;
        assert_eq!(decisions.len(), 1);
        assert!(matches!(
            decisions[0].action.direction,
            ScaleDirection::ScaleUp
        ));
    }

    #[tokio::test]
    async fn test_cpu_utilization_trigger_scale_down() {
        let engine = AutoScalingPolicyEngine::new();

        let mut policy = AutoScalingPolicy::new(
            "cpu-policy".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );

        policy.add_trigger(ScalingTrigger::CpuUtilization {
            threshold: 80.0,
            direction: ScaleDirection::ScaleDown,
            scale_by: 3,
        });

        engine.add_policy(policy).await;

        let metrics = MetricsSnapshot::new("pool-1".to_string()).with_values(5, 30.0, 10); // CPU < threshold

        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context).await;
        assert_eq!(decisions.len(), 1);
        assert!(matches!(
            decisions[0].action.direction,
            ScaleDirection::ScaleDown
        ));
    }

    #[tokio::test]
    async fn test_custom_metric_trigger() {
        let engine = AutoScalingPolicyEngine::new();

        let mut policy = AutoScalingPolicy::new(
            "custom-policy".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );

        policy.add_trigger(ScalingTrigger::Custom {
            metric_name: "request_rate".to_string(),
            threshold: 100.0,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
        });

        engine.add_policy(policy).await;

        let mut metrics = MetricsSnapshot::new("pool-1".to_string());
        metrics.queue_length = 5;
        metrics.cpu_utilization = 50.0;
        metrics.active_workers = 5;
        metrics
            .custom_metrics
            .insert("request_rate".to_string(), 150.0);

        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context).await;
        assert_eq!(decisions.len(), 1);
        assert!(matches!(
            decisions[0].action.direction,
            ScaleDirection::ScaleUp
        ));
    }

    #[tokio::test]
    async fn test_constraints_enforced() {
        let engine = AutoScalingPolicyEngine::new();

        let mut policy = AutoScalingPolicy::new(
            "test-policy".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(5, 20), // Min 5, Max 20
        );

        policy.add_trigger(ScalingTrigger::QueueLength {
            threshold: 10,
            direction: ScaleDirection::ScaleUp,
            scale_by: 50, // This exceeds max
        });

        engine.add_policy(policy).await;

        let metrics = MetricsSnapshot::new("pool-1".to_string()).with_values(15, 50.0, 15);

        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context).await;
        assert_eq!(decisions.len(), 1);

        // Scale by should be limited to max_workers - current_size = 20 - 15 = 5
        let scale_by = decisions[0].action.scale_by.unwrap();
        assert!(scale_by <= 5);
    }

    #[tokio::test]
    async fn test_strategy_modifiers() {
        let engine = AutoScalingPolicyEngine::new();

        // Test Conservative strategy
        let mut conservative_policy = AutoScalingPolicy::new(
            "conservative".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );

        conservative_policy.add_trigger(ScalingTrigger::QueueLength {
            threshold: 10,
            direction: ScaleDirection::ScaleUp,
            scale_by: 10, // Will be limited by Conservative strategy
        });

        engine.add_policy(conservative_policy).await;

        let metrics = MetricsSnapshot::new("pool-1".to_string()).with_values(15, 50.0, 5);

        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context).await;
        assert_eq!(decisions.len(), 1);

        // Conservative should limit to max 2
        let scale_by = decisions[0].action.scale_by.unwrap();
        assert!(scale_by <= 2);
    }

    #[tokio::test]
    async fn test_engine_enabled_state() {
        let engine = AutoScalingPolicyEngine::new();

        let mut policy = AutoScalingPolicy::new(
            "test-policy".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );

        policy.add_trigger(ScalingTrigger::QueueLength {
            threshold: 10,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
        });

        engine.add_policy(policy).await;

        // Engine enabled - should evaluate
        let metrics = MetricsSnapshot::new("pool-1".to_string()).with_values(15, 50.0, 5);
        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context.clone()).await;
        assert_eq!(decisions.len(), 1);

        // Disable engine - should not evaluate
        engine.set_enabled(false).await;
        let decisions = engine.evaluate_policies(context).await;
        assert!(decisions.is_empty());
    }

    #[tokio::test]
    async fn test_policy_enable_disable() {
        let engine = AutoScalingPolicyEngine::new();

        let policy = AutoScalingPolicy::new(
            "test-policy".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );

        engine.add_policy(policy).await;

        // Disable policy
        engine.set_policy_enabled("test-policy", false).await;

        let metrics = MetricsSnapshot::new("pool-1".to_string()).with_values(15, 50.0, 5);
        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context).await;
        assert!(decisions.is_empty());
    }

    #[tokio::test]
    async fn test_prediction_engine() {
        let engine = AutoScalingPolicyEngine::new();

        // Add some historical metrics
        for i in 0..10 {
            let metric = HistoricalMetric {
                timestamp: Utc::now() - chrono::Duration::minutes(i),
                value: 10.0 + i as f64,
                metric_type: "cpu_utilization".to_string(),
            };
            engine.prediction_engine.add_metric("pool-1", metric).await;
        }

        let prediction = engine
            .prediction_engine
            .predict("pool-1", "cpu_utilization", Duration::from_secs(300))
            .await;

        assert!(prediction.is_some());
        let pred = prediction.unwrap();
        assert_eq!(pred.metric_name, "cpu_utilization");
    }

    #[tokio::test]
    async fn test_multiple_policies_same_pool() {
        let engine = AutoScalingPolicyEngine::new();

        // Add two policies with different priorities
        let mut policy1 = AutoScalingPolicy::new(
            "policy-1".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Conservative,
            ScalingConstraints::new(1, 100),
        );
        policy1.priority = 100;
        policy1.add_trigger(ScalingTrigger::QueueLength {
            threshold: 10,
            direction: ScaleDirection::ScaleUp,
            scale_by: 5,
        });

        let mut policy2 = AutoScalingPolicy::new(
            "policy-2".to_string(),
            "pool-1".to_string(),
            ScalingStrategy::Aggressive,
            ScalingConstraints::new(1, 100),
        );
        policy2.priority = 200; // Higher priority
        policy2.add_trigger(ScalingTrigger::CpuUtilization {
            threshold: 80.0,
            direction: ScaleDirection::ScaleUp,
            scale_by: 10,
        });

        engine.add_policy(policy1).await;
        engine.add_policy(policy2).await;

        let metrics = MetricsSnapshot::new("pool-1".to_string()).with_values(15, 50.0, 5); // Queue length > threshold, CPU < threshold

        let context = EvaluationContext {
            current_metrics: metrics,
            historical_metrics: Vec::new(),
            previous_decisions: Vec::new(),
            active_policies: Vec::new(),
        };

        let decisions = engine.evaluate_policies(context).await;
        // Only queue policy should trigger (CPU threshold not met)
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].policy_name, "policy-1");
    }
}
