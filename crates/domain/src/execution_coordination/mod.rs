//! Execution Coordination Bounded Context
//!
//! Orchestrates execution between jobs and providers
//! - Coordinates job submission to providers
//! - Tracks execution status across providers

pub mod services;

// Re-exports
pub use services::ExecutionCoordinator;
