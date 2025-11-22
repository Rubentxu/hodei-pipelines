/// Hodei SDK Core - Core framework for Hodei CI/CD platform SDK
///
/// This crate provides the core abstractions and utilities for building
/// language-specific SDKs for the Hodei CI/CD platform.

pub mod client;
pub mod error;
pub mod types;

// Re-export commonly used types
pub use client::{ClientConfig, HttpClient};
pub use error::{SdkError, SdkResult};
pub use types::{
    Job, JobStatus, Pipeline, PipelineConfig, PipelineStatus, ResourceRequirements,
    StageConfig, TriggerConfig, Worker, WorkerStatus,
};
