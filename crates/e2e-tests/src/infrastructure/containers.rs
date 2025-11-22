//! Container orchestration for E2E tests
//!
//! This module provides Docker container management using Testcontainers
//! for infrastructure services like NATS, PostgreSQL, Prometheus, and Jaeger.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use testcontainers::clients::Cli;
use testcontainers::core::{Container, ExecCommand, WaitFor};
// Use generic Docker images for now
use testcontainers::GenericImage;

/// Container manager for E2E tests
#[derive(Clone)]
pub struct ContainerManager {
    docker: Arc<RwLock<Cli>>,
    containers: Arc<RwLock<HashMap<String, ContainerHandle>>>,
}

/// Handle to a managed container
#[derive(Clone)]
pub struct ContainerHandle {
    pub name: String,
    pub container: Container<dyn testcontainers::core::Image>,
    pub ports: HashMap<String, u16>,
    pub labels: HashMap<String, String>,
}

impl ContainerManager {
    /// Create a new container manager
    pub fn new() -> Self {
        Self {
            docker: Arc::new(RwLock::new(Cli::default())),
            containers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start NATS JetStream container
    pub async fn start_nats(
        &self,
    ) -> Result<ContainerHandle, Box<dyn std::error::Error + Send + Sync>> {
        info!("🚀 Starting NATS container...");

        let mut ports = HashMap::new();
        ports.insert("4222".to_string(), 4222);
        ports.insert("8222".to_string(), 8222);

        let nats_image = GenericImage::new("nats", "2.10-alpine")
            .with_exposed_port(testcontainers::core::ContainerPort::Tcp(4222))
            .with_exposed_port(testcontainers::core::ContainerPort::Tcp(8222));

        let container = self.docker.write().await.run(nats_image);

        info!("✅ NATS container started successfully");

        let handle = ContainerHandle {
            name: "nats".to_string(),
            container,
            ports,
            labels: HashMap::from([
                ("service".to_string(), "nats".to_string()),
                ("type".to_string(), "message-broker".to_string()),
            ]),
        };

        self.containers
            .write()
            .await
            .insert("nats".to_string(), handle.clone());

        Ok(handle)
    }

    /// Start PostgreSQL container
    pub async fn start_postgres(
        &self,
    ) -> Result<ContainerHandle, Box<dyn std::error::Error + Send + Sync>> {
        info!("🚀 Starting PostgreSQL container...");

        let mut ports = HashMap::new();
        ports.insert("5432".to_string(), 5432);

        let postgres_image = GenericImage::new("postgres", "15-alpine")
            .with_exposed_port(testcontainers::core::ContainerPort::Tcp(5432));

        let container = self.docker.write().await.run(postgres_image);

        let connection_string = format!(
            "postgresql://postgres:postgres@localhost:{}/postgres",
            container.get_host_port_ipv4(5432)
        );

        info!("✅ PostgreSQL container started: {}", connection_string);

        let handle = ContainerHandle {
            name: "postgres".to_string(),
            container,
            ports,
            labels: HashMap::from([
                ("service".to_string(), "postgres".to_string()),
                ("type".to_string(), "database".to_string()),
            ]),
        };

        self.containers
            .write()
            .await
            .insert("postgres".to_string(), handle.clone());

        Ok(handle)
    }

    /// Start Prometheus container
    pub async fn start_prometheus(
        &self,
    ) -> Result<ContainerHandle, Box<dyn std::error::Error + Send + Sync>> {
        info!("🚀 Starting Prometheus container...");

        let mut ports = HashMap::new();
        ports.insert("9090".to_string(), 9090);

        let prometheus_image = GenericImage::new("prom/prometheus", "latest")
            .with_exposed_port(testcontainers::core::ContainerPort::Tcp(9090));

        let container = self.docker.write().await.run(prometheus_image);

        info!("✅ Prometheus container started");

        let handle = ContainerHandle {
            name: "prometheus".to_string(),
            container,
            ports,
            labels: HashMap::from([
                ("service".to_string(), "prometheus".to_string()),
                ("type".to_string(), "metrics".to_string()),
            ]),
        };

        self.containers
            .write()
            .await
            .insert("prometheus".to_string(), handle.clone());

        Ok(handle)
    }

    /// Start Jaeger container (for distributed tracing)
    pub async fn start_jaeger(
        &self,
    ) -> Result<ContainerHandle, Box<dyn std::error::Error + Send + Sync>> {
        info!("🚀 Starting Jaeger container...");

        let mut ports = HashMap::new();
        ports.insert("16686".to_string(), 16686);
        ports.insert("14268".to_string(), 14268);

        let docker = self.docker.read().await;
        let image = testcontainers::GenericImage::new("jaegertracing/all-in-one", "1.54")
            .with_exposed_port(testcontainers::core::ContainerPort::Tcp(16686))
            .with_exposed_port(testcontainers::core::ContainerPort::Tcp(14268));

        let container = docker.run(image);

        info!("✅ Jaeger container started");

        let handle = ContainerHandle {
            name: "jaeger".to_string(),
            container,
            ports,
            labels: HashMap::from([
                ("service".to_string(), "jaeger".to_string()),
                ("type".to_string(), "tracing".to_string()),
            ]),
        };

        self.containers
            .write()
            .await
            .insert("jaeger".to_string(), handle.clone());

        Ok(handle)
    }

    /// Get a container by name
    pub async fn get(&self, name: &str) -> Option<ContainerHandle> {
        let containers = self.containers.read().await;
        containers.get(name).cloned()
    }

    /// Execute a command in a container
    pub async fn exec(
        &self,
        name: &str,
        command: Vec<&str>,
    ) -> Result<std::process::Output, Box<dyn std::error::Error + Send + Sync>> {
        let containers = self.containers.read().await;

        if let Some(handle) = containers.get(name) {
            let cmd = ExecCommand::new(command);
            Ok(handle.container.exec(cmd))
        } else {
            Err(format!("Container '{}' not found", name).into())
        }
    }

    /// Get container logs
    pub async fn logs(
        &self,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let containers = self.containers.read().await;

        if let Some(handle) = containers.get(name) {
            let logs = handle.container.logs().await;
            Ok(logs)
        } else {
            Err(format!("Container '{}' not found", name).into())
        }
    }

    /// Stop all containers
    pub async fn stop_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🛑 Stopping all containers...");

        let mut containers = self.containers.write().await;
        let container_names: Vec<String> = containers.keys().cloned().collect();

        for name in container_names {
            if let Some(handle) = containers.remove(&name) {
                info!("Stopping container: {}", handle.name);
                drop(handle);
            }
        }

        info!("✅ All containers stopped");

        Ok(())
    }

    /// Get all running containers
    pub async fn list(&self) -> Vec<ContainerHandle> {
        let containers = self.containers.read().await;
        containers.values().cloned().collect()
    }
}

impl Default for ContainerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for starting infrastructure services
pub struct InfrastructureBuilder {
    container_manager: ContainerManager,
    start_nats: bool,
    start_postgres: bool,
    start_prometheus: bool,
    start_jaeger: bool,
}

impl InfrastructureBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            container_manager: ContainerManager::new(),
            start_nats: false,
            start_postgres: false,
            start_prometheus: false,
            start_jaeger: false,
        }
    }

    /// Enable NATS
    pub fn with_nats(mut self) -> Self {
        self.start_nats = true;
        self
    }

    /// Enable PostgreSQL
    pub fn with_postgres(mut self) -> Self {
        self.start_postgres = true;
        self
    }

    /// Enable Prometheus
    pub fn with_prometheus(mut self) -> Self {
        self.start_prometheus = true;
        self
    }

    /// Enable Jaeger
    pub fn with_jaeger(mut self) -> Self {
        self.start_jaeger = true;
        self
    }

    /// Start all infrastructure services
    pub async fn start(self) -> Result<ContainerManager, Box<dyn std::error::Error + Send + Sync>> {
        info!("🏗️  Starting infrastructure services...");

        // Start required services
        let mut started = Vec::new();

        if self.start_nats {
            match self.container_manager.start_nats().await {
                Ok(handle) => {
                    info!("✅ NATS started");
                    started.push(handle);
                }
                Err(e) => {
                    error!("❌ Failed to start NATS: {}", e);
                    return Err(e);
                }
            }
        }

        if self.start_postgres {
            match self.container_manager.start_postgres().await {
                Ok(handle) => {
                    info!("✅ PostgreSQL started");
                    started.push(handle);
                }
                Err(e) => {
                    error!("❌ Failed to start PostgreSQL: {}", e);
                    return Err(e);
                }
            }
        }

        if self.start_prometheus {
            match self.container_manager.start_prometheus().await {
                Ok(handle) => {
                    info!("✅ Prometheus started");
                    started.push(handle);
                }
                Err(e) => {
                    error!("❌ Failed to start Prometheus: {}", e);
                    return Err(e);
                }
            }
        }

        if self.start_jaeger {
            match self.container_manager.start_jaeger().await {
                Ok(handle) => {
                    info!("✅ Jaeger started");
                    started.push(handle);
                }
                Err(e) => {
                    error!("❌ Failed to start Jaeger: {}", e);
                    return Err(e);
                }
            }
        }

        info!(
            "✅ All infrastructure services started ({} services)",
            started.len()
        );

        Ok(self.container_manager)
    }
}

impl Default for InfrastructureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to wait for a service to be ready
pub async fn wait_for_service(
    url: &str,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("⏳ Waiting for service at {} to be ready...", url);

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        match reqwest::get(url).await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("✅ Service is ready!");
                    return Ok(());
                }
            }
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }

    Err(format!("Timeout waiting for service at {}", url).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_container_manager_creation() {
        let manager = ContainerManager::new();
        assert!(manager.list().await.is_empty());
    }

    #[tokio::test]
    async fn test_infrastructure_builder() {
        let builder = InfrastructureBuilder::new().with_nats().with_postgres();
        assert!(builder.start_nats);
        assert!(builder.start_postgres);
        assert!(!builder.start_prometheus);
    }
}
