#[cfg(test)]
mod tests {
    use hodei_shared_types::job_definitions::{JobId, JobSpec, ResourceQuota};
    use hodei_shared_types::{CorrelationId, DomainError, TenantId};
    use std::collections::HashMap;

    #[test]
    fn test_job_creation_with_valid_spec() {
        // Arrange
        let spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::new(100, 512),
            timeout_ms: 30000,
            retries: 2,
            env: HashMap::new(),
            secret_refs: vec![],
        };
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());

        // Act
        let result = Job::create(spec, correlation_id, tenant_id);

        // Assert
        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.created_at.is_some());
    }

    #[test]
    fn test_job_creation_with_empty_name() {
        // Arrange
        let mut spec = JobSpec {
            name: "".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::new(100, 512),
            timeout_ms: 30000,
            retries: 2,
            env: HashMap::new(),
            secret_refs: vec![],
        };
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());

        // Act
        let result = Job::create(spec, correlation_id, tenant_id);

        // Assert
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, DomainError::Validation(_)));
        }
    }

    #[test]
    fn test_job_state_transition_from_pending_to_scheduled() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec, correlation_id, tenant_id).unwrap();

        // Act
        let events = job.schedule().unwrap();

        // Assert
        assert_eq!(job.state.as_str(), JobState::SCHEDULED);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_job_state_transition_from_scheduled_to_running() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec, correlation_id, tenant_id).unwrap();
        job.schedule().unwrap();

        // Act
        let events = job.start().unwrap();

        // Assert
        assert_eq!(job.state.as_str(), JobState::RUNNING);
        assert!(job.started_at.is_some());
        assert!(!events.is_empty());
    }

    #[test]
    fn test_job_state_transition_from_running_to_success() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec, correlation_id, tenant_id).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();

        // Act
        let events = job.complete().unwrap();

        // Assert
        assert_eq!(job.state.as_str(), JobState::SUCCESS);
        assert!(job.completed_at.is_some());
        assert!(!events.is_empty());
    }

    #[test]
    fn test_job_state_transition_from_running_to_failed() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec, correlation_id, tenant_id).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();

        // Act
        let events = job.fail("Command failed".to_string(), true).unwrap();

        // Assert
        assert_eq!(job.state.as_str(), JobState::FAILED);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_invalid_state_transition_from_running_to_pending() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec, correlation_id, tenant_id).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();

        // Act
        let result = job.transition_to(&JobState::from(JobState::PENDING.to_string()));

        // Assert
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, DomainError::InvalidStateTransition { .. }));
        }
    }

    #[test]
    fn test_job_cancellation_from_pending() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec, correlation_id, tenant_id).unwrap();

        // Act
        let events = job.cancel().unwrap();

        // Assert
        assert_eq!(job.state.as_str(), JobState::CANCELLED);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_job_retry_from_failed() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec, correlation_id, tenant_id).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();
        job.fail("Command failed".to_string(), true).unwrap();

        // Act
        let events = job.retry().unwrap();

        // Assert
        assert_eq!(job.state.as_str(), JobState::PENDING);
        assert!(job.attempts > 1);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_job_with_max_retries() {
        // Arrange
        let spec = create_test_spec();
        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test-tenant".to_string());
        let mut job = Job::create(spec.clone(), correlation_id.clone(), tenant_id.clone()).unwrap();
        job.schedule().unwrap();
        job.start().unwrap();
        job.fail("Command failed".to_string(), true).unwrap();
        job.retry().unwrap();

        let mut second_job = Job::create(spec, correlation_id, tenant_id).unwrap();
        second_job.schedule().unwrap();
        second_job.start().unwrap();
        second_job.fail("Command failed".to_string(), true).unwrap();

        // Act
        let events = second_job.retry();

        // Assert - should fail due to max retries
        assert!(events.is_err());
    }

    fn create_test_spec() -> JobSpec {
        JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::new(100, 512),
            timeout_ms: 30000,
            retries: 2,
            env: HashMap::new(),
            secret_refs: vec![],
        }
    }
}
