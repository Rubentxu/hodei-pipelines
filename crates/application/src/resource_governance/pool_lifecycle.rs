//! Resource Pool Lifecycle Manager Module
//!
//! This module provides complete lifecycle management for resource pools including
//! creation, configuration, monitoring, scaling, and destruction.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use hodei_pipelines_domain::{Result, WorkerId};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Lifecycle state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolLifecycleState {
    Creating,
    Initializing,
    Active,
    Scaling,
    Draining,
    Destroying,
    Destroyed,
    Error(String),
}

impl std::fmt::Display for PoolLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolLifecycleState::Creating => write!(f, "Creating"),
            PoolLifecycleState::Initializing => write!(f, "Initializing"),
            PoolLifecycleState::Active => write!(f, "Active"),
            PoolLifecycleState::Scaling => write!(f, "Scaling"),
            PoolLifecycleState::Draining => write!(f, "Draining"),
            PoolLifecycleState::Destroying => write!(f, "Destroying"),
            PoolLifecycleState::Destroyed => write!(f, "Destroyed"),
            PoolLifecycleState::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// Pool state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub pool_id: String,
    pub status: PoolLifecycleState,
    pub config: PoolConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub worker_count: u32,
    pub healthy_workers: u32,
    pub last_health_check: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

/// Pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub pool_id: String,
    pub name: String,
    pub provider_type: String, // docker, kubernetes, static
    pub initial_size: u32,
    pub min_size: u32,
    pub max_size: u32,
    pub pre_warm: bool,
    pub health_check_interval: Duration,
    pub metadata: HashMap<String, String>,
    pub scaling_enabled: bool,
}

impl FromStr for PoolConfig {
    type Err = hodei_pipelines_domain::DomainError;

    fn from_str(s: &str) -> Result<Self> {
        // Simple default config for health checks
        Ok(PoolConfig {
            pool_id: s.to_string(),
            name: s.to_string(),
            provider_type: "mock".to_string(),
            initial_size: 0,
            min_size: 0,
            max_size: 0,
            pre_warm: false,
            health_check_interval: Duration::from_secs(30),
            metadata: HashMap::new(),
            scaling_enabled: false,
        })
    }
}

/// Pool event types
#[derive(Debug, Clone)]
pub enum PoolEvent {
    Created {
        pool_id: String,
        config: PoolConfig,
        timestamp: DateTime<Utc>,
    },
    Initialized {
        pool_id: String,
        worker_count: u32,
        timestamp: DateTime<Utc>,
    },
    ConfigUpdated {
        pool_id: String,
        old_config: PoolConfig,
        new_config: PoolConfig,
        timestamp: DateTime<Utc>,
    },
    Scaling {
        pool_id: String,
        from_size: u32,
        to_size: u32,
        timestamp: DateTime<Utc>,
    },
    HealthCheck {
        pool_id: String,
        healthy_workers: u32,
        total_workers: u32,
        timestamp: DateTime<Utc>,
    },
    Draining {
        pool_id: String,
        remaining_jobs: u32,
        timestamp: DateTime<Utc>,
    },
    Destroyed {
        pool_id: String,
        timestamp: DateTime<Utc>,
    },
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub pool_id: String,
    pub healthy: bool,
    pub worker_count: u32,
    pub healthy_workers: u32,
    pub last_check: DateTime<Utc>,
    pub errors: Vec<String>,
}

/// Lifecycle errors
#[derive(Error, Debug)]
pub enum LifecycleError {
    #[error("Pool not found: {0}")]
    PoolNotFound(String),

    #[error("Invalid pool configuration: {0}")]
    InvalidConfig(String),

    #[error("Pool is in incompatible state: {0}")]
    InvalidState(PoolLifecycleState),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Operation timeout: {0}")]
    Timeout(String),

    #[error("State store error: {0}")]
    StateStoreError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Simple in-memory state store
#[derive(Debug, Default, Clone)]
pub struct InMemoryStateStore {
    pools: Arc<RwLock<HashMap<String, PoolState>>>,
}

impl InMemoryStateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn save_pool_state(&self, pool_id: &str, state: PoolState) -> Result<()> {
        let mut pools = self.pools.write().await;
        pools.insert(pool_id.to_string(), state);
        Ok(())
    }

    pub async fn get_pool_state(&self, pool_id: &str) -> Result<PoolState> {
        let pools = self.pools.read().await;
        pools
            .get(pool_id)
            .cloned()
            .ok_or_else(|| LifecycleError::PoolNotFound(pool_id.to_string()).into())
    }

    pub async fn update_pool_config(&self, pool_id: &str, config: &PoolConfig) -> Result<()> {
        let mut pools = self.pools.write().await;
        if let Some(state) = pools.get_mut(pool_id) {
            state.config = config.clone();
            state.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LifecycleError::PoolNotFound(pool_id.to_string()).into())
        }
    }

    pub async fn update_pool_status(
        &self,
        pool_id: &str,
        status: PoolLifecycleState,
    ) -> Result<()> {
        let mut pools = self.pools.write().await;
        if let Some(state) = pools.get_mut(pool_id) {
            state.status = status;
            state.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LifecycleError::PoolNotFound(pool_id.to_string()).into())
        }
    }

    pub async fn remove_pool(&self, pool_id: &str) -> Result<()> {
        let mut pools = self.pools.write().await;
        pools
            .remove(pool_id)
            .ok_or_else(|| LifecycleError::PoolNotFound(pool_id.to_string()))?;
        Ok(())
    }

    pub async fn list_pools(&self) -> Vec<PoolState> {
        let pools = self.pools.read().await;
        pools.values().cloned().collect()
    }
}

/// Mock resource pool for testing
#[derive(Debug)]
pub struct MockResourcePool {
    pub pool_id: String,
    pub config: PoolConfig,
    pub state: PoolLifecycleState,
    pub workers: Vec<WorkerId>,
    pub created_at: DateTime<Utc>,
}

impl MockResourcePool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pool_id: config.pool_id.clone(),
            config: config.clone(),
            state: PoolLifecycleState::Creating,
            workers: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.state = PoolLifecycleState::Initializing;

        // Pre-warm workers if configured
        if self.config.pre_warm {
            for _ in 0..self.config.initial_size {
                let worker_id = WorkerId::new();
                self.workers.push(worker_id);
            }
        }

        self.state = PoolLifecycleState::Active;
        Ok(())
    }

    pub async fn update_config(&mut self, new_config: &PoolConfig) -> Result<()> {
        self.config = new_config.clone();
        Ok(())
    }

    pub fn requires_restart(&self, new_config: &PoolConfig) -> bool {
        self.config.provider_type != new_config.provider_type
    }

    pub async fn scale_to(&mut self, new_size: u32) -> Result<()> {
        self.state = PoolLifecycleState::Scaling;

        if new_size > self.workers.len() as u32 {
            // Add workers
            for _ in 0..(new_size - self.workers.len() as u32) {
                let worker_id = WorkerId::new();
                self.workers.push(worker_id);
            }
        } else if new_size < self.workers.len() as u32 {
            // Remove workers
            self.workers.truncate(new_size as usize);
        }

        self.state = PoolLifecycleState::Active;
        Ok(())
    }

    pub async fn force_destroy(&mut self) -> Result<()> {
        self.state = PoolLifecycleState::Destroying;
        self.workers.clear();
        self.state = PoolLifecycleState::Destroyed;
        Ok(())
    }

    pub fn get_health(&self) -> HealthCheckResult {
        HealthCheckResult {
            pool_id: self.pool_id.clone(),
            healthy: self.state == PoolLifecycleState::Active,
            worker_count: self.workers.len() as u32,
            healthy_workers: self.workers.len() as u32,
            last_check: Utc::now(),
            errors: Vec::new(),
        }
    }
}

/// Resource pool lifecycle manager
pub struct ResourcePoolLifecycleManager {
    pools: Arc<RwLock<HashMap<String, Arc<RwLock<MockResourcePool>>>>>,
    state_store: Arc<InMemoryStateStore>,
    event_handlers: Vec<Arc<dyn PoolEventHandler>>,
    health_check_interval: Duration,
}

impl ResourcePoolLifecycleManager {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            state_store: Arc::new(InMemoryStateStore::new()),
            event_handlers: Vec::new(),
            health_check_interval: Duration::from_secs(30),
        }
    }

    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = interval;
        self
    }

    pub fn add_event_handler(&mut self, handler: Arc<dyn PoolEventHandler>) {
        self.event_handlers.push(handler);
    }

    /// Create a new pool
    pub async fn create_pool(&self, config: PoolConfig) -> Result<String> {
        info!(pool_id = %config.pool_id, "Creating resource pool");

        // Validate configuration
        self.validate_config(&config)?;

        // Create pool instance
        let pool = MockResourcePool::new(config.clone());
        let pool_id = config.pool_id.clone();

        // Initialize pool state
        let initial_state = PoolState {
            pool_id: pool_id.clone(),
            status: PoolLifecycleState::Creating,
            config: config.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            worker_count: 0,
            healthy_workers: 0,
            last_health_check: None,
            last_error: None,
        };

        // Persist state
        self.state_store
            .save_pool_state(&pool_id, initial_state)
            .await?;

        // Add to manager
        let pool_arc = Arc::new(RwLock::new(pool));
        let mut pools = self.pools.write().await;
        pools.insert(pool_id.clone(), pool_arc.clone());
        drop(pools);

        // Initialize pool
        {
            let mut pool = pool_arc.write().await;
            pool.initialize().await?;
        }

        // Update state
        let pool_state = PoolState {
            pool_id: pool_id.clone(),
            status: PoolLifecycleState::Active,
            config: config.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            worker_count: config.initial_size,
            healthy_workers: config.initial_size,
            last_health_check: Some(Utc::now()),
            last_error: None,
        };
        self.state_store
            .save_pool_state(&pool_id, pool_state)
            .await?;

        // Emit event
        self.emit_event(PoolEvent::Created {
            pool_id: pool_id.clone(),
            config: config.clone(),
            timestamp: Utc::now(),
        });

        self.emit_event(PoolEvent::Initialized {
            pool_id: pool_id.clone(),
            worker_count: config.initial_size,
            timestamp: Utc::now(),
        });

        info!(pool_id = %pool_id, "Pool created and initialized successfully");
        Ok(pool_id)
    }

    /// Update pool configuration
    pub async fn update_pool_config(&self, pool_id: &str, new_config: PoolConfig) -> Result<()> {
        info!(pool_id, "Updating pool configuration");

        // Validate new configuration
        self.validate_config(&new_config)?;

        // Get pool reference
        let pool = {
            let pools = self.pools.read().await;
            pools
                .get(pool_id)
                .cloned()
                .ok_or_else(|| LifecycleError::PoolNotFound(pool_id.to_string()))?
        };

        // Check if restart is required and update
        {
            let pool_read = pool.read().await;
            let _requires_restart = pool_read.requires_restart(&new_config);
            drop(pool_read);

            // Update config
            let mut pool_write = pool.write().await;
            pool_write.update_config(&new_config).await?;
        }

        // Update state
        self.state_store
            .update_pool_config(pool_id, &new_config)
            .await?;

        // Get old config
        let old_state = self.state_store.get_pool_state(pool_id).await?;
        let old_config = old_state.config;

        // Emit event
        self.emit_event(PoolEvent::ConfigUpdated {
            pool_id: pool_id.to_string(),
            old_config,
            new_config: new_config.clone(),
            timestamp: Utc::now(),
        });

        info!(pool_id, "Pool configuration updated successfully");
        Ok(())
    }

    /// Scale pool to new size
    pub async fn scale_pool(&self, pool_id: &str, new_size: u32) -> Result<()> {
        info!(pool_id, "Scaling pool to {}", new_size);

        // Get pool reference
        let pool = {
            let pools = self.pools.read().await;
            pools
                .get(pool_id)
                .cloned()
                .ok_or_else(|| LifecycleError::PoolNotFound(pool_id.to_string()))?
        };

        let current_size = {
            let pool_read = pool.read().await;
            pool_read.workers.len() as u32
        };

        // Scale
        {
            let mut pool_write = pool.write().await;
            pool_write.scale_to(new_size).await?;
        }

        // Update state
        let mut state = self.state_store.get_pool_state(pool_id).await?;
        state.worker_count = new_size;
        state.healthy_workers = new_size;
        self.state_store.save_pool_state(pool_id, state).await?;

        // Emit event
        self.emit_event(PoolEvent::Scaling {
            pool_id: pool_id.to_string(),
            from_size: current_size,
            to_size: new_size,
            timestamp: Utc::now(),
        });

        info!(
            pool_id,
            "Pool scaled successfully from {} to {}", current_size, new_size
        );
        Ok(())
    }

    /// Destroy pool
    pub async fn destroy_pool(&self, pool_id: &str, force: bool) -> Result<()> {
        info!(pool_id, "Destroying pool (force={})", force);

        let mut pools = self.pools.write().await;
        let pool = pools
            .remove(pool_id)
            .ok_or_else(|| LifecycleError::PoolNotFound(pool_id.to_string()))?;
        drop(pools);

        // Destroy pool
        {
            let mut pool_write = pool.write().await;
            pool_write.force_destroy().await?;
        }

        // Update state
        self.state_store
            .update_pool_status(pool_id, PoolLifecycleState::Destroyed)
            .await?;

        // Emit event
        self.emit_event(PoolEvent::Destroyed {
            pool_id: pool_id.to_string(),
            timestamp: Utc::now(),
        });

        // Remove from state store
        self.state_store.remove_pool(pool_id).await?;

        info!(pool_id, "Pool destroyed successfully");
        Ok(())
    }

    /// Get pool state
    pub async fn get_pool_state(&self, pool_id: &str) -> Result<PoolState> {
        self.state_store.get_pool_state(pool_id).await
    }

    /// List all pools
    pub async fn list_pools(&self) -> Vec<PoolState> {
        self.state_store.list_pools().await
    }

    /// Perform health check on all pools
    pub async fn health_check_all(&self) -> Vec<HealthCheckResult> {
        let pools = self.pools.read().await;
        let mut results = Vec::new();

        for (pool_id, pool) in pools.iter() {
            let health = {
                let pool_read = pool.read().await;
                pool_read.get_health()
            };

            // Update state store
            let state = PoolState {
                pool_id: pool_id.clone(),
                status: if health.healthy {
                    PoolLifecycleState::Active
                } else {
                    PoolLifecycleState::Error("Unhealthy".to_string())
                },
                config: health.pool_id.parse().unwrap_or_else(|_| PoolConfig {
                    pool_id: pool_id.clone(),
                    name: pool_id.clone(),
                    provider_type: "unknown".to_string(),
                    initial_size: 0,
                    min_size: 0,
                    max_size: 0,
                    pre_warm: false,
                    health_check_interval: Duration::from_secs(30),
                    metadata: HashMap::new(),
                    scaling_enabled: false,
                }),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                worker_count: health.worker_count,
                healthy_workers: health.healthy_workers,
                last_health_check: Some(health.last_check),
                last_error: if health.errors.is_empty() {
                    None
                } else {
                    Some(health.errors.join(", "))
                },
            };
            self.state_store.save_pool_state(pool_id, state).await.ok();

            results.push(health);
        }

        if !results.is_empty() {
            self.emit_event(PoolEvent::HealthCheck {
                pool_id: "all".to_string(),
                healthy_workers: results.iter().map(|r| r.healthy_workers).sum(),
                total_workers: results.iter().map(|r| r.worker_count).sum(),
                timestamp: Utc::now(),
            });
        }

        results
    }

    /// Validate pool configuration
    fn validate_config(&self, config: &PoolConfig) -> Result<()> {
        if config.pool_id.is_empty() {
            return Err(
                LifecycleError::InvalidConfig("Pool ID cannot be empty".to_string()).into(),
            );
        }

        if config.provider_type.is_empty() {
            return Err(
                LifecycleError::InvalidConfig("Provider type cannot be empty".to_string()).into(),
            );
        }

        if config.min_size > config.max_size {
            return Err(LifecycleError::InvalidConfig(
                "Min size cannot be greater than max size".to_string(),
            )
            .into());
        }

        if config.initial_size < config.min_size || config.initial_size > config.max_size {
            return Err(LifecycleError::InvalidConfig(
                "Initial size must be between min and max size".to_string(),
            )
            .into());
        }

        Ok(())
    }

    /// Emit event to all handlers
    fn emit_event(&self, event: PoolEvent) {
        for handler in &self.event_handlers {
            handler.handle_event(&event);
        }
    }
}

impl Default for ResourcePoolLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Event handler trait
pub trait PoolEventHandler: Send + Sync {
    fn handle_event(&self, event: &PoolEvent);
}

/// Simple event handler that logs events
#[derive(Debug)]
pub struct LoggingEventHandler;

impl Default for LoggingEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggingEventHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PoolEventHandler for LoggingEventHandler {
    fn handle_event(&self, event: &PoolEvent) {
        match event {
            PoolEvent::Created { pool_id, .. } => info!(pool_id, "Pool created"),
            PoolEvent::Initialized { pool_id, .. } => info!(pool_id, "Pool initialized"),
            PoolEvent::ConfigUpdated { pool_id, .. } => info!(pool_id, "Pool config updated"),
            PoolEvent::Scaling {
                pool_id,
                from_size,
                to_size,
                ..
            } => {
                info!(pool_id, "Pool scaling from {} to {}", from_size, to_size)
            }
            PoolEvent::HealthCheck { pool_id, .. } => {
                debug!(pool_id, "Health check performed")
            }
            PoolEvent::Draining { pool_id, .. } => warn!(pool_id, "Pool draining"),
            PoolEvent::Destroyed { pool_id, .. } => info!(pool_id, "Pool destroyed"),
        }
    }
}

// Convert LifecycleError to DomainError
impl From<LifecycleError> for hodei_pipelines_domain::DomainError {
    fn from(err: LifecycleError) -> Self {
        hodei_pipelines_domain::DomainError::Other(err.to_string())
    }
}
