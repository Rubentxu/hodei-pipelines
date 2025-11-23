//! Redb (Embedded) Repository Implementations
//!
//! Embedded storage using Redb - perfect for edge devices, development, and testing.

use async_trait::async_trait;
use hodei_core::{Job, JobId, Pipeline, PipelineId, Worker, WorkerId};
use hodei_ports::{
    JobRepository, JobRepositoryError, PipelineRepository, PipelineRepositoryError,
    WorkerRepository, WorkerRepositoryError,
};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

// Table definitions
const JOBS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("jobs");
const WORKERS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("workers");
const PIPELINES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("pipelines");

/// Redb-backed job repository
pub struct RedbJobRepository {
    db: Arc<Database>,
}

impl RedbJobRepository {
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(db) }
    }

    pub fn new_with_path(path: PathBuf) -> Result<Self, JobRepositoryError> {
        let db = Database::create(&path).map_err(|e| {
            JobRepositoryError::Database(format!("Failed to create Redb database: {}", e))
        })?;
        Ok(Self::new(db))
    }

    /// Initialize database schema
    pub async fn init(&self) -> Result<(), JobRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let _jobs_table = tx.open_table(JOBS_TABLE).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to create jobs table: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to commit init transaction: {}", e))
        })?;

        info!("Redb job repository initialized");
        Ok(())
    }
}

#[async_trait]
impl JobRepository for RedbJobRepository {
    async fn save_job(&self, job: &Job) -> Result<(), JobRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let mut table = tx.open_table(JOBS_TABLE).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
            })?;

            let key = job.id.to_string().into_bytes();
            let value = serde_json::to_vec(job).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to serialize job: {}", e))
            })?;

            table
                .insert(key.as_slice(), value.as_slice())
                .map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to insert job: {}", e))
                })?;
        }

        tx.commit().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to commit job save: {}", e))
        })?;

        info!("Saved job to Redb: {}", job.id);
        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<Job>, JobRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(JOBS_TABLE).map_err(|e| {
            JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
        })?;

        let key = id.to_string().into_bytes();
        match table.get(key.as_slice()) {
            Ok(Some(value)) => {
                let job: Job = serde_json::from_slice(value.value()).map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to deserialize job: {}", e))
                })?;
                Ok(Some(job))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JobRepositoryError::Database(format!(
                "Failed to get job: {}",
                e
            ))),
        }
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(JOBS_TABLE).map_err(|e| {
            JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
        })?;

        let mut jobs = Vec::new();

        // Open iterator handle
        let iter = table
            .iter()
            .map_err(|e| JobRepositoryError::Database(format!("Failed to iterate jobs: {}", e)))?;

        for item in iter {
            let (_, value) = item
                .map_err(|e| JobRepositoryError::Database(format!("Failed to read item: {}", e)))?;
            if let Ok(job) = serde_json::from_slice::<Job>(value.value()) {
                if job.is_pending() {
                    jobs.push(job);
                }
            }
        }

        Ok(jobs)
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(JOBS_TABLE).map_err(|e| {
            JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
        })?;

        let mut jobs = Vec::new();

        // Open iterator handle
        let iter = table
            .iter()
            .map_err(|e| JobRepositoryError::Database(format!("Failed to iterate jobs: {}", e)))?;

        for item in iter {
            let (_, value) = item
                .map_err(|e| JobRepositoryError::Database(format!("Failed to read item: {}", e)))?;
            if let Ok(job) = serde_json::from_slice::<Job>(value.value()) {
                if job.is_running() {
                    jobs.push(job);
                }
            }
        }

        Ok(jobs)
    }

    async fn delete_job(&self, id: &JobId) -> Result<(), JobRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let mut table = tx.open_table(JOBS_TABLE).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
            })?;

            let key = id.to_string().into_bytes();
            table.remove(key.as_slice()).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to delete job: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to commit job deletion: {}", e))
        })?;

        Ok(())
    }

    async fn compare_and_swap_status(
        &self,
        id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool, JobRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        let mut swapped = false;

        {
            let table = tx.open_table(JOBS_TABLE).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
            })?;

            let key = id.to_string().into_bytes();

            // Get current value and extract data
            let (job_bytes, should_update) = if let Some(value) = table
                .get(key.as_slice())
                .map_err(|e| JobRepositoryError::Database(format!("Failed to get job: {}", e)))?
            {
                let job_bytes = value.value().to_vec();

                let job: Job = serde_json::from_slice(&job_bytes).map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to deserialize job: {}", e))
                })?;

                if job.state.as_str() == expected_state {
                    (job_bytes, true)
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            };

            // Drop immutable borrow
            drop(table);

            if should_update {
                let mut table = tx.open_table(JOBS_TABLE).map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
                })?;

                let mut job: Job = serde_json::from_slice(&job_bytes).map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to deserialize job: {}", e))
                })?;

                job.state = hodei_core::JobState::new(new_state.to_string())
                    .map_err(|e| JobRepositoryError::Validation(e.to_string()))?;
                job.updated_at = chrono::Utc::now();

                let new_value = serde_json::to_vec(&job).map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to serialize job: {}", e))
                })?;

                table
                    .insert(key.as_slice(), new_value.as_slice())
                    .map_err(|e| {
                        JobRepositoryError::Database(format!("Failed to insert job: {}", e))
                    })?;

                swapped = true;
            }
        }

        tx.commit().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to commit status swap: {}", e))
        })?;

        Ok(swapped)
    }
}

/// Redb-backed worker repository
pub struct RedbWorkerRepository {
    db: Arc<Database>,
}

impl RedbWorkerRepository {
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(db) }
    }

    pub fn new_with_path(path: PathBuf) -> Result<Self, WorkerRepositoryError> {
        let db = Database::create(&path).map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to create Redb database: {}", e))
        })?;
        Ok(Self::new(db))
    }

    /// Initialize database schema
    pub async fn init(&self) -> Result<(), WorkerRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let _workers_table = tx.open_table(WORKERS_TABLE).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to create workers table: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to commit init transaction: {}", e))
        })?;

        info!("Redb worker repository initialized");
        Ok(())
    }
}

#[async_trait]
impl WorkerRepository for RedbWorkerRepository {
    async fn save_worker(&self, worker: &Worker) -> Result<(), WorkerRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let mut table = tx.open_table(WORKERS_TABLE).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to open workers table: {}", e))
            })?;

            let key = worker.id.to_string().into_bytes();
            let value = serde_json::to_vec(worker).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to serialize worker: {}", e))
            })?;

            table
                .insert(key.as_slice(), value.as_slice())
                .map_err(|e| {
                    WorkerRepositoryError::Database(format!("Failed to insert worker: {}", e))
                })?;
        }

        tx.commit().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to commit worker save: {}", e))
        })?;

        info!("Saved worker to Redb: {}", worker.id);
        Ok(())
    }

    async fn get_worker(&self, id: &WorkerId) -> Result<Option<Worker>, WorkerRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(WORKERS_TABLE).map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to open workers table: {}", e))
        })?;

        let key = id.to_string().into_bytes();
        match table.get(key.as_slice()) {
            Ok(Some(value)) => {
                let worker: Worker = serde_json::from_slice(value.value()).map_err(|e| {
                    WorkerRepositoryError::Database(format!("Failed to deserialize worker: {}", e))
                })?;
                Ok(Some(worker))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(WorkerRepositoryError::Database(format!(
                "Failed to get worker: {}",
                e
            ))),
        }
    }

    async fn get_all_workers(&self) -> Result<Vec<Worker>, WorkerRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(WORKERS_TABLE).map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to open workers table: {}", e))
        })?;

        let mut workers = Vec::new();

        // Open iterator handle
        let iter = table.iter().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to iterate workers: {}", e))
        })?;

        for item in iter {
            let (_, value) = item.map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to read item: {}", e))
            })?;
            if let Ok(worker) = serde_json::from_slice::<Worker>(value.value()) {
                workers.push(worker);
            }
        }

        Ok(workers)
    }

    async fn delete_worker(&self, id: &WorkerId) -> Result<(), WorkerRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let mut table = tx.open_table(WORKERS_TABLE).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to open workers table: {}", e))
            })?;

            let key = id.to_string().into_bytes();
            table.remove(key.as_slice()).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to delete worker: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to commit worker deletion: {}", e))
        })?;

        Ok(())
    }
}

/// Redb-backed pipeline repository
pub struct RedbPipelineRepository {
    db: Arc<Database>,
}

impl RedbPipelineRepository {
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(db) }
    }

    pub fn new_with_path(path: PathBuf) -> Result<Self, PipelineRepositoryError> {
        let db = Database::create(&path).map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to create Redb database: {}", e))
        })?;
        Ok(Self::new(db))
    }

    /// Initialize database schema
    pub async fn init(&self) -> Result<(), PipelineRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let _pipelines_table = tx.open_table(PIPELINES_TABLE).map_err(|e| {
                PipelineRepositoryError::Database(format!(
                    "Failed to create pipelines table: {}",
                    e
                ))
            })?;
        }

        tx.commit().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to commit init transaction: {}", e))
        })?;

        info!("Redb pipeline repository initialized");
        Ok(())
    }
}

#[async_trait]
impl PipelineRepository for RedbPipelineRepository {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<(), PipelineRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let mut table = tx.open_table(PIPELINES_TABLE).map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to open pipelines table: {}", e))
            })?;

            let key = pipeline.id.to_string().into_bytes();
            let value = serde_json::to_vec(pipeline).map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to serialize pipeline: {}", e))
            })?;

            table
                .insert(key.as_slice(), value.as_slice())
                .map_err(|e| {
                    PipelineRepositoryError::Database(format!("Failed to insert pipeline: {}", e))
                })?;
        }

        tx.commit().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to commit pipeline save: {}", e))
        })?;

        info!("Saved pipeline to Redb: {}", pipeline.id);
        Ok(())
    }

    async fn get_pipeline(
        &self,
        id: &PipelineId,
    ) -> Result<Option<Pipeline>, PipelineRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(PIPELINES_TABLE).map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to open pipelines table: {}", e))
        })?;

        let key = id.to_string().into_bytes();
        match table.get(key.as_slice()) {
            Ok(Some(value)) => {
                let pipeline: Pipeline = serde_json::from_slice(value.value()).map_err(|e| {
                    PipelineRepositoryError::Database(format!(
                        "Failed to deserialize pipeline: {}",
                        e
                    ))
                })?;
                Ok(Some(pipeline))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(PipelineRepositoryError::Database(format!(
                "Failed to get pipeline: {}",
                e
            ))),
        }
    }

    async fn delete_pipeline(&self, id: &PipelineId) -> Result<(), PipelineRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            let mut table = tx.open_table(PIPELINES_TABLE).map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to open pipelines table: {}", e))
            })?;

            let key = id.to_string().into_bytes();
            table.remove(key.as_slice()).map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to delete pipeline: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to commit pipeline deletion: {}", e))
        })?;

        Ok(())
    }
}
