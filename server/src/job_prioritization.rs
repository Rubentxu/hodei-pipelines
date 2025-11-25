//! Job Queue Prioritization API Module
//!
//! This module provides REST API endpoints for job queue prioritization,
//! exposing the QueuePrioritizationEngine capabilities through HTTP endpoints.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use hodei_core::JobId;
use hodei_modules::{
    queue_prioritization::{
        FairShareAllocation, PreemptionCandidate, PreemptionPolicy, PrioritizationInfo,
        PrioritizationStats, PrioritizationStrategy, QueuePrioritizationEngine,
    },
    sla_tracking::SLALevel,
};

/// API application state
#[derive(Clone)]
pub struct JobPrioritizationAppState {
    pub service: JobPrioritizationService,
}

/// Job prioritization service
#[derive(Clone)]
pub struct JobPrioritizationService {
    pub engine: Arc<tokio::sync::Mutex<QueuePrioritizationEngine>>,
}

/// DTOs for request/response

#[derive(Debug, Serialize, Deserialize)]
pub struct PrioritizeJobRequest {
    pub job_id: String,
    pub base_priority: u8,
    pub sla_level: String,
    pub tenant_id: String,
}

impl PrioritizeJobRequest {
    fn to_domain(self) -> Result<PrioritizationInfo, String> {
        let job_id =
            JobId::from(uuid::Uuid::parse_str(&self.job_id).map_err(|_| "Invalid job ID")?);

        let sla_level = match self.sla_level.to_lowercase().as_str() {
            "critical" => SLALevel::Critical,
            "high" => SLALevel::High,
            "medium" => SLALevel::Medium,
            "low" => SLALevel::Low,
            "best_effort" => SLALevel::BestEffort,
            _ => return Err("Invalid SLA level".to_string()),
        };

        Ok(PrioritizationInfo {
            job_id,
            base_priority: self.base_priority,
            sla_level,
            tenant_id: self.tenant_id,
            priority_score: 0.0,
            can_preempt: false,
            preemption_score: 0.0,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrioritizeJobResponse {
    pub job_id: String,
    pub priority_score: f64,
    pub estimated_start_time: Option<u64>,
    pub position_in_queue: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreemptionCandidateDto {
    pub job_id: String,
    pub current_job_id: String,
    pub priority_score: f64,
    pub tenant_id: String,
    pub estimated_waste_seconds: u64,
}

impl From<PreemptionCandidate> for PreemptionCandidateDto {
    fn from(candidate: PreemptionCandidate) -> Self {
        Self {
            job_id: candidate.job_id.to_string(),
            current_job_id: candidate.current_job_id.to_string(),
            priority_score: candidate.priority_score,
            tenant_id: candidate.tenant_id,
            estimated_waste_seconds: candidate.estimated_waste.as_secs(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutePreemptionRequest {
    pub job_id: String,
    pub current_job_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrioritizationStatsDto {
    pub total_prioritized: u64,
    pub preemptions_requested: u64,
    pub preemptions_executed: u64,
    pub average_priority_score: f64,
    pub fairness_variance: f64,
    pub timestamp: u64,
}

impl From<PrioritizationStats> for PrioritizationStatsDto {
    fn from(stats: PrioritizationStats) -> Self {
        Self {
            total_prioritized: stats.total_prioritized,
            preemptions_requested: stats.preemptions_requested,
            preemptions_executed: stats.preemptions_executed,
            average_priority_score: stats.average_priority_score,
            fairness_variance: stats.fairness_variance,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueStateDto {
    pub jobs: Vec<PrioritizationInfoDto>,
    pub total_jobs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizationInfoDto {
    pub job_id: String,
    pub base_priority: u8,
    pub sla_level: String,
    pub tenant_id: String,
    pub priority_score: f64,
    pub can_preempt: bool,
    pub preemption_score: f64,
}

impl From<PrioritizationInfo> for PrioritizationInfoDto {
    fn from(info: PrioritizationInfo) -> Self {
        Self {
            job_id: info.job_id.to_string(),
            base_priority: info.base_priority,
            sla_level: match info.sla_level {
                SLALevel::Critical => "critical".to_string(),
                SLALevel::High => "high".to_string(),
                SLALevel::Medium => "medium".to_string(),
                SLALevel::Low => "low".to_string(),
                SLALevel::BestEffort => "best_effort".to_string(),
            },
            tenant_id: info.tenant_id,
            priority_score: info.priority_score,
            can_preempt: info.can_preempt,
            preemption_score: info.preemption_score,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FairShareAllocationDto {
    pub tenant_id: String,
    pub allocated_slots: u32,
    pub used_slots: u32,
    pub weight: f64,
    pub fairness_score: f64,
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

impl JobPrioritizationService {
    /// Create new job prioritization service
    pub fn new(engine: QueuePrioritizationEngine) -> Self {
        Self {
            engine: Arc::new(tokio::sync::Mutex::new(engine)),
        }
    }

    /// Prioritize a job in the queue
    pub async fn prioritize_job(
        &self,
        request: PrioritizeJobRequest,
    ) -> Result<PrioritizeJobResponse, String> {
        let job_id = request.job_id.clone();
        let prioritization_info = request.to_domain()?;

        let engine = self.engine.lock().await;
        let position = engine.prioritized_jobs.read().await.len();
        let info = engine
            .prioritize_job(
                prioritization_info.job_id,
                prioritization_info.base_priority,
                prioritization_info.sla_level,
                prioritization_info.tenant_id,
            )
            .await;

        Ok(PrioritizeJobResponse {
            job_id,
            priority_score: info.priority_score,
            estimated_start_time: None,
            position_in_queue: position,
        })
    }

    /// Get next job from the queue
    pub async fn get_next_job(&self) -> Result<Option<PrioritizationInfoDto>, String> {
        let engine = self.engine.lock().await;
        let job = engine.get_next_job().await;
        Ok(job.map(|j| j.into()))
    }

    /// Check preemption candidates
    pub async fn check_preemption_candidates(
        &self,
        job_id: &str,
    ) -> Result<Vec<PreemptionCandidateDto>, String> {
        let job_id_uuid = uuid::Uuid::parse_str(job_id).map_err(|_| "Invalid job ID")?;
        let job_id_obj = JobId::from(job_id_uuid);

        let engine = self.engine.lock().await;
        let candidates = engine.check_preemption_candidates(job_id_obj).await;

        Ok(candidates.into_iter().map(|c| c.into()).collect())
    }

    /// Execute preemption
    pub async fn execute_preemption(
        &self,
        job_id: &str,
        current_job_id: &str,
    ) -> Result<(), String> {
        let job_id_uuid = uuid::Uuid::parse_str(job_id).map_err(|_| "Invalid job ID")?;
        let current_job_id_uuid =
            uuid::Uuid::parse_str(current_job_id).map_err(|_| "Invalid current job ID")?;

        let candidate = PreemptionCandidate {
            job_id: JobId::from(job_id_uuid),
            current_job_id: JobId::from(current_job_id_uuid),
            priority_score: 0.0,
            tenant_id: "unknown".to_string(),
            estimated_waste: std::time::Duration::from_secs(0),
        };

        let mut engine = self.engine.lock().await;
        engine
            .execute_preemption(&candidate)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get prioritization statistics
    pub async fn get_stats(&self) -> Result<PrioritizationStatsDto, String> {
        let engine = self.engine.lock().await;
        let stats = engine.get_stats().await;
        Ok(stats.into())
    }

    /// Get queue state
    pub async fn get_queue_state(&self) -> Result<QueueStateDto, String> {
        let engine = self.engine.lock().await;
        let jobs = engine.get_queue_state().await;
        let job_dtos: Vec<PrioritizationInfoDto> = jobs.into_iter().map(|j| j.into()).collect();

        Ok(QueueStateDto {
            jobs: job_dtos.clone(),
            total_jobs: job_dtos.len(),
        })
    }

    /// Clear the prioritization queue
    pub async fn clear(&self) -> Result<(), String> {
        let engine = self.engine.lock().await;
        engine.clear().await;
        Ok(())
    }

    /// Remove a job from the queue
    pub async fn remove_job(&self, job_id: &str) -> Result<bool, String> {
        let job_id_uuid = uuid::Uuid::parse_str(job_id).map_err(|_| "Invalid job ID")?;
        let job_id_obj = JobId::from(job_id_uuid);

        let engine = self.engine.lock().await;
        let removed = engine.remove_job(&job_id_obj).await;
        Ok(removed)
    }
}

/// API Routes

/// Prioritize a job in the queue
/// POST /api/v1/jobs/prioritize
pub async fn prioritize_job_handler(
    State(state): State<JobPrioritizationAppState>,
    Json(request): Json<PrioritizeJobRequest>,
) -> Result<Json<ApiResponseDto<PrioritizeJobResponse>>, (StatusCode, String)> {
    info!("Prioritizing job {}", request.job_id);

    match state.service.prioritize_job(request).await {
        Ok(response) => {
            let response_wrapper = ApiResponseDto::success(response);
            Ok(Json(response_wrapper))
        }
        Err(e) => {
            error!("Failed to prioritize job: {}", e);
            Err((StatusCode::BAD_REQUEST, e))
        }
    }
}

/// Get next job from the queue
/// GET /api/v1/jobs/next
pub async fn get_next_job_handler(
    State(state): State<JobPrioritizationAppState>,
) -> Result<Json<ApiResponseDto<Option<PrioritizationInfoDto>>>, (StatusCode, String)> {
    info!("Getting next job from queue");

    match state.service.get_next_job().await {
        Ok(job) => {
            let response = ApiResponseDto::success(job);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get next job: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Check preemption candidates
/// POST /api/v1/jobs/{job_id}/preemption-candidates
pub async fn check_preemption_candidates_handler(
    State(state): State<JobPrioritizationAppState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponseDto<Vec<PreemptionCandidateDto>>>, (StatusCode, String)> {
    info!("Checking preemption candidates for job {}", job_id);

    match state.service.check_preemption_candidates(&job_id).await {
        Ok(candidates) => {
            let response = ApiResponseDto::success(candidates);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to check preemption candidates: {}", e);
            Err((StatusCode::BAD_REQUEST, e))
        }
    }
}

/// Execute preemption
/// POST /api/v1/jobs/preemption
pub async fn execute_preemption_handler(
    State(state): State<JobPrioritizationAppState>,
    Json(request): Json<ExecutePreemptionRequest>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!(
        "Executing preemption: {} preempting {}",
        request.job_id, request.current_job_id
    );

    match state
        .service
        .execute_preemption(&request.job_id, &request.current_job_id)
        .await
    {
        Ok(_) => {
            let response = ApiResponseDto::success("Preemption executed successfully".to_string());
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to execute preemption: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Get prioritization statistics
/// GET /api/v1/jobs/prioritization/stats
pub async fn get_stats_handler(
    State(state): State<JobPrioritizationAppState>,
) -> Result<Json<ApiResponseDto<PrioritizationStatsDto>>, (StatusCode, String)> {
    match state.service.get_stats().await {
        Ok(stats) => {
            let response = ApiResponseDto::success(stats);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get prioritization stats: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Get queue state
/// GET /api/v1/jobs/prioritization/queue
pub async fn get_queue_state_handler(
    State(state): State<JobPrioritizationAppState>,
) -> Result<Json<ApiResponseDto<QueueStateDto>>, (StatusCode, String)> {
    match state.service.get_queue_state().await {
        Ok(queue_state) => {
            let response = ApiResponseDto::success(queue_state);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get queue state: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

/// Clear the prioritization queue
/// POST /api/v1/jobs/prioritization/clear
pub async fn clear_queue_handler(
    State(state): State<JobPrioritizationAppState>,
) -> Result<Json<ApiResponseDto<String>>, (StatusCode, String)> {
    info!("Clearing prioritization queue");

    match state.service.clear().await {
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

/// Remove a job from the queue
/// DELETE /api/v1/jobs/{job_id}
pub async fn remove_job_handler(
    State(state): State<JobPrioritizationAppState>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponseDto<bool>>, (StatusCode, String)> {
    info!("Removing job {} from queue", job_id);

    match state.service.remove_job(&job_id).await {
        Ok(removed) => {
            let response = ApiResponseDto::success(removed);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to remove job: {}", e);
            Err((StatusCode::BAD_REQUEST, e))
        }
    }
}

/// Create router for job prioritization routes
pub fn job_prioritization_routes() -> Router<JobPrioritizationAppState> {
    Router::new()
        .route("/api/v1/jobs/prioritize", post(prioritize_job_handler))
        .route("/api/v1/jobs/next", get(get_next_job_handler))
        .route(
            "/api/v1/jobs/:job_id/preemption-candidates",
            post(check_preemption_candidates_handler),
        )
        .route("/api/v1/jobs/preemption", post(execute_preemption_handler))
        .route("/api/v1/jobs/prioritization/stats", get(get_stats_handler))
        .route(
            "/api/v1/jobs/prioritization/queue",
            get(get_queue_state_handler),
        )
        .route(
            "/api/v1/jobs/prioritization/clear",
            post(clear_queue_handler),
        )
        .route("/api/v1/jobs/:job_id", delete(remove_job_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use hodei_modules::{
        queue_prioritization::QueuePrioritizationEngine, sla_tracking::SLATracker,
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    fn create_test_app_state() -> JobPrioritizationAppState {
        let sla_tracker = Arc::new(SLATracker::new());
        let engine = QueuePrioritizationEngine::new(sla_tracker);
        let service = JobPrioritizationService::new(engine);

        JobPrioritizationAppState { service }
    }

    #[tokio::test]
    async fn test_get_next_job_empty() {
        let state = create_test_app_state();

        let result = state.service.get_next_job().await;
        assert!(result.is_ok());

        let job = result.unwrap();
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let state = create_test_app_state();

        let result = state.service.get_stats().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_prioritized, 0);
    }

    #[tokio::test]
    async fn test_get_queue_state() {
        let state = create_test_app_state();

        let result = state.service.get_queue_state().await;
        assert!(result.is_ok());

        let queue_state = result.unwrap();
        assert_eq!(queue_state.total_jobs, 0);
        assert!(queue_state.jobs.is_empty());
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let state = create_test_app_state();

        let result = state.service.clear().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_job_nonexistent() {
        let state = create_test_app_state();

        let job_id = uuid::Uuid::new_v4().to_string();
        let result = state.service.remove_job(&job_id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false for non-existent job
    }

    #[tokio::test]
    async fn test_api_endpoints() {
        let state = create_test_app_state();
        let app = job_prioritization_routes().with_state(state.clone());

        // Test stats endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs/prioritization/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test queue state endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/jobs/prioritization/queue")
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
                    .uri("/api/v1/jobs/prioritization/clear")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
