//! Application Layer
//!
//! Orchestrates domain logic and coordinates between bounded contexts

pub mod job_service;
pub mod provider_service;

// Re-exports
pub use job_service::JobService;
pub use provider_service::ProviderService;
