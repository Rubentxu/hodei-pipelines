//! Test fixtures for E2E testing
//!
//! This module provides pre-defined test data and configurations that can be
//! reused across multiple test scenarios.

use serde::{Deserialize, Serialize};

/// Standard test pipeline configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPipelines;

impl StandardPipelines {
    /// Simple Rust build pipeline
    pub fn simple_rust_build() -> serde_json::Value {
        serde_json::json!({
            "name": "simple-rust-build",
            "description": "Simple Rust build and test pipeline",
            "stages": [
                {
                    "name": "build",
                    "image": "rust:1.75",
                    "commands": ["cargo build"]
                },
                {
                    "name": "test",
                    "image": "rust:1.75",
                    "commands": ["cargo test"]
                }
            ],
            "environment": {
                "RUST_LOG": "info"
            },
            "triggers": []
        })
    }

    /// Multi-stage deployment pipeline
    pub fn multi_stage_deployment() -> serde_json::Value {
        serde_json::json!({
            "name": "multi-stage-deployment",
            "description": "Deploy application through multiple environments",
            "stages": [
                {
                    "name": "build",
                    "image": "rust:1.75",
                    "commands": ["cargo build --release"]
                },
                {
                    "name": "test",
                    "image": "rust:1.75",
                    "commands": ["cargo test --all-features"]
                },
                {
                    "name": "deploy-staging",
                    "image": "alpine:latest",
                    "commands": ["echo Deploying to staging"]
                },
                {
                    "name": "deploy-production",
                    "image": "alpine:latest",
                    "commands": ["echo Deploying to production"]
                }
            ],
            "environment": {
                "RUST_LOG": "debug",
                "DEPLOY_ENV": "production"
            },
            "triggers": ["git-push"]
        })
    }

    /// Parallel execution pipeline
    pub fn parallel_execution() -> serde_json::Value {
        serde_json::json!({
            "name": "parallel-execution",
            "description": "Pipeline with parallel job execution",
            "stages": [
                {
                    "name": "unit-tests",
                    "image": "rust:1.75",
                    "commands": ["cargo test --lib"]
                },
                {
                    "name": "integration-tests",
                    "image": "rust:1.75",
                    "commands": ["cargo test --test integration"]
                },
                {
                    "name": "linting",
                    "image": "rust:1.75",
                    "commands": ["cargo clippy"]
                }
            ],
            "environment": {
                "RUST_LOG": "info"
            },
            "triggers": []
        })
    }

    /// Failing pipeline for error testing
    pub fn failing_pipeline() -> serde_json::Value {
        serde_json::json!({
            "name": "failing-pipeline",
            "description": "Pipeline that intentionally fails",
            "stages": [
                {
                    "name": "fail-stage",
                    "image": "alpine:latest",
                    "commands": ["exit 1"]
                }
            ],
            "environment": {},
            "triggers": []
        })
    }
}

/// Standard worker configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardWorkers;

impl StandardWorkers {
    /// Rust worker configuration
    pub fn rust_worker() -> serde_json::Value {
        serde_json::json!({
            "type": "rust",
            "config": {
                "image": "rust:1.75",
                "resources": {
                    "cpu": "500m",
                    "memory": "512Mi"
                }
            }
        })
    }

    /// Node.js worker configuration
    pub fn node_worker() -> serde_json::Value {
        serde_json::json!({
            "type": "node",
            "config": {
                "image": "node:18-alpine",
                "resources": {
                    "cpu": "250m",
                    "memory": "256Mi"
                }
            }
        })
    }

    /// Docker worker configuration
    pub fn docker_worker() -> serde_json::Value {
        serde_json::json!({
            "type": "docker",
            "config": {
                "image": "docker:latest",
                "resources": {
                    "cpu": "1000m",
                    "memory": "1Gi"
                }
            }
        })
    }
}

/// Prometheus configuration for test environment
pub const PROMETHEUS_CONFIG: &str = r#"
global:
  scrape_interval: 5s
  evaluation_interval: 5s

scrape_configs:
  - job_name: 'orchestrator'
    static_configs:
      - targets: ['host.docker.internal:8080']
    metrics_path: '/metrics'

  - job_name: 'scheduler'
    static_configs:
      - targets: ['host.docker.internal:8081']
    metrics_path: '/metrics'

  - job_name: 'worker-manager'
    static_configs:
      - targets: ['host.docker.internal:8082']
    metrics_path: '/metrics'

  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']
"#;

/// Jaeger configuration for distributed tracing
pub const JAEGER_CONFIG: &str = r#"
{
  "service_name": "hodei-pipelines",
  "sampler": {
    "type": "const",
    "param": 1
  },
  "reporter": {
    "log_spans": true,
    "local_agent_host_port": "jaeger:6831"
  }
}
"#;

/// Docker Compose configuration for test environment
pub const DOCKER_COMPOSE_CONFIG: &str = r#"
version: '3.8'

services:
  nats:
    image: nats:2.10-alpine
    ports:
      - "4222:4222"
      - "8222:8222"
    command: ["-js", "-DV"]
    healthcheck:
      test: ["CMD", "nats", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: hodei_jobs_test
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U test"]
      interval: 10s
      timeout: 5s
      retries: 5

  prometheus:
    image: prom/prometheus:v2.47.0
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'

  jaeger:
    image: jaegertracing/all-in-one:1.48
    ports:
      - "16686:16686"
      - "14268:14268"
    environment:
      COLLECTOR_OTLP_ENABLED: true

volumes:
  postgres_data:
"#;

/// Test environment variables
#[derive(Debug, Clone)]
pub struct TestEnvVars;

impl TestEnvVars {
    pub fn orchestrator_url() -> String {
        std::env::var("TEST_ORCHESTRATOR_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string())
    }

    pub fn scheduler_url() -> String {
        std::env::var("TEST_SCHEDULER_URL").unwrap_or_else(|_| "http://localhost:8081".to_string())
    }

    pub fn worker_manager_url() -> String {
        std::env::var("TEST_WORKER_MANAGER_URL")
            .unwrap_or_else(|_| "http://localhost:8082".to_string())
    }

    pub fn nats_url() -> String {
        std::env::var("TEST_NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string())
    }

    pub fn postgres_url() -> String {
        std::env::var("TEST_POSTGRES_URL")
            .unwrap_or_else(|_| "postgresql://test:test@localhost:5432/hodei_jobs_test".to_string())
    }
}

/// Expected metrics for validation
pub const EXPECTED_METRICS: &[&str] = &[
    "jobs_total",
    "jobs_created_total",
    "jobs_completed_total",
    "jobs_failed_total",
    "workers_total",
    "workers_active",
    "pipeline_executions_total",
    "pipeline_duration_seconds",
];

/// Expected traces for validation
pub const EXPECTED_SPANS: &[&str] = &[
    "orchestrator.create_pipeline",
    "orchestrator.create_job",
    "scheduler.schedule_job",
    "worker.execute_job",
    "worker_manager.start_worker",
];

/// Sample test data for various scenarios
#[derive(Debug)]
pub struct SampleData;

impl SampleData {
    /// Returns sample job IDs for testing
    pub fn job_ids() -> Vec<String> {
        (0..10).map(|i| format!("job-{:03}", i)).collect()
    }

    /// Returns sample pipeline IDs for testing
    pub fn pipeline_ids() -> Vec<String> {
        (0..5).map(|i| format!("pipeline-{:03}", i)).collect()
    }

    /// Returns sample worker types
    pub fn worker_types() -> Vec<&'static str> {
        vec!["rust", "node", "docker", "python", "go"]
    }

    /// Returns test environment configuration
    pub fn test_config() -> serde_json::Value {
        serde_json::json!({
            "timeout": 300,
            "retries": 3,
            "parallel_jobs": 5,
            "log_level": "info",
            "metrics_enabled": true,
            "tracing_enabled": true
        })
    }
}
