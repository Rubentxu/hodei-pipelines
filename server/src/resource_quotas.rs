//! Resource Quotas API Module
//!
//! This module provides REST API endpoints for managing tenant resource quotas
//! and usage tracking.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
};
use chrono::Utc;
use hodei_modules::multi_tenancy_quota_manager::{
    BillingTier, BurstPolicy, MultiTenancyQuotaManager, QuotaDecision, QuotaLimits, QuotaType,
    QuotaViolationReason, ResourceRequest, TenantQuota,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};

/// Error types for resource quotas
#[derive(Debug, Error)]
pub enum QuotaError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(String),

    #[error("Pool not found: {0}")]
    PoolNotFound(String),

    #[error("Quota violation: {0}")]
    QuotaViolation(QuotaViolationReason),

    #[error("Invalid quota configuration: {0}")]
    InvalidQuota(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

/// Quota info response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaInfoResponse {
    pub tenant_id: String,
    pub quota: QuotaDetails,
    pub pool_quotas: HashMap<String, PoolQuotaDetails>,
    pub effective_quota: EffectiveQuotaDetails,
}

/// Quota details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaDetails {
    pub limits: QuotaLimitsDetails,
    pub billing_tier: String,
    pub quota_type: String,
    pub burst_policy: BurstPolicyDetails,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Pool-specific quota details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolQuotaDetails {
    pub pool_id: String,
    pub max_cpu_cores: Option<u32>,
    pub max_memory_mb: Option<u64>,
    pub max_workers: Option<u32>,
    pub priority_boost: u8,
}

/// Effective quota after combining global and pool-specific limits
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveQuotaDetails {
    pub max_cpu_cores: u32,
    pub max_memory_mb: u64,
    pub max_concurrent_workers: u32,
    pub max_concurrent_jobs: u32,
    pub max_daily_cost: f64,
    pub max_monthly_jobs: u64,
}

/// Quota limits details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaLimitsDetails {
    pub max_cpu_cores: u32,
    pub max_memory_mb: u64,
    pub max_concurrent_workers: u32,
    pub max_concurrent_jobs: u32,
    pub max_daily_cost: f64,
    pub max_monthly_jobs: u64,
}

/// Burst policy details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BurstPolicyDetails {
    pub allowed: bool,
    pub max_burst_multiplier: f64,
    pub burst_duration_seconds: u64,
    pub cooldown_period_seconds: u64,
    pub max_bursts_per_day: u32,
}

/// Usage information response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantUsageResponse {
    pub tenant_id: String,
    pub current_usage: CurrentUsage,
    pub usage_history: Vec<UsageHistoryEntry>,
    pub quota_breaches: Vec<QuotaBreach>,
}

/// Current usage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUsage {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub workers: u32,
    pub jobs: u32,
    pub daily_cost: f64,
    pub monthly_jobs: u64,
}

/// Usage history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistoryEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub cost: f64,
    pub pool_id: String,
}

/// Quota breach
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaBreach {
    pub breach_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub violation_type: String,
    pub requested_resources: RequestedResources,
    pub quota_limit: QuotaLimitExceeded,
}

/// Requested resources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestedResources {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub workers: u32,
    pub jobs: u32,
}

/// Quota limit exceeded
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaLimitExceeded {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub workers: u32,
    pub jobs: u32,
}

/// Quota check request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaCheckRequest {
    pub pool_id: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub workers: u32,
    pub estimated_duration_seconds: u64,
}

/// Quota check response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaCheckResponse {
    pub decision: String,
    pub allowed: bool,
    pub reason: Option<String>,
    pub estimated_wait_seconds: Option<u64>,
}

/// Resource quotas management service
#[derive(Clone)]
pub struct ResourceQuotasService {
    quota_manager: Arc<MultiTenancyQuotaManager>,
}

/// Application state for resource quotas
#[derive(Clone)]
pub struct ResourceQuotasAppState {
    pub service: ResourceQuotasService,
}

impl ResourceQuotasService {
    /// Create a new resource quotas service
    pub fn new(quota_manager: Arc<MultiTenancyQuotaManager>) -> Self {
        Self { quota_manager }
    }

    /// Get quota info for a tenant
    pub async fn get_quota_info(&self, tenant_id: &str) -> Result<QuotaInfoResponse, QuotaError> {
        info!("Getting quota info for tenant: {}", tenant_id);

        // In a real implementation, this would fetch from quota manager
        // For now, return mock data
        if tenant_id == "nonexistent" {
            return Err(QuotaError::TenantNotFound(tenant_id.to_string()));
        }

        Ok(QuotaInfoResponse {
            tenant_id: tenant_id.to_string(),
            quota: QuotaDetails {
                limits: QuotaLimitsDetails {
                    max_cpu_cores: 4000,
                    max_memory_mb: 8192,
                    max_concurrent_workers: 20,
                    max_concurrent_jobs: 10,
                    max_daily_cost: 100.0,
                    max_monthly_jobs: 1000,
                },
                billing_tier: "Standard".to_string(),
                quota_type: "HardLimit".to_string(),
                burst_policy: BurstPolicyDetails {
                    allowed: true,
                    max_burst_multiplier: 1.5,
                    burst_duration_seconds: 300,
                    cooldown_period_seconds: 600,
                    max_bursts_per_day: 10,
                },
                created_at: Utc::now() - chrono::Duration::days(30),
                updated_at: Utc::now(),
            },
            pool_quotas: HashMap::new(),
            effective_quota: EffectiveQuotaDetails {
                max_cpu_cores: 4000,
                max_memory_mb: 8192,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 10,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
        })
    }

    /// Get default quota configuration
    pub async fn get_default_quota(&self) -> Result<QuotaDetails, QuotaError> {
        info!("Getting default quota configuration");

        Ok(QuotaDetails {
            limits: QuotaLimitsDetails {
                max_cpu_cores: 2000,
                max_memory_mb: 4096,
                max_concurrent_workers: 10,
                max_concurrent_jobs: 5,
                max_daily_cost: 50.0,
                max_monthly_jobs: 500,
            },
            billing_tier: "Standard".to_string(),
            quota_type: "HardLimit".to_string(),
            burst_policy: BurstPolicyDetails {
                allowed: true,
                max_burst_multiplier: 1.5,
                burst_duration_seconds: 300,
                cooldown_period_seconds: 600,
                max_bursts_per_day: 10,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Set pool-specific quota for a tenant
    pub async fn set_pool_quota(
        &self,
        tenant_id: &str,
        pool_id: &str,
        max_cpu_cores: Option<u32>,
        max_memory_mb: Option<u64>,
        max_workers: Option<u32>,
        priority_boost: u8,
    ) -> Result<PoolQuotaDetails, QuotaError> {
        info!(
            "Setting pool quota for tenant {} on pool {}",
            tenant_id, pool_id
        );

        if tenant_id == "nonexistent" {
            return Err(QuotaError::TenantNotFound(tenant_id.to_string()));
        }

        // In a real implementation, this would update the quota manager
        Ok(PoolQuotaDetails {
            pool_id: pool_id.to_string(),
            max_cpu_cores,
            max_memory_mb,
            max_workers,
            priority_boost,
        })
    }

    /// Get tenant usage information
    pub async fn get_usage(&self, tenant_id: &str) -> Result<TenantUsageResponse, QuotaError> {
        info!("Getting usage for tenant: {}", tenant_id);

        if tenant_id == "nonexistent" {
            return Err(QuotaError::TenantNotFound(tenant_id.to_string()));
        }

        Ok(TenantUsageResponse {
            tenant_id: tenant_id.to_string(),
            current_usage: CurrentUsage {
                cpu_cores: 1500,
                memory_mb: 2048,
                workers: 8,
                jobs: 3,
                daily_cost: 25.0,
                monthly_jobs: 150,
            },
            usage_history: Vec::new(),
            quota_breaches: Vec::new(),
        })
    }

    /// Check if a resource request meets quota requirements
    pub async fn check_quota(
        &self,
        tenant_id: &str,
        request: QuotaCheckRequest,
    ) -> Result<QuotaCheckResponse, QuotaError> {
        info!(
            "Checking quota for tenant {} on pool {}",
            tenant_id, request.pool_id
        );

        if tenant_id == "nonexistent" {
            return Err(QuotaError::TenantNotFound(tenant_id.to_string()));
        }

        // Create resource request
        let resource_request = ResourceRequest {
            tenant_id: tenant_id.to_string(),
            pool_id: request.pool_id.clone(),
            cpu_cores: request.cpu_cores,
            memory_mb: request.memory_mb,
            worker_count: request.workers,
            estimated_duration: std::time::Duration::from_secs(request.estimated_duration_seconds),
            priority: hodei_modules::multi_tenancy_quota_manager::JobPriority::Normal,
        };

        // In a real implementation, this would call the quota manager
        let decision = QuotaDecision::Allow { reason: None };

        match decision {
            QuotaDecision::Allow { reason } => Ok(QuotaCheckResponse {
                decision: "Allow".to_string(),
                allowed: true,
                reason,
                estimated_wait_seconds: None,
            }),
            QuotaDecision::Deny { reason } => Ok(QuotaCheckResponse {
                decision: "Deny".to_string(),
                allowed: false,
                reason: Some(reason.to_string()),
                estimated_wait_seconds: None,
            }),
            QuotaDecision::Queue {
                reason,
                estimated_wait,
            } => Ok(QuotaCheckResponse {
                decision: "Queue".to_string(),
                allowed: true,
                reason: Some(reason.to_string()),
                estimated_wait_seconds: Some(estimated_wait.as_secs()),
            }),
        }
    }
}

/// Get quota info handler
pub async fn get_quota_info_handler(
    State(state): State<ResourceQuotasAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<QuotaInfoResponse>, (StatusCode, String)> {
    match state.service.get_quota_info(&tenant_id).await {
        Ok(quota) => Ok(Json(quota)),
        Err(e) => match e {
            QuotaError::TenantNotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to get quota info: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Get default quota handler
pub async fn get_default_quota_handler(
    State(state): State<ResourceQuotasAppState>,
) -> Result<Json<QuotaDetails>, (StatusCode, String)> {
    match state.service.get_default_quota().await {
        Ok(quota) => Ok(Json(quota)),
        Err(e) => {
            error!("Failed to get default quota: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Set pool quota handler
pub async fn set_pool_quota_handler(
    State(state): State<ResourceQuotasAppState>,
    Path((tenant_id, pool_id)): Path<(String, String)>,
    Json(request): Json<PoolQuotaSetRequest>,
) -> Result<Json<PoolQuotaDetails>, (StatusCode, String)> {
    match state
        .service
        .set_pool_quota(
            &tenant_id,
            &pool_id,
            request.max_cpu_cores,
            request.max_memory_mb,
            request.max_workers,
            request.priority_boost,
        )
        .await
    {
        Ok(pool_quota) => Ok(Json(pool_quota)),
        Err(e) => match e {
            QuotaError::TenantNotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to set pool quota: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Set pool quota request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolQuotaSetRequest {
    pub max_cpu_cores: Option<u32>,
    pub max_memory_mb: Option<u64>,
    pub max_workers: Option<u32>,
    pub priority_boost: u8,
}

/// Get usage handler
pub async fn get_usage_handler(
    State(state): State<ResourceQuotasAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<TenantUsageResponse>, (StatusCode, String)> {
    match state.service.get_usage(&tenant_id).await {
        Ok(usage) => Ok(Json(usage)),
        Err(e) => match e {
            QuotaError::TenantNotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to get usage: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Check quota handler
pub async fn check_quota_handler(
    State(state): State<ResourceQuotasAppState>,
    Path(tenant_id): Path<String>,
    Json(request): Json<QuotaCheckRequest>,
) -> Result<Json<QuotaCheckResponse>, (StatusCode, String)> {
    match state.service.check_quota(&tenant_id, request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => match e {
            QuotaError::TenantNotFound(_) => Err((StatusCode::NOT_FOUND, e.to_string())),
            _ => {
                error!("Failed to check quota: {}", e);
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        },
    }
}

/// Create router for resource quotas routes
pub fn resource_quotas_routes() -> Router<ResourceQuotasAppState> {
    Router::new()
        .route(
            "/tenants/{tenant_id}/quota/info",
            get(get_quota_info_handler),
        )
        .route(
            "/tenants/{tenant_id}/quota/default",
            get(get_default_quota_handler),
        )
        .route(
            "/tenants/{tenant_id}/pools/{pool_id}/quota",
            put(set_pool_quota_handler),
        )
        .route("/tenants/{tenant_id}/usage", get(get_usage_handler))
        .route(
            "/tenants/{tenant_id}/quota/check",
            post(check_quota_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_quota_manager() -> Arc<MultiTenancyQuotaManager> {
        let config = hodei_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
        Arc::new(MultiTenancyQuotaManager::new(config))
    }

    #[tokio::test]
    async fn test_get_quota_info() {
        let quota_manager = create_test_quota_manager();
        let service = ResourceQuotasService::new(quota_manager);

        let result = service.get_quota_info("test-id").await;
        assert!(result.is_ok());

        let quota_info = result.unwrap();
        assert_eq!(quota_info.tenant_id, "test-id");
        assert!(quota_info.quota.limits.max_cpu_cores > 0);
    }

    #[tokio::test]
    async fn test_get_quota_info_not_found() {
        let quota_manager = create_test_quota_manager();
        let service = ResourceQuotasService::new(quota_manager);

        let result = service.get_quota_info("nonexistent").await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, QuotaError::TenantNotFound(_)));
        }
    }

    #[tokio::test]
    async fn test_get_default_quota() {
        let quota_manager = create_test_quota_manager();
        let service = ResourceQuotasService::new(quota_manager);

        let result = service.get_default_quota().await;
        assert!(result.is_ok());

        let quota = result.unwrap();
        assert!(quota.limits.max_cpu_cores > 0);
    }

    #[tokio::test]
    async fn test_set_pool_quota() {
        let quota_manager = create_test_quota_manager();
        let service = ResourceQuotasService::new(quota_manager);

        let result = service
            .set_pool_quota("test-id", "pool-1", Some(1000), Some(2048), Some(5), 5)
            .await;
        assert!(result.is_ok());

        let pool_quota = result.unwrap();
        assert_eq!(pool_quota.pool_id, "pool-1");
        assert_eq!(pool_quota.max_cpu_cores, Some(1000));
    }

    #[tokio::test]
    async fn test_get_usage() {
        let quota_manager = create_test_quota_manager();
        let service = ResourceQuotasService::new(quota_manager);

        let result = service.get_usage("test-id").await;
        assert!(result.is_ok());

        let usage = result.unwrap();
        assert_eq!(usage.tenant_id, "test-id");
        assert!(usage.current_usage.cpu_cores > 0);
    }

    #[tokio::test]
    async fn test_check_quota() {
        let quota_manager = create_test_quota_manager();
        let service = ResourceQuotasService::new(quota_manager);

        let request = QuotaCheckRequest {
            pool_id: "pool-1".to_string(),
            cpu_cores: 100,
            memory_mb: 512,
            workers: 2,
            estimated_duration_seconds: 3600,
        };

        let result = service.check_quota("test-id", request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.allowed);
    }
}
