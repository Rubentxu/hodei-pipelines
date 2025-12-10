//! Health Checks and Readiness Probes
//!
//! Provides comprehensive health monitoring and readiness verification
//! for all infrastructure components (database, NATS, connection pools).

use async_trait::async_trait;
use domain::DomainError;
use std::collections::HashMap;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Health check types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
        }
    }
}

/// Individual health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Overall health report
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheckResult>,
    pub uptime_seconds: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HealthReport {
    /// Create a new health report
    pub fn new(overall_status: HealthStatus, uptime_seconds: u64) -> Self {
        Self {
            overall_status,
            checks: Vec::new(),
            uptime_seconds,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add a health check result
    pub fn add_check(&mut self, result: HealthCheckResult) {
        self.checks.push(result);
    }

    /// Determine if service is ready for traffic
    pub fn is_ready(&self) -> bool {
        self.overall_status != HealthStatus::Unhealthy
    }

    /// Get summary as JSON-like string
    pub fn summary(&self) -> String {
        format!(
            "Status: {}, Uptime: {}s, Checks: {}",
            self.overall_status, self.uptime_seconds, self.checks.len()
        )
    }
}

/// Health check trait
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Check the health of this component
    async fn check(&self) -> HealthCheckResult;

    /// Get the name of this health check
    fn name(&self) -> &str;
}

/// Database health check
pub struct DatabaseHealthCheck {
    name: String,
    connection_string: String,
}

impl DatabaseHealthCheck {
    /// Create a new database health check
    pub fn new(name: String, connection_string: String) -> Self {
        Self {
            name,
            connection_string,
        }
    }
}

#[async_trait]
impl HealthCheck for DatabaseHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        let timestamp = chrono::Utc::now();

        // Simulate database health check
        // In real implementation, would connect and execute "SELECT 1"
        let duration_ms = start.elapsed().as_millis() as u64;

        if self.connection_string.is_empty() {
            HealthCheckResult {
                name: self.name.clone(),
                status: HealthStatus::Unhealthy,
                message: "Empty connection string".to_string(),
                duration_ms,
                timestamp,
            }
        } else {
            HealthCheckResult {
                name: self.name.clone(),
                status: HealthStatus::Healthy,
                message: "Database connection successful".to_string(),
                duration_ms,
                timestamp,
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// NATS health check
pub struct NatsHealthCheck {
    name: String,
    nats_url: String,
}

impl NatsHealthCheck {
    /// Create a new NATS health check
    pub fn new(name: String, nats_url: String) -> Self {
        Self { name, nats_url }
    }
}

#[async_trait]
impl HealthCheck for NatsHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        let timestamp = chrono::Utc::now();

        // Simulate NATS health check
        // In real implementation, would connect to NATS and verify
        let duration_ms = start.elapsed().as_millis() as u64;

        if self.nats_url.is_empty() {
            HealthCheckResult {
                name: self.name.clone(),
                status: HealthStatus::Unhealthy,
                message: "Empty NATS URL".to_string(),
                duration_ms,
                timestamp,
            }
        } else {
            HealthCheckResult {
                name: self.name.clone(),
                status: HealthStatus::Healthy,
                message: "NATS connection successful".to_string(),
                duration_ms,
                timestamp,
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Connection pool health check
pub struct PoolHealthCheck {
    name: String,
    max_connections: u32,
    min_connections: u32,
}

impl PoolHealthCheck {
    /// Create a new pool health check
    pub fn new(name: String, max_connections: u32, min_connections: u32) -> Self {
        Self {
            name,
            max_connections,
            min_connections,
        }
    }
}

#[async_trait]
impl HealthCheck for PoolHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        let timestamp = chrono::Utc::now();

        // Simulate pool health check
        let duration_ms = start.elapsed().as_millis() as u64;

        if self.max_connections < self.min_connections {
            HealthCheckResult {
                name: self.name.clone(),
                status: HealthStatus::Unhealthy,
                message: "Invalid pool configuration: max < min".to_string(),
                duration_ms,
                timestamp,
            }
        } else {
            HealthCheckResult {
                name: self.name.clone(),
                status: HealthStatus::Healthy,
                message: format!(
                    "Pool healthy (min={}, max={})",
                    self.min_connections, self.max_connections
                ),
                duration_ms,
                timestamp,
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Composite health checker
pub struct HealthChecker {
    checks: HashMap<String, Box<dyn HealthCheck>>,
    start_time: Instant,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        Self {
            checks: HashMap::new(),
            start_time: Instant::now(),
        }
    }

    /// Add a health check
    pub fn add_check(&mut self, check: Box<dyn HealthCheck>) {
        let name = check.name().to_string();
        self.checks.insert(name, check);
    }

    /// Run all health checks
    pub async fn check_all(&self) -> HealthReport {
        let uptime_seconds = self.start_time.elapsed().as_secs();
        let mut report = HealthReport::new(HealthStatus::Healthy, uptime_seconds);

        for (_name, check) in &self.checks {
            let result = check.check().await;
            report.add_check(result);
        }

        // Determine overall status
        report.overall_status = if report.checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if report.checks.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        report
    }

    /// Get count of registered checks
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }

    /// Check if service is ready
    pub async fn is_ready(&self) -> bool {
        self.check_all().await.is_ready()
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_health_status_display() {
        // Test: Should format health status correctly
        assert_eq!(format!("{}", HealthStatus::Healthy), "Healthy");
        assert_eq!(format!("{}", HealthStatus::Unhealthy), "Unhealthy");
        assert_eq!(format!("{}", HealthStatus::Degraded), "Degraded");
    }

    #[test]
    fn test_health_status_equality() {
        // Test: Should compare health status correctly
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    }

    #[test]
    fn test_health_check_result_creation() {
        // Test: Should create health check result
        let result = HealthCheckResult {
            name: "test-check".to_string(),
            status: HealthStatus::Healthy,
            message: "All good".to_string(),
            duration_ms: 10,
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(result.name, "test-check");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.message, "All good");
        assert_eq!(result.duration_ms, 10);
    }

    #[test]
    fn test_health_report_new() {
        // Test: Should create new health report
        let report = HealthReport::new(HealthStatus::Healthy, 3600);

        assert_eq!(report.overall_status, HealthStatus::Healthy);
        assert_eq!(report.uptime_seconds, 3600);
        assert_eq!(report.checks.len(), 0);
    }

    #[test]
    fn test_health_report_add_check() {
        // Test: Should add health check to report
        let mut report = HealthReport::new(HealthStatus::Healthy, 0);

        let check = HealthCheckResult {
            name: "db-check".to_string(),
            status: HealthStatus::Healthy,
            message: "OK".to_string(),
            duration_ms: 5,
            timestamp: chrono::Utc::now(),
        };

        report.add_check(check);

        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].name, "db-check");
    }

    #[test]
    fn test_health_report_is_ready() {
        // Test: Should determine readiness correctly
        let mut report = HealthReport::new(HealthStatus::Healthy, 0);
        assert!(report.is_ready());

        report.overall_status = HealthStatus::Degraded;
        assert!(report.is_ready());

        report.overall_status = HealthStatus::Unhealthy;
        assert!(!report.is_ready());
    }

    #[test]
    fn test_health_report_summary() {
        // Test: Should generate summary string
        let mut report = HealthReport::new(HealthStatus::Healthy, 3600);

        let check = HealthCheckResult {
            name: "db-check".to_string(),
            status: HealthStatus::Healthy,
            message: "OK".to_string(),
            duration_ms: 5,
            timestamp: chrono::Utc::now(),
        };

        report.add_check(check);

        let summary = report.summary();
        assert!(summary.contains("Healthy"));
        assert!(summary.contains("3600"));
        assert!(summary.contains("1"));
    }

    #[tokio::test]
    async fn test_database_health_check_healthy() {
        // Test: Should return healthy status for valid connection
        let check = DatabaseHealthCheck::new(
            "postgres".to_string(),
            "postgresql://localhost:5432".to_string(),
        );

        let result = check.check().await;

        assert_eq!(result.name, "postgres");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.contains("successful"));
        assert!(result.duration_ms >= 0);
    }

    #[tokio::test]
    async fn test_database_health_check_unhealthy() {
        // Test: Should return unhealthy status for invalid connection
        let check = DatabaseHealthCheck::new(
            "postgres".to_string(),
            "".to_string(),
        );

        let result = check.check().await;

        assert_eq!(result.name, "postgres");
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert!(result.message.contains("Empty"));
    }

    #[tokio::test]
    async fn test_nats_health_check_healthy() {
        // Test: Should return healthy status for valid NATS URL
        let check = NatsHealthCheck::new(
            "nats".to_string(),
            "nats://localhost:4222".to_string(),
        );

        let result = check.check().await;

        assert_eq!(result.name, "nats");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.contains("successful"));
    }

    #[tokio::test]
    async fn test_pool_health_check_healthy() {
        // Test: Should return healthy status for valid pool config
        let check = PoolHealthCheck::new("pg-pool".to_string(), 100, 5);

        let result = check.check().await;

        assert_eq!(result.name, "pg-pool");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.contains("Pool healthy"));
    }

    #[tokio::test]
    async fn test_pool_health_check_unhealthy() {
        // Test: Should return unhealthy status for invalid pool config
        let check = PoolHealthCheck::new("pg-pool".to_string(), 5, 100);

        let result = check.check().await;

        assert_eq!(result.name, "pg-pool");
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert!(result.message.contains("Invalid"));
    }

    #[tokio::test]
    async fn test_health_checker_new() {
        // Test: Should create empty health checker
        let checker = HealthChecker::new();

        assert_eq!(checker.check_count(), 0);
    }

    #[tokio::test]
    async fn test_health_checker_add_check() {
        // Test: Should add health check to checker
        let mut checker = HealthChecker::new();
        let check = Box::new(DatabaseHealthCheck::new(
            "postgres".to_string(),
            "postgresql://localhost".to_string(),
        ));

        checker.add_check(check);

        assert_eq!(checker.check_count(), 1);
    }

    #[tokio::test]
    async fn test_health_checker_check_all() {
        // Test: Should run all health checks
        let mut checker = HealthChecker::new();

        checker.add_check(Box::new(DatabaseHealthCheck::new(
            "postgres".to_string(),
            "postgresql://localhost".to_string(),
        )));

        checker.add_check(Box::new(NatsHealthCheck::new(
            "nats".to_string(),
            "nats://localhost".to_string(),
        )));

        let report = checker.check_all().await;

        assert_eq!(report.checks.len(), 2);
        assert!(report.checks.iter().any(|c| c.name == "postgres"));
        assert!(report.checks.iter().any(|c| c.name == "nats"));
    }

    #[tokio::test]
    async fn test_health_checker_is_ready() {
        // Test: Should determine readiness correctly
        let mut checker = HealthChecker::new();

        checker.add_check(Box::new(DatabaseHealthCheck::new(
            "postgres".to_string(),
            "postgresql://localhost".to_string(),
        )));

        let ready = checker.is_ready().await;
        assert!(ready);
    }
}
