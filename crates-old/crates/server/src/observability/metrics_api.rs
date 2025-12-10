//! Dashboard Metrics API
//!
//! This module provides the Dashboard Metrics API endpoint for US-011
//! Returns aggregated KPI data for the dashboard

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Request parameters for dashboard metrics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DashboardMetricsRequest {
    /// Optional tenant ID filter
    pub tenant_id: Option<String>,
    /// Optional time range filter (in hours)
    pub time_range_hours: Option<u32>,
}

/// Dashboard metrics response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(example = json!({ "total_pipelines": 50, "active_pipelines": 42, "total_executions_today": 128, "success_rate": 94.5, "avg_duration": 125, "cost_per_run": 0.45, "queue_time": 12 }))]
pub struct DashboardMetrics {
    /// Total number of pipelines
    pub total_pipelines: u32,
    /// Number of active pipelines
    pub active_pipelines: u32,
    /// Total executions today
    pub total_executions_today: u32,
    /// Success rate percentage (0-100)
    pub success_rate: f64,
    /// Average execution duration in seconds
    pub avg_duration: u64,
    /// Average cost per run in dollars
    pub cost_per_run: f64,
    /// Average queue time in seconds
    pub queue_time: u64,
    /// Timestamp of the metrics
    pub timestamp: DateTime<Utc>,
}

/// Service for aggregating dashboard metrics
#[derive(Debug)]
pub struct DashboardMetricsService {
    /// Cache of recent metrics by tenant
    metrics_cache: Arc<RwLock<HashMap<String, Vec<DashboardMetrics>>>>,
    /// Global metrics (not tenant-specific)
    global_metrics: Arc<RwLock<DashboardMetrics>>,
}

impl DashboardMetricsService {
    /// Create new dashboard metrics service
    pub fn new() -> Self {
        // Initialize with mock data
        let default_metrics = DashboardMetrics {
            total_pipelines: 50,
            active_pipelines: 42,
            total_executions_today: 128,
            success_rate: 94.5,
            avg_duration: 125,
            cost_per_run: 0.45,
            queue_time: 12,
            timestamp: Utc::now(),
        };

        Self {
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
            global_metrics: Arc::new(RwLock::new(default_metrics)),
        }
    }

    /// Get dashboard metrics with optional filters
    pub async fn get_metrics(&self, request: &DashboardMetricsRequest) -> DashboardMetrics {
        // In production, this would query the database
        // For now, return mock data with slight variations based on filters

        let mut metrics = self.global_metrics.read().await.clone();

        // Apply time range filter (simulated)
        if let Some(hours) = request.time_range_hours {
            // Adjust metrics based on time range
            let multiplier = match hours {
                1 => 0.1,
                24 => 1.0,
                168 => 7.0,
                _ => 1.0,
            };
            metrics.total_executions_today =
                (metrics.total_executions_today as f64 * multiplier) as u32;
        }

        // Apply tenant filter (simulated)
        if let Some(ref tenant_id) = request.tenant_id {
            // For specific tenant, simulate different metrics
            let tenant_hash = tenant_id.len() as u32;
            metrics.total_pipelines = 10 + (tenant_hash % 30);
            metrics.active_pipelines = metrics.total_pipelines - (tenant_hash % 5);
            metrics.total_executions_today = 20 + (tenant_hash % 50);
        }

        // Update timestamp
        metrics.timestamp = Utc::now();

        // Cache the metrics
        if let Some(ref tenant_id) = request.tenant_id {
            let mut cache = self.metrics_cache.write().await;
            let entry = cache.entry(tenant_id.clone()).or_insert_with(Vec::new);
            entry.push(metrics.clone());
        }

        metrics
    }

    /// Record execution completion for metrics aggregation
    pub async fn record_execution(
        &self,
        tenant_id: Option<&str>,
        duration: u64,
        cost: f64,
        success: bool,
    ) {
        let metrics = DashboardMetrics {
            total_pipelines: 50, // Would be calculated from DB
            active_pipelines: 42,
            total_executions_today: 1, // Increment this
            success_rate: if success { 94.5 } else { 93.0 },
            avg_duration: duration,
            cost_per_run: cost,
            queue_time: 12,
            timestamp: Utc::now(),
        };

        if let Some(tenant) = tenant_id {
            let mut cache = self.metrics_cache.write().await;
            let entry = cache.entry(tenant.to_string()).or_insert_with(Vec::new);
            entry.push(metrics);
        } else {
            let mut global = self.global_metrics.write().await;
            *global = metrics;
        }
    }
}

impl Default for DashboardMetricsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for Dashboard Metrics API
#[derive(Clone)]
pub struct DashboardMetricsApiAppState {
    pub service: Arc<DashboardMetricsService>,
}

/// GET /api/v1/metrics/dashboard - Get dashboard metrics
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    params(
        ("tenant_id" = Option<String>, Query, description = "Filter by tenant ID"),
        ("range" = Option<String>, Query, description = "Time range in hours (e.g., '24h', '168h')"),
        ("time_range_hours" = Option<u32>, Query, description = "Alternative time range parameter")
    ),
    responses(
        (status = 200, description = "Dashboard metrics", body = DashboardMetrics),
        (status = 500, description = "Internal server error")
    ),
    tag = "observability"
)]
pub async fn get_dashboard_metrics(
    State(state): State<DashboardMetricsApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<DashboardMetrics>, StatusCode> {
    info!("📊 Dashboard metrics requested");

    // Parse request parameters
    let request = DashboardMetricsRequest {
        tenant_id: params.get("tenant_id").cloned(),
        time_range_hours: params
            .get("range")
            .and_then(|s| {
                if s.ends_with('h') {
                    s.trim_end_matches('h').parse().ok()
                } else {
                    s.parse().ok()
                }
            })
            .or_else(|| params.get("time_range_hours").and_then(|s| s.parse().ok())),
    };

    // Get metrics from service
    let metrics = state.service.get_metrics(&request).await;

    info!(
        "✅ Dashboard metrics returned - Pipelines: {}, Success rate: {:.1}%",
        metrics.total_pipelines, metrics.success_rate
    );

    Ok(Json(metrics))
}

/// Dashboard Metrics API routes
pub fn dashboard_metrics_api_routes() -> Router<DashboardMetricsApiAppState> {
    Router::new().route("/metrics/dashboard", get(get_dashboard_metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dashboard_metrics_service_new() {
        let service = DashboardMetricsService::new();
        let metrics = service
            .get_metrics(&DashboardMetricsRequest {
                tenant_id: None,
                time_range_hours: None,
            })
            .await;

        assert_eq!(metrics.total_pipelines, 50);
        assert_eq!(metrics.active_pipelines, 42);
        assert!(metrics.success_rate > 0.0);
    }

    #[tokio::test]
    async fn test_dashboard_metrics_service_with_tenant() {
        let service = DashboardMetricsService::new();
        let metrics = service
            .get_metrics(&DashboardMetricsRequest {
                tenant_id: Some("tenant-123".to_string()),
                time_range_hours: Some(24),
            })
            .await;

        // Should have some pipelines for the tenant
        assert!(metrics.total_pipelines >= 10);
        assert!(metrics.total_pipelines <= 40);
    }

    #[tokio::test]
    async fn test_dashboard_metrics_service_with_time_range() {
        let service = DashboardMetricsService::new();

        // Test 24h range
        let metrics_24h = service
            .get_metrics(&DashboardMetricsRequest {
                tenant_id: None,
                time_range_hours: Some(24),
            })
            .await;

        // Test 1h range (should be less)
        let metrics_1h = service
            .get_metrics(&DashboardMetricsRequest {
                tenant_id: None,
                time_range_hours: Some(1),
            })
            .await;

        assert!(metrics_1h.total_executions_today <= metrics_24h.total_executions_today);
    }

    #[tokio::test]
    async fn test_dashboard_metrics_serialization() {
        let metrics = DashboardMetrics {
            total_pipelines: 50,
            active_pipelines: 42,
            total_executions_today: 128,
            success_rate: 94.5,
            avg_duration: 125,
            cost_per_run: 0.45,
            queue_time: 12,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("total_pipelines"));
        assert!(json.contains("active_pipelines"));
        assert!(json.contains("success_rate"));

        let deserialized: DashboardMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_pipelines, metrics.total_pipelines);
        assert_eq!(deserialized.success_rate, metrics.success_rate);
    }

    #[tokio::test]
    async fn test_dashboard_metrics_request_parsing() {
        let request = DashboardMetricsRequest {
            tenant_id: Some("test-tenant".to_string()),
            time_range_hours: Some(24),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tenant_id"));
        assert!(json.contains("time_range_hours"));
    }

    #[tokio::test]
    async fn test_record_execution() {
        let service = DashboardMetricsService::new();

        service
            .record_execution(Some("tenant-123"), 120, 0.50, true)
            .await;

        // In a real implementation, we would verify the metrics were updated
        // For now, just ensure it doesn't panic
    }
}
