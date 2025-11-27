//! E2E Testing Framework for Hodei Pipelines Platform

//! Test result type
pub type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
