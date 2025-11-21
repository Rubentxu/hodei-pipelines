//! Application layer - Use cases and Application Services
//!
//! This layer orchestrates domain entities and provides application-specific logic.

use crate::domain::{Job, JobEvent};
use async_trait::async_trait;
use hodei_shared_types::{CorrelationId, DomainError, JobId, JobSpec, TenantId};

/// Data Transfer Objects
pub mod dtos {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct CreateJobRequest {
        pub spec: hodei_shared_types::JobSpec,
        pub correlation_id: Option<hodei_shared_types::CorrelationId>,
        pub tenant_id: hodei_shared_types::TenantId,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct CreateJobResponse {
        pub job_id: hodei_shared_types::JobId,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct JobStatusResponse {
        pub job_id: hodei_shared_types::JobId,
        pub state: String,
        pub attempts: u8,
        pub created_at: Option<String>,
        pub started_at: Option<String>,
        pub completed_at: Option<String>,
    }
}

/// Job Use Cases
#[async_trait]
pub trait JobUseCases {
    async fn create_job(
        &self,
        request: dtos::CreateJobRequest,
    ) -> Result<dtos::CreateJobResponse, DomainError>;

    async fn get_job_status(&self, job_id: JobId) -> Result<dtos::JobStatusResponse, DomainError>;

    async fn cancel_job(&self, job_id: JobId) -> Result<(), DomainError>;
}

/// In-memory Job Repository (for testing)
pub struct InMemoryJobRepository {
    jobs: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<JobId, Job>>>,
}

impl InMemoryJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn save(&self, job: Job) -> Result<(), DomainError> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(job.id.clone(), job);
        Ok(())
    }

    pub async fn find(&self, job_id: &JobId) -> Result<Option<Job>, DomainError> {
        let jobs = self.jobs.lock().unwrap();
        Ok(jobs.get(job_id).cloned())
    }
}

/// Job Application Service
pub struct JobApplicationService {
    repository: InMemoryJobRepository,
}

impl JobApplicationService {
    pub fn new(repository: InMemoryJobRepository) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl JobUseCases for JobApplicationService {
    async fn create_job(
        &self,
        request: dtos::CreateJobRequest,
    ) -> Result<dtos::CreateJobResponse, DomainError> {
        let correlation_id = request.correlation_id.unwrap_or_default();

        // Use domain service to create job
        let job = Job::create(request.spec, correlation_id, request.tenant_id)?;

        // Save to repository
        self.repository.save(job.clone()).await?;

        Ok(dtos::CreateJobResponse { job_id: job.id })
    }

    async fn get_job_status(&self, job_id: JobId) -> Result<dtos::JobStatusResponse, DomainError> {
        let job =
            self.repository.find(&job_id).await?.ok_or_else(|| {
                DomainError::NotFound(format!("job {} not found", job_id.as_uuid()))
            })?;

        Ok(dtos::JobStatusResponse {
            job_id: job.id,
            state: job.state.as_str().to_string(),
            attempts: job.attempts,
            created_at: job.created_at.map(|t| t.to_rfc3339()),
            started_at: job.started_at.map(|t| t.to_rfc3339()),
            completed_at: job.completed_at.map(|t| t.to_rfc3339()),
        })
    }

    async fn cancel_job(&self, job_id: JobId) -> Result<(), DomainError> {
        let mut job =
            self.repository.find(&job_id).await?.ok_or_else(|| {
                DomainError::NotFound(format!("job {} not found", job_id.as_uuid()))
            })?;

        job.cancel()?;

        self.repository.save(job).await?;
        Ok(())
    }
}
