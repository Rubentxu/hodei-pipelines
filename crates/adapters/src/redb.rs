//! Redb (Embedded) Repository Implementations
//!
//! Embedded storage using Redb - perfect for edge devices, development, and testing.

use async_trait::async_trait;
use dashmap::DashMap;
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
            // Main jobs table
            let _jobs_table = tx.open_table(JOBS_TABLE).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to create jobs table: {}", e))
            })?;
        }

        tx.commit().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to commit init transaction: {}", e))
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

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::ResourceQuota;
    use hodei_core::pipeline::{PipelineStep, PipelineStepId};
    use hodei_core::{Job, JobId, JobSpec, JobState, Pipeline, PipelineId, Worker, WorkerId};
    use std::collections::HashMap;
    use std::path::PathBuf;
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

    // ===== RedbWorkerRepository Tests =====

    #[test]
    async fn test_redb_worker_repository_init() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path).unwrap();
        let init_result = repo.init().await;

        assert!(init_result.is_ok());
    }

    #[test]
    async fn test_redb_worker_save_and_retrieve() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_worker_get_all() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_worker_delete() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_worker_update_last_seen_success() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let original_time = chrono::Utc::now() - chrono::Duration::minutes(5);

        let worker = Worker {
            id: WorkerId::new(),
            name: "test-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: original_time.into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(2, 4096),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: original_time,
        };

        repo.save_worker(&worker).await.unwrap();

        // Update the last_seen timestamp
        repo.update_last_seen(&worker.id).await.unwrap();

        // Verify the timestamp was updated
        let retrieved = repo.get_worker(&worker.id).await.unwrap().unwrap();
        let new_timestamp = retrieved.last_heartbeat;

        assert!(new_timestamp > original_time);
        assert!(new_timestamp > original_time + chrono::Duration::seconds(4));
    }

    #[test]
    async fn test_redb_worker_update_last_seen_not_found() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let non_existent_id = WorkerId::new();

        let result = repo.update_last_seen(&non_existent_id).await;

        assert!(result.is_err());
        if let Err(e) = result {
            match e {
                WorkerRepositoryError::NotFound(_) => {}
                _ => panic!("Expected NotFound error, got {:?}", e),
            }
        }
    }

    #[test]
    async fn test_redb_worker_find_stale_workers() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let recent_time = chrono::Utc::now() - chrono::Duration::seconds(30);
        let stale_time = chrono::Utc::now() - chrono::Duration::minutes(10);

        let fresh_worker = Worker {
            id: WorkerId::new(),
            name: "fresh-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: recent_time.into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(2, 4096),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: recent_time,
        };

        let stale_worker = Worker {
            id: WorkerId::new(),
            name: "stale-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: stale_time.into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(4, 8192),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: stale_time,
        };

        repo.save_worker(&fresh_worker).await.unwrap();
        repo.save_worker(&stale_worker).await.unwrap();

        // Find workers stale for more than 1 minute
        let threshold = std::time::Duration::from_secs(60);
        let stale_workers = repo.find_stale_workers(threshold).await.unwrap();

        assert_eq!(stale_workers.len(), 1);
        assert_eq!(stale_workers[0].name, "stale-worker");
    }

    #[test]
    async fn test_redb_worker_find_stale_workers_multiple_stale() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let recent_time = chrono::Utc::now() - chrono::Duration::seconds(10);
        let stale_time1 = chrono::Utc::now() - chrono::Duration::minutes(5);
        let stale_time2 = chrono::Utc::now() - chrono::Duration::hours(2);

        let fresh_worker = Worker {
            id: WorkerId::new(),
            name: "fresh-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: recent_time.into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(2, 4096),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: recent_time,
        };

        let stale_worker1 = Worker {
            id: WorkerId::new(),
            name: "stale-worker-1".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: stale_time1.into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(4, 8192),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: stale_time1,
        };

        let stale_worker2 = Worker {
            id: WorkerId::new(),
            name: "stale-worker-2".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: stale_time2.into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(8, 16384),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: stale_time2,
        };

        repo.save_worker(&fresh_worker).await.unwrap();
        repo.save_worker(&stale_worker1).await.unwrap();
        repo.save_worker(&stale_worker2).await.unwrap();

        // Find workers stale for more than 1 minute
        let threshold = std::time::Duration::from_secs(60);
        let stale_workers = repo.find_stale_workers(threshold).await.unwrap();

        assert_eq!(stale_workers.len(), 2);

        let stale_names: Vec<String> = stale_workers.iter().map(|w| w.name.clone()).collect();
        assert!(stale_names.contains(&"stale-worker-1".to_string()));
        assert!(stale_names.contains(&"stale-worker-2".to_string()));
    }

    #[test]
    async fn test_redb_worker_find_stale_workers_none_stale() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        let recent_time = chrono::Utc::now() - chrono::Duration::seconds(10);

        let worker = Worker {
            id: WorkerId::new(),
            name: "fresh-worker".to_string(),
            status: hodei_core::WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: recent_time.into(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tenant_id: None,
            capabilities: hodei_core::WorkerCapabilities::new(2, 4096),
            metadata: HashMap::new(),
            current_jobs: vec![],
            last_heartbeat: recent_time,
        };

        repo.save_worker(&worker).await.unwrap();

        // Set threshold to 5 minutes - no workers should be stale
        let threshold = std::time::Duration::from_secs(300);
        let stale_workers = repo.find_stale_workers(threshold).await.unwrap();

        assert_eq!(stale_workers.len(), 0);
    }

    #[test]
    async fn test_redb_worker_find_stale_workers_empty_repository() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_workers.redb");

        let repo = RedbWorkerRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

        // Set threshold to 1 second - no workers should be stale
        let threshold = std::time::Duration::from_secs(1);
        let stale_workers = repo.find_stale_workers(threshold).await.unwrap();

        assert_eq!(stale_workers.len(), 0);
    }

    // ===== RedbPipelineRepository Tests =====

    #[test]
    async fn test_redb_pipeline_repository_init() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_pipelines.redb");

        let repo = RedbPipelineRepository::new_with_path(db_path).unwrap();
        let init_result = repo.init().await;

        assert!(init_result.is_ok());
    }

    #[test]
    async fn test_redb_pipeline_save_and_retrieve() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_pipelines.redb");

        let repo = RedbPipelineRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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
    async fn test_redb_pipeline_delete() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test_pipelines.redb");

        let repo = RedbPipelineRepository::new_with_path(db_path.clone()).unwrap();
        repo.init().await.unwrap();

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

            // Serialize job data
            let job_data = Self::job_to_bytes(job);
            let key = job.id.to_string().into_bytes();

            table
                .insert(key.as_slice(), job_data.as_slice())
                .map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to insert job: {}", e))
                })?;

            // Update in-memory cache (Performance Optimization)
            self.cache.insert(job.id.to_string(), job.clone());
        }

        tx.commit().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to commit job save: {}", e))
        })?;

        info!("Saved job to Redb with cache: {}", job.id);
        Ok(())
    }

    async fn get_job(&self, id: &JobId) -> Result<Option<Job>, JobRepositoryError> {
        // Check cache first (Performance Optimization)
        let id_str = id.to_string();
        if let Some(job) = self.cache.get(&id_str) {
            return Ok(Some(job.clone()));
        }

        // If not in cache, read from database
        let tx = self.db.begin_read().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(JOBS_TABLE).map_err(|e| {
            JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
        })?;

        let key = id_str.into_bytes();
        match table.get(key.as_slice()) {
            Ok(Some(value)) => {
                if let Some(job) = Self::bytes_to_job(value.value()) {
                    // Update cache
                    self.cache.insert(id.to_string(), job.clone());
                    Ok(Some(job))
                } else {
                    Ok(None)
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(JobRepositoryError::Database(format!(
                "Failed to get job: {}",
                e
            ))),
        }
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
        // Full table scan with cache support
        let tx = self.db.begin_read().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let jobs_table = tx.open_table(JOBS_TABLE).map_err(|e| {
            JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
        })?;

        let mut jobs = Vec::new();

        // Iterate through all jobs and filter by state
        let iter = jobs_table.iter().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to iterate jobs table: {}", e))
        })?;

        for item in iter {
            let (_, value) = item
                .map_err(|e| JobRepositoryError::Database(format!("Failed to read job: {}", e)))?;

            if let Some(job) = Self::bytes_to_job(value.value()) {
                if job.state.as_str() == "PENDING" {
                    jobs.push(job);
                }
            }
        }

        Ok(jobs)
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
        // Full table scan with cache support
        let tx = self.db.begin_read().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let jobs_table = tx.open_table(JOBS_TABLE).map_err(|e| {
            JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
        })?;

        let mut jobs = Vec::new();

        // Iterate through all jobs and filter by state
        let iter = jobs_table.iter().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to iterate jobs table: {}", e))
        })?;

        for item in iter {
            let (_, value) = item
                .map_err(|e| JobRepositoryError::Database(format!("Failed to read job: {}", e)))?;

            if let Some(job) = Self::bytes_to_job(value.value()) {
                if job.state.as_str() == "RUNNING" {
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
            // First, get the job to know its state for index cleanup
            let mut table = tx.open_table(JOBS_TABLE).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to open jobs table: {}", e))
            })?;

            let key = id.to_string().into_bytes();

            table.remove(key.as_slice()).map_err(|e| {
                JobRepositoryError::Database(format!("Failed to delete job: {}", e))
            })?;

            // Remove from cache
            self.cache.remove(&id.to_string());
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
            let (_job_bytes, old_job, should_update) = if let Some(value) = table
                .get(key.as_slice())
                .map_err(|e| JobRepositoryError::Database(format!("Failed to get job: {}", e)))?
            {
                let job_bytes = value.value().to_vec();

                let job: Job = serde_json::from_slice(&job_bytes).map_err(|e| {
                    JobRepositoryError::Database(format!("Failed to deserialize job: {}", e))
                })?;

                if job.state.as_str() == expected_state {
                    (job_bytes, job, true)
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

                let mut job = old_job;

                // Delegate state transition validation to domain layer
                match job.compare_and_swap_status(expected_state, new_state) {
                    Ok(updated) => {
                        if updated {
                            // State was successfully updated
                            let new_value = serde_json::to_vec(&job).map_err(|e| {
                                JobRepositoryError::Database(format!(
                                    "Failed to serialize job: {}",
                                    e
                                ))
                            })?;

                            table
                                .insert(key.as_slice(), new_value.as_slice())
                                .map_err(|e| {
                                    JobRepositoryError::Database(format!(
                                        "Failed to insert job: {}",
                                        e
                                    ))
                                })?;

                            // Update cache
                            self.cache.insert(id.to_string(), job);

                            swapped = true;
                        }
                        // else: state didn't match (already checked above, so shouldn't happen)
                    }
                    Err(e) => {
                        // Invalid state transition - domain rule violation
                        return Err(JobRepositoryError::Validation(e.to_string()));
                    }
                }
            }
        }

        tx.commit().map_err(|e| {
            JobRepositoryError::Database(format!("Failed to commit status swap: {}", e))
        })?;

        Ok(swapped)
    }

    async fn assign_worker(
        &self,
        _job_id: &JobId,
        _worker_id: &WorkerId,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement worker assignment in redb
        Ok(())
    }

    async fn set_job_start_time(
        &self,
        _job_id: &JobId,
        _start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement start time tracking in redb
        Ok(())
    }

    async fn set_job_finish_time(
        &self,
        _job_id: &JobId,
        _finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement finish time tracking in redb
        Ok(())
    }

    async fn set_job_duration(
        &self,
        _job_id: &JobId,
        _duration_ms: i64,
    ) -> Result<(), JobRepositoryError> {
        // TODO: Implement duration tracking in redb
        Ok(())
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

    async fn update_last_seen(&self, id: &WorkerId) -> Result<(), WorkerRepositoryError> {
        let tx = self.db.begin_write().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to begin write transaction: {}", e))
        })?;

        let key = id.to_string().into_bytes();

        // First, get the current worker data outside the mutable borrow scope
        let worker_data = {
            let table = tx.open_table(WORKERS_TABLE).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to open workers table: {}", e))
            })?;

            match table.get(key.as_slice()).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to get worker: {}", e))
            })? {
                Some(value) => {
                    let worker: Worker = serde_json::from_slice(value.value()).map_err(|e| {
                        WorkerRepositoryError::Database(format!(
                            "Failed to deserialize worker: {}",
                            e
                        ))
                    })?;
                    serde_json::to_vec(&worker).map_err(|e| {
                        WorkerRepositoryError::Database(format!(
                            "Failed to serialize worker: {}",
                            e
                        ))
                    })?
                }
                None => {
                    return Err(WorkerRepositoryError::NotFound(id.clone()));
                }
            }
        };

        // Now update the worker in a new scope with mutable borrow
        {
            let mut table = tx.open_table(WORKERS_TABLE).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to open workers table: {}", e))
            })?;

            // Deserialize, update, serialize
            let mut worker: Worker = serde_json::from_slice(&worker_data).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to deserialize worker: {}", e))
            })?;

            // Update the last_heartbeat timestamp
            worker.last_heartbeat = chrono::Utc::now();

            // Serialize and save the updated worker
            let updated_value = serde_json::to_vec(&worker).map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to serialize worker: {}", e))
            })?;

            table
                .insert(key.as_slice(), updated_value.as_slice())
                .map_err(|e| {
                    WorkerRepositoryError::Database(format!(
                        "Failed to insert updated worker: {}",
                        e
                    ))
                })?;

            info!("Updated last_seen for worker: {}", id);
        }

        tx.commit().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to commit last_seen update: {}", e))
        })?;

        Ok(())
    }

    async fn find_stale_workers(
        &self,
        threshold_duration: std::time::Duration,
    ) -> Result<Vec<Worker>, WorkerRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(WORKERS_TABLE).map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to open workers table: {}", e))
        })?;

        let threshold_time =
            chrono::Utc::now() - chrono::Duration::from_std(threshold_duration).unwrap_or_default();

        let mut stale_workers = Vec::new();

        // Iterate through all workers and check heartbeat timestamps
        let iter = table.iter().map_err(|e| {
            WorkerRepositoryError::Database(format!("Failed to iterate workers: {}", e))
        })?;

        for item in iter {
            let (_, value) = item.map_err(|e| {
                WorkerRepositoryError::Database(format!("Failed to read worker: {}", e))
            })?;

            if let Ok(worker) = serde_json::from_slice::<Worker>(value.value()) {
                if worker.last_heartbeat < threshold_time {
                    stale_workers.push(worker);
                }
            }
        }

        info!(
            "Found {} stale workers (threshold: {} seconds)",
            stale_workers.len(),
            threshold_duration.as_secs()
        );

        Ok(stale_workers)
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

    async fn get_all_pipelines(&self) -> Result<Vec<Pipeline>, PipelineRepositoryError> {
        let tx = self.db.begin_read().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(PIPELINES_TABLE).map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to open pipelines table: {}", e))
        })?;

        let mut pipelines = Vec::new();

        // Collect all pipeline entries
        let iter = table.iter().map_err(|e| {
            PipelineRepositoryError::Database(format!("Failed to iterate pipelines: {}", e))
        })?;

        for item in iter {
            let (_key, value) = item.map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to get pipeline entry: {}", e))
            })?;

            let pipeline_bytes = value.value();
            let pipeline: Pipeline = bincode::deserialize(pipeline_bytes).map_err(|e| {
                PipelineRepositoryError::Database(format!("Failed to deserialize pipeline: {}", e))
            })?;

            pipelines.push(pipeline);
        }

        info!("Retrieved {} pipelines from redb", pipelines.len());
        Ok(pipelines)
    }
}
