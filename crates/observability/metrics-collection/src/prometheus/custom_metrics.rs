//! Custom Metrics Module
//!
//! Provides metric registration and collection functionality.

use prometheus::{Gauge, Histogram, HistogramOpts, IntCounter, Opts, Registry};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("Metric registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Metric not found: {0}")]
    MetricNotFound(String),

    #[error("Invalid metric configuration: {0}")]
    InvalidConfiguration(String),
}

/// Metrics collector for registering and managing Prometheus metrics
pub struct MetricsCollector {
    registry: Arc<Registry>,
}

impl MetricsCollector {
    /// Create a new metrics collector with default registry
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Registry::new()),
        }
    }

    /// Create a new metrics collector with custom registry
    pub fn with_registry(registry: Registry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Get the underlying registry
    pub fn registry(&self) -> Arc<Registry> {
        Arc::clone(&self.registry)
    }

    /// Register a new counter metric
    pub fn register_counter(&self, name: &str, help: &str) -> Result<IntCounter, MetricsError> {
        let opts = Opts::new(name, help);
        let counter = IntCounter::with_opts(opts)
            .map_err(|e| MetricsError::InvalidConfiguration(e.to_string()))?;

        self.registry
            .register(Box::new(counter.clone()))
            .map_err(|e| MetricsError::RegistrationFailed(e.to_string()))?;

        Ok(counter)
    }

    /// Register a new gauge metric
    pub fn register_gauge(&self, name: &str, help: &str) -> Result<Gauge, MetricsError> {
        let opts = Opts::new(name, help);
        let gauge = Gauge::with_opts(opts)
            .map_err(|e| MetricsError::InvalidConfiguration(e.to_string()))?;

        self.registry
            .register(Box::new(gauge.clone()))
            .map_err(|e| MetricsError::RegistrationFailed(e.to_string()))?;

        Ok(gauge)
    }

    /// Register a new histogram metric
    pub fn register_histogram(
        &self,
        name: &str,
        help: &str,
        buckets: Vec<f64>,
    ) -> Result<Histogram, MetricsError> {
        let opts = HistogramOpts::new(name, help).buckets(buckets);
        let histogram = Histogram::with_opts(opts)
            .map_err(|e| MetricsError::InvalidConfiguration(e.to_string()))?;

        self.registry
            .register(Box::new(histogram.clone()))
            .map_err(|e| MetricsError::RegistrationFailed(e.to_string()))?;

        Ok(histogram)
    }

    /// Gather all metrics in Prometheus text format
    pub fn gather(&self) -> Result<String, MetricsError> {
        use prometheus::Encoder;

        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();

        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|e| MetricsError::RegistrationFailed(format!("Encoding failed: {}", e)))?;

        String::from_utf8(buffer).map_err(|e| {
            MetricsError::RegistrationFailed(format!("UTF-8 conversion failed: {}", e))
        })
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
