//! Event Orchestrator for Application Layer
//!
//! Coordinates event publishing from application services to the event bus

use domain::shared_kernel::{DomainEvent, EventPublisher, EventResult};
use domain::shared_kernel::types::{JobId, ProviderId, ProviderCapabilities};

/// Event Orchestrator - publishes domain events from application layer
pub struct EventOrchestrator {
    event_publisher: Box<dyn EventPublisher>,
}

impl EventOrchestrator {
    pub fn new(event_publisher: Box<dyn EventPublisher>) -> Self {
        Self { event_publisher }
    }

    /// Publish JobCreated event
    pub async fn publish_job_created(
        &self,
        job_id: JobId,
        job_name: String,
        provider_id: Option<ProviderId>,
    ) -> EventResult<()> {
        let event = DomainEvent::JobCreated {
            job_id,
            job_name,
            provider_id,
        };
        self.event_publisher.publish(&event).await
    }

    /// Publish JobScheduled event
    pub async fn publish_job_scheduled(
        &self,
        job_id: JobId,
        provider_id: ProviderId,
    ) -> EventResult<()> {
        let event = DomainEvent::JobScheduled { job_id, provider_id };
        self.event_publisher.publish(&event).await
    }

    /// Publish JobStarted event
    pub async fn publish_job_started(
        &self,
        job_id: JobId,
        provider_id: ProviderId,
        execution_id: String,
    ) -> EventResult<()> {
        let event = DomainEvent::JobStarted {
            job_id,
            provider_id,
            execution_id,
        };
        self.event_publisher.publish(&event).await
    }

    /// Publish JobCompleted event
    pub async fn publish_job_completed(
        &self,
        job_id: JobId,
        provider_id: ProviderId,
        execution_id: String,
        success: bool,
        output: Option<String>,
        error: Option<String>,
        execution_time_ms: u64,
    ) -> EventResult<()> {
        let event = DomainEvent::JobCompleted {
            job_id,
            provider_id,
            execution_id,
            success,
            output,
            error,
            execution_time_ms,
        };
        self.event_publisher.publish(&event).await
    }

    /// Publish JobFailed event
    pub async fn publish_job_failed(
        &self,
        job_id: JobId,
        provider_id: ProviderId,
        execution_id: String,
        error_message: String,
    ) -> EventResult<()> {
        let event = DomainEvent::JobFailed {
            job_id,
            provider_id,
            execution_id,
            error_message,
        };
        self.event_publisher.publish(&event).await
    }

    /// Publish JobCancelled event
    pub async fn publish_job_cancelled(
        &self,
        job_id: JobId,
        provider_id: ProviderId,
    ) -> EventResult<()> {
        let event = DomainEvent::JobCancelled { job_id, provider_id };
        self.event_publisher.publish(&event).await
    }

    /// Publish WorkerConnected event
    pub async fn publish_worker_connected(
        &self,
        provider_id: ProviderId,
        capabilities: ProviderCapabilities,
    ) -> EventResult<()> {
        let event = DomainEvent::WorkerConnected {
            provider_id,
            capabilities,
        };
        self.event_publisher.publish(&event).await
    }

    /// Publish WorkerDisconnected event
    pub async fn publish_worker_disconnected(&self, provider_id: ProviderId) -> EventResult<()> {
        let event = DomainEvent::WorkerDisconnected { provider_id };
        self.event_publisher.publish(&event).await
    }

    /// Publish WorkerHeartbeat event
    pub async fn publish_worker_heartbeat(
        &self,
        provider_id: ProviderId,
        active_jobs: u32,
        resource_usage: domain::shared_kernel::types::ResourceRequirements,
    ) -> EventResult<()> {
        let event = DomainEvent::WorkerHeartbeat {
            provider_id,
            timestamp: chrono::Utc::now(),
            active_jobs,
            resource_usage,
        };
        self.event_publisher.publish(&event).await
    }

    /// Publish multiple events in batch
    pub async fn publish_batch(&self, events: Vec<DomainEvent>) -> EventResult<()> {
        for event in events.iter() {
            self.event_publisher.publish(event).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::shared_kernel::{EventPublisher, EventError};
    use domain::shared_kernel::types::{JobId, ProviderId, ProviderCapabilities, ResourceRequirements};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock EventPublisher for testing
    struct MockEventPublisher {
        published_events: Arc<Mutex<Vec<DomainEvent>>>,
    }

    impl MockEventPublisher {
        fn new() -> Self {
            Self {
                published_events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_published_events(&self) -> Arc<Mutex<Vec<DomainEvent>>> {
            self.published_events.clone()
        }
    }

    #[async_trait::async_trait]
    impl EventPublisher for MockEventPublisher {
        async fn publish(&self, event: &DomainEvent) -> Result<(), EventError> {
            let mut events = self.published_events.lock().await;
            events.push(event.clone());
            Ok(())
        }
    }

    // ==================== TDD RED PHASE ====================
    // Writing tests BEFORE implementation

    #[tokio::test]
    async fn test_publish_job_created_event() {
        // Test: Should publish JobCreated event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let job_id = JobId::new("test-job-1".to_string());
        let provider_id = ProviderId::new("provider-1".to_string());

        let result = orchestrator
            .publish_job_created(job_id.clone(), "Test Job".to_string(), Some(provider_id.clone()))
            .await;

        assert!(result.is_ok(), "Failed to publish event: {:?}", result.err());

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::JobCreated { job_id: id, job_name, provider_id: p_id } => {
                assert_eq!(id, &job_id);
                assert_eq!(job_name, "Test Job");
                assert_eq!(p_id, &Some(provider_id));
            }
            _ => panic!("Wrong event type published"),
        }
    }

    #[tokio::test]
    async fn test_publish_job_scheduled_event() {
        // Test: Should publish JobScheduled event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let job_id = JobId::new("test-job-2".to_string());
        let provider_id = ProviderId::new("provider-1".to_string());

        let result = orchestrator
            .publish_job_scheduled(job_id.clone(), provider_id.clone())
            .await;

        assert!(result.is_ok(), "Failed to publish event");

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::JobScheduled { job_id: id, provider_id: p_id } => {
                assert_eq!(id, &job_id);
                assert_eq!(p_id, &provider_id);
            }
            _ => panic!("Wrong event type published"),
        }
    }

    #[tokio::test]
    async fn test_publish_job_completed_event() {
        // Test: Should publish JobCompleted event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let job_id = JobId::new("test-job-3".to_string());
        let provider_id = ProviderId::new("provider-1".to_string());
        let execution_id = "exec-123".to_string();

        let result = orchestrator
            .publish_job_completed(
                job_id.clone(),
                provider_id.clone(),
                execution_id.clone(),
                true,
                Some("Success".to_string()),
                None,
                1500,
            )
            .await;

        assert!(result.is_ok(), "Failed to publish event");

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::JobCompleted {
                job_id: id,
                success,
                output,
                execution_time_ms,
                ..
            } => {
                assert_eq!(id, &job_id);
                assert!(*success);
                assert_eq!(output, &Some("Success".to_string()));
                assert_eq!(*execution_time_ms, 1500);
            }
            _ => panic!("Wrong event type published"),
        }
    }

    #[tokio::test]
    async fn test_publish_worker_connected_event() {
        // Test: Should publish WorkerConnected event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let provider_id = ProviderId::new("docker-provider".to_string());
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(2048),
            cpu_limit: Some(2.0),
        };

        let result = orchestrator
            .publish_worker_connected(provider_id.clone(), capabilities.clone())
            .await;

        assert!(result.is_ok(), "Failed to publish event");

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::WorkerConnected {
                provider_id: p_id,
                capabilities: caps,
            } => {
                assert_eq!(p_id, &provider_id);
                assert_eq!(caps.max_concurrent_jobs, 10);
                assert_eq!(caps.supported_job_types.len(), 1);
            }
            _ => panic!("Wrong event type published"),
        }
    }

    #[tokio::test]
    async fn test_publish_worker_heartbeat_event() {
        // Test: Should publish WorkerHeartbeat event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let provider_id = ProviderId::new("provider-1".to_string());
        let resource_usage = ResourceRequirements {
            memory_mb: Some(2048),
            cpu_cores: Some(2.0),
            timeout_seconds: Some(300),
        };

        let result = orchestrator
            .publish_worker_heartbeat(provider_id.clone(), 5, resource_usage.clone())
            .await;

        assert!(result.is_ok(), "Failed to publish event");

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::WorkerHeartbeat {
                provider_id: p_id,
                active_jobs,
                resource_usage: usage,
                ..
            } => {
                assert_eq!(p_id, &provider_id);
                assert_eq!(*active_jobs, 5);
                assert_eq!(usage.memory_mb, Some(2048));
            }
            _ => panic!("Wrong event type published"),
        }
    }

    #[tokio::test]
    async fn test_publish_batch_events() {
        // Test: Should publish multiple events in batch
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let job_id = JobId::new("batch-job".to_string());
        let provider_id = ProviderId::new("provider-1".to_string());

        let events_to_publish = vec![
            DomainEvent::JobCreated {
                job_id: job_id.clone(),
                job_name: "Batch Job".to_string(),
                provider_id: Some(provider_id.clone()),
            },
            DomainEvent::JobScheduled {
                job_id: job_id.clone(),
                provider_id: provider_id.clone(),
            },
        ];

        let result = orchestrator.publish_batch(events_to_publish).await;

        assert!(result.is_ok(), "Failed to publish batch events");

        let published = events.lock().await;
        assert_eq!(published.len(), 2, "Should have published 2 events");

        match &published[0] {
            DomainEvent::JobCreated { job_name, .. } => {
                assert_eq!(job_name, "Batch Job");
            }
            _ => panic!("Wrong event type"),
        }

        match &published[1] {
            DomainEvent::JobScheduled { job_id: id, .. } => {
                assert_eq!(id, &job_id);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_publish_job_cancelled_event() {
        // Test: Should publish JobCancelled event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let job_id = JobId::new("cancelled-job".to_string());
        let provider_id = ProviderId::new("provider-1".to_string());

        let result = orchestrator
            .publish_job_cancelled(job_id.clone(), provider_id.clone())
            .await;

        assert!(result.is_ok(), "Failed to publish event");

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::JobCancelled { job_id: id, provider_id: p_id } => {
                assert_eq!(id, &job_id);
                assert_eq!(p_id, &provider_id);
            }
            _ => panic!("Wrong event type published"),
        }
    }

    #[tokio::test]
    async fn test_publish_job_failed_event() {
        // Test: Should publish JobFailed event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let job_id = JobId::new("failed-job".to_string());
        let provider_id = ProviderId::new("provider-1".to_string());
        let execution_id = "exec-failed".to_string();

        let result = orchestrator
            .publish_job_failed(
                job_id.clone(),
                provider_id.clone(),
                execution_id.clone(),
                "Command not found".to_string(),
            )
            .await;

        assert!(result.is_ok(), "Failed to publish event");

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::JobFailed {
                job_id: id,
                error_message,
                ..
            } => {
                assert_eq!(id, &job_id);
                assert_eq!(error_message, "Command not found");
            }
            _ => panic!("Wrong event type published"),
        }
    }

    #[tokio::test]
    async fn test_publish_worker_disconnected_event() {
        // Test: Should publish WorkerDisconnected event
        let mock_publisher = MockEventPublisher::new();
        let events = mock_publisher.get_published_events();
        let orchestrator = EventOrchestrator::new(Box::new(mock_publisher));

        let provider_id = ProviderId::new("provider-disconnect".to_string());

        let result = orchestrator
            .publish_worker_disconnected(provider_id.clone())
            .await;

        assert!(result.is_ok(), "Failed to publish event");

        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::WorkerDisconnected { provider_id: p_id } => {
                assert_eq!(p_id, &provider_id);
            }
            _ => panic!("Wrong event type published"),
        }
    }
}
