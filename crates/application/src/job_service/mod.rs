//! Application Service for Job Management

use domain::job_execution::{Job, JobRepository, JobSpec};
use domain::provider_management::{Provider, ProviderRepository};
use domain::shared_kernel::{DomainError, DomainResult, JobId, JobResult, JobState};

pub struct JobService {
    job_repo: Box<dyn JobRepository>,
    provider_repo: Option<Box<dyn ProviderRepository>>,
}

impl JobService {
    pub fn new(job_repo: Box<dyn JobRepository>) -> Self {
        Self {
            job_repo,
            provider_repo: None,
        }
    }

    pub fn with_providers(
        job_repo: Box<dyn JobRepository>,
        provider_repo: Box<dyn ProviderRepository>,
    ) -> Self {
        Self {
            job_repo,
            provider_repo: Some(provider_repo),
        }
    }

    pub async fn create_job(&self, spec: JobSpec) -> DomainResult<Job> {
        let job = Job::new(JobId::new(uuid::Uuid::new_v4().to_string()), spec)?;
        self.job_repo.save(&job).await?;
        Ok(job)
    }

    pub async fn execute_job(&self, job_id: &JobId) -> DomainResult<Job> {
        let mut job = self
            .job_repo
            .find_by_id(job_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Job not found".to_string()))?;

        // For MVP, just mark as pending
        job.state = JobState::Pending;
        self.job_repo.save(&job).await?;

        Ok(job)
    }

    pub async fn get_job(&self, job_id: &JobId) -> DomainResult<Job> {
        self.job_repo
            .find_by_id(job_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Job not found".to_string()))
    }

    pub async fn get_job_result(&self, job_id: &JobId) -> DomainResult<Option<JobResult>> {
        let job = self.get_job(job_id).await?;

        // For MVP, return None
        Ok(None)
    }

    pub async fn cancel_job(&self, job_id: &JobId) -> DomainResult<()> {
        let mut job = self.get_job(job_id).await?;

        job.state = JobState::Cancelled;
        self.job_repo.save(&job).await?;

        Ok(())
    }

    pub async fn list_jobs(&self) -> DomainResult<Vec<Job>> {
        self.job_repo.list().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job_execution::{Job, JobRepository, JobSpec};
    use domain::provider_management::{Provider, ProviderRepository};
    use domain::provider_management::entities::ProviderStatus;
    use domain::shared_kernel::{JobId, ProviderId, JobResult, ProviderType, ProviderCapabilities};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock implementations for testing
    struct MockJobRepository {
        jobs: Arc<Mutex<Vec<Job>>>,
    }

    impl MockJobRepository {
        fn new() -> Self {
            Self {
                jobs: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl JobRepository for MockJobRepository {
        async fn save(&self, job: &Job) -> domain::shared_kernel::DomainResult<()> {
            let mut jobs = self.jobs.lock().await;

            // Check if job already exists
            if let Some(index) = jobs.iter().position(|j| j.id == job.id) {
                jobs[index] = job.clone();
            } else {
                jobs.push(job.clone());
            }

            Ok(())
        }

        async fn find_by_id(&self, id: &JobId) -> domain::shared_kernel::DomainResult<Option<Job>> {
            let jobs = self.jobs.lock().await;
            Ok(jobs.iter().find(|j| j.id == *id).cloned())
        }

        async fn list(&self) -> domain::shared_kernel::DomainResult<Vec<Job>> {
            let jobs = self.jobs.lock().await;
            Ok(jobs.clone())
        }

        async fn delete(&self, id: &JobId) -> domain::shared_kernel::DomainResult<()> {
            let mut jobs = self.jobs.lock().await;
            jobs.retain(|j| j.id != *id);
            Ok(())
        }
    }

    struct MockProviderRepository {
        providers: Arc<Mutex<Vec<Provider>>>,
    }

    impl MockProviderRepository {
        fn new() -> Self {
            Self {
                providers: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl ProviderRepository for MockProviderRepository {
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

    // ==================== TDD RED PHASE ====================
    // Writing tests BEFORE implementation

    #[tokio::test]
    async fn test_create_job_success() {
        // Test: Should create a new job successfully
        let repo = MockJobRepository::new();
        let service = JobService::new(Box::new(repo));

        let job_spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string(), "hello".to_string()],
            vec![],
        );

        let result = service.create_job(job_spec).await;
        assert!(result.is_ok(), "Failed to create job: {:?}", result.err());

        let job = result.unwrap();
        assert_eq!(job.spec.name, "test-job");
        assert!(!job.id.to_string().is_empty());
    }

    #[tokio::test]
    async fn test_create_job_with_empty_name() {
        // Test: Should reject job with empty name
        let repo = MockJobRepository::new();
        let service = JobService::new(Box::new(repo));

        let job_spec = JobSpec::new(
            "".to_string(),
            vec!["echo".to_string(), "hello".to_string()],
            vec![],
        );

        let result = service.create_job(job_spec).await;
        assert!(result.is_err(), "Should reject empty job name");
    }

    #[tokio::test]
    async fn test_execute_job_success() {
        // Test: Should execute job successfully
        let job_repo = MockJobRepository::new();
        let provider_repo = MockProviderRepository::new();

        // Create a test provider
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };
        let provider = Provider::new(
            ProviderId::new("docker-provider-1".to_string()),
            "Docker Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
        );
        provider_repo.save(&provider).await.unwrap();

        let service = JobService::with_providers(
            Box::new(job_repo),
            Box::new(provider_repo),
        );

        // Create a job
        let job_spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string(), "hello".to_string()],
            vec![],
        );

        let job = service.create_job(job_spec).await.unwrap();

        // Execute the job
        let result = service.execute_job(&job.id).await;
        assert!(result.is_ok(), "Failed to execute job: {:?}", result.err());

        let executed_job = result.unwrap();
        assert!(matches!(executed_job.state, domain::shared_kernel::JobState::Pending));
    }

    #[tokio::test]
    async fn test_get_job_result_success() {
        // Test: Should retrieve job result
        let repo = MockJobRepository::new();
        let service = JobService::new(Box::new(repo));

        let job_spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string(), "hello".to_string()],
            vec![],
        );

        let job = service.create_job(job_spec).await.unwrap();

        let result = service.get_job_result(&job.id).await;
        assert!(result.is_ok(), "Failed to get job result");

        let job_result = result.unwrap();
        assert!(job_result.is_none(), "New job should not have result yet");
    }

    #[tokio::test]
    async fn test_cancel_job_success() {
        // Test: Should cancel a job
        let repo = MockJobRepository::new();
        let service = JobService::new(Box::new(repo));

        let job_spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string(), "hello".to_string()],
            vec![],
        );

        let job = service.create_job(job_spec).await.unwrap();

        let result = service.cancel_job(&job.id).await;
        assert!(result.is_ok(), "Failed to cancel job: {:?}", result.err());

        let cancelled_job = service.get_job(&job.id).await.unwrap();
        assert!(matches!(cancelled_job.state, domain::shared_kernel::JobState::Cancelled));
    }

    #[tokio::test]
    async fn test_list_jobs() {
        // Test: Should list all jobs
        let repo = MockJobRepository::new();
        let service = JobService::new(Box::new(repo));

        // Create multiple jobs
        for i in 0..5 {
            let job_spec = JobSpec::new(
                format!("test-job-{}", i),
                vec!["echo".to_string(), format!("hello {}", i)],
                vec![],
            );

            service.create_job(job_spec).await.unwrap();
        }

        let jobs = service.list_jobs().await.unwrap();
        assert_eq!(jobs.len(), 5, "Should have 5 jobs");

        for (i, job) in jobs.iter().enumerate() {
            assert_eq!(job.spec.name, format!("test-job-{}", i));
        }
    }

    #[tokio::test]
    async fn test_get_nonexistent_job() {
        // Test: Should return error for non-existent job
        let repo = MockJobRepository::new();
        let service = JobService::new(Box::new(repo));

        let fake_id = JobId::new("nonexistent".to_string());
        let result = service.get_job(&fake_id).await;
        assert!(result.is_err(), "Should error for non-existent job");
    }
}
