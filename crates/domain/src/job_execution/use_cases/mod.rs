//! Job Execution Use Cases
//!
//! Application-level use cases for job lifecycle management

use super::{Job, JobRepository, JobScheduler, JobSpec};
use crate::shared_kernel::DomainResult;

/// Use case for creating a new job
pub struct CreateJobUseCase {
    job_repo: Box<dyn JobRepository>,
}

impl CreateJobUseCase {
    pub fn new(job_repo: Box<dyn JobRepository>) -> Self {
        Self { job_repo }
    }

    pub async fn execute(&self, spec: JobSpec) -> DomainResult<Job> {
        let job = Job::new(
            crate::shared_kernel::JobId::new(uuid::Uuid::new_v4().to_string()),
            spec,
        )?;

        self.job_repo.save(&job).await?;
        Ok(job)
    }
}

/// Use case for executing a job
pub struct ExecuteJobUseCase {
    job_repo: Box<dyn JobRepository>,
    scheduler: JobScheduler,
}

impl ExecuteJobUseCase {
    pub fn new(job_repo: Box<dyn JobRepository>) -> Self {
        Self {
            job_repo,
            scheduler: JobScheduler::new(),
        }
    }

    pub async fn execute(
        &self,
        job_id: &crate::shared_kernel::JobId,
        available_providers: Vec<super::services::ProviderInfo>,
    ) -> DomainResult<Job> {
        let mut job = self.job_repo.find_by_id(job_id).await?.ok_or_else(|| {
            crate::shared_kernel::DomainError::NotFound("Job not found".to_string())
        })?;

        let provider_id = self.scheduler.select_provider(&job, &available_providers)?;

        let context = super::value_objects::ExecutionContext::new(
            job.id.clone(),
            provider_id.clone(),
            format!("exec-{}", chrono::Utc::now().timestamp()),
        );

        job.submit_to_provider(provider_id, context);
        self.job_repo.save(&job).await?;

        Ok(job)
    }
}

/// Use case for getting job results
pub struct GetJobResultUseCase {
    job_repo: Box<dyn JobRepository>,
}

impl GetJobResultUseCase {
    pub fn new(job_repo: Box<dyn JobRepository>) -> Self {
        Self { job_repo }
    }

    pub async fn execute(
        &self,
        job_id: &crate::shared_kernel::JobId,
    ) -> DomainResult<Option<crate::shared_kernel::JobResult>> {
        let job = self.job_repo.find_by_id(job_id).await?.ok_or_else(|| {
            crate::shared_kernel::DomainError::NotFound("Job not found".to_string())
        })?;

        Ok(job.execution_context.and_then(|ctx| ctx.result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_execution::services::ProviderInfo;
    use crate::job_execution::{Job, JobRepository, JobScheduler, JobSpec};
    use crate::shared_kernel::{DomainError, JobId, ProviderId, ProviderType};
    use std::sync::Arc;

    struct MockJobRepository {
        jobs: std::sync::Mutex<std::collections::HashMap<String, Job>>,
    }

    impl MockJobRepository {
        fn new() -> Self {
            Self {
                jobs: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl JobRepository for MockJobRepository {
        async fn save(&self, job: &Job) -> crate::shared_kernel::DomainResult<()> {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.insert(job.id.to_string(), job.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &JobId) -> crate::shared_kernel::DomainResult<Option<Job>> {
            let jobs = self.jobs.lock().unwrap();
            Ok(jobs.get(&id.to_string()).cloned())
        }

        async fn list(&self) -> crate::shared_kernel::DomainResult<Vec<Job>> {
            let jobs = self.jobs.lock().unwrap();
            Ok(jobs.values().cloned().collect())
        }

        async fn delete(&self, id: &JobId) -> crate::shared_kernel::DomainResult<()> {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.remove(&id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_create_job_use_case_success() {
        let repo = MockJobRepository::new();
        let use_case = CreateJobUseCase::new(Box::new(repo));

        let spec = JobSpec::new(
            "test-job".to_string(),
            vec!["echo".to_string()],
            vec!["hello".to_string()],
        );

        let result = use_case.execute(spec).await.unwrap();

        assert_eq!(result.spec.name, "test-job");
        assert_eq!(result.state, crate::shared_kernel::JobState::Pending);
    }

    #[tokio::test]
    async fn test_create_job_use_case_empty_name_fails() {
        let repo = MockJobRepository::new();
        let use_case = CreateJobUseCase::new(Box::new(repo));

        let spec = JobSpec::new(
            "".to_string(),
            vec!["echo".to_string()],
            vec!["hello".to_string()],
        );

        let result = use_case.execute(spec).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, DomainError::Validation(_)));
        }
    }

    #[tokio::test]
    async fn test_execute_job_use_case_success() {
        let repo = MockJobRepository::new();
        let job_id = JobId::new("job-123".to_string());

        // Pre-populate repository with a job
        {
            let spec = JobSpec::new("test-job".to_string(), vec!["echo".to_string()], vec![]);
            let job = Job::new(job_id.clone(), spec).unwrap();
            let mut jobs = repo.jobs.lock().unwrap();
            jobs.insert(job_id.to_string(), job);
        }

        let use_case = ExecuteJobUseCase::new(Box::new(repo));

        let provider_info = ProviderInfo {
            id: ProviderId::new("provider-1".to_string()),
            provider_type: ProviderType::Docker,
            capabilities: crate::shared_kernel::ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["docker".to_string()],
                memory_limit_mb: Some(4096),
                cpu_limit: Some(2.0),
            },
            current_jobs: 0,
        };

        let result = use_case
            .execute(&job_id, vec![provider_info])
            .await
            .unwrap();

        assert_eq!(result.id, job_id);
        assert_eq!(result.state, crate::shared_kernel::JobState::Running);
        assert!(result.execution_context.is_some());
    }

    #[tokio::test]
    async fn test_execute_job_use_case_job_not_found() {
        let repo = MockJobRepository::new();
        let use_case = ExecuteJobUseCase::new(Box::new(repo));

        let job_id = JobId::new("non-existent-job".to_string());
        let provider_info = ProviderInfo {
            id: ProviderId::new("provider-1".to_string()),
            provider_type: ProviderType::Docker,
            capabilities: crate::shared_kernel::ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["docker".to_string()],
                memory_limit_mb: Some(4096),
                cpu_limit: Some(2.0),
            },
            current_jobs: 0,
        };

        let result = use_case.execute(&job_id, vec![provider_info]).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, DomainError::NotFound(_)));
        }
    }

    #[tokio::test]
    async fn test_get_job_result_use_case_success() {
        let repo = MockJobRepository::new();
        let job_id = JobId::new("job-123".to_string());

        // Pre-populate repository with a job that has a result
        {
            let spec = JobSpec::new("test-job".to_string(), vec!["echo".to_string()], vec![]);
            let mut job = Job::new(job_id.clone(), spec).unwrap();

            let context = crate::job_execution::value_objects::ExecutionContext::new(
                job_id.clone(),
                ProviderId::new("provider-1".to_string()),
                "exec-123".to_string(),
            );

            job.submit_to_provider(ProviderId::new("provider-1".to_string()), context);
            job.complete();

            let mut jobs = repo.jobs.lock().unwrap();
            jobs.insert(job_id.to_string(), job);
        }

        let use_case = GetJobResultUseCase::new(Box::new(repo));

        let result = use_case.execute(&job_id).await.unwrap();

        assert!(result.is_none()); // Job was completed but no result was set
    }
}
