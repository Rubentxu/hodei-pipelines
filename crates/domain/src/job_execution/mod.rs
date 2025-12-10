//! Job Execution Bounded Context
//!
//! Manages the lifecycle of jobs from creation to completion
//! - Job aggregate root
//! - Job specification and execution context value objects
//! - Job repository port
//! - Job services and use cases

pub mod entities;
pub mod repositories;
pub mod services;
pub mod use_cases;
pub mod value_objects;

// Re-exports
pub use entities::Job;
pub use repositories::JobRepository;
pub use services::JobScheduler;
pub use use_cases::{CreateJobUseCase, ExecuteJobUseCase, GetJobResultUseCase};
pub use value_objects::{ExecutionContext, JobSpec};
