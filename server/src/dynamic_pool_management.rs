//! Dynamic Pool Management API Module
//!
//! Provides management for dynamic resource pools that automatically scale
//! based on demand. Dynamic pools provision workers on-demand and scale up/down.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Application state for dynamic pool management
#[derive(Clone)]
pub struct DynamicPoolManagementAppState {
    pub dynamic_pools: Arc<RwLock<HashMap<String, DynamicPool>>>,
    pub pool_configs: Arc<RwLock<HashMap<String, DynamicPoolConfig>>>,
    pub scaling_history: Arc<RwLock<HashMap<String, Vec<ScalingEvent>>>>,
}

/// Dynamic pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPoolConfig {
    pub pool_id: String,
    pub name: String,
    pub provider_type: String,
    pub min_size: u32,
    pub max_size: u32,
    pub current_size: u32,
    pub target_size: u32,
    pub scaling_policy: ScalingPolicy,
    pub cooldown_period: Duration,
    pub health_check_interval: Duration,
    pub metadata: HashMap<String, String>,
    pub auto_scaling: bool,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub scale_up_cooldown: Duration,
    pub scale_down_cooldown: Duration,
}

/// Scaling policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPolicy {
    pub policy_type: ScalingPolicyType,
    pub target_utilization: f64,
    pub scale_up_increment: u32,
    pub scale_down_increment: u32,
    pub stabilization_window: Duration,
}

/// Scaling policy types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingPolicyType {
    TargetTracking,
    StepScaling,
    Predictive,
}

/// Dynamic pool status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPoolStatus {
    pub pool_id: String,
    pub name: String,
    pub state: DynamicPoolState,
    pub current_size: u32,
    pub target_size: u32,
    pub pending_scale_up: u32,
    pub pending_scale_down: u32,
    pub utilization_rate: f64,
    pub pending_requests: u32,
    pub last_scale_event: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dynamic pool state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DynamicPoolState {
    Creating,
    Provisioning,
    Active,
    ScalingUp,
    ScalingDown,
    Degraded,
    Draining,
    Destroyed,
}

/// Dynamic pool definition
#[derive(Debug, Clone)]
pub struct DynamicPool {
    pub config: DynamicPoolConfig,
    pub workers: Vec<DynamicWorker>,
    pub status: DynamicPoolStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dynamic worker in a pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicWorker {
    pub worker_id: String,
    pub status: DynamicWorkerStatus,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Dynamic worker status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DynamicWorkerStatus {
    Provisioning,
    Starting,
    Ready,
    Busy,
    Terminating,
    Terminated,
}

/// Scaling event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: ScalingEventType,
    pub old_size: u32,
    pub new_size: u32,
    pub trigger: String,
}

/// Scaling event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingEventType {
    ScaleUp,
    ScaleDown,
    ScaleTo,
}

/// Service for dynamic pool management operations
#[derive(Clone)]
pub struct DynamicPoolManagementService {
    dynamic_pools: Arc<RwLock<HashMap<String, DynamicPool>>>,
    pool_configs: Arc<RwLock<HashMap<String, DynamicPoolConfig>>>,
    scaling_history: Arc<RwLock<HashMap<String, Vec<ScalingEvent>>>>,
}

impl DynamicPoolManagementService {
    pub fn new(
        dynamic_pools: Arc<RwLock<HashMap<String, DynamicPool>>>,
        pool_configs: Arc<RwLock<HashMap<String, DynamicPoolConfig>>>,
        scaling_history: Arc<RwLock<HashMap<String, Vec<ScalingEvent>>>>,
    ) -> Self {
        Self {
            dynamic_pools,
            pool_configs,
            scaling_history,
        }
    }

    /// Create a new dynamic pool
    pub async fn create_dynamic_pool(
        &self,
        request: CreateDynamicPoolRequest,
    ) -> Result<DynamicPoolResponse, String> {
        if request.min_size > request.max_size {
            return Err("min_size cannot be greater than max_size".to_string());
        }

        let pool_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let scaling_policy = ScalingPolicy {
            policy_type: request.scaling_policy.policy_type,
            target_utilization: request.scaling_policy.target_utilization,
            scale_up_increment: request.scaling_policy.scale_up_increment,
            scale_down_increment: request.scaling_policy.scale_down_increment,
            stabilization_window: request.scaling_policy.stabilization_window,
        };

        let config = DynamicPoolConfig {
            pool_id: pool_id.clone(),
            name: request.name,
            provider_type: request.provider_type,
            min_size: request.min_size,
            max_size: request.max_size,
            current_size: request.initial_size.unwrap_or(request.min_size),
            target_size: request.initial_size.unwrap_or(request.min_size),
            scaling_policy,
            cooldown_period: request.cooldown_period,
            health_check_interval: request.health_check_interval,
            metadata: request.metadata.unwrap_or_default(),
            auto_scaling: request.auto_scaling,
            scale_up_threshold: request.scale_up_threshold,
            scale_down_threshold: request.scale_down_threshold,
            scale_up_cooldown: request.scale_up_cooldown,
            scale_down_cooldown: request.scale_down_cooldown,
        };

        let status = DynamicPoolStatus {
            pool_id: pool_id.clone(),
            name: config.name.clone(),
            state: DynamicPoolState::Creating,
            current_size: 0,
            target_size: config.target_size,
            pending_scale_up: 0,
            pending_scale_down: 0,
            utilization_rate: 0.0,
            pending_requests: 0,
            last_scale_event: None,
            created_at: now,
            updated_at: now,
        };

        let pool = DynamicPool {
            config: config.clone(),
            workers: Vec::new(),
            status: status.clone(),
            created_at: now,
            updated_at: now,
        };

        // Store pool config
        let mut configs = self.pool_configs.write().await;
        configs.insert(pool_id.clone(), config.clone());

        // Store pool
        let mut pools = self.dynamic_pools.write().await;
        pools.insert(pool_id.clone(), pool);

        info!("Created dynamic pool: {}", config.name);

        Ok(DynamicPoolResponse {
            id: pool_id,
            config,
            status,
        })
    }

    /// Get dynamic pool by ID
    pub async fn get_dynamic_pool(&self, pool_id: &str) -> Result<DynamicPoolResponse, String> {
        let pools = self.dynamic_pools.read().await;
        match pools.get(pool_id) {
            Some(pool) => Ok(DynamicPoolResponse {
                id: pool_id.to_string(),
                config: pool.config.clone(),
                status: pool.status.clone(),
            }),
            None => Err("Dynamic pool not found".to_string()),
        }
    }

    /// List all dynamic pools
    pub async fn list_dynamic_pools(&self) -> Result<Vec<DynamicPoolResponse>, String> {
        let pools = self.dynamic_pools.read().await;
        let result = pools
            .iter()
            .map(|(id, pool)| DynamicPoolResponse {
                id: id.clone(),
                config: pool.config.clone(),
                status: pool.status.clone(),
            })
            .collect();
        Ok(result)
    }

    /// Scale dynamic pool to target size
    pub async fn scale_pool(
        &self,
        pool_id: &str,
        target_size: u32,
    ) -> Result<DynamicPoolResponse, String> {
        let mut pools = self.dynamic_pools.write().await;
        match pools.get_mut(pool_id) {
            Some(pool) => {
                if target_size < pool.config.min_size || target_size > pool.config.max_size {
                    return Err("Target size out of bounds".to_string());
                }

                let old_size = pool.config.current_size;
                let now = Utc::now();

                // Determine scaling direction
                let event_type = if target_size > old_size {
                    ScalingEventType::ScaleUp
                } else if target_size < old_size {
                    ScalingEventType::ScaleDown
                } else {
                    ScalingEventType::ScaleTo
                };

                // Update pool status
                pool.status.state = if target_size > old_size {
                    DynamicPoolState::ScalingUp
                } else if target_size < old_size {
                    DynamicPoolState::ScalingDown
                } else {
                    DynamicPoolState::Active
                };

                pool.config.target_size = target_size;
                pool.status.target_size = target_size;
                pool.status.updated_at = now;

                // Record scaling event
                let event = ScalingEvent {
                    timestamp: now,
                    event_type,
                    old_size,
                    new_size: target_size,
                    trigger: "manual".to_string(),
                };

                let mut history = self.scaling_history.write().await;
                history
                    .entry(pool_id.to_string())
                    .or_insert_with(Vec::new)
                    .push(event);

                info!(
                    "Scaled dynamic pool {} from {} to {} workers",
                    pool.config.name, old_size, target_size
                );

                Ok(DynamicPoolResponse {
                    id: pool_id.to_string(),
                    config: pool.config.clone(),
                    status: pool.status.clone(),
                })
            }
            None => Err("Dynamic pool not found".to_string()),
        }
    }

    /// Update dynamic pool configuration
    pub async fn update_dynamic_pool(
        &self,
        pool_id: &str,
        updates: UpdateDynamicPoolRequest,
    ) -> Result<DynamicPoolResponse, String> {
        let mut pools = self.dynamic_pools.write().await;
        match pools.get_mut(pool_id) {
            Some(pool) => {
                if let Some(name) = updates.name {
                    pool.config.name = name;
                    pool.status.name = pool.config.name.clone();
                }

                if let Some(min_size) = updates.min_size {
                    pool.config.min_size = min_size;
                }

                if let Some(max_size) = updates.max_size {
                    pool.config.max_size = max_size;
                }

                if let Some(auto_scaling) = updates.auto_scaling {
                    pool.config.auto_scaling = auto_scaling;
                }

                if let Some(scale_up_threshold) = updates.scale_up_threshold {
                    pool.config.scale_up_threshold = scale_up_threshold;
                }

                if let Some(scale_down_threshold) = updates.scale_down_threshold {
                    pool.config.scale_down_threshold = scale_down_threshold;
                }

                if let Some(metadata) = updates.metadata {
                    pool.config.metadata = metadata;
                }

                pool.status.updated_at = Utc::now();

                info!("Updated dynamic pool: {}", pool.config.name);

                Ok(DynamicPoolResponse {
                    id: pool_id.to_string(),
                    config: pool.config.clone(),
                    status: pool.status.clone(),
                })
            }
            None => Err("Dynamic pool not found".to_string()),
        }
    }

    /// Delete dynamic pool
    pub async fn delete_dynamic_pool(&self, pool_id: &str) -> Result<(), String> {
        let mut pools = self.dynamic_pools.write().await;
        match pools.remove(pool_id) {
            Some(pool) => {
                let mut configs = self.pool_configs.write().await;
                configs.remove(pool_id);

                let mut history = self.scaling_history.write().await;
                history.remove(pool_id);

                info!("Deleted dynamic pool: {}", pool.config.name);
                Ok(())
            }
            None => Err("Dynamic pool not found".to_string()),
        }
    }

    /// Get pool scaling history
    pub async fn get_scaling_history(&self, pool_id: &str) -> Result<Vec<ScalingEvent>, String> {
        let history = self.scaling_history.read().await;
        match history.get(pool_id) {
            Some(events) => Ok(events.clone()),
            None => Err("Scaling history not found".to_string()),
        }
    }

    /// Get pool metrics
    pub async fn get_pool_metrics(
        &self,
        pool_id: &str,
    ) -> Result<DynamicPoolMetricsResponse, String> {
        let pools = self.dynamic_pools.read().await;
        match pools.get(pool_id) {
            Some(pool) => {
                let history = self.scaling_history.read().await;
                let events = history.get(pool_id).cloned().unwrap_or_else(Vec::new);

                let recent_events = events
                    .iter()
                    .filter(|e| Utc::now().signed_duration_since(e.timestamp).num_hours() < 24)
                    .count();

                Ok(DynamicPoolMetricsResponse {
                    pool_id: pool_id.to_string(),
                    current_size: pool.status.current_size,
                    target_size: pool.status.target_size,
                    utilization_rate: pool.status.utilization_rate,
                    pending_requests: pool.status.pending_requests,
                    scaling_events_24h: recent_events as u32,
                    average_scale_up_time: Duration::from_secs(30), // Placeholder
                    average_scale_down_time: Duration::from_secs(45), // Placeholder
                })
            }
            None => Err("Dynamic pool not found".to_string()),
        }
    }
}

/// Request to create a dynamic pool
#[derive(Debug, Deserialize)]
pub struct CreateDynamicPoolRequest {
    pub name: String,
    pub provider_type: String,
    pub min_size: u32,
    pub max_size: u32,
    pub initial_size: Option<u32>,
    pub scaling_policy: ScalingPolicyRequest,
    pub cooldown_period: Duration,
    pub health_check_interval: Duration,
    pub metadata: Option<HashMap<String, String>>,
    pub auto_scaling: bool,
    pub scale_up_threshold: f64,
    pub scale_down_threshold: f64,
    pub scale_up_cooldown: Duration,
    pub scale_down_cooldown: Duration,
}

/// Scaling policy request
#[derive(Debug, Deserialize)]
pub struct ScalingPolicyRequest {
    pub policy_type: ScalingPolicyType,
    pub target_utilization: f64,
    pub scale_up_increment: u32,
    pub scale_down_increment: u32,
    pub stabilization_window: Duration,
}

/// Request to update a dynamic pool
#[derive(Debug, Deserialize, Default)]
pub struct UpdateDynamicPoolRequest {
    pub name: Option<String>,
    pub min_size: Option<u32>,
    pub max_size: Option<u32>,
    pub auto_scaling: Option<bool>,
    pub scale_up_threshold: Option<f64>,
    pub scale_down_threshold: Option<f64>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Request to scale a dynamic pool
#[derive(Debug, Deserialize)]
pub struct ScaleDynamicPoolRequest {
    pub target_size: u32,
}

/// Response for dynamic pool operations
#[derive(Debug, Serialize)]
pub struct DynamicPoolResponse {
    pub id: String,
    pub config: DynamicPoolConfig,
    pub status: DynamicPoolStatus,
}

/// Response for pool metrics
#[derive(Debug, Serialize)]
pub struct DynamicPoolMetricsResponse {
    pub pool_id: String,
    pub current_size: u32,
    pub target_size: u32,
    pub utilization_rate: f64,
    pub pending_requests: u32,
    pub scaling_events_24h: u32,
    pub average_scale_up_time: Duration,
    pub average_scale_down_time: Duration,
}

/// Create a new dynamic pool
pub async fn create_dynamic_pool_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
    Json(payload): Json<CreateDynamicPoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.create_dynamic_pool(payload).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e.contains("cannot be greater") {
                Err((StatusCode::BAD_REQUEST, e))
            } else {
                error!("Failed to create dynamic pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Get a specific dynamic pool
pub async fn get_dynamic_pool_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.get_dynamic_pool(&pool_id).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Dynamic pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to get dynamic pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// List all dynamic pools
pub async fn list_dynamic_pools_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.list_dynamic_pools().await {
        Ok(pools) => Ok(Json(pools)),
        Err(e) => {
            error!("Failed to list dynamic pools: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Scale dynamic pool
pub async fn scale_dynamic_pool_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<ScaleDynamicPoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.scale_pool(&pool_id, payload.target_size).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Dynamic pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else if e.contains("out of bounds") {
                Err((StatusCode::BAD_REQUEST, e))
            } else {
                error!("Failed to scale dynamic pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Update dynamic pool
pub async fn update_dynamic_pool_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<UpdateDynamicPoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.update_dynamic_pool(&pool_id, payload).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Dynamic pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to update dynamic pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Delete dynamic pool
pub async fn delete_dynamic_pool_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.delete_dynamic_pool(&pool_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            if e == "Dynamic pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to delete dynamic pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Get dynamic pool scaling history
pub async fn get_dynamic_pool_scaling_history_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.get_scaling_history(&pool_id).await {
        Ok(history) => Ok(Json(history)),
        Err(e) => {
            if e == "Scaling history not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to get scaling history: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Get dynamic pool metrics
pub async fn get_dynamic_pool_metrics_handler(
    State(app_state): State<DynamicPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = DynamicPoolManagementService::new(
        app_state.dynamic_pools,
        app_state.pool_configs,
        app_state.scaling_history,
    );
    match service.get_pool_metrics(&pool_id).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => {
            if e == "Dynamic pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to get dynamic pool metrics: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Create dynamic pool management router
pub fn dynamic_pool_management_routes() -> Router<DynamicPoolManagementAppState> {
    Router::new()
        .route("/dynamic-pools", post(create_dynamic_pool_handler))
        .route("/dynamic-pools", get(list_dynamic_pools_handler))
        .route("/dynamic-pools/{pool_id}", get(get_dynamic_pool_handler))
        .route("/dynamic-pools/{pool_id}", put(update_dynamic_pool_handler))
        .route(
            "/dynamic-pools/{pool_id}",
            delete(delete_dynamic_pool_handler),
        )
        .route(
            "/dynamic-pools/{pool_id}/scale",
            post(scale_dynamic_pool_handler),
        )
        .route(
            "/dynamic-pools/{pool_id}/scaling-history",
            get(get_dynamic_pool_scaling_history_handler),
        )
        .route(
            "/dynamic-pools/{pool_id}/metrics",
            get(get_dynamic_pool_metrics_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_create_dynamic_pool() {
        let dynamic_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let scaling_history = Arc::new(RwLock::new(HashMap::new()));
        let service =
            DynamicPoolManagementService::new(dynamic_pools, pool_configs, scaling_history);

        let request = CreateDynamicPoolRequest {
            name: "test-dynamic-pool".to_string(),
            provider_type: "kubernetes".to_string(),
            min_size: 2,
            max_size: 20,
            initial_size: Some(5),
            scaling_policy: ScalingPolicyRequest {
                policy_type: ScalingPolicyType::TargetTracking,
                target_utilization: 70.0,
                scale_up_increment: 2,
                scale_down_increment: 1,
                stabilization_window: Duration::from_secs(300),
            },
            cooldown_period: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            metadata: Some(HashMap::from([("env".to_string(), "test".to_string())])),
            auto_scaling: true,
            scale_up_threshold: 80.0,
            scale_down_threshold: 50.0,
            scale_up_cooldown: Duration::from_secs(60),
            scale_down_cooldown: Duration::from_secs(120),
        };

        let result = service.create_dynamic_pool(request).await.unwrap();
        assert_eq!(result.config.name, "test-dynamic-pool");
        assert_eq!(result.config.min_size, 2);
        assert_eq!(result.config.max_size, 20);
    }

    #[tokio::test]
    async fn test_list_dynamic_pools() {
        let dynamic_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let scaling_history = Arc::new(RwLock::new(HashMap::new()));
        let service =
            DynamicPoolManagementService::new(dynamic_pools, pool_configs, scaling_history);

        let request = CreateDynamicPoolRequest {
            name: "pool1".to_string(),
            provider_type: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            initial_size: None,
            scaling_policy: ScalingPolicyRequest {
                policy_type: ScalingPolicyType::StepScaling,
                target_utilization: 75.0,
                scale_up_increment: 1,
                scale_down_increment: 1,
                stabilization_window: Duration::from_secs(300),
            },
            cooldown_period: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            metadata: None,
            auto_scaling: true,
            scale_up_threshold: 80.0,
            scale_down_threshold: 40.0,
            scale_up_cooldown: Duration::from_secs(60),
            scale_down_cooldown: Duration::from_secs(120),
        };

        service.create_dynamic_pool(request.clone()).await.unwrap();
        service.create_dynamic_pool(request).await.unwrap();

        let pools = service.list_dynamic_pools().await.unwrap();
        assert_eq!(pools.len(), 2);
    }

    #[tokio::test]
    async fn test_scale_pool() {
        let dynamic_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let scaling_history = Arc::new(RwLock::new(HashMap::new()));
        let service =
            DynamicPoolManagementService::new(dynamic_pools, pool_configs, scaling_history);

        let request = CreateDynamicPoolRequest {
            name: "scale-test".to_string(),
            provider_type: "docker".to_string(),
            min_size: 1,
            max_size: 20,
            initial_size: None,
            scaling_policy: ScalingPolicyRequest {
                policy_type: ScalingPolicyType::TargetTracking,
                target_utilization: 70.0,
                scale_up_increment: 2,
                scale_down_increment: 1,
                stabilization_window: Duration::from_secs(300),
            },
            cooldown_period: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            metadata: None,
            auto_scaling: true,
            scale_up_threshold: 80.0,
            scale_down_threshold: 50.0,
            scale_up_cooldown: Duration::from_secs(60),
            scale_down_cooldown: Duration::from_secs(120),
        };

        let pool = service.create_dynamic_pool(request).await.unwrap();
        let scaled = service.scale_pool(&pool.id, 10).await.unwrap();

        assert_eq!(scaled.config.target_size, 10);
    }

    #[tokio::test]
    async fn test_get_scaling_history() {
        let dynamic_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let scaling_history = Arc::new(RwLock::new(HashMap::new()));
        let service =
            DynamicPoolManagementService::new(dynamic_pools, pool_configs, scaling_history);

        let request = CreateDynamicPoolRequest {
            name: "history-test".to_string(),
            provider_type: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            initial_size: None,
            scaling_policy: ScalingPolicyRequest {
                policy_type: ScalingPolicyType::TargetTracking,
                target_utilization: 70.0,
                scale_up_increment: 1,
                scale_down_increment: 1,
                stabilization_window: Duration::from_secs(300),
            },
            cooldown_period: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            metadata: None,
            auto_scaling: true,
            scale_up_threshold: 80.0,
            scale_down_threshold: 50.0,
            scale_up_cooldown: Duration::from_secs(60),
            scale_down_cooldown: Duration::from_secs(120),
        };

        let pool = service.create_dynamic_pool(request).await.unwrap();
        service.scale_pool(&pool.id, 5).await.unwrap();

        let history = service.get_scaling_history(&pool.id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].new_size, 5);
    }

    #[tokio::test]
    async fn test_get_pool_metrics() {
        let dynamic_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let scaling_history = Arc::new(RwLock::new(HashMap::new()));
        let service =
            DynamicPoolManagementService::new(dynamic_pools, pool_configs, scaling_history);

        let request = CreateDynamicPoolRequest {
            name: "metrics-test".to_string(),
            provider_type: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            initial_size: None,
            scaling_policy: ScalingPolicyRequest {
                policy_type: ScalingPolicyType::TargetTracking,
                target_utilization: 70.0,
                scale_up_increment: 1,
                scale_down_increment: 1,
                stabilization_window: Duration::from_secs(300),
            },
            cooldown_period: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            metadata: None,
            auto_scaling: true,
            scale_up_threshold: 80.0,
            scale_down_threshold: 50.0,
            scale_up_cooldown: Duration::from_secs(60),
            scale_down_cooldown: Duration::from_secs(120),
        };

        let pool = service.create_dynamic_pool(request).await.unwrap();
        let metrics = service.get_pool_metrics(&pool.id).await.unwrap();

        assert_eq!(metrics.pool_id, pool.id);
    }
}
