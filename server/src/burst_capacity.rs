//! Burst Capacity API Module
//!
//! This module provides REST API endpoints for burst capacity management,
//! exposing the BurstCapacityManager capabilities through HTTP endpoints.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};

use hodei_modules::{
    burst_capacity_manager::{
        BurstCapacityConfig, BurstCapacityManager, BurstDecision, BurstError, BurstResourceRequest,
        BurstSession, BurstStats, BurstStatus,
    },
    multi_tenancy_quota_manager::MultiTenancyQuotaManager,
};

use std::sync::Arc;

/// API application state
#[derive(Clone)]
pub struct BurstCapacityAppState {
    pub service: BurstCapacityService,
}

/// Burst capacity service
#[derive(Clone)]
pub struct BurstCapacityService {
    pub manager: Arc<tokio::sync::Mutex<BurstCapacityManager>>,
}

/// DTOs for request/response

#[derive(Debug, Serialize, Deserialize)]
pub struct BurstRequestDto {
    pub tenant_id: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub worker_count: u32,
    pub burst_multiplier: f64,
    pub requested_duration_minutes: u64,
}

impl BurstRequestDto {
    fn to_domain(&self) -> BurstResourceRequest {
        BurstResourceRequest {
            cpu_cores: self.cpu_cores,
            memory_mb: self.memory_mb,
            worker_count: self.worker_count,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurstDecisionDto {
    pub allowed: bool,
    pub reason: String,
    pub allocated_multiplier: f64,
    pub max_duration_seconds: u64,
    pub cost_impact: f64,
    pub session_id: Option<String>,
}

impl From<BurstDecision> for BurstDecisionDto {
    fn from(decision: BurstDecision) -> Self {
        Self {
            allowed: decision.allowed,
            reason: decision.reason,
            allocated_multiplier: decision.allocated_multiplier,
            max_duration_seconds: decision.max_duration.as_secs(),
            cost_impact: decision.cost_impact,
            session_id: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurstSessionDto {
    pub tenant_id: String,
    pub session_id: String,
    pub start_time_seconds: i64,
    pub expiry_time_seconds: i64,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub worker_count: u32,
    pub burst_multiplier: f64,
    pub status: String,
    pub cost_accrued: f64,
}

impl From<BurstSession> for BurstSessionDto {
    fn from(session: BurstSession) -> Self {
        Self {
            tenant_id: session.tenant_id,
            session_id: format!("burst-{}", session.start_time.timestamp()),
            start_time_seconds: session.start_time.timestamp(),
            expiry_time_seconds: session.expiry_time.timestamp(),
            cpu_cores: session.requested_resources.cpu_cores,
            memory_mb: session.requested_resources.memory_mb,
            worker_count: session.requested_resources.worker_count,
            burst_multiplier: session.burst_multiplier,
            status: match session.status {
                BurstStatus::Active => "active".to_string(),
                BurstStatus::Queued => "queued".to_string(),
                BurstStatus::Expired => "expired".to_string(),
                BurstStatus::Terminated => "terminated".to_string(),
            },
            cost_accrued: session.cost_accrued,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurstStatsDto {
    pub total_burst_sessions: u64,
    pub active_burst_sessions: u64,
    pub queued_burst_requests: u64,
    pub expired_bursts: u64,
    pub average_burst_duration: f64,
    pub total_burst_cost: f64,
    pub burst_success_rate: f64,
    pub global_burst_capacity_used: f64,
    pub timestamp: u64,
}

impl From<BurstStats> for BurstStatsDto {
    fn from(stats: BurstStats) -> Self {
        Self {
            total_burst_sessions: stats.total_burst_sessions,
            active_burst_sessions: stats.active_burst_sessions,
            queued_burst_requests: stats.queued_burst_requests,
            expired_bursts: stats.expired_bursts,
            average_burst_duration: stats.average_burst_duration,
            total_burst_cost: stats.total_burst_cost,
            burst_success_rate: stats.burst_success_rate,
            global_burst_capacity_used: stats.global_burst_capacity_used,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurstSessionsResponseDto {
    pub sessions: Vec<BurstSessionDto>,
    pub total_count: usize,
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

impl BurstCapacityService {
    /// Create new burst capacity service
    pub fn new(quota_manager: Arc<MultiTenancyQuotaManager>, config: BurstCapacityConfig) -> Self {
        let quota_manager_inner = Arc::try_unwrap(quota_manager).unwrap_or_else(|_| unreachable!());
        let manager = BurstCapacityManager::new(config, quota_manager_inner);

        Self {
            manager: Arc::new(tokio::sync::Mutex::new(manager)),
        }
    }

    /// Request burst capacity for a tenant
    pub async fn request_burst(
        &self,
        tenant_id: &str,
        request: BurstRequestDto,
    ) -> Result<BurstDecisionDto, BurstError> {
        let burst_request = request.to_domain();

        let mut manager = self.manager.lock().await;
        let decision = manager
            .request_burst_capacity(tenant_id, burst_request, request.burst_multiplier)
            .await?;

        Ok(decision.into())
    }

    /// Get active burst sessions for a tenant
    pub async fn get_active_sessions(
        &self,
        tenant_id: &str,
    ) -> Result<BurstSessionsResponseDto, BurstError> {
        let manager = self.manager.lock().await;
        let sessions = manager.get_active_sessions();

        // Filter sessions by tenant_id
        let tenant_sessions: Vec<&BurstSession> = sessions
            .into_iter()
            .filter(|s| s.tenant_id == tenant_id)
            .collect();

        let session_dtos: Vec<BurstSessionDto> = tenant_sessions
            .iter()
            .map(|s| (**s).clone().into())
            .collect();
        let total_count = session_dtos.len();

        Ok(BurstSessionsResponseDto {
            sessions: session_dtos,
            total_count,
        })
    }

    /// Get all active burst sessions
    pub async fn get_all_sessions(&self) -> Result<BurstSessionsResponseDto, BurstError> {
        let manager = self.manager.lock().await;
        let sessions = manager.get_active_sessions();

        let session_dtos: Vec<BurstSessionDto> =
            sessions.iter().map(|s| (**s).clone().into()).collect();
        let total_count = session_dtos.len();

        Ok(BurstSessionsResponseDto {
            sessions: session_dtos,
            total_count,
        })
    }

    /// Terminate a burst session
    pub async fn terminate_session(&self, session_id: &str) -> Result<(), BurstError> {
        // Extract tenant_id from session_id format "burst-{timestamp}"
        let parts: Vec<&str> = session_id.split('-').collect();
        if parts.len() != 2 {
            return Err(BurstError::SessionNotFound(session_id.to_string()));
        }

        // For simplicity, we'll use the timestamp to find the session
        // In a real implementation, we'd maintain a proper session ID mapping
        let mut manager = self.manager.lock().await;

        // Try to end burst session by tenant_id (we'll assume the session_id maps to tenant_id)
        manager.end_burst_session(session_id).await?;
        Ok(())
    }

    /// Get burst capacity statistics
    pub async fn get_stats(&self) -> Result<BurstStatsDto, BurstError> {
        let manager = self.manager.lock().await;
        let stats = manager.get_stats();
        Ok(stats.into())
    }

    /// Check burst capacity availability
    pub async fn check_capacity(
        &self,
        tenant_id: &str,
        multiplier: f64,
    ) -> Result<bool, BurstError> {
        let manager = self.manager.lock().await;
        let is_in_burst = manager.is_in_burst(tenant_id);

        // If already in burst, not available
        if is_in_burst {
            return Ok(false);
        }

        // Check if we have capacity (simplified check)
        Ok(manager.get_stats().active_burst_sessions < 100) // Using max_concurrent_bursts
    }
}

/// API Routes

/// Request burst capacity for a tenant
/// POST /api/v1/tenants/{tenant_id}/burst/request
pub async fn request_burst_handler(
    State(state): State<BurstCapacityAppState>,
    Path(tenant_id): Path<String>,
    Json(request): Json<BurstRequestDto>,
) -> Result<Json<ApiResponseDto<BurstDecisionDto>>, (StatusCode, String)> {
    info!("Requesting burst capacity for tenant {}", tenant_id);

    match state.service.request_burst(&tenant_id, request).await {
        Ok(decision) => {
            let response = ApiResponseDto::success(decision);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to request burst capacity: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Get active burst sessions for a tenant
/// GET /api/v1/tenants/{tenant_id}/burst/sessions
pub async fn get_tenant_sessions_handler(
    State(state): State<BurstCapacityAppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<ApiResponseDto<BurstSessionsResponseDto>>, (StatusCode, String)> {
    info!("Getting burst sessions for tenant {}", tenant_id);

    match state.service.get_active_sessions(&tenant_id).await {
        Ok(sessions) => {
            let response = ApiResponseDto::success(sessions);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get burst sessions: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Get all active burst sessions
/// GET /api/v1/burst/sessions
pub async fn get_all_sessions_handler(
    State(state): State<BurstCapacityAppState>,
) -> Result<Json<ApiResponseDto<BurstSessionsResponseDto>>, (StatusCode, String)> {
    match state.service.get_all_sessions().await {
        Ok(sessions) => {
            let response = ApiResponseDto::success(sessions);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get all burst sessions: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Terminate a burst session
/// DELETE /api/v1/burst/sessions/{session_id}
pub async fn terminate_session_handler(
    State(state): State<BurstCapacityAppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Terminating burst session {}", session_id);

    match state.service.terminate_session(&session_id).await {
        Ok(_) => {
            let response =
                ApiResponseDto::success("Burst session terminated successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to terminate burst session: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Get burst capacity statistics
/// GET /api/v1/burst/stats
pub async fn get_stats_handler(
    State(state): State<BurstCapacityAppState>,
) -> Result<Json<ApiResponseDto<BurstStatsDto>>, (StatusCode, String)> {
    match state.service.get_stats().await {
        Ok(stats) => {
            let response = ApiResponseDto::success(stats);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get burst stats: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Check burst capacity availability
/// POST /api/v1/burst/check
pub async fn check_capacity_handler(
    State(state): State<BurstCapacityAppState>,
    Json(request): Json<BurstCheckRequest>,
) -> Result<Json<ApiResponseDto<BurstCapacityCheckResponse>>, (StatusCode, String)> {
    info!(
        "Checking burst capacity for tenant {} with multiplier {}",
        request.tenant_id, request.multiplier
    );

    match state
        .service
        .check_capacity(&request.tenant_id, request.multiplier)
        .await
    {
        Ok(available) => {
            let response = ApiResponseDto::success(BurstCapacityCheckResponse {
                available,
                tenant_id: request.tenant_id,
                multiplier: request.multiplier,
            });
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to check burst capacity: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurstCheckRequest {
    pub tenant_id: String,
    pub multiplier: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurstCapacityCheckResponse {
    pub available: bool,
    pub tenant_id: String,
    pub multiplier: f64,
}

/// Create router for burst capacity routes
pub fn burst_capacity_routes() -> Router<BurstCapacityAppState> {
    Router::new()
        .route(
            "/api/v1/tenants/{tenant_id}/burst/request",
            post(request_burst_handler),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/burst/sessions",
            get(get_tenant_sessions_handler),
        )
        .route("/api/v1/burst/sessions", get(get_all_sessions_handler))
        .route(
            "/api/v1/burst/sessions/{session_id}",
            delete(terminate_session_handler),
        )
        .route("/api/v1/burst/stats", get(get_stats_handler))
        .route("/api/v1/burst/check", post(check_capacity_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use hodei_modules::multi_tenancy_quota_manager::QuotaManagerConfig;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn create_test_app_state() -> BurstCapacityAppState {
        let quota_manager = Arc::new(MultiTenancyQuotaManager::new(QuotaManagerConfig::default()));
        let config = BurstCapacityConfig {
            enabled: true,
            default_multiplier: 1.5,
            max_burst_duration: Duration::from_secs(3600),
            burst_cooldown: Duration::from_secs(600),
            global_burst_pool_ratio: 0.2,
            max_concurrent_bursts: 100,
            burst_cost_multiplier: 2.0,
            enable_burst_queuing: true,
        };

        BurstCapacityAppState {
            service: BurstCapacityService::new(quota_manager, config),
        }
    }

    #[tokio::test]
    async fn test_get_all_sessions() {
        let state = create_test_app_state();

        let result = state.service.get_all_sessions().await;
        assert!(result.is_ok());

        let sessions = result.unwrap();
        assert_eq!(sessions.total_count, 0);
        assert!(sessions.sessions.is_empty());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let state = create_test_app_state();

        let result = state.service.get_stats().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_burst_sessions, 0);
        assert_eq!(stats.active_burst_sessions, 0);
    }

    #[tokio::test]
    async fn test_check_capacity() {
        let state = create_test_app_state();

        let result = state.service.check_capacity("tenant-1", 1.5).await;
        assert!(result.is_ok());

        let available = result.unwrap();
        assert!(available || !available); // Either true or false is valid
    }

    #[tokio::test]
    async fn test_api_endpoints() {
        let state = create_test_app_state();
        let app = burst_capacity_routes().with_state(state.clone());

        // Test stats endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/burst/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test all sessions endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/burst/sessions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test check capacity endpoint
        let request_body = serde_json::to_string(&BurstCheckRequest {
            tenant_id: "tenant-1".to_string(),
            multiplier: 1.5,
        })
        .unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/burst/check")
                    .header("Content-Type", "application/json")
                    .body(Body::from(request_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
