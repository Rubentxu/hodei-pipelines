//! Infrastructure layer for E2E tests

pub mod config;

pub use config::TestConfig;

/// Simple test environment
pub struct TestEnvironment {
    pub config: TestConfig,
}

impl TestEnvironment {
    /// Create a new test environment
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config = TestConfig::from_env().unwrap_or_default();
        Ok(Self { config })
    }
}
