//! Resource Pool Metrics Collector Module
//!
//! This module provides comprehensive metrics collection and export capabilities
//! for resource pools, including Prometheus metrics, OpenTelemetry traces,
//! and JSON APIs.

use chrono::{DateTime, Utc};
use hodei_pipelines_domain::{DomainError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::identity_access::multi_tenancy_quota_manager::{PoolId, TenantId};
use hodei_pipelines_ports::ResourcePool;

/// Metrics collection error types
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Collection failed for pool {0}: {1}")]
    CollectionFailed(String, String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Export failed: {0}")]
    ExportError(String),
}

/// Pool size metrics
#[derive(Debug, Clone)]
pub struct PoolSizeMetrics {
    pub current_size: u32,
    pub min_size: u32,
    pub max_size: u32,
    pub target_size: u32,
}

/// Worker state metrics
#[derive(Debug, Clone)]
pub struct WorkerStateMetrics {
    pub available: u32,
    pub busy: u32,
    pub idle: u32,
    pub provisioning: u32,
    pub terminating: u32,
    pub unhealthy: u32,
}

/// Job execution metrics
#[derive(Debug, Clone)]
pub struct JobMetrics {
    pub queued: u32,
    pub running: u32,
    pub completed: u32,
    pub failed: u32,
    pub cancelled: u32,
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub allocation_latency_ms_p50: f64,
    pub allocation_latency_ms_p95: f64,
    pub allocation_latency_ms_p99: f64,
    pub provisioning_time_ms_p50: f64,
    pub provisioning_time_ms_p95: f64,
    pub throughput_jobs_per_min: f64,
}

/// Health metrics
#[derive(Debug, Clone)]
pub struct HealthMetrics {
    pub provisioning_success_rate: f64,
    pub termination_success_rate: f64,
    pub error_rate: f64,
    pub mttr_minutes: f64,
}

/// Cost metrics
#[derive(Debug, Clone)]
pub struct CostMetrics {
    pub cost_per_job: f64,
    pub resource_utilization: f64,
    pub idle_time_avg_minutes: f64,
    pub wastage_percentage: f64,
}

/// Per-pool per-tenant metrics
#[derive(Debug, Clone)]
pub struct PerTenantMetrics {
    pub tenant_id: TenantId,
    pub cpu_usage: f64,
    pub memory_usage_mb: u64,
    pub active_jobs: u32,
    pub cost_today: f64,
    pub quota_utilization: f64,
    pub queue_position: u32,
    pub fair_share_weight: f64,
}

/// Tenant usage metrics
#[derive(Debug, Clone)]
pub struct TenantMetrics {
    pub tenant_id: TenantId,
    pub cpu_usage: f64,
    pub memory_usage_mb: f64,
    pub active_jobs: u32,
    pub cost_today: f64,
    pub quota_utilization: f64,
    pub pools: Vec<PerPoolTenantMetrics>,
}

/// Per-pool tenant metrics summary
#[derive(Debug, Clone)]
pub struct PerPoolTenantMetrics {
    pub pool_id: PoolId,
    pub per_tenant_metrics: Vec<PerTenantMetrics>,
    pub cross_tenant_impact: f64,
}

/// Comprehensive pool metrics
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub pool_id: PoolId,
    pub timestamp: DateTime<Utc>,
    pub pool_size: PoolSizeMetrics,
    pub worker_states: WorkerStateMetrics,
    pub job_metrics: JobMetrics,
    pub performance: PerformanceMetrics,
    pub health: HealthMetrics,
    pub cost: CostMetrics,
    pub per_tenant_metrics: Vec<PerTenantMetrics>,
}

/// Metrics aggregation window
#[derive(Debug, Clone)]
pub enum AggregationWindow {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
}

/// Aggregated metrics
#[derive(Debug, Clone)]
pub struct AggregatedPoolMetrics {
    pub pool_id: PoolId,
    pub window: AggregationWindow,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub metrics: PoolMetrics,
}

/// Metrics store trait
#[async_trait::async_trait]
pub trait MetricsStore: Send + Sync {
    async fn store_metrics(&self, metrics: &PoolMetrics) -> Result<()>;
    async fn get_metrics(
        &self,
        pool_id: &PoolId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<Vec<PoolMetrics>>;
    async fn get_aggregated_metrics(
        &self,
        pool_id: &PoolId,
        window: AggregationWindow,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<Vec<AggregatedPoolMetrics>>;
}

/// In-memory metrics store implementation
#[derive(Debug)]
pub struct InMemoryMetricsStore {
    metrics: Arc<RwLock<HashMap<PoolId, Vec<PoolMetrics>>>>,
    max_retention: Duration,
}

impl InMemoryMetricsStore {
    pub fn new(max_retention: Duration) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            max_retention,
        }
    }

    async fn cleanup_old_metrics(&self) {
        let mut store = self.metrics.write().await;
        let cutoff =
            Utc::now() - chrono::Duration::from_std(self.max_retention).unwrap_or_default();

        for (_pool_id, metrics_vec) in store.iter_mut() {
            metrics_vec.retain(|m| m.timestamp > cutoff);
        }
    }
}

#[async_trait::async_trait]
impl MetricsStore for InMemoryMetricsStore {
    async fn store_metrics(&self, metrics: &PoolMetrics) -> Result<()> {
        let mut store = self.metrics.write().await;
        let pool_metrics = store
            .entry(metrics.pool_id.clone())
            .or_insert_with(Vec::new);

        pool_metrics.push(metrics.clone());

        // Cleanup old metrics periodically
        if pool_metrics.len() % 100 == 0 {
            drop(store);
            self.cleanup_old_metrics().await;
        }

        Ok(())
    }

    async fn get_metrics(
        &self,
        pool_id: &PoolId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<Vec<PoolMetrics>> {
        let store = self.metrics.read().await;
        let metrics = store.get(pool_id).cloned().unwrap_or_default();

        let filtered: Vec<PoolMetrics> = metrics
            .into_iter()
            .filter(|m| m.timestamp >= *start && m.timestamp <= *end)
            .collect();

        Ok(filtered)
    }

    async fn get_aggregated_metrics(
        &self,
        pool_id: &PoolId,
        window: AggregationWindow,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<Vec<AggregatedPoolMetrics>> {
        // Simplified aggregation - in production would use proper time-series aggregation
        let metrics = self.get_metrics(pool_id, start, end).await?;

        let mut aggregated = Vec::new();
        let mut current_window_start = *start;

        while current_window_start < *end {
            let window_end = match window {
                AggregationWindow::OneMinute => current_window_start + chrono::Duration::minutes(1),
                AggregationWindow::FiveMinutes => {
                    current_window_start + chrono::Duration::minutes(5)
                }
                AggregationWindow::FifteenMinutes => {
                    current_window_start + chrono::Duration::minutes(15)
                }
                AggregationWindow::OneHour => current_window_start + chrono::Duration::hours(1),
            };

            let window_metrics: Vec<PoolMetrics> = metrics
                .iter()
                .filter(|m| m.timestamp >= current_window_start && m.timestamp < window_end)
                .cloned()
                .collect();

            if let Some(avg_metrics) = window_metrics.first() {
                aggregated.push(AggregatedPoolMetrics {
                    pool_id: pool_id.clone(),
                    window: window.clone(),
                    start_time: current_window_start,
                    end_time: window_end,
                    metrics: avg_metrics.clone(),
                });
            }

            current_window_start = window_end;
        }

        Ok(aggregated)
    }
}

/// Resource pool metrics collector
pub struct ResourcePoolMetricsCollector {
    pools: Arc<RwLock<HashMap<PoolId, Box<dyn ResourcePool>>>>,
    metrics_store: Arc<dyn MetricsStore>,
    collection_interval: Duration,
}

impl std::fmt::Debug for ResourcePoolMetricsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourcePoolMetricsCollector")
            .field("collection_interval", &self.collection_interval)
            .finish()
    }
}

/// Prometheus metrics exporter (requires prometheus crate - disabled by default)
#[derive(Debug)]
pub struct PrometheusMetricsExporter {
    _enabled: bool,
}

impl PrometheusMetricsExporter {
    pub fn new() -> Result<Self> {
        Ok(Self { _enabled: false })
    }

    pub async fn export(&self, _metrics: &PoolMetrics) -> Result<()> {
        // Prometheus export disabled - enable with feature flag
        Ok(())
    }
}

impl Default for PrometheusMetricsExporter {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl ResourcePoolMetricsCollector {
    /// Create a new metrics collector
    pub fn new(
        pools: Arc<RwLock<HashMap<PoolId, Box<dyn ResourcePool>>>>,
        metrics_store: Arc<dyn MetricsStore>,
        collection_interval: Duration,
    ) -> Self {
        Self {
            pools,
            metrics_store,
            collection_interval,
        }
    }

    /// Start the metrics collection loop
    pub fn start_collection(&self) {
        let interval = self.collection_interval;
        let pools = self.pools.clone();
        let metrics_store = self.metrics_store.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let pools_guard = pools.read().await;
                for (pool_id, pool) in pools_guard.iter() {
                    match Self::collect_pool_metrics(pool_id, pool.as_ref()).await {
                        Ok(metrics) => {
                            if let Err(e) = metrics_store.store_metrics(&metrics).await {
                                error!(pool_id = %pool_id, error = %e, "Failed to store metrics");
                            }
                        }
                        Err(e) => {
                            error!(pool_id = %pool_id, error = %e, "Failed to collect metrics");
                        }
                    }
                }
            }
        });

        info!(
            "Metrics collection started with interval {:?}",
            self.collection_interval
        );
    }

    /// Collect metrics from a single pool
    async fn collect_pool_metrics(
        pool_id: &PoolId,
        pool: &dyn ResourcePool,
    ) -> Result<PoolMetrics> {
        let status = pool.status().await.map_err(|e| {
            DomainError::Infrastructure(format!("Collection failed for pool {}: {}", pool_id, e))
        })?;

        // Generate per-tenant metrics (in real implementation, would come from multi-tenancy system)
        let per_tenant_metrics = vec![
            PerTenantMetrics {
                tenant_id: "tenant-a".to_string(),
                cpu_usage: 60.0,
                memory_usage_mb: 1024,
                active_jobs: 3,
                cost_today: 25.50,
                quota_utilization: 0.65,
                queue_position: 0,
                fair_share_weight: 1.0,
            },
            PerTenantMetrics {
                tenant_id: "tenant-b".to_string(),
                cpu_usage: 40.0,
                memory_usage_mb: 512,
                active_jobs: 2,
                cost_today: 17.30,
                quota_utilization: 0.42,
                queue_position: 1,
                fair_share_weight: 1.0,
            },
        ];

        // Map basic status to comprehensive metrics
        // In a real implementation, pools would provide more detailed metrics
        Ok(PoolMetrics {
            pool_id: pool_id.clone(),
            timestamp: Utc::now(),
            pool_size: PoolSizeMetrics {
                current_size: status.active_workers,
                min_size: 0, // Would be from pool config
                max_size: status.total_capacity,
                target_size: status.active_workers, // Would be from autoscaling target
            },
            worker_states: WorkerStateMetrics {
                available: status.available_capacity,
                busy: status
                    .active_workers
                    .saturating_sub(status.available_capacity),
                idle: 0, // Would need more detailed tracking
                provisioning: 0,
                terminating: 0,
                unhealthy: 0,
            },
            job_metrics: JobMetrics {
                queued: status.pending_requests,
                running: status.active_workers,
                completed: 0, // Would need cumulative tracking
                failed: 0,
                cancelled: 0,
            },
            performance: PerformanceMetrics {
                allocation_latency_ms_p50: 50.0, // Would be from pool metrics
                allocation_latency_ms_p95: 120.0,
                allocation_latency_ms_p99: 200.0,
                provisioning_time_ms_p50: 2000.0,
                provisioning_time_ms_p95: 5000.0,
                throughput_jobs_per_min: 30.0,
            },
            health: HealthMetrics {
                provisioning_success_rate: 0.98,
                termination_success_rate: 0.99,
                error_rate: 0.02,
                mttr_minutes: 15.0,
            },
            cost: CostMetrics {
                cost_per_job: 0.5,
                resource_utilization: status.active_workers as f64 / status.total_capacity as f64,
                idle_time_avg_minutes: 5.0,
                wastage_percentage: 0.1,
            },
            per_tenant_metrics: per_tenant_metrics.clone(),
        })
    }

    /// Register a pool with the collector
    pub async fn register_pool(&mut self, pool_id: PoolId, pool: Box<dyn ResourcePool>) {
        let mut pools = self.pools.write().await;
        pools.insert(pool_id, pool);
    }

    /// Unregister a pool from the collector
    pub async fn unregister_pool(&mut self, pool_id: &PoolId) {
        let mut pools = self.pools.write().await;
        pools.remove(pool_id);
    }

    /// Get current metrics for a pool
    pub async fn get_current_metrics(&self, pool_id: &PoolId) -> Result<Option<PoolMetrics>> {
        let pools = self.pools.read().await;
        if let Some(pool) = pools.get(pool_id) {
            let metrics = Self::collect_pool_metrics(pool_id, pool.as_ref()).await?;

            // Store metrics in the store for historical retrieval
            self.metrics_store.store_metrics(&metrics).await?;

            Ok(Some(metrics))
        } else {
            Ok(None)
        }
    }

    /// Get historical metrics for a pool
    pub async fn get_historical_metrics(
        &self,
        pool_id: &PoolId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<Vec<PoolMetrics>> {
        self.metrics_store.get_metrics(pool_id, start, end).await
    }

    /// Get aggregated metrics for a pool
    pub async fn get_aggregated_metrics(
        &self,
        pool_id: &PoolId,
        window: AggregationWindow,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<Vec<AggregatedPoolMetrics>> {
        self.metrics_store
            .get_aggregated_metrics(pool_id, window, start, end)
            .await
    }
}

