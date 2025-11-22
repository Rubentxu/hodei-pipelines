/// Hodei Rust SDK - Idiomatic Rust SDK for Hodei CI/CD platform
///
/// This crate provides a comprehensive Rust SDK for interacting with the
/// Hodei CI/CD platform, with async/await support and type-safe APIs.
///
/// # Example
/// ```no_run
/// use hodei_rust_sdk::{CicdClient, PipelineBuilder};
/// use std::collections::HashMap;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create client
///     let client = CicdClient::new("https://api.hodei.example.com", "your-token")?;
///
///     // Create pipeline using builder
///     let pipeline = PipelineBuilder::new("my-pipeline")
///         .description("My first pipeline")
///         .stage("build", "rust:1.70", vec!["cargo build"])
///         .stage("test", "rust:1.70", vec!["cargo test"])
///         .build();
///
///     // Create and execute pipeline
///     let created_pipeline = client.create_pipeline(pipeline).await?;
///     let job = client.execute_pipeline(&created_pipeline.id).await?;
///
///     println!("Job started: {}", job.id);
///     Ok(())
/// }
/// ```

pub mod builder;
pub mod client;

// Re-export core types
pub use hodei_sdk_core::{
    Job, JobStatus, Pipeline, PipelineConfig, PipelineStatus, ResourceRequirements,
    SdkError, SdkResult, StageConfig, TriggerConfig, Worker, WorkerStatus,
};

// Re-export client
pub use client::CicdClient;

// Re-export builder
pub use builder::PipelineBuilder;
