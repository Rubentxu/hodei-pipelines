//! Docker Provider Integration Tests
//!
//! Tests Docker provider operations (requires Docker daemon)

use infrastructure::DockerProviderAdapter;
use domain::shared_kernel::{JobId, JobSpec, ProviderId};
use tokio::time::Duration;

/// Docker Provider integration tests
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_docker_provider_submit_job() {
        // Test: Should submit job to Docker provider
        let provider_id = ProviderId::new("docker-provider-test".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let job_id = JobId::new("test-job-123".to_string());
        let job_spec = JobSpec::new(
            "echo-job".to_string(),
            vec!["echo".to_string(), "Hello from Docker".to_string()],
            vec![],
        );

        let result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(result.is_ok(), "Failed to submit job: {:?}", result.err());

        let execution_id = result.unwrap();
        assert!(!execution_id.is_empty(), "Execution ID is empty");
    }

    #[tokio::test]
    async fn test_docker_provider_job_with_env() {
        // Test: Should handle job with environment variables
        let provider_id = ProviderId::new("docker-provider-env".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let job_id = JobId::new("env-job-456".to_string());
        let job_spec = JobSpec::new(
            "env-test".to_string(),
            vec!["printenv".to_string(), "TEST_VAR".to_string()],
            vec![],
        );

        let result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(result.is_ok(), "Failed to submit job with env");

        let execution_id = result.unwrap();
        assert!(!execution_id.is_empty());
    }

    #[tokio::test]
    async fn test_docker_provider_invalid_spec() {
        // Test: Should reject invalid job spec
        let provider_id = ProviderId::new("docker-provider-invalid".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let job_id = JobId::new("invalid-job".to_string());
        let job_spec = JobSpec::new(
            "invalid-job".to_string(),
            vec![], // Empty command list
            vec![],
        );

        let result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(result.is_err(), "Should reject empty command list");

        if let Err(e) = result {
            assert!(e.to_string().contains("must contain"), "Error message should mention requirements");
        }
    }

    #[tokio::test]
    async fn test_docker_provider_get_status() {
        // Test: Should get execution status
        let provider_id = ProviderId::new("docker-provider-status".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let job_id = JobId::new("status-job".to_string());
        let job_spec = JobSpec::new(
            "status-test".to_string(),
            vec!["sleep".to_string(), "1".to_string()],
            vec![],
        );

        // Submit job
        let result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(result.is_ok());

        let execution_id = result.unwrap();

        // Get status
        let status = adapter.get_execution_status(&execution_id).await;
        assert!(status.is_ok(), "Failed to get execution status");

        let execution_status = status.unwrap();
        assert!(
            matches!(execution_status, domain::shared_kernel::ExecutionStatus::Queued)
                || matches!(execution_status, domain::shared_kernel::ExecutionStatus::Running),
            "Status should be Queued or Running"
        );
    }

    #[tokio::test]
    async fn test_docker_provider_cancel_job() {
        // Test: Should cancel running job
        let provider_id = ProviderId::new("docker-provider-cancel".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let job_id = JobId::new("cancel-job".to_string());
        let job_spec = JobSpec::new(
            "cancel-test".to_string(),
            vec!["sleep".to_string(), "10".to_string()],
            vec![],
        );

        // Submit job
        let result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(result.is_ok());

        let execution_id = result.unwrap();

        // Cancel job
        let cancel_result = adapter.cancel_job(&execution_id).await;
        assert!(cancel_result.is_ok(), "Failed to cancel job");
    }

    #[tokio::test]
    async fn test_docker_provider_multiple_jobs() {
        // Test: Should handle multiple concurrent jobs
        let provider_id = ProviderId::new("docker-provider-multi".to_string());
        let adapter = DockerProviderAdapter::new(provider_id);

        let mut handles = vec![];

        // Submit multiple jobs concurrently
        for i in 0..10 {
            let adapter_clone = adapter.clone();
            let job_id = JobId::new(format!("multi-job-{}", i));
            let job_spec = JobSpec::new(
                format!("multi-job-{}", i),
                vec!["echo".to_string(), format!("Job {}", i)],
                vec![],
            );

            let handle = tokio::spawn(async move {
                let result = adapter_clone.submit_job(&job_id, &job_spec).await;
                (i, result)
            });

            handles.push(handle);
        }

        // Wait for all jobs to complete
        for handle in handles {
            let (i, result) = handle.await.unwrap();
            assert!(result.is_ok(), "Job {} failed: {:?}", i, result.err());
        }
    }

    #[tokio::test]
    async fn test_docker_provider_with_custom_socket() {
        // Test: Should use custom Docker socket path
        let provider_id = ProviderId::new("docker-provider-custom".to_string());
        let adapter = DockerProviderAdapter::new(provider_id)
            .with_socket("/var/run/docker.sock".to_string());

        let job_id = JobId::new("custom-socket-job".to_string());
        let job_spec = JobSpec::new(
            "custom-socket-test".to_string(),
            vec!["echo".to_string(), "Custom socket".to_string()],
            vec![],
        );

        let result = adapter.submit_job(&job_id, &job_spec).await;
        assert!(result.is_ok(), "Failed to submit job with custom socket");

        let execution_id = result.unwrap();
        assert!(!execution_id.is_empty());
    }
}
