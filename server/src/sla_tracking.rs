//! SLA Tracking API Module
//!
//! This module provides REST API endpoints for SLA tracking,
//! exposing the SLATracker capabilities through HTTP endpoints.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info};

use hodei_core::JobId;
use hodei_modules::sla_tracking::{
    PriorityAdjustment, SLALevel, SLARegistrationRequest, SLAResponse, SLATracker,
    SLAViolationEvent,
};

/// API application state
#[derive(Clone)]
pub struct SLATrackingAppState {
    pub service: SLATrackingService,
}

/// SLA tracking service
#[derive(Clone)]
pub struct SLATrackingService {
    pub tracker: Arc<tokio::sync::RwLock<SLATracker>>,
}

/// DTOs for request/response

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterSLARequest {
    pub job_id: String,
    pub sla_level: String,
    pub queue_position: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SLAStatusResponse {
    pub job_id: String,
    pub status: String,
    pub sla_level: String,
    pub deadline: String,
    pub time_remaining_seconds: u64,
    pub priority_boost: u8,
    pub queue_position: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SLAViolationResponse {
    pub job_id: String,
    pub deadline: String,
    pub violation_time: String,
    pub sla_level: String,
    pub wait_time_seconds: u64,
    pub priority_at_violation: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SLAStatsResponse {
    pub total_tracked: u64,
    pub on_time_completions: u64,
    pub sla_violations: u64,
    pub compliance_rate: f64,
    pub average_wait_time_seconds: u64,
    pub average_deadline_buffer_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SLAMetricsResponse {
    pub total_jobs: u64,
    pub at_risk_jobs: u64,
    pub critical_jobs: u64,
    pub violated_jobs: u64,
    pub compliance_rate: f64,
    pub average_time_remaining_seconds: u64,
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
}

impl SLATrackingService {
    /// Create new SLA tracking service
    pub fn new(tracker: SLATracker) -> Self {
        Self {
            tracker: Arc::new(tokio::sync::RwLock::new(tracker)),
        }
    }

    /// Register a job for SLA tracking
    pub async fn register_job(
        &self,
        job_id: String,
        sla_level: SLALevel,
        queue_position: usize,
    ) -> Result<SLAStatusResponse, String> {
        let job_id_obj = JobId::from(uuid::Uuid::parse_str(&job_id).map_err(|_| "Invalid job ID")?);

        let sla_info = self
            .tracker
            .write()
            .await
            .register_job(job_id_obj, sla_level, queue_position)
            .await;

        Ok(SLAStatusResponse {
            job_id,
            status: "registered".to_string(),
            sla_level: format!("{:?}", sla_info.sla_level),
            deadline: sla_info.deadline.to_rfc3339(),
            time_remaining_seconds: (sla_info.deadline - Utc::now())
                .to_std()
                .unwrap_or_default()
                .as_secs(),
            priority_boost: sla_info.priority_boost,
            queue_position,
        })
    }

    /// Get SLA status for a job
    pub async fn get_status(&self, job_id: &str) -> Result<SLAStatusResponse, String> {
        let job_id_obj = JobId::from(uuid::Uuid::parse_str(job_id).map_err(|_| "Invalid job ID")?);

        let (status, time_remaining) = self.tracker.read().await.get_sla_status(&job_id_obj).await;

        match status {
            Some((status, time_remaining)) => {
                Ok(SLAStatusResponse {
                    job_id: job_id.to_string(),
                    status: format!("{:?}", status),
                    sla_level: "unknown".to_string(), // Would need separate method to get this
                    deadline: Utc::now().to_rfc3339(), // Placeholder
                    time_remaining_seconds: time_remaining.as_secs(),
                    priority_boost: 0, // Would need separate method to get this
                    queue_position: 0, // Would need to be tracked separately
                })
            }
            None => Err("Job not found in SLA tracking".to_string()),
        }
    }

    /// Get SLA statistics
    pub async fn get_stats(&self) -> Result<SLAStatsResponse, String> {
        let stats = self.tracker.read().await.get_stats().await;
        Ok(SLAStatsResponse {
            total_tracked: stats.total_tracked,
            on_time_completions: stats.on_time_completions,
            sla_violations: stats.sla_violations,
            compliance_rate: stats.compliance_rate,
            average_wait_time_seconds: stats.average_wait_time.as_secs(),
            average_deadline_buffer_seconds: stats.average_deadline_buffer.as_secs(),
        })
    }

    /// Get SLA metrics snapshot
    pub async fn get_metrics(&self) -> Result<SLAMetricsResponse, String> {
        let metrics = self.tracker.read().await.get_metrics().await;
        Ok(SLAMetricsResponse {
            total_jobs: metrics.total_jobs,
            at_risk_jobs: metrics.at_risk_jobs,
            critical_jobs: metrics.critical_jobs,
            violated_jobs: metrics.violated_jobs,
            compliance_rate: metrics.compliance_rate,
            average_time_remaining_seconds: metrics.average_time_remaining.as_secs(),
        })
    }

    /// Check for SLA violations
    pub async fn check_violations(&self) -> Result<Vec<SLAViolationResponse>, String> {
        let alerts = self.tracker.read().await.check_violations().await;
        Ok(alerts
            .into_iter()
            .map(|alert| SLAViolationResponse {
                job_id: alert.job_id.to_string(),
                deadline: alert.deadline.to_rfc3339(),
                violation_time: alert.violation_time.to_rfc3339(),
                sla_level: format!("{:?}", alert.sla_level),
                wait_time_seconds: 0,     // Not available in alert
                priority_at_violation: 0, // Not available in alert
            })
            .collect())
    }
}

/// API Routes

/// Register a job for SLA tracking
/// POST /api/v1/sla/register
pub async fn register_job_handler(
    State(state): State<SLATrackingAppState>,
    Json(request): Json<RegisterSLARequest>,
) -> Result<Json<ApiResponseDto<SLAStatusResponse>>, (StatusCode, String)> {
    info!("Registering job {} for SLA tracking", request.job_id);

    let sla_level = match request.sla_level.to_lowercase().as_str() {
        "critical" => SLALevel::Critical,
        "high" => SLALevel::High,
        "medium" => SLALevel::Medium,
        "low" => SLALevel::Low,
        "best_effort" => SLALevel::BestEffort,
        _ => return Err((StatusCode::BAD_REQUEST, "Invalid SLA level".to_string())),
    };

    match state
        .service
        .register_job(request.job_id, sla_level, request.queue_position)
        .await
    {
        Ok(response) => {
            let api_response = ApiResponseDto::success(response);
            Ok(Json(api_response))
        }
        Err(e) => {
            error!("Failed to register job for SLA tracking: {}", e);
            Err((StatusCode::BAD_REQUEST, e))
        }
    }
}

/// Get SLA status for a job
/// GET /api/v1/sla/status/{job_id}
pub async fn get_status_handler(
    State(state): State<SLATrackingAppState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponseDto<SLAStatusResponse>>, (StatusCode, String)> {
    info!("Getting SLA status for job {}", job_id);

    match state.service.get_status(&job_id).await {
        Ok(response) => {
            let api_response = ApiResponseDto::success(response);
            Ok(Json(api_response))
        }
        Err(e) => {
            error!("Failed to get SLA status: {}", e);
            Err((StatusCode::NOT_FOUND, e))
        }
    }
}

/// Get SLA statistics
/// GET /api/v1/sla/stats
pub async fn get_stats_handler(
    State(state): State<SLATrackingAppState>,
) -> Result<Json<ApiResponseDto<SLAStatsResponse>>, (StatusCode, String)> {
    match state.service.get_stats().await {
        Ok(stats) => {
            let api_response = ApiResponseDto::success(stats);
            Ok(Json(api_response))
        }
        Err(e) => {
            error!("Failed to get SLA stats: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Get SLA metrics snapshot
/// GET /api/v1/sla/metrics
pub async fn get_metrics_handler(
    State(state): State<SLATrackingAppState>,
) -> Result<Json<ApiResponseDto<SLAMetricsResponse>>, (StatusCode, String)> {
    match state.service.get_metrics().await {
        Ok(metrics) => {
            let api_response = ApiResponseDto::success(metrics);
            Ok(Json(api_response))
        }
        Err(e) => {
            error!("Failed to get SLA metrics: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Check for SLA violations
/// GET /api/v1/sla/violations
pub async fn get_violations_handler(
    State(state): State<SLATrackingAppState>,
) -> Result<Json<ApiResponseDto<Vec<SLAViolationResponse>>>, (StatusCode, String)> {
    match state.service.check_violations().await {
        Ok(violations) => {
            let api_response = ApiResponseDto::success(violations);
            Ok(Json(api_response))
        }
        Err(e) => {
            error!("Failed to check SLA violations: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Create router for SLA tracking routes
pub fn sla_tracking_routes() -> Router<SLATrackingAppState> {
    Router::new()
        .route("/api/v1/sla/register", post(register_job_handler))
        .route("/api/v1/sla/status/:job_id", get(get_status_handler))
        .route("/api/v1/sla/stats", get(get_stats_handler))
        .route("/api/v1/sla/metrics", get(get_metrics_handler))
        .route("/api/v1/sla/violations", get(get_violations_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn create_test_app_state() -> SLATrackingAppState {
        let tracker = SLATracker::new();
        let service = SLATrackingService::new(tracker);
        SLATrackingAppState { service }
    }

    #[tokio::test]
    async fn test_register_job() {
        let state = create_test_app_state();
        let job_id = uuid::Uuid::new_v4().to_string();

        let result = state.service.register_job(job_id, SLALevel::High, 0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let state = create_test_app_state();

        let result = state.service.get_stats().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_tracked, 0);
    }

    #[tokio::test]
    async fn test_api_endpoints() {
        let state = create_test_app_state();
        let app = sla_tracking_routes().with_state(state.clone());

        // Test stats endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/sla/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test metrics endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/sla/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test violations endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/sla/violations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
