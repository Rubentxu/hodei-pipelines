//! Orchestrator Module

use hodei_core::{Job, JobId, JobSpec, Pipeline, PipelineId, Result};
use hodei_ports::{EventPublisher, JobRepository, PipelineRepository};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_concurrent_jobs: usize,
    pub default_timeout_ms: u64,
}

pub struct OrchestratorModule<R, E, P>
where
    R: JobRepository,
    E: EventPublisher,
    P: PipelineRepository,
{
    job_repo: Arc<R>,
    event_bus: Arc<E>,
    pipeline_repo: Arc<P>,
    config: OrchestratorConfig,
}

impl<R, E, P> OrchestratorModule<R, E, P>
where
    R: JobRepository,
    E: EventPublisher,
    P: PipelineRepository,
{
    pub fn new(
        job_repo: Arc<R>,
        event_bus: Arc<E>,
        pipeline_repo: Arc<P>,
        config: OrchestratorConfig,
    ) -> Self {
        Self {
            job_repo,
            event_bus,
            pipeline_repo,
            config,
        }
    }

    pub async fn create_job(&self, spec: JobSpec) -> Result<Job> {
        info!("Creating job: {}", spec.name);

        spec.validate()
            .map_err(|e| OrchestratorError::Validation(e.to_string()))?;

        let job = Job::new(JobId::new(), spec)
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        self.job_repo
            .save_job(&job)
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        self.event_bus
            .publish(hodei_ports::SystemEvent::JobCreated(job.spec.clone()))
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        Ok(job)
    }

    pub async fn get_job(&self, id: &JobId) -> Result<Option<Job>> {
        self.job_repo
            .get_job(id)
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()).into())
    }

    pub async fn cancel_job(&self, id: &JobId) -> Result<()> {
        info!("Canceling job: {}", id);

        let job = self
            .job_repo
            .get_job(id)
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?
            .ok_or(OrchestratorError::JobNotFound(*id))?;

        let mut job = job;
        job.cancel()
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        self.job_repo
            .save_job(&job)
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        Ok(())
    }

    pub async fn create_pipeline(
        &self,
        name: String,
        steps: Vec<hodei_core::pipeline::PipelineStep>,
    ) -> Result<Pipeline> {
        info!("Creating pipeline: {}", name);

        let pipeline = Pipeline::new(PipelineId::new(), name, steps)
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineCreated(pipeline.clone()))
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        Ok(pipeline)
    }

    pub async fn start_pipeline(&self, id: &PipelineId) -> Result<()> {
        info!("Starting pipeline: {}", id);

        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?
            .ok_or(OrchestratorError::PipelineNotFound(id.clone()))?;

        pipeline
            .start()
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineStarted {
                pipeline_id: id.clone(),
            })
            .await
            .map_err(|e| OrchestratorError::DomainError(e.to_string()))?;

        Ok(())
    }
}

impl<R, E, P> Clone for OrchestratorModule<R, E, P>
where
    R: JobRepository,
    E: EventPublisher,
    P: PipelineRepository,
{
    fn clone(&self) -> Self {
        Self {
            job_repo: self.job_repo.clone(),
            event_bus: self.event_bus.clone(),
            pipeline_repo: self.pipeline_repo.clone(),
            config: self.config.clone(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum OrchestratorError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Domain error: {0}")]
    DomainError(String),

    #[error("Job not found: {0}")]
    JobNotFound(JobId),

    #[error("Pipeline not found: {0}")]
    PipelineNotFound(PipelineId),

    #[error("Job repository error: {0}")]
    JobRepository(hodei_ports::JobRepositoryError),

    #[error("Pipeline repository error: {0}")]
    PipelineRepository(hodei_ports::PipelineRepositoryError),

    #[error("Event bus error: {0}")]
    EventBus(hodei_ports::EventBusError),
}

// Error conversion to DomainError
impl From<OrchestratorError> for hodei_core::DomainError {
    fn from(err: OrchestratorError) -> Self {
        hodei_core::DomainError::Infrastructure(err.to_string())
    }
}
