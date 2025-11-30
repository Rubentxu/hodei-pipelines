//! Basic PostgreSQL Integration Test with TestContainers
//!
//! This test validates PostgreSQL connectivity and basic operations
//! using TestContainers to spin up a real PostgreSQL instance.

use std::collections::HashMap;
use testcontainers::{GenericImage, ImageExt, core::ContainerPort, runners::AsyncRunner};
use tracing::info;

use hodei_pipelines_adapters::postgres::PostgreSqlJobRepository;
use hodei_pipelines_core::job::{Job, JobId, JobSpec, ResourceQuota};
use hodei_pipelines_ports::JobRepository;

#[tokio::test]
async fn test_postgres_connection_basic() {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting PostgreSQL integration test with TestContainers");

    // Start PostgreSQL container using TestContainers
    info!("📦 Starting PostgreSQL container...");

    let node = GenericImage::new("postgres", "16-alpine")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres")
        .with_mapped_port(5432, ContainerPort::Tcp(5432))
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let port = node
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get PostgreSQL port");

    info!("✅ PostgreSQL container started on port {}", port);

    // Build connection string
    let connection_string = format!("postgresql://postgres:postgres@127.0.0.1:{}/postgres", port);

    info!("📡 Connecting to PostgreSQL at {}", connection_string);

    // Wait for PostgreSQL to be ready
    let mut attempts = 0;
    let max_attempts = 30;
    loop {
        match sqlx::PgPool::connect(&connection_string).await {
            Ok(_) => {
                info!("✅ PostgreSQL connection established");
                break;
            }
            Err(_e) => {
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

    // Connect to PostgreSQL
    let pool = sqlx::PgPool::connect(&connection_string)
        .await
        .expect("Failed to connect to PostgreSQL");

    info!("✅ PostgreSQL ready for queries");

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
    info!("🔌 Container will be automatically cleaned up by TestContainers");
}
