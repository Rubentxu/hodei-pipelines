//! Metrics Collection System Module
//!
//! This module provides real-time metrics collection, historical data storage,
//! and aggregation for auto-scaling decisions.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::error;

/// Core metric types for resource pools
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetricType {
    /// CPU utilization percentage (0.0 - 100.0)
    CpuUtilization,
    /// Memory utilization percentage (0.0 - 100.0)
    MemoryUtilization,
    /// Number of active workers
    ActiveWorkers,
    /// Number of idle workers
    IdleWorkers,
    /// Number of jobs in queue
    QueueLength,
    /// Jobs processed per minute
    JobArrivalRate,
    /// Job processing time in seconds
    JobProcessingTime,
    /// Worker provisioning time in seconds
    WorkerProvisioningTime,
    /// Custom metric with name
    Custom(String),
}

/// Metric value with timestamp
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub metric_type: MetricType,
    pub pool_id: String,
}

/// Aggregation window for metrics
#[derive(Debug, Clone, Copy)]
pub enum AggregationWindow {
    /// 1 minute window
    OneMinute,
    /// 5 minutes window
    FiveMinutes,
    /// 15 minutes window
    FifteenMinutes,
    /// 1 hour window
    OneHour,
    /// Custom window
    Custom(Duration),
}

/// Aggregated metric result
#[derive(Debug, Clone)]
pub struct AggregatedMetric {
    pub metric_type: MetricType,
    pub pool_id: String,
    pub window: AggregationWindow,
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub timestamp: DateTime<Utc>,
}

/// Metrics collection configuration
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub collection_interval: Duration,
    pub retention_period: Duration,
    pub aggregation_intervals: Vec<AggregationWindow>,
    pub enabled_metrics: Vec<MetricType>,
}

/// Metrics collector for resource pools
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    config: MetricsConfig,
    metrics_store: Arc<RwLock<MetricsStore>>,
    aggregation_engine: Arc<RwLock<AggregationEngine>>,
}

/// In-memory metrics storage
#[derive(Debug)]
pub struct MetricsStore {
    metrics: HashMap<String, Vec<MetricValue>>,
    max_retention: Duration,
}

/// Metrics aggregation engine
#[derive(Debug)]
pub struct AggregationEngine {
    aggregators: HashMap<String, Aggregator>,
}

/// Single metric aggregator
#[derive(Debug)]
struct Aggregator {
    window: AggregationWindow,
    values: Vec<MetricValue>,
    last_aggregation: DateTime<Utc>,
}

/// Real-time metrics snapshot
#[derive(Debug, Clone)]
pub struct RealTimeSnapshot {
    pub pool_id: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: HashMap<MetricType, f64>,
}

/// Metrics collection error
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("Invalid metric type: {0}")]
    InvalidMetricType(String),
    #[error("Aggregation error: {0}")]
    AggregationError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config: config.clone(),
            metrics_store: Arc::new(RwLock::new(MetricsStore::new(config.retention_period))),
            aggregation_engine: Arc::new(RwLock::new(AggregationEngine::new())),
        }
    }

    /// Record a metric value
    pub async fn record(&self, metric: MetricValue) -> Result<(), MetricsError> {
        let mut store = self.metrics_store.write().await;
        store.add_metric(metric)
    }

    /// Get real-time snapshot for a pool
    pub async fn get_snapshot(&self, pool_id: &str) -> Result<RealTimeSnapshot, MetricsError> {
        let store = self.metrics_store.read().await;
        store.get_latest_snapshot(pool_id)
    }

    /// Get aggregated metrics for a pool
    pub async fn get_aggregated(
        &self,
        pool_id: &str,
        metric_type: &MetricType,
        window: AggregationWindow,
    ) -> Result<AggregatedMetric, MetricsError> {
        let store = self.metrics_store.read().await;
        store.get_aggregated(pool_id, metric_type, window)
    }

    /// Clean up old metrics
    pub async fn cleanup(&self) -> Result<u64, MetricsError> {
        let mut store = self.metrics_store.write().await;
        store.cleanup()
    }
}

impl MetricsStore {
    pub fn new(max_retention: Duration) -> Self {
        Self {
            metrics: HashMap::new(),
            max_retention,
        }
    }

    pub fn add_metric(&mut self, metric: MetricValue) -> Result<(), MetricsError> {
        let pool_id = metric.pool_id.clone();
        self.metrics
            .entry(pool_id)
            .or_default()
            .push(metric);
        Ok(())
    }

    pub fn get_latest_snapshot(&self, pool_id: &str) -> Result<RealTimeSnapshot, MetricsError> {
        let metrics = self
            .metrics
            .get(pool_id)
            .ok_or_else(|| MetricsError::StorageError(format!("Pool {} not found", pool_id)))?;

        let mut latest_metrics: HashMap<MetricType, f64> = HashMap::new();
        let mut latest_timestamp = chrono::DateTime::<Utc>::MIN_UTC;

        for metric in metrics.iter() {
            if metric.timestamp > latest_timestamp {
                latest_timestamp = metric.timestamp;
                latest_metrics.insert(metric.metric_type.clone(), metric.value);
            }
        }

        Ok(RealTimeSnapshot {
            pool_id: pool_id.to_string(),
            timestamp: latest_timestamp,
            metrics: latest_metrics,
        })
    }

    pub fn get_aggregated(
        &self,
        pool_id: &str,
        metric_type: &MetricType,
        window: AggregationWindow,
    ) -> Result<AggregatedMetric, MetricsError> {
        let metrics = self
            .metrics
            .get(pool_id)
            .ok_or_else(|| MetricsError::StorageError(format!("Pool {} not found", pool_id)))?;

        let values: Vec<f64> = metrics
            .iter()
            .filter(|m| &m.metric_type == metric_type)
            .map(|m| m.value)
            .collect();

        if values.is_empty() {
            return Err(MetricsError::AggregationError(
                "No metrics found for aggregation".to_string(),
            ));
        }

        let count = values.len() as u64;
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg = values.iter().sum::<f64>() / count as f64;

        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50 = percentile(&sorted_values, 50.0);
        let p95 = percentile(&sorted_values, 95.0);
        let p99 = percentile(&sorted_values, 99.0);

        Ok(AggregatedMetric {
            metric_type: metric_type.clone(),
            pool_id: pool_id.to_string(),
            window,
            count,
            min,
            max,
            avg,
            p50,
            p95,
            p99,
            timestamp: Utc::now(),
        })
    }

    pub fn cleanup(&mut self) -> Result<u64, MetricsError> {
        let cutoff = Utc::now() - self.max_retention;
        let mut removed_count = 0usize;

        for (_, pool_metrics) in self.metrics.iter_mut() {
            let before_len = pool_metrics.len();
            pool_metrics.retain(|m| m.timestamp > cutoff);
            removed_count += before_len - pool_metrics.len();
        }

        Ok(removed_count as u64)
    }
}

impl Default for AggregationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregationEngine {
    pub fn new() -> Self {
        Self {
            aggregators: HashMap::new(),
        }
    }
}

/// Calculate percentile from sorted values
fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let idx = (p / 100.0) * (sorted_values.len() - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;

    if lower == upper {
        sorted_values[lower]
    } else {
        let weight = idx - lower as f64;
        sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
    }
}

