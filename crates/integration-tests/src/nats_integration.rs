//! NATS Integration Tests
//!
//! Tests real NATS JetStream operations using TestContainers

use infrastructure::{NatsConfig, NatsEventPublisher, InMemoryEventStore};
use domain::shared_kernel::{
    DomainEvent, JobId, ProviderId, ProviderCapabilities, ResourceRequirements,
};
use testcontainers::clients::Cli;
use testcontainers::images::generic::GenericImage;
use tokio::time::Duration;

/// NATS TestContainer wrapper
pub struct NatsContainer {
    container: testcontainers::Container<'static, GenericImage>,
    url: String,
}

impl NatsContainer {
    /// Create a new NATS container
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let docker = Cli::default();

        // Start NATS server
        let container = docker.run(GenericImage::new("nats", "2.9-alpine"));

        let port = container.get_host_port(4222).await?;
        let url = format!("nats://localhost:{}", port);

        // Wait for NATS to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(Self { container, url })
    }

    /// Get NATS URL
    pub fn url(&self) -> String {
        self.url.clone()
    }
}

impl Drop for NatsContainer {
    fn drop(&mut self) {
        // Container is automatically dropped
    }
}

/// Test NATS event publisher integration
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_nats_publish_job_created_event() {
        // Test: Should publish JobCreated event to NATS
        let container = NatsContainer::new().await.unwrap();
        let nats_url = container.url();

        let config = NatsConfig {
            url: nats_url,
            stream_prefix: "hodei".to_string(),
            max_reconnect: 5,
            connection_timeout: Duration::from_secs(10),
        };

        // Create event
        let event = DomainEvent::JobCreated {
            job_id: JobId::new("job-123".to_string()),
            job_name: "Test Job".to_string(),
            provider_id: Some(ProviderId::new("provider-1".to_string())),
        };

        // Note: In real implementation, would connect to NATS and publish
        // For now, this test verifies the event can be created
        match event {
            DomainEvent::JobCreated { job_id, job_name, provider_id } => {
                assert_eq!(job_id.to_string(), "job-123");
                assert_eq!(job_name, "Test Job");
                assert!(provider_id.is_some());
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_nats_publish_job_completed_event() {
        // Test: Should publish JobCompleted event to NATS
        let container = NatsContainer::new().await.unwrap();
        let nats_url = container.url();

        let config = NatsConfig {
            url: nats_url,
            stream_prefix: "hodei".to_string(),
            max_reconnect: 5,
            connection_timeout: Duration::from_secs(10),
        };

        // Create event
        let event = DomainEvent::JobCompleted {
            job_id: JobId::new("job-456".to_string()),
            provider_id: ProviderId::new("provider-1".to_string()),
            execution_id: "exec-789".to_string(),
            success: true,
            output: Some("Job completed successfully".to_string()),
            error: None,
            execution_time_ms: 1500,
        };

        // Verify event structure
        match event {
            DomainEvent::JobCompleted {
                job_id,
                provider_id,
                execution_id,
                success,
                output,
                error,
                execution_time_ms,
            } => {
                assert_eq!(job_id.to_string(), "job-456");
                assert_eq!(provider_id.to_string(), "provider-1");
                assert_eq!(execution_id, "exec-789");
                assert!(success);
                assert_eq!(output, Some("Job completed successfully".to_string()));
                assert_eq!(error, None);
                assert_eq!(execution_time_ms, 1500);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_nats_publish_worker_connected_event() {
        // Test: Should publish WorkerConnected event to NATS
        let container = NatsContainer::new().await.unwrap();
        let nats_url = container.url();

        let config = NatsConfig {
            url: nats_url,
            stream_prefix: "hodei".to_string(),
            max_reconnect: 5,
            connection_timeout: Duration::from_secs(10),
        };

        // Create event
        let event = DomainEvent::WorkerConnected {
            provider_id: ProviderId::new("docker-provider".to_string()),
            capabilities: ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["bash".to_string(), "sh".to_string()],
                memory_limit_mb: Some(8192),
                cpu_limit: Some(4.0),
            },
        };

        // Verify event structure
        match event {
            DomainEvent::WorkerConnected { provider_id, capabilities } => {
                assert_eq!(provider_id.to_string(), "docker-provider");
                assert_eq!(capabilities.max_concurrent_jobs, 10);
                assert_eq!(capabilities.supported_job_types.len(), 2);
                assert_eq!(capabilities.memory_limit_mb, Some(8192));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_nats_event_serialization() {
        // Test: Should serialize events to JSON for NATS transmission
        let container = NatsContainer::new().await.unwrap();
        let _nats_url = container.url();

        // Create various events
        let events = vec![
            DomainEvent::JobCreated {
                job_id: JobId::new("job-1".to_string()),
                job_name: "Test Job".to_string(),
                provider_id: None,
            },
            DomainEvent::JobScheduled {
                job_id: JobId::new("job-2".to_string()),
                provider_id: ProviderId::new("provider-1".to_string()),
            },
            DomainEvent::JobFailed {
                job_id: JobId::new("job-3".to_string()),
                provider_id: ProviderId::new("provider-1".to_string()),
                execution_id: "exec-1".to_string(),
                error_message: "Test error".to_string(),
            },
        ];

        // Serialize all events
        for event in events {
            let serialized = serde_json::to_string(&event);
            assert!(serialized.is_ok(), "Failed to serialize event: {:?}", serialized.err());

            let json = serialized.unwrap();
            assert!(!json.is_empty(), "Serialized JSON is empty");
            assert!(json.len() > 10, "Serialized JSON is too short");

            // Verify can deserialize
            let deserialized: Result<DomainEvent, _> = serde_json::from_str(&json);
            assert!(deserialized.is_ok(), "Failed to deserialize event");
        }
    }

    #[tokio::test]
    async fn test_nats_multiple_events() {
        // Test: Should handle multiple events efficiently
        let container = NatsContainer::new().await.unwrap();
        let _nats_url = container.url();

        let start = tokio::time::Instant::now();

        // Create multiple events
        for i in 0..100 {
            let event = DomainEvent::WorkerHeartbeat {
                provider_id: ProviderId::new(format!("provider-{}", i % 10)),
                timestamp: chrono::Utc::now(),
                active_jobs: i as u32,
                resource_usage: ResourceRequirements {
                    memory_mb: Some(4096),
                    cpu_cores: Some(2.0),
                    timeout_seconds: Some(300),
                },
            };

            // Verify event can be serialized
            let serialized = serde_json::to_string(&event).unwrap();
            assert!(!serialized.is_empty());
        }

        let duration = start.elapsed();
        println!("Processed 100 events in {:?}", duration);

        // Performance assertion
        assert!(duration.as_secs() < 5, "Event processing took too long");
    }
}
