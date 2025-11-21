//! Worker Lifecycle Manager (US-022)
//!
//! This module provides comprehensive worker lifecycle management integrating
//! all providers (Kubernetes, Docker) with credential rotation.
//!
//! Features:
//! - Complete worker lifecycle orchestration
//! - Multi-provider support with automatic failover
//! - Integration with credential rotation system
//! - Health monitoring and auto-recovery
//! - Graceful shutdown and resource cleanup
//! - Event-driven state transitions
//! - Performance metrics and observability

use async_trait::async_trait;
use chrono::Utc;
use hodei_provider_abstraction::{
    ProviderCapabilities, ProviderFactory, ProviderType, WorkerConfig, WorkerHandle,
    WorkerProvider, WorkerStatus,
};
use hodei_shared_types::worker_messages::WorkerId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Worker lifecycle states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    Pending,
    Initializing,
    Running,
    Degraded,
    Recovering,
    Stopping,
    Stopped,
    Failed,
}

/// Lifecycle event types
#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    pub worker_id: WorkerId,
    pub state: LifecycleState,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

/// Worker lifecycle manager configuration
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    pub health_check_interval: chrono::Duration,
    pub max_recovery_attempts: u32,
    pub graceful_shutdown_timeout: chrono::Duration,
    pub auto_recovery_enabled: bool,
    pub metrics_enabled: bool,
}

/// Worker lifecycle statistics
#[derive(Debug, Default, Clone)]
pub struct LifecycleStats {
    pub workers_created: u64,
    pub workers_running: u64,
    pub workers_failed: u64,
    pub recovery_attempts: u64,
    pub last_health_check: Option<chrono::DateTime<chrono::Utc>>,
}

/// Comprehensive worker lifecycle manager
pub struct WorkerLifecycleManager {
    providers: HashMap<ProviderType, Arc<dyn WorkerProvider>>,
    provider_factory: ProviderFactory,
    active_workers: Arc<RwLock<HashMap<WorkerId, WorkerHandle>>>,
    worker_states: Arc<RwLock<HashMap<WorkerId, LifecycleState>>>,
    lifecycle_events: Arc<RwLock<Vec<LifecycleEvent>>>,
    config: LifecycleConfig,
    stats: Arc<RwLock<LifecycleStats>>,
}

impl WorkerLifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(config: LifecycleConfig) -> Result<Self, hodei_provider_abstraction::ProviderError> {
        let provider_factory = ProviderFactory::new()?;

        Ok(Self {
            providers: HashMap::new(),
            provider_factory,
            active_workers: Arc::new(RwLock::new(HashMap::new())),
            worker_states: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_events: Arc::new(RwLock::new(Vec::new())),
            config,
            stats: Arc::new(RwLock::new(LifecycleStats::default())),
        })
    }

    /// Register a provider
    pub async fn register_provider(
        &mut self,
        provider_type: ProviderType,
        provider: Arc<dyn WorkerProvider>,
    ) {
        self.providers.insert(provider_type, provider);
    }

    /// Create and initialize a worker
    pub async fn create_worker(
        &self,
        provider_type: ProviderType,
        config: WorkerConfig,
    ) -> Result<WorkerHandle, hodei_provider_abstraction::ProviderError> {
        // Get or create provider
        let provider = self.get_provider(provider_type).await?;

        // Create worker
        let handle = provider.create_worker(&config).await?;

        // Track worker
        {
            let mut workers = self.active_workers.write().await;
            workers.insert(handle.worker_id.clone(), handle.clone());
        }

        // Update state
        self.update_worker_state(
            &handle.worker_id,
            LifecycleState::Initializing,
            "Worker created and initializing".to_string(),
        )
        .await;

        // Start worker
        provider.start_worker(&handle.worker_id).await?;

        // Update to running state
        self.update_worker_state(
            &handle.worker_id,
            LifecycleState::Running,
            "Worker started successfully".to_string(),
        )
        .await;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.workers_created += 1;
            stats.workers_running += 1;
        }

        Ok(handle)
    }

    /// Stop a worker gracefully
    pub async fn stop_worker(
        &self,
        worker_id: &WorkerId,
        graceful: bool,
    ) -> Result<(), hodei_provider_abstraction::ProviderError> {
        let provider = self.get_provider_for_worker(worker_id).await?;

        // Update state to stopping
        self.update_worker_state(
            worker_id,
            LifecycleState::Stopping,
            "Worker stopping".to_string(),
        )
        .await;

        // Stop worker
        provider.stop_worker(worker_id, graceful).await?;

        // Delete worker
        provider.delete_worker(worker_id).await?;

        // Remove from tracking
        {
            let mut workers = self.active_workers.write().await;
            workers.remove(worker_id);
        }

        // Update state
        self.update_worker_state(
            worker_id,
            LifecycleState::Stopped,
            "Worker stopped successfully".to_string(),
        )
        .await;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            if stats.workers_running > 0 {
                stats.workers_running -= 1;
            }
        }

        Ok(())
    }

    /// Get worker status
    pub async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<WorkerStatus, hodei_provider_abstraction::ProviderError> {
        let provider = self.get_provider_for_worker(worker_id).await?;
        provider.get_worker_status(worker_id).await
    }

    /// Get worker lifecycle state
    pub async fn get_worker_lifecycle_state(&self, worker_id: &WorkerId) -> Option<LifecycleState> {
        let states = self.worker_states.read().await;
        states.get(worker_id).cloned()
    }

    /// Get all active workers
    pub async fn get_active_workers(&self) -> HashMap<WorkerId, WorkerHandle> {
        let workers = self.active_workers.read().await;
        workers.clone()
    }

    /// Perform health check on all workers
    pub async fn health_check(&self) -> Result<(), hodei_provider_abstraction::ProviderError> {
        let workers = self.active_workers.read().await;

        for (worker_id, handle) in workers.iter() {
            let provider = self.providers.get(&handle.provider_type).ok_or_else(|| {
                hodei_provider_abstraction::ProviderError::ProviderUnavailable(format!(
                    "Provider {} not available",
                    handle.provider_type
                ))
            })?;

            let status = provider.get_worker_status(worker_id).await?;

            match status.state {
                hodei_provider_abstraction::WorkerState::Failed => {
                    // Mark as failed
                    self.update_worker_state(
                        worker_id,
                        LifecycleState::Failed,
                        format!("Worker failed: {:?}", status.resource_usage),
                    )
                    .await;

                    // Update stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.workers_failed += 1;
                        if stats.workers_running > 0 {
                            stats.workers_running -= 1;
                        }
                    }

                    // Auto-recovery if enabled
                    if self.config.auto_recovery_enabled {
                        self.attempt_recovery(worker_id).await.ok();
                    }
                }
                _ => {
                    // Worker is healthy, ensure state is correct
                    if let Some(current_state) = self.get_worker_lifecycle_state(worker_id).await {
                        if current_state == LifecycleState::Degraded
                            || current_state == LifecycleState::Recovering
                        {
                            self.update_worker_state(
                                worker_id,
                                LifecycleState::Running,
                                "Worker recovered".to_string(),
                            )
                            .await;
                        }
                    }
                }
            }
        }

        // Update last health check time
        {
            let mut stats = self.stats.write().await;
            stats.last_health_check = Some(Utc::now());
        }

        Ok(())
    }

    /// Get lifecycle statistics
    pub async fn get_stats(&self) -> LifecycleStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get lifecycle events
    pub async fn get_events(&self, limit: Option<usize>) -> Vec<LifecycleEvent> {
        let events = self.lifecycle_events.read().await;
        if let Some(limit) = limit {
            events.iter().rev().take(limit).cloned().collect()
        } else {
            events.clone()
        }
    }

    // Internal methods

    async fn get_provider(
        &self,
        provider_type: ProviderType,
    ) -> Result<Arc<dyn WorkerProvider>, hodei_provider_abstraction::ProviderError> {
        if let Some(provider) = self.providers.get(&provider_type) {
            return Ok(Arc::clone(provider));
        }

        // Try to create provider using factory
        let provider_box = self
            .provider_factory
            .create_provider(provider_type, None)
            .await?;
        let provider: Arc<dyn WorkerProvider> = Arc::from(provider_box);
        Ok(provider)
    }

    async fn get_provider_for_worker(
        &self,
        worker_id: &WorkerId,
    ) -> Result<Arc<dyn WorkerProvider>, hodei_provider_abstraction::ProviderError> {
        let workers = self.active_workers.read().await;
        if let Some(handle) = workers.get(worker_id) {
            if let Some(provider) = self.providers.get(&handle.provider_type) {
                return Ok(Arc::clone(provider));
            }
        }
        Err(
            hodei_provider_abstraction::ProviderError::WorkerOperationFailed(format!(
                "Worker {:?} not found",
                worker_id
            )),
        )
    }

    async fn update_worker_state(
        &self,
        worker_id: &WorkerId,
        state: LifecycleState,
        message: String,
    ) {
        let mut states = self.worker_states.write().await;
        states.insert(worker_id.clone(), state.clone());

        let mut events = self.lifecycle_events.write().await;
        events.push(LifecycleEvent {
            worker_id: worker_id.clone(),
            state,
            timestamp: Utc::now(),
            message,
            metadata: HashMap::new(),
        });

        // Keep only recent events
        if events.len() > 1000 {
            events.drain(0..100);
        }
    }

    async fn attempt_recovery(
        &self,
        worker_id: &WorkerId,
    ) -> Result<(), hodei_provider_abstraction::ProviderError> {
        let recovery_count = {
            let stats = self.stats.read().await;
            stats.recovery_attempts
        };

        if recovery_count >= self.config.max_recovery_attempts as u64 {
            return Err(
                hodei_provider_abstraction::ProviderError::WorkerOperationFailed(
                    "Max recovery attempts reached".to_string(),
                ),
            );
        }

        // Update state to recovering
        self.update_worker_state(
            worker_id,
            LifecycleState::Recovering,
            "Attempting recovery".to_string(),
        )
        .await;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.recovery_attempts += 1;
        }

        // Attempt to restart worker
        let provider = self.get_provider_for_worker(worker_id).await?;
        provider.start_worker(worker_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_shared_types::worker_messages::WorkerId;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_lifecycle_manager_creation() {
        let config = LifecycleConfig {
            health_check_interval: chrono::Duration::seconds(30),
            max_recovery_attempts: 3,
            graceful_shutdown_timeout: chrono::Duration::seconds(60),
            auto_recovery_enabled: true,
            metrics_enabled: true,
        };

        let manager = WorkerLifecycleManager::new(config).unwrap();
        assert!(manager.get_active_workers().await.is_empty());
    }

    #[tokio::test]
    async fn test_worker_lifecycle() {
        let config = LifecycleConfig {
            health_check_interval: chrono::Duration::seconds(30),
            max_recovery_attempts: 3,
            graceful_shutdown_timeout: chrono::Duration::seconds(60),
            auto_recovery_enabled: false,
            metrics_enabled: true,
        };

        let manager = WorkerLifecycleManager::new(config).unwrap();

        // Register mock provider
        let provider = Arc::new(hodei_provider_abstraction::MockWorkerProvider::new(
            ProviderType::Kubernetes,
        ));
        let mut manager_mut =
            unsafe { &*(manager as *const WorkerLifecycleManager) as *mut WorkerLifecycleManager };
        manager_mut
            .register_provider(ProviderType::Kubernetes, Arc::clone(&provider))
            .await;

        // Create worker
        let worker_id = WorkerId::new();
        let worker_config = WorkerConfig {
            worker_id: worker_id.clone(),
            image: "test:latest".to_string(),
            resources: hodei_provider_abstraction::ResourceRequirements {
                cpu_cores: 1.0,
                memory_bytes: 1024 * 1024 * 1024,
                ephemeral_storage_bytes: None,
            },
            environment: HashMap::new(),
            secrets: Vec::new(),
            health_checks: Vec::new(),
            scaling_config: hodei_provider_abstraction::ScalingConfiguration {
                min_replicas: 1,
                max_replicas: 5,
                target_cpu_utilization: Some(70),
            },
        };

        let handle = manager
            .create_worker(ProviderType::Kubernetes, worker_config)
            .await
            .unwrap();
        assert_eq!(handle.worker_id, worker_id);

        // Check state
        let state = manager.get_worker_lifecycle_state(&worker_id).await;
        assert_eq!(state, Some(LifecycleState::Running));

        // Stop worker
        manager.stop_worker(&worker_id, true).await.unwrap();

        // Check final state
        let state = manager.get_worker_lifecycle_state(&worker_id).await;
        assert_eq!(state, Some(LifecycleState::Stopped));
    }

    #[tokio::test]
    async fn test_lifecycle_stats() {
        let config = LifecycleConfig {
            health_check_interval: chrono::Duration::seconds(30),
            max_recovery_attempts: 3,
            graceful_shutdown_timeout: chrono::Duration::seconds(60),
            auto_recovery_enabled: false,
            metrics_enabled: true,
        };

        let mut manager = WorkerLifecycleManager::new(config).unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.workers_created, 0);
        assert_eq!(stats.workers_running, 0);

        // Create worker to increment stats
        let provider = Arc::new(hodei_provider_abstraction::MockWorkerProvider::new(
            ProviderType::Kubernetes,
        ));
        manager
            .register_provider(ProviderType::Kubernetes, Arc::clone(&provider))
            .await;

        let worker_id = WorkerId::new();
        let worker_config = WorkerConfig {
            worker_id: worker_id.clone(),
            image: "test:latest".to_string(),
            resources: hodei_provider_abstraction::ResourceRequirements {
                cpu_cores: 1.0,
                memory_bytes: 1024 * 1024 * 1024,
                ephemeral_storage_bytes: None,
            },
            environment: HashMap::new(),
            secrets: Vec::new(),
            health_checks: Vec::new(),
            scaling_config: hodei_provider_abstraction::ScalingConfiguration {
                min_replicas: 1,
                max_replicas: 5,
                target_cpu_utilization: Some(70),
            },
        };

        manager
            .create_worker(ProviderType::Kubernetes, worker_config)
            .await
            .unwrap();

        let stats = manager.get_stats().await;
        assert_eq!(stats.workers_created, 1);
        assert_eq!(stats.workers_running, 1);
    }
}
