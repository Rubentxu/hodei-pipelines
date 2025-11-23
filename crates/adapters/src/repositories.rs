//! In-Memory Repository Implementations

use async_trait::async_trait;
use hodei_core::{Job, JobId, Pipeline, PipelineId, Worker, WorkerId};
use hodei_ports::{JobRepository, PipelineRepository, WorkerRepository};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// In-memory job repository
pub struct InMemoryJobRepository {
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
}

impl InMemoryJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn save_job(&self, job: &Job) -> Result<(), hodei_ports::JobRepositoryError> {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job.clone());
        info!("Saved job: {}", job.id);
        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<Job>, hodei_ports::JobRepositoryError> {
        let jobs = self.jobs.read().await;
        Ok(jobs.get(id).cloned())
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>, hodei_ports::JobRepositoryError> {
        let jobs = self.jobs.read().await;
        let pending: Vec<Job> = jobs
            .values()
            .filter(|job| job.is_pending())
            .cloned()
            .collect();
        Ok(pending)
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>, hodei_ports::JobRepositoryError> {
        let jobs = self.jobs.read().await;
        let running: Vec<Job> = jobs
            .values()
            .filter(|job| job.is_running())
            .cloned()
            .collect();
        Ok(running)
    }

    async fn delete_job(&self, id: &JobId) -> Result<(), hodei_ports::JobRepositoryError> {
        let mut jobs = self.jobs.write().await;
        jobs.remove(id);
        Ok(())
    }

    async fn compare_and_swap_status(
        &self,
        id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool, hodei_ports::JobRepositoryError> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(id) {
            if job.state.as_str() == expected_state {
                job.state = hodei_core::JobState::new(new_state.to_string())
                    .map_err(|e| hodei_ports::JobRepositoryError::Validation(e.to_string()))?;
                job.updated_at = chrono::Utc::now();
                return Ok(true);
            }
            return Ok(false);
        }

        Err(hodei_ports::JobRepositoryError::NotFound(id.clone()))
    }
}

/// In-memory worker repository
pub struct InMemoryWorkerRepository {
    workers: Arc<RwLock<HashMap<WorkerId, Worker>>>,
}

impl InMemoryWorkerRepository {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryWorkerRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkerRepository for InMemoryWorkerRepository {
    async fn save_worker(&self, worker: &Worker) -> Result<(), hodei_ports::WorkerRepositoryError> {
        let mut workers = self.workers.write().await;
        workers.insert(worker.id.clone(), worker.clone());
        info!("Saved worker: {}", worker.id);
        Ok(())
    }

    async fn get_worker(
        &self,
        id: &WorkerId,
    ) -> Result<Option<Worker>, hodei_ports::WorkerRepositoryError> {
        let workers = self.workers.read().await;
        Ok(workers.get(id).cloned())
    }

    async fn get_all_workers(&self) -> Result<Vec<Worker>, hodei_ports::WorkerRepositoryError> {
        let workers = self.workers.read().await;
        Ok(workers.values().cloned().collect())
    }

    async fn delete_worker(&self, id: &WorkerId) -> Result<(), hodei_ports::WorkerRepositoryError> {
        let mut workers = self.workers.write().await;
        workers.remove(id);
        Ok(())
    }
}

/// In-memory pipeline repository
pub struct InMemoryPipelineRepository {
    pipelines: Arc<RwLock<HashMap<PipelineId, Pipeline>>>,
}

impl InMemoryPipelineRepository {
    pub fn new() -> Self {
        Self {
            pipelines: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryPipelineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineRepository for InMemoryPipelineRepository {
    async fn save_pipeline(
        &self,
        pipeline: &Pipeline,
    ) -> Result<(), hodei_ports::PipelineRepositoryError> {
        let mut pipelines = self.pipelines.write().await;
        pipelines.insert(pipeline.id.clone(), pipeline.clone());
        info!("Saved pipeline: {}", pipeline.id);
        Ok(())
    }

    async fn get_pipeline(
        &self,
        id: &PipelineId,
    ) -> Result<Option<Pipeline>, hodei_ports::PipelineRepositoryError> {
        let pipelines = self.pipelines.read().await;
        Ok(pipelines.get(id).cloned())
    }

    async fn delete_pipeline(
        &self,
        id: &PipelineId,
    ) -> Result<(), hodei_ports::PipelineRepositoryError> {
        let mut pipelines = self.pipelines.write().await;
        pipelines.remove(id);
        Ok(())
    }
}
