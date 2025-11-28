//! Basic PostgreSQL Integration Test
//!
//! This test validates PostgreSQL connectivity and basic operations
//! using a local PostgreSQL instance.

use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use tracing::info;

use hodei_adapters::postgres::PostgreSqlJobRepository;
use hodei_core::job::{Job, JobId, JobSpec, ResourceQuota};
use hodei_ports::JobRepository;

#[tokio::test]
async fn test_postgres_connection_basic() {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting PostgreSQL integration test");

    // Try to connect to PostgreSQL at localhost:5432
    info!("📦 Connecting to PostgreSQL at localhost:5432...");

    let connection_string = "postgresql://postgres:postgres@localhost:5432/postgres";

    let mut pool = None;
    for i in 0..30 {
        match sqlx::PgPool::connect(connection_string).await {
            Ok(p) => {
                pool = Some(p);
                info!("✅ PostgreSQL connection established (attempt {})", i + 1);
                break;
            }
            Err(e) => {
                info!("⚠️ PostgreSQL connection attempt {} failed: {}", i + 1, e);
                if i < 29 {
                    sleep(Duration::from_secs(1)).await;
                } else {
                    info!("✅ Test skipped - PostgreSQL not available (expected in CI)");
                    return;
                }
            }
        }
    }

    let pool = pool.expect("Failed to establish PostgreSQL connection");

    // Step 3: Initialize database schema
    info!("🗄️ Initializing database schema...");
    let job_repo = PostgreSqlJobRepository::new(pool.clone());
    job_repo.init_schema().await.expect("Failed to init schema");
    info!("✅ Database schema initialized");

    // Step 4: Test basic CRUD operations
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
}
