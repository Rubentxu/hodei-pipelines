use async_trait::async_trait;
use hodei_core::{JobId, WorkerId, JobSpec};
use hodei_ports::{WorkerClient, WorkerStatus, WorkerClientError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct MockWorkerClient {
    workers: Arc<RwLock<HashMap<WorkerId, WorkerStatus>>>,
}

impl MockWorkerClient {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MockWorkerClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkerClient for MockWorkerClient {
    async fn assign_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
        _job_spec: &JobSpec,
    ) -> Result<(), WorkerClientError> {
        let mut workers = self.workers.write().await;
        
        if let Some(status) = workers.get_mut(worker_id) {
            status.current_jobs.push(job_id.clone());
            info!("Assigned job {} to worker {}", job_id, worker_id);
            Ok(())
        } else {
            Err(WorkerClientError::NotFound(worker_id.clone()))
        }
    }

    async fn cancel_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<(), WorkerClientError> {
        let mut workers = self.workers.write().await;
        
        if let Some(status) = workers.get_mut(worker_id) {
            status.current_jobs.retain(|id| id != job_id);
            Ok(())
        } else {
            Err(WorkerClientError::NotFound(worker_id.clone()))
        }
    }

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<WorkerStatus, WorkerClientError> {
        let workers = self.workers.read().await;
        
        if let Some(status) = workers.get(worker_id) {
            Ok(status.clone())
        } else {
            Err(WorkerClientError::NotFound(worker_id.clone()))
        }
    }

    async fn send_heartbeat(
        &self,
        worker_id: &WorkerId,
    ) -> Result<(), WorkerClientError> {
        let mut workers = self.workers.write().await;
        
        if let Some(status) = workers.get_mut(worker_id) {
            status.last_heartbeat = std::time::SystemTime::now();
            Ok(())
        } else {
            Err(WorkerClientError::NotFound(worker_id.clone()))
        }
    }
}
