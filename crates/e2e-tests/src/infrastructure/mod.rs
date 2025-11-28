//! Infrastructure layer for E2E tests

pub mod config;
pub mod containers;
pub mod observability;
pub mod services;

pub use config::TestConfig;
pub use containers::{ContainerManager, InfrastructureBuilder};
pub use services::{OrchestratorClient, SchedulerClient, WorkerManagerClient};

/// Simple test environment
#[derive(Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_environment_new() {
        let env = TestEnvironment::new().await.unwrap();
        assert_eq!(env.config.orchestrator_port, 8080);
        assert_eq!(env.config.scheduler_port, 8081);
    }

    #[test]
    fn test_test_environment_clone() {
        let env = TestEnvironment {
            config: TestConfig::default(),
        };
        let _cloned = env.clone();
    }

    #[test]
    fn test_test_environment_default() {
        let env = TestEnvironment {
            config: TestConfig::default(),
        };
        assert_eq!(env.config.timeout_secs, 300);
        assert_eq!(env.config.max_retries, 3);
    }
}
