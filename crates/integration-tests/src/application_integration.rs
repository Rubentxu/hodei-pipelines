//! Application Layer Integration Tests
//!
//! Tests the integration between JobService, ProviderService, and EventOrchestrator

#[cfg(test)]
mod tests {
    use application::{JobService, ProviderService, EventOrchestrator};
    use domain::job_execution::{JobRepository, JobSpec, Job};
    use domain::provider_management::{ProviderRepository, Provider};
    use domain::shared_kernel::{
        DomainEvent, EventPublisher, EventError,
        JobId, ProviderId, ProviderType, ProviderCapabilities,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

// Mock implementations
struct InMemoryJobRepository {
    jobs: Arc<Mutex<Vec<domain::job_execution::Job>>>,
}

impl InMemoryJobRepository {
    fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn save(&self, job: &domain::job_execution::Job) -> domain::shared_kernel::DomainResult<()> {
        let mut jobs = self.jobs.lock().await;

        if let Some(index) = jobs.iter().position(|j| j.id == job.id) {
            jobs[index] = job.clone();
        } else {
            jobs.push(job.clone());
        }

        Ok(())
    }

    async fn find_by_id(&self, id: &JobId) -> domain::shared_kernel::DomainResult<Option<domain::job_execution::Job>> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.iter().find(|j| j.id == *id).cloned())
    }

    async fn list(&self) -> domain::shared_kernel::DomainResult<Vec<domain::job_execution::Job>> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.clone())
    }

    async fn delete(&self, id: &JobId) -> domain::shared_kernel::DomainResult<()> {
        let mut jobs = self.jobs.lock().await;
        jobs.retain(|j| j.id != *id);
        Ok(())
    }
}

struct InMemoryProviderRepository {
    providers: Arc<Mutex<Vec<Provider>>>,
}

impl InMemoryProviderRepository {
    fn new() -> Self {
        Self {
            providers: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl ProviderRepository for InMemoryProviderRepository {
    async fn save(&self, provider: &Provider) -> domain::shared_kernel::DomainResult<()> {
        let mut providers = self.providers.lock().await;

        if let Some(index) = providers.iter().position(|p| p.id == provider.id) {
            providers[index] = provider.clone();
        } else {
            providers.push(provider.clone());
        }

        Ok(())
    }

    async fn find_by_id(&self, id: &ProviderId) -> domain::shared_kernel::DomainResult<Option<Provider>> {
        let providers = self.providers.lock().await;
        Ok(providers.iter().find(|p| p.id == *id).cloned())
    }

    async fn list(&self) -> domain::shared_kernel::DomainResult<Vec<Provider>> {
        let providers = self.providers.lock().await;
        Ok(providers.clone())
    }

    async fn delete(&self, id: &ProviderId) -> domain::shared_kernel::DomainResult<()> {
        let mut providers = self.providers.lock().await;
        providers.retain(|p| p.id != *id);
        Ok(())
    }
}

struct InMemoryEventPublisher {
    published_events: Arc<Mutex<Vec<DomainEvent>>>,
}

impl InMemoryEventPublisher {
    fn new() -> Self {
        Self {
            published_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_events(&self) -> Arc<Mutex<Vec<DomainEvent>>> {
        self.published_events.clone()
    }
}

#[async_trait::async_trait]
impl EventPublisher for InMemoryEventPublisher {
    async fn publish(&self, event: &DomainEvent) -> Result<(), EventError> {
        let mut events = self.published_events.lock().await;
        events.push(event.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_create_job_and_publish_event() {
        // Test: Integration between JobService and EventOrchestrator
        let job_repo = InMemoryJobRepository::new();
        let event_publisher = InMemoryEventPublisher::new();
        let events = event_publisher.get_events();

        let job_service = JobService::new(Box::new(job_repo));
        let event_orchestrator = EventOrchestrator::new(Box::new(event_publisher));

        // Create a job
        let job_spec = JobSpec::new(
            "integration-test-job".to_string(),
            vec!["echo".to_string(), "test".to_string()],
            vec![],
        );

        let job = job_service.create_job(job_spec).await.unwrap();

        // Publish JobCreated event
        event_orchestrator
            .publish_job_created(
                job.id.clone(),
                job.spec.name.clone(),
                None,
            )
            .await
            .unwrap();

        // Verify event was published
        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::JobCreated { job_id, job_name, .. } => {
                assert_eq!(job_id, &job.id);
                assert_eq!(job_name, "integration-test-job");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_register_provider_and_list_jobs() {
        // Test: Integration between ProviderService and JobService
        let provider_repo = InMemoryProviderRepository::new();
        let job_repo = InMemoryJobRepository::new();

        let provider_service = ProviderService::new(Box::new(provider_repo));
        let job_service = JobService::new(Box::new(job_repo));

        // Register a provider
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string(), "python".to_string()],
            memory_limit_mb: Some(2048),
            cpu_limit: Some(2.0),
        };

        let provider = provider_service
            .register_provider(
                ProviderId::new("docker-1".to_string()),
                "Docker Provider".to_string(),
                ProviderType::Docker,
                capabilities,
                domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
            )
            .await
            .unwrap();

        // Verify provider was registered
        assert_eq!(provider.name, "Docker Provider");

        // Create multiple jobs
        for i in 0..3 {
            let job_spec = JobSpec::new(
                format!("job-{}", i),
                vec!["echo".to_string(), format!("job-{}", i)],
                vec![],
            );

            job_service.create_job(job_spec).await.unwrap();
        }

        // Verify jobs were created
        let jobs = job_service.list_jobs().await.unwrap();
        assert_eq!(jobs.len(), 3, "Should have 3 jobs");

        // Verify provider still exists
        let providers = provider_service.list_providers().await.unwrap();
        assert_eq!(providers.len(), 1, "Should have 1 provider");
        assert_eq!(providers[0].name, "Docker Provider");
    }

    #[tokio::test]
    async fn test_job_lifecycle_with_events() {
        // Test: Complete job lifecycle with event publishing
        let job_repo = InMemoryJobRepository::new();
        let provider_repo = InMemoryProviderRepository::new();
        let event_publisher = InMemoryEventPublisher::new();
        let events = event_publisher.get_events();

        let job_service = JobService::new(Box::new(job_repo));
        let provider_service = ProviderService::new(Box::new(provider_repo));
        let event_orchestrator = EventOrchestrator::new(Box::new(event_publisher));

        // Register provider
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 5,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };

        let provider = provider_service
            .register_provider(
                ProviderId::new("test-provider".to_string()),
                "Test Provider".to_string(),
                ProviderType::Docker,
                capabilities,
                domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
            )
            .await
            .unwrap();

        // Create job
        let job_spec = JobSpec::new(
            "lifecycle-test".to_string(),
            vec!["sleep".to_string(), "1".to_string()],
            vec![],
        );

        let job = job_service.create_job(job_spec).await.unwrap();
        event_orchestrator
            .publish_job_created(job.id.clone(), job.spec.name.clone(), Some(provider.id.clone()))
            .await
            .unwrap();

        // Execute job
        job_service.execute_job(&job.id).await.unwrap();
        event_orchestrator
            .publish_job_scheduled(job.id.clone(), provider.id.clone())
            .await
            .unwrap();

        // Complete job
        event_orchestrator
            .publish_job_completed(
                job.id.clone(),
                provider.id.clone(),
                "exec-123".to_string(),
                true,
                Some("Success".to_string()),
                None,
                1000,
            )
            .await
            .unwrap();

        // Verify all events were published
        let published = events.lock().await;
        assert_eq!(published.len(), 3, "Should have published 3 events");

        // Verify event sequence
        match &published[0] {
            DomainEvent::JobCreated { job_name, .. } => {
                assert_eq!(job_name, "lifecycle-test");
            }
            _ => panic!("First event should be JobCreated"),
        }

        match &published[1] {
            DomainEvent::JobScheduled { job_id, .. } => {
                assert_eq!(job_id, &job.id);
            }
            _ => panic!("Second event should be JobScheduled"),
        }

        match &published[2] {
            DomainEvent::JobCompleted { success, .. } => {
                assert!(*success);
            }
            _ => panic!("Third event should be JobCompleted"),
        }
    }

    #[tokio::test]
    async fn test_provider_activation_with_heartbeat() {
        // Test: Provider lifecycle with heartbeat events
        let provider_repo = InMemoryProviderRepository::new();
        let event_publisher = InMemoryEventPublisher::new();
        let events = event_publisher.get_events();

        let provider_service = ProviderService::new(Box::new(provider_repo));
        let event_orchestrator = EventOrchestrator::new(Box::new(event_publisher));

        // Register provider
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 20,
            supported_job_types: vec!["bash".to_string(), "sh".to_string()],
            memory_limit_mb: Some(4096),
            cpu_limit: Some(4.0),
        };

        let provider_id = ProviderId::new("heartbeat-provider".to_string());
        provider_service
            .register_provider(
                provider_id.clone(),
                "Heartbeat Provider".to_string(),
                ProviderType::Docker,
                capabilities,
                domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
            )
            .await
            .unwrap();

        // Publish WorkerConnected event
        event_orchestrator
            .publish_worker_connected(provider_id.clone(), capabilities.clone())
            .await
            .unwrap();

        // Simulate heartbeats
        for i in 0..3 {
            event_orchestrator
                .publish_worker_heartbeat(
                    provider_id.clone(),
                    i * 2,
                    domain::shared_kernel::types::ResourceRequirements {
                        memory_mb: Some(1024 * (i + 1)),
                        cpu_cores: Some(1.0 + i as f64),
                        timeout_seconds: Some(300),
                    },
                )
                .await
                .unwrap();
        }

        // Deactivate provider
        provider_service.deactivate_provider(&provider_id).await.unwrap();

        // Publish WorkerDisconnected event
        event_orchestrator
            .publish_worker_disconnected(provider_id.clone())
            .await
            .unwrap();

        // Verify all events were published
        let published = events.lock().await;
        assert_eq!(published.len(), 5, "Should have published 5 events");

        // Verify event types
        let event_types: Vec<_> = published.iter().map(|e| match e {
            DomainEvent::WorkerConnected { .. } => "WorkerConnected",
            DomainEvent::WorkerHeartbeat { .. } => "WorkerHeartbeat",
            DomainEvent::WorkerDisconnected { .. } => "WorkerDisconnected",
            _ => "Other",
        }).collect();

        assert_eq!(event_types[0], "WorkerConnected");
        assert_eq!(event_types[1], "WorkerHeartbeat");
        assert_eq!(event_types[2], "WorkerHeartbeat");
        assert_eq!(event_types[3], "WorkerHeartbeat");
        assert_eq!(event_types[4], "WorkerDisconnected");

        // Verify provider is inactive
        let status = provider_service.get_provider_status(&provider_id).await.unwrap();
        assert!(matches!(status, domain::provider_management::entities::ProviderStatus::Inactive));
    }

    #[tokio::test]
    async fn test_job_execution_failure_handling() {
        // Test: Job failure handling with error events
        let job_repo = InMemoryJobRepository::new();
        let provider_repo = InMemoryProviderRepository::new();
        let event_publisher = InMemoryEventPublisher::new();
        let events = event_publisher.get_events();

        let job_service = JobService::new(Box::new(job_repo));
        let provider_service = ProviderService::new(Box::new(provider_repo));
        let event_orchestrator = EventOrchestrator::new(Box::new(event_publisher));

        // Register provider
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 5,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };

        let provider = provider_service
            .register_provider(
                ProviderId::new("failure-provider".to_string()),
                "Failure Provider".to_string(),
                ProviderType::Docker,
                capabilities,
                domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
            )
            .await
            .unwrap();

        // Create job
        let job_spec = JobSpec::new(
            "failing-job".to_string(),
            vec!["false".to_string()], // Command that fails
            vec![],
        );

        let job = job_service.create_job(job_spec).await.unwrap();

        // Execute job
        job_service.execute_job(&job.id).await.unwrap();

        // Simulate failure
        event_orchestrator
            .publish_job_failed(
                job.id.clone(),
                provider.id.clone(),
                "exec-fail-123".to_string(),
                "Command exited with status 1".to_string(),
            )
            .await
            .unwrap();

        // Verify failure event was published
        let published = events.lock().await;
        assert_eq!(published.len(), 1, "Should have published 1 event");

        match &published[0] {
            DomainEvent::JobFailed { job_id, error_message, .. } => {
                assert_eq!(job_id, &job.id);
                assert!(error_message.contains("Command exited"));
            }
            _ => panic!("Should be JobFailed event"),
        }
    }

    #[tokio::test]
    async fn test_batch_event_publishing() {
        // Test: Batch event publishing from orchestrator
        let event_publisher = InMemoryEventPublisher::new();
        let events = event_publisher.get_events();

        let event_orchestrator = EventOrchestrator::new(Box::new(event_publisher));

        // Create batch of events
        let job_id = JobId::new("batch-job-123".to_string());
        let provider_id = ProviderId::new("batch-provider".to_string());

        let batch_events = vec![
            DomainEvent::JobCreated {
                job_id: job_id.clone(),
                job_name: "Batch Job".to_string(),
                provider_id: Some(provider_id.clone()),
            },
            DomainEvent::JobScheduled {
                job_id: job_id.clone(),
                provider_id: provider_id.clone(),
            },
            DomainEvent::JobStarted {
                job_id: job_id.clone(),
                provider_id: provider_id.clone(),
                execution_id: "batch-exec".to_string(),
            },
        ];

        // Publish batch
        event_orchestrator.publish_batch(batch_events).await.unwrap();

        // Verify all events were published
        let published = events.lock().await;
        assert_eq!(published.len(), 3, "Should have published 3 events");

        // Verify event sequence
        assert!(matches!(published[0], DomainEvent::JobCreated { .. }));
        assert!(matches!(published[1], DomainEvent::JobScheduled { .. }));
        assert!(matches!(published[2], DomainEvent::JobStarted { .. }));
    }
}
