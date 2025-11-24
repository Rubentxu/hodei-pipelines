//! Configuration management for E2E tests

use serde::{Deserialize, Serialize};
use std::env;

/// Test configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestConfig {
    pub orchestrator_port: u16,
    pub scheduler_port: u16,
    pub worker_manager_port: u16,
    pub prometheus_port: u16,
    pub jaeger_port: u16,
    pub postgres_port: u16,
    pub nats_port: u16,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub log_level: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            orchestrator_port: 8080,
            scheduler_port: 8081,
            worker_manager_port: 8082,
            prometheus_port: 9090,
            jaeger_port: 16686,
            postgres_port: 5432,
            nats_port: 4222,
            timeout_secs: 300,
            max_retries: 3,
            log_level: "info".to_string(),
        }
    }
}

impl TestConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config = TestConfig {
            orchestrator_port: env::var("TEST_ORCHESTRATOR_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse::<u16>()?,
            scheduler_port: env::var("TEST_SCHEDULER_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse::<u16>()?,
            worker_manager_port: env::var("TEST_WORKER_MANAGER_PORT")
                .unwrap_or_else(|_| "8082".to_string())
                .parse::<u16>()?,
            prometheus_port: env::var("TEST_PROMETHEUS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse::<u16>()?,
            jaeger_port: env::var("TEST_JAEGER_PORT")
                .unwrap_or_else(|_| "16686".to_string())
                .parse::<u16>()?,
            postgres_port: env::var("TEST_POSTGRES_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse::<u16>()?,
            nats_port: env::var("TEST_NATS_PORT")
                .unwrap_or_else(|_| "4222".to_string())
                .parse::<u16>()?,
            timeout_secs: env::var("TEST_TIMEOUT_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse::<u64>()?,
            max_retries: env::var("TEST_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse::<u32>()?,
            log_level: env::var("TEST_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        };

        Ok(config)
    }

    pub fn orchestrator_url(&self) -> String {
        format!("http://localhost:{}", self.orchestrator_port)
    }

    pub fn scheduler_url(&self) -> String {
        format!("http://localhost:{}", self.scheduler_port)
    }

    pub fn worker_manager_url(&self) -> String {
        format!("http://localhost:{}", self.worker_manager_port)
    }

    pub fn print(&self) {
        println!("🧪 Test Configuration:");
        println!("  Orchestrator: {}", self.orchestrator_url());
        println!("  Scheduler: {}", self.scheduler_url());
        println!("  Worker Manager: {}", self.worker_manager_url());
        println!("  Timeout: {}s", self.timeout_secs);
        println!("  Max Retries: {}", self.max_retries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TestConfig::default();

        assert_eq!(config.orchestrator_port, 8080);
        assert_eq!(config.scheduler_port, 8081);
        assert_eq!(config.worker_manager_port, 8082);
        assert_eq!(config.prometheus_port, 9090);
        assert_eq!(config.jaeger_port, 16686);
        assert_eq!(config.postgres_port, 5432);
        assert_eq!(config.nats_port, 4222);
        assert_eq!(config.timeout_secs, 300);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_orchestrator_url() {
        let config = TestConfig {
            orchestrator_port: 9090,
            scheduler_port: 8081,
            worker_manager_port: 8082,
            prometheus_port: 9090,
            jaeger_port: 16686,
            postgres_port: 5432,
            nats_port: 4222,
            timeout_secs: 300,
            max_retries: 3,
            log_level: "info".to_string(),
        };

        assert_eq!(config.orchestrator_url(), "http://localhost:9090");
    }

    #[test]
    fn test_scheduler_url() {
        let config = TestConfig {
            orchestrator_port: 8080,
            scheduler_port: 9999,
            worker_manager_port: 8082,
            prometheus_port: 9090,
            jaeger_port: 16686,
            postgres_port: 5432,
            nats_port: 4222,
            timeout_secs: 300,
            max_retries: 3,
            log_level: "info".to_string(),
        };

        assert_eq!(config.scheduler_url(), "http://localhost:9999");
    }

    #[test]
    fn test_worker_manager_url() {
        let config = TestConfig {
            orchestrator_port: 8080,
            scheduler_port: 8081,
            worker_manager_port: 7777,
            prometheus_port: 9090,
            jaeger_port: 16686,
            postgres_port: 5432,
            nats_port: 4222,
            timeout_secs: 300,
            max_retries: 3,
            log_level: "info".to_string(),
        };

        assert_eq!(config.worker_manager_url(), "http://localhost:7777");
    }

    #[test]
    fn test_config_clone() {
        let config = TestConfig::default();
        let cloned = config.clone();

        assert_eq!(config.orchestrator_port, cloned.orchestrator_port);
        assert_eq!(config.scheduler_port, cloned.scheduler_port);
        assert_eq!(config.log_level, cloned.log_level);
    }

    #[test]
    fn test_config_serialization() {
        let config = TestConfig::default();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("orchestrator_port"));
        assert!(json.contains("8080"));
    }

    #[test]
    fn test_config_deserialization() {
        let json = r#"{
            "orchestrator_port": 8080,
            "scheduler_port": 8081,
            "worker_manager_port": 8082,
            "prometheus_port": 9090,
            "jaeger_port": 16686,
            "postgres_port": 5432,
            "nats_port": 4222,
            "timeout_secs": 300,
            "max_retries": 3,
            "log_level": "info"
        }"#;

        let config: TestConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.orchestrator_port, 8080);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_config_url_generation_all_ports() {
        let config = TestConfig {
            orchestrator_port: 1000,
            scheduler_port: 2000,
            worker_manager_port: 3000,
            prometheus_port: 4000,
            jaeger_port: 5000,
            postgres_port: 6000,
            nats_port: 7000,
            timeout_secs: 120,
            max_retries: 5,
            log_level: "trace".to_string(),
        };

        assert_eq!(config.orchestrator_url(), "http://localhost:1000");
        assert_eq!(config.scheduler_url(), "http://localhost:2000");
        assert_eq!(config.worker_manager_url(), "http://localhost:3000");
    }

    #[test]
    fn test_config_extreme_values() {
        let config = TestConfig {
            orchestrator_port: u16::MAX,
            scheduler_port: u16::MAX - 1,
            worker_manager_port: u16::MAX - 2,
            prometheus_port: u16::MAX - 3,
            jaeger_port: u16::MAX - 4,
            postgres_port: u16::MAX - 5,
            nats_port: u16::MAX - 6,
            timeout_secs: u64::MAX,
            max_retries: u32::MAX,
            log_level: "error".to_string(),
        };

        assert_eq!(config.orchestrator_port, u16::MAX);
        assert_eq!(config.timeout_secs, u64::MAX);
        assert_eq!(config.max_retries, u32::MAX);
    }

    #[test]
    fn test_config_zero_values() {
        let config = TestConfig {
            orchestrator_port: 0,
            scheduler_port: 0,
            worker_manager_port: 0,
            prometheus_port: 0,
            jaeger_port: 0,
            postgres_port: 0,
            nats_port: 0,
            timeout_secs: 0,
            max_retries: 0,
            log_level: "".to_string(),
        };

        assert_eq!(config.orchestrator_port, 0);
        assert_eq!(config.timeout_secs, 0);
        assert_eq!(config.max_retries, 0);
    }
}
