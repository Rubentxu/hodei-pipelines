//! Redb (Embedded) Repository Implementations
//!
//! Embedded storage using Redb - perfect for edge devices, development, and testing.

use dashmap::DashMap;
use hodei_core::{
    DomainError, Job, JobId, Result, WorkerId,
};
use hodei_ports::JobRepository;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

// Table definitions
const JOBS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("jobs");
const WORKERS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("workers");
const PIPELINES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("pipelines");

/// Redb-backed job repository with performance optimizations
pub struct RedbJobRepository {
    db: Arc<Database>,
    /// In-memory cache for hot data (lock-free)
    cache: Arc<DashMap<String, Job>>,
}

impl RedbJobRepository {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(db),
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn new_with_path(path: PathBuf) -> Result<Self> {
        let db = Database::create(&path).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create Redb database: {}", e))
        })?;
        Ok(Self::new(db))
    }

    /// Initialize database schema
    pub async fn init(&self) -> Result<()> {
        let tx = self.db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin write transaction: {}", e))
        })?;

        {
            // Main jobs table
            let _jobs_table = tx.open_table(JOBS_TABLE).map_err(|e| {
                DomainError::Infrastructure(format!("Failed to create jobs table: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit init transaction: {}", e))
        })?;

        info!("Redb job repository initialized with cache");
        Ok(())
    }

    /// Helper function to serialize job to bytes
    fn job_to_bytes(job: &Job) -> Vec<u8> {
        serde_json::to_vec(job).unwrap_or_default()
    }

    /// Helper function to deserialize bytes to job
    fn bytes_to_job(data: &[u8]) -> Option<Job> {
        serde_json::from_slice(data).ok()
    }
}

#[async_trait::async_trait]
impl JobRepository for RedbJobRepository {
    async fn save_job(&self, job: &Job) -> Result<()> {
        unimplemented!()
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<Job>> {
        unimplemented!()
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>> {
        unimplemented!()
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>> {
        unimplemented!()
    }

    async fn delete_job(&self, id: &JobId) -> Result<()> {
        unimplemented!()
    }

    async fn compare_and_swap_status(
        &self,
        _id: &JobId,
        _expected_state: &str,
        _new_state: &str,
    ) -> Result<bool> {
        unimplemented!()
    }

    async fn assign_worker(&self, _job_id: &JobId, _worker_id: &WorkerId) -> Result<()> {
        unimplemented!()
    }

    async fn set_job_start_time(
        &self,
        _job_id: &JobId,
        _start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn set_job_finish_time(
        &self,
        _job_id: &JobId,
        _finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn set_job_duration(&self, _job_id: &JobId, _duration_ms: i64) -> Result<()> {
        unimplemented!()
    }

    async fn create_job(&self, _job_spec: hodei_core::job::JobSpec) -> Result<JobId> {
        unimplemented!()
    }

    async fn update_job_state(&self, _job_id: &JobId, _state: hodei_core::job::JobState) -> Result<()> {
        unimplemented!()
    }

    async fn list_jobs(&self) -> Result<Vec<Job>> {
        unimplemented!()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::ResourceQuota;
    
    use hodei_core::{Job, JobId, JobSpec};
    use std::collections::HashMap;
    
    use tempfile::tempdir;
    use tokio::test;

    // ===== RedbJobRepository Tests =====

    #[test]
    async fn test_redb_job_repository_init() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path).unwrap();
        let init_result = repo.init().await;

        assert!(init_result.is_ok());
    }

    #[test]
    async fn test_redb_job_save_and_retrieve() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_job_get_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let non_existent_id = JobId::new();

        let retrieved = repo.get_job(&non_existent_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    async fn test_redb_job_get_pending_jobs() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_job_get_running_jobs() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_job_delete() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_job_compare_and_swap_success() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_job_compare_and_swap_failed() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_job_compare_and_swap_not_found() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let non_existent_id = JobId::new();

        let result = repo
            .compare_and_swap_status(&non_existent_id, "PENDING", "RUNNING")
            .await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    async fn test_redb_job_empty_pending_and_running() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_jobs.redb");

        let repo = RedbJobRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let pending = repo.get_pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 0);

        let running = repo.get_running_jobs().await.unwrap();
        assert_eq!(running.len(), 0);
    }
}
