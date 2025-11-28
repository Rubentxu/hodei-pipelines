//! Cached Job Repository Integration Tests with TestContainers
//!
//! These tests validate the CachedRepository hybrid storage implementation
//! using real PostgreSQL instances spun up via TestContainers and Redb instances.

use std::collections::HashMap;
use testcontainers::clients::Cli;
use testcontainers::containers::Postgres;
use testcontainers::Container;
use tracing::info;

use hodei_adapters::{CachedJobRepository, RedbJobRepository};
use hodei_core::job::{Job, JobId, JobSpec, ResourceQuota};
use hodei_ports::JobRepository;

const POSTGRES_IMAGE: &str = "postgres:15-alpine";
const POSTGRES_USER: &str = "postgres";
const POSTGRES_PASSWORD: &str = "postgres";
const POSTGRES_DB: &str = "testdb";

/// TestContainer setup for PostgreSQL
struct PostgresTestContainer {
    container: Container<Postgres>,
    connection_string: String,
    port: u16,
}

impl PostgresTestContainer {
    async fn start() -> Self {
        info!("🐳 Starting PostgreSQL TestContainer...");

        let docker = Cli::default();
        let node = docker.run(
            Postgres::new(POSTGRES_IMAGE)
                .with_env_var(("POSTGRES_USER", POSTGRES_USER))
                .with_env_var(("POSTGRES_PASSWORD", POSTGRES_PASSWORD))
                .with_env_var(("POSTGRES_DB", POSTGRES_DB)),
        );

        let port = node.get_host_port_ipv4(5432);
        let connection_string = format!(
            "postgresql://{}:{}@localhost:{}/{}",
            POSTGRES_USER, POSTGRES_PASSWORD, port, POSTGRES_DB
        );

        // Wait for PostgreSQL to be ready
        let mut attempts = 0;
        let max_attempts = 30;
        loop {
            match sqlx::PgPool::connect(&connection_string).await {
                Ok(_) => {
                    info!("✅ PostgreSQL TestContainer is ready on port {}", port);
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        panic!(
                            "Failed to connect to PostgreSQL after {} attempts",
                            attempts
                        );
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        Self {
            container: node,
            connection_string,
            port,
        }
    }
}

/// Create a CachedJobRepository for testing
async fn create_cached_repo() -> (CachedJobRepository, tempfile::TempDir) {
    // Start PostgreSQL container
    let pg_container = PostgresTestContainer::start().await;

    // Create database pool
    let pool = sqlx::PgPool::connect(&pg_container.connection_string)
        .await
        .expect("Failed to create PostgreSQL pool");

    // Create temporary directory for Redb
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("cache.redb");

    // Create Redb repository
    let redb_repo = RedbJobRepository::new_with_path(db_path.to_str().unwrap())
        .expect("Failed to create Redb repository");
    redb_repo
        .init_schema()
        .await
        .expect("Failed to init Redb schema");

    // Create CachedRepository
    let cached_repo = CachedJobRepository::new(redb_repo, pool);

    // Initialize database schema
    cached_repo
        .init_schema()
        .await
        .expect("Failed to init database schema");

    (cached_repo, temp_dir)
}

#[tokio::test]
async fn test_cached_repository_basic_operations() {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting CachedRepository Integration Test");

    // Create CachedRepository with TestContainers
    let (repo, _temp_dir) = create_cached_repo().await;
    info!("✅ CachedRepository initialized with PostgreSQL and Redb");

    // Create test job spec
    let job_spec = JobSpec {
        name: "test-cached-job".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["echo".to_string(), "hello cached".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    // Create and save job
    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");

    info!("💾 Saving job to CachedRepository...");
    repo.save_job(&job).await.expect("Failed to save job");
    info!("✅ Job saved successfully");

    // Retrieve job (should be cached)
    info!("📖 Retrieving job from CachedRepository...");
    let retrieved = repo
        .get_job(&job_id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(retrieved.id, job_id);
    assert_eq!(retrieved.name, "test-cached-job");
    info!("✅ Job retrieved and verified successfully");

    // Check cache stats
    let stats = repo.get_stats().await;
    info!(
        "📊 Cache stats: hits={}, misses={}, db_reads={}, db_writes={}",
        stats.cache_hits, stats.cache_misses, stats.db_reads, stats.db_writes
    );

    // First get should be a write (not in cache yet)
    assert_eq!(stats.db_writes, 1);
    // Second get should be a hit (in cache now)
    let _retrieved2 = repo.get_job(&job_id).await.unwrap().unwrap();
    let stats2 = repo.get_stats().await;
    assert_eq!(stats2.cache_hits, 1);
}

#[tokio::test]
async fn test_cached_repository_cache_miss() {
    tracing_subscriber::fmt::init();

    info!("🚀 Testing CachedRepository cache miss scenario");

    // Create CachedRepository
    let (repo, _temp_dir) = create_cached_repo().await;

    // Create and save job
    let job_spec = JobSpec {
        name: "test-cache-miss".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["echo".to_string(), "cache miss".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");
    repo.save_job(&job).await.expect("Failed to save job");

    // Create a new repository instance (simulates fresh cache)
    // Note: In production, cache would be shared, but for testing we simulate a cache miss
    let (repo2, _temp_dir) = create_cached_repo().await;

    // Try to get job (should be cache miss, load from DB)
    info!("📖 Retrieving job (expecting cache miss)...");
    let retrieved = repo2
        .get_job(&job_id)
        .await
        .expect("Failed to get job")
        .expect("Job not found in DB");

    assert_eq!(retrieved.id, job_id);
    assert_eq!(retrieved.name, "test-cache-miss");

    // Check stats - should show cache miss and DB read
    let stats = repo2.get_stats().await;
    info!(
        "📊 Cache stats after cache miss: hits={}, misses={}, db_reads={}, db_writes={}",
        stats.cache_hits, stats.cache_misses, stats.db_reads, stats.db_writes
    );

    assert_eq!(stats.cache_misses, 1);
    assert_eq!(stats.db_reads, 1);
    assert_eq!(stats.db_writes, 0); // We didn't write in this instance

    info!("✅ Cache miss handled correctly");
}

#[tokio::test]
async fn test_cached_repository_write_through() {
    tracing_subscriber::fmt::init();

    info!("🚀 Testing CachedRepository write-through strategy");

    // Create CachedRepository
    let (repo, _temp_dir) = create_cached_repo().await;

    // Create test job
    let job_spec = JobSpec {
        name: "test-write-through".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["echo".to_string(), "write through".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");

    // Write job (should update both cache and DB)
    info!("💾 Writing job (write-through strategy)...");
    repo.save_job(&job).await.expect("Failed to save job");

    // Check stats - should show DB write
    let stats = repo.get_stats().await;
    assert_eq!(stats.db_writes, 1);
    info!("✅ Write-through completed: job written to both cache and DB");

    // Verify job is in cache
    let retrieved = repo.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, job_id);

    let stats2 = repo.get_stats().await;
    assert_eq!(stats2.cache_hits, 1);
    info!("✅ Job retrieved from cache");
}

#[tokio::test]
async fn test_cached_repository_state_transitions() {
    tracing_subscriber::fmt::init();

    info!("🚀 Testing CachedRepository state transitions");

    // Create CachedRepository
    let (repo, _temp_dir) = create_cached_repo().await;

    // Create test job
    let job_spec = JobSpec {
        name: "test-state-transition".to_string(),
        image: "ubuntu:latest".to_string(),
        command: vec!["sleep".to_string(), "100".to_string()],
        resources: ResourceQuota::default(),
        timeout_ms: 300000,
        retries: 3,
        env: HashMap::new(),
        secret_refs: Vec::new(),
    };

    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");
    repo.save_job(&job).await.expect("Failed to save job");

    // Transition state from PENDING to RUNNING
    info!("🔄 Transitioning job state: PENDING -> RUNNING...");
    let swapped = repo
        .compare_and_swap_status(&job_id, "PENDING", "RUNNING")
        .await
        .expect("Failed to compare and swap");
    assert!(swapped, "Expected successful state transition");

    // Verify state in cache
    let retrieved = repo.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(retrieved.state.as_str(), "RUNNING");

    // Verify state in DB (by creating new repo instance and checking)
    let (repo2, _temp_dir) = create_cached_repo().await;
    let retrieved2 = repo2.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(retrieved2.state.as_str(), "RUNNING");

    info!("✅ State transition propagated to both cache and DB");
}

#[tokio::test]
async fn test_cached_repository_deletion() {
    tracing_subscriber::fmt::init();

    info!("🚀 Testing CachedRepository deletion");

    // Create CachedRepository
    let (repo, _temp_dir) = create_cached_repo().await;

    // Create test job
    let job_spec = JobSpec {
        name: "test-deletion".to_string(),
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
    assert!(retrieved.is_some());

    // Delete job
    info!("🗑️ Deleting job...");
    repo.delete_job(&job_id)
        .await
        .expect("Failed to delete job");

    // Verify job is deleted from cache
    let retrieved = repo.get_job(&job_id).await.unwrap();
    assert!(retrieved.is_none());

    // Verify job is deleted from DB (new repo instance)
    let (repo2, _temp_dir) = create_cached_repo().await;
    let retrieved2 = repo2.get_job(&job_id).await.unwrap();
    assert!(retrieved2.is_none());

    info!("✅ Job deleted from both cache and DB");
}
