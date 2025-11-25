//! Static Pool Management API Module
//!
//! Provides management for static resource pools with pre-provisioned workers.
//! Static pools maintain a fixed number of workers that are always ready.

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

use hodei_ports::resource_pool::{ResourcePoolConfig, ResourcePoolStatus, ResourcePoolType};

/// Response message
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

/// Application state for static pool management
#[derive(Clone)]
pub struct StaticPoolManagementAppState {
    pub static_pools: Arc<RwLock<HashMap<String, StaticPool>>>,
    pub pool_configs: Arc<RwLock<HashMap<String, StaticPoolConfig>>>,
}

/// Static pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticPoolConfig {
    pub pool_id: String,
    pub name: String,
    pub provider_type: String,
    pub min_size: u32,
    pub max_size: u32,
    pub current_size: u32,
    pub pre_warm: bool,
    pub health_check_interval: Duration,
    pub metadata: HashMap<String, String>,
    pub auto_recovery: bool,
    pub reserved_capacity: u32,
}

/// Static pool status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticPoolStatus {
    pub pool_id: String,
    pub name: String,
    pub state: StaticPoolState,
    pub current_size: u32,
    pub healthy_workers: u32,
    pub unavailable_workers: u32,
    pub reserved_workers: u32,
    pub available_workers: u32,
    pub utilization_rate: f64,
    pub last_health_check: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Static pool state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaticPoolState {
    Creating,
    Provisioning,
    Active,
    Degraded,
    Recovering,
    Draining,
    Destroyed,
}

/// Static pool definition
#[derive(Debug, Clone)]
pub struct StaticPool {
    pub config: StaticPoolConfig,
    pub workers: Vec<StaticWorker>,
    pub status: StaticPoolStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Static worker in a pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticWorker {
    pub worker_id: String,
    pub status: WorkerStatus,
    pub reserved: bool,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Worker status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerStatus {
    Pending,
    Provisioning,
    Ready,
    Busy,
    Unavailable,
    Terminated,
}

/// Service for static pool management operations
#[derive(Clone)]
pub struct StaticPoolManagementService {
    static_pools: Arc<RwLock<HashMap<String, StaticPool>>>,
    pool_configs: Arc<RwLock<HashMap<String, StaticPoolConfig>>>,
}

impl StaticPoolManagementService {
    pub fn new(
        static_pools: Arc<RwLock<HashMap<String, StaticPool>>>,
        pool_configs: Arc<RwLock<HashMap<String, StaticPoolConfig>>>,
    ) -> Self {
        Self {
            static_pools,
            pool_configs,
        }
    }

    /// Create a new static pool
    pub async fn create_static_pool(
        &self,
        request: CreateStaticPoolRequest,
    ) -> Result<StaticPoolResponse, String> {
        if request.min_size > request.max_size {
            return Err("min_size cannot be greater than max_size".to_string());
        }

        let pool_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let config = StaticPoolConfig {
            pool_id: pool_id.clone(),
            name: request.name,
            provider_type: request.provider_type,
            min_size: request.min_size,
            max_size: request.max_size,
            current_size: request.initial_size.unwrap_or(request.min_size),
            pre_warm: request.pre_warm,
            health_check_interval: request.health_check_interval,
            metadata: request.metadata.unwrap_or_default(),
            auto_recovery: request.auto_recovery,
            reserved_capacity: request.reserved_capacity.unwrap_or(0),
        };

        let status = StaticPoolStatus {
            pool_id: pool_id.clone(),
            name: config.name.clone(),
            state: StaticPoolState::Creating,
            current_size: 0,
            healthy_workers: 0,
            unavailable_workers: 0,
            reserved_workers: 0,
            available_workers: 0,
            utilization_rate: 0.0,
            last_health_check: None,
            created_at: now,
            updated_at: now,
        };

        let pool = StaticPool {
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
        let mut pools = self.static_pools.write().await;
        pools.insert(pool_id.clone(), pool);

        info!("Created static pool: {}", config.name);

        Ok(StaticPoolResponse {
            id: pool_id,
            config,
            status,
        })
    }

    /// Get static pool by ID
    pub async fn get_static_pool(&self, pool_id: &str) -> Result<StaticPoolResponse, String> {
        let pools = self.static_pools.read().await;
        match pools.get(pool_id) {
            Some(pool) => Ok(StaticPoolResponse {
                id: pool_id.to_string(),
                config: pool.config.clone(),
                status: pool.status.clone(),
            }),
            None => Err("Static pool not found".to_string()),
        }
    }

    /// List all static pools
    pub async fn list_static_pools(&self) -> Result<Vec<StaticPoolResponse>, String> {
        let pools = self.static_pools.read().await;
        let result = pools
            .iter()
            .map(|(id, pool)| StaticPoolResponse {
                id: id.clone(),
                config: pool.config.clone(),
                status: pool.status.clone(),
            })
            .collect();
        Ok(result)
    }

    /// Scale static pool to target size
    pub async fn scale_pool(
        &self,
        pool_id: &str,
        target_size: u32,
    ) -> Result<StaticPoolResponse, String> {
        let mut pools = self.static_pools.write().await;
        match pools.get_mut(pool_id) {
            Some(pool) => {
                if target_size < pool.config.min_size || target_size > pool.config.max_size {
                    return Err("Target size out of bounds".to_string());
                }

                pool.status.state = StaticPoolState::Provisioning;
                pool.config.current_size = target_size;
                pool.status.updated_at = Utc::now();

                // Update availability
                pool.status.available_workers =
                    target_size.saturating_sub(pool.status.reserved_workers);
                pool.status.utilization_rate = if target_size > 0 {
                    (pool.status.reserved_workers as f64 / target_size as f64) * 100.0
                } else {
                    0.0
                };

                info!(
                    "Scaled static pool {} to size {}",
                    pool.config.name, target_size
                );

                Ok(StaticPoolResponse {
                    id: pool_id.to_string(),
                    config: pool.config.clone(),
                    status: pool.status.clone(),
                })
            }
            None => Err("Static pool not found".to_string()),
        }
    }

    /// Update static pool configuration
    pub async fn update_static_pool(
        &self,
        pool_id: &str,
        updates: UpdateStaticPoolRequest,
    ) -> Result<StaticPoolResponse, String> {
        let mut pools = self.static_pools.write().await;
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

                if let Some(pre_warm) = updates.pre_warm {
                    pool.config.pre_warm = pre_warm;
                }

                if let Some(auto_recovery) = updates.auto_recovery {
                    pool.config.auto_recovery = auto_recovery;
                }

                if let Some(reserved_capacity) = updates.reserved_capacity {
                    pool.config.reserved_capacity = reserved_capacity;
                    pool.status.reserved_workers = reserved_capacity;
                    pool.status.available_workers =
                        pool.config.current_size.saturating_sub(reserved_capacity);
                }

                if let Some(metadata) = updates.metadata {
                    pool.config.metadata = metadata;
                }

                pool.status.updated_at = Utc::now();

                info!("Updated static pool: {}", pool.config.name);

                Ok(StaticPoolResponse {
                    id: pool_id.to_string(),
                    config: pool.config.clone(),
                    status: pool.status.clone(),
                })
            }
            None => Err("Static pool not found".to_string()),
        }
    }

    /// Delete static pool
    pub async fn delete_static_pool(&self, pool_id: &str) -> Result<(), String> {
        let mut pools = self.static_pools.write().await;
        match pools.remove(pool_id) {
            Some(pool) => {
                let mut configs = self.pool_configs.write().await;
                configs.remove(pool_id);

                info!("Deleted static pool: {}", pool.config.name);
                Ok(())
            }
            None => Err("Static pool not found".to_string()),
        }
    }

    /// Get pool health status
    pub async fn get_pool_health(&self, pool_id: &str) -> Result<StaticPoolHealthResponse, String> {
        let pools = self.static_pools.read().await;
        match pools.get(pool_id) {
            Some(pool) => {
                let health_percentage = if pool.status.current_size > 0 {
                    (pool.status.healthy_workers as f64 / pool.status.current_size as f64) * 100.0
                } else {
                    0.0
                };

                let health_status = match health_percentage {
                    100.0 => "HEALTHY".to_string(),
                    80.0..=99.9 => "DEGRADED".to_string(),
                    _ => "UNHEALTHY".to_string(),
                };

                Ok(StaticPoolHealthResponse {
                    pool_id: pool_id.to_string(),
                    health_status,
                    health_percentage,
                    total_workers: pool.status.current_size,
                    healthy_workers: pool.status.healthy_workers,
                    unavailable_workers: pool.status.unavailable_workers,
                    last_health_check: pool.status.last_health_check,
                })
            }
            None => Err("Static pool not found".to_string()),
        }
    }

    /// Pre-warm static pool
    pub async fn pre_warm_pool(&self, pool_id: &str) -> Result<(), String> {
        let mut pools = self.static_pools.write().await;
        match pools.get_mut(pool_id) {
            Some(pool) => {
                if !pool.config.pre_warm {
                    return Err("Pre-warm is not enabled for this pool".to_string());
                }

                pool.status.state = StaticPoolState::Provisioning;
                pool.status.updated_at = Utc::now();

                info!("Pre-warming static pool: {}", pool.config.name);

                // In a real implementation, this would provision the workers
                pool.status.current_size = pool.config.current_size.max(pool.config.min_size);
                pool.status.healthy_workers = pool.status.current_size;
                pool.status.available_workers = pool.status.current_size;
                pool.status.state = StaticPoolState::Active;
                pool.status.updated_at = Utc::now();

                Ok(())
            }
            None => Err("Static pool not found".to_string()),
        }
    }
}

/// Request to create a static pool
#[derive(Debug, Deserialize)]
pub struct CreateStaticPoolRequest {
    pub name: String,
    pub provider_type: String,
    pub min_size: u32,
    pub max_size: u32,
    pub initial_size: Option<u32>,
    pub pre_warm: bool,
    pub health_check_interval: Duration,
    pub metadata: Option<HashMap<String, String>>,
    pub auto_recovery: bool,
    pub reserved_capacity: Option<u32>,
}

/// Request to update a static pool
#[derive(Debug, Deserialize, Default)]
pub struct UpdateStaticPoolRequest {
    pub name: Option<String>,
    pub min_size: Option<u32>,
    pub max_size: Option<u32>,
    pub pre_warm: Option<bool>,
    pub auto_recovery: Option<bool>,
    pub reserved_capacity: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Response for static pool operations
#[derive(Debug, Serialize)]
pub struct StaticPoolResponse {
    pub id: String,
    pub config: StaticPoolConfig,
    pub status: StaticPoolStatus,
}

/// Response for pool health
#[derive(Debug, Serialize)]
pub struct StaticPoolHealthResponse {
    pub pool_id: String,
    pub health_status: String,
    pub health_percentage: f64,
    pub total_workers: u32,
    pub healthy_workers: u32,
    pub unavailable_workers: u32,
    pub last_health_check: Option<DateTime<Utc>>,
}

/// Create a new static pool
pub async fn create_static_pool_handler(
    State(app_state): State<StaticPoolManagementAppState>,
    Json(payload): Json<CreateStaticPoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.create_static_pool(payload).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e.contains("cannot be greater") {
                Err((StatusCode::BAD_REQUEST, e))
            } else {
                error!("Failed to create static pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Get a specific static pool
pub async fn get_static_pool_handler(
    State(app_state): State<StaticPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.get_static_pool(&pool_id).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Static pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to get static pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// List all static pools
pub async fn list_static_pools_handler(
    State(app_state): State<StaticPoolManagementAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.list_static_pools().await {
        Ok(pools) => Ok(Json(pools)),
        Err(e) => {
            error!("Failed to list static pools: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Scale static pool
pub async fn scale_static_pool_handler(
    State(app_state): State<StaticPoolManagementAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<ScaleStaticPoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.scale_pool(&pool_id, payload.target_size).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Static pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else if e.contains("out of bounds") {
                Err((StatusCode::BAD_REQUEST, e))
            } else {
                error!("Failed to scale static pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Update static pool
pub async fn update_static_pool_handler(
    State(app_state): State<StaticPoolManagementAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<UpdateStaticPoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.update_static_pool(&pool_id, payload).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Static pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to update static pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Delete static pool
pub async fn delete_static_pool_handler(
    State(app_state): State<StaticPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.delete_static_pool(&pool_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            if e == "Static pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to delete static pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Get static pool health
pub async fn get_static_pool_health_handler(
    State(app_state): State<StaticPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.get_pool_health(&pool_id).await {
        Ok(health) => Ok(Json(health)),
        Err(e) => {
            if e == "Static pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to get static pool health: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Pre-warm static pool
pub async fn pre_warm_static_pool_handler(
    State(app_state): State<StaticPoolManagementAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = StaticPoolManagementService::new(app_state.static_pools, app_state.pool_configs);
    match service.pre_warm_pool(&pool_id).await {
        Ok(_) => Ok(Json(MessageResponse {
            message: "Static pool pre-warm initiated".to_string(),
        })),
        Err(e) => {
            if e == "Static pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else if e.contains("not enabled") {
                Err((StatusCode::BAD_REQUEST, e))
            } else {
                error!("Failed to pre-warm static pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Request to scale a static pool
#[derive(Debug, Deserialize)]
pub struct ScaleStaticPoolRequest {
    pub target_size: u32,
}

/// Create static pool management router
pub fn static_pool_management_routes() -> Router<StaticPoolManagementAppState> {
    Router::new()
        .route("/static-pools", post(create_static_pool_handler))
        .route("/static-pools", get(list_static_pools_handler))
        .route("/static-pools/{pool_id}", get(get_static_pool_handler))
        .route("/static-pools/{pool_id}", put(update_static_pool_handler))
        .route(
            "/static-pools/{pool_id}",
            delete(delete_static_pool_handler),
        )
        .route(
            "/static-pools/{pool_id}/scale",
            post(scale_static_pool_handler),
        )
        .route(
            "/static-pools/{pool_id}/health",
            get(get_static_pool_health_handler),
        )
        .route(
            "/static-pools/{pool_id}/pre-warm",
            post(pre_warm_static_pool_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_create_static_pool() {
        let static_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let service = StaticPoolManagementService::new(static_pools, pool_configs);

        let request = CreateStaticPoolRequest {
            name: "test-static-pool".to_string(),
            provider_type: "docker".to_string(),
            min_size: 2,
            max_size: 10,
            initial_size: Some(5),
            pre_warm: true,
            health_check_interval: Duration::from_secs(30),
            metadata: Some(HashMap::from([("env".to_string(), "test".to_string())])),
            auto_recovery: true,
            reserved_capacity: Some(1),
        };

        let result = service.create_static_pool(request).await.unwrap();
        assert_eq!(result.config.name, "test-static-pool");
        assert_eq!(result.config.min_size, 2);
        assert_eq!(result.config.max_size, 10);
    }

    #[tokio::test]
    async fn test_list_static_pools() {
        let static_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let service = StaticPoolManagementService::new(static_pools, pool_configs);

        let request = CreateStaticPoolRequest {
            name: "pool1".to_string(),
            provider_type: "docker".to_string(),
            min_size: 1,
            max_size: 5,
            initial_size: None,
            pre_warm: false,
            health_check_interval: Duration::from_secs(30),
            metadata: None,
            auto_recovery: false,
            reserved_capacity: None,
        };

        service.create_static_pool(request.clone()).await.unwrap();
        service.create_static_pool(request).await.unwrap();

        let pools = service.list_static_pools().await.unwrap();
        assert_eq!(pools.len(), 2);
    }

    #[tokio::test]
    async fn test_scale_pool() {
        let static_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let service = StaticPoolManagementService::new(static_pools, pool_configs);

        let request = CreateStaticPoolRequest {
            name: "scale-test".to_string(),
            provider_type: "docker".to_string(),
            min_size: 2,
            max_size: 20,
            initial_size: None,
            pre_warm: false,
            health_check_interval: Duration::from_secs(30),
            metadata: None,
            auto_recovery: false,
            reserved_capacity: None,
        };

        let pool = service.create_static_pool(request).await.unwrap();
        let scaled = service.scale_pool(&pool.id, 10).await.unwrap();

        assert_eq!(scaled.config.current_size, 10);
    }

    #[tokio::test]
    async fn test_get_pool_health() {
        let static_pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_configs = Arc::new(RwLock::new(HashMap::new()));
        let service = StaticPoolManagementService::new(static_pools, pool_configs);

        let request = CreateStaticPoolRequest {
            name: "health-test".to_string(),
            provider_type: "docker".to_string(),
            min_size: 1,
            max_size: 5,
            initial_size: None,
            pre_warm: false,
            health_check_interval: Duration::from_secs(30),
            metadata: None,
            auto_recovery: false,
            reserved_capacity: None,
        };

        let pool = service.create_static_pool(request).await.unwrap();
        let health = service.get_pool_health(&pool.id).await.unwrap();

        assert_eq!(health.pool_id, pool.id);
    }
}
