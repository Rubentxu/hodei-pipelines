//! Performance Metrics Module
//!
//! Provides performance tracking metrics (latency, throughput, error rates).

use super::MetricsCollector;
use prometheus::{Histogram, IntCounter};
use std::sync::Arc;

/// Performance metrics collector
pub struct PerformanceMetrics {
    // Request metrics
    request_total: IntCounter,
    request_duration: Histogram,

    // Error metrics
    error_total: IntCounter,

    // Business metrics
    jobs_processed: IntCounter,
    jobs_failed: IntCounter,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new(collector: Arc<MetricsCollector>) -> Result<Self, super::custom_metrics::MetricsError> {
        Ok(Self {
            request_total: collector.register_counter(
                "http_requests_total",
                "Total HTTP requests"
            )?,
            request_duration: collector.register_histogram(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]
            )?,
            error_total: collector.register_counter(
                "http_errors_total",
                "Total HTTP errors"
            )?,
            jobs_processed: collector.register_counter(
                "jobs_processed_total",
                "Total jobs processed successfully"
            )?,
            jobs_failed: collector.register_counter(
                "jobs_failed_total",
                "Total jobs that failed"
            )?,
        })
    }

    /// Record a request
    pub fn record_request(&self, duration_secs: f64) {
        self.request_total.inc();
        self.request_duration.observe(duration_secs);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.error_total.inc();
    }

    /// Record job success
    pub fn record_job_success(&self) {
        self.jobs_processed.inc();
    }

    /// Record job failure
    pub fn record_job_failure(&self) {
        self.jobs_failed.inc();
    }
}
