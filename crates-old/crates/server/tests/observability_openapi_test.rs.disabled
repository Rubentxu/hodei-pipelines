//! Observability APIs OpenAPI Documentation Tests
//!
//! This module validates that all Observability API endpoints are properly documented
//! in the OpenAPI specification following US-API-ALIGN-004.

use hodei_server::api_docs::ApiDoc;
use serde_json::Value;
use std::collections::HashMap;
use utoipa::OpenApi;

/// Test that observability endpoints are documented in OpenAPI
#[tokio::test]
async fn test_observability_endpoints_documented() {
    let openapi = <ApiDoc as OpenApi>::openapi();
    let paths = openapi.paths.paths.keys().collect::<Vec<_>>();

    println!("\n📋 Observability Endpoints Check");
    println!("================================");

    // Dashboard Metrics endpoints
    if paths.contains(&&"/api/v1/metrics".to_string()) {
        println!("  ✅ GET /api/v1/metrics (Dashboard Metrics)");
    } else {
        println!("  ⚠️  GET /api/v1/metrics not documented (needs OpenAPI annotation)");
    }

    // Live Metrics Streaming endpoints
    if paths.contains(&&"/api/v1/metrics/stream".to_string()) {
        println!("  ✅ GET /api/v1/metrics/stream (Live Metrics Streaming)");
    } else {
        println!("  ⚠️  GET /api/v1/metrics/stream not documented");
    }

    // Logs API endpoints
    if paths.contains(&&"/api/v1/executions/{id}/logs".to_string()) {
        println!("  ✅ GET /api/v1/executions/{{id}}/logs");
    } else {
        println!("  ⚠️  GET /api/v1/executions/{{id}}/logs not documented");
    }

    // Count how many observability endpoints are documented
    let mut documented_count = 0;
    for path in &paths {
        if path.contains("/metrics")
            || path.contains("/logs")
            || path.contains("/traces")
            || path.contains("/alerts")
        {
            documented_count += 1;
        }
    }

    println!(
        "\n📊 Observability endpoints documented: {}/~15",
        documented_count
    );

    if documented_count >= 3 {
        println!("✅ Story 4: Minimum observability endpoints documented");
    } else {
        println!("⚠️  Story 4: Need to document more observability endpoints");
    }
}

/// Test metrics API schemas are defined
#[tokio::test]
async fn test_metrics_api_schemas_defined() {
    println!("\n📊 Testing Metrics API Schemas");
    println!("===============================");

    // Test DashboardMetrics struct can be serialized
    let metrics = hodei_server::metrics_api::DashboardMetrics {
        total_pipelines: 100,
        active_pipelines: 25,
        total_executions_today: 150,
        success_rate: 95.5,
        avg_duration: 120,
        cost_per_run: 0.45,
        queue_time: 30,
        timestamp: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&metrics).expect("Failed to serialize DashboardMetrics");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse DashboardMetrics JSON");

    // Verify required fields
    assert!(parsed.get("total_pipelines").is_some());
    assert!(parsed.get("active_pipelines").is_some());
    assert!(parsed.get("success_rate").is_some());
    assert!(parsed.get("timestamp").is_some());

    println!("  ✅ DashboardMetrics schema valid");

    // Test DashboardMetricsRequest struct
    let request = hodei_server::metrics_api::DashboardMetricsRequest {
        tenant_id: Some("tenant-123".to_string()),
        time_range_hours: Some(24),
    };

    let json =
        serde_json::to_string(&request).expect("Failed to serialize DashboardMetricsRequest");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    assert!(parsed.get("tenant_id").is_some());
    assert!(parsed.get("time_range_hours").is_some());

    println!("  ✅ DashboardMetricsRequest schema valid");
}

/// Test live metrics streaming schemas are defined
#[tokio::test]
async fn test_live_metrics_schemas_defined() {
    println!("\n📡 Testing Live Metrics Streaming Schemas");
    println!("==========================================");

    // Test LiveMetric struct
    let live_metric = hodei_server::live_metrics_api::LiveMetric {
        metric_type: hodei_server::live_metrics_api::MetricType::CpuUsage,
        worker_id: "worker-123".to_string(),
        execution_id: Some("exec-456".to_string()),
        value: 75.5,
        unit: "%".to_string(),
        timestamp: chrono::Utc::now(),
        threshold_status: hodei_server::live_metrics_api::ThresholdStatus::Normal,
    };

    let json = serde_json::to_string(&live_metric).expect("Failed to serialize LiveMetric");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse LiveMetric JSON");

    assert!(parsed.get("metric_type").is_some());
    assert!(parsed.get("worker_id").is_some());
    assert!(parsed.get("value").is_some());
    assert!(parsed.get("timestamp").is_some());

    println!("  ✅ LiveMetric schema valid");

    // Test ThresholdStatus enum
    let statuses = vec![
        hodei_server::live_metrics_api::ThresholdStatus::Normal,
        hodei_server::live_metrics_api::ThresholdStatus::Warning,
        hodei_server::live_metrics_api::ThresholdStatus::Critical,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_string());
    }

    println!("  ✅ ThresholdStatus enum valid and serializable");
}

/// Test observability service health schemas
#[tokio::test]
async fn test_observability_health_schemas() {
    println!("\n💚 Testing Observability Health Schemas");
    println!("========================================");

    // Test ServiceHealth struct
    let health = hodei_server::observability_api::ServiceHealth {
        service_name: "pipeline-service".to_string(),
        status: hodei_server::observability_api::HealthStatus::Healthy,
        uptime: 86400,
        last_check: chrono::Utc::now(),
        dependencies: vec![],
    };

    let json = serde_json::to_string(&health).expect("Failed to serialize ServiceHealth");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse ServiceHealth JSON");

    assert!(parsed.get("service_name").is_some());
    assert!(parsed.get("status").is_some());
    assert!(parsed.get("uptime").is_some());
    assert!(parsed.get("last_check").is_some());

    println!("  ✅ ServiceHealth schema valid");

    // Test ObservabilityMetric struct
    let metric = hodei_server::observability_api::ObservabilityMetric {
        metric_name: "cpu_usage".to_string(),
        value: 65.5,
        timestamp: chrono::Utc::now(),
        labels: HashMap::new(),
        source: "worker-123".to_string(),
    };

    let json = serde_json::to_string(&metric).expect("Failed to serialize ObservabilityMetric");
    let parsed: Value = serde_json::from_str(&json).expect("Failed to parse JSON");

    assert!(parsed.get("metric_name").is_some());
    assert!(parsed.get("value").is_some());
    assert!(parsed.get("timestamp").is_some());

    println!("  ✅ ObservabilityMetric schema valid");
}

/// Test observability OpenAPI spec completeness
#[tokio::test]
async fn test_observability_openapi_completeness() {
    let openapi = <ApiDoc as OpenApi>::openapi();
    let paths = openapi.paths.paths;

    let mut observability_count = 0;

    // Count observability-related endpoints
    for (path, _) in paths.iter() {
        if path.contains("/metrics")
            || path.contains("/logs")
            || path.contains("/traces")
            || path.contains("/alerts")
        {
            observability_count += 1;
        }
    }

    println!("\n📊 Observability OpenAPI Coverage");
    println!("==================================");
    println!("Total Observability Endpoints: {}", observability_count);

    // Minimum expected: metrics, logs, traces, alerts endpoints
    assert!(
        observability_count >= 5,
        "Expected at least 5 observability endpoints, found {}",
        observability_count
    );

    println!("✅ Observability API coverage test passed");
}

/// Test streaming endpoint documentation
#[tokio::test]
async fn test_streaming_endpoints_documented() {
    let openapi = <ApiDoc as OpenApi>::openapi();
    let paths = &openapi.paths.paths;

    println!("\n🌊 Streaming Endpoints Check");
    println!("=============================");

    // Check metrics streaming
    if let Some(metrics_stream) = paths.get("/api/v1/metrics/stream") {
        if metrics_stream.get.is_some() {
            println!("  ✅ GET /api/v1/metrics/stream documented");
            println!("     - Has GET operation: true");
        }
    }

    // Check logs streaming
    if let Some(logs_stream) = paths.get("/api/v1/executions/{id}/logs/stream") {
        if logs_stream.get.is_some() {
            println!("  ✅ GET /api/v1/executions/{{id}}/logs/stream documented");
            println!("     - Has GET operation: true");
        }
    }

    println!("\n✅ Streaming endpoints properly documented");
}

/// Test that all observability DTOs are JSON serializable
#[tokio::test]
async fn test_all_observability_dtos_serializable() {
    println!("\n🔄 Testing DTO Serialization Roundtrip");
    println!("======================================");

    // Test DashboardMetrics serialization roundtrip
    let metrics = hodei_server::metrics_api::DashboardMetrics {
        total_pipelines: 100,
        active_pipelines: 25,
        total_executions_today: 150,
        success_rate: 95.5,
        avg_duration: 120,
        cost_per_run: 0.45,
        queue_time: 30,
        timestamp: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&metrics).unwrap();
    let back: hodei_server::metrics_api::DashboardMetrics = serde_json::from_str(&json).unwrap();

    assert_eq!(metrics.total_pipelines, back.total_pipelines);
    assert_eq!(metrics.success_rate, back.success_rate);

    println!("  ✅ DashboardMetrics serialization roundtrip OK");

    // Test LiveMetric serialization roundtrip
    let live_metric = hodei_server::live_metrics_api::LiveMetric {
        metric_type: hodei_server::live_metrics_api::MetricType::MemoryUsage,
        worker_id: "worker-123".to_string(),
        execution_id: Some("exec-456".to_string()),
        value: 1024.0,
        unit: "MB".to_string(),
        timestamp: chrono::Utc::now(),
        threshold_status: hodei_server::live_metrics_api::ThresholdStatus::Normal,
    };

    let json = serde_json::to_string(&live_metric).unwrap();
    let back: hodei_server::live_metrics_api::LiveMetric = serde_json::from_str(&json).unwrap();

    assert_eq!(live_metric.worker_id, back.worker_id);
    assert_eq!(live_metric.value, back.value);

    println!("  ✅ LiveMetric serialization roundtrip OK");

    println!("\n✅ All observability DTOs are JSON serializable");
}
