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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use hodei_pipelines_ports::resource_pool::{ResourcePoolConfig, ResourcePoolStatus};

use crate::dtos::*;

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
        pool: CreatePoolRequestDto,
    ) -> Result<ResourcePoolResponseDto, String> {
        let pool_id = uuid::Uuid::new_v4().to_string();

        let pool_config = ResourcePoolConfig {
            pool_type: pool.pool_type.into(),
            name: pool.name,
            provider_name: pool.provider_name,
            min_size: pool.min_size,
            max_size: pool.max_size,
            default_resources: pool.default_resources.into(),
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

        Ok(ResourcePoolResponseDto {
            id: pool_id,
            config: pool_config.into(),
        })
    }

    /// Get a resource pool by ID
    pub async fn get_pool(&self, pool_id: &str) -> Result<ResourcePoolResponseDto, String> {
        let pools = self.pools.read().await;
        match pools.get(pool_id) {
            Some(config) => Ok(ResourcePoolResponseDto {
                id: pool_id.to_string(),
                config: config.clone().into(),
            }),
            None => Err("Pool not found".to_string()),
        }
    }

    /// List all resource pools
    pub async fn list_pools(&self) -> Result<Vec<ResourcePoolResponseDto>, String> {
        let pools = self.pools.read().await;
        let result = pools
            .iter()
            .map(|(id, config)| ResourcePoolResponseDto {
                id: id.clone(),
                config: config.clone().into(),
            })
            .collect();
        Ok(result)
    }

    /// Update a resource pool
    pub async fn update_pool(
        &self,
        pool_id: &str,
        updates: UpdatePoolRequestDto,
    ) -> Result<ResourcePoolResponseDto, String> {
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

                Ok(ResourcePoolResponseDto {
                    id: pool_id.to_string(),
                    config: config.clone().into(),
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
    pub async fn get_pool_status(&self, pool_id: &str) -> Result<ResourcePoolStatusDto, String> {
        let pool_statuses = self.pool_statuses.read().await;
        match pool_statuses.get(pool_id) {
            Some(status) => Ok(status.clone().into()),
            None => Err("Pool status not found".to_string()),
        }
    }
}

/// Get all resource pools
#[utoipa::path(
    get,
    path = "/api/v1/worker-pools",
    responses(
        (status = 200, description = "List of resource pools", body = Vec<ResourcePoolResponseDto>),
        (status = 500, description = "Internal server error")
    ),
    tag = "worker-pools"
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/worker-pools/{pool_id}",
    params(
        ("pool_id" = String, Path, description = "Pool ID")
    ),
    responses(
        (status = 200, description = "Resource pool details", body = ResourcePoolResponseDto),
        (status = 404, description = "Pool not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "worker-pools"
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/worker-pools",
    request_body = CreatePoolRequestDto,
    responses(
        (status = 200, description = "Resource pool created", body = ResourcePoolResponseDto),
        (status = 400, description = "Bad request"),
        (status = 409, description = "Pool already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "worker-pools"
)]
pub async fn create_pool_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Json(payload): Json<CreatePoolRequestDto>,
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
#[utoipa::path(
    put,
    path = "/api/v1/worker-pools/{pool_id}",
    params(
        ("pool_id" = String, Path, description = "Pool ID")
    ),
    request_body = UpdatePoolRequestDto,
    responses(
        (status = 200, description = "Resource pool updated", body = ResourcePoolResponseDto),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Pool not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "worker-pools"
)]
pub async fn update_pool_put_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<UpdatePoolRequestDto>,
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
#[utoipa::path(
    delete,
    path = "/api/v1/worker-pools/{pool_id}",
    params(
        ("pool_id" = String, Path, description = "Pool ID")
    ),
    responses(
        (status = 204, description = "Resource pool deleted"),
        (status = 404, description = "Pool not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "worker-pools"
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/worker-pools/{pool_id}/status",
    params(
        ("pool_id" = String, Path, description = "Pool ID")
    ),
    responses(
        (status = 200, description = "Resource pool status", body = ResourcePoolStatusDto),
        (status = 404, description = "Pool status not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "worker-pools"
)]
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
#[utoipa::path(
    patch,
    path = "/api/v1/worker-pools/{pool_id}",
    params(
        ("pool_id" = String, Path, description = "Pool ID")
    ),
    request_body = UpdatePoolRequestDto,
    responses(
        (status = 200, description = "Resource pool updated", body = ResourcePoolResponseDto),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Pool not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "worker-pools"
)]
pub async fn update_pool_patch_handler(
    State(app_state): State<ResourcePoolCrudAppState>,
    Path(pool_id): Path<String>,
    Json(payload): Json<UpdatePoolRequestDto>,
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
        .route("/", get(list_pools_handler))
        .route("/", post(create_pool_handler))
        .route("/{pool_id}", get(get_pool_handler))
        .route("/{pool_id}", put(update_pool_put_handler))
        .route("/{pool_id}", patch(update_pool_patch_handler))
        .route("/{pool_id}", delete(delete_pool_handler))
        .route("/{pool_id}/status", get(get_pool_status_handler))
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

        let request = CreatePoolRequestDto {
            pool_type: ResourcePoolTypeDto::Docker,
            name: "test-pool".to_string(),
            provider_name: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            default_resources: ResourceQuotaDto {
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

        let request = CreatePoolRequestDto {
            pool_type: ResourcePoolTypeDto::Kubernetes,
            name: "k8s-pool".to_string(),
            provider_name: "kubernetes".to_string(),
            min_size: 2,
            max_size: 20,
            default_resources: ResourceQuotaDto {
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

        let request1 = CreatePoolRequestDto {
            pool_type: ResourcePoolTypeDto::Static,
            name: "static-pool".to_string(),
            provider_name: "static".to_string(),
            min_size: 5,
            max_size: 5,
            default_resources: ResourceQuotaDto {
                cpu_m: 1000,
                memory_mb: 2048,
                gpu: None,
            },
            tags: None,
        };

        let request2 = CreatePoolRequestDto {
            pool_type: ResourcePoolTypeDto::Cloud,
            name: "cloud-pool".to_string(),
            provider_name: "aws".to_string(),
            min_size: 0,
            max_size: 100,
            default_resources: ResourceQuotaDto {
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

        let request = CreatePoolRequestDto {
            pool_type: ResourcePoolTypeDto::Docker,
            name: "original-pool".to_string(),
            provider_name: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            default_resources: ResourceQuotaDto {
                cpu_m: 2000,
                memory_mb: 4096,
                gpu: None,
            },
            tags: None,
        };

        let created = service.create_pool(request).await.unwrap();

        let updates = UpdatePoolRequestDto {
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

        let request = CreatePoolRequestDto {
            pool_type: ResourcePoolTypeDto::Kubernetes,
            name: "delete-me".to_string(),
            provider_name: "k8s".to_string(),
            min_size: 1,
            max_size: 5,
            default_resources: ResourceQuotaDto {
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

        let request = CreatePoolRequestDto {
            pool_type: ResourcePoolTypeDto::Docker,
            name: "status-test".to_string(),
            provider_name: "docker".to_string(),
            min_size: 1,
            max_size: 10,
            default_resources: ResourceQuotaDto {
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
