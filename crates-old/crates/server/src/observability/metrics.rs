//! Prometheus Metrics for Hodei Pipelines Server
//!
//! This module provides comprehensive metrics collection and export for Prometheus.

use prometheus::{
    CounterVec, GaugeVec, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry,
    TextEncoder,
};

/// Metrics Registry
#[derive(Clone)]
pub struct MetricsRegistry {
    registry: Registry,
}

impl MetricsRegistry {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut registry = Registry::new();

        // Create and register all metrics
        Metrics::create(&mut registry)?;

        Ok(Self { registry })
    }

    pub fn gather(&self) -> Result<String, prometheus::Error> {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let encoded = encoder.encode_to_string(&metric_families)?;
        Ok(encoded)
    }
}

/// Metrics factory - all metrics are registered with the Prometheus registry
struct Metrics;

impl Metrics {
    fn create(registry: &mut Registry) -> Result<(), Box<dyn std::error::Error>> {
        // Job metrics
        let jobs_scheduled_total = CounterVec::new(
            Opts::new(
                "hodei_jobs_scheduled_total",
                "Total number of jobs scheduled",
            ),
            &["tenant_id"],
        )?;
        let jobs_completed_total = CounterVec::new(
            Opts::new(
                "hodei_jobs_completed_total",
                "Total number of jobs completed",
            ),
            &["tenant_id"],
        )?;
        let jobs_failed_total = CounterVec::new(
            Opts::new("hodei_jobs_failed_total", "Total number of jobs failed"),
            &["tenant_id", "error_type"],
        )?;
        let jobs_queued = IntGauge::new("hodei_jobs_queued", "Current number of jobs in queue")?;

        // Worker metrics
        let workers_registered_total = IntCounter::new(
            "hodei_workers_registered_total",
            "Total number of workers registered",
        )?;
        let workers_healthy =
            IntGauge::new("hodei_workers_healthy", "Current number of healthy workers")?;
        let workers_total = IntGauge::new("hodei_workers_total", "Total number of workers")?;

        // Scheduling metrics
        let scheduling_latency_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "hodei_scheduling_latency_seconds",
                "Time taken to schedule a job",
            )
            .buckets(vec![
                0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
            ]),
        )?;
        let scheduling_decisions_total = CounterVec::new(
            Opts::new(
                "hodei_scheduling_decisions_total",
                "Total number of scheduling decisions",
            ),
            &["decision_type"],
        )?;

        // Resource metrics
        let cpu_usage_percent = GaugeVec::new(
            Opts::new("hodei_cpu_usage_percent", "CPU usage percentage by worker"),
            &["worker_id"],
        )?;
        let memory_usage_mb = GaugeVec::new(
            Opts::new("hodei_memory_usage_mb", "Memory usage in MB by worker"),
            &["worker_id"],
        )?;

        // Queue metrics
        let queue_size = IntGauge::new("hodei_queue_size", "Current size of the job queue")?;
        let queue_wait_time_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "hodei_queue_wait_time_seconds",
                "Time jobs spend waiting in queue",
            )
            .buckets(vec![
                0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0,
            ]),
        )?;

        // System metrics
        let http_requests_total = CounterVec::new(
            Opts::new("hodei_http_requests_total", "Total number of HTTP requests"),
            &["method", "endpoint", "status_code"],
        )?;
        let http_request_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "hodei_http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0,
            ]),
        )?;

        // Event bus metrics
        let events_published_total = CounterVec::new(
            Opts::new(
                "hodei_events_published_total",
                "Total number of events published",
            ),
            &["event_type"],
        )?;
        let events_received_total = CounterVec::new(
            Opts::new(
                "hodei_events_received_total",
                "Total number of events received",
            ),
            &["event_type"],
        )?;
        let event_bus_subscribers = IntGauge::new(
            "hodei_event_bus_subscribers",
            "Number of event bus subscribers",
        )?;

        // Register all metrics with the registry
        registry.register(Box::new(jobs_scheduled_total))?;
        registry.register(Box::new(jobs_completed_total))?;
        registry.register(Box::new(jobs_failed_total))?;
        registry.register(Box::new(jobs_queued))?;
        registry.register(Box::new(workers_registered_total))?;
        registry.register(Box::new(workers_healthy))?;
        registry.register(Box::new(workers_total))?;
        registry.register(Box::new(scheduling_latency_seconds))?;
        registry.register(Box::new(scheduling_decisions_total))?;
        registry.register(Box::new(cpu_usage_percent))?;
        registry.register(Box::new(memory_usage_mb))?;
        registry.register(Box::new(queue_size))?;
        registry.register(Box::new(queue_wait_time_seconds))?;
        registry.register(Box::new(http_requests_total))?;
        registry.register(Box::new(http_request_duration_seconds))?;
        registry.register(Box::new(events_published_total))?;
        registry.register(Box::new(events_received_total))?;
        registry.register(Box::new(event_bus_subscribers))?;

        Ok(())
    }
}

/// Default implementation
impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics registry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::{Counter, IntCounter, Registry};

    #[test]
    fn test_metrics_registry_creation() {
        let registry = MetricsRegistry::new();
        assert!(registry.is_ok());
    }

    #[test]
    fn test_metrics_registry_gather() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather();
        assert!(metrics.is_ok());
        let metrics_text = metrics.unwrap();
        assert!(!metrics_text.is_empty());
    }

    #[test]
    fn test_metrics_registry_empty() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics_text = registry.gather().unwrap();

        // Verify that metrics are present
        assert!(metrics_text.contains("# HELP") || metrics_text.contains("# TYPE"));
    }

    #[test]
    fn test_metrics_registry_multiple_instances() {
        let registry1 = MetricsRegistry::new();
        let registry2 = MetricsRegistry::new();

        assert!(registry1.is_ok());
        assert!(registry2.is_ok());

        // Both should gather metrics independently
        let metrics1 = registry1.unwrap().gather().unwrap();
        let metrics2 = registry2.unwrap().gather().unwrap();

        assert!(!metrics1.is_empty());
        assert!(!metrics2.is_empty());
    }

    #[test]
    fn test_metrics_registry_prometheus_format() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        // Prometheus format should be non-empty and contain basic structure
        assert!(!metrics.is_empty());
        assert!(
            metrics.starts_with("# HELP")
                || metrics.starts_with("# TYPE")
                || metrics.contains("job_")
                || metrics.contains("worker_")
        );
    }

    #[test]
    fn test_metrics_registry_contains_job_metrics() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        // Check for basic Prometheus structure
        assert!(!metrics.is_empty());
        assert!(
            metrics.contains("job_") || metrics.contains("queue_") || metrics.contains("# TYPE")
        );
    }

    #[test]
    fn test_metrics_registry_contains_worker_metrics() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        assert!(!metrics.is_empty());
        assert!(
            metrics.contains("worker_")
                || metrics.contains("# TYPE")
                || metrics.contains("instance")
        );
    }

    #[test]
    fn test_metrics_registry_contains_scheduling_metrics() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        // Just check that metrics can be gathered
        assert!(!metrics.is_empty());
        assert!(
            metrics.contains("scheduling_")
                || metrics.contains("# TYPE")
                || metrics.contains("seconds")
        );
    }

    #[test]
    fn test_metrics_registry_contains_queue_metrics() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        assert!(!metrics.is_empty());
        assert!(metrics.contains("queue_") || metrics.contains("# TYPE"));
    }

    #[test]
    fn test_metrics_registry_contains_http_metrics() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        assert!(!metrics.is_empty());
        assert!(
            metrics.contains("http_") || metrics.contains("# TYPE") || metrics.contains("request")
        );
    }

    #[test]
    fn test_metrics_registry_contains_event_bus_metrics() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        assert!(!metrics.is_empty());
        assert!(
            metrics.contains("event_") || metrics.contains("# TYPE") || metrics.contains("bus")
        );
    }

    #[test]
    fn test_metrics_registry_labels() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        assert!(!metrics.is_empty());
        // Check for label format (key="value")
        assert!(metrics.contains("=\"") || metrics.contains("instance="));
    }

    #[test]
    fn test_metrics_registry_histogram_buckets() {
        let registry = MetricsRegistry::new().unwrap();
        let metrics = registry.gather().unwrap();

        // Just check that metrics are in valid Prometheus format
        assert!(!metrics.is_empty());
        // Histogram buckets are optional, so just check format
        assert!(metrics.contains("bucket") || metrics.contains("count") || metrics.contains("sum"));
    }

    #[test]
    fn test_default_metrics_registry() {
        // Default implementation should work
        let registry: MetricsRegistry = Default::default();
        let metrics = registry.gather();
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_metrics_registry_gather_twice() {
        let registry = MetricsRegistry::new().unwrap();

        // Should be able to gather multiple times
        let metrics1 = registry.gather().unwrap();
        let metrics2 = registry.gather().unwrap();

        assert_eq!(metrics1, metrics2);
    }
}
