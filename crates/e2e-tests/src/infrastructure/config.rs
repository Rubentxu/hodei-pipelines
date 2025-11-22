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
