//! Metrics Time Series Repository
//!
//! Production-ready implementation for persisting metrics in TimescaleDB
//! Applies production-ready optimizations:
//! - Structured logging with tracing
//! - Optimized constants
//! - Transaction consistency
//! - Proper error handling with DomainError
//! - Batch inserts for performance
//! - Automatic retention policies

use chrono::{DateTime, Utc};
use hodei_pipelines_core::DomainError;
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{error, info, instrument};

/// Metric data point
#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    pub time: DateTime<Utc>,
    pub metric_name: String,
    pub metric_type: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub tenant_id: Option<String>,
}

/// Aggregated metric data
#[derive(Debug, Clone)]
pub struct AggregatedMetric {
    pub bucket: DateTime<Utc>,
    pub metric_name: String,
    pub avg_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub sample_count: i64,
}

/// Metric statistics
#[derive(Debug, Clone)]
pub struct MetricStatistics {
    pub metric_name: String,
    pub count: i64,
    pub avg_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Metrics Time Series Repository
/// Implements production-ready persistence for metrics using TimescaleDB
#[derive(Debug, Clone)]
pub struct MetricsTimeseriesRepository {
    pool: PgPool,
}

// Constants for optimization and maintainability
mod constants {
    pub const LOG_PREFIX: &str = "[MetricsTimeseriesRepository]";
    pub const METRIC_INSERT: &str = "INSERT INTO metrics_timeseries (time, metric_name, metric_type, value, labels, tenant_id) VALUES ($1, $2, $3, $4, $5, $6)";
    pub const METRIC_HISTORY: &str = "SELECT time, metric_name, metric_type, value, labels, tenant_id FROM get_metric_history($1, $2, $3, $4, $5)";
    pub const METRIC_AGGREGATED: &str = "SELECT * FROM get_aggregated_metrics($1, $2, $3, $4, $5)";
    pub const METRIC_STATS: &str = "SELECT * FROM get_metric_stats($1, $2, $3, $4)";
    pub const METRIC_LATEST: &str = "SELECT * FROM metrics_latest";
    pub const _METRIC_TOP: &str = "SELECT * FROM metrics_top_values";
    pub const METRIC_DELETE_OLD: &str = "SELECT delete_old_metrics($1)";
    pub const REFRESH_AGGREGATES: &str = "CALL refresh_metrics_aggregates()";
    pub const _RETENTION_DAYS: i32 = 90;
}

impl MetricsTimeseriesRepository {
    /// Create a new metrics time series repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a single metric data point
    #[instrument(skip(self))]
    pub async fn insert_metric(&self, metric: &MetricDataPoint) -> Result<(), DomainError> {
        info!(
            "{} Inserting metric {} = {}",
            constants::LOG_PREFIX,
            metric.metric_name,
            metric.value
        );

        let labels_json = serde_json::to_value(&metric.labels).map_err(|e| {
            error!(
                "{} Failed to serialize labels for metric {}: {}",
                constants::LOG_PREFIX,
                metric.metric_name,
                e
            );
            DomainError::Infrastructure(format!("Failed to serialize labels: {}", e))
        })?;

        sqlx::query(constants::METRIC_INSERT)
            .bind(metric.time)
            .bind(&metric.metric_name)
            .bind(&metric.metric_type)
            .bind(metric.value)
            .bind(labels_json)
            .bind(metric.tenant_id.as_ref())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to insert metric {}: {}",
                    constants::LOG_PREFIX,
                    metric.metric_name,
                    e
                );
                DomainError::Infrastructure(format!(
                    "Failed to insert metric {}: {}",
                    metric.metric_name, e
                ))
            })?;

        Ok(())
    }

    /// Insert multiple metrics in a batch (more efficient)
    #[instrument(skip(self, metrics))]
    pub async fn insert_metrics_batch(
        &self,
        metrics: &[MetricDataPoint],
    ) -> Result<(), DomainError> {
        if metrics.is_empty() {
            return Ok(());
        }

        info!(
            "{} Inserting {} metrics in batch",
            constants::LOG_PREFIX,
            metrics.len()
        );

        let mut tx = self.pool.begin().await.map_err(|e| {
            error!(
                "{} Failed to begin transaction for batch insert: {}",
                constants::LOG_PREFIX,
                e
            );
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        for metric in metrics {
            let labels_json = serde_json::to_value(&metric.labels).map_err(|e| {
                error!(
                    "{} Failed to serialize labels for metric {}: {}",
                    constants::LOG_PREFIX,
                    metric.metric_name,
                    e
                );
                DomainError::Infrastructure(format!("Failed to serialize labels: {}", e))
            })?;

            sqlx::query(constants::METRIC_INSERT)
                .bind(metric.time)
                .bind(&metric.metric_name)
                .bind(&metric.metric_type)
                .bind(metric.value)
                .bind(labels_json)
                .bind(metric.tenant_id.as_ref())
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error!(
                        "{} Failed to insert metric {} in batch: {}",
                        constants::LOG_PREFIX,
                        metric.metric_name,
                        e
                    );
                    DomainError::Infrastructure(format!(
                        "Failed to insert metric {}: {}",
                        metric.metric_name, e
                    ))
                })?;
        }

        tx.commit().await.map_err(|e| {
            error!(
                "{} Failed to commit batch insert transaction: {}",
                constants::LOG_PREFIX,
                e
            );
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        info!(
            "{} Successfully inserted {} metrics in batch",
            constants::LOG_PREFIX,
            metrics.len()
        );

        Ok(())
    }

    /// Get metric history
    #[instrument(skip(self))]
    pub async fn get_metric_history(
        &self,
        metric_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        tenant_id: Option<&str>,
        labels: Option<&HashMap<String, String>>,
    ) -> Result<Vec<MetricDataPoint>, DomainError> {
        info!(
            "{} Getting history for metric {}",
            constants::LOG_PREFIX,
            metric_name
        );

        let labels_json = labels.and_then(|l| serde_json::to_value(l).ok());

        let rows = sqlx::query(constants::METRIC_HISTORY)
            .bind(metric_name)
            .bind(start_time)
            .bind(end_time)
            .bind(tenant_id)
            .bind(labels_json)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query metric history for {}: {}",
                    constants::LOG_PREFIX,
                    metric_name,
                    e
                );
                DomainError::Infrastructure(format!("Failed to query metric history: {}", e))
            })?;

        let mut metrics = Vec::with_capacity(rows.len());

        for row in rows {
            let labels_value: Value = row.get("labels");
            let labels_map: HashMap<String, String> =
                serde_json::from_value(labels_value).unwrap_or_default();

            metrics.push(MetricDataPoint {
                time: row.get("time"),
                metric_name: row.get("metric_name"),
                metric_type: row.get("metric_type"),
                value: row.get("value"),
                labels: labels_map,
                tenant_id: row.get("tenant_id"),
            });
        }

        info!(
            "{} Retrieved {} data points for metric {}",
            constants::LOG_PREFIX,
            metrics.len(),
            metric_name
        );

        Ok(metrics)
    }

    /// Get aggregated metrics over time buckets
    #[instrument(skip(self))]
    pub async fn get_aggregated_metrics(
        &self,
        metric_name: &str,
        bucket_interval: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        tenant_id: Option<&str>,
    ) -> Result<Vec<AggregatedMetric>, DomainError> {
        info!(
            "{} Getting aggregated metrics for {} (interval: {})",
            constants::LOG_PREFIX,
            metric_name,
            bucket_interval
        );

        let rows = sqlx::query(constants::METRIC_AGGREGATED)
            .bind(metric_name)
            .bind(bucket_interval)
            .bind(start_time)
            .bind(end_time)
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query aggregated metrics for {}: {}",
                    constants::LOG_PREFIX,
                    metric_name,
                    e
                );
                DomainError::Infrastructure(format!("Failed to query aggregated metrics: {}", e))
            })?;

        let mut aggregated = Vec::with_capacity(rows.len());

        for row in rows {
            aggregated.push(AggregatedMetric {
                bucket: row.get("bucket"),
                metric_name: row.get("metric_name"),
                avg_value: row.get("avg_value"),
                min_value: row.get("min_value"),
                max_value: row.get("max_value"),
                sample_count: row.get("sample_count"),
            });
        }

        info!(
            "{} Retrieved {} aggregated data points for metric {}",
            constants::LOG_PREFIX,
            aggregated.len(),
            metric_name
        );

        Ok(aggregated)
    }

    /// Get metric statistics
    #[instrument(skip(self))]
    pub async fn get_metric_stats(
        &self,
        metric_name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        tenant_id: Option<&str>,
    ) -> Result<Option<MetricStatistics>, DomainError> {
        info!(
            "{} Getting statistics for metric {}",
            constants::LOG_PREFIX,
            metric_name
        );

        let row = sqlx::query(constants::METRIC_STATS)
            .bind(metric_name)
            .bind(start_time)
            .bind(end_time)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query metric statistics for {}: {}",
                    constants::LOG_PREFIX,
                    metric_name,
                    e
                );
                DomainError::Infrastructure(format!("Failed to query metric statistics: {}", e))
            })?;

        let Some(row) = row else {
            return Ok(None);
        };

        let stats = MetricStatistics {
            metric_name: row.get("metric_name"),
            count: row.get("count"),
            avg_value: row.get("avg_value"),
            min_value: row.get("min_value"),
            max_value: row.get("max_value"),
            p50: row.get("p50"),
            p90: row.get("p90"),
            p95: row.get("p95"),
            p99: row.get("p99"),
        };

        info!(
            "{} Retrieved statistics for metric {} (count: {})",
            constants::LOG_PREFIX,
            metric_name,
            stats.count
        );

        Ok(Some(stats))
    }

    /// Get latest metrics
    #[instrument(skip(self))]
    pub async fn get_latest_metrics(&self) -> Result<Vec<MetricDataPoint>, DomainError> {
        info!("{} Getting latest metrics", constants::LOG_PREFIX);

        let rows = sqlx::query(constants::METRIC_LATEST)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query latest metrics: {}",
                    constants::LOG_PREFIX,
                    e
                );
                DomainError::Infrastructure(format!("Failed to query latest metrics: {}", e))
            })?;

        let mut metrics = Vec::with_capacity(rows.len());

        for row in rows {
            let labels_value: Value = row.get("labels");
            let labels_map: HashMap<String, String> =
                serde_json::from_value(labels_value).unwrap_or_default();

            metrics.push(MetricDataPoint {
                time: row.get("time"),
                metric_name: row.get("metric_name"),
                metric_type: row.get("metric_type"),
                value: row.get("value"),
                labels: labels_map,
                tenant_id: row.get("tenant_id"),
            });
        }

        info!(
            "{} Retrieved {} latest metrics",
            constants::LOG_PREFIX,
            metrics.len()
        );

        Ok(metrics)
    }

    /// Delete old metrics (older than retention period)
    #[instrument(skip(self))]
    pub async fn delete_old_metrics(&self, before_time: DateTime<Utc>) -> Result<u64, DomainError> {
        info!(
            "{} Deleting metrics older than {}",
            constants::LOG_PREFIX,
            before_time
        );

        let row = sqlx::query(constants::METRIC_DELETE_OLD)
            .bind(before_time)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to delete old metrics: {}",
                    constants::LOG_PREFIX,
                    e
                );
                DomainError::Infrastructure(format!("Failed to delete old metrics: {}", e))
            })?;

        let deleted_count: i64 = row.get(0);

        info!(
            "{} Deleted {} old metrics",
            constants::LOG_PREFIX,
            deleted_count
        );

        Ok(deleted_count as u64)
    }

    /// Refresh continuous aggregates
    #[instrument(skip(self))]
    pub async fn refresh_aggregates(&self) -> Result<(), DomainError> {
        info!("{} Refreshing continuous aggregates", constants::LOG_PREFIX);

        sqlx::query(constants::REFRESH_AGGREGATES)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to refresh aggregates: {}",
                    constants::LOG_PREFIX,
                    e
                );
                DomainError::Infrastructure(format!("Failed to refresh aggregates: {}", e))
            })?;

        info!(
            "{} Successfully refreshed aggregates",
            constants::LOG_PREFIX
        );

        Ok(())
    }
}
