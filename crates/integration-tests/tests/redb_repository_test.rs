//! Redb Job Repository Integration Tests
//!
//! These tests validate the RedbJobRepository implementation using
//! temporary directories (Redb is embedded, not a server-based database).

use std::collections::HashMap;
use tracing::info;

use hodei_adapters::redb::RedbJobRepository;
use hodei_core::job::{Job, JobId, JobSpec, ResourceQuota};
use hodei_ports::JobRepository;

fn init_tracing() {
    let _ = tracing_subscriber::fmt::try_init();
}

/// Helper to create a temporary Redb repository for testing
struct RedbTestRepo {
    temp_dir: tempfile::TempDir,
}

impl RedbTestRepo {
    async fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        info!("🔧 Using temporary Redb database at: {:?}", temp_dir.path());

        Self { temp_dir }
    }

    fn db_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("test.redb")
    }

    async fn create_repo(&self) -> RedbJobRepository {
        let repo = RedbJobRepository::new_with_path(self.db_path().to_str().unwrap())
            .expect("Failed to create Redb repository");
        repo.init_schema().await.expect("Failed to init schema");
        repo
    }
}

#[tokio::test]
async fn test_redb_job_save_and_retrieve() {
    init_tracing();

    info!("🚀 Starting Redb Job Repository Integration Test");

    // Setup Redb test repository (using temp dir)
    let test_repo = RedbTestRepo::new().await;
    let repo = test_repo.create_repo().await;
    info!("✅ Redb repository initialized");

    // Create test job spec
    let job_spec = JobSpec {
        name: "test-redb-job".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["echo".to_string(), "hello redb".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    // Create and save job
    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");

    info!("💾 Saving job to Redb...");
    repo.save_job(&job).await.expect("Failed to save job");
    info!("✅ Job saved successfully");

    // Retrieve job
    info!("📖 Retrieving job from Redb...");
    let retrieved = repo
        .get_job(&job_id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(retrieved.id, job_id);
    assert_eq!(retrieved.name, "test-redb-job");
    info!("✅ Job retrieved and verified successfully");

    // Test job listing
    let jobs = repo.list_jobs().await.expect("Failed to list jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, job_id);
    info!("✅ Job listing works correctly");
}

#[tokio::test]
async fn test_redb_job_compare_and_swap() {
    init_tracing();

    info!("🚀 Testing Redb compare-and-swap operations");

    // Setup Redb repository
    let test_repo = RedbTestRepo::new().await;
    let repo = test_repo.create_repo().await;

    // Create test job
    let job_spec = JobSpec {
        name: "test-cas-job".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["sleep".to_string(), "10".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");
    repo.save_job(&job).await.expect("Failed to save job");

    // Test successful state transition
    info!("🔄 Testing successful state transition...");
    let swapped = repo
        .compare_and_swap_status(&job_id, "PENDING", "RUNNING")
        .await
        .expect("Failed to compare and swap");
    assert!(swapped, "Expected successful swap");

    let retrieved = repo.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(retrieved.state.as_str(), "RUNNING");
    info!("✅ State transition successful");

    // Test failed state transition (wrong expected state)
    info!("🔄 Testing failed state transition...");
    let swapped = repo
        .compare_and_swap_status(&job_id, "PENDING", "FAILED")
        .await
        .expect("Failed to compare and swap");
    assert!(!swapped, "Expected failed swap");

    let retrieved = repo.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(retrieved.state.as_str(), "RUNNING");
    info!("✅ Failed state transition handled correctly");
}

#[tokio::test]
async fn test_redb_job_deletion() {
    init_tracing();

    info!("🚀 Testing Redb job deletion");

    // Setup Redb repository
    let test_repo = RedbTestRepo::new().await;
    let repo = test_repo.create_repo().await;

    // Create test job
    let job_spec = JobSpec {
        name: "test-delete-job".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["echo".to_string(), "delete me".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");
    repo.save_job(&job).await.expect("Failed to save job");

    // Verify job exists
    let retrieved = repo.get_job(&job_id).await.unwrap();
    assert!(retrieved.is_some(), "Job should exist before deletion");

    // Delete job
    info!("🗑️ Deleting job...");
    repo.delete_job(&job_id)
        .await
        .expect("Failed to delete job");

    // Verify job is deleted
    let retrieved = repo.get_job(&job_id).await.unwrap();
    assert!(retrieved.is_none(), "Job should not exist after deletion");
    info!("✅ Job deleted successfully");
}

#[tokio::test]
async fn test_redb_pending_and_running_jobs() {
    init_tracing();

    info!("🚀 Testing Redb pending and running jobs filtering");

    // Setup Redb repository
    let test_repo = RedbTestRepo::new().await;
    let repo = test_repo.create_repo().await;

    // Create multiple jobs with different states
    let pending_spec = JobSpec {
        name: "pending-job".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["sleep".to_string(), "100".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    let running_spec = JobSpec {
        name: "running-job".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["sleep".to_string(), "100".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    // Create jobs
    let pending_id = JobId::new();
    let running_id = JobId::new();

    let pending_job = Job::new(pending_id.clone(), pending_spec).expect("Failed to create job");
    let running_job = Job::new(running_id.clone(), running_spec).expect("Failed to create job");

    // Save jobs
    repo.save_job(&pending_job)
        .await
        .expect("Failed to save pending job");
    repo.save_job(&running_job)
        .await
        .expect("Failed to save running job");

    // Update running job state
    let _ = repo
        .compare_and_swap_status(&running_id, "PENDING", "RUNNING")
        .await;

    // Test pending jobs
    let pending_jobs = repo
        .get_pending_jobs()
        .await
        .expect("Failed to get pending jobs");
    assert_eq!(pending_jobs.len(), 1);
    assert_eq!(pending_jobs[0].id, pending_id);
    info!("✅ Pending jobs filtered correctly");

    // Test running jobs
    let running_jobs = repo
        .get_running_jobs()
        .await
        .expect("Failed to get running jobs");
    assert_eq!(running_jobs.len(), 1);
    assert_eq!(running_jobs[0].id, running_id);
    info!("✅ Running jobs filtered correctly");
}
