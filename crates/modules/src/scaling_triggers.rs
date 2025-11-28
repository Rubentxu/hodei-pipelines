//! Scaling Triggers and Policies Module
//!
//! This module provides configurable scaling triggers, evaluation intervals,
//! and policy validation for dynamic resource pools.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::error;

pub use crate::auto_scaling_engine::ScaleDirection;
use crate::metrics_collection::{MetricType, RealTimeSnapshot};
use crate::scaling_policies::ScalingError;

/// Time-based scaling trigger
#[derive(Debug, Clone)]
pub struct TimeBasedTrigger {
    pub cron_expression: String,
    pub scale_by: u32,
    pub direction: ScaleDirection,
    pub enabled: bool,
}

/// Event-based scaling trigger
#[derive(Debug, Clone)]
pub struct EventBasedTrigger {
    pub event_type: String,
    pub conditions: Vec<EventCondition>,
    pub scale_by: u32,
    pub direction: ScaleDirection,
    pub enabled: bool,
}

/// Event condition for trigger evaluation
#[derive(Debug, Clone)]
pub struct EventCondition {
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: String,
}

/// Trigger evaluation result
#[derive(Debug, Clone)]
pub struct TriggerEvaluationResult {
    pub trigger_id: String,
    pub triggered: bool,
    pub current_value: f64,
    pub threshold: f64,
    pub direction: ScaleDirection,
    pub scale_by: u32,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Comparison operator
#[derive(Debug, Clone)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

/// Composite trigger combining multiple triggers
#[derive(Debug, Clone)]
pub struct CompositeTrigger {
    pub id: String,
    pub name: String,
    pub operators: Vec<CompositeOperator>,
    pub triggers: Vec<Trigger>,
    pub enabled: bool,
}

/// Composite operator for combining triggers
#[derive(Debug, Clone)]
pub enum CompositeOperator {
    All,  // All triggers must fire
    Any,  // Any trigger can fire
    None, // No triggers should fire
}

/// Trigger evaluation engine
#[derive(Debug)]
pub struct TriggerEvaluationEngine {
    config: TriggerEvaluationConfig,
    evaluation_history: Arc<RwLock<HashMap<String, Vec<TriggerEvaluationResult>>>>,
}

/// Trigger evaluation configuration
#[derive(Debug, Clone)]
pub struct TriggerEvaluationConfig {
    pub evaluation_interval: Duration,
    pub evaluation_timeout: Duration,
    pub cooldown_period: Duration,
    pub max_trigger_rate: u32, // triggers per minute
    pub enabled: bool,
}

/// Individual trigger configuration
#[derive(Debug, Clone)]
pub struct Trigger {
    pub id: String,
    pub name: String,
    pub trigger_type: TriggerType,
    pub enabled: bool,
    pub threshold: f64,
    pub direction: ScaleDirection,
    pub scale_by: u32,
    pub cooldown: Option<Duration>,
    pub evaluation_interval: Duration,
    pub last_evaluation: Option<DateTime<Utc>>,
    pub last_triggered: Option<DateTime<Utc>>,
}

/// Trigger type
#[derive(Debug, Clone)]
pub enum TriggerType {
    MetricBased(MetricTrigger),
    TimeBased(TimeBasedTrigger),
    EventBased(EventBasedTrigger),
    Custom(CustomTrigger),
}

/// Metric-based trigger
#[derive(Debug, Clone)]
pub struct MetricTrigger {
    pub metric_type: MetricType,
    pub operator: ComparisonOperator,
    pub threshold: f64,
    pub evaluation_window: Option<Duration>,
}

/// Custom trigger
#[derive(Debug, Clone)]
pub struct CustomTrigger {
    pub name: String,
    pub handler: String, // Handler name for custom evaluation
}

/// Trigger error types
#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
    #[error("Trigger evaluation error: {0}")]
    EvaluationError(String),
    #[error("Trigger validation error: {0}")]
    ValidationError(String),
    #[error("Trigger configuration error: {0}")]
    ConfigurationError(String),
    #[error("Metric not found: {0}")]
    MetricNotFound(String),
    #[error("Cooldown violation: {0}")]
    CooldownViolation(String),
}

impl TriggerEvaluationEngine {
    /// Create a new trigger evaluation engine
    pub fn new(config: TriggerEvaluationConfig) -> Self {
        Self {
            config,
            evaluation_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Evaluate a single trigger
    pub async fn evaluate_trigger(
        &self,
        trigger: &Trigger,
        metrics: &RealTimeSnapshot,
    ) -> Result<TriggerEvaluationResult, ScalingError> {
        // Check if trigger is enabled
        if !trigger.enabled {
            return Ok(TriggerEvaluationResult {
                trigger_id: trigger.id.clone(),
                triggered: false,
                current_value: 0.0,
                threshold: trigger.threshold,
                direction: trigger.direction.clone(),
                scale_by: trigger.scale_by,
                reason: "Trigger is disabled".to_string(),
                timestamp: Utc::now(),
            });
        }

        // Check cooldown period
        if let Some(cooldown) = trigger.cooldown
            && let Some(last_triggered) = trigger.last_triggered {
                let cooldown_duration = chrono::Duration::from_std(cooldown)
                    .map_err(|e| ScalingError::ConfigurationError(e.to_string()))?;
                if Utc::now().signed_duration_since(last_triggered) < cooldown_duration {
                    return Ok(TriggerEvaluationResult {
                        trigger_id: trigger.id.clone(),
                        triggered: false,
                        current_value: 0.0,
                        threshold: trigger.threshold,
                        direction: trigger.direction.clone(),
                        scale_by: trigger.scale_by,
                        reason: "Cooldown period active".to_string(),
                        timestamp: Utc::now(),
                    });
                }
            }

        // Evaluate based on trigger type
        let (triggered, current_value) = match &trigger.trigger_type {
            TriggerType::MetricBased(metric_trigger) => {
                self.evaluate_metric_trigger(metric_trigger, metrics)?
            }
            TriggerType::TimeBased(time_trigger) => self.evaluate_time_trigger(time_trigger)?,
            TriggerType::EventBased(event_trigger) => {
                self.evaluate_event_trigger(event_trigger).await?
            }
            TriggerType::Custom(custom_trigger) => {
                self.evaluate_custom_trigger(custom_trigger).await?
            }
        };

        // Record evaluation result
        self.record_evaluation(&trigger.id, triggered, current_value, trigger)
            .await;

        Ok(TriggerEvaluationResult {
            trigger_id: trigger.id.clone(),
            triggered,
            current_value,
            threshold: trigger.threshold,
            direction: trigger.direction.clone(),
            scale_by: trigger.scale_by,
            reason: if triggered {
                format!(
                    "Trigger fired: {} {} {}",
                    current_value,
                    self.operator_to_string(trigger),
                    trigger.threshold
                )
            } else {
                "Trigger condition not met".to_string()
            },
            timestamp: Utc::now(),
        })
    }

    /// Evaluate a metric-based trigger
    fn evaluate_metric_trigger(
        &self,
        metric_trigger: &MetricTrigger,
        metrics: &RealTimeSnapshot,
    ) -> Result<(bool, f64), ScalingError> {
        // Get metric value
        let current_value = metrics
            .metrics
            .get(&metric_trigger.metric_type)
            .copied()
            .ok_or_else(|| {
                ScalingError::MetricNotFound(format!("{:?}", metric_trigger.metric_type))
            })?;

        // Evaluate condition
        let triggered = self
            .compare_values(
                current_value,
                &metric_trigger.operator,
                metric_trigger.threshold,
            )
            .map_err(|e| ScalingError::EvaluationError(e.to_string()))?;

        Ok((triggered, current_value))
    }

    /// Evaluate a time-based trigger
    fn evaluate_time_trigger(
        &self,
        _time_trigger: &TimeBasedTrigger,
    ) -> Result<(bool, f64), ScalingError> {
        // Time-based triggers are evaluated by a scheduler
        // For now, return false
        Ok((false, 0.0))
    }

    /// Evaluate an event-based trigger
    async fn evaluate_event_trigger(
        &self,
        _event_trigger: &EventBasedTrigger,
    ) -> Result<(bool, f64), ScalingError> {
        // Event-based triggers would be evaluated based on events
        // For now, return false
        Ok((false, 0.0))
    }

    /// Evaluate a custom trigger
    async fn evaluate_custom_trigger(
        &self,
        _custom_trigger: &CustomTrigger,
    ) -> Result<(bool, f64), ScalingError> {
        // Custom triggers would be handled by custom evaluation logic
        // For now, return false
        Ok((false, 0.0))
    }

    /// Compare values using operator
    fn compare_values(
        &self,
        left: f64,
        operator: &ComparisonOperator,
        right: f64,
    ) -> Result<bool, ScalingError> {
        match operator {
            ComparisonOperator::GreaterThan => Ok(left > right),
            ComparisonOperator::LessThan => Ok(left < right),
            ComparisonOperator::GreaterThanOrEqual => Ok(left >= right),
            ComparisonOperator::LessThanOrEqual => Ok(left <= right),
            ComparisonOperator::Equal => Ok((left - right).abs() < f64::EPSILON),
            ComparisonOperator::NotEqual => Ok((left - right).abs() >= f64::EPSILON),
        }
    }

    /// Convert operator to string
    fn operator_to_string(&self, trigger: &Trigger) -> String {
        match &trigger.trigger_type {
            TriggerType::MetricBased(metric_trigger) => match metric_trigger.operator {
                ComparisonOperator::GreaterThan => ">".to_string(),
                ComparisonOperator::LessThan => "<".to_string(),
                ComparisonOperator::GreaterThanOrEqual => ">=".to_string(),
                ComparisonOperator::LessThanOrEqual => "<=".to_string(),
                ComparisonOperator::Equal => "==".to_string(),
                ComparisonOperator::NotEqual => "!=".to_string(),
            },
            _ => "unknown".to_string(),
        }
    }

    /// Record evaluation in history
    async fn record_evaluation(
        &self,
        trigger_id: &str,
        triggered: bool,
        current_value: f64,
        trigger: &Trigger,
    ) {
        let mut history = self.evaluation_history.write().await;
        history
            .entry(trigger_id.to_string())
            .or_insert_with(Vec::new)
            .push(TriggerEvaluationResult {
                trigger_id: trigger_id.to_string(),
                triggered,
                current_value,
                threshold: trigger.threshold,
                direction: trigger.direction.clone(),
                scale_by: trigger.scale_by,
                reason: if triggered {
                    "Trigger fired".to_string()
                } else {
                    "No trigger".to_string()
                },
                timestamp: Utc::now(),
            });

        // Keep only last 100 evaluations
        if let Some(evaluations) = history.get_mut(trigger_id)
            && evaluations.len() > 100 {
                evaluations.drain(0..evaluations.len() - 100);
            }
    }

    /// Get evaluation history for a trigger
    pub async fn get_evaluation_history(
        &self,
        trigger_id: &str,
    ) -> Option<Vec<TriggerEvaluationResult>> {
        let history = self.evaluation_history.read().await;
        history.get(trigger_id).cloned()
    }

    /// Get trigger statistics
    pub async fn get_trigger_stats(&self, trigger_id: &str) -> Option<TriggerStats> {
        let history = self.evaluation_history.read().await;
        let evaluations = history.get(trigger_id)?;

        let total = evaluations.len() as u64;
        let triggered = evaluations.iter().filter(|e| e.triggered).count() as f64;
        let trigger_rate = if total > 0 {
            triggered / total as f64
        } else {
            0.0
        };

        let avg_threshold = if total > 0 {
            evaluations.iter().map(|e| e.threshold).sum::<f64>() / total as f64
        } else {
            0.0
        };

        Some(TriggerStats {
            total_evaluations: total,
            trigger_rate,
            avg_threshold,
        })
    }
}

/// Trigger statistics
#[derive(Debug, Clone)]
pub struct TriggerStats {
    pub total_evaluations: u64,
    pub trigger_rate: f64, // percentage of evaluations that triggered (0.0 - 1.0)
    pub avg_threshold: f64,
}

/// Create a default trigger evaluation configuration
impl Default for TriggerEvaluationConfig {
    fn default() -> Self {
        Self {
            evaluation_interval: Duration::from_secs(30),
            evaluation_timeout: Duration::from_secs(5),
            cooldown_period: Duration::from_secs(60),
            max_trigger_rate: 10,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics_collection::MetricsConfig;

    #[tokio::test]
    async fn test_trigger_evaluation_engine_creation() {
        let config = TriggerEvaluationConfig::default();
        let engine = TriggerEvaluationEngine::new(config);
        assert!(engine.config.enabled);
        assert_eq!(engine.config.evaluation_interval, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_metric_trigger_evaluation() {
        let config = TriggerEvaluationConfig::default();
        let engine = TriggerEvaluationEngine::new(config);

        let mut snapshot = RealTimeSnapshot {
            pool_id: "test-pool".to_string(),
            timestamp: Utc::now(),
            metrics: HashMap::new(),
        };
        snapshot.metrics.insert(MetricType::CpuUtilization, 85.0);

        let trigger = Trigger {
            id: "test-trigger".to_string(),
            name: "CPU High".to_string(),
            trigger_type: TriggerType::MetricBased(MetricTrigger {
                metric_type: MetricType::CpuUtilization,
                operator: ComparisonOperator::GreaterThan,
                threshold: 80.0,
                evaluation_window: None,
            }),
            enabled: true,
            threshold: 80.0,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
            cooldown: Some(Duration::from_secs(60)),
            evaluation_interval: Duration::from_secs(30),
            last_evaluation: None,
            last_triggered: None,
        };

        let result = engine.evaluate_trigger(&trigger, &snapshot).await.unwrap();
        assert!(result.triggered);
        assert_eq!(result.current_value, 85.0);
        assert_eq!(result.threshold, 80.0);
    }

    #[tokio::test]
    async fn test_trigger_cooldown() {
        let config = TriggerEvaluationConfig::default();
        let engine = TriggerEvaluationEngine::new(config);

        let mut snapshot = RealTimeSnapshot {
            pool_id: "test-pool".to_string(),
            timestamp: Utc::now(),
            metrics: HashMap::new(),
        };
        snapshot.metrics.insert(MetricType::CpuUtilization, 90.0);

        let mut trigger = Trigger {
            id: "test-trigger".to_string(),
            name: "CPU High".to_string(),
            trigger_type: TriggerType::MetricBased(MetricTrigger {
                metric_type: MetricType::CpuUtilization,
                operator: ComparisonOperator::GreaterThan,
                threshold: 80.0,
                evaluation_window: None,
            }),
            enabled: true,
            threshold: 80.0,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
            cooldown: Some(Duration::from_secs(10)),
            evaluation_interval: Duration::from_secs(30),
            last_evaluation: None,
            last_triggered: Some(Utc::now()),
        };

        let result = engine.evaluate_trigger(&trigger, &snapshot).await.unwrap();
        assert!(!result.triggered);
        assert!(result.reason.contains("Cooldown"));

        trigger.last_triggered = Some(Utc::now() - chrono::Duration::seconds(11));
        let result = engine.evaluate_trigger(&trigger, &snapshot).await.unwrap();
        assert!(result.triggered);
    }

    #[tokio::test]
    async fn test_trigger_comparison_operators() {
        let config = TriggerEvaluationConfig::default();
        let engine = TriggerEvaluationEngine::new(config);

        let mut snapshot = RealTimeSnapshot {
            pool_id: "test-pool".to_string(),
            timestamp: Utc::now(),
            metrics: HashMap::new(),
        };
        snapshot.metrics.insert(MetricType::QueueLength, 50.0);

        // Greater than
        let triggered = engine
            .compare_values(60.0, &ComparisonOperator::GreaterThan, 50.0)
            .unwrap();
        assert!(triggered);

        // Less than
        let triggered = engine
            .compare_values(40.0, &ComparisonOperator::LessThan, 50.0)
            .unwrap();
        assert!(triggered);

        // Equal
        let triggered = engine
            .compare_values(50.0, &ComparisonOperator::Equal, 50.0)
            .unwrap();
        assert!(triggered);

        // Not equal
        let triggered = engine
            .compare_values(60.0, &ComparisonOperator::NotEqual, 50.0)
            .unwrap();
        assert!(triggered);
    }

    #[tokio::test]
    async fn test_trigger_evaluation_history() {
        let config = TriggerEvaluationConfig::default();
        let engine = TriggerEvaluationEngine::new(config);

        let mut snapshot = RealTimeSnapshot {
            pool_id: "test-pool".to_string(),
            timestamp: Utc::now(),
            metrics: HashMap::new(),
        };
        snapshot.metrics.insert(MetricType::CpuUtilization, 85.0);

        let trigger = Trigger {
            id: "test-trigger".to_string(),
            name: "Test".to_string(),
            trigger_type: TriggerType::MetricBased(MetricTrigger {
                metric_type: MetricType::CpuUtilization,
                operator: ComparisonOperator::GreaterThan,
                threshold: 80.0,
                evaluation_window: None,
            }),
            enabled: true,
            threshold: 80.0,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
            cooldown: None,
            evaluation_interval: Duration::from_secs(30),
            last_evaluation: None,
            last_triggered: None,
        };

        // Evaluate trigger multiple times
        for _ in 0..5 {
            let _ = engine.evaluate_trigger(&trigger, &snapshot).await;
        }

        let history = engine.get_evaluation_history("test-trigger").await.unwrap();
        assert_eq!(history.len(), 5);
    }

    #[tokio::test]
    async fn test_trigger_stats() {
        let config = TriggerEvaluationConfig::default();
        let engine = TriggerEvaluationEngine::new(config);

        let snapshot = RealTimeSnapshot {
            pool_id: "test-pool".to_string(),
            timestamp: Utc::now(),
            metrics: HashMap::new(),
        };

        let trigger = Trigger {
            id: "test-trigger".to_string(),
            name: "Test".to_string(),
            trigger_type: TriggerType::MetricBased(MetricTrigger {
                metric_type: MetricType::CpuUtilization,
                operator: ComparisonOperator::GreaterThan,
                threshold: 80.0,
                evaluation_window: None,
            }),
            enabled: true,
            threshold: 80.0,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
            cooldown: None,
            evaluation_interval: Duration::from_secs(30),
            last_evaluation: None,
            last_triggered: None,
        };

        // Evaluate trigger 10 times
        for i in 0..10 {
            let mut snapshot = snapshot.clone();
            snapshot
                .metrics
                .insert(MetricType::CpuUtilization, if i < 5 { 85.0 } else { 75.0 });
            let _ = engine.evaluate_trigger(&trigger, &snapshot).await;
        }

        let stats = engine.get_trigger_stats("test-trigger").await.unwrap();
        assert_eq!(stats.total_evaluations, 10);
        assert!((stats.trigger_rate - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_disabled_trigger() {
        let config = TriggerEvaluationConfig::default();
        let engine = TriggerEvaluationEngine::new(config);

        let mut snapshot = RealTimeSnapshot {
            pool_id: "test-pool".to_string(),
            timestamp: Utc::now(),
            metrics: HashMap::new(),
        };
        snapshot.metrics.insert(MetricType::CpuUtilization, 90.0);

        let trigger = Trigger {
            id: "test-trigger".to_string(),
            name: "Test".to_string(),
            trigger_type: TriggerType::MetricBased(MetricTrigger {
                metric_type: MetricType::CpuUtilization,
                operator: ComparisonOperator::GreaterThan,
                threshold: 80.0,
                evaluation_window: None,
            }),
            enabled: false, // Disabled
            threshold: 80.0,
            direction: ScaleDirection::ScaleUp,
            scale_by: 2,
            cooldown: None,
            evaluation_interval: Duration::from_secs(30),
            last_evaluation: None,
            last_triggered: None,
        };

        let result = engine.evaluate_trigger(&trigger, &snapshot).await.unwrap();
        assert!(!result.triggered);
        assert!(result.reason.contains("disabled"));
    }
}
