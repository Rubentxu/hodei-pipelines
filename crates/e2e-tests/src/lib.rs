//! E2E Testing Framework for Hodei Pipelines Platform

pub mod helpers;
pub mod infrastructure;

pub use infrastructure::TestConfig;

/// Test result type
pub type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
