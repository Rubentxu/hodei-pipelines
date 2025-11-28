//! Redb (Embedded Key-Value Store) Job Repository
//!
//! Production-ready implementation using Redb for embedded storage.
//! Redb provides ACID transactions, MVCC, and excellent performance.

use async_trait::async_trait;
use hodei_core::{DomainError, Job, JobId, Result, WorkerId};
use hodei_ports::JobRepository;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Redb Job Repository
#[derive(Debug, Clone)]
pub struct RedbJobRepository {
    db: Arc<Mutex<Database>>,
}

impl RedbJobRepository {
    /// Create a new RedbJobRepository with in-memory database
    pub fn new_in_memory() -> Result<Self> {
        let db = Arc::new(Mutex::new(Database::create("mem:redb-jobs").map_err(
            |e| DomainError::Infrastructure(format!("Failed to create Redb database: {}", e)),
        )?));

        Ok(Self { db })
    }

    /// Create a new RedbJobRepository with database file
    pub fn new_with_path(path: &str) -> Result<Self> {
        let db = Arc::new(Mutex::new(Database::create(path).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create Redb database: {}", e))
        })?));

        Ok(Self { db })
    }

    /// Initialize database schema
    pub async fn init_schema(&self) -> Result<()> {
        info!("Initializing Redb schema for jobs");

        let db = self.db.lock().await;
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        // Create jobs table
        tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create jobs table: {}", e))
        })?;

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        info!("Redb schema initialized successfully");
        Ok(())
    }

    /// Table definition for jobs - using [u8; 16] as key (UUID is 16 bytes)
    const JOBS_TABLE: TableDefinition<'_, [u8; 16], Vec<u8>> = TableDefinition::new("jobs");

    /// Serialize job to bytes using JSON
    fn job_to_bytes(job: &Job) -> Vec<u8> {
        // Use JSON serialization for compatibility with all serde types
        let json = serde_json::to_string(job).expect("Failed to serialize job to JSON");
        let bytes = json.into_bytes();
        tracing::debug!(
            "Successfully serialized job to JSON ({} bytes)",
            bytes.len()
        );
        bytes
    }

    /// Deserialize job from bytes using JSON
    fn bytes_to_job(data: &[u8]) -> Option<Job> {
        eprintln!("🔍 Attempting to deserialize {} bytes of data", data.len());

        // Check if we have any data
        if data.is_empty() {
            eprintln!("❌ No data to deserialize");
            return None;
        }

        // Convert bytes to string and deserialize from JSON
        match String::from_utf8(data.to_vec()) {
            Ok(json_str) => match serde_json::from_str::<Job>(&json_str) {
                Ok(job) => {
                    eprintln!("✅ Successfully deserialized job with name: {}", job.name);
                    Some(job)
                }
                Err(e) => {
                    eprintln!("❌ Failed to deserialize JSON to Job:");
                    eprintln!("   Error: {:?}", e);
                    eprintln!(
                        "   JSON (first 200 chars): {}",
                        &json_str[..std::cmp::min(json_str.len(), 200)]
                    );
                    None
                }
            },
            Err(e) => {
                eprintln!("❌ Failed to convert bytes to UTF-8:");
                eprintln!("   Error: {:?}", e);
                None
            }
        }
    }

    /// Convert JobId to bytes
    fn job_id_to_bytes(job_id: &JobId) -> [u8; 16] {
        *job_id.as_uuid().as_bytes()
    }
}

#[async_trait]
impl JobRepository for RedbJobRepository {
    async fn save_job(&self, job: &Job) -> Result<()> {
        debug!("Saving job: {}", job.id);

        let db = self.db.lock().await;
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        let mut table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(&job.id);
        let value = Self::job_to_bytes(job);

        table
            .insert(key, value)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to insert job: {}", e)))?;

        drop(table); // Explicitly drop before commit

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn get_job(&self, job_id: &JobId) -> Result<Option<Job>> {
        let db = self.db.lock().await;
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(job_id);
        let value = table
            .get(key)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        if let Some(val) = value {
            let job = Self::bytes_to_job(&val.value())
                .ok_or_else(|| DomainError::Validation("Failed to deserialize job".to_string()))?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>> {
        let db = self.db.lock().await;
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let mut pending_jobs = Vec::new();
        let iter = table.iter().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to iterate jobs table: {}", e))
        })?;

        for item in iter {
            let (_key, value) = item
                .map_err(|e| DomainError::Infrastructure(format!("Failed to read job: {}", e)))?;

            if let Some(job) = Self::bytes_to_job(&value.value()) {
                if job.state.as_str() == "PENDING" {
                    pending_jobs.push(job);
                }
            }
        }

        Ok(pending_jobs)
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>> {
        let db = self.db.lock().await;
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let mut running_jobs = Vec::new();
        let iter = table.iter().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to iterate jobs table: {}", e))
        })?;

        for item in iter {
            let (_key, value) = item
                .map_err(|e| DomainError::Infrastructure(format!("Failed to read job: {}", e)))?;

            if let Some(job) = Self::bytes_to_job(&value.value()) {
                if job.state.as_str() == "RUNNING" {
                    running_jobs.push(job);
                }
            }
        }

        Ok(running_jobs)
    }

    async fn delete_job(&self, job_id: &JobId) -> Result<()> {
        let db = self.db.lock().await;
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        let mut table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(job_id);
        table
            .remove(key)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to delete job: {}", e)))?;

        drop(table); // Explicitly drop before commit

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn compare_and_swap_status(
        &self,
        job_id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool> {
        let db = self.db.lock().await;

        // First, read transaction to get the job
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(job_id);
        let value = table
            .get(key)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        let job = if let Some(val) = value {
            Self::bytes_to_job(&val.value())
                .ok_or_else(|| DomainError::Validation("Failed to deserialize job".to_string()))?
        } else {
            return Ok(false);
        };

        if job.state.as_str() != expected_state {
            return Ok(false);
        }

        // Update state
        let updated_job = Job {
            state: hodei_core::job::JobState::new(new_state.to_string()).map_err(|_| {
                DomainError::Validation(format!("Invalid job state: {}", new_state))
            })?,
            ..job
        };

        let updated_value = Self::job_to_bytes(&updated_job);

        // Now perform write transaction
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin write transaction: {}", e))
        })?;

        let mut table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        table
            .insert(key, updated_value)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to update job: {}", e)))?;

        drop(table); // Explicitly drop before commit

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(true)
    }

    async fn assign_worker(&self, job_id: &JobId, _worker_id: &WorkerId) -> Result<()> {
        let db = self.db.lock().await;

        // First, read transaction to verify job exists
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(job_id);
        let value = table
            .get(key)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        if value.is_none() {
            return Err(DomainError::NotFound(format!("Job not found: {}", job_id)));
        }

        // Worker assignment is tracked externally in this implementation
        // This is a no-op but keeps the interface compatible
        Ok(())
    }

    async fn set_job_start_time(
        &self,
        job_id: &JobId,
        start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let db = self.db.lock().await;

        // First, read transaction to get the job
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(job_id);
        let value = table
            .get(key)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        let job = if let Some(val) = value {
            Self::bytes_to_job(&val.value())
                .ok_or_else(|| DomainError::Validation("Failed to deserialize job".to_string()))?
        } else {
            return Err(DomainError::NotFound(format!("Job not found: {}", job_id)));
        };

        // Now perform write transaction
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin write transaction: {}", e))
        })?;

        let mut table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let mut updated_job = job;
        updated_job.started_at = Some(start_time);
        updated_job.updated_at = chrono::Utc::now();

        let updated_value = Self::job_to_bytes(&updated_job);
        table
            .insert(key, updated_value)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to update job: {}", e)))?;

        drop(table); // Explicitly drop before commit

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn set_job_finish_time(
        &self,
        job_id: &JobId,
        finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let db = self.db.lock().await;

        // First, read transaction to get the job
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(job_id);
        let value = table
            .get(key)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        let job = if let Some(val) = value {
            Self::bytes_to_job(&val.value())
                .ok_or_else(|| DomainError::Validation("Failed to deserialize job".to_string()))?
        } else {
            return Err(DomainError::NotFound(format!("Job not found: {}", job_id)));
        };

        // Now perform write transaction
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin write transaction: {}", e))
        })?;

        let mut table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let mut updated_job = job;
        updated_job.completed_at = Some(finish_time);
        updated_job.updated_at = chrono::Utc::now();

        let updated_value = Self::job_to_bytes(&updated_job);
        table
            .insert(key, updated_value)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to update job: {}", e)))?;

        drop(table); // Explicitly drop before commit

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn set_job_duration(&self, _job_id: &JobId, _duration_ms: i64) -> Result<()> {
        // Duration is calculated from started_at and completed_at
        // This is a no-op for now
        Ok(())
    }

    async fn create_job(&self, job_spec: hodei_core::job::JobSpec) -> Result<JobId> {
        let job_id = JobId::new();
        let job = Job::new(job_id, job_spec)?;
        self.save_job(&job).await?;
        Ok(job_id)
    }

    async fn update_job_state(
        &self,
        job_id: &JobId,
        state: hodei_core::job::JobState,
    ) -> Result<()> {
        let db = self.db.lock().await;

        // First, read transaction to get the job
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin read transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let key = Self::job_id_to_bytes(job_id);
        let value = table
            .get(key)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        let job = if let Some(val) = value {
            Self::bytes_to_job(&val.value())
                .ok_or_else(|| DomainError::Validation("Failed to deserialize job".to_string()))?
        } else {
            return Err(DomainError::NotFound(format!("Job not found: {}", job_id)));
        };

        // Now perform write transaction
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin write transaction: {}", e))
        })?;

        let mut table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let mut updated_job = job;
        updated_job.state = state;
        updated_job.updated_at = chrono::Utc::now();

        let updated_value = Self::job_to_bytes(&updated_job);
        table
            .insert(key, updated_value)
            .map_err(|e| DomainError::Infrastructure(format!("Failed to update job: {}", e)))?;

        drop(table); // Explicitly drop before commit

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn list_jobs(&self) -> Result<Vec<Job>> {
        let db = self.db.lock().await;
        let tx = db.begin_read().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        let table = tx.open_table(Self::JOBS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to open jobs table: {}", e))
        })?;

        let mut jobs = Vec::new();
        let iter = table.iter().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to iterate jobs table: {}", e))
        })?;

        for item in iter {
            let (_key, value) = item
                .map_err(|e| DomainError::Infrastructure(format!("Failed to read job: {}", e)))?;

            if let Some(job) = Self::bytes_to_job(&value.value()) {
                jobs.push(job);
            }
        }

        Ok(jobs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::job::JobSpec;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_redb_job_save_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        // First, test serialization directly
        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: hodei_core::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job = Job::new(JobId::new(), job_spec).unwrap();
        let job_id = job.id;

        // Test serialization
        let serialized = RedbJobRepository::job_to_bytes(&job);
        eprintln!("🔍 Serialized job to {} bytes", serialized.len());

        // Test deserialization directly with JSON
        match String::from_utf8(serialized.clone()) {
            Ok(json_str) => match serde_json::from_str::<Job>(&json_str) {
                Ok(deserialized_job) => {
                    assert_eq!(deserialized_job.id, job_id, "Job ID mismatch");
                    assert_eq!(deserialized_job.name, job.name, "Job name mismatch");
                    eprintln!("✅ Direct serialization test passed");
                }
                Err(e) => {
                    eprintln!("❌ Direct JSON deserialization failed:");
                    eprintln!("   Error: {:?}", e);
                    panic!("❌ Deserialization failed: {:?}", e);
                }
            },
            Err(e) => {
                eprintln!("❌ Failed to convert bytes to UTF-8:");
                panic!("❌ UTF-8 conversion failed: {:?}", e);
            }
        }

        // Now test via repository
        repo.save_job(&job).await.unwrap();

        let retrieved = repo.get_job(&job_id).await.unwrap();
        assert!(
            retrieved.is_some(),
            "Failed to retrieve job from repository"
        );
        assert_eq!(retrieved.unwrap().id, job_id, "Retrieved job ID mismatch");
    }

    #[tokio::test]
    async fn test_redb_job_get_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let job_id = JobId::new();
        let retrieved = repo.get_job(&job_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_redb_job_compare_and_swap_success() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: hodei_core::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job = Job::new(JobId::new(), job_spec).unwrap();
        repo.save_job(&job).await.unwrap();

        let swapped = repo
            .compare_and_swap_status(&job.id, "PENDING", "RUNNING")
            .await
            .unwrap();
        assert!(swapped);

        let retrieved = repo.get_job(&job.id).await.unwrap().unwrap();
        assert_eq!(retrieved.state.as_str(), "RUNNING");
    }

    #[tokio::test]
    async fn test_redb_job_compare_and_swap_failed() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: hodei_core::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job = Job::new(JobId::new(), job_spec).unwrap();
        repo.save_job(&job).await.unwrap();

        let swapped = repo
            .compare_and_swap_status(&job.id, "RUNNING", "PENDING")
            .await
            .unwrap();
        assert!(!swapped);

        let retrieved = repo.get_job(&job.id).await.unwrap().unwrap();
        assert_eq!(retrieved.state.as_str(), "PENDING");
    }

    #[tokio::test]
    async fn test_redb_job_compare_and_swap_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let job_id = JobId::new();
        let swapped = repo
            .compare_and_swap_status(&job_id, "PENDING", "RUNNING")
            .await
            .unwrap();
        assert!(!swapped);
    }

    #[tokio::test]
    async fn test_redb_job_delete() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: hodei_core::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job = Job::new(JobId::new(), job_spec).unwrap();
        repo.save_job(&job).await.unwrap();

        repo.delete_job(&job.id).await.unwrap();

        let retrieved = repo.get_job(&job.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_redb_job_get_pending_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        // Create pending job
        let job_spec = JobSpec {
            name: "pending-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: hodei_core::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job1 = Job::new(JobId::new(), job_spec.clone()).unwrap();
        repo.save_job(&job1).await.unwrap();

        // Create running job
        let mut job2 = Job::new(JobId::new(), job_spec).unwrap();
        job2.state = hodei_core::job::JobState::new("RUNNING".to_string()).unwrap();
        repo.save_job(&job2).await.unwrap();

        let pending = repo.get_pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, job1.id);
    }

    #[tokio::test]
    async fn test_redb_job_get_running_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        // Create pending job
        let job_spec = JobSpec {
            name: "pending-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: hodei_core::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job1 = Job::new(JobId::new(), job_spec.clone()).unwrap();
        repo.save_job(&job1).await.unwrap();

        // Create running job
        let mut job2 = Job::new(JobId::new(), job_spec).unwrap();
        job2.state = hodei_core::job::JobState::new("RUNNING".to_string()).unwrap();
        repo.save_job(&job2).await.unwrap();

        let running = repo.get_running_jobs().await.unwrap();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, job2.id);
    }

    #[tokio::test]
    async fn test_redb_job_empty_pending_and_running() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let pending = repo.get_pending_jobs().await.unwrap();
        assert_eq!(pending.len(), 0);

        let running = repo.get_running_jobs().await.unwrap();
        assert_eq!(running.len(), 0);
    }

    #[tokio::test]
    async fn test_redb_job_list_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: hodei_core::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        };

        let job1 = Job::new(JobId::new(), job_spec.clone()).unwrap();
        let job2 = Job::new(JobId::new(), job_spec).unwrap();

        repo.save_job(&job1).await.unwrap();
        repo.save_job(&job2).await.unwrap();

        let jobs = repo.list_jobs().await.unwrap();
        assert_eq!(jobs.len(), 2);
    }
}
