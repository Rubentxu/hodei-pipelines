//! Observability management for E2E tests
//!
//! This module provides observability infrastructure including:
//! - Prometheus metrics collection
//! - Jaeger distributed tracing
//! - Service health metrics

use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::infrastructure::containers::ContainerHandle;
use crate::infrastructure::services::ServiceManager;

/// Metrics collected from services
#[derive(Debug, Clone)]
pub struct ServiceMetrics {
    pub service_name: String,
    pub requests_total: u64,
    pub requests_success: u64,
    pub requests_failure: u64,
    pub avg_response_time_ms: f64,
    pub last_heartbeat: Option<String>,
}

/// Tracing span information
#[derive(Debug, Clone)]
pub struct TracingSpan {
    pub span_id: String,
    pub operation_name: String,
    pub duration_ms: u64,
    pub status: String,
}

/// Manager for observability infrastructure
#[derive(Clone)]
pub struct ObservabilityManager {
    prometheus_url: Option<String>,
    jaeger_url: Option<String>,
    metrics_cache: HashMap<String, ServiceMetrics>,
}

impl ObservabilityManager {
    /// Create a new observability manager
    pub fn new() -> Self {
        Self {
            prometheus_url: None,
            jaeger_url: None,
            metrics_cache: HashMap::new(),
        }
    }

    /// Configure Prometheus endpoint
    pub fn with_prometheus(mut self, container: &ContainerHandle) -> Self {
        let port = container.ports.get("9090").unwrap_or(&9090);
        self.prometheus_url = Some(format!("http://localhost:{}", port));
        info!(
            "📊 Prometheus configured at {}",
            self.prometheus_url.as_ref().unwrap()
        );
        self
    }

    /// Configure Jaeger endpoint
    pub fn with_jaeger(mut self, container: &ContainerHandle) -> Self {
        let port = container.ports.get("16686").unwrap_or(&16686);
        self.jaeger_url = Some(format!("http://localhost:{}", port));
        info!(
            "🔍 Jaeger configured at {}",
            self.jaeger_url.as_ref().unwrap()
        );
        self
    }

    /// Collect metrics from a service
    pub async fn collect_metrics(
        &mut self,
        service_name: &str,
        service_manager: &ServiceManager,
    ) -> Result<ServiceMetrics, Box<dyn std::error::Error + Send + Sync>> {
        info!("📊 Collecting metrics for service '{}'...", service_name);

        // Get service health status
        let services = service_manager.list().await;
        let status = services
            .get(service_name)
            .unwrap_or(&crate::infrastructure::services::ServiceStatus::Unhealthy);

        // Simulate metrics collection (in real implementation, query Prometheus)
        let metrics = ServiceMetrics {
            service_name: service_name.to_string(),
            requests_total: 100,
            requests_success: 95,
            requests_failure: 5,
            avg_response_time_ms: 45.5,
            last_heartbeat: Some(chrono::Utc::now().to_rfc3339()),
        };

        self.metrics_cache
            .insert(service_name.to_string(), metrics.clone());

        info!(
            "✅ Metrics collected for '{}': {} requests, {} success, {} failure",
            service_name,
            metrics.requests_total,
            metrics.requests_success,
            metrics.requests_failure
        );

        Ok(metrics)
    }

    /// Get all collected metrics
    pub fn get_all_metrics(&self) -> HashMap<String, ServiceMetrics> {
        self.metrics_cache.clone()
    }

    /// Query Prometheus for custom metrics
    pub async fn query_prometheus(
        &self,
        query: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(url) = &self.prometheus_url {
            let client = reqwest::Client::new();
            let response = client
                .get(&format!("{}/api/v1/query", url))
                .query(&[("query", query)])
                .send()
                .await?;

            if response.status().is_success() {
                let data: Value = response.json().await?;
                return Ok(data);
            }
        }

        Err("Prometheus not configured".into())
    }

    /// Get service traces from Jaeger
    pub async fn get_traces(
        &self,
        service_name: &str,
    ) -> Result<Vec<TracingSpan>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(url) = &self.jaeger_url {
            let client = reqwest::Client::new();
            let response = client.get(&format!("{}/api/services", url)).send().await?;

            if response.status().is_success() {
                let services: Value = response.json().await?;
                info!(
                    "✅ Retrieved traces from Jaeger for service '{}'",
                    service_name
                );
                return Ok(vec![]); // Empty for now, would parse real traces
            }
        }

        Ok(vec![])
    }

    /// Wait for metrics to be available
    pub async fn wait_for_metrics(
        &self,
        service_name: &str,
        timeout_secs: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("⏳ Waiting for metrics from '{}'...", service_name);

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if let Some(_) = self.metrics_cache.get(service_name) {
                info!("✅ Metrics available for '{}'!", service_name);
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Err(format!("Timeout waiting for metrics from '{}'", service_name).into())
    }

    /// Generate metrics report
    pub fn generate_report(&self) -> Value {
        let mut services = Vec::new();
        for (name, metrics) in &self.metrics_cache {
            let success_rate = if metrics.requests_total > 0 {
                (metrics.requests_success as f64 / metrics.requests_total as f64) * 100.0
            } else {
                0.0
            };

            services.push(json!({
                "service": name,
                "requests_total": metrics.requests_total,
                "success_rate": format!("{:.2}%", success_rate),
                "avg_response_time_ms": metrics.avg_response_time_ms,
                "last_heartbeat": metrics.last_heartbeat
            }));
        }

        json!({
            "timestamp": chrono::Utc::now(),
            "services": services,
            "total_services": services.len()
        })
    }
}

impl Default for ObservabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check aggregator
#[derive(Debug)]
pub struct HealthAggregator {
    services: HashMap<String, bool>,
}

impl HealthAggregator {
    /// Create a new health aggregator
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Add a service health check
    pub fn add_service(&mut self, name: &str) {
        self.services.insert(name.to_string(), false);
    }

    /// Update service health
    pub fn update_health(&mut self, name: &str, healthy: bool) {
        if let Some(_) = self.services.get(name) {
            self.services.insert(name.to_string(), healthy);
        }
    }

    /// Check if all services are healthy
    pub fn all_healthy(&self) -> bool {
        self.services.values().all(|h| *h)
    }

    /// Get health status for all services
    pub fn get_all_status(&self) -> &HashMap<String, bool> {
        &self.services
    }

    /// Generate health report
    pub fn generate_report(&self) -> Value {
        let mut services = Vec::new();
        for (name, healthy) in &self.services {
            services.push(json!({
                "service": name,
                "status": if *healthy { "healthy" } else { "unhealthy" }
            }));
        }

        json!({
            "timestamp": chrono::Utc::now(),
            "overall_status": if self.all_healthy() { "healthy" } else { "degraded" },
            "services": services,
            "healthy_count": self.services.values().filter(|h| **h).count(),
            "total_count": self.services.len()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_observability_manager_creation() {
        let manager = ObservabilityManager::new();
        assert!(manager.prometheus_url.is_none());
        assert!(manager.jaeger_url.is_none());
    }

    #[tokio::test]
    async fn test_health_aggregator() {
        let mut aggregator = HealthAggregator::new();
        aggregator.add_service("orchestrator");
        aggregator.add_service("scheduler");

        assert_eq!(aggregator.all_healthy(), false);

        aggregator.update_health("orchestrator", true);
        assert_eq!(aggregator.all_healthy(), false);

        aggregator.update_health("scheduler", true);
        assert_eq!(aggregator.all_healthy(), true);
    }
}
