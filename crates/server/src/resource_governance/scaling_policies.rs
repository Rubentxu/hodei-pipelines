//! Scaling Policies API
//!
//! This module provides endpoints for managing scaling policies for resource pools,
//! including target tracking, step scaling, and predictive scaling policies.

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

/// Scaling policy types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScalingPolicyType {
    TargetTracking,
    StepScaling,
    PredictiveScaling,
}

/// Target tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetTrackingConfig {
    pub metric_name: String,
    pub target_value: f64,
    pub metric_type: String, // cpu_utilization, memory_utilization, queue_depth, etc.
    pub scale_in_cooldown: Duration,
    pub scale_out_cooldown: Duration,
}

/// Step scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepScalingConfig {
    pub metric_name: String,
    pub metric_type: String,
    pub adjustment_type: String, // percent_change, exact_count, change_in_capacity
    pub step_adjustments: Vec<StepAdjustment>,
    pub min_step_adjustment: Option<i32>,
    pub max_step_adjustment: Option<i32>,
    pub cooldown: Duration,
}

/// Step adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepAdjustment {
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
    pub adjustment: i32,
}

/// Predictive scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveScalingConfig {
    pub metric_name: String,
    pub metric_type: String,
    pub schedule_type: String, // cron, recurring, forecasted
    pub schedule_expression: String,
    pub prediction_window: Duration,
    pub scaling_lookback: Duration,
    pub min_capacity: i32,
    pub max_capacity: i32,
    pub enabled: bool,
}

/// Scaling policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPolicy {
    pub policy_id: String,
    pub name: String,
    pub policy_type: ScalingPolicyType,
    pub pool_id: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Configuration based on policy type
    pub target_tracking_config: Option<TargetTrackingConfig>,
    pub step_scaling_config: Option<StepScalingConfig>,
    pub predictive_scaling_config: Option<PredictiveScalingConfig>,
}

/// Scaling policy response
#[derive(Debug, Serialize, Deserialize)]
pub struct ScalingPolicyResponse {
    pub policy_id: String,
    pub name: String,
    pub policy_type: String,
    pub pool_id: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: HashMap<String, serde_json::Value>,
}

/// Create scaling policy request
#[derive(Debug, Deserialize)]
pub struct CreateScalingPolicyRequest {
    pub name: String,
    pub policy_type: String, // target_tracking, step_scaling, predictive_scaling
    pub pool_id: String,
    pub is_enabled: bool,
    pub target_tracking_config: Option<TargetTrackingConfig>,
    pub step_scaling_config: Option<StepScalingConfig>,
    pub predictive_scaling_config: Option<PredictiveScalingConfig>,
}

/// Update scaling policy request
#[derive(Debug, Deserialize)]
pub struct UpdateScalingPolicyRequest {
    pub name: Option<String>,
    pub is_enabled: Option<bool>,
    pub target_tracking_config: Option<TargetTrackingConfig>,
    pub step_scaling_config: Option<StepScalingConfig>,
    pub predictive_scaling_config: Option<PredictiveScalingConfig>,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct ScalingPolicyMessageResponse {
    pub message: String,
}

/// Scaling policy statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ScalingPolicyStats {
    pub policy_id: String,
    pub executions_count: u64,
    pub scale_out_events: u64,
    pub scale_in_events: u64,
    pub last_execution: Option<DateTime<Utc>>,
    pub last_scale_out: Option<DateTime<Utc>>,
    pub last_scale_in: Option<DateTime<Utc>>,
    pub avg_execution_time_ms: u64,
}

/// Scaling Policies Service
#[derive(Debug, Clone)]
pub struct ScalingPoliciesService {
    /// Scaling policies
    policies: Arc<RwLock<HashMap<String, ScalingPolicy>>>,
    /// Policy execution history
    execution_history: Arc<RwLock<HashMap<String, Vec<PolicyExecution>>>>,
}

/// Policy execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyExecution {
    pub execution_id: String,
    pub policy_id: String,
    pub execution_time: DateTime<Utc>,
    pub scale_direction: String, // scale_out, scale_in
    pub previous_capacity: i32,
    pub new_capacity: i32,
    pub trigger_metric: f64,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

impl ScalingPoliciesService {
    /// Create new Scaling Policies Service
    pub fn new() -> Self {
        info!("Initializing Scaling Policies Service");
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create scaling policy
    pub async fn create_policy(
        &self,
        request: CreateScalingPolicyRequest,
    ) -> Result<ScalingPolicy, String> {
        let policy_type = match request.policy_type.to_lowercase().as_str() {
            "target_tracking" => ScalingPolicyType::TargetTracking,
            "step_scaling" => ScalingPolicyType::StepScaling,
            "predictive_scaling" => ScalingPolicyType::PredictiveScaling,
            _ => return Err("Invalid policy type".to_string()),
        };

        // Validate configuration based on policy type
        match &policy_type {
            ScalingPolicyType::TargetTracking => {
                if request.target_tracking_config.is_none() {
                    return Err("Target tracking config required".to_string());
                }
            }
            ScalingPolicyType::StepScaling => {
                if request.step_scaling_config.is_none() {
                    return Err("Step scaling config required".to_string());
                }
            }
            ScalingPolicyType::PredictiveScaling => {
                if request.predictive_scaling_config.is_none() {
                    return Err("Predictive scaling config required".to_string());
                }
            }
        }

        let policy = ScalingPolicy {
            policy_id: uuid::Uuid::new_v4().to_string(),
            name: request.name,
            policy_type: policy_type.clone(),
            pool_id: request.pool_id,
            is_enabled: request.is_enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            target_tracking_config: request.target_tracking_config,
            step_scaling_config: request.step_scaling_config,
            predictive_scaling_config: request.predictive_scaling_config,
        };

        let mut policies = self.policies.write().await;
        policies.insert(policy.policy_id.clone(), policy.clone());

        info!(
            "Created scaling policy: {} ({:?})",
            policy.policy_id, policy_type
        );

        Ok(policy)
    }

    /// Get scaling policy
    pub async fn get_policy(&self, policy_id: &str) -> Result<ScalingPolicy, String> {
        let policies = self.policies.read().await;
        policies
            .get(policy_id)
            .cloned()
            .ok_or_else(|| "Policy not found".to_string())
    }

    /// List scaling policies
    pub async fn list_policies(&self) -> Vec<ScalingPolicy> {
        let policies = self.policies.read().await;
        policies.values().cloned().collect()
    }

    /// List policies by pool
    pub async fn list_policies_by_pool(&self, pool_id: &str) -> Vec<ScalingPolicy> {
        let policies = self.policies.read().await;
        policies
            .values()
            .filter(|p| p.pool_id == pool_id)
            .cloned()
            .collect()
    }

    /// Update scaling policy
    pub async fn update_policy(
        &self,
        policy_id: &str,
        request: UpdateScalingPolicyRequest,
    ) -> Result<ScalingPolicy, String> {
        let mut policies = self.policies.write().await;
        let policy = policies
            .get_mut(policy_id)
            .ok_or_else(|| "Policy not found".to_string())?;

        if let Some(name) = request.name {
            policy.name = name;
        }
        if let Some(is_enabled) = request.is_enabled {
            policy.is_enabled = is_enabled;
        }
        if let Some(target_tracking_config) = request.target_tracking_config {
            policy.target_tracking_config = Some(target_tracking_config);
        }
        if let Some(step_scaling_config) = request.step_scaling_config {
            policy.step_scaling_config = Some(step_scaling_config);
        }
        if let Some(predictive_scaling_config) = request.predictive_scaling_config {
            policy.predictive_scaling_config = Some(predictive_scaling_config);
        }

        policy.updated_at = Utc::now();

        info!("Updated scaling policy: {}", policy_id);

        Ok(policy.clone())
    }

    /// Delete scaling policy
    pub async fn delete_policy(&self, policy_id: &str) -> Result<(), String> {
        let mut policies = self.policies.write().await;

        if !policies.contains_key(policy_id) {
            return Err("Policy not found".to_string());
        }

        policies.remove(policy_id);

        // Clean up execution history
        let mut history = self.execution_history.write().await;
        history.remove(policy_id);

        info!("Deleted scaling policy: {}", policy_id);

        Ok(())
    }

    /// Enable/disable policy
    pub async fn set_policy_enabled(&self, policy_id: &str, enabled: bool) -> Result<(), String> {
        let mut policies = self.policies.write().await;
        let policy = policies
            .get_mut(policy_id)
            .ok_or_else(|| "Policy not found".to_string())?;

        policy.is_enabled = enabled;
        policy.updated_at = Utc::now();

        info!(
            "Scaling policy {} {}",
            policy_id,
            if enabled { "enabled" } else { "disabled" }
        );

        Ok(())
    }

    /// Get policy statistics
    pub async fn get_policy_stats(&self, policy_id: &str) -> Result<ScalingPolicyStats, String> {
        let history = self.execution_history.read().await;
        let executions = history.get(policy_id).cloned().unwrap_or_else(Vec::new);

        let executions_count = executions.len() as u64;
        let scale_out_events = executions
            .iter()
            .filter(|e| e.scale_direction == "scale_out")
            .count() as u64;
        let scale_in_events = executions
            .iter()
            .filter(|e| e.scale_direction == "scale_in")
            .count() as u64;

        let last_execution = executions.iter().map(|e| e.execution_time).max();
        let last_scale_out = executions
            .iter()
            .filter(|e| e.scale_direction == "scale_out")
            .map(|e| e.execution_time)
            .max();
        let last_scale_in = executions
            .iter()
            .filter(|e| e.scale_direction == "scale_in")
            .map(|e| e.execution_time)
            .max();

        let avg_execution_time_ms = if executions.is_empty() {
            0
        } else {
            executions.iter().map(|e| e.execution_time_ms).sum::<u64>() / executions_count
        };

        Ok(ScalingPolicyStats {
            policy_id: policy_id.to_string(),
            executions_count,
            scale_out_events,
            scale_in_events,
            last_execution,
            last_scale_out,
            last_scale_in,
            avg_execution_time_ms,
        })
    }

    /// Record policy execution
    pub async fn record_execution(
        &self,
        policy_id: &str,
        scale_direction: &str,
        previous_capacity: i32,
        new_capacity: i32,
        trigger_metric: f64,
    ) {
        let execution = PolicyExecution {
            execution_id: uuid::Uuid::new_v4().to_string(),
            policy_id: policy_id.to_string(),
            execution_time: Utc::now(),
            scale_direction: scale_direction.to_string(),
            previous_capacity,
            new_capacity,
            trigger_metric,
            execution_time_ms: 10, // Mock execution time
            success: true,
            error_message: None,
        };

        let mut history = self.execution_history.write().await;
        history
            .entry(policy_id.to_string())
            .or_insert_with(Vec::new)
            .push(execution);

        info!(
            "Recorded execution for policy {}: {} ({} → {})",
            policy_id, scale_direction, previous_capacity, new_capacity
        );
    }
}

/// Application state for Scaling Policies
#[derive(Clone)]
pub struct ScalingPoliciesAppState {
    pub service: Arc<ScalingPoliciesService>,
}

/// Create router for Scaling Policies API
pub fn scaling_policies_routes() -> Router<ScalingPoliciesAppState> {
    Router::new()
        .route("/scaling-policies", post(create_scaling_policy_handler))
        .route("/scaling-policies", get(list_scaling_policies_handler))
        .route(
            "/scaling-policies/:policy_id",
            get(get_scaling_policy_handler),
        )
        .route(
            "/scaling-policies/:policy_id",
            put(update_scaling_policy_handler),
        )
        .route(
            "/scaling-policies/:policy_id",
            delete(delete_scaling_policy_handler),
        )
        .route(
            "/scaling-policies/:policy_id/enable",
            post(enable_scaling_policy_handler),
        )
        .route(
            "/scaling-policies/:policy_id/disable",
            post(disable_scaling_policy_handler),
        )
        .route(
            "/scaling-policies/pool/:pool_id",
            get(list_scaling_policies_by_pool_handler),
        )
        .route(
            "/scaling-policies/:policy_id/stats",
            get(get_scaling_policy_stats_handler),
        )
}

/// Create scaling policy handler
async fn create_scaling_policy_handler(
    State(state): State<ScalingPoliciesAppState>,
    Json(payload): Json<CreateScalingPolicyRequest>,
) -> Result<Json<ScalingPolicyMessageResponse>, StatusCode> {
    match state.service.create_policy(payload).await {
        Ok(policy) => Ok(Json(ScalingPolicyMessageResponse {
            message: format!("Scaling policy created: {}", policy.policy_id),
        })),
        Err(e) => {
            error!("Failed to create scaling policy: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get scaling policy handler
async fn get_scaling_policy_handler(
    State(state): State<ScalingPoliciesAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
) -> Result<Json<ScalingPolicyResponse>, StatusCode> {
    match state.service.get_policy(&policy_id).await {
        Ok(policy) => {
            let mut config = HashMap::new();

            if let Some(tt_config) = &policy.target_tracking_config {
                config.insert(
                    "target_tracking".to_string(),
                    serde_json::to_value(tt_config).unwrap(),
                );
            }
            if let Some(ss_config) = &policy.step_scaling_config {
                config.insert(
                    "step_scaling".to_string(),
                    serde_json::to_value(ss_config).unwrap(),
                );
            }
            if let Some(ps_config) = &policy.predictive_scaling_config {
                config.insert(
                    "predictive_scaling".to_string(),
                    serde_json::to_value(ps_config).unwrap(),
                );
            }

            Ok(Json(ScalingPolicyResponse {
                policy_id: policy.policy_id,
                name: policy.name,
                policy_type: format!("{:?}", policy.policy_type),
                pool_id: policy.pool_id,
                is_enabled: policy.is_enabled,
                created_at: policy.created_at,
                updated_at: policy.updated_at,
                config,
            }))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// List scaling policies handler
async fn list_scaling_policies_handler(
    State(state): State<ScalingPoliciesAppState>,
) -> Result<Json<Vec<ScalingPolicyResponse>>, StatusCode> {
    let policies = state.service.list_policies().await;

    let responses: Vec<ScalingPolicyResponse> = policies
        .into_iter()
        .map(|policy| {
            let mut config = HashMap::new();

            if let Some(tt_config) = &policy.target_tracking_config {
                config.insert(
                    "target_tracking".to_string(),
                    serde_json::to_value(tt_config).unwrap(),
                );
            }
            if let Some(ss_config) = &policy.step_scaling_config {
                config.insert(
                    "step_scaling".to_string(),
                    serde_json::to_value(ss_config).unwrap(),
                );
            }
            if let Some(ps_config) = &policy.predictive_scaling_config {
                config.insert(
                    "predictive_scaling".to_string(),
                    serde_json::to_value(ps_config).unwrap(),
                );
            }

            ScalingPolicyResponse {
                policy_id: policy.policy_id,
                name: policy.name,
                policy_type: format!("{:?}", policy.policy_type),
                pool_id: policy.pool_id,
                is_enabled: policy.is_enabled,
                created_at: policy.created_at,
                updated_at: policy.updated_at,
                config,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// List scaling policies by pool handler
async fn list_scaling_policies_by_pool_handler(
    State(state): State<ScalingPoliciesAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<Vec<ScalingPolicyResponse>>, StatusCode> {
    let policies = state.service.list_policies_by_pool(&pool_id).await;

    let responses: Vec<ScalingPolicyResponse> = policies
        .into_iter()
        .map(|policy| {
            let mut config = HashMap::new();

            if let Some(tt_config) = &policy.target_tracking_config {
                config.insert(
                    "target_tracking".to_string(),
                    serde_json::to_value(tt_config).unwrap(),
                );
            }
            if let Some(ss_config) = &policy.step_scaling_config {
                config.insert(
                    "step_scaling".to_string(),
                    serde_json::to_value(ss_config).unwrap(),
                );
            }
            if let Some(ps_config) = &policy.predictive_scaling_config {
                config.insert(
                    "predictive_scaling".to_string(),
                    serde_json::to_value(ps_config).unwrap(),
                );
            }

            ScalingPolicyResponse {
                policy_id: policy.policy_id,
                name: policy.name,
                policy_type: format!("{:?}", policy.policy_type),
                pool_id: policy.pool_id,
                is_enabled: policy.is_enabled,
                created_at: policy.created_at,
                updated_at: policy.updated_at,
                config,
            }
        })
        .collect();

    Ok(Json(responses))
}

/// Update scaling policy handler
async fn update_scaling_policy_handler(
    State(state): State<ScalingPoliciesAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
    Json(payload): Json<UpdateScalingPolicyRequest>,
) -> Result<Json<ScalingPolicyMessageResponse>, StatusCode> {
    match state.service.update_policy(&policy_id, payload).await {
        Ok(_) => Ok(Json(ScalingPolicyMessageResponse {
            message: format!("Scaling policy {} updated successfully", policy_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Delete scaling policy handler
async fn delete_scaling_policy_handler(
    State(state): State<ScalingPoliciesAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
) -> Result<Json<ScalingPolicyMessageResponse>, StatusCode> {
    match state.service.delete_policy(&policy_id).await {
        Ok(_) => Ok(Json(ScalingPolicyMessageResponse {
            message: format!("Scaling policy {} deleted successfully", policy_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Enable scaling policy handler
async fn enable_scaling_policy_handler(
    State(state): State<ScalingPoliciesAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
) -> Result<Json<ScalingPolicyMessageResponse>, StatusCode> {
    match state.service.set_policy_enabled(&policy_id, true).await {
        Ok(_) => Ok(Json(ScalingPolicyMessageResponse {
            message: format!("Scaling policy {} enabled successfully", policy_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Disable scaling policy handler
async fn disable_scaling_policy_handler(
    State(state): State<ScalingPoliciesAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
) -> Result<Json<ScalingPolicyMessageResponse>, StatusCode> {
    match state.service.set_policy_enabled(&policy_id, false).await {
        Ok(_) => Ok(Json(ScalingPolicyMessageResponse {
            message: format!("Scaling policy {} disabled successfully", policy_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get scaling policy statistics handler
async fn get_scaling_policy_stats_handler(
    State(state): State<ScalingPoliciesAppState>,
    axum::extract::Path(policy_id): axum::extract::Path<String>,
) -> Result<Json<ScalingPolicyStats>, StatusCode> {
    match state.service.get_policy_stats(&policy_id).await {
        Ok(stats) => Ok(Json(stats)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_target_tracking_policy() {
        let service = ScalingPoliciesService::new();

        let request = CreateScalingPolicyRequest {
            name: "CPU Target Tracking".to_string(),
            policy_type: "target_tracking".to_string(),
            pool_id: "pool-1".to_string(),
            is_enabled: true,
            target_tracking_config: Some(TargetTrackingConfig {
                metric_name: "cpu_utilization".to_string(),
                target_value: 70.0,
                metric_type: "cpu_utilization".to_string(),
                scale_in_cooldown: Duration::from_secs(300),
                scale_out_cooldown: Duration::from_secs(300),
            }),
            step_scaling_config: None,
            predictive_scaling_config: None,
        };

        let result = service.create_policy(request).await;
        assert!(result.is_ok());

        let policy = result.unwrap();
        assert_eq!(policy.name, "CPU Target Tracking");
        assert_eq!(policy.pool_id, "pool-1");
    }

    #[tokio::test]
    async fn test_list_policies() {
        let service = ScalingPoliciesService::new();

        let request = CreateScalingPolicyRequest {
            name: "Test Policy".to_string(),
            policy_type: "target_tracking".to_string(),
            pool_id: "pool-1".to_string(),
            is_enabled: true,
            target_tracking_config: Some(TargetTrackingConfig {
                metric_name: "cpu_utilization".to_string(),
                target_value: 70.0,
                metric_type: "cpu_utilization".to_string(),
                scale_in_cooldown: Duration::from_secs(300),
                scale_out_cooldown: Duration::from_secs(300),
            }),
            step_scaling_config: None,
            predictive_scaling_config: None,
        };

        service.create_policy(request).await.unwrap();

        let policies = service.list_policies().await;
        assert_eq!(policies.len(), 1);
    }

    #[tokio::test]
    async fn test_update_policy() {
        let service = ScalingPoliciesService::new();

        let request = CreateScalingPolicyRequest {
            name: "Test Policy".to_string(),
            policy_type: "target_tracking".to_string(),
            pool_id: "pool-1".to_string(),
            is_enabled: true,
            target_tracking_config: Some(TargetTrackingConfig {
                metric_name: "cpu_utilization".to_string(),
                target_value: 70.0,
                metric_type: "cpu_utilization".to_string(),
                scale_in_cooldown: Duration::from_secs(300),
                scale_out_cooldown: Duration::from_secs(300),
            }),
            step_scaling_config: None,
            predictive_scaling_config: None,
        };

        let policy = service.create_policy(request).await.unwrap();

        let update_request = UpdateScalingPolicyRequest {
            name: Some("Updated Policy".to_string()),
            is_enabled: Some(false),
            target_tracking_config: None,
            step_scaling_config: None,
            predictive_scaling_config: None,
        };

        let result = service
            .update_policy(&policy.policy_id, update_request)
            .await;
        assert!(result.is_ok());

        let updated = result.unwrap();
        assert_eq!(updated.name, "Updated Policy");
        assert_eq!(updated.is_enabled, false);
    }

    #[tokio::test]
    async fn test_delete_policy() {
        let service = ScalingPoliciesService::new();

        let request = CreateScalingPolicyRequest {
            name: "Test Policy".to_string(),
            policy_type: "target_tracking".to_string(),
            pool_id: "pool-1".to_string(),
            is_enabled: true,
            target_tracking_config: Some(TargetTrackingConfig {
                metric_name: "cpu_utilization".to_string(),
                target_value: 70.0,
                metric_type: "cpu_utilization".to_string(),
                scale_in_cooldown: Duration::from_secs(300),
                scale_out_cooldown: Duration::from_secs(300),
            }),
            step_scaling_config: None,
            predictive_scaling_config: None,
        };

        let policy = service.create_policy(request).await.unwrap();

        let result = service.delete_policy(&policy.policy_id).await;
        assert!(result.is_ok());

        let policies = service.list_policies().await;
        assert_eq!(policies.len(), 0);
    }

    #[tokio::test]
    async fn test_get_policy_stats() {
        let service = ScalingPoliciesService::new();

        let request = CreateScalingPolicyRequest {
            name: "Test Policy".to_string(),
            policy_type: "target_tracking".to_string(),
            pool_id: "pool-1".to_string(),
            is_enabled: true,
            target_tracking_config: Some(TargetTrackingConfig {
                metric_name: "cpu_utilization".to_string(),
                target_value: 70.0,
                metric_type: "cpu_utilization".to_string(),
                scale_in_cooldown: Duration::from_secs(300),
                scale_out_cooldown: Duration::from_secs(300),
            }),
            step_scaling_config: None,
            predictive_scaling_config: None,
        };

        let policy = service.create_policy(request).await.unwrap();

        // Record some executions
        service
            .record_execution(&policy.policy_id, "scale_out", 5, 10, 75.0)
            .await;
        service
            .record_execution(&policy.policy_id, "scale_in", 10, 5, 50.0)
            .await;

        let stats = service.get_policy_stats(&policy.policy_id).await.unwrap();
        assert_eq!(stats.executions_count, 2);
        assert_eq!(stats.scale_out_events, 1);
        assert_eq!(stats.scale_in_events, 1);
    }

    #[tokio::test]
    async fn test_invalid_policy_type() {
        let service = ScalingPoliciesService::new();

        let request = CreateScalingPolicyRequest {
            name: "Test Policy".to_string(),
            policy_type: "invalid_type".to_string(),
            pool_id: "pool-1".to_string(),
            is_enabled: true,
            target_tracking_config: None,
            step_scaling_config: None,
            predictive_scaling_config: None,
        };

        let result = service.create_policy(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid policy type"));
    }
}
