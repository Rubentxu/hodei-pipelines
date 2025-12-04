//! Metrics Persistence Service - Bridge between In-Memory Collection and TSDB
//!
//! This service periodically flushes metrics from in-memory collection to TimescaleDB
//! to prevent OOM issues and enable long-term metric storage with retention policies.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info, instrument, warn};

use crate::MetricsTimeseriesRepository;
use crate::observability::metrics_timeseries_repository;

/// Batch size for TSDB persistence
const BATCH_SIZE: usize = 1000;

/// Default flush interval
const DEFAULT_FLUSH_INTERVAL_SECS: u64 = 60;

/// Metrics persistence configuration
#[derive(Debug, Clone)]
pub struct MetricsPersistenceConfig {
    pub flush_interval_secs: u64,
    pub batch_size: usize,
    pub enabled: bool,
}

impl Default for MetricsPersistenceConfig {
    fn default() -> Self {
        Self {
            flush_interval_secs: DEFAULT_FLUSH_INTERVAL_SECS,
            batch_size: BATCH_SIZE,
            enabled: true,
        }
    }
}

/// Service for persisting metrics to TSDB
#[derive(Debug)]
pub struct MetricsPersistenceService {
    config: MetricsPersistenceConfig,
    metrics_repository: Arc<MetricsTimeseriesRepository>,
    pending_metrics: Arc<RwLock<HashMap<String, Vec<PersistableMetric>>>>,
}

/// Internal representation of metrics ready for persistence
#[derive(Debug, Clone)]
struct PersistableMetric {
    time: DateTime<Utc>,
    metric_name: String,
    metric_type: String,
    value: f64,
    labels: HashMap<String, String>,
    tenant_id: Option<String>,
}

impl MetricsPersistenceService {
    /// Create a new metrics persistence service
    #[instrument(skip(metrics_repository))]
    pub fn new(
        config: MetricsPersistenceConfig,
        metrics_repository: Arc<MetricsTimeseriesRepository>,
    ) -> Self {
        info!(
            "Creating MetricsPersistenceService with flush_interval={}s, batch_size={}",
            config.flush_interval_secs, config.batch_size
        );

        Self {
            config,
            metrics_repository,
            pending_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the persistence service background task
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.enabled {
            warn!("Metrics persistence service is disabled");
            return Ok(());
        }

        info!("Starting Metrics Persistence Service");

        // Start the flush loop
        let service_clone = self.clone();
        tokio::spawn(async move {
            service_clone.flush_loop().await;
        });

        Ok(())
    }

    /// Add a metric to the pending queue for persistence
    #[instrument(skip(self))]
    pub async fn queue_metric(
        &self,
        time: DateTime<Utc>,
        metric_name: String,
        metric_type: String,
        value: f64,
        labels: HashMap<String, String>,
        tenant_id: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut pending = self.pending_metrics.write().await;

        let persistable = PersistableMetric {
            time,
            metric_name,
            metric_type,
            value,
            labels,
            tenant_id,
        };

        let pool_id = persistable
            .labels
            .get("pool_id")
            .unwrap_or(&"default".to_string())
            .clone();

        pending
            .entry(pool_id.clone())
            .or_default()
            .push(persistable.clone());

        info!(
            "Queued metric for persistence: {} = {}",
            persistable.metric_name, persistable.value
        );

        // If batch is full, flush immediately
        let current_batch_size = pending.values().map(|v| v.len()).sum::<usize>();

        if current_batch_size >= self.config.batch_size {
            warn!("Batch size limit reached ({})", current_batch_size);
            drop(pending);
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush pending metrics to TSDB
    #[instrument(skip(self))]
    pub async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut pending = self.pending_metrics.write().await;

        if pending.is_empty() {
            return Ok(());
        }

        let metrics_to_persist: Vec<_> = pending
            .values_mut()
            .flat_map(|metrics| metrics.drain(..))
            .collect();

        drop(pending);

        if metrics_to_persist.is_empty() {
            return Ok(());
        }

        info!(
            "Flushing {} metrics to TimescaleDB",
            metrics_to_persist.len()
        );

        let converted_metrics: Vec<metrics_timeseries_repository::MetricDataPoint> =
            metrics_to_persist
                .into_iter()
                .map(|m| metrics_timeseries_repository::MetricDataPoint {
                    time: m.time,
                    metric_name: m.metric_name,
                    metric_type: m.metric_type,
                    value: m.value,
                    labels: m.labels,
                    tenant_id: m.tenant_id,
                })
                .collect();

        self.metrics_repository
            .insert_metrics_batch(&converted_metrics)
            .await
            .map_err(|e| {
                error!("Failed to flush metrics to TSDB: {}", e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;

        info!("Successfully flushed metrics to TimescaleDB");
        Ok(())
    }

    /// Background loop for periodic flushes
    async fn flush_loop(&self) {
        let mut interval = interval(Duration::from_secs(self.config.flush_interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.flush().await {
                error!("Error in flush loop: {}", e);
            }
        }
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Shutting down Metrics Persistence Service");

        // Final flush
        self.flush().await?;

        info!("Metrics Persistence Service shutdown complete");
        Ok(())
    }
}

impl MetricsPersistenceService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            metrics_repository: self.metrics_repository.clone(),
            pending_metrics: self.pending_metrics.clone(),
        }
    }
}

impl Clone for MetricsPersistenceService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            metrics_repository: self.metrics_repository.clone(),
            pending_metrics: self.pending_metrics.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_default_config() {
        let config = MetricsPersistenceConfig::default();

        assert_eq!(config.flush_interval_secs, DEFAULT_FLUSH_INTERVAL_SECS);
        assert_eq!(config.batch_size, BATCH_SIZE);
        assert!(config.enabled);
    }

    #[tokio::test]
    async fn test_queue_metric() {
        let config = MetricsPersistenceConfig::default();
        let repository = Arc::new(MetricsTimeseriesRepository::new(
            sqlx::postgres::PgPool::connect_lazy(
                "postgres://postgres:postgres@localhost:5432/test",
            )
            .unwrap(),
        ));

        let service = MetricsPersistenceService::new(config, repository);

        let mut labels = HashMap::new();
        labels.insert("pool_id".to_string(), "test-pool".to_string());

        let result = service
            .queue_metric(
                Utc::now(),
                "test_metric".to_string(),
                "gauge".to_string(),
                42.0,
                labels,
                None,
            )
            .await;

        assert!(result.is_ok());
    }
}
