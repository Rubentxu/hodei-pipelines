/// Basic example of using the Hodei Rust SDK
use hodei_rust_sdk::{CicdClient, PipelineBuilder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create the CICD client
    let client = CicdClient::new(
        "http://localhost:8080",
        "your-api-token",
    )?;

    // Build a pipeline configuration
    let pipeline_config = PipelineBuilder::new("rust-build-pipeline")
        .description("Build and test Rust project")
        .env("RUST_LOG", "info")
        .env("CARGO_TERM_COLOR", "always")
        .stage(
            "checkout",
            "alpine/git:latest",
            vec![
                "git clone https://github.com/user/repo.git /workspace",
                "cd /workspace",
            ],
        )
        .stage_with_resources(
            "build",
            "rust:1.70",
            vec![
                "cargo build --release",
            ],
            2.0,  // 2 CPU cores
            2048, // 2GB RAM
        )
        .stage_with_deps(
            "test",
            "rust:1.70",
            vec!["cargo test"],
            vec!["build"], // Depends on build stage
        )
        .stage_with_deps(
            "deploy",
            "alpine:latest",
            vec!["./scripts/deploy.sh"],
            vec!["test"], // Depends on test stage
        )
        .trigger_on_push("main", "https://github.com/user/repo")
        .trigger_on_schedule("0 2 * * *") // Daily at 2 AM
        .build();

    // Create the pipeline
    println!("Creating pipeline...");
    let pipeline = client.create_pipeline(pipeline_config).await?;
    println!("✓ Pipeline created: {} (ID: {})", pipeline.name, pipeline.id);

    // Execute the pipeline
    println!("\nExecuting pipeline...");
    let job = client.execute_pipeline(&pipeline.id).await?;
    println!("✓ Job started: {} (Status: {:?})", job.id, job.status);

    // Wait for completion
    println!("\nWaiting for job completion...");
    let completed_job = client
        .wait_for_completion(&job.id, Duration::from_secs(5))
        .await?;

    println!("\n✓ Job completed!");
    println!("  Status: {:?}", completed_job.status);
    println!("  Duration: {:?}",
        completed_job.finished_at.unwrap().signed_duration_since(completed_job.started_at)
    );

    // Get job logs
    if let Ok(logs) = client.get_job_logs(&job.id).await {
        println!("\nJob logs:");
        println!("{}", logs);
    }

    Ok(())
}
