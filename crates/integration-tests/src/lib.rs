//! Integration Tests for Infrastructure Components
//!
//! Uses TestContainers to provide real PostgreSQL, NATS, and other
//! dependencies for comprehensive end-to-end testing.

pub mod postgres_integration;
pub mod nats_integration;
pub mod docker_provider_integration;

pub use postgres_integration::*;
pub use nats_integration::*;
pub use docker_provider_integration::*;

use std::sync::Arc;
use tokio::sync::Mutex;

/// Test environment manager
pub struct TestEnvironment {
    postgres_container: Option<Arc<Mutex<PostgresContainer>>>,
    nats_container: Option<Arc<Mutex<NatsContainer>>>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub fn new() -> Self {
        Self {
            postgres_container: None,
            nats_container: None,
        }
    }

    /// Initialize with PostgreSQL
    pub async fn with_postgres(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let container = PostgresContainer::new().await?;
        self.postgres_container = Some(Arc::new(Mutex::new(container)));
        Ok(self)
    }

    /// Initialize with NATS
    pub async fn with_nats(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let container = NatsContainer::new().await?;
        self.nats_container = Some(Arc::new(Mutex::new(container)));
        Ok(self)
    }

    /// Get PostgreSQL connection string
    pub async fn postgres_connection_string(&self) -> Option<String> {
        if let Some(container) = &self.postgres_container {
            let container = container.lock().await;
            Some(container.connection_string())
        } else {
            None
        }
    }

    /// Get NATS URL
    pub async fn nats_url(&self) -> Option<String> {
        if let Some(container) = &self.nats_container {
            let container = container.lock().await;
            Some(container.url())
        } else {
            None
        }
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
