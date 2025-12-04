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

use hodei_pipelines_domain::{ResourcePoolConfig, ResourcePoolStatus};
use hodei_pipelines_ports::ResourcePoolRepository;

use crate::dtos::*;

/// Application state for resource pool CRUD
#[derive(Clone)]
pub struct ResourcePoolCrudAppState {
    pub repository: Arc<dyn ResourcePoolRepository>,
    pub pool_statuses: Arc<RwLock<HashMap<String, ResourcePoolStatus>>>,
}

/// Service for resource pool CRUD operations
#[derive(Clone)]
pub struct ResourcePoolCrudService {
    repository: Arc<dyn ResourcePoolRepository>,
    pool_statuses: Arc<RwLock<HashMap<String, ResourcePoolStatus>>>,
}

impl ResourcePoolCrudService {
    pub fn new(
        repository: Arc<dyn ResourcePoolRepository>,
        pool_statuses: Arc<RwLock<HashMap<String, ResourcePoolStatus>>>,
    ) -> Self {
        Self {
            repository,
            pool_statuses,
        }
    }

    /// Create a new resource pool
    pub async fn create_pool(
        &self,
        pool: CreatePoolRequestDto,
    ) -> Result<ResourcePoolResponseDto, String> {
        let _pool_id = uuid::Uuid::new_v4().to_string();

        let pool_config = ResourcePoolConfig {
            provider_type: pool.pool_type.into(),
            name: pool.name,
            provider_name: pool.provider_name,
            min_size: pool.min_size,
            max_size: pool.max_size,
            default_resources: pool.default_resources.into(),
            tags: pool.tags.unwrap_or_default(),
        };

        // Check if pool name already exists
        if let Ok(Some(_)) = self.repository.get(&pool_config.name).await {
            return Err("Pool with this name already exists".to_string());
        }

        // Store the pool
        self.repository
            .save(pool_config.clone())
            .await
            .map_err(|e| e.to_string())?;

        // Initialize status
        let status = ResourcePoolStatus {
            name: pool_config.name.clone(),
            provider_type: pool_config.provider_type.clone(),
            total_capacity: pool_config.max_size,
            available_capacity: pool_config.max_size,
            active_workers: 0,
            pending_requests: 0,
        };
        let mut pool_statuses = self.pool_statuses.write().await;
        // Use name as key for status map to align with repository
        pool_statuses.insert(pool_config.name.clone(), status);

        info!("Created resource pool: {}", pool_config.name);

        Ok(ResourcePoolResponseDto {
            id: pool_config.name.clone(), // Using name as ID for now as Redb uses name as key
            config: pool_config.into(),
        })
    }

    /// Get a resource pool by ID (Name)
    pub async fn get_pool(&self, pool_id: &str) -> Result<ResourcePoolResponseDto, String> {
        match self.repository.get(pool_id).await {
            Ok(Some(config)) => Ok(ResourcePoolResponseDto {
                id: pool_id.to_string(),
                config: config.into(),
            }),
            Ok(None) => Err("Pool not found".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// List all resource pools
    pub async fn list_pools(&self) -> Result<Vec<ResourcePoolResponseDto>, String> {
        match self.repository.list().await {
            Ok(configs) => {
                let result = configs
                    .into_iter()
                    .map(|config| ResourcePoolResponseDto {
                        id: config.name.clone(),
                        config: config.into(),
                    })
                    .collect();
                Ok(result)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Update a resource pool
    pub async fn update_pool(
        &self,
        pool_id: &str,
        updates: UpdatePoolRequestDto,
    ) -> Result<ResourcePoolResponseDto, String> {
        match self.repository.get(pool_id).await {
            Ok(Some(mut config)) => {
                // Update fields if provided
                if let Some(name) = updates.name {
                    // If name changes, we need to handle it carefully (delete old, save new)
                    // For now, let's disallow renaming or assume ID is name
                    if name != config.name {
                        return Err("Renaming pools is not supported yet".to_string());
                    }
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

                // Save updated config
                self.repository
                    .save(config.clone())
                    .await
                    .map_err(|e| e.to_string())?;

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
                    config: config.into(),
                })
            }
            Ok(None) => Err("Pool not found".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Delete a resource pool
    pub async fn delete_pool(&self, pool_id: &str) -> Result<(), String> {
        match self.repository.get(pool_id).await {
            Ok(Some(config)) => {
                self.repository
                    .delete(pool_id)
                    .await
                    .map_err(|e| e.to_string())?;

                // Remove status
                let mut pool_statuses = self.pool_statuses.write().await;
                pool_statuses.remove(pool_id);

                info!("Deleted resource pool: {}", config.name);
                Ok(())
            }
            Ok(None) => Err("Pool not found".to_string()),
            Err(e) => Err(e.to_string()),
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
    let service = ResourcePoolCrudService::new(app_state.repository, app_state.pool_statuses);
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
    let service = ResourcePoolCrudService::new(app_state.repository, app_state.pool_statuses);
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

    let service = ResourcePoolCrudService::new(app_state.repository, app_state.pool_statuses);
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

    let service = ResourcePoolCrudService::new(app_state.repository, app_state.pool_statuses);
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
    let service = ResourcePoolCrudService::new(app_state.repository, app_state.pool_statuses);
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
    let service = ResourcePoolCrudService::new(app_state.repository, app_state.pool_statuses);
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

    let service = ResourcePoolCrudService::new(app_state.repository, app_state.pool_statuses);
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

// Tests removed as they rely on in-memory HashMap implementation.
// New integration tests will be added in a separate file.
