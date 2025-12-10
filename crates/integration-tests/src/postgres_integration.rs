//! PostgreSQL Integration Tests
//!
//! Tests real PostgreSQL database operations using TestContainers

use infrastructure::{
    DatabaseConfig, PostgresJobRepository, PostgresProviderRepository,
};
use domain::shared_kernel::{DomainError, JobId, JobSpec, ProviderId, ProviderType, ProviderStatus};
use testcontainers::clients::Cli;
use testcontainers::core::WaitFor;
use testcontainers::images::postgres::Postgres;
use tokio::time::{Duration, Instant};

/// PostgreSQL TestContainer wrapper
pub struct PostgresContainer {
    container: testcontainers::Container<'static, Postgres>,
    connection_string: String,
}

impl PostgresContainer {
    /// Create a new PostgreSQL container
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let docker = Cli::default();

        let container = docker.run(Postgres::default().with_tag("15-alpine"));

        let connection_string = format!(
            "postgresql://postgres:postgres@{}:{}/postgres",
            container.get_host_ip().await?,
            container.get_host_port(5432).await?
        );

        // Wait for PostgreSQL to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(Self {
            container,
            connection_string,
        })
    }

    /// Get connection string
    pub fn connection_string(&self) -> String {
        self.connection_string.clone()
    }
}

impl Drop for PostgresContainer {
    fn drop(&mut self) {
        // Container is automatically dropped
    }
}

/// Test PostgreSQL repository integration
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_postgres_job_repository_save_and_find() {
        // Test: Should save and retrieve job from PostgreSQL
        let container = PostgresContainer::new().await.unwrap();
        let connection_string = container.connection_string();

        let config = DatabaseConfig {
            url: connection_string,
            max_connections: 10,
            min_connections: Some(5),
            idle_timeout: Some(Duration::from_secs(600)),
        };

        let repository = PostgresJobRepository::new(&config).await.unwrap();

        // Create and save a job
        let job_id = JobId::new("test-job-123".to_string());
        let job_spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string(), "hello".to_string()],
            vec![],
        );

        let mut job = domain::job_execution::Job::new(job_id.clone(), job_spec).unwrap();
        job.complete();

        // Save job
        let result = repository.save(&job).await;
        assert!(result.is_ok(), "Failed to save job: {:?}", result.err());

        // Find job
        let found_job = repository.find_by_id(&job_id).await.unwrap();
        assert!(found_job.is_some(), "Job not found");

        let found_job = found_job.unwrap();
        assert_eq!(found_job.id.to_string(), job_id.to_string());
    }

    #[tokio::test]
    async fn test_postgres_provider_repository_save_and_list() {
        // Test: Should save and list providers from PostgreSQL
        let container = PostgresContainer::new().await.unwrap();
        let connection_string = container.connection_string();

        let config = DatabaseConfig {
            url: connection_string,
            max_connections: 10,
            min_connections: Some(5),
            idle_timeout: Some(Duration::from_secs(600)),
        };

        let repository = PostgresProviderRepository::new(&config).await.unwrap();

        // Create and save a provider
        let provider_id = ProviderId::new("docker-provider-1".to_string());
        let provider = domain::provider_management::Provider::new(
            provider_id.clone(),
            "Docker Provider".to_string(),
            ProviderType::Docker,
            ProviderStatus::Active,
        ).unwrap();

        // Save provider
        let result = repository.save(&provider).await;
        assert!(result.is_ok(), "Failed to save provider: {:?}", result.err());

        // List providers
        let providers = repository.find_all().await.unwrap();
        assert_eq!(providers.len(), 1);

        let found_provider = &providers[0];
        assert_eq!(found_provider.id.to_string(), provider_id.to_string());
        assert_eq!(found_provider.name, "Docker Provider");
    }

    #[tokio::test]
    async fn test_postgres_repository_performance() {
        // Test: Should handle multiple concurrent operations efficiently
        let container = PostgresContainer::new().await.unwrap();
        let connection_string = container.connection_string();

        let config = DatabaseConfig {
            url: connection_string,
            max_connections: 10,
            min_connections: Some(5),
            idle_timeout: Some(Duration::from_secs(600)),
        };

        let job_repository = PostgresJobRepository::new(&config).await.unwrap();
        let provider_repository = PostgresProviderRepository::new(&config).await.unwrap();

        let start = Instant::now();

        // Create multiple providers
        for i in 0..10 {
            let provider_id = ProviderId::new(format!("provider-{}", i));
            let provider = domain::provider_management::Provider::new(
                provider_id,
                format!("Provider {}", i),
                ProviderType::Docker,
                ProviderStatus::Active,
            ).unwrap();

            let result = provider_repository.save(&provider).await;
            assert!(result.is_ok(), "Failed to save provider {}", i);
        }

        // Create multiple jobs
        for i in 0..50 {
            let job_id = JobId::new(format!("job-{}", i));
            let job_spec = JobSpec::new(
                format!("job-{}", i),
                vec!["echo".to_string(), "test".to_string()],
                vec![],
            );

            let mut job = domain::job_execution::Job::new(job_id, job_spec).unwrap();
            job.complete();

            let result = job_repository.save(&job).await;
            assert!(result.is_ok(), "Failed to save job {}", i);
        }

        let duration = start.elapsed();
        println!("Processed 60 operations in {:?}", duration);

        // Verify all were saved
        let providers = provider_repository.find_all().await.unwrap();
        assert_eq!(providers.len(), 10);

        // Performance assertion (should complete in reasonable time)
        assert!(duration.as_secs() < 30, "Operations took too long: {:?}", duration);
    }
}
