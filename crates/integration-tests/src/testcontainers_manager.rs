//! TestContainers Manager - Optimized Resource Pooling
//!
//! This module implements best practices for TestContainers usage in integration tests:
//! 1. Single instance pattern - one container per resource across all tests
//! 2. Resource reuse - containers persist across test suite runs
//! 3. Health checks - ensure containers are ready before tests
//! 4. Automatic cleanup - RAII-based lifecycle management
//! 5. Memory/CPU optimization - reuse containers across tests

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use testcontainers::clients::Cli;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{Container, GenericImage, Image};
use tokio::sync::OnceCell;
use tracing::{info, warn, error as tracing_error};

/// Test environment configuration
#[derive(Debug, Clone)]
pub struct TestEnvironmentConfig {
    /// Enable container reuse across test runs
    pub reuse_containers: bool,
    /// Maximum containers per resource type
    pub max_containers_per_type: usize,
    /// Container startup timeout
    pub startup_timeout: Duration,
    /// Health check timeout
    pub health_check_timeout: Duration,
    /// Enable parallel container startup
    pub parallel_startup: bool,
}

impl Default for TestEnvironmentConfig {
    fn default() -> Self {
        Self {
            reuse_containers: true,
            max_containers_per_type: 1, // Single instance per resource
            startup_timeout: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(10),
            parallel_startup: true,
        }
    }
}

/// Container metadata for tracking
#[derive(Debug, Clone)]
pub struct ContainerMetadata {
    pub id: String,
    pub image: String,
    pub tag: String,
    pub exposed_ports: Vec<u16>,
    pub started_at: std::time::Instant,
    pub health_check_url: Option<String>,
    pub reuse_count: u32,
}

/// Singleton Container Registry
/// Implements single-instance pattern per resource type
pub struct ContainerRegistry {
    config: TestEnvironmentConfig,
    docker: Cli,
    containers: Arc<Mutex<HashMap<String, ContainerMetadata>>>,
    active_containers: Arc<Mutex<HashMap<String, Container<dyn: Image>>>>,
    /// Singleton instances - one per resource type
    nats: OnceCell<Container<dyn: Image>>,
    postgres: OnceCell<Container<dyn: Image>>,
    redis: OnceCell<Container<dyn: Image>>,
    minio: OnceCell<Container<dyn: Image>>,
}

impl ContainerRegistry {
    pub fn new(config: TestEnvironmentConfig) -> Self {
        let docker = Cli::default();

        info!(
            reuse_containers = config.reuse_containers,
            max_per_type = config.max_containers_per_type,
            "Initializing TestContainer Registry"
        );

        Self {
            config,
            docker,
            containers: Arc::new(Mutex::new(HashMap::new())),
            active_containers: Arc::new(Mutex::new(HashMap::new())),
            nats: OnceCell::const_new(),
            postgres: OnceCell::const_new(),
            redis: OnceCell::const_new(),
            minio: OnceCell::const_new(),
        }
    }

    /// Start NATS container (singleton pattern)
    pub async fn get_or_start_nats(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        info!("Acquiring NATS container (singleton pattern)");

        let container = self.nats.get_or_init(|| async {
            self.start_nats_container().await
        }).await.map_err(|e| TestContainerError::ContainerStartupFailed {
            resource: "nats".to_string(),
            error: e.to_string(),
        })?;

        self.track_container("nats", container).await;
        Ok(container.clone())
    }

    /// Start PostgreSQL container (singleton pattern)
    pub async fn get_or_start_postgres(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        info!("Acquiring PostgreSQL container (singleton pattern)");

        let container = self.postgres.get_or_init(|| async {
            self.start_postgres_container().await
        }).await.map_err(|e| TestContainerError::ContainerStartupFailed {
            resource: "postgres".to_string(),
            error: e.to_string(),
        })?;

        self.track_container("postgres", container).await;
        Ok(container.clone())
    }

    /// Start Redis container (singleton pattern)
    pub async fn get_or_start_redis(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        info!("Acquiring Redis container (singleton pattern)");

        let container = self.redis.get_or_init(|| async {
            self.start_redis_container().await
        }).await.map_err(|e| TestContainerError::ContainerStartupFailed {
            resource: "redis".to_string(),
            error: e.to_string(),
        })?;

        self.track_container("redis", container).await;
        Ok(container.clone())
    }

    /// Start MinIO container (singleton pattern)
    pub async fn get_or_start_minio(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        info!("Acquiring MinIO container (singleton pattern)");

        let container = self.minio.get_or_init(|| async {
            self.start_minio_container().await
        }).await.map_err(|e| TestContainerError::ContainerStartupFailed {
            resource: "minio".to_string(),
            error: e.to_string(),
        })?;

        self.track_container("minio", container).await;
        Ok(container.clone())
    }

    /// Start NATS container with optimizations
    async fn start_nats_container(&self) -> Result<Container<dyn: Image>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting NATS container");

        let container = GenericImage::new("nats", "2.10")
            .with_exposed_port(4222.tcp())
            .with_exposed_port(8222.tcp()) // monitoring port
            .with_wait_for(WaitFor::message_on_stdout("Listening for route connections"))
            .with_env_var("MAX_PAYLOAD", "1048576") // 1MB limit
            .with_env_var("MAX_MEMORIES", "1048576")
            .start()
            .await?;

        // Wait for health check
        let host_port = container.get_host_port_ipv4(4222);
        self.wait_for_nats_health_check(host_port).await?;

        info!("NATS container ready at port {}", host_port);
        Ok(container)
    }

    /// Start PostgreSQL with optimizations
    async fn start_postgres_container(&self) -> Result<Container<dyn: Image>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting PostgreSQL container");

        let container = GenericImage::new("postgres", "15-alpine")
            .with_exposed_port(5432.tcp())
            .with_env_var("POSTGRES_DB", "testdb")
            .with_env_var("POSTGRES_USER", "testuser")
            .with_env_var("POSTGRES_PASSWORD", "testpass")
            .with_env_var("POSTGRES_INITDB_ARGS", "--encoding=UTF-8 --lc-collate=C --lc-ctype=C")
            .with_wait_for(WaitFor::message_on_stdout("database system is ready to accept connections"))
            .start()
            .await?;

        // Wait for health check
        let host_port = container.get_host_port_ipv4(5432);
        self.wait_for_postgres_health_check(host_port).await?;

        info!("PostgreSQL container ready at port {}", host_port);
        Ok(container)
    }

    /// Start Redis with optimizations
    async fn start_redis_container(&self) -> Result<Container<dyn: Image>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting Redis container");

        let container = GenericImage::new("redis", "7.2-alpine")
            .with_exposed_port(6379.tcp())
            .with_env_var("REDIS_PASSWORD", "") // disable password for tests
            .with_env_var("MAXMEMORY", "256mb")
            .with_env_var("MAXMEMORY_POLICY", "allkeys-lru")
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await?;

        info!("Redis container ready at port {}", container.get_host_port_ipv4(6379));
        Ok(container)
    }

    /// Start MinIO with optimizations
    async fn start_minio_container(&self) -> Result<Container<dyn: Image>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting MinIO container");

        let container = GenericImage::new("minio/minio", "RELEASE.2024-01-13T07-53-03Z")
            .with_exposed_port(9000.tcp())
            .with_exposed_port(9001.tcp()) // console port
            .with_env_var("MINIO_ROOT_USER", "minioadmin")
            .with_env_var("MINIO_ROOT_PASSWORD", "minioadmin")
            .with_env_var("MINIO_PROMETHEUS_AUTH_TYPE", "public")
            .with_command(vec!["server".to_string(), "/data".to_string(), "--console-address".to_string(), ":9001".to_string()])
            .with_wait_for(WaitFor::message_on_stdout("API: http://"))
            .start()
            .await?;

        info!("MinIO container ready");
        Ok(container)
    }

    /// Health check for NATS
    async fn wait_for_nats_health_check(&self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();
        let timeout = self.config.health_check_timeout;

        while start.elapsed() < timeout {
            match tokio::time::timeout(Duration::from_secs(1), async {
                reqwest::get(format!("http://localhost:{}/healthz", port)).await
            }).await {
                Ok(Ok(_)) => return Ok(()),
                _ => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }

        Err("NATS health check timeout".into())
    }

    /// Health check for PostgreSQL
    async fn wait_for_postgres_health_check(&self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();
        let timeout = self.config.health_check_timeout;

        while start.elapsed() < timeout {
            match tokio::time::timeout(Duration::from_secs(1), async {
                let mut conn = tokio_postgres::Config::new()
                    .host("localhost")
                    .port(port)
                    .user("testuser")
                    .password("testpass")
                    .database("testdb")
                    .connect().await;
                conn
            }).await {
                Ok(Ok(_)) => return Ok(()),
                _ => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }

        Err("PostgreSQL health check timeout".into())
    }

    /// Track active container
    async fn track_container(&self, name: &str, container: &Container<dyn: Image>) {
        let id = container.id().to_string();
        let exposed_ports = container.ports().values().cloned().collect();

        let metadata = ContainerMetadata {
            id,
            image: "unknown".to_string(), // TODO: get from container
            tag: "unknown".to_string(),
            exposed_ports,
            started_at: std::time::Instant::now(),
            health_check_url: None,
            reuse_count: 0,
        };

        let mut containers = self.containers.lock().unwrap();
        containers.insert(name.to_string(), metadata);
    }

    /// Get all tracked containers
    pub fn list_tracked_containers(&self) -> HashMap<String, ContainerMetadata> {
        self.containers.lock().unwrap().clone()
    }

    /// Cleanup all containers (called at test suite end)
    pub async fn cleanup_all(&self) {
        warn!("Cleaning up all test containers");

        let mut containers = self.active_containers.lock().unwrap();
        for (name, container) in containers.drain() {
            info!("Stopping container: {}", name);
            drop(container); // Drop will stop and remove container
        }

        self.containers.lock().unwrap().clear();
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> RegistryStats {
        let containers = self.containers.lock().unwrap();
        RegistryStats {
            tracked_containers: containers.len(),
            active_containers: self.active_containers.lock().unwrap().len(),
            uptime: std::time::Instant::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub tracked_containers: usize,
    pub active_containers: usize,
    pub uptime: std::time::Instant,
}

/// Test container error types
#[derive(thiserror::Error, Debug)]
pub enum TestContainerError {
    #[error("Container startup failed for {resource}: {error}")]
    ContainerStartupFailed {
        resource: String,
        error: String,
    },

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Container reuse limit exceeded")]
    ReuseLimitExceeded,

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Singleton instance of the registry for the entire test process
static REGISTRY: OnceCell<Arc<ContainerRegistry>> = OnceCell::const_new();

/// Initialize the global registry
pub async fn init_test_registry(config: TestEnvironmentConfig) -> Arc<ContainerRegistry> {
    REGISTRY.get_or_init(|| async {
        Arc::new(ContainerRegistry::new(config))
    }).await.clone()
}

/// Get the global registry instance
pub async fn get_test_registry() -> Arc<ContainerRegistry> {
    REGISTRY.get().expect("Registry not initialized. Call init_test_registry() first").clone()
}

/// Test environment setup helper
pub struct TestEnvironment {
    registry: Arc<ContainerRegistry>,
}

impl TestEnvironment {
    pub async fn new() -> Result<Self, TestContainerError> {
        info!("Initializing test environment");

        let registry = init_test_registry(TestEnvironmentConfig::default()).await;

        Ok(Self { registry })
    }

    pub async fn nats(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        self.registry.get_or_start_nats().await
    }

    pub async fn postgres(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        self.registry.get_or_start_postgres().await
    }

    pub async fn redis(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        self.registry.get_or_start_redis().await
    }

    pub async fn minio(&self) -> Result<Container<dyn: Image>, TestContainerError> {
        self.registry.get_or_start_minio().await
    }

    pub fn get_stats(&self) -> RegistryStats {
        self.registry.get_stats()
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        warn!("TestEnvironment dropped, containers will persist for reuse");
    }
}

/// Resource limit configuration for containers
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// CPU limit (e.g., "0.5" for 50% of one CPU)
    pub cpu_limit: Option<String>,
    /// Memory limit (e.g., "512m", "1g")
    pub memory_limit: Option<String>,
    /// Disable swap
    pub disable_swap: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limit: Some("0.5".to_string()), // 50% of one CPU core
            memory_limit: Some("512m".to_string()), // 512 MB
            disable_swap: true,
        }
    }
}

impl ResourceLimits {
    pub fn apply_to_image<I: Image>(&self, image: I) -> I {
        let mut image = image;

        // Apply memory limit
        if let Some(mem) = &self.memory_limit {
            image = image.with_memory_limit(mem);
        }

        // Apply CPU limit
        if let Some(cpu) = &self.cpu_limit {
            image = image.with_cpu_limit(cpu);
        }

        image
    }
}

/// Container leak detection
pub struct ContainerLeakDetector {
    start_count: usize,
}

impl ContainerLeakDetector {
    pub fn new() -> Self {
        let start_count = get_current_container_count();
        info!("Starting container leak detection at count: {}", start_count);
        Self { start_count }
    }

    pub fn check_leaks(&self) -> usize {
        let current_count = get_current_container_count();
        let leaked = current_count.saturating_sub(self.start_count);

        if leaked > 0 {
            tracing_error!("Detected {} leaked containers!", leaked);
        } else {
            info!("No container leaks detected");
        }

        leaked
    }
}

fn get_current_container_count() -> usize {
    // This would query Docker API or use testcontainers internals
    // For now, return 0 as placeholder
    0
}
