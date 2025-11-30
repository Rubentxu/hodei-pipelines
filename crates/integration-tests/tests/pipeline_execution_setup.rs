/// One-time test setup for pipeline integration tests
/// This file should be compiled but not run directly

use hodei_pipelines_adapters::postgres::{PostgreSqlJobRepository, PostgreSqlPipelineRepository};
use sqlx::postgres::PgPoolOptions;
use sqlx::Pool;
use std::time::Duration;
use testcontainers::{GenericImage, ImageExt, core::ContainerPort, runners::AsyncRunner};
use tracing::info;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("🗄️ Setting up test database...");
    
    // Start PostgreSQL container
    let postgres_node = GenericImage::new("postgres", "16-alpine")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_DB", "postgres")
        .with_mapped_port(5432, ContainerPort::Tcp(5432))
        .start()
        .await?;
    
    let pg_port = postgres_node
        .get_host_port_ipv4(5432)
        .await?;
    
    let connection_string = format!(
        "postgresql://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );
    
    info!("✅ PostgreSQL container started on port {}", pg_port);
    
    // Connect with large pool
    let pool = PgPoolOptions::new()
        .max_connections(1000)
        .acquire_timeout(Duration::from_secs(60))
        .connect(&connection_string)
        .await?;
    
    info!("✅ Connected to PostgreSQL");
    
    // Initialize schema
    let pipeline_repo = PostgreSqlPipelineRepository::new(pool.clone());
    let job_repo = PostgreSqlJobRepository::new(pool.clone());
    
    pipeline_repo.init_schema().await?;
    job_repo.init_schema().await?;
    
    info!("✅ Schema initialized successfully");
    info!("✅ Setup complete!");
    
    Ok(())
}
