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
            // Delegate state transition validation to domain layer
            match job.compare_and_swap_status(expected_state, new_state) {
                Ok(updated) => {
                    if updated {
                        // Job state was successfully updated
                        Ok(true)
                    } else {
                        // State didn't match, nothing to update
                        Ok(false)
                    }
                }
                Err(e) => {
                    // Invalid state transition - domain rule violation
                    Err(hodei_ports::JobRepositoryError::Validation(e.to_string()))
                }
            }
        } else {
            Err(hodei_ports::JobRepositoryError::NotFound(id.clone()))
        }
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

    async fn update_last_seen(
        &self,
        id: &WorkerId,
    ) -> Result<(), hodei_ports::WorkerRepositoryError> {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.get_mut(id) {
            worker.last_heartbeat = chrono::Utc::now();
            info!("Updated last_seen for worker: {}", id);
            Ok(())
        } else {
            Err(hodei_ports::WorkerRepositoryError::NotFound(id.clone()))
        }
    }

    async fn find_stale_workers(
        &self,
        threshold_duration: std::time::Duration,
    ) -> Result<Vec<Worker>, hodei_ports::WorkerRepositoryError> {
        let workers = self.workers.read().await;
        let threshold_time =
            chrono::Utc::now() - chrono::Duration::from_std(threshold_duration).unwrap_or_default();

        let stale_workers: Vec<Worker> = workers
            .values()
            .filter(|worker| worker.last_heartbeat < threshold_time)
            .cloned()
            .collect();

        info!(
            "Found {} stale workers (threshold: {} seconds)",
            stale_workers.len(),
            threshold_duration.as_secs()
        );

        Ok(stale_workers)
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

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::ResourceQuota;
    use hodei_core::pipeline::{PipelineStep, PipelineStepId};
    use hodei_core::{Job, JobId, JobSpec, JobState, Pipeline, PipelineId, Worker, WorkerId};
    use std::collections::HashMap;
    use tokio::test;

    // ===== InMemoryJobRepository Tests =====

    #[test]
    async fn test_in_memory_job_save_and_retrieve() {
        let repo = InMemoryJobRepository::new();
        let job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("Test job".to_string()),
            Some("test-tenant".to_string()),
        )
        .unwrap();

        repo.save_job(&job).await.unwrap();

        let retrieved = repo.get_job(&job.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test-job");
    }

    #[test]
    async fn test_in_memory_job_get_nonexistent() {
        let repo = InMemoryJobRepository::new();
        let non_existent_id = JobId::new();

        let retrieved = repo.get_job(&non_existent_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    async fn test_in_memory_job_get_pending_jobs() {
        let repo = InMemoryJobRepository::new();

        let pending_job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("pending-job".to_string()),
            None::<String>,
        )
        .unwrap();

        let mut running_job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("running-job".to_string()),
            None::<String>,
        )
        .unwrap();
        running_job.schedule().unwrap();
        running_job.start().unwrap();

        repo.save_job(&pending_job).await.unwrap();
        repo.save_job(&running_job).await.unwrap();

        let pending = repo.get_pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].name(), "test-job");
    }

    #[test]
    async fn test_in_memory_job_get_running_jobs() {
        let repo = InMemoryJobRepository::new();

        let pending_job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("pending-job".to_string()),
            None::<String>,
        )
        .unwrap();

        let mut running_job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("running-job".to_string()),
            None::<String>,
        )
        .unwrap();
        running_job.schedule().unwrap();
        running_job.start().unwrap();

        repo.save_job(&pending_job).await.unwrap();
        repo.save_job(&running_job).await.unwrap();

        let running = repo.get_running_jobs().await.unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].name(), "test-job");
    }

    #[test]
    async fn test_in_memory_job_delete() {
        let repo = InMemoryJobRepository::new();
        let job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("test-job".to_string()),
            None::<String>,
        )
        .unwrap();

        repo.save_job(&job).await.unwrap();
        assert!(repo.get_job(&job.id).await.unwrap().is_some());

        repo.delete_job(&job.id).await.unwrap();
        assert!(repo.get_job(&job.id).await.unwrap().is_none());
    }

    #[test]
    async fn test_in_memory_job_compare_and_swap_success() {
        let repo = InMemoryJobRepository::new();
        let job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("test-job".to_string()),
            None::<String>,
        )
        .unwrap();

        repo.save_job(&job).await.unwrap();

        // First transition: PENDING -> SCHEDULED
        let swapped = repo
            .compare_and_swap_status(&job.id, "PENDING", "SCHEDULED")
            .await
            .unwrap();
        assert!(swapped);

        // Second transition: SCHEDULED -> RUNNING
        let swapped = repo
            .compare_and_swap_status(&job.id, "SCHEDULED", "RUNNING")
            .await
            .unwrap();
        assert!(swapped);

        let retrieved = repo.get_job(&job.id).await.unwrap().unwrap();
        assert_eq!(retrieved.state.as_str(), "RUNNING");
    }

    #[test]
    async fn test_in_memory_job_compare_and_swap_failed() {
        let repo = InMemoryJobRepository::new();
        let job = Job::create(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            Some("test-job".to_string()),
            None::<String>,
        )
        .unwrap();

        repo.save_job(&job).await.unwrap();

        // First set job to RUNNING state
        repo.compare_and_swap_status(&job.id, "PENDING", "SCHEDULED")
            .await
            .unwrap();
        repo.compare_and_swap_status(&job.id, "SCHEDULED", "RUNNING")
            .await
            .unwrap();

        // Now try to swap expecting PENDING (but actual state is RUNNING)
        let swapped = repo
            .compare_and_swap_status(&job.id, "PENDING", "FAILED")
            .await
            .unwrap();
        assert!(!swapped);

        let retrieved = repo.get_job(&job.id).await.unwrap().unwrap();
        assert_eq!(retrieved.state.as_str(), "RUNNING");
    }

    #[test]
    async fn test_in_memory_job_compare_and_swap_not_found() {
        let repo = InMemoryJobRepository::new();
        let non_existent_id = JobId::new();

        let result = repo
            .compare_and_swap_status(&non_existent_id, "PENDING", "RUNNING")
            .await;
        assert!(result.is_err());
    }

    #[test]
    async fn test_in_memory_job_empty_pending_and_running() {
        let repo = InMemoryJobRepository::new();

        let pending = repo.get_pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 0);

        let running = repo.get_running_jobs().await.unwrap();
        assert_eq!(running.len(), 0);
    }

    // ===== InMemoryWorkerRepository Tests =====

    #[test]
    async fn test_in_memory_worker_save_and_retrieve() {
        let repo = InMemoryWorkerRepository::new();
        let worker = Worker {
            id: WorkerId::new(),
            name: "test-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: Some("test-tenant".to_string()),
            capabilities: hodei_core::WorkerCapabilities::new(4, 8192),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: chrono::Utc::now(),
        };

        repo.save_worker(&worker).await.unwrap();

        let retrieved = repo.get_worker(&worker.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-worker");
    }

    #[test]
    async fn test_in_memory_worker_get_nonexistent() {
        let repo = InMemoryWorkerRepository::new();
        let non_existent_id = WorkerId::new();

        let retrieved = repo.get_worker(&non_existent_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    async fn test_in_memory_worker_get_all() {
        let repo = InMemoryWorkerRepository::new();

        let worker1 = Worker {
            id: WorkerId::new(),
            name: "worker-1".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(2, 4096),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: chrono::Utc::now(),
        };

        let worker2 = Worker {
            id: WorkerId::new(),
            name: "worker-2".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(4, 8192),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: chrono::Utc::now(),
        };

        repo.save_worker(&worker1).await.unwrap();
        repo.save_worker(&worker2).await.unwrap();

        let all = repo.get_all_workers().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    async fn test_in_memory_worker_delete() {
        let repo = InMemoryWorkerRepository::new();
        let worker = Worker {
            id: WorkerId::new(),
            name: "test-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(2, 4096),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: chrono::Utc::now(),
        };

        repo.save_worker(&worker).await.unwrap();
        assert!(repo.get_worker(&worker.id).await.unwrap().is_some());

        repo.delete_worker(&worker.id).await.unwrap();
        assert!(repo.get_worker(&worker.id).await.unwrap().is_none());
    }

    #[test]
    async fn test_in_memory_worker_get_all_empty() {
        let repo = InMemoryWorkerRepository::new();

        let all = repo.get_all_workers().await.unwrap();
        assert_eq!(all.len(), 0);
    }

    #[test]
    async fn test_in_memory_worker_update() {
        let repo = InMemoryWorkerRepository::new();
        let worker = Worker {
            id: WorkerId::new(),
            name: "test-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(2, 4096),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: chrono::Utc::now(),
        };

        repo.save_worker(&worker).await.unwrap();

        let mut updated_worker = worker.clone();
        updated_worker.name = "updated-worker".to_string();

        repo.save_worker(&updated_worker).await.unwrap();

        let retrieved = repo.get_worker(&worker.id).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "updated-worker");
    }

    // ===== InMemoryPipelineRepository Tests =====

    #[test]
    async fn test_in_memory_pipeline_save_and_retrieve() {
        let repo = InMemoryPipelineRepository::new();
        let pipeline = Pipeline {
            id: PipelineId::new(),
            name: "test-pipeline".to_string(),
            description: Some("Test pipeline".to_string()),
            steps: vec![PipelineStep {
                id: PipelineStepId::new(),
                name: "step1".to_string(),
                job_spec: JobSpec {
                    name: "test-job".to_string(),
                    image: "test:latest".to_string(),
                    command: vec!["echo".to_string()],
                    resources: ResourceQuota::default(),
                    timeout_ms: 30000,
                    retries: 3,
                    env: HashMap::new(),
                    secret_refs: vec![],
                },
                depends_on: vec![],
                timeout_ms: 60000,
            }],
            status: hodei_core::PipelineStatus::PENDING,
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: Some("test-tenant".to_string()),
            workflow_definition: serde_json::Value::Null,
        };

        repo.save_pipeline(&pipeline).await.unwrap();

        let retrieved = repo.get_pipeline(&pipeline.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-pipeline");
    }

    #[test]
    async fn test_in_memory_pipeline_get_nonexistent() {
        let repo = InMemoryPipelineRepository::new();
        let non_existent_id = PipelineId::new();

        let retrieved = repo.get_pipeline(&non_existent_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    async fn test_in_memory_pipeline_delete() {
        let repo = InMemoryPipelineRepository::new();
        let pipeline = Pipeline {
            id: PipelineId::new(),
            name: "test-pipeline".to_string(),
            description: None,
            steps: vec![],
            status: hodei_core::PipelineStatus::PENDING,
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            workflow_definition: serde_json::Value::Null,
        };

        repo.save_pipeline(&pipeline).await.unwrap();
        assert!(repo.get_pipeline(&pipeline.id).await.unwrap().is_some());

        repo.delete_pipeline(&pipeline.id).await.unwrap();
        assert!(repo.get_pipeline(&pipeline.id).await.unwrap().is_none());
    }

    #[test]
    async fn test_in_memory_pipeline_update() {
        let repo = InMemoryPipelineRepository::new();
        let pipeline = Pipeline {
            id: PipelineId::new(),
            name: "test-pipeline".to_string(),
            description: None,
            steps: vec![],
            status: hodei_core::PipelineStatus::PENDING,
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            workflow_definition: serde_json::Value::Null,
        };

        repo.save_pipeline(&pipeline).await.unwrap();

        let mut updated_pipeline = pipeline.clone();
        updated_pipeline.name = "updated-pipeline".to_string();

        repo.save_pipeline(&updated_pipeline).await.unwrap();

        let retrieved = repo.get_pipeline(&pipeline.id).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "updated-pipeline");
    }
}
