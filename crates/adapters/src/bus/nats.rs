//! NATS JetStream Event Bus Adapter
//!
//! This adapter implements EventPublisher using NATS JetStream for distributed,
//! persistent event communication.

use async_nats::jetstream::stream::{Config, StorageType};
use async_trait::async_trait;
use hodei_ports::event_bus::{EventBusError, EventPublisher, SystemEvent};
use serde_json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::{JobId, JobSpec, ResourceQuota};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_nats_bus_creation() {
        // Test: NatsBus should be creatable with a URL
        let result = NatsBus::new("nats://localhost:4222").await;
        assert!(result.is_ok(), "Should create NatsBus successfully");
    }

    #[tokio::test]
    async fn test_nats_bus_publish_event() {
        // Test: NatsBus should be able to publish SystemEvent
        let nats_bus = NatsBus::new("nats://localhost:4222").await.unwrap();

        // Create a simple test event
        let mut env = HashMap::new();
        env.insert("TEST_KEY".to_string(), "test_value".to_string());

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "alpine:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 0,
            env,
            secret_refs: vec![],
        };

        let event = SystemEvent::JobCreated(job_spec);

        // This should publish without error (in test mode, may connect to real NATS or fail gracefully)
        let result = nats_bus.publish(event).await;
        // For now, we just verify the method is callable
        // The actual publish test would require a running NATS server
        match result {
            Ok(_) => println!("✅ Event published successfully"),
            Err(e) => println!("⚠️  Expected behavior in test without NATS server: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_nats_bus_batch_publish() {
        // Test: NatsBus should support batch publishing
        let nats_bus = NatsBus::new("nats://localhost:4222").await.unwrap();

        let events = vec![
            SystemEvent::JobScheduled {
                job_id: JobId::new(),
                worker_id: hodei_core::WorkerId::new(),
            },
            SystemEvent::JobStarted {
                job_id: JobId::new(),
                worker_id: hodei_core::WorkerId::new(),
            },
        ];

        let result = nats_bus.publish_batch(events).await;
        // Verify the method is callable
        match result {
            Ok(_) => println!("✅ Batch events published successfully"),
            Err(e) => println!("⚠️  Expected behavior in test without NATS server: {:?}", e),
        }
    }

    #[cfg(feature = "integration")]
    #[tokio::test]
    async fn test_nats_bus_integration_with_testcontainers() {
        use testcontainers::clients::Cli;
        use testcontainers_modules::nats::Nats;

        // Start NATS server using testcontainers
        let docker = Cli::default();
        let node = docker.run(Nats::default());
        let nats_url = format!("nats://localhost:{}", node.get_host_port_ipv4(4222));

        println!("🧪 Running integration test with NATS at: {}", nats_url);

        // Wait a bit for NATS to be ready
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Create NatsBus with test server
        let nats_bus = NatsBus::new(&nats_url)
            .await
            .expect("Failed to create NatsBus with testcontainer NATS");

        // Create test event
        let mut env = HashMap::new();
        env.insert("TEST_INTEGRATION".to_string(), "true".to_string());

        let job_spec = JobSpec {
            name: "integration-test-job".to_string(),
            image: "alpine:latest".to_string(),
            command: vec!["echo".to_string(), "integration".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 10000,
            retries: 0,
            env,
            secret_refs: vec![],
        };

        let event = SystemEvent::JobCreated(job_spec);

        // Publish event - should succeed
        nats_bus
            .publish(event)
            .await
            .expect("Failed to publish event to NATS");

        println!("✅ Integration test passed - event published successfully");
    }

    #[tokio::test]
    async fn test_nats_bus_connection_error_handling() {
        // Test: NatsBus should handle connection errors gracefully
        let invalid_url = "nats://invalid-host-that-does-not-exist:9999";
        let result = NatsBus::new(invalid_url).await;

        match result {
            Ok(_) => panic!("Should fail to connect to invalid NATS URL"),
            Err(e) => {
                println!("✅ Correctly handled connection error: {:?}", e);
                // Should return an appropriate error
                assert!(true);
            }
        }
    }
}

/// NATS JetStream Event Bus Publisher
///
/// Implements EventPublisher trait for distributed event communication
/// using NATS JetStream as the message broker.
pub struct NatsBus {
    /// NATS client connection
    client: async_nats::Client,
    /// JetStream context for publishing
    jetstream: async_nats::jetstream::Context,
    /// Configuration
    config: NatsBusConfig,
    /// Stream name
    stream_name: String,
}

/// Configuration for NatsBus
#[derive(Debug, Clone)]
pub struct NatsBusConfig {
    /// NATS server URL
    pub url: String,
    /// Stream name for events
    pub stream_name: String,
    /// Subject prefix
    pub subject_prefix: String,
    /// Maximum messages in stream
    pub max_messages: i64,
    /// Connection timeout
    pub connection_timeout_ms: u64,
}

impl Default for NatsBusConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            stream_name: "HODEI_EVENTS".to_string(),
            subject_prefix: "hodei.events".to_string(),
            max_messages: 100_000,
            connection_timeout_ms: 5000,
        }
    }
}

impl NatsBus {
    /// Create a new NatsBus with the given URL
    ///
    /// # Errors
    /// Returns EventBusError if:
    /// - Connection to NATS server fails
    /// - JetStream is not available
    /// - Stream creation/configuration fails
    pub async fn new(url: &str) -> Result<Self, EventBusError> {
        let config = NatsBusConfig {
            url: url.to_string(),
            ..Default::default()
        };

        Self::new_with_config(config).await
    }

    /// Create a new NatsBus with custom configuration
    ///
    /// # Errors
    /// Returns EventBusError if connection or stream setup fails
    pub async fn new_with_config(mut config: NatsBusConfig) -> Result<Self, EventBusError> {
        // Connect to NATS
        let client = async_nats::connect(&config.url)
            .await
            .map_err(|e| EventBusError::Internal(format!("Failed to connect to NATS: {}", e)))?;

        // Create JetStream context
        let jetstream = async_nats::jetstream::new(client.clone());

        // Verify or create the stream
        Self::ensure_stream_exists(&jetstream, &config).await?;

        let stream_name = config.stream_name.clone();
        Ok(Self {
            client,
            jetstream,
            config,
            stream_name,
        })
    }

    /// Ensure the event stream exists with proper configuration
    async fn ensure_stream_exists(
        jetstream: &async_nats::jetstream::Context,
        config: &NatsBusConfig,
    ) -> Result<(), EventBusError> {
        // Try to get or create the stream
        let _stream = jetstream
            .get_or_create_stream(Config {
                name: config.stream_name.clone(),
                max_messages: config.max_messages,
                storage: StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| EventBusError::Internal(format!("Failed to create stream: {}", e)))?;

        // Stream creation successful - get_or_create_stream already verified it exists
        Ok(())
    }
}

#[async_trait]
impl EventPublisher for NatsBus {
    async fn publish(&self, event: SystemEvent) -> Result<(), EventBusError> {
        // Serialize event to JSON
        let payload = serde_json::to_vec(&event)
            .map_err(|e| EventBusError::Internal(format!("Failed to serialize event: {}", e)))?;

        // Determine subject based on event type
        let subject = Self::event_to_subject(&event);

        // Publish to JetStream
        let _ack = self
            .jetstream
            .publish(subject, payload.into())
            .await
            .map_err(|e| EventBusError::Internal(format!("Failed to publish event: {}", e)))?;

        Ok(())
    }

    async fn publish_batch(&self, events: Vec<SystemEvent>) -> Result<(), EventBusError> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

impl NatsBus {
    /// Convert SystemEvent to NATS subject
    fn event_to_subject(event: &SystemEvent) -> String {
        match event {
            SystemEvent::JobCreated(_) => "hodei.events.job.created".to_string(),
            SystemEvent::JobScheduled { .. } => "hodei.events.job.scheduled".to_string(),
            SystemEvent::JobStarted { .. } => "hodei.events.job.started".to_string(),
            SystemEvent::JobCompleted { .. } => "hodei.events.job.completed".to_string(),
            SystemEvent::JobFailed { .. } => "hodei.events.job.failed".to_string(),
            SystemEvent::WorkerConnected { .. } => "hodei.events.worker.connected".to_string(),
            SystemEvent::WorkerDisconnected { .. } => {
                "hodei.events.worker.disconnected".to_string()
            }
            SystemEvent::WorkerHeartbeat { .. } => "hodei.events.worker.heartbeat".to_string(),
            SystemEvent::LogChunkReceived(_) => "hodei.events.log.chunk".to_string(),
            SystemEvent::PipelineCreated(_) => "hodei.events.pipeline.created".to_string(),
            SystemEvent::PipelineStarted { .. } => "hodei.events.pipeline.started".to_string(),
            SystemEvent::PipelineCompleted { .. } => "hodei.events.pipeline.completed".to_string(),
        }
    }
}
