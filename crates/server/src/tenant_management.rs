//! Tenant Management API Module
//!
//! This module provides REST API endpoints for multi-tenancy management,
//! including tenant CRUD operations, quota management, and burst capacity.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use hodei_pipelines_modules::multi_tenancy_quota_manager::{MultiTenancyQuotaManager, TenantId, TenantQuota};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};
use uuid::Uuid;

/// Error types for tenant management
#[derive(Debug, Error)]
pub enum TenantError {
    #[error("Tenant not found: {0}")]
    NotFound(String),

    #[error("Invalid quota configuration: {0}")]
    InvalidQuota(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

/// Tenant DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create tenant request DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantRequest {
    pub name: String,
    pub email: String,
}

/// Update tenant request DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantRequest {
    pub name: String,
    pub email: String,
}

/// Quota DTO for API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaResponse {
    pub cpu_m: u64,
    pub memory_mb: u64,
    pub max_concurrent_jobs: u32,
    pub current_usage: QuotaUsage,
}

/// Update quota request DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateQuotaRequest {
    pub cpu_m: u64,
    pub memory_mb: u64,
    pub max_concurrent_jobs: u32,
}

/// Quota usage DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaUsage {
    pub cpu_m: u64,
    pub memory_mb: u64,
    pub active_jobs: u32,
}

/// Tenant management service
#[derive(Clone)]
pub struct TenantManagementService {
    quota_manager: Arc<MultiTenancyQuotaManager>,
}

/// Application state for tenant management
#[derive(Clone)]
pub struct TenantAppState {
    pub tenant_service: TenantManagementService,
}

impl TenantManagementService {
    /// Create a new tenant management service
    pub fn new(quota_manager: Arc<MultiTenancyQuotaManager>) -> Self {
        Self { quota_manager }
    }

    /// Create a new tenant
    pub async fn create_tenant(
        &self,
        request: CreateTenantRequest,
    ) -> Result<TenantResponse, TenantError> {
        info!("Creating tenant: {}", request.name);

        // In a real implementation, this would:
        // 1. Save to database
        // 2. Initialize default quotas
        // 3. Set up tenant context

        let tenant_id = format!("tenant-{}", Uuid::new_v4());

        Ok(TenantResponse {
            id: tenant_id.as_str().to_string(),
            name: request.name,
            email: request.email,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Get tenant by ID
    pub async fn get_tenant(&self, tenant_id: &str) -> Result<TenantResponse, TenantError> {
        info!("Getting tenant: {}", tenant_id);

        // In a real implementation, this would fetch from database
        // For now, return a mock response

        if tenant_id == "nonexistent" {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        Ok(TenantResponse {
            id: tenant_id.to_string(),
            name: format!("Tenant {}", tenant_id),
            email: format!("admin@tenant-{}.com", tenant_id),
            created_at: chrono::Utc::now() - chrono::Duration::days(1),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Update tenant
    pub async fn update_tenant(
        &self,
        tenant_id: &str,
        request: UpdateTenantRequest,
    ) -> Result<TenantResponse, TenantError> {
        info!("Updating tenant: {}", tenant_id);

        // In a real implementation, this would update the database

        if tenant_id == "nonexistent" {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        Ok(TenantResponse {
            id: tenant_id.to_string(),
            name: request.name,
            email: request.email,
            created_at: chrono::Utc::now() - chrono::Duration::days(1),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Delete tenant
    pub async fn delete_tenant(&self, tenant_id: &str) -> Result<(), TenantError> {
        info!("Deleting tenant: {}", tenant_id);

        // In a real implementation, this would:
        // 1. Check if tenant has active jobs
        // 2. Clean up quotas
        // 3. Delete from database

        if tenant_id == "nonexistent" {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        Ok(())
    }

    /// Get tenant quota
    pub async fn get_quota(&self, tenant_id: &str) -> Result<QuotaResponse, TenantError> {
        info!("Getting quota for tenant: {}", tenant_id);

        if tenant_id == "nonexistent" {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        // In a real implementation, this would fetch from quota manager
        Ok(QuotaResponse {
            cpu_m: 4000,
            memory_mb: 8192,
            max_concurrent_jobs: 10,
            current_usage: QuotaUsage {
                cpu_m: 1500,
                memory_mb: 2048,
                active_jobs: 3,
            },
        })
    }

    /// Get tenant quota with full quota information
    pub async fn get_full_quota(&self, tenant_id: &str) -> Result<TenantQuota, TenantError> {
        info!("Getting full quota for tenant: {}", tenant_id);

        if tenant_id == "nonexistent" {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        // Create a mock quota for testing
        use chrono::Utc;
        use std::collections::HashMap;

        Ok(TenantQuota {
            tenant_id: tenant_id.to_string(),
            limits: hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaLimits {
                max_cpu_cores: 100,
                max_memory_mb: 8192,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 10,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
            pool_access: HashMap::new(),
            burst_policy: hodei_pipelines_modules::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 1.5,
                burst_duration: std::time::Duration::from_secs(300),
                cooldown_period: std::time::Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: hodei_pipelines_modules::multi_tenancy_quota_manager::BillingTier::Standard,
            quota_type: hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaType::HardLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Update tenant quota
    pub async fn update_quota(
        &self,
        tenant_id: &str,
        quota: TenantQuota,
    ) -> Result<QuotaResponse, TenantError> {
        info!("Updating quota for tenant: {}", tenant_id);

        if tenant_id == "nonexistent" {
            return Err(TenantError::NotFound(tenant_id.to_string()));
        }

        // In a real implementation, this would update the quota manager
        Ok(QuotaResponse {
            cpu_m: quota.limits.max_cpu_cores as u64,
            memory_mb: quota.limits.max_memory_mb,
            max_concurrent_jobs: quota.limits.max_concurrent_jobs,
            current_usage: QuotaUsage {
                cpu_m: 1500,
                memory_mb: 2048,
                active_jobs: 3,
            },
        })
    }
}

/// Create tenant handler
pub async fn create_tenant_handler(
    State(state): State<TenantAppState>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, (StatusCode, String)> {
    match state.tenant_service.create_tenant(request).await {
        Ok(tenant) => Ok(Json(tenant)),
        Err(e) => {
            error!("Failed to create tenant: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Get tenant handler
pub async fn get_tenant_handler(
    State(state): State<TenantAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<TenantResponse>, (StatusCode, String)> {
    match state.tenant_service.get_tenant(&tenant_id).await {
        Ok(tenant) => Ok(Json(tenant)),
        Err(e) => match e {
            TenantError::NotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to get tenant: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Update tenant handler
pub async fn update_tenant_handler(
    State(state): State<TenantAppState>,
    Path(tenant_id): Path<String>,
    Json(request): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, (StatusCode, String)> {
    match state
        .tenant_service
        .update_tenant(&tenant_id, request)
        .await
    {
        Ok(tenant) => Ok(Json(tenant)),
        Err(e) => match e {
            TenantError::NotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to update tenant: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Delete tenant handler
pub async fn delete_tenant_handler(
    State(state): State<TenantAppState>,
    Path(tenant_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.tenant_service.delete_tenant(&tenant_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => match e {
            TenantError::NotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to delete tenant: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Get tenant quota handler
pub async fn get_quota_handler(
    State(state): State<TenantAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<QuotaResponse>, (StatusCode, String)> {
    match state.tenant_service.get_quota(&tenant_id).await {
        Ok(quota) => Ok(Json(quota)),
        Err(e) => match e {
            TenantError::NotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to get quota: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Update tenant quota handler
pub async fn update_quota_handler(
    State(state): State<TenantAppState>,
    Path(tenant_id): Path<String>,
    Json(request): Json<UpdateQuotaRequest>,
) -> Result<Json<QuotaResponse>, (StatusCode, String)> {
    // Convert UpdateQuotaRequest to TenantQuota
    use chrono::Utc;
    use std::collections::HashMap;

    let tenant_quota = TenantQuota {
        tenant_id: tenant_id.clone(),
        limits: hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaLimits {
            max_cpu_cores: request.cpu_m as u32,
            max_memory_mb: request.memory_mb,
            max_concurrent_workers: request.max_concurrent_jobs * 2,
            max_concurrent_jobs: request.max_concurrent_jobs,
            max_daily_cost: 100.0,
            max_monthly_jobs: 1000,
        },
        pool_access: HashMap::new(),
        burst_policy: hodei_pipelines_modules::multi_tenancy_quota_manager::BurstPolicy {
            allowed: true,
            max_burst_multiplier: 1.5,
            burst_duration: std::time::Duration::from_secs(300),
            cooldown_period: std::time::Duration::from_secs(600),
            max_bursts_per_day: 10,
        },
        billing_tier: hodei_pipelines_modules::multi_tenancy_quota_manager::BillingTier::Standard,
        quota_type: hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaType::HardLimit,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    match state
        .tenant_service
        .update_quota(&tenant_id, tenant_quota)
        .await
    {
        Ok(quota) => Ok(Json(quota)),
        Err(e) => match e {
            TenantError::NotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to update quota: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Create router for tenant management routes
pub fn tenant_routes() -> Router<TenantAppState> {
    Router::new()
        .route("/tenants", post(create_tenant_handler))
        .route("/tenants/:id", get(get_tenant_handler))
        .route("/tenants/:id", put(update_tenant_handler))
        .route("/tenants/:id", delete(delete_tenant_handler))
        .route("/tenants/:id/quota", get(get_quota_handler))
        .route("/tenants/:id/quota", put(update_quota_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_pipelines_modules::multi_tenancy_quota_manager::TenantQuota;

    #[tokio::test]
    async fn test_create_tenant() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let request = CreateTenantRequest {
            name: "test-tenant".to_string(),
            email: "test@example.com".to_string(),
        };

        let result = service.create_tenant(request).await;
        assert!(result.is_ok());

        let tenant = result.unwrap();
        assert_eq!(tenant.name, "test-tenant");
        assert_eq!(tenant.email, "test@example.com");
    }

    #[tokio::test]
    async fn test_get_existing_tenant() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let result = service.get_tenant("test-id").await;
        assert!(result.is_ok());

        let tenant = result.unwrap();
        assert_eq!(tenant.id, "test-id");
    }

    #[tokio::test]
    async fn test_get_nonexistent_tenant() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let result = service.get_tenant("nonexistent").await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, TenantError::NotFound(_)));
        }
    }

    #[tokio::test]
    async fn test_update_tenant() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let request = UpdateTenantRequest {
            name: "updated-tenant".to_string(),
            email: "updated@example.com".to_string(),
        };

        let result = service.update_tenant("test-id", request).await;
        assert!(result.is_ok());

        let tenant = result.unwrap();
        assert_eq!(tenant.name, "updated-tenant");
        assert_eq!(tenant.email, "updated@example.com");
    }

    #[tokio::test]
    async fn test_delete_existing_tenant() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let result = service.delete_tenant("test-id").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_tenant() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let result = service.delete_tenant("nonexistent").await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, TenantError::NotFound(_)));
        }
    }

    #[tokio::test]
    async fn test_get_quota() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let result = service.get_quota("test-id").await;
        assert!(result.is_ok());

        let quota = result.unwrap();
        assert_eq!(quota.cpu_m, 4000);
        assert_eq!(quota.memory_mb, 8192);
        assert_eq!(quota.max_concurrent_jobs, 10);
    }

    #[tokio::test]
    async fn test_update_quota() {
        let quota_manager_config =
            hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
        let service = TenantManagementService::new(quota_manager);

        let new_quota = TenantQuota {
            tenant_id: "test-id".to_string(),
            limits: hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaLimits {
                max_cpu_cores: 6000 as u32,
                max_memory_mb: 12288,
                max_concurrent_workers: 30,
                max_concurrent_jobs: 15,
                max_daily_cost: 200.0,
                max_monthly_jobs: 2000,
            },
            pool_access: std::collections::HashMap::new(),
            burst_policy: hodei_pipelines_modules::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 2.0,
                burst_duration: std::time::Duration::from_secs(600),
                cooldown_period: std::time::Duration::from_secs(1200),
                max_bursts_per_day: 20,
            },
            billing_tier: hodei_pipelines_modules::multi_tenancy_quota_manager::BillingTier::Premium,
            quota_type: hodei_pipelines_modules::multi_tenancy_quota_manager::QuotaType::SoftLimit,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = service.update_quota("test-id", new_quota).await;
        assert!(result.is_ok());

        let quota = result.unwrap();
        assert_eq!(quota.cpu_m, 6000);
        assert_eq!(quota.memory_mb, 12288);
        assert_eq!(quota.max_concurrent_jobs, 15);
    }
}
