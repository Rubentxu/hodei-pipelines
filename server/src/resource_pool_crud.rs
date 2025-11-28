//! Resource Pool CRUD API Module
//!
//! Provides Create, Read, Update, and Delete operations for resource pools
//! in the system.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, patch, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use hodei_ports::resource_pool::{ResourcePoolConfig, ResourcePoolStatus, ResourcePoolType};

/// Application state for resource pool CRUD
#[derive(Clone)]
pub struct ResourcePoolCrudAppState {
    pub pools: Arc<RwLock<HashMap<String, ResourcePoolConfig>>>,
    pub pool_statuses: Arc<RwLock<HashMap<String, ResourcePoolStatus>>>,
}

/// Service for resource pool CRUD operations
#[derive(Clone)]
pub struct ResourcePoolCrudService {
    pools: Arc<RwLock<HashMap<String, ResourcePoolConfig>>>,
    pool_statuses: Arc<RwLock<HashMap<String, ResourcePoolStatus>>>,
}

impl ResourcePoolCrudService {
    pub fn new(
        pools: Arc<RwLock<HashMap<String, ResourcePoolConfig>>>,
        pool_statuses: Arc<RwLock<HashMap<String, ResourcePoolStatus>>>,
    ) -> Self {
        Self {
            pools,
            pool_statuses,
        }
    }

    /// Create a new resource pool
    pub async fn create_pool(
        &self,
        pool: CreatePoolRequest,
    ) -> Result<ResourcePoolResponse, String> {
        let pool_id = uuid::Uuid::new_v4().to_string();

        let pool_config = ResourcePoolConfig {
            pool_type: pool.pool_type,
            name: pool.name,
            provider_name: pool.provider_name,
            min_size: pool.min_size,
            max_size: pool.max_size,
            default_resources: pool.default_resources,
            tags: pool.tags.unwrap_or_default(),
        };

        // Check if pool name already exists
        let pools = self.pools.read().await;
        if pools.values().any(|p| p.name == pool_config.name) {
            return Err("Pool with this name already exists".to_string());
        }
        drop(pools);

        // Store the pool
        let pool_config_clone = pool_config.clone();
        let mut pools = self.pools.write().await;
        pools.insert(pool_id.clone(), pool_config_clone.clone());

        // Initialize status
        let status = ResourcePoolStatus {
            name: pool_config_clone.name.clone(),
            pool_type: pool_config_clone.pool_type,
            total_capacity: pool_config_clone.max_size,
            available_capacity: pool_config_clone.max_size,
            active_workers: 0,
            pending_requests: 0,
        };
        let mut pool_statuses = self.pool_statuses.write().await;
        pool_statuses.insert(pool_id.clone(), status);

        info!("Created resource pool: {}", pool_config_clone.name);

        Ok(ResourcePoolResponse {
            id: pool_id,
            config: pool_config,
        })
    }

    /// Get a resource pool by ID
    pub async fn get_pool(&self, pool_id: &str) -> Result<ResourcePoolResponse, String> {
        let pools = self.pools.read().await;
        match pools.get(pool_id) {
            Some(config) => Ok(ResourcePoolResponse {
                id: pool_id.to_string(),
                config: config.clone(),
            }),
            None => Err("Pool not found".to_string()),
        }
    }

    /// List all resource pools
    pub async fn list_pools(&self) -> Result<Vec<ResourcePoolResponse>, String> {
        let pools = self.pools.read().await;
        let result = pools
            .iter()
            .map(|(id, config)| ResourcePoolResponse {
                id: id.clone(),
                config: config.clone(),
            })
            .collect();
        Ok(result)
    }

    /// Update a resource pool
    pub async fn update_pool(
        &self,
        pool_id: &str,
        updates: UpdatePoolRequest,
    ) -> Result<ResourcePoolResponse, String> {
        let mut pools = self.pools.write().await;
        match pools.get_mut(pool_id) {
            Some(config) => {
                // Update fields if provided
                if let Some(name) = updates.name {
                    config.name = name;
                }
                if let Some(min_size) = updates.min_size {
                    config.min_size = min_size;
                }
                if let Some(max_size) = updates.max_size {
                    config.max_size = max_size;
                }
                if let Some(tags) = updates.tags {
                    config.tags = tags;
                }

                // Update status if max_size changed
                if let Some(max_size) = updates.max_size {
                    let mut pool_statuses = self.pool_statuses.write().await;
                    if let Some(status) = pool_statuses.get_mut(pool_id) {
                        status.total_capacity = max_size;
                        // Available capacity adjusts based on active workers
                        status.available_capacity = max_size.saturating_sub(status.active_workers);
                    }
                }

                info!("Updated resource pool: {}", config.name);

                Ok(ResourcePoolResponse {
                    id: pool_id.to_string(),
                    config: config.clone(),
                })
            }
            None => Err("Pool not found".to_string()),
        }
    }

    /// Delete a resource pool
    pub async fn delete_pool(&self, pool_id: &str) -> Result<(), String> {
        let mut pools = self.pools.write().await;
        match pools.remove(pool_id) {
            Some(config) => {
                // Remove status
                let mut pool_statuses = self.pool_statuses.write().await;
                pool_statuses.remove(pool_id);

                info!("Deleted resource pool: {}", config.name);
                Ok(())
            }
            None => Err("Pool not found".to_string()),
        }
    }

    /// Get pool status
    pub async fn get_pool_status(&self, pool_id: &str) -> Result<ResourcePoolStatus, String> {
        let pool_statuses = self.pool_statuses.read().await;
        match pool_statuses.get(pool_id) {
            Some(status) => Ok(status.clone()),
            None => Err("Pool status not found".to_string()),
        }
    }
}

/// Request to create a new resource pool
#[derive(Debug, Deserialize)]
pub struct CreatePoolRequest {
    pub pool_type: ResourcePoolType,
    pub name: String,
    pub provider_name: String,
    pub min_size: u32,
    pub max_size: u32,
    pub default_resources: hodei_core::ResourceQuota,
    pub tags: Option<HashMap<String, String>>,
}

/// Request to update a resource pool
#[derive(Debug, Deserialize, Default)]
pub struct UpdatePoolRequest {
    pub name: Option<String>,
    pub min_size: Option<u32>,
    pub max_size: Option<u32>,
    pub tags: Option<HashMap<String, String>>,
}

/// Response for resource pool operations
#[derive(Debug, Serialize)]
pub struct ResourcePoolResponse {
    pub id: String,
    pub config: ResourcePoolConfig,
}

/// Get all resource pools
pub async fn list_pools_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = ResourcePoolCrudService::new(app_state.pools, app_state.pool_statuses);
    match service.list_pools().await {
        Ok(pools) => Ok(Json(pools)),
        Err(e) => {
            error!("Failed to list pools: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Get a specific resource pool
pub async fn get_pool_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = ResourcePoolCrudService::new(app_state.pools, app_state.pool_statuses);
    match service.get_pool(&pool_id).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to get pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Create a new resource pool
pub async fn create_pool_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Json(payload): Json<CreatePoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate request
    if payload.min_size > payload.max_size {
        return Err((
            StatusCode::BAD_REQUEST,
            "min_size cannot be greater than max_size".to_string(),
        ));
    }

    let service = ResourcePoolCrudService::new(app_state.pools, app_state.pool_statuses);
    match service.create_pool(payload).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e.contains("already exists") {
                Err((StatusCode::CONFLICT, e))
            } else {
                error!("Failed to create pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Update a resource pool (PUT - full update)
pub async fn update_pool_put_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<UpdatePoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate min_size <= max_size if both are provided
    if let (Some(min_size), Some(max_size)) = (payload.min_size, payload.max_size) {
        if min_size > max_size {
            return Err((
                StatusCode::BAD_REQUEST,
                "min_size cannot be greater than max_size".to_string(),
            ));
        }
    }

    let service = ResourcePoolCrudService::new(app_state.pools, app_state.pool_statuses);
    match service.update_pool(&pool_id, payload).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to update pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Delete a resource pool
pub async fn delete_pool_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = ResourcePoolCrudService::new(app_state.pools, app_state.pool_statuses);
    match service.delete_pool(&pool_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            if e == "Pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to delete pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Get resource pool status
pub async fn get_pool_status_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Path(pool_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = ResourcePoolCrudService::new(app_state.pools, app_state.pool_statuses);
    match service.get_pool_status(&pool_id).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            if e == "Pool status not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to get pool status: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Update a resource pool (PATCH - partial update) - US-10.4
pub async fn update_pool_patch_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<UpdatePoolRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Same validation and update logic as PUT
    // The difference is semantic: PATCH is for partial updates
    if let (Some(min_size), Some(max_size)) = (payload.min_size, payload.max_size) {
        if min_size > max_size {
            return Err((
                StatusCode::BAD_REQUEST,
                "min_size cannot be greater than max_size".to_string(),
            ));
        }
    }

    let service = ResourcePoolCrudService::new(app_state.pools, app_state.pool_statuses);
    match service.update_pool(&pool_id, payload).await {
        Ok(pool) => Ok(Json(pool)),
        Err(e) => {
            if e == "Pool not found" {
                Err((StatusCode::NOT_FOUND, e))
            } else {
                error!("Failed to update pool: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        }
    }
}

/// Create resource pool CRUD router
/// Routes use "worker-pools" for consistency with frontend (US-10.2)
pub fn resource_pool_crud_routes() -> Router<ResourcePoolCrudAppState> {
    Router::new()
        .route("/worker-pools", get(list_pools_handler))
        .route("/worker-pools", post(create_pool_handler))
        .route("/worker-pools/{pool_id}", get(get_pool_handler))
        .route("/worker-pools/{pool_id}", put(update_pool_put_handler))
        .route("/worker-pools/{pool_id}", patch(update_pool_patch_handler))
        .route("/worker-pools/{pool_id}", delete(delete_pool_handler))
        .route(
            "/worker-pools/{pool_id}/status",
            get(get_pool_status_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_create_pool() {
        let pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_statuses = Arc::new(RwLock::new(HashMap::new()));
        let service = ResourcePoolCrudService::new(pools, pool_statuses);

        let request = CreatePoolRequest {
            pool_type: ResourcePoolType::Docker,
            name: "test-pool".to_string(),
            provider_name: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            default_resources: hodei_core::ResourceQuota {
                cpu_m: 2000,
                memory_mb: 4096,
                gpu: None,
            },
            tags: Some(HashMap::from([("env".to_string(), "test".to_string())])),
        };

        let result = service.create_pool(request).await.unwrap();
        assert_eq!(result.config.name, "test-pool");
        assert_eq!(result.config.min_size, 1);
        assert_eq!(result.config.max_size, 10);
    }

    #[tokio::test]
    async fn test_get_pool() {
        let pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_statuses = Arc::new(RwLock::new(HashMap::new()));
        let service = ResourcePoolCrudService::new(pools, pool_statuses);

        let request = CreatePoolRequest {
            pool_type: ResourcePoolType::Kubernetes,
            name: "k8s-pool".to_string(),
            provider_name: "kubernetes".to_string(),
            min_size: 2,
            max_size: 20,
            default_resources: hodei_core::ResourceQuota {
                cpu_m: 4000,
                memory_mb: 8192,
                gpu: None,
            },
            tags: None,
        };

        let created = service.create_pool(request).await.unwrap();
        let retrieved = service.get_pool(&created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.config.name, "k8s-pool");
    }

    #[tokio::test]
    async fn test_list_pools() {
        let pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_statuses = Arc::new(RwLock::new(HashMap::new()));
        let service = ResourcePoolCrudService::new(pools, pool_statuses);

        let request1 = CreatePoolRequest {
            pool_type: ResourcePoolType::Static,
            name: "static-pool".to_string(),
            provider_name: "static".to_string(),
            min_size: 5,
            max_size: 5,
            default_resources: hodei_core::ResourceQuota {
                cpu_m: 1000,
                memory_mb: 2048,
                gpu: None,
            },
            tags: None,
        };

        let request2 = CreatePoolRequest {
            pool_type: ResourcePoolType::Cloud,
            name: "cloud-pool".to_string(),
            provider_name: "aws".to_string(),
            min_size: 0,
            max_size: 100,
            default_resources: hodei_core::ResourceQuota {
                cpu_m: 8000,
                memory_mb: 16384,
                gpu: None,
            },
            tags: None,
        };

        service.create_pool(request1).await.unwrap();
        service.create_pool(request2).await.unwrap();

        let pools = service.list_pools().await.unwrap();
        assert_eq!(pools.len(), 2);
    }

    #[tokio::test]
    async fn test_update_pool() {
        let pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_statuses = Arc::new(RwLock::new(HashMap::new()));
        let service = ResourcePoolCrudService::new(pools, pool_statuses);

        let request = CreatePoolRequest {
            pool_type: ResourcePoolType::Docker,
            name: "original-pool".to_string(),
            provider_name: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            default_resources: hodei_core::ResourceQuota {
                cpu_m: 2000,
                memory_mb: 4096,
                gpu: None,
            },
            tags: None,
        };

        let created = service.create_pool(request).await.unwrap();

        let updates = UpdatePoolRequest {
            name: Some("updated-pool".to_string()),
            min_size: Some(2),
            max_size: Some(20),
            tags: Some(HashMap::from([("env".to_string(), "prod".to_string())])),
        };

        let updated = service.update_pool(&created.id, updates).await.unwrap();
        assert_eq!(updated.config.name, "updated-pool");
        assert_eq!(updated.config.min_size, 2);
        assert_eq!(updated.config.max_size, 20);
    }

    #[tokio::test]
    async fn test_delete_pool() {
        let pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_statuses = Arc::new(RwLock::new(HashMap::new()));
        let service = ResourcePoolCrudService::new(pools, pool_statuses);

        let request = CreatePoolRequest {
            pool_type: ResourcePoolType::Kubernetes,
            name: "delete-me".to_string(),
            provider_name: "k8s".to_string(),
            min_size: 1,
            max_size: 5,
            default_resources: hodei_core::ResourceQuota {
                cpu_m: 2000,
                memory_mb: 4096,
                gpu: None,
            },
            tags: None,
        };

        let created = service.create_pool(request).await.unwrap();
        service.delete_pool(&created.id).await.unwrap();

        // Verify pool is deleted
        let result = service.get_pool(&created.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_pool_status() {
        let pools = Arc::new(RwLock::new(HashMap::new()));
        let pool_statuses = Arc::new(RwLock::new(HashMap::new()));
        let service = ResourcePoolCrudService::new(pools, pool_statuses);

        let request = CreatePoolRequest {
            pool_type: ResourcePoolType::Docker,
            name: "status-test".to_string(),
            provider_name: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            default_resources: hodei_core::ResourceQuota {
                cpu_m: 2000,
                memory_mb: 4096,
                gpu: None,
            },
            tags: None,
        };

        let created = service.create_pool(request).await.unwrap();
        let status = service.get_pool_status(&created.id).await.unwrap();

        assert_eq!(status.name, "status-test");
        assert_eq!(status.total_capacity, 10);
        assert_eq!(status.available_capacity, 10);
        assert_eq!(status.active_workers, 0);
    }
}
