//! Prometheus Metrics Server Tests (US-023)
//!
//! Test suite for Prometheus metrics collection and server implementation.

use hodei_metrics_collection::prometheus::{MetricsServer, MetricsCollector};
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_prometheus_server_startup() {
    // RED: Test that Prometheus server starts and exposes /metrics endpoint
    let server = MetricsServer::new("127.0.0.1", 9091).unwrap();

    let handle = tokio::spawn(async move {
        server.start().await
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test HTTP endpoint availability
    let response = reqwest::get("http://127.0.0.1:9091/metrics").await;
    assert!(response.is_ok());
    assert!(response.unwrap().status().is_success());

    handle.abort();
}

#[test]
fn test_counter_metric_creation() {
    // RED: Test counter metric creation and increment
    let collector = MetricsCollector::new();
    let counter = collector.register_counter("test_counter", "Test counter metric").unwrap();

    counter.inc();
    assert_eq!(counter.get(), 1);

    counter.inc_by(5);
    assert_eq!(counter.get(), 6);
}

#[test]
fn test_gauge_metric_creation() {
    // RED: Test gauge metric creation
    let collector = MetricsCollector::new();
    let gauge = collector.register_gauge("test_gauge", "Test gauge metric").unwrap();

    gauge.set(42.0);
    assert_eq!(gauge.get(), 42.0);

    gauge.inc();
    assert_eq!(gauge.get(), 43.0);

    gauge.dec();
    assert_eq!(gauge.get(), 42.0);
}

#[test]
fn test_histogram_metric_creation() {
    // RED: Test histogram metric creation
    let collector = MetricsCollector::new();
    let histogram = collector.register_histogram(
        "test_histogram",
        "Test histogram metric",
        vec![0.1, 0.5, 1.0, 5.0, 10.0]
    ).unwrap();

    histogram.observe(0.3);
    histogram.observe(0.8);
    histogram.observe(2.5);

    // Histogram should track observations
    assert_eq!(histogram.get_sample_count(), 3);
}

#[tokio::test]
async fn test_metrics_endpoint_response_format() {
    // RED: Test that /metrics endpoint returns Prometheus text format
    let server = MetricsServer::new("127.0.0.1", 9092).unwrap();

    let collector = server.get_collector();
    let counter = collector.register_counter("test_requests_total", "Total test requests").unwrap();
    counter.inc_by(42);

    let handle = tokio::spawn(async move {
        server.start().await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let response = reqwest::get("http://127.0.0.1:9092/metrics").await.unwrap();
    let body = response.text().await.unwrap();

    // Should contain the metric name and value
    assert!(body.contains("test_requests_total"));
    assert!(body.contains("42"));

    handle.abort();
}

#[test]
fn test_business_metrics_registration() {
    // RED: Test business metrics registration
    let collector = MetricsCollector::new();

    // Pipeline success rate
    let pipeline_success = collector.register_counter(
        "pipeline_success_total",
        "Total successful pipeline executions"
    ).unwrap();

    let pipeline_failed = collector.register_counter(
        "pipeline_failed_total",
        "Total failed pipeline executions"
    ).unwrap();

    // Worker utilization
    let worker_utilization = collector.register_gauge(
        "worker_utilization_percent",
        "Current worker utilization percentage"
    ).unwrap();

    // Job queue depth
    let job_queue_depth = collector.register_gauge(
        "job_queue_depth",
        "Current number of jobs in queue"
    ).unwrap();

    // Test metrics work
    pipeline_success.inc_by(100);
    pipeline_failed.inc_by(5);
    worker_utilization.set(75.5);
    job_queue_depth.set(123.0);

    assert_eq!(pipeline_success.get(), 100);
    assert_eq!(pipeline_failed.get(), 5);
    assert_eq!(worker_utilization.get(), 75.5);
    assert_eq!(job_queue_depth.get(), 123.0);
}

#[test]
fn test_technical_metrics_registration() {
    // RED: Test technical resource metrics
    let collector = MetricsCollector::new();

    // CPU metrics
    let cpu_usage = collector.register_gauge(
        "system_cpu_usage_percent",
        "System CPU usage percentage"
    ).unwrap();

    // Memory metrics
    let memory_usage = collector.register_gauge(
        "system_memory_usage_bytes",
        "System memory usage in bytes"
    ).unwrap();

    // Disk metrics
    let disk_usage = collector.register_gauge(
        "system_disk_usage_bytes",
        "System disk usage in bytes"
    ).unwrap();

    // Network metrics
    let network_rx = collector.register_counter(
        "system_network_rx_bytes_total",
        "Total network bytes received"
    ).unwrap();

    let network_tx = collector.register_counter(
        "system_network_tx_bytes_total",
        "Total network bytes transmitted"
    ).unwrap();

    // Test metrics work
    cpu_usage.set(45.2);
    memory_usage.set(2147483648.0); // 2GB
    disk_usage.set(10737418240.0); // 10GB
    network_rx.inc_by(1024000);
    network_tx.inc_by(512000);

    assert_eq!(cpu_usage.get(), 45.2);
    assert_eq!(memory_usage.get(), 2147483648.0);
}
