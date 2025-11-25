//! Queue Status API Module
//!
//! Provides comprehensive queue status monitoring, statistics, and health checks
//! for job queues across the system.

use axum::{Router, extract::State, response::Json, routing::get};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use hodei_modules::{
    queue_prioritization::QueuePrioritizationEngine,
    weighted_fair_queuing::WeightedFairQueueingEngine,
};

/// Application state for queue status
#[derive(Clone)]
pub struct QueueStatusAppState {
    pub prioritization_engine: Arc<RwLock<QueuePrioritizationEngine>>,
    pub wfq_engine: Arc<RwLock<WeightedFairQueueingEngine>>,
}

/// Service for queue status operations
#[derive(Clone)]
pub struct QueueStatusService {
    prioritization_engine: Arc<RwLock<QueuePrioritizationEngine>>,
    wfq_engine: Arc<RwLock<WeightedFairQueueingEngine>>,
}

impl QueueStatusService {
    pub fn new(
        prioritization_engine: Arc<RwLock<QueuePrioritizationEngine>>,
        wfq_engine: Arc<RwLock<WeightedFairQueueingEngine>>,
    ) -> Self {
        Self {
            prioritization_engine,
            wfq_engine,
        }
    }

    /// Get overall queue status
    pub async fn get_overall_status(&self) -> Result<QueueStatusResponse, String> {
        let prior_engine = self.prioritization_engine.read().await;
        let wfq_engine = self.wfq_engine.read().await;

        let total_depth = wfq_engine.get_queue_depth().await;
        let prioritization_queue = prior_engine.get_queue_state().await;

        Ok(QueueStatusResponse {
            timestamp: current_timestamp(),
            total_jobs_in_queue: total_depth,
            queue_health: calculate_queue_health(total_depth),
            prioritization_enabled: !prioritization_queue.is_empty(),
        })
    }

    /// Get queue depth for all tenants
    pub async fn get_queue_depth(&self) -> Result<HashMap<String, QueueDepthResponse>, String> {
        let prior_engine = self.prioritization_engine.read().await;
        let queue_state = prior_engine.get_queue_state().await;

        let mut tenant_depths: HashMap<String, u64> = HashMap::new();

        for info in queue_state {
            tenant_depths
                .entry(info.tenant_id.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        let mut result = HashMap::new();
        for (tenant_id, depth) in tenant_depths {
            result.insert(
                tenant_id.clone(),
                QueueDepthResponse {
                    tenant_id,
                    queue_depth: depth,
                    average_priority_score: 0.0,
                    high_priority_jobs: 0,
                    low_priority_jobs: 0,
                },
            );
        }

        Ok(result)
    }

    /// Get detailed queue information
    pub async fn get_queue_details(&self) -> Result<Vec<QueueJobDetail>, String> {
        let prior_engine = self.prioritization_engine.read().await;
        let queue_state = prior_engine.get_queue_state().await;

        let mut details = Vec::new();
        for info in queue_state {
            details.push(QueueJobDetail {
                job_id: info.job_id.to_string(),
                tenant_id: info.tenant_id,
                base_priority: info.base_priority,
                priority_score: info.priority_score,
                sla_level: format!("{:?}", info.sla_level),
                can_preempt: info.can_preempt,
                preemption_score: info.preemption_score,
            });
        }

        Ok(details)
    }

    /// Get queue health metrics
    pub async fn get_queue_health(&self) -> Result<QueueHealthResponse, String> {
        let wfq_engine = self.wfq_engine.read().await;
        let total_depth = wfq_engine.get_queue_depth().await;

        // Calculate health metrics
        let health_status = calculate_queue_health(total_depth);
        let utilization_ratio = (total_depth as f64 / 1000.0).min(1.0); // Assume max capacity of 1000

        Ok(QueueHealthResponse {
            timestamp: current_timestamp(),
            health_status,
            total_queue_depth: total_depth,
            utilization_ratio,
            risk_level: calculate_risk_level(total_depth),
            recommendations: generate_recommendations(total_depth),
        })
    }

    /// Get queue statistics over time
    pub async fn get_queue_statistics(&self) -> Result<QueueStatisticsResponse, String> {
        let prior_engine = self.prioritization_engine.read().await;
        let wfq_engine = self.wfq_engine.read().await;

        let total_depth = wfq_engine.get_queue_depth().await;
        let prioritization_queue = prior_engine.get_queue_state().await;

        // Calculate distribution by priority
        let mut priority_distribution = HashMap::new();
        for i in 1..=10 {
            priority_distribution.insert(i as u8, 0u64);
        }

        for info in &prioritization_queue {
            priority_distribution
                .entry(info.base_priority)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        // Calculate distribution by SLA level
        let mut sla_distribution = HashMap::new();
        for info in &prioritization_queue {
            let sla_key = format!("{:?}", info.sla_level);
            sla_distribution
                .entry(sla_key)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        Ok(QueueStatisticsResponse {
            timestamp: current_timestamp(),
            total_jobs: total_depth,
            priority_distribution,
            sla_distribution,
            average_priority_score: prioritization_queue
                .iter()
                .map(|p| p.priority_score)
                .sum::<f64>()
                / prioritization_queue.len().max(1) as f64,
            tenant_count: prioritization_queue
                .iter()
                .map(|p| p.tenant_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .len() as u64,
        })
    }
}

/// Response for queue status
#[derive(Debug, Serialize)]
pub struct QueueStatusResponse {
    pub timestamp: u64,
    pub total_jobs_in_queue: u64,
    pub queue_health: String,
    pub prioritization_enabled: bool,
}

/// Response for queue depth
#[derive(Debug, Serialize)]
pub struct QueueDepthResponse {
    pub tenant_id: String,
    pub queue_depth: u64,
    pub average_priority_score: f64,
    pub high_priority_jobs: u64,
    pub low_priority_jobs: u64,
}

/// Response for queue job details
#[derive(Debug, Serialize)]
pub struct QueueJobDetail {
    pub job_id: String,
    pub tenant_id: String,
    pub base_priority: u8,
    pub priority_score: f64,
    pub sla_level: String,
    pub can_preempt: bool,
    pub preemption_score: f64,
}

/// Response for queue health
#[derive(Debug, Serialize)]
pub struct QueueHealthResponse {
    pub timestamp: u64,
    pub health_status: String,
    pub total_queue_depth: u64,
    pub utilization_ratio: f64,
    pub risk_level: String,
    pub recommendations: Vec<String>,
}

/// Response for queue statistics
#[derive(Debug, Serialize)]
pub struct QueueStatisticsResponse {
    pub timestamp: u64,
    pub total_jobs: u64,
    pub priority_distribution: HashMap<u8, u64>,
    pub sla_distribution: HashMap<String, u64>,
    pub average_priority_score: f64,
    pub tenant_count: u64,
}

/// Get overall queue status
pub async fn get_overall_status_handler(
    State(app_state): State<QueueStatusAppState>,
) -> Result<Json<QueueStatusResponse>, (axum::http::StatusCode, String)> {
    let service = QueueStatusService::new(app_state.prioritization_engine, app_state.wfq_engine);
    match service.get_overall_status().await {
        Ok(status) => Ok(Json(status)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// Get queue depth per tenant
pub async fn get_queue_depth_handler(
    State(app_state): State<QueueStatusAppState>,
) -> Result<Json<HashMap<String, QueueDepthResponse>>, (axum::http::StatusCode, String)> {
    let service = QueueStatusService::new(app_state.prioritization_engine, app_state.wfq_engine);
    match service.get_queue_depth().await {
        Ok(depths) => Ok(Json(depths)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// Get detailed queue information
pub async fn get_queue_details_handler(
    State(app_state): State<QueueStatusAppState>,
) -> Result<Json<Vec<QueueJobDetail>>, (axum::http::StatusCode, String)> {
    let service = QueueStatusService::new(app_state.prioritization_engine, app_state.wfq_engine);
    match service.get_queue_details().await {
        Ok(details) => Ok(Json(details)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// Get queue health metrics
pub async fn get_queue_health_handler(
    State(app_state): State<QueueStatusAppState>,
) -> Result<Json<QueueHealthResponse>, (axum::http::StatusCode, String)> {
    let service = QueueStatusService::new(app_state.prioritization_engine, app_state.wfq_engine);
    match service.get_queue_health().await {
        Ok(health) => Ok(Json(health)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// Get queue statistics
pub async fn get_queue_statistics_handler(
    State(app_state): State<QueueStatusAppState>,
) -> Result<Json<QueueStatisticsResponse>, (axum::http::StatusCode, String)> {
    let service = QueueStatusService::new(app_state.prioritization_engine, app_state.wfq_engine);
    match service.get_queue_statistics().await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

/// Create queue status router
pub fn queue_status_routes() -> Router<QueueStatusAppState> {
    Router::new()
        .route("/queue/status", get(get_overall_status_handler))
        .route("/queue/depth", get(get_queue_depth_handler))
        .route("/queue/details", get(get_queue_details_handler))
        .route("/queue/health", get(get_queue_health_handler))
        .route("/queue/statistics", get(get_queue_statistics_handler))
}

/// Helper function to get current timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Calculate queue health status
fn calculate_queue_health(queue_depth: u64) -> String {
    match queue_depth {
        0..=100 => "HEALTHY".to_string(),
        101..=500 => "MODERATE".to_string(),
        501..=800 => "WARNING".to_string(),
        _ => "CRITICAL".to_string(),
    }
}

/// Calculate risk level based on queue depth
fn calculate_risk_level(queue_depth: u64) -> String {
    match queue_depth {
        0..=100 => "LOW".to_string(),
        101..=500 => "MEDIUM".to_string(),
        501..=800 => "HIGH".to_string(),
        _ => "VERY_HIGH".to_string(),
    }
}

/// Generate recommendations based on queue state
fn generate_recommendations(queue_depth: u64) -> Vec<String> {
    let mut recommendations = Vec::new();

    if queue_depth > 500 {
        recommendations.push("Consider scaling up worker pool".to_string());
        recommendations.push("Enable aggressive preemption".to_string());
        recommendations.push("Review SLA priorities".to_string());
    } else if queue_depth > 100 {
        recommendations.push("Monitor queue growth".to_string());
        recommendations.push("Consider resource optimization".to_string());
    } else {
        recommendations.push("Queue is healthy".to_string());
    }

    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_modules::{
        queue_prioritization::QueuePrioritizationEngine,
        weighted_fair_queuing::WeightedFairQueueingEngine,
    };

    #[tokio::test]
    async fn test_get_overall_status() {
        let sla_tracker = Arc::new(hodei_modules::sla_tracking::SLATracker::new());
        let prioritization_engine =
            Arc::new(RwLock::new(QueuePrioritizationEngine::new(sla_tracker)));
        let wfq_config = hodei_modules::weighted_fair_queuing::WFQConfig::default();
        let wfq_engine = Arc::new(RwLock::new(WeightedFairQueueingEngine::new(wfq_config)));

        let service = QueueStatusService::new(prioritization_engine, wfq_engine);
        let status = service.get_overall_status().await.unwrap();

        assert_eq!(status.total_jobs_in_queue, 0);
        assert_eq!(status.queue_health, "HEALTHY");
    }

    #[tokio::test]
    async fn test_get_queue_depth() {
        let sla_tracker = Arc::new(hodei_modules::sla_tracking::SLATracker::new());
        let prioritization_engine =
            Arc::new(RwLock::new(QueuePrioritizationEngine::new(sla_tracker)));
        let wfq_config = hodei_modules::weighted_fair_queuing::WFQConfig::default();
        let wfq_engine = Arc::new(RwLock::new(WeightedFairQueueingEngine::new(wfq_config)));

        let service = QueueStatusService::new(prioritization_engine, wfq_engine);
        let depths = service.get_queue_depth().await.unwrap();

        assert!(depths.is_empty());
    }

    #[tokio::test]
    async fn test_get_queue_health() {
        let sla_tracker = Arc::new(hodei_modules::sla_tracking::SLATracker::new());
        let prioritization_engine =
            Arc::new(RwLock::new(QueuePrioritizationEngine::new(sla_tracker)));
        let wfq_config = hodei_modules::weighted_fair_queuing::WFQConfig::default();
        let wfq_engine = Arc::new(RwLock::new(WeightedFairQueueingEngine::new(wfq_config)));

        let service = QueueStatusService::new(prioritization_engine, wfq_engine);
        let health = service.get_queue_health().await.unwrap();

        assert_eq!(health.health_status, "HEALTHY");
        assert_eq!(health.utilization_ratio, 0.0);
    }

    #[tokio::test]
    async fn test_get_queue_statistics() {
        let sla_tracker = Arc::new(hodei_modules::sla_tracking::SLATracker::new());
        let prioritization_engine =
            Arc::new(RwLock::new(QueuePrioritizationEngine::new(sla_tracker)));
        let wfq_config = hodei_modules::weighted_fair_queuing::WFQConfig::default();
        let wfq_engine = Arc::new(RwLock::new(WeightedFairQueueingEngine::new(wfq_config)));

        let service = QueueStatusService::new(prioritization_engine, wfq_engine);
        let stats = service.get_queue_statistics().await.unwrap();

        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.tenant_count, 0);
    }

    #[test]
    fn test_calculate_queue_health() {
        assert_eq!(calculate_queue_health(0), "HEALTHY");
        assert_eq!(calculate_queue_health(50), "HEALTHY");
        assert_eq!(calculate_queue_health(200), "MODERATE");
        assert_eq!(calculate_queue_health(600), "WARNING");
        assert_eq!(calculate_queue_health(900), "CRITICAL");
    }

    #[test]
    fn test_generate_recommendations() {
        let low_recs = generate_recommendations(10);
        assert!(low_recs.contains(&"Queue is healthy".to_string()));

        let medium_recs = generate_recommendations(200);
        assert!(medium_recs.contains(&"Monitor queue growth".to_string()));

        let high_recs = generate_recommendations(700);
        assert!(high_recs.contains(&"Consider scaling up worker pool".to_string()));
    }
}
