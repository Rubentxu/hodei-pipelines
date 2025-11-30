//! Quota Enforcement API Module
//!
//! This module provides REST API endpoints for quota enforcement and admission control,
//! exposing the QuotaEnforcementEngine capabilities through HTTP endpoints.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};

use hodei_pipelines_modules::{
    multi_tenancy_quota_manager::{MultiTenancyQuotaManager, ResourceRequest, TenantId},
    quota_enforcement::{
        AdmissionDecision, EnforcementAction, EnforcementError, EnforcementPolicy,
        EnforcementStats, PreemptionCandidate, QueuedRequest, QuotaEnforcementEngine,
    },
};

/// API application state
#[derive(Clone)]
pub struct QuotaEnforcementAppState {
    pub service: QuotaEnforcementService,
}

/// Quota enforcement service
#[derive(Clone)]
pub struct QuotaEnforcementService {
    pub enforcement_engine: Arc<tokio::sync::Mutex<QuotaEnforcementEngine>>,
}

/// DTOs for request/response

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceRequestDto {
    pub tenant_id: String,
    pub pool_id: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub worker_count: u32,
    pub priority: String,
    pub estimated_duration_seconds: u64,
}

impl ResourceRequestDto {
    fn to_domain(&self) -> ResourceRequest {
        ResourceRequest {
            tenant_id: self.tenant_id.clone(),
            pool_id: self.pool_id.clone(),
            cpu_cores: self.cpu_cores,
            memory_mb: self.memory_mb,
            worker_count: self.worker_count,
            estimated_duration: Duration::from_secs(self.estimated_duration_seconds),
            priority: match self.priority.to_lowercase().as_str() {
                "critical" => hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::Critical,
                "high" => hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::High,
                "low" => hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::Low,
                "batch" => hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::Batch,
                _ => hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::Normal,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdmissionDecisionDto {
    pub allowed: bool,
    pub reason: Option<String>,
    pub estimated_wait_seconds: Option<u64>,
    pub enforcement_action: String,
    pub priority_hint: Option<i32>,
}

impl From<AdmissionDecision> for AdmissionDecisionDto {
    fn from(decision: AdmissionDecision) -> Self {
        Self {
            allowed: decision.allowed,
            reason: decision.reason,
            estimated_wait_seconds: decision.estimated_wait.map(|d| d.as_secs()),
            enforcement_action: match decision.enforcement_action {
                EnforcementAction::Allow => "allow".to_string(),
                EnforcementAction::Deny => "deny".to_string(),
                EnforcementAction::Queue => "queue".to_string(),
                EnforcementAction::Preempt => "preempt".to_string(),
                EnforcementAction::Defer => "defer".to_string(),
                EnforcementAction::Throttle => "throttle".to_string(),
            },
            priority_hint: match decision.enforcement_action {
                EnforcementAction::Queue => Some(50),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnforcementPolicyDto {
    pub strict_mode: bool,
    pub queue_on_violation: bool,
    pub preemption_enabled: bool,
    pub grace_period_seconds: u64,
    pub enforcement_delay_seconds: u64,
    pub max_queue_size: usize,
    pub enable_burst_enforcement: bool,
}

impl Default for EnforcementPolicyDto {
    fn default() -> Self {
        Self {
            strict_mode: false,
            queue_on_violation: true,
            preemption_enabled: false,
            grace_period_seconds: 30,
            enforcement_delay_seconds: 5,
            max_queue_size: 100,
            enable_burst_enforcement: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnforcementStatsDto {
    pub total_requests: u64,
    pub admitted_requests: u64,
    pub denied_requests: u64,
    pub queued_requests: u64,
    pub preempted_jobs: u64,
    pub enforcement_actions: HashMap<String, u64>,
    pub average_enforcement_latency_ms: f64,
    pub queue_utilization: f64,
    pub violation_detected: u64,
    pub timestamp: u64,
}

impl From<EnforcementStats> for EnforcementStatsDto {
    fn from(stats: EnforcementStats) -> Self {
        Self {
            total_requests: stats.total_requests,
            admitted_requests: stats.admitted_requests,
            denied_requests: stats.denied_requests,
            queued_requests: stats.queued_requests,
            preempted_jobs: stats.preempted_jobs,
            enforcement_actions: stats.enforcement_actions,
            average_enforcement_latency_ms: stats.average_enforcement_latency_ms,
            queue_utilization: stats.queue_utilization,
            violation_detected: stats.violation_detected,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueuedRequestDto {
    pub request: ResourceRequestDto,
    pub queued_at_seconds: i64,
    pub priority: i32,
    pub attempts: u32,
}

impl From<QueuedRequest> for QueuedRequestDto {
    fn from(queued: QueuedRequest) -> Self {
        Self {
            request: ResourceRequestDto {
                tenant_id: queued.request.tenant_id,
                pool_id: queued.request.pool_id,
                cpu_cores: queued.request.cpu_cores,
                memory_mb: queued.request.memory_mb,
                worker_count: queued.request.worker_count,
                priority: match queued.request.priority {
                    hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::Critical => {
                        "critical".to_string()
                    }
                    hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::High => {
                        "high".to_string()
                    }
                    hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::Low => {
                        "low".to_string()
                    }
                    hodei_pipelines_modules::multi_tenancy_quota_manager::JobPriority::Batch => {
                        "batch".to_string()
                    }
                    _ => "normal".to_string(),
                },
                estimated_duration_seconds: queued.request.estimated_duration.as_secs(),
            },
            queued_at_seconds: queued.queued_at.timestamp(),
            priority: queued.priority,
            attempts: queued.attempts,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueuedRequestsResponseDto {
    pub tenant_id: String,
    pub queue_size: usize,
    pub requests: Vec<QueuedRequestDto>,
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

impl QuotaEnforcementService {
    /// Create new quota enforcement service
    pub fn new(quota_manager: Arc<MultiTenancyQuotaManager>, policy: EnforcementPolicy) -> Self {
        let enforcement_engine = QuotaEnforcementEngine::new(quota_manager, policy);

        Self {
            enforcement_engine: Arc::new(tokio::sync::Mutex::new(enforcement_engine)),
        }
    }

    /// Evaluate admission request with quota enforcement
    pub async fn evaluate_admission(
        &self,
        request: ResourceRequestDto,
    ) -> Result<AdmissionDecisionDto, EnforcementError> {
        let domain_request = request.to_domain();
        let mut engine = self.enforcement_engine.lock().await;

        let decision = engine.evaluate_admission(domain_request).await?;

        Ok(decision.into())
    }

    /// Admit a resource request
    pub async fn admit_request(&self, request: ResourceRequestDto) -> Result<(), EnforcementError> {
        let domain_request = request.to_domain();
        let mut engine = self.enforcement_engine.lock().await;

        engine.admit_request(&domain_request).await?;

        Ok(())
    }

    /// Get enforcement statistics
    pub async fn get_stats(&self) -> Result<EnforcementStatsDto, EnforcementError> {
        let engine = self.enforcement_engine.lock().await;
        let stats = engine.get_stats();
        Ok(stats.into())
    }

    /// Get queued requests for a tenant
    pub async fn get_queued_requests(
        &self,
        tenant_id: &str,
    ) -> Result<QueuedRequestsResponseDto, EnforcementError> {
        let engine = self.enforcement_engine.lock().await;
        let queued = engine.get_queued_requests(tenant_id);

        let requests = match queued {
            Some(queue) => queue.iter().map(|q| q.clone().into()).collect(),
            None => Vec::new(),
        };

        Ok(QueuedRequestsResponseDto {
            tenant_id: tenant_id.to_string(),
            queue_size: requests.len(),
            requests,
        })
    }

    /// Clear queued requests for a tenant
    pub async fn clear_queue(&self, tenant_id: &str) -> Result<(), EnforcementError> {
        let mut engine = self.enforcement_engine.lock().await;
        engine.clear_queue(tenant_id);
        Ok(())
    }

    /// Process queued requests
    pub async fn process_queued_requests(&self) -> Result<(), EnforcementError> {
        let mut engine = self.enforcement_engine.lock().await;
        engine.process_queued_requests().await?;
        Ok(())
    }
}

/// API Routes

/// Evaluate admission request with quota enforcement
/// POST /api/v1/quotas/enforce/evaluate
pub async fn evaluate_admission_handler(
    State(state): State<QuotaEnforcementAppState>,
    Json(request): Json<ResourceRequestDto>,
) -> Result<Json<ApiResponseDto<AdmissionDecisionDto>>, (StatusCode, String)> {
    info!("Evaluating admission for tenant {}", request.tenant_id);

    match state.service.evaluate_admission(request).await {
        Ok(decision) => {
            let response = ApiResponseDto::success(decision);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to evaluate admission: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Admit a resource request
/// POST /api/v1/quotas/enforce/admit
pub async fn admit_request_handler(
    State(state): State<QuotaEnforcementAppState>,
    Json(request): Json<ResourceRequestDto>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Admitting request for tenant {}", request.tenant_id);

    match state.service.admit_request(request).await {
        Ok(_) => {
            let response = ApiResponseDto::success("Request admitted successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to admit request: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Get enforcement statistics
/// GET /api/v1/enforcement/stats
pub async fn get_stats_handler(
    State(state): State<QuotaEnforcementAppState>,
) -> Result<Json<ApiResponseDto<EnforcementStatsDto>>, (StatusCode, String)> {
    match state.service.get_stats().await {
        Ok(stats) => {
            let response = ApiResponseDto::success(stats);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get stats: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Get queued requests for a tenant
/// GET /api/v1/tenants/{tenant_id}/enforcement/queued
pub async fn get_queued_requests_handler(
    State(state): State<QuotaEnforcementAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<ApiResponseDto<QueuedRequestsResponseDto>>, (StatusCode, String)> {
    info!("Getting queued requests for tenant {}", tenant_id);

    match state.service.get_queued_requests(&tenant_id).await {
        Ok(queued) => {
            let response = ApiResponseDto::success(queued);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get queued requests: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Clear queued requests for a tenant
/// POST /api/v1/tenants/{tenant_id}/enforcement/clear
pub async fn clear_queue_handler(
    State(state): State<QuotaEnforcementAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Clearing queue for tenant {}", tenant_id);

    match state.service.clear_queue(&tenant_id).await {
        Ok(_) => {
            let response = ApiResponseDto::success("Queue cleared successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to clear queue: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Process queued requests
/// POST /api/v1/enforcement/process-queued
pub async fn process_queued_requests_handler(
    State(state): State<QuotaEnforcementAppState>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Processing queued requests");

    match state.service.process_queued_requests().await {
        Ok(_) => {
            let response =
                ApiResponseDto::success("Queued requests processed successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to process queued requests: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Create router for quota enforcement routes
pub fn quota_enforcement_routes() -> Router<QuotaEnforcementAppState> {
    Router::new()
        .route(
            "/api/v1/quotas/enforce/evaluate",
            post(evaluate_admission_handler),
        )
        .route("/api/v1/quotas/enforce/admit", post(admit_request_handler))
        .route("/api/v1/enforcement/stats", get(get_stats_handler))
        .route(
            "/api/v1/tenants/{tenant_id}/enforcement/queued",
            get(get_queued_requests_handler),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/enforcement/clear",
            post(clear_queue_handler),
        )
        .route(
            "/api/v1/enforcement/process-queued",
            post(process_queued_requests_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use hodei_pipelines_modules::multi_tenancy_quota_manager::{
        BillingTier, BurstPolicy, QuotaLimits, QuotaManagerConfig, QuotaType, TenantQuota,
    };
    use std::time::Duration;
    use tower::ServiceExt;

    async fn create_test_app_state() -> QuotaEnforcementAppState {
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(QuotaManagerConfig::default()));

        // Create a test tenant with sufficient resources
        let test_quota = TenantQuota {
            tenant_id: "tenant-1".to_string(),
            limits: QuotaLimits {
                max_cpu_cores: 1000,
                max_memory_mb: 10240,
                max_concurrent_workers: 100,
                max_concurrent_jobs: 50,
                max_daily_cost: 1000.0,
                max_monthly_jobs: 10000,
            },
            pool_access: HashMap::new(),
            burst_policy: BurstPolicy {
                allowed: true,
                max_burst_multiplier: 2.0,
                burst_duration: Duration::from_secs(300),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: BillingTier::Standard,
            quota_type: QuotaType::HardLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Register the test tenant
        // Note: We're in a test context, so we need to handle this properly
        let quota_manager_clone = Arc::clone(&quota_manager);
        quota_manager_clone
            .register_tenant(test_quota)
            .await
            .unwrap();

        let policy = EnforcementPolicy {
            strict_mode: false,
            queue_on_violation: true,
            preemption_enabled: false,
            grace_period: Duration::from_secs(30),
            enforcement_delay: Duration::from_secs(5),
            max_queue_size: 100,
            enable_burst_enforcement: true,
        };

        QuotaEnforcementAppState {
            service: QuotaEnforcementService::new(quota_manager, policy),
        }
    }

    #[tokio::test]
    async fn test_evaluate_admission_allowed() {
        let state = create_test_app_state().await;

        let request = ResourceRequestDto {
            tenant_id: "tenant-1".to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores: 10,
            memory_mb: 256,
            worker_count: 5,
            priority: "normal".to_string(),
            estimated_duration_seconds: 3600,
        };

        let result = state.service.evaluate_admission(request).await;
        assert!(result.is_ok());

        let decision = result.unwrap();
        assert!(decision.allowed || decision.enforcement_action == "queue");
    }

    #[tokio::test]
    async fn test_admit_request() {
        let state = create_test_app_state().await;

        let request = ResourceRequestDto {
            tenant_id: "tenant-1".to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores: 10,
            memory_mb: 256,
            worker_count: 5,
            priority: "normal".to_string(),
            estimated_duration_seconds: 3600,
        };

        let result = state.service.admit_request(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let state = create_test_app_state().await;

        let result = state.service.get_stats().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_requests, 0);
    }

    #[tokio::test]
    async fn test_get_queued_requests_empty() {
        let state = create_test_app_state().await;

        let result = state.service.get_queued_requests("tenant-1").await;
        assert!(result.is_ok());

        let queued = result.unwrap();
        assert_eq!(queued.tenant_id, "tenant-1");
        assert_eq!(queued.queue_size, 0);
        assert!(queued.requests.is_empty());
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let state = create_test_app_state().await;

        // Clear non-existent queue should succeed
        let result = state.service.clear_queue("tenant-1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_queued_requests() {
        let state = create_test_app_state().await;

        let result = state.service.process_queued_requests().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_endpoints() {
        let state = create_test_app_state().await;
        let app = quota_enforcement_routes().with_state(state);

        // Test stats endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/enforcement/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test clear queue endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tenants/tenant-1/enforcement/clear")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test process queued endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/enforcement/process-queued")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_evaluate_admission_api() {
        let state = create_test_app_state().await;
        let app = quota_enforcement_routes().with_state(state);

        let request_body = serde_json::to_string(&ResourceRequestDto {
            tenant_id: "tenant-1".to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores: 10,
            memory_mb: 256,
            worker_count: 5,
            priority: "normal".to_string(),
            estimated_duration_seconds: 3600,
        })
        .unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/quotas/enforce/evaluate")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_admit_request_api() {
        let state = create_test_app_state().await;
        let app = quota_enforcement_routes().with_state(state);

        let request_body = serde_json::to_string(&ResourceRequestDto {
            tenant_id: "tenant-1".to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores: 10,
            memory_mb: 256,
            worker_count: 5,
            priority: "normal".to_string(),
            estimated_duration_seconds: 3600,
        })
        .unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/quotas/enforce/admit")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_queued_requests_api() {
        let state = create_test_app_state().await;
        let app = quota_enforcement_routes().with_state(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/tenants/tenant-1/enforcement/queued")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
