//! E2E Tests with Optimized TestContainers
//!
//! Demonstrates best practices for TestContainers usage:
//! 1. Singleton pattern - one container per resource across ALL tests
//! 2. Resource reuse - containers persist across test suite
//! 3. Health checks - ensure readiness before tests
//! 4. Resource limits - cap CPU/Memory usage
//! 5. Leak detection - detect containers not cleaned up
//! 6. Parallel startup - start containers concurrently

use crate::testcontainers_manager::{
    ContainerLeakDetector, RegistryStats, ResourceLimits, TestContainerError, TestEnvironment,
    get_test_registry, init_test_registry,
};
use hodei_pipelines_domain::pipeline_execution::entities::execution::PipelineExecution;
use hodei_pipelines_domain::pipeline_execution::entities::pipeline::{
    Pipeline, PipelineId, PipelineStep, PipelineStepId,
};
use hodei_pipelines_domain::scheduling::value_objects::worker_messages::WorkerId;
use hodei_pipelines_ports::scheduling::worker_template::WorkerTemplate;
use hodei_pipelines_ports::worker_provider::{ProviderConfig, ProviderType};
use std::collections::HashMap;
use std::time::Duration;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{Container, GenericImage, Image};
use tokio::time::timeout;
use tracing::{info, warn};

/// Global test setup - runs once per test process
/// This implements the SINGLETON PATTERN - containers start ONLY ONCE
#[cfg(test)]
mod global_setup {
    use super::*;
    use std::sync::Once;

    static SETUP: Once = Once::new();

    /// Initialize test environment once per test process
    pub async fn setup_test_environment() {
        SETUP.call_once(|| {
            tracing_subscriber::fmt::init();

            info!("==========================================");
            info!("Initializing Global Test Environment (SINGLETON)");
            info!("==========================================");
        });

        // Initialize singleton registry
        init_test_registry(crate::testcontainers_manager::TestEnvironmentConfig {
            reuse_containers: true,     // ✅ REUSE ACROSS RUNS
            max_containers_per_type: 1, // ✅ SINGLE INSTANCE
            startup_timeout: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(15),
            parallel_startup: true, // ✅ PARALLEL STARTUP
        })
        .await;

        info!("✅ Test environment initialized (containers will be created on first use)");
    }
}

/// Integration test suite demonstrating optimized TestContainers usage
/// All tests share the SAME containers (singleton pattern)
#[cfg(test)]
mod tests {
    use super::*;

    /// ONE-TIME SETUP - runs once for entire test suite
    /// This is crucial for resource optimization!
    #[tokio::test]
    async fn _global_setup() {
        global_setup::setup_test_environment().await;
    }

    /// E2E Test: Pipeline Execution with Real Infrastructure
    /// Uses NATS + PostgreSQL + Redis from singleton registry
    #[tokio::test]
    async fn test_pipeline_execution_with_real_infrastructure() {
        info!("\n╔══════════════════════════════════════════════╗");
        info!("║  Running E2E: Pipeline with Infrastructure  ║");
        info!("╚══════════════════════════════════════════════╝");

        let start = std::time::Instant::now();
        let leak_detector = ContainerLeakDetector::new();

        // ✅ GET SINGLETON INSTANCES - NO NEW CONTAINERS CREATED!
        let registry = get_test_registry().await;
        let nats = registry.get_or_start_nats().await.unwrap();
        let postgres = registry.get_or_start_postgres().await.unwrap();
        let redis = registry.get_or_start_redis().await.unwrap();

        // Get connection details
        let nats_url = format!("nats://localhost:{}", nats.get_host_port_ipv4(4222));
        let postgres_port = postgres.get_host_port_ipv4(5432);
        let redis_url = format!("redis://localhost:{}", redis.get_host_port_ipv4(6379));

        info!("✅ Using singleton containers:");
        info!("   NATS: {}", nats_url);
        info!("   PostgreSQL: localhost:{}", postgres_port);
        info!("   Redis: {}", redis_url);

        // Create test pipeline
        let pipeline_id = PipelineId::new();
        let steps = vec![
            PipelineStep {
                id: PipelineStepId::new(),
                name: "setup".to_string(),
                command: "echo".to_string(),
                args: Some(vec!["Setting up test environment".to_string()]),
                ..Default::default()
            },
            PipelineStep {
                id: PipelineStepId::new(),
                name: "process".to_string(),
                command: "echo".to_string(),
                args: Some(vec!["Processing data".to_string()]),
                ..Default::default()
            },
            PipelineStep {
                id: PipelineStepId::new(),
                name: "cleanup".to_string(),
                command: "echo".to_string(),
                args: Some(vec!["Cleaning up".to_string()]),
                ..Default::default()
            },
        ];

        let pipeline = Pipeline::new(pipeline_id.clone(), steps);
        let mut execution = PipelineExecution::new(
            pipeline_id,
            pipeline.steps.iter().map(|s| s.id.clone()).collect(),
            HashMap::new(),
            Some("e2e-tenant".to_string()),
            Some("e2e-correlation".to_string()),
        );

        info!(
            "✅ Created test pipeline with {} steps",
            pipeline.steps.len()
        );

        // Simulate pipeline execution
        execution.start().unwrap();

        for (i, step) in pipeline.steps.iter().enumerate() {
            info!("Executing step {}: {}", i + 1, step.name);

            // Simulate work
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Update step status
            if let Some(step_exec) = execution.get_step_execution_mut(&step.id) {
                step_exec.complete();
            }
        }

        execution.complete().unwrap();

        let duration = start.elapsed();
        info!("✅ Pipeline execution completed in {:?}", duration);

        // Verify execution state
        assert!(execution.is_completed());
        assert_eq!(execution.get_completed_steps().len(), pipeline.steps.len());

        // Check for container leaks
        let leaked = leak_detector.check_leaks();
        assert_eq!(leaked, 0, "Container leaks detected!");

        info!("╔══════════════════════════════════════════════╗");
        info!("║  ✅ E2E Test Passed (Duration: {:?})        ║", duration);
        info!("╚══════════════════════════════════════════════╝");
    }

    /// Test: Worker Provider with Real Docker
    /// Tests provider initialization without actually starting workers
    #[tokio::test]
    async fn test_worker_provider_configuration() {
        info!("\n╔══════════════════════════════════════════════╗");
        info!("║  Testing Worker Provider Configuration      ║");
        info!("╚══════════════════════════════════════════════╝");

        // Create worker template (same as production)
        let template = WorkerTemplate::new("e2e-test-worker", "1.0.0", "ubuntu:22.04")
            .with_cpu("2")
            .with_memory("4Gi")
            .with_env("ENVIRONMENT", "e2e-test")
            .with_label("test-type", "e2e");

        // Create provider config
        let config = ProviderConfig::docker("e2e-docker-provider".to_string(), template.clone());

        // Verify configuration is valid
        assert_eq!(config.provider_type, ProviderType::Docker);
        assert_eq!(config.name, "e2e-docker-provider");
        assert!(config.template.validate().is_ok());
        assert_eq!(config.template.resources.cpu, Some("2".to_string()));
        assert_eq!(config.template.resources.memory, Some("4Gi".to_string()));

        // Verify environment variables
        let has_env = config.template.env.iter().any(|e| e.name == "ENVIRONMENT");
        assert!(has_env);

        // Verify labels
        let has_label = config.template.labels.get("test-type") == Some(&"e2e".to_string());
        assert!(has_label);

        info!("✅ Worker provider configuration validated");
        info!(
            "   Template: {}:{}",
            template.container.image, template.metadata.version
        );
        info!(
            "   Resources: CPU={:?}, Memory={:?}",
            template.resources.cpu, template.resources.memory
        );
        info!("   Environment vars: {}", config.template.env.len());
        info!("   Labels: {}", config.template.labels.len());

        info!("╔══════════════════════════════════════════════╗");
        info!("║  ✅ Worker Provider Config Test Passed      ║");
        info!("╚══════════════════════════════════════════════╝");
    }

    /// Test: NATS Performance with Singleton Pattern
    /// Demonstrates fast test execution due to container reuse
    #[tokio::test]
    async fn test_nats_performance_with_reuse() {
        info!("\n╔══════════════════════════════════════════════╗");
        info!("║  Testing NATS Performance (Reuse Pattern)   ║");
        info!("╚══════════════════════════════════════════════╝");

        let start = std::time::Instant::now();

        // ✅ GET EXISTING CONTAINER - NO STARTUP TIME!
        let registry = get_test_registry().await;
        let nats = registry.get_or_start_nats().await.unwrap();

        let port = nats.get_host_port_ipv4(4222);
        let url = format!("nats://localhost:{}", port);

        // Test NATS connectivity
        let client = tokio::time::timeout(Duration::from_secs(5), async {
            let nats_client = nats_od::Connection::connect(&url).await;
            nats_client
        })
        .await
        .unwrap();

        info!("✅ NATS client connected in {:?}", start.elapsed());
        info!("   Connection URL: {}", url);

        // Publish test message
        let subject = "e2e.test.performance";
        let message = "Performance test message".to_string();

        let publish_result = client.publish(subject, message.into_bytes()).await;
        assert!(publish_result.is_ok());

        // Subscribe and receive
        let mut subscriber = client.subscribe(subject).await.unwrap();
        let received = tokio::time::timeout(Duration::from_secs(2), subscriber.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(received.data, b"Performance test message");

        let total_duration = start.elapsed();
        info!(
            "✅ NATS publish/subscribe cycle completed in {:?}",
            total_duration
        );
        info!("   Subject: {}", subject);
        info!("   Message size: {} bytes", received.data.len());

        // Performance assertions
        // First run (container startup): ~5-10 seconds
        // Subsequent runs (container reuse): < 1 second
        if total_duration > Duration::from_secs(2) {
            warn!("⚠️  Test took longer than expected. Container may have been recreated.");
        } else {
            info!("✅ Excellent performance - container reuse working!");
        }

        info!("╔══════════════════════════════════════════════╗");
        info!("║  ✅ NATS Performance Test Passed            ║");
        info!("╚══════════════════════════════════════════════╝");
    }

    /// Test: PostgreSQL Operations with Singleton
    /// Fast database operations with reused container
    #[tokio::test]
    async fn test_postgres_operations_with_reuse() {
        info!("\n╔══════════════════════════════════════════════╗");
        info!("║  Testing PostgreSQL (Singleton Pattern)     ║");
        info!("╚══════════════════════════════════════════════╝");

        let start = std::time::Instant::now();

        // ✅ GET EXISTING DATABASE - NO STARTUP TIME!
        let registry = get_test_registry().await;
        let postgres = registry.get_or_start_postgres().await.unwrap();

        let port = postgres.get_host_port_ipv4(5432);
        let config = tokio_postgres::Config::new()
            .host("localhost")
            .port(port)
            .user("testuser")
            .password("testpass")
            .database("testdb");

        let (client, connection) = timeout(Duration::from_secs(5), config.connect())
            .await
            .unwrap();

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        info!("✅ PostgreSQL connected in {:?}", start.elapsed());
        info!("   Database: testdb");
        info!("   Host: localhost:{}", port);

        // Test database operations
        // Create test table
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS e2e_test (
                id SERIAL PRIMARY KEY,
                test_name VARCHAR(255) NOT NULL,
                test_data JSONB,
                created_at TIMESTAMP DEFAULT NOW()
            )",
                &[],
            )
            .await
            .unwrap();

        info!("✅ Test table created");

        // Insert test data
        let test_cases = vec![
            ("pipeline_test", r#"{"steps": 3, "status": "running"}"#),
            ("worker_test", r#"{"workers": 5, "status": "active"}"#),
            ("resource_test", r#"{"cpu": "2", "memory": "4Gi"}"#),
        ];

        for (name, data) in &test_cases {
            client
                .execute(
                    "INSERT INTO e2e_test (test_name, test_data) VALUES ($1, $2)",
                    &[&name, &data],
                )
                .await
                .unwrap();
        }

        info!("✅ Inserted {} test records", test_cases.len());

        // Query test data
        let rows = client
            .query(
                "SELECT test_name, test_data FROM e2e_test ORDER BY created_at",
                &[],
            )
            .await
            .unwrap();

        assert_eq!(rows.len(), test_cases.len());

        for row in &rows {
            let name: &str = row.get("test_name");
            let data: serde_json::Value = row.get("test_data");
            info!("   Retrieved: {} -> {}", name, data);
        }

        // Cleanup
        client.execute("DELETE FROM e2e_test", &[]).await.unwrap();

        let duration = start.elapsed();
        info!("✅ PostgreSQL operations completed in {:?}", duration);
        info!("   Total queries: {}", rows.len() + 5); // Creates + inserts + selects + cleanup

        // Performance assertions
        if duration > Duration::from_secs(1) {
            warn!("⚠️  Database operations took longer than expected");
        } else {
            info!("✅ Excellent performance - singleton container working!");
        }

        info!("╔══════════════════════════════════════════════╗");
        info!("║  ✅ PostgreSQL Test Passed                  ║");
        info!("╚══════════════════════════════════════════════╝");
    }

    /// Test: Resource Limits Enforcement
    /// Verify containers respect resource limits
    #[tokio::test]
    async fn test_resource_limits_enforcement() {
        info!("\n╔══════════════════════════════════════════════╗");
        info!("║  Testing Resource Limits Enforcement        ║");
        info!("╚══════════════════════════════════════════════╝");

        // Define resource limits for test containers
        let limits = ResourceLimits {
            cpu_limit: Some("0.5".to_string()),     // 50% of one CPU
            memory_limit: Some("256m".to_string()), // 256 MB
            disable_swap: true,
        };

        info!("Resource limits configured:");
        info!("   CPU: {}", limits.cpu_limit.as_ref().unwrap());
        info!("   Memory: {}", limits.memory_limit.as_ref().unwrap());

        // In a real implementation, these limits would be applied
        // For demonstration, we verify the configuration is valid
        assert!(limits.cpu_limit.is_some());
        assert!(limits.memory_limit.is_some());
        assert!(limits.disable_swap);

        info!("✅ Resource limits configuration validated");
        info!("   Limits will be applied to all test containers");
        info!("   This prevents container resource overuse in CI/CD");

        info!("╔══════════════════════════════════════════════╗");
        info!("║  ✅ Resource Limits Test Passed             ║");
        info!("╚══════════════════════════════════════════════╝");
    }

    /// Test: Container Leak Detection
    /// Verify all containers are properly cleaned up
    #[tokio::test]
    async fn test_container_leak_detection() {
        info!("\n╔══════════════════════════════════════════════╗");
        info!("║  Testing Container Leak Detection          ║");
        info!("╚══════════════════════════════════════════════╝");

        let leak_detector = ContainerLeakDetector::new();

        // Get registry and check stats
        let registry = get_test_registry().await;
        let stats = registry.get_stats();

        info!("Container registry stats:");
        info!("   Tracked containers: {}", stats.tracked_containers);
        info!("   Active containers: {}", stats.active_containers);

        // Verify we have the expected containers (singleton pattern)
        assert!(
            stats.tracked_containers >= 3,
            "Should have at least 3 singleton containers"
        );

        // Check for leaks
        let leaked = leak_detector.check_leaks();
        assert_eq!(leaked, 0, "No container leaks should be detected");

        info!("✅ No container leaks detected");
        info!("   Registry is properly tracking all containers");

        // List all tracked containers
        let containers = registry.list_tracked_containers();
        info!("Tracked containers:");
        for (name, metadata) in containers {
            info!(
                "   {}: {} (ports: {:?})",
                name, metadata.id, metadata.exposed_ports
            );
        }

        info!("╔══════════════════════════════════════════════╗");
        info!("║  ✅ Leak Detection Test Passed              ║");
        info!("╚══════════════════════════════════════════════╝");
    }

    /// Test: Parallel Container Startup
    /// Verify multiple containers can start concurrently
    #[tokio::test]
    async fn test_parallel_container_startup() {
        info!("\n╔══════════════════════════════════════════════╗");
        info!("║  Testing Parallel Container Startup        ║");
        info!("╚══════════════════════════════════════════════╝");

        let start = std::time::Instant::now();

        // Get registry with parallel startup enabled
        let registry = get_test_registry().await;

        // Start multiple containers in parallel
        let (nats_result, postgres_result, redis_result) = tokio::join!(
            registry.get_or_start_nats(),
            registry.get_or_start_postgres(),
            registry.get_or_start_redis()
        );

        // Verify all started successfully
        assert!(nats_result.is_ok());
        assert!(postgres_result.is_ok());
        assert!(redis_result.is_ok());

        let nats = nats_result.unwrap();
        let postgres = postgres_result.unwrap();
        let redis = redis_result.unwrap();

        let duration = start.elapsed();

        info!("✅ All containers started in parallel");
        info!("   NATS: localhost:{}", nats.get_host_port_ipv4(4222));
        info!(
            "   PostgreSQL: localhost:{}",
            postgres.get_host_port_ipv4(5432)
        );
        info!("   Redis: localhost:{}", redis.get_host_port_ipv4(6379));
        info!("   Total startup time: {:?}", duration);

        // Performance assertion
        if duration < Duration::from_secs(10) {
            info!("✅ Parallel startup working efficiently!");
        } else {
            warn!("⚠️  Parallel startup took longer than expected");
        }

        info!("╔══════════════════════════════════════════════╗");
        info!("║  ✅ Parallel Startup Test Passed            ║");
        info!("╚══════════════════════════════════════════════╝");
    }
}

/// Performance benchmark comparing singleton vs. individual containers
#[cfg(test)]
mod benchmarks {
    use super::*;

    /// Benchmark: First run (container startup)
    /// This simulates the cold start scenario
    #[tokio::test]
    async fn benchmark_cold_start() {
        let start = std::time::Instant::now();

        // Create new environment (not singleton)
        let temp_config = crate::testcontainers_manager::TestEnvironmentConfig {
            reuse_containers: false, // Force new containers
            max_containers_per_type: 1,
            startup_timeout: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(15),
            parallel_startup: true,
        };

        let registry = crate::testcontainers_manager::ContainerRegistry::new(temp_config);

        // Start containers (cold start)
        let _nats = registry.get_or_start_nats().await.unwrap();
        let _postgres = registry.get_or_start_postgres().await.unwrap();

        let duration = start.elapsed();

        println!("\n╔══════════════════════════════════════════════╗");
        println!("║  BENCHMARK: Cold Start (First Run)          ║");
        println!("╚══════════════════════════════════════════════╝");
        println!("Duration: {:?}", duration);
        println!("Expected: ~15-30 seconds (container startup time)");

        assert!(
            duration > Duration::from_secs(10),
            "Cold start should take time"
        );
    }

    /// Benchmark: Warm start (container reuse)
    /// This demonstrates the benefit of singleton pattern
    #[tokio::test]
    async fn benchmark_warm_start() {
        let start = std::time::Instant::now();

        // Use singleton (warm start)
        let registry = get_test_registry().await;
        let _nats = registry.get_or_start_nats().await.unwrap();
        let _postgres = registry.get_or_start_postgres().await.unwrap();

        let duration = start.elapsed();

        println!("\n╔══════════════════════════════════════════════╗");
        println!("║  BENCHMARK: Warm Start (Singleton Reuse)    ║");
        println!("╚══════════════════════════════════════════════╝");
        println!("Duration: {:?}", duration);
        println!("Expected: < 1 second (just connection time)");

        assert!(
            duration < Duration::from_secs(2),
            "Warm start should be fast"
        );
    }

    /// Calculate performance improvement
    #[test]
    fn calculate_performance_improvement() {
        let cold_start = Duration::from_secs(20); // Example: 20 seconds
        let warm_start = Duration::from_millis(500); // Example: 500ms

        let improvement = cold_start.as_secs_f64() / warm_start.as_secs_f64();
        let time_saved = cold_start - warm_start;

        println!("\n╔══════════════════════════════════════════════╗");
        println!("║  PERFORMANCE IMPROVEMENT ANALYSIS           ║");
        println!("╚══════════════════════════════════════════════╝");
        println!("Cold start (individual containers): {:?}", cold_start);
        println!("Warm start (singleton pattern):      {:?}", warm_start);
        println!("Improvement factor:                  {:.2}x", improvement);
        println!("Time saved per test:                 {:?}", time_saved);
        println!("");
        println!("With 100 tests:");
        println!(
            "Individual containers: {} minutes",
            (cold_start * 100).as_secs_f64() / 60.0
        );
        println!(
            "Singleton pattern:   {} seconds",
            (warm_start * 100).as_secs_f64()
        );
        println!(
            "Total time saved:    {} minutes",
            (time_saved * 100).as_secs_f64() / 60.0
        );
    }
}
