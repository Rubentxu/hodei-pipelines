//! Metric Type Definitions
//!
//! Common metric types and enumerations.

use serde::{Deserialize, Serialize};

/// Metric types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Counter metric - monotonically increasing value
    Counter,
    /// Gauge metric - can go up or down
    Gauge,
    /// Histogram metric - distribution of values
    Histogram,
    /// Summary metric - quantiles over sliding time window
    Summary,
}

/// Metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Current value
    pub value: f64,
    /// Optional labels
    pub labels: Vec<(String, String)>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MetricValue {
    /// Create a new metric value
    pub fn new(name: String, metric_type: MetricType, value: f64) -> Self {
        Self {
            name,
            metric_type,
            value,
            labels: Vec::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add a label to the metric
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.push((key, value));
        self
    }
}
