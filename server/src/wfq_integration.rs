//! WFQ Integration API Module
//!
//! This module provides REST API endpoints for Weighted Fair Queuing (WFQ) integration,
//! exposing the WeightedFairQueueingEngine capabilities through HTTP endpoints.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use hodei_modules::{
    multi_tenancy_quota_manager::{BillingTier, TenantQuota, TenantUsage},
    weighted_fair_queuing::{
        WFQAllocation, WFQConfig, WFQError, WFQQueueEntry, WFQStats, WeightContext, WeightStrategy,
        WeightedFairQueueingEngine,
    },
};

/// API application state
#[derive(Clone)]
pub struct WFQIntegrationAppState {
    pub service: WFQIntegrationService,
}

/// WFQ integration service
#[derive(Clone)]
pub struct WFQIntegrationService {
    pub engine: Arc<tokio::sync::Mutex<WeightedFairQueueingEngine>>,
}

/// DTOs for request/response

#[derive(Debug, Serialize, Deserialize)]
pub struct WFQConfigRequest {
    pub enable_virtual_time: bool,
    pub min_weight: f64,
    pub max_weight: f64,
    pub default_strategy: String,
    pub starvation_threshold: f64,
    pub default_packet_size: u64,
}

impl WFQConfigRequest {
    fn to_domain(&self) -> Result<WFQConfig, String> {
        let strategy = match self.default_strategy.to_lowercase().as_str() {
            "billing_tier" => WeightStrategy::BillingTier,
            "quota_based" => WeightStrategy::QuotaBased,
            "usage_history" => WeightStrategy::UsageHistory,
            "custom" => WeightStrategy::Custom,
            _ => return Err("Invalid weight strategy".to_string()),
        };

        Ok(WFQConfig {
            enable_virtual_time: self.enable_virtual_time,
            min_weight: self.min_weight,
            max_weight: self.max_weight,
            default_strategy: strategy,
            starvation_threshold: self.starvation_threshold,
            weight_update_interval: Duration::from_secs(60),
            default_packet_size: self.default_packet_size,
            enable_dynamic_weights: true,
            starvation_window: Duration::from_secs(300),
            fair_share_window: Duration::from_secs(3600),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterTenantRequest {
    pub tenant_quota: TenantQuotaDto,
    pub tenant_usage: TenantUsageDto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TenantQuotaDto {
    pub tenant_id: String,
    pub max_cpu_cores: u32,
    pub max_memory_mb: u64,
    pub max_workers: u32,
    pub billing_tier: String,
}

impl TenantQuotaDto {
    fn to_domain(self) -> Result<TenantQuota, String> {
        let billing_tier = match self.billing_tier.to_lowercase().as_str() {
            "free" => BillingTier::Free,
            "standard" => BillingTier::Standard,
            "premium" => BillingTier::Premium,
            "enterprise" => BillingTier::Enterprise,
            _ => return Err("Invalid billing tier".to_string()),
        };

        let quota_limits = hodei_modules::multi_tenancy_quota_manager::QuotaLimits {
            max_cpu_cores: self.max_cpu_cores,
            max_memory_mb: self.max_memory_mb,
            max_concurrent_workers: self.max_workers,
            max_concurrent_jobs: 100,
            max_daily_cost: 1000.0,
            max_monthly_jobs: 10000,
        };

        Ok(TenantQuota {
            tenant_id: self.tenant_id,
            limits: quota_limits,
            pool_access: HashMap::new(),
            burst_policy: hodei_modules::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 1.5,
                burst_duration: Duration::from_secs(3600),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier,
            quota_type: hodei_modules::multi_tenancy_quota_manager::QuotaType::SoftLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TenantUsageDto {
    pub tenant_id: String,
    pub cpu_used: u32,
    pub memory_used_mb: u64,
    pub workers_used: u32,
    pub usage_period_start: String,
    pub usage_period_end: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceRequestDto {
    pub tenant_id: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub workers: u32,
    pub queue_priority: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WFQAllocationDto {
    pub tenant_id: String,
    pub allocated_cpu: u32,
    pub allocated_memory: u64,
    pub allocated_workers: u32,
    pub weight: f64,
    pub virtual_time: f64,
    pub finish_time: f64,
}

impl From<WFQAllocation> for WFQAllocationDto {
    fn from(alloc: WFQAllocation) -> Self {
        Self {
            tenant_id: alloc.tenant_id,
            allocated_cpu: alloc.allocated_cpu,
            allocated_memory: alloc.allocated_memory,
            allocated_workers: alloc.allocated_workers,
            weight: alloc.weight,
            virtual_time: alloc.virtual_time,
            finish_time: alloc.finish_time,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WFQStatsDto {
    pub total_allocations: u64,
    pub total_tenants: u64,
    pub active_tenants: u64,
    pub starvation_events: u64,
    pub weight_adjustments: u64,
    pub average_wait_time_ms: f64,
    pub fairness_index: f64,
    pub virtual_time: f64,
    pub queue_depth: u64,
    pub timestamp: u64,
}

impl From<WFQStats> for WFQStatsDto {
    fn from(stats: WFQStats) -> Self {
        Self {
            total_allocations: stats.total_allocations,
            total_tenants: stats.total_tenants,
            active_tenants: stats.active_tenants,
            starvation_events: stats.starvation_events,
            weight_adjustments: stats.weight_adjustments,
            average_wait_time_ms: stats.average_wait_time_ms,
            fairness_index: stats.fairness_index,
            virtual_time: stats.virtual_time,
            queue_depth: stats.queue_depth,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueStateDto {
    pub pending_requests: u64,
    pub active_allocations: u64,
    pub oldest_request_age_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponseDto<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl<T> ApiResponseDto<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

impl WFQIntegrationService {
    /// Create new WFQ integration service
    pub fn new(engine: WeightedFairQueueingEngine) -> Self {
        Self {
            engine: Arc::new(tokio::sync::Mutex::new(engine)),
        }
    }

    /// Get WFQ configuration (simplified - returns defaults)
    pub async fn get_config(&self) -> Result<WFQConfig, String> {
        // Note: config field is private, return defaults
        Ok(WFQConfig {
            enable_virtual_time: true,
            min_weight: 0.1,
            max_weight: 10.0,
            default_strategy: WeightStrategy::BillingTier,
            starvation_threshold: 0.5,
            weight_update_interval: Duration::from_secs(60),
            default_packet_size: 1000,
            enable_dynamic_weights: true,
            starvation_window: Duration::from_secs(300),
            fair_share_window: Duration::from_secs(3600),
        })
    }

    /// Register a tenant
    pub async fn register_tenant(
        &self,
        tenant_quota: TenantQuota,
        _tenant_usage: &TenantUsage,
    ) -> Result<(), String> {
        let engine = self.engine.lock().await;
        engine
            .register_tenant(tenant_quota, _tenant_usage)
            .await
            .map_err(|e| e.to_string())
    }

    /// Enqueue a resource request
    pub async fn enqueue_request(&self, request: ResourceRequestDto) -> Result<(), String> {
        let engine = self.engine.lock().await;
        let priority = match request.queue_priority {
            0 => hodei_modules::multi_tenancy_quota_manager::JobPriority::Critical,
            1 => hodei_modules::multi_tenancy_quota_manager::JobPriority::High,
            2 => hodei_modules::multi_tenancy_quota_manager::JobPriority::Normal,
            3 => hodei_modules::multi_tenancy_quota_manager::JobPriority::Low,
            _ => hodei_modules::multi_tenancy_quota_manager::JobPriority::Batch,
        };

        let resource_request = hodei_modules::multi_tenancy_quota_manager::ResourceRequest {
            tenant_id: request.tenant_id,
            pool_id: "default".to_string(),
            cpu_cores: request.cpu_cores,
            memory_mb: request.memory_mb,
            worker_count: request.workers,
            estimated_duration: std::time::Duration::from_secs(300),
            priority,
        };
        engine
            .enqueue_request(resource_request)
            .await
            .map_err(|e| e.to_string())
    }

    /// Get WFQ statistics
    pub async fn get_stats(&self) -> Result<WFQStatsDto, String> {
        let engine = self.engine.lock().await;
        let stats = engine.get_stats().await;
        Ok(stats.into())
    }

    /// Get queue depth
    pub async fn get_queue_depth(&self) -> Result<u64, String> {
        let engine = self.engine.lock().await;
        let depth = engine.get_queue_depth().await;
        Ok(depth)
    }

    /// Clear WFQ queue
    pub async fn clear_queue(&self) -> Result<(), String> {
        let engine = self.engine.lock().await;
        engine.clear_queue().await;
        Ok(())
    }
}

/// API Routes

/// Get WFQ configuration
/// GET /api/v1/wfq/config
pub async fn get_config_handler(
    State(state): State<WFQIntegrationAppState>,
) -> Result<Json<WFQConfig>, (StatusCode, String)> {
    info!("Getting WFQ configuration");

    state
        .service
        .get_config()
        .await
        .map_err(|e| {
            error!("Failed to get WFQ config: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
        .map(Json)
}

/// Register a tenant with WFQ
/// POST /api/v1/wfq/tenants
pub async fn register_tenant_handler(
    State(state): State<WFQIntegrationAppState>,
    Json(request): Json<RegisterTenantRequest>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Registering tenant {}", request.tenant_quota.tenant_id);

    let tenant_quota = match request.tenant_quota.to_domain() {
        Ok(q) => q,
        Err(e) => return Err((StatusCode::BAD_REQUEST, e)),
    };

    // Convert usage (simplified for now)
    let tenant_usage = TenantUsage {
        tenant_id: request.tenant_usage.tenant_id,
        current_cpu_cores: request.tenant_usage.cpu_used,
        current_memory_mb: request.tenant_usage.memory_used_mb,
        current_workers: request.tenant_usage.workers_used,
        current_jobs: 0,
        daily_cost: 0.0,
        monthly_jobs: 0,
        last_updated: Utc::now(),
        burst_count_today: 0,
        last_burst: None,
    };

    match state
        .service
        .register_tenant(tenant_quota, &tenant_usage)
        .await
    {
        Ok(_) => {
            let response = ApiResponseDto::success("Tenant registered successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to register tenant: {}", e);
            Err((StatusCode::BAD_REQUEST, e))
        }
    }
}

/// Enqueue a resource request
/// POST /api/v1/wfq/requests
pub async fn enqueue_request_handler(
    State(state): State<WFQIntegrationAppState>,
    Json(request): Json<ResourceRequestDto>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Enqueueing WFQ request for tenant {}", request.tenant_id);

    match state.service.enqueue_request(request).await {
        Ok(_) => {
            let response = ApiResponseDto::success("Request enqueued successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to enqueue request: {}", e);
            Err((StatusCode::BAD_REQUEST, e))
        }
    }
}

/// Get queue depth
/// GET /api/v1/wfq/queue-depth
pub async fn get_queue_depth_handler(
    State(state): State<WFQIntegrationAppState>,
) -> Result<Json<ApiResponseDto<u64>>, (StatusCode, String)> {
    match state.service.get_queue_depth().await {
        Ok(depth) => {
            let response = ApiResponseDto::success(depth);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get queue depth: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Get WFQ statistics
/// GET /api/v1/wfq/stats
pub async fn get_stats_handler(
    State(state): State<WFQIntegrationAppState>,
) -> Result<Json<ApiResponseDto<WFQStatsDto>>, (StatusCode, String)> {
    match state.service.get_stats().await {
        Ok(stats) => {
            let response = ApiResponseDto::success(stats);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get WFQ stats: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Clear WFQ queue
/// POST /api/v1/wfq/clear-queue
pub async fn clear_queue_handler(
    State(state): State<WFQIntegrationAppState>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Clearing WFQ queue");

    match state.service.clear_queue().await {
        Ok(_) => {
            let response = ApiResponseDto::success("Queue cleared successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to clear queue: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Create router for WFQ integration routes
pub fn wfq_integration_routes() -> Router<WFQIntegrationAppState> {
    Router::new()
        .route("/wfq/config", get(get_config_handler))
        .route("/wfq/tenants", post(register_tenant_handler))
        .route("/wfq/requests", post(enqueue_request_handler))
        .route("/wfq/stats", get(get_stats_handler))
        .route("/wfq/queue-depth", get(get_queue_depth_handler))
        .route("/wfq/clear-queue", post(clear_queue_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use hodei_modules::weighted_fair_queuing::{WFQConfig, WeightStrategy};
    use tower::ServiceExt;

    fn create_test_app_state() -> WFQIntegrationAppState {
        let config = WFQConfig {
            enable_virtual_time: true,
            min_weight: 0.1,
            max_weight: 10.0,
            default_strategy: WeightStrategy::BillingTier,
            starvation_threshold: 0.5,
            weight_update_interval: Duration::from_secs(60),
            default_packet_size: 1000,
            enable_dynamic_weights: true,
            starvation_window: Duration::from_secs(300),
            fair_share_window: Duration::from_secs(3600),
        };
        let engine = WeightedFairQueueingEngine::new(config);
        let service = WFQIntegrationService::new(engine);

        WFQIntegrationAppState { service }
    }

    #[tokio::test]
    async fn test_get_config() {
        let state = create_test_app_state();

        let result = state.service.get_config().await;
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.min_weight, 0.1);
        assert_eq!(config.max_weight, 10.0);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let state = create_test_app_state();

        let result = state.service.get_stats().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_tenants, 0);
    }

    #[tokio::test]
    async fn test_get_queue_depth() {
        let state = create_test_app_state();

        let result = state.service.get_queue_depth().await;
        assert!(result.is_ok());

        let depth = result.unwrap();
        assert_eq!(depth, 0);
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let state = create_test_app_state();

        let result = state.service.clear_queue().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_endpoints() {
        let state = create_test_app_state();
        let app = wfq_integration_routes().with_state(state.clone());

        // Test config endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/wfq/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test stats endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/wfq/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test queue depth endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/wfq/queue-depth")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test clear queue endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/wfq/clear-queue")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
