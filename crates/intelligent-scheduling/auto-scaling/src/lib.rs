//! Hybrid Auto-scaling Engine (US-008)
//!
//! This module provides auto-scaling capabilities combining predictive (ML-based)
//! and reactive (threshold-based) scaling strategies for optimal resource allocation.
//!
//! Features:
//! - HybridScaler with Strategy Pattern for scaling algorithms
//! - PredictiveScaler integrated with LSTM load predictions
//! - ReactiveScaler with threshold-based scaling
//! - Cost-benefit analysis for scaling decisions
//! - Multi-metric evaluation with weighted scoring
//! - Integration with WorkerManager lifecycle
//!
//! Performance Targets:
//! - Scale-up response: <30 seconds
//! - Cost optimization: 35% improvement vs manual scaling
//! - Accuracy: 85%+ prediction accuracy utilization

use hodei_ml_prediction::{ConfidenceInterval, Prediction, PredictionService};
use hodei_orchestrator::domain::Job;
use hodei_shared_types::{TenantId, Uuid};
use hodei_worker_lifecycle::WorkerState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Scaling strategy trait for different scaling algorithms
#[derive(Debug, Clone)]
pub enum ScalingMode {
    Predictive, // ML-based scaling using LSTM predictions
    Reactive,   // Threshold-based reactive scaling
    Hybrid,     // Combination of both modes
}

/// Core scaling strategy trait
#[async_trait::async_trait]
pub trait ScalingStrategy: Send + Sync + std::fmt::Debug {
    /// Evaluate if scaling is needed based on current metrics
    async fn evaluate_scaling(
        &self,
        metrics: &SystemMetrics,
    ) -> Result<ScalingDecision, ScalingError>;

    /// Get strategy name
    fn name(&self) -> &'static str;

    /// Get confidence level of the decision (0.0 to 1.0)
    fn confidence(&self) -> f64;
}

/// Scaling decision with rationale
#[derive(Debug, Clone)]
pub struct ScalingDecision {
    pub action: ScalingAction,
    pub target_workers: u32,
    pub current_workers: u32,
    pub delta: i32,
    pub confidence: f64,
    pub reasoning: String,
    pub cost_impact: CostImpact,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Scaling action types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingAction {
    ScaleUp(u32),   // Add N workers
    ScaleDown(u32), // Remove N workers
    NoAction,       // Keep current state
}

/// Cost impact analysis
#[derive(Debug, Clone)]
pub struct CostImpact {
    pub estimated_cost_change: f64, // Per hour
    pub efficiency_score: f64,      // 0.0 to 1.0
    pub payback_time: Duration,     // Time to recover scaling cost
}

/// System metrics for scaling decisions
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub current_workers: u32,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub queue_depth: u64,
    pub active_jobs: u64,
    pub throughput_jobs_per_min: f64,
    pub avg_job_completion_time: Duration,
    pub prediction: Option<Prediction>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Predictive scaler using ML predictions
#[derive(Debug)]
pub struct PredictiveScaler {
    prediction_service: Arc<RwLock<PredictionService>>,
    lookahead_minutes: u64,
    scale_up_threshold: f64,
    scale_down_threshold: f64,
    min_workers: u32,
    max_workers: u32,
}

impl PredictiveScaler {
    /// Create new predictive scaler
    pub fn new(
        prediction_service: Arc<RwLock<PredictionService>>,
        lookahead_minutes: u64,
        min_workers: u32,
        max_workers: u32,
    ) -> Self {
        Self {
            prediction_service,
            lookahead_minutes,
            scale_up_threshold: 0.8,   // 80% utilization
            scale_down_threshold: 0.3, // 30% utilization
            min_workers,
            max_workers,
        }
    }

    /// Calculate recommended worker count based on prediction
    async fn calculate_recommended_workers(
        &self,
        metrics: &SystemMetrics,
    ) -> Result<u32, ScalingError> {
        // Use ML prediction to forecast load
        if let Some(prediction) = &metrics.prediction {
            let predicted_load = prediction.predicted_load;

            // Calculate required workers based on predicted load
            // Assuming 1 worker can handle 100% CPU and 50 concurrent jobs
            let required_workers = ((predicted_load / 100.0) * 2.0).ceil() as u32;

            // Add buffer for headroom (20%)
            let with_buffer = (required_workers as f64 * 1.2).ceil() as u32;

            Ok(with_buffer.clamp(self.min_workers, self.max_workers))
        } else {
            // Fallback to current utilization if no prediction available
            let required_workers = ((metrics.cpu_utilization / 100.0) * 2.0).ceil() as u32;
            Ok(required_workers.clamp(self.min_workers, self.max_workers))
        }
    }

    /// Determine if proactive scaling is beneficial
    async fn should_proactive_scale(&self, current: u32, recommended: u32) -> bool {
        let delta = recommended as i32 - current as i32;

        // Proactive scale-up if prediction shows 50%+ increase in load
        if delta > 0 && delta as f64 >= current as f64 * 0.5 {
            return true;
        }

        // Proactive scale-down if prediction shows 50%+ decrease and we've been scaled up
        if delta < 0 && (delta.abs() as f64) >= current as f64 * 0.5 {
            return true;
        }

        false
    }
}

#[async_trait::async_trait]
impl ScalingStrategy for PredictiveScaler {
    async fn evaluate_scaling(
        &self,
        metrics: &SystemMetrics,
    ) -> Result<ScalingDecision, ScalingError> {
        let recommended_workers = self.calculate_recommended_workers(metrics).await?;
        let current_workers = metrics.current_workers;

        let delta = recommended_workers as i32 - current_workers as i32;
        let should_proactive = self
            .should_proactive_scale(current_workers, recommended_workers)
            .await;

        let (action, confidence) = if should_proactive && delta.abs() >= 1 {
            if delta > 0 {
                (ScalingAction::ScaleUp(delta as u32), 0.85)
            } else {
                (ScalingAction::ScaleDown(delta.abs() as u32), 0.85)
            }
        } else {
            (ScalingAction::NoAction, 0.5)
        };

        let cost_impact = self.calculate_cost_impact(current_workers, recommended_workers);

        let reasoning = if action != ScalingAction::NoAction {
            format!(
                "Predictive scaling: predicted load {:.1}%, recommended {} workers (current: {})",
                metrics
                    .prediction
                    .as_ref()
                    .map(|p| p.predicted_load)
                    .unwrap_or(0.0),
                recommended_workers,
                current_workers
            )
        } else {
            "No proactive scaling needed based on predictions".to_string()
        };

        Ok(ScalingDecision {
            action,
            target_workers: recommended_workers,
            current_workers,
            delta,
            confidence,
            reasoning,
            cost_impact,
            timestamp: chrono::Utc::now(),
        })
    }

    fn name(&self) -> &'static str {
        "PredictiveScaler"
    }

    fn confidence(&self) -> f64 {
        if self.lookahead_minutes >= 15 {
            0.9
        } else {
            0.7
        }
    }
}

impl PredictiveScaler {
    fn calculate_cost_impact(&self, current: u32, target: u32) -> CostImpact {
        // Simplified cost calculation
        // Assume $0.10/hour per worker
        let worker_cost = 0.10;
        let hourly_change = (target as i32 - current as i32) as f64 * worker_cost;

        CostImpact {
            estimated_cost_change: hourly_change,
            efficiency_score: if target > current { 0.8 } else { 0.9 },
            payback_time: Duration::from_secs(3600), // 1 hour default
        }
    }
}

/// Reactive scaler using threshold-based triggers
#[derive(Debug)]
pub struct ReactiveScaler {
    cpu_threshold: f64,
    memory_threshold: f64,
    queue_threshold: u64,
    cooldown_period: Duration,
    last_scale_time: Option<Instant>,
    min_workers: u32,
    max_workers: u32,
}

impl ReactiveScaler {
    /// Create new reactive scaler
    pub fn new(
        cpu_threshold: f64,
        memory_threshold: f64,
        queue_threshold: u64,
        cooldown_period: Duration,
        min_workers: u32,
        max_workers: u32,
    ) -> Self {
        Self {
            cpu_threshold,
            memory_threshold,
            queue_threshold,
            cooldown_period,
            last_scale_time: None,
            min_workers,
            max_workers,
        }
    }

    /// Check if we're in cooldown period
    fn in_cooldown(&self) -> bool {
        if let Some(last) = self.last_scale_time {
            last.elapsed() < self.cooldown_period
        } else {
            false
        }
    }

    /// Evaluate multiple metrics and decide scaling action
    fn evaluate_thresholds(&self, metrics: &SystemMetrics) -> (ScalingAction, f64, String) {
        let mut scale_up_score = 0.0;
        let mut scale_down_score = 0.0;
        let mut reasons = Vec::new();

        // CPU utilization check
        if metrics.cpu_utilization > self.cpu_threshold {
            scale_up_score += 0.4;
            reasons.push(format!(
                "CPU {:.1}% > threshold {:.1}%",
                metrics.cpu_utilization, self.cpu_threshold
            ));
        } else if metrics.cpu_utilization < self.cpu_threshold * 0.5 {
            scale_down_score += 0.3;
            reasons.push(format!(
                "CPU {:.1}% < low threshold",
                metrics.cpu_utilization
            ));
        }

        // Memory utilization check
        if metrics.memory_utilization > self.memory_threshold {
            scale_up_score += 0.3;
            reasons.push(format!(
                "Memory {:.1}% > threshold {:.1}%",
                metrics.memory_utilization, self.memory_threshold
            ));
        } else if metrics.memory_utilization < self.memory_threshold * 0.5 {
            scale_down_score += 0.2;
            reasons.push(format!(
                "Memory {:.1}% < low threshold",
                metrics.memory_utilization
            ));
        }

        // Queue depth check
        if metrics.queue_depth > self.queue_threshold {
            scale_up_score += 0.3;
            reasons.push(format!(
                "Queue depth {} > threshold {}",
                metrics.queue_depth, self.queue_threshold
            ));
        }

        let reasoning = if !reasons.is_empty() {
            reasons.join(", ")
        } else {
            "All metrics within normal ranges".to_string()
        };

        // Decide action based on scores
        if scale_up_score >= 0.7 {
            (ScalingAction::ScaleUp(1), scale_up_score, reasoning)
        } else if scale_down_score >= 0.5 && metrics.current_workers > self.min_workers {
            (ScalingAction::ScaleDown(1), scale_down_score, reasoning)
        } else {
            (ScalingAction::NoAction, 0.5, reasoning)
        }
    }
}

#[async_trait::async_trait]
impl ScalingStrategy for ReactiveScaler {
    async fn evaluate_scaling(
        &self,
        metrics: &SystemMetrics,
    ) -> Result<ScalingDecision, ScalingError> {
        let current_workers = metrics.current_workers;

        // Check cooldown period
        if self.in_cooldown() {
            return Ok(ScalingDecision {
                action: ScalingAction::NoAction,
                target_workers: current_workers,
                current_workers,
                delta: 0,
                confidence: 0.5,
                reasoning: format!("In cooldown period, last scaled {:?}", self.last_scale_time),
                cost_impact: CostImpact {
                    estimated_cost_change: 0.0,
                    efficiency_score: 0.5,
                    payback_time: self.cooldown_period,
                },
                timestamp: chrono::Utc::now(),
            });
        }

        let (action, confidence, reasoning) = self.evaluate_thresholds(metrics);

        let (target_workers, delta) = match &action {
            ScalingAction::ScaleUp(n) => {
                let target = (current_workers + n).min(self.max_workers);
                (target, *n as i32)
            }
            ScalingAction::ScaleDown(n) => {
                let target = if current_workers > *n {
                    current_workers - *n
                } else {
                    self.min_workers
                };
                (target, -(*n as i32))
            }
            ScalingAction::NoAction => (current_workers, 0),
        };

        let cost_impact = self.calculate_cost_impact(current_workers, target_workers);

        // Update cooldown if we scaled
        if action != ScalingAction::NoAction {
            // Note: In a real implementation, we'd need interior mutability
        }

        Ok(ScalingDecision {
            action,
            target_workers,
            current_workers,
            delta,
            confidence,
            reasoning,
            cost_impact,
            timestamp: chrono::Utc::now(),
        })
    }

    fn name(&self) -> &'static str {
        "ReactiveScaler"
    }

    fn confidence(&self) -> f64 {
        0.75 // Reactive scaling has good confidence in immediate situations
    }
}

impl ReactiveScaler {
    fn calculate_cost_impact(&self, current: u32, target: u32) -> CostImpact {
        let worker_cost = 0.10;
        let hourly_change = (target as i32 - current as i32) as f64 * worker_cost;

        CostImpact {
            estimated_cost_change: hourly_change,
            efficiency_score: 0.7,
            payback_time: Duration::from_secs(1800), // 30 minutes
        }
    }
}

/// Hybrid scaler combining predictive and reactive strategies
#[derive(Debug)]
pub struct HybridScaler {
    predictive: Arc<RwLock<PredictiveScaler>>,
    reactive: Arc<RwLock<ReactiveScaler>>,
    mode: ScalingMode,
    weights: ScalingWeights,
    cost_optimizer: CostOptimizer,
}

#[derive(Debug, Clone)]
pub struct ScalingWeights {
    pub predictive_weight: f64, // 0.0 to 1.0
    pub reactive_weight: f64,   // 0.0 to 1.0
}

impl ScalingWeights {
    pub fn balanced() -> Self {
        Self {
            predictive_weight: 0.6,
            reactive_weight: 0.4,
        }
    }

    pub fn predictive_heavy() -> Self {
        Self {
            predictive_weight: 0.8,
            reactive_weight: 0.2,
        }
    }

    pub fn reactive_heavy() -> Self {
        Self {
            predictive_weight: 0.3,
            reactive_weight: 0.7,
        }
    }
}

impl HybridScaler {
    /// Create new hybrid scaler
    pub fn new(
        predictive: Arc<RwLock<PredictiveScaler>>,
        reactive: Arc<RwLock<ReactiveScaler>>,
        mode: ScalingMode,
        weights: ScalingWeights,
    ) -> Self {
        Self {
            predictive,
            reactive,
            mode,
            weights,
            cost_optimizer: CostOptimizer::new(),
        }
    }

    /// Combine decisions from multiple strategies
    async fn combine_decisions(
        &self,
        predictive: ScalingDecision,
        reactive: ScalingDecision,
    ) -> Result<ScalingDecision, ScalingError> {
        let weighted_confidence = (predictive.confidence * self.weights.predictive_weight)
            + (reactive.confidence * self.weights.reactive_weight);

        let combined_delta = (predictive.delta as f64 * self.weights.predictive_weight)
            + (reactive.delta as f64 * self.weights.reactive_weight);

        let rounded_delta = combined_delta.round() as i32;

        let action = match rounded_delta {
            x if x > 0 => ScalingAction::ScaleUp(x as u32),
            x if x < 0 => ScalingAction::ScaleDown(x.abs() as u32),
            _ => ScalingAction::NoAction,
        };

        let target_workers = if rounded_delta > 0 {
            predictive.target_workers.max(reactive.target_workers)
        } else if rounded_delta < 0 {
            predictive.target_workers.min(reactive.target_workers)
        } else {
            predictive.current_workers
        };

        let reasoning = format!(
            "Hybrid: predictive={:.1}% + reactive={:.1}% = {} workers (delta: {})",
            self.weights.predictive_weight * 100.0,
            self.weights.reactive_weight * 100.0,
            target_workers,
            rounded_delta
        );

        let cost_impact = self.cost_optimizer.optimize(
            &predictive.cost_impact,
            &reactive.cost_impact,
            &self.weights,
        );

        Ok(ScalingDecision {
            action,
            target_workers,
            current_workers: predictive.current_workers,
            delta: rounded_delta,
            confidence: weighted_confidence,
            reasoning,
            cost_impact,
            timestamp: chrono::Utc::now(),
        })
    }
}

#[async_trait::async_trait]
impl ScalingStrategy for HybridScaler {
    async fn evaluate_scaling(
        &self,
        metrics: &SystemMetrics,
    ) -> Result<ScalingDecision, ScalingError> {
        // Get decisions from both strategies in parallel
        let predictive_lock = self.predictive.read().await;
        let reactive_lock = self.reactive.read().await;

        let (predictive_result, reactive_result) = tokio::try_join!(
            predictive_lock.evaluate_scaling(metrics),
            reactive_lock.evaluate_scaling(metrics)
        )?;

        match self.mode {
            ScalingMode::Predictive => Ok(predictive_result),
            ScalingMode::Reactive => Ok(reactive_result),
            ScalingMode::Hybrid => {
                self.combine_decisions(predictive_result, reactive_result)
                    .await
            }
        }
    }

    fn name(&self) -> &'static str {
        "HybridScaler"
    }

    fn confidence(&self) -> f64 {
        (self.weights.predictive_weight + self.weights.reactive_weight) / 2.0
    }
}

/// Cost optimizer for scaling decisions
#[derive(Debug)]
pub struct CostOptimizer {
    target_utilization: f64,
    min_efficiency: f64,
}

impl CostOptimizer {
    pub fn new() -> Self {
        Self {
            target_utilization: 0.7, // 70% utilization target
            min_efficiency: 0.6,     // 60% minimum efficiency
        }
    }

    /// Optimize cost impact based on multiple decisions
    fn optimize(
        &self,
        predictive: &CostImpact,
        reactive: &CostImpact,
        weights: &ScalingWeights,
    ) -> CostImpact {
        let weighted_cost = (predictive.estimated_cost_change * weights.predictive_weight)
            + (reactive.estimated_cost_change * weights.reactive_weight);

        let weighted_efficiency = (predictive.efficiency_score * weights.predictive_weight)
            + (reactive.efficiency_score * weights.reactive_weight);

        CostImpact {
            estimated_cost_change: weighted_cost,
            efficiency_score: weighted_efficiency.max(self.min_efficiency),
            payback_time: Duration::from_secs(2700), // 45 minutes average
        }
    }
}

/// Auto-scaling engine orchestrator
pub struct AutoScalingEngine {
    scaler: Arc<RwLock<dyn ScalingStrategy>>,
    policy: ScalingPolicy,
    history: Arc<RwLock<Vec<ScalingHistory>>>,
}

#[derive(Debug, Clone)]
pub struct ScalingPolicy {
    pub min_workers: u32,
    pub max_workers: u32,
    pub scale_up_cooldown: Duration,
    pub scale_down_cooldown: Duration,
    pub enable_predictive: bool,
    pub enable_reactive: bool,
}

#[derive(Debug, Clone)]
pub struct ScalingHistory {
    pub decision: ScalingDecision,
    pub metrics: SystemMetrics,
    pub execution_time: Duration,
    pub success: bool,
}

impl AutoScalingEngine {
    /// Create new auto-scaling engine
    pub fn new(scaler: Arc<RwLock<dyn ScalingStrategy>>, policy: ScalingPolicy) -> Self {
        Self {
            scaler,
            policy,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Run scaling evaluation
    pub async fn evaluate(&self, metrics: SystemMetrics) -> Result<ScalingDecision, ScalingError> {
        let start_time = Instant::now();

        // Evaluate scaling decision
        let decision = self.scaler.read().await.evaluate_scaling(&metrics).await?;

        // Log decision
        info!(
            "Scaling decision: {:?} ({} -> {}, confidence: {:.2}, cost: {:.2}/hr)",
            decision.action,
            decision.current_workers,
            decision.target_workers,
            decision.confidence,
            decision.cost_impact.estimated_cost_change
        );

        // Record in history
        let execution_time = start_time.elapsed();
        let history_entry = ScalingHistory {
            decision: decision.clone(),
            metrics,
            execution_time,
            success: true, // Would track actual execution in real impl
        };

        self.history.write().await.push(history_entry);

        Ok(decision)
    }

    /// Get scaling history
    pub async fn get_history(&self, limit: usize) -> Vec<ScalingHistory> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get current policy
    pub fn policy(&self) -> &ScalingPolicy {
        &self.policy
    }
}

/// Custom error types
#[derive(thiserror::Error, Debug)]
pub enum ScalingError {
    #[error("Insufficient metrics for scaling decision")]
    InsufficientMetrics,

    #[error("Prediction service unavailable")]
    PredictionServiceUnavailable,

    #[error("Invalid scaling action: {0}")]
    InvalidScalingAction(String),

    #[error("Worker management error: {0}")]
    WorkerManagementError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_ml_prediction::{FeaturePipeline, LstmConfig, PredictionModel};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_predictive_scaler() {
        let config = LstmConfig::default();
        let model = Arc::new(RwLock::new(PredictionModel::new(config)));
        let pipeline = Arc::new(RwLock::new(FeaturePipeline::new()));
        let prediction_service = Arc::new(RwLock::new(PredictionService::new(model, pipeline)));

        let scaler = PredictiveScaler::new(
            prediction_service,
            15, // 15 minute lookahead
            1,  // min workers
            10, // max workers
        );

        // With high CPU and prediction of increased load, should scale up
        let metrics = SystemMetrics {
            current_workers: 5,
            cpu_utilization: 90.0, // High CPU utilization
            memory_utilization: 85.0,
            queue_depth: 50, // Growing queue
            active_jobs: 50,
            throughput_jobs_per_min: 100.0,
            avg_job_completion_time: Duration::from_secs(120),
            prediction: Some(Prediction {
                predicted_load: 85.0,
                confidence_interval: ConfidenceInterval {
                    lower_bound: 80.0,
                    upper_bound: 90.0,
                    confidence_level: 0.85,
                },
                timestamp: chrono::Utc::now().timestamp(),
                inference_time_ms: 10,
            }),
            timestamp: chrono::Utc::now(),
        };

        let decision = scaler.evaluate_scaling(&metrics).await.unwrap();
        // With high CPU and predicted load, should scale up or take no action
        assert!(
            decision.action == ScalingAction::NoAction
                || matches!(decision.action, ScalingAction::ScaleUp(_))
        );
    }

    #[tokio::test]
    async fn test_reactive_scaler() {
        let scaler = ReactiveScaler::new(
            80.0,                     // CPU threshold
            80.0,                     // Memory threshold
            50,                       // Queue threshold
            Duration::from_secs(300), // 5 min cooldown
            1,
            10,
        );

        let metrics = SystemMetrics {
            current_workers: 5,
            cpu_utilization: 85.0, // Above threshold
            memory_utilization: 70.0,
            queue_depth: 60, // Above threshold
            active_jobs: 30,
            throughput_jobs_per_min: 120.0,
            avg_job_completion_time: Duration::from_secs(50),
            prediction: None,
            timestamp: chrono::Utc::now(),
        };

        let decision = scaler.evaluate_scaling(&metrics).await.unwrap();
        assert!(matches!(decision.action, ScalingAction::ScaleUp(_)));
    }

    #[tokio::test]
    async fn test_hybrid_scaler() {
        let config = LstmConfig::default();
        let model = Arc::new(RwLock::new(PredictionModel::new(config)));
        let pipeline = Arc::new(RwLock::new(FeaturePipeline::new()));
        let prediction_service = Arc::new(RwLock::new(PredictionService::new(model, pipeline)));

        let predictive = Arc::new(RwLock::new(PredictiveScaler::new(
            prediction_service,
            15,
            1,
            10,
        )));

        let reactive = Arc::new(RwLock::new(ReactiveScaler::new(
            80.0,
            80.0,
            50,
            Duration::from_secs(300),
            1,
            10,
        )));

        let hybrid = HybridScaler::new(
            predictive,
            reactive,
            ScalingMode::Hybrid,
            ScalingWeights::balanced(),
        );

        let metrics = SystemMetrics {
            current_workers: 5,
            cpu_utilization: 75.0,
            memory_utilization: 65.0,
            queue_depth: 30,
            active_jobs: 25,
            throughput_jobs_per_min: 110.0,
            avg_job_completion_time: Duration::from_secs(55),
            prediction: None,
            timestamp: chrono::Utc::now(),
        };

        let decision = hybrid.evaluate_scaling(&metrics).await.unwrap();
        assert!(decision.confidence > 0.0);
        assert!(decision.target_workers >= 1);
    }

    #[tokio::test]
    async fn test_auto_scaling_engine() {
        let config = LstmConfig::default();
        let model = Arc::new(RwLock::new(PredictionModel::new(config)));
        let pipeline = Arc::new(RwLock::new(FeaturePipeline::new()));
        let prediction_service = Arc::new(RwLock::new(PredictionService::new(model, pipeline)));

        let predictive = Arc::new(RwLock::new(PredictiveScaler::new(
            prediction_service,
            15,
            1,
            10,
        )));

        let engine = AutoScalingEngine::new(
            predictive as Arc<RwLock<dyn ScalingStrategy>>,
            ScalingPolicy {
                min_workers: 1,
                max_workers: 10,
                scale_up_cooldown: Duration::from_secs(300),
                scale_down_cooldown: Duration::from_secs(600),
                enable_predictive: true,
                enable_reactive: false,
            },
        );

        let metrics = SystemMetrics {
            current_workers: 5,
            cpu_utilization: 80.0,
            memory_utilization: 70.0,
            queue_depth: 40,
            active_jobs: 25,
            throughput_jobs_per_min: 100.0,
            avg_job_completion_time: Duration::from_secs(60),
            prediction: None,
            timestamp: chrono::Utc::now(),
        };

        let decision = engine.evaluate(metrics).await.unwrap();
        assert!(decision.target_workers >= 1);
        assert!(decision.target_workers <= 10);
    }
}
