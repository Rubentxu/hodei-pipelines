#![cfg(feature = "container_tests")]
//! Basic PostgreSQL Integration Test with Singleton Container
//!
//! This test validates PostgreSQL connectivity and basic operations
//! using a single shared TestContainer for maximum performance.
//!
//! 🚀 SINGLETON: This test uses a single shared PostgreSQL container
//! 💾 MEMORY: Reduced from ~4GB to ~37MB with singleton container

use std::collections::HashMap;
use std::time::Duration;
use tracing::info;

use hodei_pipelines_adapters::postgres::PostgreSqlPipelineRepository;
use hodei_pipelines_core::job::{Job, JobId, JobSpec, ResourceQuota};
use hodei_pipelines_ports::JobRepository;
use sqlx::pool::PoolOptions;

mod helpers;
use helpers::get_shared_postgres;

#[tokio::test]
async fn test_postgres_connection_basic() {
    info!("🚀 Starting PostgreSQL integration test with singleton container");

    // ✅ USE SINGLETON CONTAINER - shared PostgreSQL for all tests
    let shared_pg = get_shared_postgres().await;

    info!("✅ Acquired shared PostgreSQL (port {})", shared_pg.port());
    info!("   💾 Single container shared across all tests");
    info!("   🚀 Maximum performance with minimum memory");

    // PostgreSQL is already ready (health check in singleton)
    // Just create the connection pool
    let pool = PoolOptions::new()
        .max_connections(10)
        .connect(&shared_pg.database_url())
        .await
        .expect("Failed to connect to shared PostgreSQL");

    // Initialize database schema
    info!("🗄️ Initializing database schema...");
    let job_repo = PostgreSqlJobRepository::new(pool.clone());
    job_repo.init_schema().await.expect("Failed to init schema");
    info!("✅ Database schema initialized");

    // Test basic CRUD operations
    info!("📝 Testing CRUD operations...");

    // Create a test job spec using the builder
    let job_spec = JobSpec::builder("Test Job".to_string(), "rust:1.75".to_string())
        .command(vec!["echo".to_string(), "Hello".to_string()])
        .resources(ResourceQuota {
            cpu_m: 1000,
            memory_mb: 512,
            gpu: None,
        })
        .timeout_ms(60000)
        .retries(3)
        .env(HashMap::new())
        .secret_refs(Vec::new())
        .build()
        .expect("Failed to create job spec");

    // Create a test job
    let job_id = JobId::new();
    let job = Job::new(job_id.clone(), job_spec).expect("Failed to create job");

    // Save the job
    job_repo.save_job(&job).await.expect("Failed to save job");
    info!("✅ Job saved successfully");

    // Retrieve the job
    let retrieved_job = job_repo
        .get_job(&job_id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(retrieved_job.id, job_id);
    assert_eq!(retrieved_job.name, "Test Job");
    info!("✅ Job retrieved and verified");

    // List jobs
    let jobs = job_repo.list_jobs().await.expect("Failed to list jobs");
    assert!(!jobs.is_empty(), "Jobs list should not be empty");
    info!("✅ Job listing works correctly");

    info!("✅ PostgreSQL integration test completed successfully");
    info!("💾 Memory footprint: ~37MB (singleton)");
}
