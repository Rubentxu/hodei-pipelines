//! Resource Pool Metrics API
//!
//! This module provides endpoints for exposing and querying resource pool metrics,
//! including real-time and historical metrics for pool performance monitoring.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_utilization: f64,    // Percentage 0-100
    pub memory_utilization: f64, // Percentage 0-100
    pub memory_used_mb: f64,
    pub memory_total_mb: f64,
    pub cpu_cores_used: f64,
    pub cpu_cores_total: f64,
    pub network_io_mbps: f64,
    pub disk_io_mbps: f64,
}

/// Job metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    pub queued_jobs: u64,
    pub running_jobs: u64,
    pub completed_jobs: u64,
    pub failed_jobs: u64,
    pub jobs_per_minute: f64,
    pub average_job_duration_ms: f64,
    pub average_queue_wait_time_ms: f64,
    pub throughput_jobs_per_second: f64,
}

/// Worker metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMetrics {
    pub total_workers: u64,
    pub active_workers: u64,
    pub idle_workers: u64,
    pub terminating_workers: u64,
    pub worker_utilization: f64, // Percentage 0-100
    pub average_task_duration_ms: f64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
}

/// Scaling metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingMetrics {
    pub scale_events_last_hour: u64,
    pub scale_out_events: u64,
    pub scale_in_events: u64,
    pub current_capacity: i32,
    pub target_capacity: i32,
    pub scaling_efficiency: f64, // Percentage 0-100
}

/// Pool metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetricsSnapshot {
    pub pool_id: String,
    pub timestamp: DateTime<Utc>,
    pub resource_metrics: ResourceMetrics,
    pub job_metrics: JobMetrics,
    pub worker_metrics: WorkerMetrics,
    pub scaling_metrics: ScalingMetrics,
    pub custom_metrics: HashMap<String, f64>,
}

/// Historical metrics data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Request to query historical metrics
#[derive(Debug, Deserialize)]
pub struct QueryMetricsRequest {
    pub pool_id: String,
    pub metric_type: String, // cpu_utilization, memory_utilization, queued_jobs, etc.
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub interval: Option<String>, // "1m", "5m", "1h", "1d"
}

/// Aggregated metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub pool_id: String,
    pub metric_type: String,
    pub data_points: Vec<MetricsDataPoint>,
    pub average_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub latest_value: f64,
}

/// Pool metrics overview
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolMetricsOverview {
    pub pool_id: String,
    pub pool_name: Option<String>,
    pub current_metrics: PoolMetricsSnapshot,
    pub health_status: String, // healthy, warning, critical
    pub alerts: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Create metrics record request
#[derive(Debug, Deserialize)]
pub struct CreateMetricsRecordRequest {
    pub pool_id: String,
    pub resource_metrics: ResourceMetrics,
    pub job_metrics: JobMetrics,
    pub worker_metrics: WorkerMetrics,
    pub scaling_metrics: ScalingMetrics,
    pub custom_metrics: Option<HashMap<String, f64>>,
}

/// Message response
#[derive(Debug, Serialize)]
pub struct MetricsMessageResponse {
    pub message: String,
}

/// Resource Pool Metrics Service
#[derive(Debug, Clone)]
pub struct ResourcePoolMetricsService {
    /// Current metrics snapshots by pool
    current_metrics: Arc<RwLock<HashMap<String, PoolMetricsSnapshot>>>,
    /// Historical metrics by pool and metric type
    historical_metrics: Arc<RwLock<HashMap<String, HashMap<String, Vec<MetricsDataPoint>>>>>,
}

impl ResourcePoolMetricsService {
    /// Create new Resource Pool Metrics Service
    pub fn new() -> Self {
        info!("Initializing Resource Pool Metrics Service");
        Self {
            current_metrics: Arc::new(RwLock::new(HashMap::new())),
            historical_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record metrics snapshot
    pub async fn record_metrics(
        &self,
        request: CreateMetricsRecordRequest,
    ) -> Result<PoolMetricsSnapshot, String> {
        let snapshot = PoolMetricsSnapshot {
            pool_id: request.pool_id.clone(),
            timestamp: Utc::now(),
            resource_metrics: request.resource_metrics,
            job_metrics: request.job_metrics,
            worker_metrics: request.worker_metrics,
            scaling_metrics: request.scaling_metrics,
            custom_metrics: request.custom_metrics.unwrap_or_else(HashMap::new),
        };

        // Update current metrics
        let mut current = self.current_metrics.write().await;
        current.insert(request.pool_id.clone(), snapshot.clone());

        // Store in historical metrics
        let mut historical = self.historical_metrics.write().await;
        let pool_metrics = historical
            .entry(request.pool_id.clone())
            .or_insert_with(HashMap::new);

        // Store each metric type
        pool_metrics
            .entry("cpu_utilization".to_string())
            .or_insert_with(Vec::new)
            .push(MetricsDataPoint {
                timestamp: snapshot.timestamp,
                value: snapshot.resource_metrics.cpu_utilization,
            });

        pool_metrics
            .entry("memory_utilization".to_string())
            .or_insert_with(Vec::new)
            .push(MetricsDataPoint {
                timestamp: snapshot.timestamp,
                value: snapshot.resource_metrics.memory_utilization,
            });

        pool_metrics
            .entry("queued_jobs".to_string())
            .or_insert_with(Vec::new)
            .push(MetricsDataPoint {
                timestamp: snapshot.timestamp,
                value: snapshot.job_metrics.queued_jobs as f64,
            });

        pool_metrics
            .entry("running_jobs".to_string())
            .or_insert_with(Vec::new)
            .push(MetricsDataPoint {
                timestamp: snapshot.timestamp,
                value: snapshot.job_metrics.running_jobs as f64,
            });

        pool_metrics
            .entry("active_workers".to_string())
            .or_insert_with(Vec::new)
            .push(MetricsDataPoint {
                timestamp: snapshot.timestamp,
                value: snapshot.worker_metrics.active_workers as f64,
            });

        // Keep only last 1000 data points per metric
        for metric_vec in pool_metrics.values_mut() {
            if metric_vec.len() > 1000 {
                metric_vec.drain(0..metric_vec.len() - 1000);
            }
        }

        info!("Recorded metrics snapshot for pool: {}", request.pool_id);

        Ok(snapshot)
    }

    /// Get current metrics for pool
    pub async fn get_current_metrics(&self, pool_id: &str) -> Result<PoolMetricsSnapshot, String> {
        let current = self.current_metrics.read().await;
        current
            .get(pool_id)
            .cloned()
            .ok_or_else(|| "Metrics not found".to_string())
    }

    /// List all pools with current metrics
    pub async fn list_pools_with_metrics(&self) -> Vec<String> {
        let current = self.current_metrics.read().await;
        current.keys().cloned().collect()
    }

    /// Query historical metrics
    pub async fn query_historical_metrics(
        &self,
        query: QueryMetricsRequest,
    ) -> Result<AggregatedMetrics, String> {
        let historical = self.historical_metrics.read().await;
        let pool_metrics = historical
            .get(&query.pool_id)
            .ok_or_else(|| "No metrics found for pool".to_string())?;

        let metric_type = &query.metric_type;
        let data_points = pool_metrics.get(metric_type).cloned().unwrap_or_default();

        if data_points.is_empty() {
            return Err("No data found for metric type".to_string());
        }

        // Apply time range filter
        let start_time = query
            .start_time
            .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
        let end_time = query.end_time.unwrap_or_else(|| Utc::now());

        let filtered: Vec<_> = data_points
            .into_iter()
            .filter(|dp| dp.timestamp >= start_time && dp.timestamp <= end_time)
            .collect();

        if filtered.is_empty() {
            return Err("No data in specified time range".to_string());
        }

        // Calculate aggregates
        let values: Vec<f64> = filtered.iter().map(|dp| dp.value).collect();
        let sum: f64 = values.iter().sum();
        let count = values.len() as f64;
        let average_value = sum / count;
        let min_value = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_value = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let latest_value = filtered.last().map(|dp| dp.value).unwrap_or(0.0);

        Ok(AggregatedMetrics {
            pool_id: query.pool_id.clone(),
            metric_type: metric_type.clone(),
            data_points: filtered,
            average_value,
            min_value,
            max_value,
            latest_value,
        })
    }

    /// Get pool metrics overview
    pub async fn get_pool_overview(&self, pool_id: &str) -> Result<PoolMetricsOverview, String> {
        let current = self.current_metrics.read().await;
        let snapshot = current
            .get(pool_id)
            .cloned()
            .ok_or_else(|| "Metrics not found".to_string())?;

        // Determine health status
        let health_status = if snapshot.resource_metrics.cpu_utilization > 90.0
            || snapshot.resource_metrics.memory_utilization > 90.0
            || snapshot.job_metrics.failed_jobs > snapshot.job_metrics.completed_jobs
        {
            "critical".to_string()
        } else if snapshot.resource_metrics.cpu_utilization > 70.0
            || snapshot.resource_metrics.memory_utilization > 70.0
        {
            "warning".to_string()
        } else {
            "healthy".to_string()
        };

        // Generate alerts
        let mut alerts = Vec::new();
        if snapshot.resource_metrics.cpu_utilization > 80.0 {
            alerts.push(format!(
                "High CPU utilization: {:.1}%",
                snapshot.resource_metrics.cpu_utilization
            ));
        }
        if snapshot.resource_metrics.memory_utilization > 80.0 {
            alerts.push(format!(
                "High memory utilization: {:.1}%",
                snapshot.resource_metrics.memory_utilization
            ));
        }
        if snapshot.job_metrics.failed_jobs as f64
            > snapshot.job_metrics.completed_jobs as f64 * 0.1
        {
            alerts.push(format!(
                "High failure rate: {} failed jobs",
                snapshot.job_metrics.failed_jobs
            ));
        }

        // Generate recommendations
        let mut recommendations = Vec::new();
        if snapshot.worker_metrics.idle_workers > snapshot.worker_metrics.active_workers {
            recommendations.push("Consider scaling down: high number of idle workers".to_string());
        }
        if snapshot.job_metrics.queued_jobs > 100 {
            recommendations.push("Consider scaling up: high queue depth".to_string());
        }

        Ok(PoolMetricsOverview {
            pool_id: snapshot.pool_id.clone(),
            pool_name: None, // Could be looked up
            current_metrics: snapshot,
            health_status,
            alerts,
            recommendations,
        })
    }

    /// Delete metrics for pool
    pub async fn delete_metrics(&self, pool_id: &str) -> Result<(), String> {
        let mut current = self.current_metrics.write().await;
        let mut historical = self.historical_metrics.write().await;

        if !current.contains_key(pool_id) {
            return Err("Metrics not found".to_string());
        }

        current.remove(pool_id);
        historical.remove(pool_id);

        info!("Deleted metrics for pool: {}", pool_id);

        Ok(())
    }

    /// Generate mock metrics for testing
    pub async fn generate_mock_metrics(&self, pool_id: &str) -> PoolMetricsSnapshot {
        let snapshot = PoolMetricsSnapshot {
            pool_id: pool_id.to_string(),
            timestamp: Utc::now(),
            resource_metrics: ResourceMetrics {
                cpu_utilization: 45.0 + rand::random::<f64>() * 40.0, // 45-85%
                memory_utilization: 30.0 + rand::random::<f64>() * 50.0, // 30-80%
                memory_used_mb: 1024.0 + rand::random::<f64>() * 2048.0,
                memory_total_mb: 4096.0,
                cpu_cores_used: 2.0 + rand::random::<f64>() * 6.0,
                cpu_cores_total: 8.0,
                network_io_mbps: rand::random::<f64>() * 100.0,
                disk_io_mbps: rand::random::<f64>() * 50.0,
            },
            job_metrics: JobMetrics {
                queued_jobs: (rand::random::<u64>() % 50) as u64,
                running_jobs: (rand::random::<u64>() % 20) as u64,
                completed_jobs: 1000 + (rand::random::<u64>() % 500) as u64,
                failed_jobs: (rand::random::<u64>() % 10) as u64,
                jobs_per_minute: rand::random::<f64>() * 50.0,
                average_job_duration_ms: 5000.0 + rand::random::<f64>() * 10000.0,
                average_queue_wait_time_ms: 1000.0 + rand::random::<f64>() * 5000.0,
                throughput_jobs_per_second: rand::random::<f64>() * 10.0,
            },
            worker_metrics: WorkerMetrics {
                total_workers: 10,
                active_workers: 5 + (rand::random::<u64>() % 5) as u64,
                idle_workers: (rand::random::<u64>() % 5) as u64,
                terminating_workers: 0,
                worker_utilization: 60.0 + rand::random::<f64>() * 30.0,
                average_task_duration_ms: 3000.0 + rand::random::<f64>() * 7000.0,
                tasks_completed: 500 + (rand::random::<u64>() % 500) as u64,
                tasks_failed: (rand::random::<u64>() % 20) as u64,
            },
            scaling_metrics: ScalingMetrics {
                scale_events_last_hour: rand::random::<u64>() % 3,
                scale_out_events: rand::random::<u64>() % 5,
                scale_in_events: rand::random::<u64>() % 5,
                current_capacity: 10 + (rand::random::<i32>() % 5),
                target_capacity: 10,
                scaling_efficiency: 85.0 + rand::random::<f64>() * 15.0,
            },
            custom_metrics: HashMap::new(),
        };

        // Store as current metrics
        let mut current = self.current_metrics.write().await;
        current.insert(pool_id.to_string(), snapshot.clone());

        info!("Generated mock metrics for pool: {}", pool_id);

        snapshot
    }
}

/// Application state for Resource Pool Metrics
#[derive(Clone)]
pub struct ResourcePoolMetricsAppState {
    pub service: Arc<ResourcePoolMetricsService>,
}

/// Create router for Resource Pool Metrics API
pub fn resource_pool_metrics_routes() -> Router<ResourcePoolMetricsAppState> {
    Router::new()
        .route("/pool-metrics", post(record_pool_metrics_handler))
        .route("/pool-metrics/:pool_id", get(get_pool_metrics_handler))
        .route(
            "/pool-metrics/:pool_id/overview",
            get(get_pool_overview_handler),
        )
        .route(
            "/pool-metrics/query",
            post(query_historical_metrics_handler),
        )
        .route("/pool-metrics/pools", get(list_pools_handler))
        .route(
            "/pool-metrics/:pool_id",
            delete(delete_pool_metrics_handler),
        )
        .route(
            "/pool-metrics/:pool_id/mock",
            post(generate_mock_metrics_handler),
        )
}

/// Record pool metrics handler
async fn record_pool_metrics_handler(
    State(state): State<ResourcePoolMetricsAppState>,
    Json(payload): Json<CreateMetricsRecordRequest>,
) -> Result<Json<MetricsMessageResponse>, StatusCode> {
    match state.service.record_metrics(payload).await {
        Ok(snapshot) => Ok(Json(MetricsMessageResponse {
            message: format!(
                "Metrics recorded for pool {} at {}",
                snapshot.pool_id, snapshot.timestamp
            ),
        })),
        Err(e) => {
            error!("Failed to record metrics: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Get pool metrics handler
async fn get_pool_metrics_handler(
    State(state): State<ResourcePoolMetricsAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<PoolMetricsSnapshot>, StatusCode> {
    match state.service.get_current_metrics(&pool_id).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get pool overview handler
async fn get_pool_overview_handler(
    State(state): State<ResourcePoolMetricsAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<PoolMetricsOverview>, StatusCode> {
    match state.service.get_pool_overview(&pool_id).await {
        Ok(overview) => Ok(Json(overview)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Query historical metrics handler
async fn query_historical_metrics_handler(
    State(state): State<ResourcePoolMetricsAppState>,
    Json(query): Json<QueryMetricsRequest>,
) -> Result<Json<AggregatedMetrics>, StatusCode> {
    match state.service.query_historical_metrics(query).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => {
            error!("Failed to query metrics: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// List pools handler
async fn list_pools_handler(
    State(state): State<ResourcePoolMetricsAppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    Ok(Json(state.service.list_pools_with_metrics().await))
}

/// Delete pool metrics handler
async fn delete_pool_metrics_handler(
    State(state): State<ResourcePoolMetricsAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<MetricsMessageResponse>, StatusCode> {
    match state.service.delete_metrics(&pool_id).await {
        Ok(_) => Ok(Json(MetricsMessageResponse {
            message: format!("Metrics deleted for pool {}", pool_id),
        })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Generate mock metrics handler
async fn generate_mock_metrics_handler(
    State(state): State<ResourcePoolMetricsAppState>,
    axum::extract::Path(pool_id): axum::extract::Path<String>,
) -> Result<Json<PoolMetricsSnapshot>, StatusCode> {
    let snapshot = state.service.generate_mock_metrics(&pool_id).await;
    Ok(Json(snapshot))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_metrics() {
        let service = ResourcePoolMetricsService::new();

        let request = CreateMetricsRecordRequest {
            pool_id: "pool-1".to_string(),
            resource_metrics: ResourceMetrics {
                cpu_utilization: 50.0,
                memory_utilization: 60.0,
                memory_used_mb: 2048.0,
                memory_total_mb: 4096.0,
                cpu_cores_used: 4.0,
                cpu_cores_total: 8.0,
                network_io_mbps: 100.0,
                disk_io_mbps: 50.0,
            },
            job_metrics: JobMetrics {
                queued_jobs: 10,
                running_jobs: 5,
                completed_jobs: 100,
                failed_jobs: 2,
                jobs_per_minute: 10.0,
                average_job_duration_ms: 5000.0,
                average_queue_wait_time_ms: 2000.0,
                throughput_jobs_per_second: 5.0,
            },
            worker_metrics: WorkerMetrics {
                total_workers: 10,
                active_workers: 7,
                idle_workers: 3,
                terminating_workers: 0,
                worker_utilization: 70.0,
                average_task_duration_ms: 3000.0,
                tasks_completed: 100,
                tasks_failed: 2,
            },
            scaling_metrics: ScalingMetrics {
                scale_events_last_hour: 2,
                scale_out_events: 1,
                scale_in_events: 1,
                current_capacity: 10,
                target_capacity: 10,
                scaling_efficiency: 90.0,
            },
            custom_metrics: None,
        };

        let result = service.record_metrics(request).await;
        assert!(result.is_ok());

        let snapshot = result.unwrap();
        assert_eq!(snapshot.pool_id, "pool-1");
        assert_eq!(snapshot.resource_metrics.cpu_utilization, 50.0);
    }

    #[tokio::test]
    async fn test_get_current_metrics() {
        let service = ResourcePoolMetricsService::new();

        let request = CreateMetricsRecordRequest {
            pool_id: "pool-1".to_string(),
            resource_metrics: ResourceMetrics {
                cpu_utilization: 50.0,
                memory_utilization: 60.0,
                memory_used_mb: 2048.0,
                memory_total_mb: 4096.0,
                cpu_cores_used: 4.0,
                cpu_cores_total: 8.0,
                network_io_mbps: 100.0,
                disk_io_mbps: 50.0,
            },
            job_metrics: JobMetrics {
                queued_jobs: 10,
                running_jobs: 5,
                completed_jobs: 100,
                failed_jobs: 2,
                jobs_per_minute: 10.0,
                average_job_duration_ms: 5000.0,
                average_queue_wait_time_ms: 2000.0,
                throughput_jobs_per_second: 5.0,
            },
            worker_metrics: WorkerMetrics {
                total_workers: 10,
                active_workers: 7,
                idle_workers: 3,
                terminating_workers: 0,
                worker_utilization: 70.0,
                average_task_duration_ms: 3000.0,
                tasks_completed: 100,
                tasks_failed: 2,
            },
            scaling_metrics: ScalingMetrics {
                scale_events_last_hour: 2,
                scale_out_events: 1,
                scale_in_events: 1,
                current_capacity: 10,
                target_capacity: 10,
                scaling_efficiency: 90.0,
            },
            custom_metrics: None,
        };

        service.record_metrics(request).await.unwrap();

        let metrics = service.get_current_metrics("pool-1").await.unwrap();
        assert_eq!(metrics.pool_id, "pool-1");
    }

    #[tokio::test]
    async fn test_get_pool_overview() {
        let service = ResourcePoolMetricsService::new();

        let request = CreateMetricsRecordRequest {
            pool_id: "pool-1".to_string(),
            resource_metrics: ResourceMetrics {
                cpu_utilization: 50.0,
                memory_utilization: 60.0,
                memory_used_mb: 2048.0,
                memory_total_mb: 4096.0,
                cpu_cores_used: 4.0,
                cpu_cores_total: 8.0,
                network_io_mbps: 100.0,
                disk_io_mbps: 50.0,
            },
            job_metrics: JobMetrics {
                queued_jobs: 10,
                running_jobs: 5,
                completed_jobs: 100,
                failed_jobs: 2,
                jobs_per_minute: 10.0,
                average_job_duration_ms: 5000.0,
                average_queue_wait_time_ms: 2000.0,
                throughput_jobs_per_second: 5.0,
            },
            worker_metrics: WorkerMetrics {
                total_workers: 10,
                active_workers: 7,
                idle_workers: 3,
                terminating_workers: 0,
                worker_utilization: 70.0,
                average_task_duration_ms: 3000.0,
                tasks_completed: 100,
                tasks_failed: 2,
            },
            scaling_metrics: ScalingMetrics {
                scale_events_last_hour: 2,
                scale_out_events: 1,
                scale_in_events: 1,
                current_capacity: 10,
                target_capacity: 10,
                scaling_efficiency: 90.0,
            },
            custom_metrics: None,
        };

        service.record_metrics(request).await.unwrap();

        let overview = service.get_pool_overview("pool-1").await.unwrap();
        assert_eq!(overview.pool_id, "pool-1");
        assert_eq!(overview.health_status, "healthy");
    }

    #[tokio::test]
    async fn test_delete_metrics() {
        let service = ResourcePoolMetricsService::new();

        let request = CreateMetricsRecordRequest {
            pool_id: "pool-1".to_string(),
            resource_metrics: ResourceMetrics {
                cpu_utilization: 50.0,
                memory_utilization: 60.0,
                memory_used_mb: 2048.0,
                memory_total_mb: 4096.0,
                cpu_cores_used: 4.0,
                cpu_cores_total: 8.0,
                network_io_mbps: 100.0,
                disk_io_mbps: 50.0,
            },
            job_metrics: JobMetrics {
                queued_jobs: 10,
                running_jobs: 5,
                completed_jobs: 100,
                failed_jobs: 2,
                jobs_per_minute: 10.0,
                average_job_duration_ms: 5000.0,
                average_queue_wait_time_ms: 2000.0,
                throughput_jobs_per_second: 5.0,
            },
            worker_metrics: WorkerMetrics {
                total_workers: 10,
                active_workers: 7,
                idle_workers: 3,
                terminating_workers: 0,
                worker_utilization: 70.0,
                average_task_duration_ms: 3000.0,
                tasks_completed: 100,
                tasks_failed: 2,
            },
            scaling_metrics: ScalingMetrics {
                scale_events_last_hour: 2,
                scale_out_events: 1,
                scale_in_events: 1,
                current_capacity: 10,
                target_capacity: 10,
                scaling_efficiency: 90.0,
            },
            custom_metrics: None,
        };

        service.record_metrics(request).await.unwrap();

        let result = service.delete_metrics("pool-1").await;
        assert!(result.is_ok());

        let pools = service.list_pools_with_metrics().await;
        assert!(pools.is_empty());
    }
}
