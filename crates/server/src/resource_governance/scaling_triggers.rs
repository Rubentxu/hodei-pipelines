//! Scaling Triggers API
//!
//! This module provides endpoints for managing scaling triggers that activate
//! scaling policies based on various metrics and conditions.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Trigger types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalingTriggerType {
    Threshold,
    TimeBased,
    MetricChange,
    Concurrency,
}

/// Metric types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    CpuUtilization,
    MemoryUtilization,
    QueueDepth,
    JobsPerMinute,
    ErrorRate,
    ResponseTime,
    Custom(String),
}

/// Threshold trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdTriggerConfig {
    pub metric_type: MetricType,
    pub comparison_operator: String, // greater_than, less_than, equal, not_equal
    pub threshold_value: f64,
    pub evaluation_period: Duration,
    pub consecutive_periods: u32,
}

/// Time-based trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedTriggerConfig {
    pub schedule_type: String,       // cron, recurring, once
    pub schedule_expression: String, // "0 9 * * 1-5" for weekdays at 9 AM
    pub target_capacity: i32,
    pub duration: Option<Duration>, // How long to maintain the capacity
}

/// Metric change trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricChangeTriggerConfig {
    pub metric_type: MetricType,
    pub change_type: String, // percent_change, absolute_change
    pub change_value: f64,
    pub direction: String, // increase, decrease, either
    pub min_change_interval: Duration,
}

/// Concurrency trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyTriggerConfig {
    pub metric_type: MetricType,
    pub window_size: Duration,
    pub threshold_value: f64,
    pub comparison_operator: String,
}

/// Scaling trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingTrigger {
    pub trigger_id: String,
    pub name: String,
    pub trigger_type: ScalingTriggerType,
    pub pool_id: String,
    pub policy_id: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    // Configuration based on trigger type
    pub threshold_config: Option<ThresholdTriggerConfig>,
    pub time_based_config: Option<TimeBasedTriggerConfig>,
    pub metric_change_config: Option<MetricChangeTriggerConfig>,
    pub concurrency_config: Option<ConcurrencyTriggerConfig>,
}

/// Scaling trigger response
#[derive(Debug, Serialize, Deserialize)]
pub struct ScalingTriggerResponse {
    pub trigger_id: String,
    pub name: String,
    pub trigger_type: String,
    pub pool_id: String,
    pub policy_id: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub config: HashMap<String, serde_json::Value>,
}

/// Create scaling trigger request
#[derive(Debug, Deserialize)]
pub struct CreateScalingTriggerRequest {
    pub name: String,
    pub trigger_type: String, // threshold, time_based, metric_change, concurrency
    pub pool_id: String,
    pub policy_id: String,
    pub is_enabled: bool,
    pub threshold_config: Option<ThresholdTriggerConfig>,
    pub time_based_config: Option<TimeBasedTriggerConfig>,
    pub metric_change_config: Option<MetricChangeTriggerConfig>,
    pub concurrency_config: Option<ConcurrencyTriggerConfig>,
}

/// Update scaling trigger request
#[derive(Debug, Deserialize)]
pub struct UpdateScalingTriggerRequest {
    pub name: Option<String>,
    pub is_enabled: Option<bool>,
    pub threshold_config: Option<ThresholdTriggerConfig>,
    pub time_based_config: Option<TimeBasedTriggerConfig>,
    pub metric_change_config: Option<MetricChangeTriggerConfig>,
    pub concurrency_config: Option<ConcurrencyTriggerConfig>,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct ScalingTriggerMessageResponse {
    pub message: String,
}

/// Scaling trigger statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ScalingTriggerStats {
    pub trigger_id: String,
    pub total_triggers: u64,
    pub last_trigger_time: Option<DateTime<Utc>>,
    pub average_interval: Option<Duration>,
    pub max_consecutive_triggers: u32,
    pub successful_evaluations: u64,
    pub failed_evaluations: u64,
}

/// Trigger evaluation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvaluation {
    pub evaluation_id: String,
    pub trigger_id: String,
    pub evaluation_time: DateTime<Utc>,
    pub metric_value: f64,
    pub threshold_value: f64,
    pub triggered: bool,
    pub evaluation_duration_ms: u64,
}

/// Scaling Triggers Service
#[derive(Debug, Clone)]
pub struct ScalingTriggersService {
    /// Scaling triggers
    triggers: Arc<RwLock<HashMap<String, ScalingTrigger>>>,
    /// Trigger evaluation history
    evaluation_history: Arc<RwLock<HashMap<String, Vec<TriggerEvaluation>>>>,
}

impl ScalingTriggersService {
    /// Create new Scaling Triggers Service
    pub fn new() -> Self {
        info!("Initializing Scaling Triggers Service");
        Self {
            triggers: Arc::new(RwLock::new(HashMap::new())),
            evaluation_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create scaling trigger
    pub async fn create_trigger(
        &self,
        request: CreateScalingTriggerRequest,
    ) -> Result<ScalingTrigger, String> {
        let trigger_type = match request.trigger_type.to_lowercase().as_str() {
            "threshold" => ScalingTriggerType::Threshold,
            "time_based" => ScalingTriggerType::TimeBased,
            "metric_change" => ScalingTriggerType::MetricChange,
            "concurrency" => ScalingTriggerType::Concurrency,
            _ => return Err("Invalid trigger type".to_string()),
        };

        // Validate configuration based on trigger type
        match &trigger_type {
            ScalingTriggerType::Threshold => {
                if request.threshold_config.is_none() {
                    return Err("Threshold config required".to_string());
                }
            }
            ScalingTriggerType::TimeBased => {
                if request.time_based_config.is_none() {
                    return Err("Time-based config required".to_string());
                }
            }
            ScalingTriggerType::MetricChange => {
                if request.metric_change_config.is_none() {
                    return Err("Metric change config required".to_string());
                }
            }
            ScalingTriggerType::Concurrency => {
                if request.concurrency_config.is_none() {
                    return Err("Concurrency config required".to_string());
                }
            }
        }

        let trigger = ScalingTrigger {
            trigger_id: uuid::Uuid::new_v4().to_string(),
            name: request.name,
            trigger_type: trigger_type.clone(),
            pool_id: request.pool_id,
            policy_id: request.policy_id,
            is_enabled: request.is_enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered: None,
            threshold_config: request.threshold_config,
            time_based_config: request.time_based_config,
            metric_change_config: request.metric_change_config,
            concurrency_config: request.concurrency_config,
        };

        let mut triggers = self.triggers.write().await;
        triggers.insert(trigger.trigger_id.clone(), trigger.clone());

        info!(
            "Created scaling trigger: {} ({:?})",
            trigger.trigger_id, trigger_type
        );

        Ok(trigger)
    }

    /// Get scaling trigger
    pub async fn get_trigger(&self, trigger_id: &str) -> Result<ScalingTrigger, String> {
        let triggers = self.triggers.read().await;
        triggers
            .get(trigger_id)
            .cloned()
            .ok_or_else(|| "Trigger not found".to_string())
    }

    /// List scaling triggers
    pub async fn list_triggers(&self) -> Vec<ScalingTrigger> {
        let triggers = self.triggers.read().await;
        triggers.values().cloned().collect()
    }

    /// List triggers by pool
    pub async fn list_triggers_by_pool(&self, pool_id: &str) -> Vec<ScalingTrigger> {
        let triggers = self.triggers.read().await;
        triggers
            .values()
            .filter(|t| t.pool_id == pool_id)
            .cloned()
            .collect()
    }

    /// List triggers by policy
    pub async fn list_triggers_by_policy(&self, policy_id: &str) -> Vec<ScalingTrigger> {
        let triggers = self.triggers.read().await;
        triggers
            .values()
            .filter(|t| t.policy_id == policy_id)
            .cloned()
            .collect()
    }

    /// Update scaling trigger
    pub async fn update_trigger(
        &self,
        trigger_id: &str,
        request: UpdateScalingTriggerRequest,
    ) -> Result<ScalingTrigger, String> {
        let mut triggers = self.triggers.write().await;
        let trigger = triggers
            .get_mut(trigger_id)
            .ok_or_else(|| "Trigger not found".to_string())?;

        if let Some(name) = request.name {
            trigger.name = name;
        }
        if let Some(is_enabled) = request.is_enabled {
            trigger.is_enabled = is_enabled;
        }
        if let Some(threshold_config) = request.threshold_config {
            trigger.threshold_config = Some(threshold_config);
        }
        if let Some(time_based_config) = request.time_based_config {
            trigger.time_based_config = Some(time_based_config);
        }
        if let Some(metric_change_config) = request.metric_change_config {
            trigger.metric_change_config = Some(metric_change_config);
        }
        if let Some(concurrency_config) = request.concurrency_config {
            trigger.concurrency_config = Some(concurrency_config);
        }

        trigger.updated_at = Utc::now();

        info!("Updated scaling trigger: {}", trigger_id);

        Ok(trigger.clone())
    }

    /// Delete scaling trigger
    pub async fn delete_trigger(&self, trigger_id: &str) -> Result<(), String> {
        let mut triggers = self.triggers.write().await;

        if !triggers.contains_key(trigger_id) {
            return Err("Trigger not found".to_string());
        }

        triggers.remove(trigger_id);

        // Clean up evaluation history
        let mut history = self.evaluation_history.write().await;
        history.remove(trigger_id);

        info!("Deleted scaling trigger: {}", trigger_id);

        Ok(())
    }

    /// Enable/disable trigger
    pub async fn set_trigger_enabled(&self, trigger_id: &str, enabled: bool) -> Result<(), String> {
        let mut triggers = self.triggers.write().await;
        let trigger = triggers
            .get_mut(trigger_id)
            .ok_or_else(|| "Trigger not found".to_string())?;

        trigger.is_enabled = enabled;
        trigger.updated_at = Utc::now();

        info!(
            "Scaling trigger {} {}",
            trigger_id,
            if enabled { "enabled" } else { "disabled" }
        );

        Ok(())
    }

    /// Get trigger statistics
    pub async fn get_trigger_stats(&self, trigger_id: &str) -> Result<ScalingTriggerStats, String> {
        let history = self.evaluation_history.read().await;
        let evaluations = history.get(trigger_id).cloned().unwrap_or_else(Vec::new);

        let total_triggers = evaluations.len() as u64;
        let successful_evaluations = evaluations.iter().filter(|e| e.triggered).count() as u64;
        let failed_evaluations = evaluations.iter().filter(|e| !e.triggered).count() as u64;

        let last_trigger_time = evaluations
            .iter()
            .filter(|e| e.triggered)
            .map(|e| e.evaluation_time)
            .max();

        // Calculate average interval between triggers (simplified)
        let sorted_evaluations: Vec<_> = evaluations.into_iter().filter(|e| e.triggered).collect();
        let average_interval = if sorted_evaluations.len() > 1 {
            let mut total_duration = Duration::from_secs(0);
            for i in 1..sorted_evaluations.len() {
                let prev = &sorted_evaluations[i - 1];
                let curr = &sorted_evaluations[i];
                if let Ok(duration) = curr
                    .evaluation_time
                    .signed_duration_since(prev.evaluation_time)
                    .to_std()
                {
                    total_duration += duration;
                }
            }
            Some(total_duration / (sorted_evaluations.len() - 1) as u32)
        } else {
            None
        };

        Ok(ScalingTriggerStats {
            trigger_id: trigger_id.to_string(),
            total_triggers,
            last_trigger_time,
            average_interval,
            max_consecutive_triggers: 1, // Simplified
            successful_evaluations,
            failed_evaluations,
        })
    }

    /// Evaluate trigger (mock implementation)
    pub async fn evaluate_trigger(&self, trigger_id: &str, metric_value: f64) -> bool {
        let triggers = self.triggers.read().await;
        if let Some(trigger) = triggers.get(trigger_id) {
            if !trigger.is_enabled {
                return false;
            }

            // Mock evaluation logic
            let triggered = match &trigger.trigger_type {
                ScalingTriggerType::Threshold => {
                    if let Some(config) = &trigger.threshold_config {
                        match config.comparison_operator.as_str() {
                            "greater_than" => metric_value > config.threshold_value,
                            "less_than" => metric_value < config.threshold_value,
                            "equal" => (metric_value - config.threshold_value).abs() < 0.01,
                            _ => false,
                        }
                    } else {
                        false
                    }
                }
                ScalingTriggerType::TimeBased => false, // Time-based triggers evaluated on schedule
                ScalingTriggerType::MetricChange => false, // Would need historical data
                ScalingTriggerType::Concurrency => {
                    if let Some(config) = &trigger.concurrency_config {
                        metric_value > config.threshold_value
                    } else {
                        false
                    }
                }
            };

            if triggered {
                info!(
                    "Trigger {} activated with metric value {}",
                    trigger_id, metric_value
                );
            }

            triggered
        } else {
            false
        }
    }

    /// Record trigger evaluation
    pub async fn record_evaluation(
        &self,
        trigger_id: &str,
        metric_value: f64,
        threshold_value: f64,
        triggered: bool,
    ) {
        let evaluation = TriggerEvaluation {
            evaluation_id: uuid::Uuid::new_v4().to_string(),
            trigger_id: trigger_id.to_string(),
            evaluation_time: Utc::now(),
            metric_value,
            threshold_value,
            triggered,
            evaluation_duration_ms: 1, // Mock evaluation time
        };

        let mut history = self.evaluation_history.write().await;
        history
            .entry(trigger_id.to_string())
            .or_insert_with(Vec::new)
            .push(evaluation);

        if triggered {
            info!(
                "Recorded trigger activation for {}: metric={}, threshold={}",
                trigger_id, metric_value, threshold_value
            );
        }
    }
}

/// Application state for Scaling Triggers
#[derive(Clone)]
pub struct ScalingTriggersAppState {
    pub service: Arc<ScalingTriggersService>,
}

/// Create router for Scaling Triggers API
pub fn scaling_triggers_routes() -> Router<ScalingTriggersAppState> {
    Router::new()
        .route("/scaling-triggers", post(create_scaling_trigger_handler))
        .route("/scaling-triggers", get(list_scaling_triggers_handler))
        .route(
            "/scaling-triggers/:trigger_id",
            get(get_scaling_trigger_handler),
        )
        .route(
            "/scaling-triggers/:trigger_id",
            put(update_scaling_trigger_handler),
        )
        .route(
            "/scaling-triggers/:trigger_id",
            delete(delete_scaling_trigger_handler),
        )
        .route(
            "/scaling-triggers/:trigger_id/enable",
            post(enable_scaling_trigger_handler),
        )
        .route(
            "/scaling-triggers/:trigger_id/disable",
            post(disable_scaling_trigger_handler),
        )
        .route(
            "/scaling-triggers/pool/:pool_id",
            get(list_scaling_triggers_by_pool_handler),
        )
        .route(
            "/scaling-triggers/policy/:policy_id",
            get(list_scaling_triggers_by_policy_handler),
        )
        .route(
            "/scaling-triggers/:trigger_id/stats",
            get(get_scaling_trigger_stats_handler),
        )
        .route(
            "/scaling-triggers/:trigger_id/evaluate",
            post(evaluate_scaling_trigger_handler),
        )
}

/// Create scaling trigger handler
async fn create_scaling_trigger_handler(
    State(state): State<ScalingTriggersAppState>,
    Json(payload): Json<CreateScalingTriggerRequest>,
) -> Result<Json<ScalingTriggerMessageResponse>, StatusCode> {
    match state.service.create_trigger(payload).await {
        Ok(trigger) => Ok(Json(ScalingTriggerMessageResponse {
            message: format!("Scaling trigger created: {}", trigger.trigger_id),
        })),
        Err(e) => {
            error!("Failed to create scaling trigger: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get scaling trigger handler
async fn get_scaling_trigger_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(trigger_id): axum::extract::Path<String>,
) -> Result<Json<ScalingTriggerResponse>, StatusCode> {
    match state.service.get_trigger(&trigger_id).await {
        Ok(trigger) => {
            let mut config = HashMap::new();

            if let Some(threshold_config) = &trigger.threshold_config {
                config.insert(
                    "threshold".to_string(),
                    serde_json::to_value(threshold_config).unwrap(),
                );
            }
            if let Some(time_based_config) = &trigger.time_based_config {
                config.insert(
                    "time_based".to_string(),
                    serde_json::to_value(time_based_config).unwrap(),
                );
            }
            if let Some(metric_change_config) = &trigger.metric_change_config {
                config.insert(
                    "metric_change".to_string(),
                    serde_json::to_value(metric_change_config).unwrap(),
                );
            }
            if let Some(concurrency_config) = &trigger.concurrency_config {
                config.insert(
                    "concurrency".to_string(),
                    serde_json::to_value(concurrency_config).unwrap(),
                );
            }

            Ok(Json(ScalingTriggerResponse {
                trigger_id: trigger.trigger_id,
                name: trigger.name,
                trigger_type: format!("{:?}", trigger.trigger_type),
                pool_id: trigger.pool_id,
                policy_id: trigger.policy_id,
                is_enabled: trigger.is_enabled,
                created_at: trigger.created_at,
                updated_at: trigger.updated_at,
                last_triggered: trigger.last_triggered,
                config,
            }))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// List scaling triggers handler
async fn list_scaling_triggers_handler(
    State(state): State<ScalingTriggersAppState>,
) -> Result<Json<Vec<ScalingTriggerResponse>>, StatusCode> {
    let triggers = state.service.list_triggers().await;

    let responses: Vec<ScalingTriggerResponse> = triggers
        .into_iter()
        .map(|trigger| {
            let mut config = HashMap::new();

            if let Some(threshold_config) = &trigger.threshold_config {
                config.insert(
                    "threshold".to_string(),
                    serde_json::to_value(threshold_config).unwrap(),
                );
            }
            if let Some(time_based_config) = &trigger.time_based_config {
                config.insert(
                    "time_based".to_string(),
                    serde_json::to_value(time_based_config).unwrap(),
                );
            }
            if let Some(metric_change_config) = &trigger.metric_change_config {
                config.insert(
                    "metric_change".to_string(),
                    serde_json::to_value(metric_change_config).unwrap(),
                );
            }
            if let Some(concurrency_config) = &trigger.concurrency_config {
                config.insert(
                    "concurrency".to_string(),
                    serde_json::to_value(concurrency_config).unwrap(),
                );
            }

            ScalingTriggerResponse {
                trigger_id: trigger.trigger_id,
                name: trigger.name,
                trigger_type: format!("{:?}", trigger.trigger_type),
                pool_id: trigger.pool_id,
                policy_id: trigger.policy_id,
                is_enabled: trigger.is_enabled,
                created_at: trigger.created_at,
                updated_at: trigger.updated_at,
                last_triggered: trigger.last_triggered,
                config,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// List scaling triggers by pool handler
async fn list_scaling_triggers_by_pool_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<Vec<ScalingTriggerResponse>>, StatusCode> {
    let triggers = state.service.list_triggers_by_pool(&pool_id).await;

    let responses: Vec<ScalingTriggerResponse> = triggers
        .into_iter()
        .map(|trigger| {
            let mut config = HashMap::new();

            if let Some(threshold_config) = &trigger.threshold_config {
                config.insert(
                    "threshold".to_string(),
                    serde_json::to_value(threshold_config).unwrap(),
                );
            }
            if let Some(time_based_config) = &trigger.time_based_config {
                config.insert(
                    "time_based".to_string(),
                    serde_json::to_value(time_based_config).unwrap(),
                );
            }
            if let Some(metric_change_config) = &trigger.metric_change_config {
                config.insert(
                    "metric_change".to_string(),
                    serde_json::to_value(metric_change_config).unwrap(),
                );
            }
            if let Some(concurrency_config) = &trigger.concurrency_config {
                config.insert(
                    "concurrency".to_string(),
                    serde_json::to_value(concurrency_config).unwrap(),
                );
            }

            ScalingTriggerResponse {
                trigger_id: trigger.trigger_id,
                name: trigger.name,
                trigger_type: format!("{:?}", trigger.trigger_type),
                pool_id: trigger.pool_id,
                policy_id: trigger.policy_id,
                is_enabled: trigger.is_enabled,
                created_at: trigger.created_at,
                updated_at: trigger.updated_at,
                last_triggered: trigger.last_triggered,
                config,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// List scaling triggers by policy handler
async fn list_scaling_triggers_by_policy_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
) -> Result<Json<Vec<ScalingTriggerResponse>>, StatusCode> {
    let triggers = state.service.list_triggers_by_policy(&policy_id).await;

    let responses: Vec<ScalingTriggerResponse> = triggers
        .into_iter()
        .map(|trigger| {
            let mut config = HashMap::new();

            if let Some(threshold_config) = &trigger.threshold_config {
                config.insert(
                    "threshold".to_string(),
                    serde_json::to_value(threshold_config).unwrap(),
                );
            }
            if let Some(time_based_config) = &trigger.time_based_config {
                config.insert(
                    "time_based".to_string(),
                    serde_json::to_value(time_based_config).unwrap(),
                );
            }
            if let Some(metric_change_config) = &trigger.metric_change_config {
                config.insert(
                    "metric_change".to_string(),
                    serde_json::to_value(metric_change_config).unwrap(),
                );
            }
            if let Some(concurrency_config) = &trigger.concurrency_config {
                config.insert(
                    "concurrency".to_string(),
                    serde_json::to_value(concurrency_config).unwrap(),
                );
            }

            ScalingTriggerResponse {
                trigger_id: trigger.trigger_id,
                name: trigger.name,
                trigger_type: format!("{:?}", trigger.trigger_type),
                pool_id: trigger.pool_id,
                policy_id: trigger.policy_id,
                is_enabled: trigger.is_enabled,
                created_at: trigger.created_at,
                updated_at: trigger.updated_at,
                last_triggered: trigger.last_triggered,
                config,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// Update scaling trigger handler
async fn update_scaling_trigger_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(trigger_id): axum::extract::Path<String>,
    Json(payload): Json<UpdateScalingTriggerRequest>,
) -> Result<Json<ScalingTriggerMessageResponse>, StatusCode> {
    match state.service.update_trigger(&trigger_id, payload).await {
        Ok(_) => Ok(Json(ScalingTriggerMessageResponse {
            message: format!("Scaling trigger {} updated successfully", trigger_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Delete scaling trigger handler
async fn delete_scaling_trigger_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(trigger_id): axum::extract::Path<String>,
) -> Result<Json<ScalingTriggerMessageResponse>, StatusCode> {
    match state.service.delete_trigger(&trigger_id).await {
        Ok(_) => Ok(Json(ScalingTriggerMessageResponse {
            message: format!("Scaling trigger {} deleted successfully", trigger_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Enable scaling trigger handler
async fn enable_scaling_trigger_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(trigger_id): axum::extract::Path<String>,
) -> Result<Json<ScalingTriggerMessageResponse>, StatusCode> {
    match state.service.set_trigger_enabled(&trigger_id, true).await {
        Ok(_) => Ok(Json(ScalingTriggerMessageResponse {
            message: format!("Scaling trigger {} enabled successfully", trigger_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Disable scaling trigger handler
async fn disable_scaling_trigger_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(trigger_id): axum::extract::Path<String>,
) -> Result<Json<ScalingTriggerMessageResponse>, StatusCode> {
    match state.service.set_trigger_enabled(&trigger_id, false).await {
        Ok(_) => Ok(Json(ScalingTriggerMessageResponse {
            message: format!("Scaling trigger {} disabled successfully", trigger_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get scaling trigger statistics handler
async fn get_scaling_trigger_stats_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(trigger_id): axum::extract::Path<String>,
) -> Result<Json<ScalingTriggerStats>, StatusCode> {
    match state.service.get_trigger_stats(&trigger_id).await {
        Ok(stats) => Ok(Json(stats)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Evaluate scaling trigger handler
async fn evaluate_scaling_trigger_handler(
    State(state): State<ScalingTriggersAppState>,
    axum::extract::Path(trigger_id): axum::extract::Path<String>,
) -> Result<Json<ScalingTriggerMessageResponse>, StatusCode> {
    // Mock metric value - in real implementation, this would come from monitoring
    let metric_value = 75.0;
    let triggered = state
        .service
        .evaluate_trigger(&trigger_id, metric_value)
        .await;

    state
        .service
        .record_evaluation(&trigger_id, metric_value, 70.0, triggered)
        .await;

    Ok(Json(ScalingTriggerMessageResponse {
        message: format!(
            "Trigger evaluation: {}",
            if triggered {
                "triggered"
            } else {
                "not triggered"
            }
        ),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_threshold_trigger() {
        let service = ScalingTriggersService::new();

        let request = CreateScalingTriggerRequest {
            name: "CPU Threshold".to_string(),
            trigger_type: "threshold".to_string(),
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            is_enabled: true,
            threshold_config: Some(ThresholdTriggerConfig {
                metric_type: MetricType::CpuUtilization,
                comparison_operator: "greater_than".to_string(),
                threshold_value: 70.0,
                evaluation_period: Duration::from_secs(60),
                consecutive_periods: 2,
            }),
            time_based_config: None,
            metric_change_config: None,
            concurrency_config: None,
        };

        let result = service.create_trigger(request).await;
        assert!(result.is_ok());

        let trigger = result.unwrap();
        assert_eq!(trigger.name, "CPU Threshold");
        assert_eq!(trigger.pool_id, "pool-1");
    }

    #[tokio::test]
    async fn test_list_triggers() {
        let service = ScalingTriggersService::new();

        let request = CreateScalingTriggerRequest {
            name: "Test Trigger".to_string(),
            trigger_type: "threshold".to_string(),
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            is_enabled: true,
            threshold_config: Some(ThresholdTriggerConfig {
                metric_type: MetricType::CpuUtilization,
                comparison_operator: "greater_than".to_string(),
                threshold_value: 70.0,
                evaluation_period: Duration::from_secs(60),
                consecutive_periods: 2,
            }),
            time_based_config: None,
            metric_change_config: None,
            concurrency_config: None,
        };

        service.create_trigger(request).await.unwrap();

        let triggers = service.list_triggers().await;
        assert_eq!(triggers.len(), 1);
    }

    #[tokio::test]
    async fn test_update_trigger() {
        let service = ScalingTriggersService::new();

        let request = CreateScalingTriggerRequest {
            name: "Test Trigger".to_string(),
            trigger_type: "threshold".to_string(),
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            is_enabled: true,
            threshold_config: Some(ThresholdTriggerConfig {
                metric_type: MetricType::CpuUtilization,
                comparison_operator: "greater_than".to_string(),
                threshold_value: 70.0,
                evaluation_period: Duration::from_secs(60),
                consecutive_periods: 2,
            }),
            time_based_config: None,
            metric_change_config: None,
            concurrency_config: None,
        };

        let trigger = service.create_trigger(request).await.unwrap();

        let update_request = UpdateScalingTriggerRequest {
            name: Some("Updated Trigger".to_string()),
            is_enabled: Some(false),
            threshold_config: None,
            time_based_config: None,
            metric_change_config: None,
            concurrency_config: None,
        };

        let result = service
            .update_trigger(&trigger.trigger_id, update_request)
            .await;
        assert!(result.is_ok());

        let updated = result.unwrap();
        assert_eq!(updated.name, "Updated Trigger");
        assert_eq!(updated.is_enabled, false);
    }

    #[tokio::test]
    async fn test_delete_trigger() {
        let service = ScalingTriggersService::new();

        let request = CreateScalingTriggerRequest {
            name: "Test Trigger".to_string(),
            trigger_type: "threshold".to_string(),
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            is_enabled: true,
            threshold_config: Some(ThresholdTriggerConfig {
                metric_type: MetricType::CpuUtilization,
                comparison_operator: "greater_than".to_string(),
                threshold_value: 70.0,
                evaluation_period: Duration::from_secs(60),
                consecutive_periods: 2,
            }),
            time_based_config: None,
            metric_change_config: None,
            concurrency_config: None,
        };

        let trigger = service.create_trigger(request).await.unwrap();

        let result = service.delete_trigger(&trigger.trigger_id).await;
        assert!(result.is_ok());

        let triggers = service.list_triggers().await;
        assert_eq!(triggers.len(), 0);
    }

    #[tokio::test]
    async fn test_evaluate_trigger() {
        let service = ScalingTriggersService::new();

        let request = CreateScalingTriggerRequest {
            name: "Test Trigger".to_string(),
            trigger_type: "threshold".to_string(),
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            is_enabled: true,
            threshold_config: Some(ThresholdTriggerConfig {
                metric_type: MetricType::CpuUtilization,
                comparison_operator: "greater_than".to_string(),
                threshold_value: 70.0,
                evaluation_period: Duration::from_secs(60),
                consecutive_periods: 2,
            }),
            time_based_config: None,
            metric_change_config: None,
            concurrency_config: None,
        };

        let trigger = service.create_trigger(request).await.unwrap();

        // Test threshold exceeded
        let result = service.evaluate_trigger(&trigger.trigger_id, 75.0).await;
        assert!(result);

        // Test threshold not exceeded
        let result = service.evaluate_trigger(&trigger.trigger_id, 65.0).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_invalid_trigger_type() {
        let service = ScalingTriggersService::new();

        let request = CreateScalingTriggerRequest {
            name: "Test Trigger".to_string(),
            trigger_type: "invalid_type".to_string(),
            pool_id: "pool-1".to_string(),
            policy_id: "policy-1".to_string(),
            is_enabled: true,
            threshold_config: None,
            time_based_config: None,
            metric_change_config: None,
            concurrency_config: None,
        };

        let result = service.create_trigger(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid trigger type"));
    }
}
