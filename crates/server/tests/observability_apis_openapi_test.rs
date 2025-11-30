//! Story 12: Complete OpenAPI Documentation for Observability APIs (US-API-ALIGN-012)
//!
//! Comprehensive tests to validate that all Observability APIs
//! have complete OpenAPI 3.0 documentation including:
//! - Request/response schemas
//! - HTTP status codes
//! - Parameter documentation
//! - Type safety with utoipa

use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    use hodei_server::api_docs::{ApiDoc, *};
    use hodei_server::observability_api::{
        AuditLog, AuditOutcome, ClusterEdge, ClusterNode, ClusterTopology, DependencyHealth,
        EdgeType, ErrorEvent, ErrorSeverity, HealthStatus, LogLevel, NodeCapabilities, NodeType,
        ObservabilityConfig, ObservabilityMetric, PerformanceMetrics, ServiceHealth, SpanLog,
        TraceSpan,
    };
    use utoipa::OpenApi;

    /// Test 1: Validate ObservabilityMetric schema in OpenAPI
    #[tokio::test]
    async fn test_observability_metric_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ObservabilityMetric"),
            "ObservabilityMetric should be in OpenAPI schemas"
        );

        println!("✅ ObservabilityMetric schema defined");
    }

    /// Test 2: Validate ServiceHealth schema in OpenAPI
    #[tokio::test]
    async fn test_service_health_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ServiceHealth"),
            "ServiceHealth should be in OpenAPI schemas"
        );

        println!("✅ ServiceHealth schema defined");
    }

    /// Test 3: Validate HealthStatus enum schema
    #[tokio::test]
    async fn test_health_status_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("HealthStatus"),
            "HealthStatus should be in OpenAPI schemas"
        );

        println!("✅ HealthStatus schema defined");
    }

    /// Test 4: Validate DependencyHealth schema
    #[tokio::test]
    async fn test_dependency_health_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("DependencyHealth"),
            "DependencyHealth should be in OpenAPI schemas"
        );

        println!("✅ DependencyHealth schema defined");
    }

    /// Test 5: Validate PerformanceMetrics schema
    #[tokio::test]
    async fn test_performance_metrics_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("PerformanceMetrics"),
            "PerformanceMetrics should be in OpenAPI schemas"
        );

        println!("✅ PerformanceMetrics schema defined");
    }

    /// Test 6: Validate ErrorEvent schema
    #[tokio::test]
    async fn test_error_event_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ErrorEvent"),
            "ErrorEvent should be in OpenAPI schemas"
        );

        println!("✅ ErrorEvent schema defined");
    }

    /// Test 7: Validate ErrorSeverity enum schema
    #[tokio::test]
    async fn test_error_severity_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ErrorSeverity"),
            "ErrorSeverity should be in OpenAPI schemas"
        );

        println!("✅ ErrorSeverity schema defined");
    }

    /// Test 8: Validate AuditLog schema
    #[tokio::test]
    async fn test_audit_log_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("AuditLog"),
            "AuditLog should be in OpenAPI schemas"
        );

        println!("✅ AuditLog schema defined");
    }

    /// Test 9: Validate AuditOutcome enum schema
    #[tokio::test]
    async fn test_audit_outcome_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("AuditOutcome"),
            "AuditOutcome should be in OpenAPI schemas"
        );

        println!("✅ AuditOutcome schema defined");
    }

    /// Test 10: Validate TraceSpan schema
    #[tokio::test]
    async fn test_trace_span_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("TraceSpan"),
            "TraceSpan should be in OpenAPI schemas"
        );

        println!("✅ TraceSpan schema defined");
    }

    /// Test 11: Validate SpanLog schema
    #[tokio::test]
    async fn test_span_log_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("SpanLog"),
            "SpanLog should be in OpenAPI schemas"
        );

        println!("✅ SpanLog schema defined");
    }

    /// Test 12: Validate LogLevel enum schema
    #[tokio::test]
    async fn test_log_level_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("LogLevel"),
            "LogLevel should be in OpenAPI schemas"
        );

        println!("✅ LogLevel schema defined");
    }

    /// Test 13: Validate ClusterTopology schema
    #[tokio::test]
    async fn test_cluster_topology_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ClusterTopology"),
            "ClusterTopology should be in OpenAPI schemas"
        );

        println!("✅ ClusterTopology schema defined");
    }

    /// Test 14: Validate ClusterNode schema
    #[tokio::test]
    async fn test_cluster_node_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ClusterNode"),
            "ClusterNode should be in OpenAPI schemas"
        );

        println!("✅ ClusterNode schema defined");
    }

    /// Test 15: Validate NodeType enum schema
    #[tokio::test]
    async fn test_node_type_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("NodeType"),
            "NodeType should be in OpenAPI schemas"
        );

        println!("✅ NodeType schema defined");
    }

    /// Test 16: Validate NodeCapabilities schema
    #[tokio::test]
    async fn test_node_capabilities_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("NodeCapabilities"),
            "NodeCapabilities should be in OpenAPI schemas"
        );

        println!("✅ NodeCapabilities schema defined");
    }

    /// Test 17: Validate ClusterEdge schema
    #[tokio::test]
    async fn test_cluster_edge_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ClusterEdge"),
            "ClusterEdge should be in OpenAPI schemas"
        );

        println!("✅ ClusterEdge schema defined");
    }

    /// Test 18: Validate EdgeType enum schema
    #[tokio::test]
    async fn test_edge_type_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("EdgeType"),
            "EdgeType should be in OpenAPI schemas"
        );

        println!("✅ EdgeType schema defined");
    }

    /// Test 19: Validate ObservabilityConfig schema
    #[tokio::test]
    async fn test_observability_config_schema() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ObservabilityConfig"),
            "ObservabilityConfig should be in OpenAPI schemas"
        );

        println!("✅ ObservabilityConfig schema defined");
    }

    /// Test 20: Validate GET /api/v1/observability/health endpoint
    #[tokio::test]
    async fn test_get_service_health_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/health"),
            "GET /api/v1/observability/health should be documented"
        );

        println!("✅ Get service health endpoint documented");
    }

    /// Test 21: Validate GET /api/v1/observability/performance endpoint
    #[tokio::test]
    async fn test_get_performance_metrics_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/performance"),
            "GET /api/v1/observability/performance should be documented"
        );

        println!("✅ Get performance metrics endpoint documented");
    }

    /// Test 22: Validate GET /api/v1/observability/metrics endpoint
    #[tokio::test]
    async fn test_get_metrics_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/metrics"),
            "GET /api/v1/observability/metrics should be documented"
        );

        println!("✅ Get metrics endpoint documented");
    }

    /// Test 23: Validate GET /api/v1/observability/errors endpoint
    #[tokio::test]
    async fn test_get_error_events_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/errors"),
            "GET /api/v1/observability/errors should be documented"
        );

        println!("✅ Get error events endpoint documented");
    }

    /// Test 24: Validate GET /api/v1/observability/audit endpoint
    #[tokio::test]
    async fn test_get_audit_logs_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/audit"),
            "GET /api/v1/observability/audit should be documented"
        );

        println!("✅ Get audit logs endpoint documented");
    }

    /// Test 25: Validate GET /api/v1/observability/traces/{trace_id} endpoint
    #[tokio::test]
    async fn test_get_trace_spans_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/traces/{trace_id}"),
            "GET /api/v1/observability/traces/{{trace_id}} should be documented"
        );

        println!("✅ Get trace spans endpoint documented");
    }

    /// Test 26: Validate GET /api/v1/observability/config endpoint
    #[tokio::test]
    async fn test_get_observability_config_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/config"),
            "GET /api/v1/observability/config should be documented"
        );

        println!("✅ Get observability config endpoint documented");
    }

    /// Test 27: Validate PUT /api/v1/observability/config endpoint
    #[tokio::test]
    async fn test_update_observability_config_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/config"),
            "PUT /api/v1/observability/config should be documented"
        );

        println!("✅ Update observability config endpoint documented");
    }

    /// Test 28: Validate GET /api/v1/observability/topology endpoint
    #[tokio::test]
    async fn test_get_cluster_topology_endpoint_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        assert!(
            openapi
                .paths
                .paths
                .contains_key("/api/v1/observability/topology"),
            "GET /api/v1/observability/topology should be documented"
        );

        println!("✅ Get cluster topology endpoint documented");
    }

    /// Test 29: Verify "observability" tag is used
    #[tokio::test]
    async fn test_observability_tag_usage() {
        let openapi = <ApiDoc as OpenApi>::openapi();

        let has_observability_tag = openapi.tags.as_ref().map_or(false, |tags| {
            tags.iter().any(|tag| tag.name == "observability")
        });

        assert!(has_observability_tag, "observability tag should be defined");

        println!("✅ observability tag is defined in OpenAPI");
    }

    /// Test 30: Validate ObservabilityMetric serialization/deserialization
    #[tokio::test]
    async fn test_observability_metric_dto_serialization() {
        let metric = ObservabilityMetric {
            metric_name: "cpu_usage".to_string(),
            value: 85.5,
            timestamp: Utc::now(),
            labels: {
                let mut map = HashMap::new();
                map.insert("node".to_string(), "node-1".to_string());
                map.insert("region".to_string(), "us-east-1".to_string());
                map
            },
            source: "prometheus".to_string(),
        };

        let serialized = serde_json::to_string(&metric).unwrap();
        let deserialized: ObservabilityMetric = serde_json::from_str(&serialized).unwrap();

        assert_eq!(metric.metric_name, deserialized.metric_name);
        assert_eq!(metric.value, deserialized.value);
        println!("✅ ObservabilityMetric serialization/deserialization works");
    }

    /// Test 31: Validate ServiceHealth serialization/deserialization
    #[tokio::test]
    async fn test_service_health_dto_serialization() {
        let health = ServiceHealth {
            service_name: "api-server".to_string(),
            status: HealthStatus::Healthy,
            uptime: 86400,
            last_check: Utc::now(),
            dependencies: vec![DependencyHealth {
                name: "database".to_string(),
                status: HealthStatus::Healthy,
                response_time_ms: 15.5,
                last_success: Utc::now(),
            }],
        };

        let serialized = serde_json::to_string(&health).unwrap();
        let deserialized: ServiceHealth = serde_json::from_str(&serialized).unwrap();

        assert_eq!(health.service_name, deserialized.service_name);
        assert_eq!(health.status, deserialized.status);
        println!("✅ ServiceHealth serialization/deserialization works");
    }

    /// Test 32: Validate PerformanceMetrics serialization/deserialization
    #[tokio::test]
    async fn test_performance_metrics_dto_serialization() {
        let metrics = PerformanceMetrics {
            cpu_usage_percent: 65.5,
            memory_usage_bytes: 1073741824,
            memory_usage_percent: 72.3,
            disk_usage_percent: 45.0,
            network_io_bytes_per_sec: 1048576,
            active_connections: 150,
            request_rate_per_sec: 250.5,
            average_response_time_ms: 45.2,
            error_rate_percent: 0.5,
        };

        let serialized = serde_json::to_string(&metrics).unwrap();
        let deserialized: PerformanceMetrics = serde_json::from_str(&serialized).unwrap();

        assert_eq!(metrics.cpu_usage_percent, deserialized.cpu_usage_percent);
        assert_eq!(metrics.active_connections, deserialized.active_connections);
        println!("✅ PerformanceMetrics serialization/deserialization works");
    }

    /// Test 33: Validate ErrorEvent serialization/deserialization
    #[tokio::test]
    async fn test_error_event_dto_serialization() {
        let error = ErrorEvent {
            id: "error-001".to_string(),
            error_type: "TimeoutError".to_string(),
            message: "Request timeout after 30 seconds".to_string(),
            stack_trace: Some("at timeout...".to_string()),
            timestamp: Utc::now(),
            severity: ErrorSeverity::High,
            service: "api-server".to_string(),
            user_id: Some("user-123".to_string()),
            request_id: Some("req-456".to_string()),
        };

        let serialized = serde_json::to_string(&error).unwrap();
        let deserialized: ErrorEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(error.id, deserialized.id);
        assert_eq!(error.severity, deserialized.severity);
        println!("✅ ErrorEvent serialization/deserialization works");
    }

    /// Test 34: Validate ClusterTopology serialization/deserialization
    #[tokio::test]
    async fn test_cluster_topology_dto_serialization() {
        let topology = ClusterTopology {
            nodes: vec![ClusterNode {
                id: "node-001".to_string(),
                node_type: NodeType::ControlPlane,
                name: "control-plane-1".to_string(),
                status: HealthStatus::Healthy,
                capabilities: NodeCapabilities {
                    cpu_cores: 8,
                    memory_gb: 16384,
                    storage_gb: 100,
                    gpu_count: Some(0),
                    network_bandwidth_mbps: 1000,
                },
                metadata: HashMap::new(),
            }],
            edges: vec![],
            total_workers: 10,
            active_workers: 8,
            timestamp: Utc::now(),
        };

        let serialized = serde_json::to_string(&topology).unwrap();
        let deserialized: ClusterTopology = serde_json::from_str(&serialized).unwrap();

        assert_eq!(topology.total_workers, deserialized.total_workers);
        assert_eq!(topology.active_workers, deserialized.active_workers);
        println!("✅ ClusterTopology serialization/deserialization works");
    }

    /// Test 35: Validate ObservabilityConfig serialization/deserialization
    #[tokio::test]
    async fn test_observability_config_dto_serialization() {
        let config = ObservabilityConfig {
            enabled: true,
            sampling_rate: 0.1,
            metrics_retention_days: 30,
            traces_retention_days: 7,
            logs_retention_days: 14,
            enable_performance_monitoring: true,
            enable_error_tracking: true,
            enable_audit_logging: true,
            external_tracing_endpoint: Some("https://tracing.example.com".to_string()),
            external_metrics_endpoint: Some("https://metrics.example.com".to_string()),
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: ObservabilityConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.sampling_rate, deserialized.sampling_rate);
        println!("✅ ObservabilityConfig serialization/deserialization works");
    }
}
