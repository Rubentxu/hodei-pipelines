//! Prometheus Metrics for Hodei Jobs Server
//!
//! This module provides comprehensive metrics collection and export for Prometheus.

use prometheus::{
    CounterVec, GaugeVec, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry,
    TextEncoder,
};
use std::sync::Arc;

/// Metrics Registry
#[derive(Clone)]
pub struct MetricsRegistry {
    registry: Registry,
    metrics: Arc<Metrics>,
}

impl MetricsRegistry {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut registry = Registry::new();

        // Create metrics
        let metrics = Metrics::create(&mut registry)?;

        Ok(Self {
            registry,
            metrics: Arc::new(metrics),
        })
    }

    pub fn get(&self) -> Arc<Metrics> {
        self.metrics.clone()
    }

    pub fn gather(&self) -> Result<String, prometheus::Error> {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let encoded = encoder.encode_to_string(&metric_families)?;
        Ok(encoded)
    }
}

/// Collection of all metrics
#[derive(Clone)]
pub struct Metrics {
    // Job metrics
    pub jobs_scheduled_total: CounterVec,
    pub jobs_completed_total: CounterVec,
    pub jobs_failed_total: CounterVec,
    pub jobs_queued: IntGauge,

    // Worker metrics
    pub workers_registered_total: IntCounter,
    pub workers_healthy: IntGauge,
    pub workers_total: IntGauge,

    // Scheduling metrics
    pub scheduling_latency_seconds: Histogram,
    pub scheduling_decisions_total: CounterVec,

    // Resource metrics
    pub cpu_usage_percent: GaugeVec,
    pub memory_usage_mb: GaugeVec,

    // Queue metrics
    pub queue_size: IntGauge,
    pub queue_wait_time_seconds: Histogram,

    // System metrics
    pub http_requests_total: CounterVec,
    pub http_request_duration_seconds: Histogram,

    // Event bus metrics
    pub events_published_total: CounterVec,
    pub events_received_total: CounterVec,
    pub event_bus_subscribers: IntGauge,
}

impl Metrics {
    fn create(registry: &mut Registry) -> Result<Self, Box<dyn std::error::Error>> {
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
                "HTTP request duration",
            )
            .buckets(vec![
                0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
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
            "Current number of event bus subscribers",
        )?;

        // Register metrics
        let metrics = Self {
            jobs_scheduled_total: jobs_scheduled_total.clone(),
            jobs_completed_total: jobs_completed_total.clone(),
            jobs_failed_total: jobs_failed_total.clone(),
            jobs_queued: jobs_queued.clone(),
            workers_registered_total: workers_registered_total.clone(),
            workers_healthy: workers_healthy.clone(),
            workers_total: workers_total.clone(),
            scheduling_latency_seconds: scheduling_latency_seconds.clone(),
            scheduling_decisions_total: scheduling_decisions_total.clone(),
            cpu_usage_percent: cpu_usage_percent.clone(),
            memory_usage_mb: memory_usage_mb.clone(),
            queue_size: queue_size.clone(),
            queue_wait_time_seconds: queue_wait_time_seconds.clone(),
            http_requests_total: http_requests_total.clone(),
            http_request_duration_seconds: http_request_duration_seconds.clone(),
            events_published_total: events_published_total.clone(),
            events_received_total: events_received_total.clone(),
            event_bus_subscribers: event_bus_subscribers.clone(),
        };

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

        Ok(metrics)
    }
}

/// Helper functions for metric recording
impl Metrics {
    pub fn record_job_scheduled(&self, tenant_id: &str) {
        self.jobs_scheduled_total
            .with_label_values(&[tenant_id])
            .inc();
    }

    pub fn record_job_completed(&self, tenant_id: &str) {
        self.jobs_completed_total
            .with_label_values(&[tenant_id])
            .inc();
    }

    pub fn record_job_failed(&self, tenant_id: &str, error_type: &str) {
        self.jobs_failed_total
            .with_label_values(&[tenant_id, error_type])
            .inc();
    }

    pub fn set_jobs_queued(&self, count: i64) {
        self.jobs_queued.set(count);
    }

    pub fn increment_workers_registered(&self) {
        self.workers_registered_total.inc();
    }

    pub fn set_workers_healthy(&self, count: i64) {
        self.workers_healthy.set(count);
    }

    pub fn set_workers_total(&self, count: i64) {
        self.workers_total.set(count);
    }

    pub fn record_scheduling_latency(&self, duration_seconds: f64) {
        self.scheduling_latency_seconds.observe(duration_seconds);
    }

    pub fn record_scheduling_decision(&self, decision_type: &str) {
        self.scheduling_decisions_total
            .with_label_values(&[decision_type])
            .inc();
    }

    pub fn set_cpu_usage(&self, worker_id: &str, cpu_percent: f64) {
        self.cpu_usage_percent
            .with_label_values(&[worker_id])
            .set(cpu_percent);
    }

    pub fn set_memory_usage(&self, worker_id: &str, memory_mb: f64) {
        self.memory_usage_mb
            .with_label_values(&[worker_id])
            .set(memory_mb);
    }

    pub fn set_queue_size(&self, size: i64) {
        self.queue_size.set(size);
    }

    pub fn record_queue_wait_time(&self, wait_time_seconds: f64) {
        self.queue_wait_time_seconds.observe(wait_time_seconds);
    }

    pub fn record_http_request(
        &self,
        method: &str,
        endpoint: &str,
        status_code: u16,
        duration_seconds: f64,
    ) {
        self.http_requests_total
            .with_label_values(&[method, endpoint, &status_code.to_string()])
            .inc();
        self.http_request_duration_seconds.observe(duration_seconds);
    }

    pub fn record_event_published(&self, event_type: &str) {
        self.events_published_total
            .with_label_values(&[event_type])
            .inc();
    }

    pub fn record_event_received(&self, event_type: &str) {
        self.events_received_total
            .with_label_values(&[event_type])
            .inc();
    }

    pub fn set_event_bus_subscribers(&self, count: i64) {
        self.event_bus_subscribers.set(count);
    }
}

/// Default implementation
impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics registry")
    }
}
