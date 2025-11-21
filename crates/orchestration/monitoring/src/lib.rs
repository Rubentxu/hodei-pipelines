//! Performance benchmarking and monitoring (US-006)
//!
//! This module provides comprehensive performance monitoring including:
//! - Metrics collection (counters, gauges, histograms)
//! - Benchmark runner with automated metrics
//! - Prometheus metrics exporter
//! - SLA monitoring and alerting

use hodei_shared_types::{CorrelationId, JobId, TenantId, Uuid};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    pub metrics: HashMap<String, MetricValue>,
    start_time: Instant,
}

#[derive(Debug, Clone)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
    pub metric_type: MetricType,
    pub labels: HashMap<String, String>,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            start_time: Instant::now(),
        }
    }

    /// Record a counter metric
    pub fn increment_counter(
        &mut self,
        name: &str,
        value: f64,
        labels: Option<HashMap<String, String>>,
    ) {
        let labels = labels.unwrap_or_default();
        let key = format!("{}_{:?}", name, labels);

        if let Some(existing) = self.metrics.get_mut(&key) {
            existing.value += value;
            existing.timestamp = Instant::now();
        } else {
            self.metrics.insert(
                key,
                MetricValue {
                    name: name.to_string(),
                    value,
                    metric_type: MetricType::Counter,
                    labels,
                    timestamp: Instant::now(),
                },
            );
        }
    }

    /// Record a gauge metric
    pub fn set_gauge(&mut self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let labels = labels.unwrap_or_default();
        let key = format!("{}_{:?}", name, labels);

        self.metrics.insert(
            key,
            MetricValue {
                name: name.to_string(),
                value,
                metric_type: MetricType::Gauge,
                labels,
                timestamp: Instant::now(),
            },
        );
    }

    /// Record a histogram metric
    pub fn record_histogram(
        &mut self,
        name: &str,
        value: f64,
        labels: Option<HashMap<String, String>>,
    ) {
        let labels = labels.unwrap_or_default();
        let key = format!("{}_{:?}", name, labels);

        self.metrics.insert(
            key,
            MetricValue {
                name: name.to_string(),
                value,
                metric_type: MetricType::Histogram,
                labels,
                timestamp: Instant::now(),
            },
        );
    }

    /// Get all metrics
    pub fn get_metrics(&self) -> Vec<&MetricValue> {
        self.metrics.values().collect()
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Clear all metrics
    pub fn clear(&mut self) {
        self.metrics.clear();
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    metrics: Arc<Mutex<MetricsCollector>>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(MetricsCollector::new())),
        }
    }

    /// Run a benchmark and record results
    pub async fn run_benchmark<F, Fut>(&self, name: &str, operation: F) -> BenchmarkResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future,
    {
        let start = Instant::now();

        operation().await;

        let duration = start.elapsed();
        let mut metrics = self.metrics.lock();

        metrics.record_histogram(
            &format!("{}_duration_ms", name),
            duration.as_millis() as f64,
            None,
        );

        BenchmarkResult {
            name: name.to_string(),
            duration,
            success: true,
        }
    }

    /// Get reference to metrics collector
    pub fn metrics(&self) -> &Arc<Mutex<MetricsCollector>> {
        &self.metrics
    }
}

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    metrics: Arc<Mutex<MetricsCollector>>,
}

impl PrometheusExporter {
    pub fn new(metrics: Arc<Mutex<MetricsCollector>>) -> Self {
        Self { metrics }
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> String {
        let metrics = self.metrics.lock();
        let mut output = String::new();

        for metric in metrics.get_metrics() {
            let labels: Vec<String> = metric
                .labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();

            let labels_str = if !labels.is_empty() {
                format!("{{{}}}", labels.join(","))
            } else {
                String::new()
            };

            output.push_str(&format!(
                "# TYPE {} {}\n{}{} {}\n\n",
                metric.name,
                format!("{:?}", metric.metric_type).to_lowercase(),
                metric.name,
                labels_str,
                metric.value
            ));
        }

        output
    }
}

/// SLA monitor for tracking service level objectives
pub struct SLAMonitor {
    metrics: Arc<Mutex<MetricsCollector>>,
    sla_thresholds: HashMap<String, Threshold>,
}

#[derive(Debug, Clone)]
pub struct Threshold {
    pub target_value: f64,
    pub comparison: Comparison,
}

#[derive(Debug, Clone)]
pub enum Comparison {
    LessThan,
    GreaterThan,
    Equal,
}

impl SLAMonitor {
    pub fn new() -> Self {
        let mut sla_thresholds = HashMap::new();

        // Define default SLAs
        sla_thresholds.insert(
            "job_scheduling_latency_ms".to_string(),
            Threshold {
                target_value: 50.0, // < 50ms
                comparison: Comparison::LessThan,
            },
        );

        sla_thresholds.insert(
            "system_uptime_percent".to_string(),
            Threshold {
                target_value: 99.9, // > 99.9%
                comparison: Comparison::GreaterThan,
            },
        );

        Self {
            metrics: Arc::new(Mutex::new(MetricsCollector::new())),
            sla_thresholds,
        }
    }

    /// Check if SLA is being met
    pub fn check_sla(&self, metric_name: &str) -> SLACheckResult {
        let metrics = self.metrics.lock();

        if let Some(threshold) = self.sla_thresholds.get(metric_name) {
            // Find metric by name (keys may have labels suffix)
            for metric in metrics.metrics.values() {
                if metric.name == metric_name {
                    let meets_sla = match threshold.comparison {
                        Comparison::LessThan => metric.value <= threshold.target_value,
                        Comparison::GreaterThan => metric.value >= threshold.target_value,
                        Comparison::Equal => metric.value == threshold.target_value,
                    };

                    return SLACheckResult {
                        metric_name: metric_name.to_string(),
                        current_value: metric.value,
                        threshold: threshold.target_value,
                        meets_sla,
                    };
                }
            }
        }

        SLACheckResult {
            metric_name: metric_name.to_string(),
            current_value: 0.0,
            threshold: 0.0,
            meets_sla: false,
        }
    }

    /// Record job scheduling latency
    pub fn record_job_scheduling_latency(&self, latency_ms: f64) {
        let mut metrics = self.metrics.lock();
        metrics.record_histogram("job_scheduling_latency_ms", latency_ms, None);
    }

    /// Record worker utilization
    pub fn record_worker_utilization(&self, worker_id: Uuid, utilization: f64) {
        let mut metrics = self.metrics.lock();
        let mut labels = HashMap::new();
        labels.insert("worker_id".to_string(), worker_id.to_string());
        metrics.set_gauge("worker_utilization_percent", utilization, Some(labels));
    }

    /// Record job throughput
    pub fn record_job_throughput(&self, jobs_per_minute: f64) {
        let mut metrics = self.metrics.lock();
        metrics.increment_counter("job_throughput_total", jobs_per_minute, None);
    }

    /// Record system health
    pub fn record_system_health(&self, health_score: f64) {
        let mut metrics = self.metrics.lock();
        metrics.set_gauge("system_health_score", health_score, None);
    }

    /// Get metrics collector reference
    pub fn metrics(&self) -> &Arc<Mutex<MetricsCollector>> {
        &self.metrics
    }
}

#[derive(Debug)]
pub struct SLACheckResult {
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
    pub meets_sla: bool,
}

#[derive(Debug)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub success: bool,
}

/// Performance profiler for detailed analysis
pub struct PerformanceProfiler {
    metrics: Arc<Mutex<MetricsCollector>>,
    profiling_enabled: bool,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(MetricsCollector::new())),
            profiling_enabled: true,
        }
    }

    /// Profile a function call
    pub async fn profile<F, Fut, T>(&self, name: &str, operation: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        if !self.profiling_enabled {
            return operation().await;
        }

        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();

        let mut metrics = self.metrics.lock();
        metrics.record_histogram(
            &format!("{}_profiled_duration_ms", name),
            duration.as_millis() as f64,
            None,
        );

        result
    }

    /// Record memory usage
    pub fn record_memory_usage(&self, bytes: usize) {
        let mut metrics = self.metrics.lock();
        metrics.set_gauge("memory_usage_bytes", bytes as f64, None);
    }

    /// Record CPU utilization
    pub fn record_cpu_utilization(&self, percentage: f64) {
        let mut metrics = self.metrics.lock();
        metrics.set_gauge("cpu_utilization_percent", percentage, None);
    }

    /// Enable/disable profiling
    pub fn set_profiling_enabled(&self, _enabled: bool) {
        // This would need interior mutability in a real implementation
        // Simplified for example purposes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collection() {
        let mut collector = MetricsCollector::new();

        collector.increment_counter("requests_total", 1.0, None);
        collector.set_gauge("memory_usage_mb", 512.0, None);
        collector.record_histogram("response_time_ms", 45.0, None);

        let metrics = collector.get_metrics();
        assert_eq!(metrics.len(), 3);

        // Small delay to ensure uptime is measurable
        std::thread::sleep(Duration::from_millis(10));

        // Check uptime is set
        assert!(collector.uptime().as_millis() > 0);
    }

    #[test]
    fn test_sla_monitoring() {
        let monitor = SLAMonitor::new();

        // Record good latency
        monitor.record_job_scheduling_latency(30.0);

        let result = monitor.check_sla("job_scheduling_latency_ms");
        assert!(result.meets_sla);
        assert_eq!(result.current_value, 30.0);

        // Record bad latency
        monitor.record_job_scheduling_latency(100.0);

        let result = monitor.check_sla("job_scheduling_latency_ms");
        assert!(!result.meets_sla);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Arc::new(Mutex::new(MetricsCollector::new()));
        {
            let mut collector = metrics.lock();
            collector.increment_counter("test_counter", 5.0, None);
            collector.set_gauge("test_gauge", 42.0, None);
        }

        let exporter = PrometheusExporter::new(metrics);
        let output = exporter.export();

        assert!(output.contains("test_counter"));
        assert!(output.contains("test_gauge"));
        assert!(output.contains("counter"));
        assert!(output.contains("gauge"));
    }

    #[tokio::test]
    async fn test_benchmark_runner() {
        let runner = BenchmarkRunner::new();

        let result = runner
            .run_benchmark("test_operation", || async {
                tokio::time::sleep(Duration::from_millis(10)).await;
            })
            .await;

        assert!(result.success);
        assert!(result.duration.as_millis() >= 10);

        let metrics = runner.metrics().lock();
        assert!(metrics.get_metrics().len() > 0);
    }
}
