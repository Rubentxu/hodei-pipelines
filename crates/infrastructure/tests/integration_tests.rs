//! Integration Tests for Infrastructure Components
//!
//! These tests verify the integration between infrastructure components
//! including database, event bus, and providers.

#[cfg(test)]
mod tests {
    use super::*;
    use domain::shared_kernel::{
        DomainEvent, JobId, ProviderId, ProviderCapabilities, ResourceRequirements,
    };

    #[tokio::test]
    async fn test_event_publisher_integration() {
        // Test: Should publish and handle events
        let config = infrastructure::NatsConfig::default();
        let connection = infrastructure::event_bus::MockNatsConnection::new();

        let publisher = infrastructure::NatsEventPublisher::new(connection, config);

        // Create a job created event
        let event = DomainEvent::JobCreated {
            job_id: JobId::new("integration-job-123".to_string()),
            job_name: "Integration Test Job".to_string(),
            provider_id: Some(ProviderId::new("docker-provider".to_string())),
        };

        // Publish the event
        let result = publisher.publish(&event).await;
        assert!(result.is_ok(), "Failed to publish event: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_health_checker_integration() {
        // Test: Should check multiple components
        let mut checker = infrastructure::HealthChecker::new();

        // Add database health check
        checker.add_check(Box::new(infrastructure::DatabaseHealthCheck::new(
            "postgres".to_string(),
            "postgresql://localhost:5432".to_string(),
        )));

        // Add NATS health check
        checker.add_check(Box::new(infrastructure::NatsHealthCheck::new(
            "nats".to_string(),
            "nats://localhost:4222".to_string(),
        )));

        // Add pool health check
        checker.add_check(Box::new(infrastructure::PoolHealthCheck::new(
            "connection-pool".to_string(),
            100,
            5,
        )));

        // Run all health checks
        let report = checker.check_all().await;

        assert_eq!(report.checks.len(), 3);
        assert!(report.checks.iter().any(|c| c.name == "postgres"));
        assert!(report.checks.iter().any(|c| c.name == "nats"));
        assert!(report.checks.iter().any(|c| c.name == "connection-pool"));
        assert!(report.is_ready());
    }

    #[tokio::test]
    async fn test_event_serialization_integration() {
        // Test: Should serialize and deserialize events correctly
        let events = vec![
            DomainEvent::JobCreated {
                job_id: JobId::new("job-1".to_string()),
                job_name: "Test Job 1".to_string(),
                provider_id: None,
            },
            DomainEvent::JobScheduled {
                job_id: JobId::new("job-2".to_string()),
                provider_id: ProviderId::new("provider-1".to_string()),
            },
            DomainEvent::JobCompleted {
                job_id: JobId::new("job-3".to_string()),
                provider_id: ProviderId::new("provider-1".to_string()),
                execution_id: "exec-123".to_string(),
                success: true,
                output: Some("Success".to_string()),
                error: None,
                execution_time_ms: 1000,
            },
            DomainEvent::JobFailed {
                job_id: JobId::new("job-4".to_string()),
                provider_id: ProviderId::new("provider-1".to_string()),
                execution_id: "exec-456".to_string(),
                error_message: "Test error".to_string(),
            },
            DomainEvent::WorkerConnected {
                provider_id: ProviderId::new("docker-provider".to_string()),
                capabilities: ProviderCapabilities {
                    max_concurrent_jobs: 10,
                    supported_job_types: vec!["bash".to_string(), "sh".to_string()],
                    memory_limit_mb: Some(8192),
                    cpu_limit: Some(4.0),
                },
            },
            DomainEvent::WorkerHeartbeat {
                provider_id: ProviderId::new("provider-1".to_string()),
                timestamp: chrono::Utc::now(),
                active_jobs: 5,
                resource_usage: ResourceRequirements {
                    memory_mb: Some(4096),
                    cpu_cores: Some(2.0),
                    timeout_seconds: Some(300),
                },
            },
        ];

        // Test serialization/deserialization for all events
        for event in events {
            let serialized = serde_json::to_string(&event).unwrap();
            let deserialized: DomainEvent = serde_json::from_str(&serialized).unwrap();

            // Verify round-trip serialization
            match (event, deserialized) {
                (DomainEvent::JobCreated { .. }, DomainEvent::JobCreated { .. }) => {}
                (DomainEvent::JobScheduled { .. }, DomainEvent::JobScheduled { .. }) => {}
                (DomainEvent::JobCompleted { .. }, DomainEvent::JobCompleted { .. }) => {}
                (DomainEvent::JobFailed { .. }, DomainEvent::JobFailed { .. }) => {}
                (DomainEvent::WorkerConnected { .. }, DomainEvent::WorkerConnected { .. }) => {}
                (DomainEvent::WorkerHeartbeat { .. }, DomainEvent::WorkerHeartbeat { .. }) => {}
                _ => panic!("Event type mismatch after deserialization"),
            }
        }
    }

    #[tokio::test]
    async fn test_pool_configuration_integration() {
        // Test: Should configure connection pool correctly
        let config = infrastructure::PgBouncerConfig::new(
            "localhost".to_string(),
            "test_db".to_string(),
            "test_user".to_string(),
            "test_pass".to_string(),
        )
        .with_min_pool_size(10)
        .with_max_pool_size(50)
        .with_port(6432);

        assert_eq!(config.host, "localhost");
        assert_eq!(config.database, "test_db");
        assert_eq!(config.username, "test_user");
        assert_eq!(config.pool_min, 10);
        assert_eq!(config.pool_max, 50);
        assert_eq!(config.port, 6432);

        // Convert to PostgreSQL config
        let pg_config = config.to_pg_config();

        // Verify the configuration can be used (without actually connecting)
        let _manager = bb8_postgres::PostgresConnectionManager::new(pg_config, tokio_postgres::NoTls);
    }

    #[tokio::test]
    async fn test_docker_provider_integration() {
        // Test: Should handle job submission flow
        let provider_id = ProviderId::new("integration-docker-provider".to_string());
        let adapter = infrastructure::DockerProviderAdapter::new(provider_id);

        // Create a test job
        let job_id = JobId::new("integration-test-job".to_string());
        let job_spec = domain::shared_kernel::JobSpec::new(
            "integration-test".to_string(),
            vec!["echo".to_string(), "Hello Integration".to_string()],
            vec![],
        );

        // Submit job
        let result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(result.is_ok(), "Failed to submit job: {:?}", result.err());

        let execution_id = result.unwrap();
        assert!(!execution_id.is_empty(), "Execution ID is empty");

        // Get status
        let status = adapter.get_execution_status(&execution_id).await;
        assert!(status.is_ok(), "Failed to get execution status");
    }

    #[tokio::test]
    async fn test_complete_workflow_integration() {
        // Test: Complete workflow from job creation to completion
        let provider_id = ProviderId::new("workflow-provider".to_string());
        let adapter = infrastructure::DockerProviderAdapter::new(provider_id);

        // 1. Create job
        let job_id = JobId::new("workflow-job".to_string());
        let job_spec = domain::shared_kernel::JobSpec::new(
            "workflow-test".to_string(),
            vec!["echo".to_string(), "Workflow test".to_string()],
            vec![],
        );

        // 2. Submit job
        let submit_result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(submit_result.is_ok());
        let execution_id = submit_result.unwrap();

        // 3. Get status
        let status = adapter.get_execution_status(&execution_id).await;
        assert!(status.is_ok());

        // 4. Get result (if available)
        let result = adapter.get_job_result(&execution_id).await;
        assert!(result.is_ok());

        // 5. Verify job can be cancelled
        let cancel_result = adapter.cancel_job(&execution_id).await;
        assert!(cancel_result.is_ok());
    }

    #[tokio::test]
    async fn test_error_handling_integration() {
        // Test: Should handle various error conditions
        let provider_id = ProviderId::new("error-test-provider".to_string());
        let adapter = infrastructure::DockerProviderAdapter::new(provider_id);

        // Test with empty job spec
        let job_id = JobId::new("empty-job".to_string());
        let empty_spec = domain::shared_kernel::JobSpec::new(
            "empty-job".to_string(),
            vec![], // Empty commands
            vec![],
        );

        let result = adapter.submit_job(&job_id, &empty_spec).await;
        assert!(result.is_err(), "Should reject empty job spec");

        if let Err(e) = result {
            assert!(
                e.to_string().contains("must contain"),
                "Error should mention command requirements"
            );
        }
    }

    #[tokio::test]
    async fn test_multiple_providers_integration() {
        // Test: Should handle multiple providers
        let provider1_id = ProviderId::new("provider-1".to_string());
        let provider2_id = ProviderId::new("provider-2".to_string());

        let adapter1 = infrastructure::DockerProviderAdapter::new(provider1_id);
        let adapter2 = infrastructure::DockerProviderAdapter::new(provider2_id);

        // Submit jobs to both providers
        let job_id1 = JobId::new("multi-provider-job-1".to_string());
        let job_spec1 = domain::shared_kernel::JobSpec::new(
            "provider-1-job".to_string(),
            vec!["echo".to_string(), "Provider 1".to_string()],
            vec![],
        );

        let job_id2 = JobId::new("multi-provider-job-2".to_string());
        let job_spec2 = domain::shared_kernel::JobSpec::new(
            "provider-2-job".to_string(),
            vec!["echo".to_string(), "Provider 2".to_string()],
            vec![],
        );

        let result1 = adapter1.submit_job(&job_id1, &job_spec1).await;
        let result2 = adapter2.submit_job(&job_id2, &job_spec2).await;

        assert!(result1.is_ok(), "Provider 1 job failed");
        assert!(result2.is_ok(), "Provider 2 job failed");
    }
}
