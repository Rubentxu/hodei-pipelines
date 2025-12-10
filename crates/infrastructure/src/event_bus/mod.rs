//! Event Bus Implementation using NATS
//!
//! Provides distributed event publishing and subscription capabilities
//! for inter-component communication in the job orchestration system.

use async_trait::async_trait;
use domain::shared_kernel::{
    DomainEvent, EventError, EventPublisher, EventResult, EventStore, EventSubscriber,
};
use serde_json;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// NATS Event Bus configuration
#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub stream_prefix: String,
    pub max_reconnect: usize,
    pub connection_timeout: std::time::Duration,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            stream_prefix: "hodei".to_string(),
            max_reconnect: 5,
            connection_timeout: std::time::Duration::from_secs(10),
        }
    }
}

/// Mock NATS connection for testing
#[derive(Debug, Clone)]
pub struct MockNatsConnection {
    pub sender: broadcast::Sender<(String, Vec<u8>)>,
}

impl MockNatsConnection {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), String> {
        self.sender
            .send((subject.to_string(), payload.to_vec()))
            .map(|_| ())
            .map_err(|e| format!("Failed to send: {:?}", e))
    }

    pub async fn subscribe(&self, subject: &str) -> Result<MockSubscription, String> {
        let mut rx = self.sender.subscribe();
        Ok(MockSubscription { rx })
    }
}

#[derive(Debug)]
pub struct MockSubscription {
    rx: broadcast::Receiver<(String, Vec<u8>)>,
}

impl MockSubscription {
    pub async fn next(&mut self) -> Option<MockMessage> {
        self.rx.recv().await.ok().map(|(subject, data)| MockMessage { subject, data })
    }
}

#[derive(Debug)]
pub struct MockMessage {
    pub subject: String,
    pub data: Vec<u8>,
}

/// NATS-based event publisher (using mock for testing)
pub struct NatsEventPublisher {
    connection: MockNatsConnection,
    config: NatsConfig,
}

impl NatsEventPublisher {
    /// Create a new NATS event publisher
    pub fn new(connection: MockNatsConnection, config: NatsConfig) -> Self {
        Self {
            connection,
            config,
        }
    }

    /// Convert domain event to NATS subject
    fn event_to_subject(&self, event: &DomainEvent) -> String {
        match event {
            DomainEvent::JobCreated { .. } => format!("{}.job.created", self.config.stream_prefix),
            DomainEvent::JobScheduled { .. } => format!("{}.job.scheduled", self.config.stream_prefix),
            DomainEvent::JobStarted { .. } => format!("{}.job.started", self.config.stream_prefix),
            DomainEvent::JobCompleted { .. } => format!("{}.job.completed", self.config.stream_prefix),
            DomainEvent::JobFailed { .. } => format!("{}.job.failed", self.config.stream_prefix),
            DomainEvent::JobCancelled { .. } => format!("{}.job.cancelled", self.config.stream_prefix),
            DomainEvent::WorkerConnected { .. } => {
                format!("{}.worker.connected", self.config.stream_prefix)
            }
            DomainEvent::WorkerDisconnected { .. } => {
                format!("{}.worker.disconnected", self.config.stream_prefix)
            }
            DomainEvent::WorkerHeartbeat { .. } => {
                format!("{}.worker.heartbeat", self.config.stream_prefix)
            }
        }
    }
}

#[async_trait]
impl EventPublisher for NatsEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> EventResult<()> {
        let subject = self.event_to_subject(event);

        // Serialize event to JSON
        let payload = serde_json::to_vec(event).map_err(|e| {
            EventError::Serialization(format!("Failed to serialize event: {}", e))
        })?;

        // Publish to NATS (mock)
        match self.connection.publish(&subject, &payload).await {
            Ok(_) => {
                debug!("Published event to subject {}: {:?}", subject, event);
                Ok(())
            }
            Err(e) => {
                error!("Failed to publish event to {}: {}", subject, e);
                Err(EventError::Publish(format!(
                    "Failed to publish to {}: {}",
                    subject, e
                )))
            }
        }
    }
}

/// In-memory event store for testing and local development
pub struct InMemoryEventStore {
    events: Vec<(DomainEvent, chrono::DateTime<chrono::Utc>)>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn save_event(&self, event: &DomainEvent) -> EventResult<()> {
        // In a real implementation, this would save to a database
        Ok(())
    }

    async fn get_events_for_job(&self, job_id: &domain::shared_kernel::JobId) -> EventResult<Vec<DomainEvent>> {
        Ok(self
            .events
            .iter()
            .filter_map(|(event, _)| match event {
                DomainEvent::JobCreated { job_id: id, .. }
                | DomainEvent::JobScheduled { job_id: id, .. }
                | DomainEvent::JobStarted { job_id: id, .. }
                | DomainEvent::JobCompleted { job_id: id, .. }
                | DomainEvent::JobFailed { job_id: id, .. }
                | DomainEvent::JobCancelled { job_id: id, .. } => {
                    if id == job_id {
                        Some(event.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect())
    }

    async fn get_events_for_provider(
        &self,
        provider_id: &domain::shared_kernel::ProviderId,
    ) -> EventResult<Vec<DomainEvent>> {
        Ok(self
            .events
            .iter()
            .filter_map(|(event, _)| match event {
                DomainEvent::JobScheduled { provider_id: id, .. }
                | DomainEvent::JobStarted { provider_id: id, .. }
                | DomainEvent::JobCompleted { provider_id: id, .. }
                | DomainEvent::JobFailed { provider_id: id, .. }
                | DomainEvent::JobCancelled { provider_id: id, .. }
                | DomainEvent::WorkerConnected { provider_id: id, .. }
                | DomainEvent::WorkerDisconnected { provider_id: id, .. }
                | DomainEvent::WorkerHeartbeat { provider_id: id, .. } => {
                    if id == provider_id {
                        Some(event.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::shared_kernel::{
        JobId, ProviderId, JobResult, ProviderCapabilities, ResourceRequirements,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn test_nats_publisher_new() {
        // Test: Should create a new NATS publisher
        let config = NatsConfig::default();
        let connection = MockNatsConnection::new();

        let publisher = NatsEventPublisher::new(connection, config);

        assert_eq!(publisher.config.url, "nats://localhost:4222");
        assert_eq!(publisher.config.stream_prefix, "hodei");
    }

    #[test]
    fn test_event_to_subject_job_created() {
        // Test: Should convert JobCreated event to correct subject
        let config = NatsConfig {
            stream_prefix: "hodei".to_string(),
            ..Default::default()
        };
        let connection = MockNatsConnection::new();
        let publisher = NatsEventPublisher::new(connection, config);

        let event = DomainEvent::JobCreated {
            job_id: JobId::new("job-123".to_string()),
            job_name: "Test Job".to_string(),
            provider_id: Some(ProviderId::new("provider-1".to_string())),
        };

        let subject = publisher.event_to_subject(&event);
        assert_eq!(subject, "hodei.job.created");
    }

    #[test]
    fn test_event_to_subject_job_scheduled() {
        // Test: Should convert JobScheduled event to correct subject
        let config = NatsConfig {
            stream_prefix: "hodei".to_string(),
            ..Default::default()
        };
        let connection = MockNatsConnection::new();
        let publisher = NatsEventPublisher::new(connection, config);

        let event = DomainEvent::JobScheduled {
            job_id: JobId::new("job-123".to_string()),
            provider_id: ProviderId::new("provider-1".to_string()),
        };

        let subject = publisher.event_to_subject(&event);
        assert_eq!(subject, "hodei.job.scheduled");
    }

    #[test]
    fn test_event_to_subject_job_started() {
        // Test: Should convert JobStarted event to correct subject
        let config = NatsConfig {
            stream_prefix: "hodei".to_string(),
            ..Default::default()
        };
        let connection = MockNatsConnection::new();
        let publisher = NatsEventPublisher::new(connection, config);

        let event = DomainEvent::JobStarted {
            job_id: JobId::new("job-123".to_string()),
            provider_id: ProviderId::new("provider-1".to_string()),
            execution_id: "exec-456".to_string(),
        };

        let subject = publisher.event_to_subject(&event);
        assert_eq!(subject, "hodei.job.started");
    }

    #[test]
    fn test_event_to_subject_job_completed() {
        // Test: Should convert JobCompleted event to correct subject
        let config = NatsConfig {
            stream_prefix: "hodei".to_string(),
            ..Default::default()
        };
        let connection = MockNatsConnection::new();
        let publisher = NatsEventPublisher::new(connection, config);

        let event = DomainEvent::JobCompleted {
            job_id: JobId::new("job-123".to_string()),
            provider_id: ProviderId::new("provider-1".to_string()),
            execution_id: "exec-456".to_string(),
            success: true,
            output: Some("Success".to_string()),
            error: None,
            execution_time_ms: 1000,
        };

        let subject = publisher.event_to_subject(&event);
        assert_eq!(subject, "hodei.job.completed");
    }

    #[test]
    fn test_event_to_subject_job_failed() {
        // Test: Should convert JobFailed event to correct subject
        let config = NatsConfig {
            stream_prefix: "hodei".to_string(),
            ..Default::default()
        };
        let connection = MockNatsConnection::new();
        let publisher = NatsEventPublisher::new(connection, config);

        let event = DomainEvent::JobFailed {
            job_id: JobId::new("job-123".to_string()),
            provider_id: ProviderId::new("provider-1".to_string()),
            execution_id: "exec-456".to_string(),
            error_message: "Something went wrong".to_string(),
        };

        let subject = publisher.event_to_subject(&event);
        assert_eq!(subject, "hodei.job.failed");
    }

    #[test]
    fn test_event_to_subject_worker_connected() {
        // Test: Should convert WorkerConnected event to correct subject
        let config = NatsConfig {
            stream_prefix: "hodei".to_string(),
            ..Default::default()
        };
        let connection = MockNatsConnection::new();
        let publisher = NatsEventPublisher::new(connection, config);

        let event = DomainEvent::WorkerConnected {
            provider_id: ProviderId::new("docker-provider".to_string()),
            capabilities: ProviderCapabilities {
                max_concurrent_jobs: 5,
                supported_job_types: vec!["bash".to_string()],
                memory_limit_mb: Some(4096),
                cpu_limit: Some(2.0),
            },
        };

        let subject = publisher.event_to_subject(&event);
        assert_eq!(subject, "hodei.worker.connected");
    }

    #[test]
    fn test_event_to_subject_worker_heartbeat() {
        // Test: Should convert WorkerHeartbeat event to correct subject
        let config = NatsConfig {
            stream_prefix: "hodei".to_string(),
            ..Default::default()
        };
        let connection = MockNatsConnection::new();
        let publisher = NatsEventPublisher::new(connection, config);

        let event = DomainEvent::WorkerHeartbeat {
            provider_id: ProviderId::new("provider-1".to_string()),
            timestamp: chrono::Utc::now(),
            active_jobs: 3,
            resource_usage: ResourceRequirements {
                memory_mb: Some(4096),
                cpu_cores: Some(2.0),
                timeout_seconds: Some(300),
            },
        };

        let subject = publisher.event_to_subject(&event);
        assert_eq!(subject, "hodei.worker.heartbeat");
    }

    #[tokio::test]
    async fn test_publish_job_created_event() {
        // Test: Should successfully publish JobCreated event
        let config = NatsConfig::default();
        let connection = MockNatsConnection::new();

        let publisher = NatsEventPublisher::new(connection.clone(), config.clone());

        let event = DomainEvent::JobCreated {
            job_id: JobId::new("job-123".to_string()),
            job_name: "Test Job".to_string(),
            provider_id: Some(ProviderId::new("provider-1".to_string())),
        };

        // Publish the event
        let result = publisher.publish(&event).await;
        assert!(result.is_ok(), "Failed to publish event: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_publish_job_completed_event() {
        // Test: Should successfully publish JobCompleted event
        let config = NatsConfig::default();
        let connection = MockNatsConnection::new();

        let publisher = NatsEventPublisher::new(connection.clone(), config.clone());

        let event = DomainEvent::JobCompleted {
            job_id: JobId::new("job-456".to_string()),
            provider_id: ProviderId::new("provider-1".to_string()),
            execution_id: "exec-789".to_string(),
            success: true,
            output: Some("Job completed successfully".to_string()),
            error: None,
            execution_time_ms: 1500,
        };

        let result = publisher.publish(&event).await;
        assert!(result.is_ok(), "Failed to publish event");
    }

    #[tokio::test]
    async fn test_publish_worker_connected_event() {
        // Test: Should successfully publish WorkerConnected event
        let config = NatsConfig::default();
        let connection = MockNatsConnection::new();

        let publisher = NatsEventPublisher::new(connection.clone(), config.clone());

        let event = DomainEvent::WorkerConnected {
            provider_id: ProviderId::new("docker-provider".to_string()),
            capabilities: ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["bash".to_string(), "sh".to_string()],
                memory_limit_mb: Some(8192),
                cpu_limit: Some(4.0),
            },
        };

        let result = publisher.publish(&event).await;
        assert!(result.is_ok(), "Failed to publish event");
    }

    #[test]
    fn test_nats_config_default() {
        // Test: Should have correct default values
        let config = NatsConfig::default();

        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.stream_prefix, "hodei");
        assert_eq!(config.max_reconnect, 5);
        assert_eq!(config.connection_timeout, std::time::Duration::from_secs(10));
    }

    #[test]
    fn test_nats_config_custom() {
        // Test: Should accept custom configuration
        let config = NatsConfig {
            url: "nats://custom:4222".to_string(),
            stream_prefix: "custom".to_string(),
            max_reconnect: 10,
            connection_timeout: std::time::Duration::from_secs(30),
        };

        assert_eq!(config.url, "nats://custom:4222");
        assert_eq!(config.stream_prefix, "custom");
        assert_eq!(config.max_reconnect, 10);
        assert_eq!(config.connection_timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_in_memory_event_store_new() {
        // Test: Should create new event store
        let store = InMemoryEventStore::new();
        assert_eq!(store.events.len(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_event_store_get_events_for_job() {
        // Test: Should retrieve events for specific job
        let mut store = InMemoryEventStore::new();

        let job_id = JobId::new("job-123".to_string());
        let other_job_id = JobId::new("job-456".to_string());

        // Events for different jobs
        let event1 = DomainEvent::JobCreated {
            job_id: job_id.clone(),
            job_name: "Job 1".to_string(),
            provider_id: None,
        };

        let event2 = DomainEvent::JobScheduled {
            job_id: other_job_id.clone(),
            provider_id: ProviderId::new("provider-1".to_string()),
        };

        let event3 = DomainEvent::JobCompleted {
            job_id: job_id.clone(),
            provider_id: ProviderId::new("provider-1".to_string()),
            execution_id: "exec-1".to_string(),
            success: true,
            output: None,
            error: None,
            execution_time_ms: 1000,
        };

        // In a real implementation, events would be saved to the store
        let _ = store.save_event(&event1).await;
        let _ = store.save_event(&event2).await;
        let _ = store.save_event(&event3).await;

        let job_events = store.get_events_for_job(&job_id).await.unwrap();
        assert_eq!(job_events.len(), 2); // JobCreated and JobCompleted
    }

    #[test]
    fn test_mock_nats_connection_new() {
        // Test: Should create new mock connection
        let connection = MockNatsConnection::new();
        assert!(connection.sender.len() <= 1000);
    }

    #[tokio::test]
    async fn test_mock_nats_connection_publish() {
        // Test: Should successfully publish to mock connection
        let connection = MockNatsConnection::new();
        let result = connection.publish("test.subject", b"test data").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_nats_connection_subscribe() {
        // Test: Should successfully subscribe to mock connection
        let connection = MockNatsConnection::new();
        let result = connection.subscribe("test.subject").await;
        assert!(result.is_ok());
    }
}
