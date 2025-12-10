//! Domain Layer - DDD Bounded Contexts
//!
//! Contains all domain logic organized in bounded contexts:
//! - shared_kernel: Common types and error handling
//! - job_execution: Job lifecycle management
//! - provider_management: Provider registration and capabilities
//! - execution_coordination: Orchestration between jobs and providers

pub mod execution_coordination;
pub mod job_execution;
pub mod provider_management;
pub mod shared_kernel;

// Re-exports for convenience
pub use shared_kernel::{DomainError, DomainResult};
