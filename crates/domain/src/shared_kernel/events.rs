//! Domain Events for inter-component communication
//!
//! These events are published when domain state changes occur.

use serde::{Deserialize, Serialize};

use super::types::{JobId, ProviderId, ProviderCapabilities, ResourceRequirements};

/// System events for job lifecycle and worker communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    /// Job has been created and is pending execution
    JobCreated {
        job_id: JobId,
        job_name: String,
        provider_id: Option<ProviderId>,
    },

    /// Job has been scheduled for execution on a specific provider
    JobScheduled {
        job_id: JobId,
        provider_id: ProviderId,
    },

    /// Job execution has started
    JobStarted {
        job_id: JobId,
        provider_id: ProviderId,
        execution_id: String,
    },

    /// Job execution completed successfully
    JobCompleted {
        job_id: JobId,
        provider_id: ProviderId,
        execution_id: String,
        success: bool,
        output: Option<String>,
        error: Option<String>,
        execution_time_ms: u64,
    },

    /// Job execution failed
    JobFailed {
        job_id: JobId,
        provider_id: ProviderId,
        execution_id: String,
        error_message: String,
    },

    /// Job execution was cancelled
    JobCancelled {
        job_id: JobId,
        provider_id: ProviderId,
    },

    /// Worker/Provider has connected
    WorkerConnected {
        provider_id: ProviderId,
        capabilities: ProviderCapabilities,
    },

    /// Worker/Provider has disconnected
    WorkerDisconnected {
        provider_id: ProviderId,
    },

    /// Worker/Provider heartbeat with current status
    WorkerHeartbeat {
        provider_id: ProviderId,
        timestamp: chrono::DateTime<chrono::Utc>,
        active_jobs: u32,
        resource_usage: ResourceRequirements,
    },
}

/// Event error types
#[derive(thiserror::Error, Debug)]
pub enum EventError {
    #[error("Event serialization failed: {0}")]
    Serialization(String),

    #[error("Event deserialization failed: {0}")]
    Deserialization(String),

    #[error("Event publishing failed: {0}")]
    Publish(String),

    #[error("Event subscription failed: {0}")]
    Subscribe(String),

    #[error("Connection error: {0}")]
    Connection(String),
}

/// Result type for event operations
pub type EventResult<T> = Result<T, EventError>;

/// Event publisher port
#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single event
    async fn publish(&self, event: &DomainEvent) -> EventResult<()>;

    /// Publish multiple events in batch
    async fn publish_batch(&self, events: Vec<&DomainEvent>) -> EventResult<()> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

/// Event subscriber port
#[async_trait::async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Subscribe to events of a specific type
    async fn subscribe(&self, event_type: &str) -> EventResult<mpsc::Receiver<DomainEvent>>;
}

/// Event store for maintaining event history
#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    /// Save an event to the store
    async fn save_event(&self, event: &DomainEvent) -> EventResult<()>;

    /// Get events for a specific job
    async fn get_events_for_job(&self, job_id: &JobId) -> EventResult<Vec<DomainEvent>>;

    /// Get events for a specific provider
    async fn get_events_for_provider(&self, provider_id: &ProviderId) -> EventResult<Vec<DomainEvent>>;
}

/// Async channel for event reception
use tokio::sync::mpsc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_kernel::types::{JobId, ProviderId, ProviderCapabilities, ResourceRequirements};

    #[test]
    fn test_domain_event_serialization() {
        // Test JobCreated event serialization
        let event = DomainEvent::JobCreated {
            job_id: JobId::new("test-job-1".to_string()),
            job_name: "Test Job".to_string(),
            provider_id: Some(ProviderId::new("provider-1".to_string())),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("JobCreated"));

        let deserialized: DomainEvent = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            DomainEvent::JobCreated { job_id, job_name, provider_id } => {
                assert_eq!(job_id.to_string(), "test-job-1");
                assert_eq!(job_name, "Test Job");
                assert!(provider_id.is_some());
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_job_completed_event_serialization() {
        let event = DomainEvent::JobCompleted {
            job_id: JobId::new("test-job-2".to_string()),
            provider_id: ProviderId::new("provider-1".to_string()),
            execution_id: "exec-123".to_string(),
            success: true,
            output: Some("Success".to_string()),
            error: None,
            execution_time_ms: 1500,
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            DomainEvent::JobCompleted { job_id, success, output, .. } => {
                assert_eq!(job_id.to_string(), "test-job-2");
                assert!(success);
                assert_eq!(output, Some("Success".to_string()));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_worker_connected_event_serialization() {
        let event = DomainEvent::WorkerConnected {
            provider_id: ProviderId::new("docker-provider".to_string()),
            capabilities: ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["bash".to_string(), "sh".to_string()],
                memory_limit_mb: Some(8192),
                cpu_limit: Some(4.0),
            },
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            DomainEvent::WorkerConnected { provider_id, capabilities } => {
                assert_eq!(provider_id.to_string(), "docker-provider");
                assert_eq!(capabilities.supported_job_types.len(), 2);
                assert_eq!(capabilities.max_concurrent_jobs, 10);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_worker_heartbeat_event_serialization() {
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

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            DomainEvent::WorkerHeartbeat { provider_id, active_jobs, .. } => {
                assert_eq!(provider_id.to_string(), "provider-1");
                assert_eq!(active_jobs, 3);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_error_display() {
        let error = EventError::Serialization("test error".to_string());
        assert!(error.to_string().contains("Event serialization failed"));

        let error = EventError::Publish("publish error".to_string());
        assert!(error.to_string().contains("Event publishing failed"));
    }
}
