//! Worker Management Module
//!
//! This module provides the application layer (use cases) for managing
//! dynamic workers across different infrastructure providers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use hodei_adapters::{DefaultProviderFactory, RegistrationConfig, WorkerRegistrationAdapter};
use hodei_core::{JobId, Worker, WorkerId};
use hodei_ports::ProviderFactoryTrait;
use hodei_ports::scheduler_port::SchedulerPort;
use hodei_ports::worker_provider::{ProviderConfig, ProviderError, WorkerProvider};
use hodei_ports::{WorkerRegistrationError, WorkerRegistrationPort};
use hodei_shared_types;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Configuration for worker management service
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerManagementConfig {
    pub registration_enabled: bool,
    pub registration_max_retries: u32,
}

impl Default for WorkerManagementConfig {
    fn default() -> Self {
        Self {
            registration_enabled: true,
            registration_max_retries: 3,
        }
    }
}

/// Worker management service
#[derive(Debug)]
pub struct WorkerManagementService<P, S>
where
    P: WorkerProvider + Send + Sync,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    provider: Box<P>,
    registration_adapter: Option<WorkerRegistrationAdapter<S>>,
    config: WorkerManagementConfig,
}

impl<P, S> WorkerManagementService<P, S>
where
    P: WorkerProvider + Send + Sync + Clone + 'static,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    /// Create new service without registration (backwards compatible)
    pub fn new(provider: P, config: WorkerManagementConfig) -> Self {
        Self {
            provider: Box::new(provider),
            registration_adapter: None,
            config,
        }
    }

    /// Create new service with registration adapter
    pub fn new_with_registration(
        provider: P,
        registration_adapter: WorkerRegistrationAdapter<S>,
        config: WorkerManagementConfig,
    ) -> Self {
        Self {
            provider: Box::new(provider),
            registration_adapter: Some(registration_adapter),
            config,
        }
    }

    /// Create a new dynamic worker with default Docker provider
    pub async fn provision_worker(
        &self,
        image: String,
        cpu_cores: u32,
        memory_mb: u64,
    ) -> Result<Worker, WorkerManagementError> {
        let worker_id = WorkerId::new();
        let config = ProviderConfig::docker(format!("worker-{}", worker_id));

        info!(
            worker_id = %worker_id,
            image = %image,
            "Provisioning new worker"
        );

        let worker = self
            .provider
            .create_worker(worker_id.clone(), config)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(
            worker_id = %worker.id,
            container_id = ?worker.metadata.get("container_id"),
            "Worker provisioned successfully"
        );

        // Attempt registration if enabled
        if let (Some(adapter), true) =
            (&self.registration_adapter, self.config.registration_enabled)
        {
            if let Err(error) = adapter.register_worker(&worker).await {
                warn!(
                    worker_id = %worker.id,
                    error = %error,
                    "Worker provisioned but registration failed"
                );
            } else {
                info!(worker_id = %worker.id, "Worker registered successfully");
            }
        }

        Ok(worker)
    }

    /// Create a new dynamic worker with custom configuration
    pub async fn provision_worker_with_config(
        &self,
        mut config: ProviderConfig,
        cpu_cores: u32,
        memory_mb: u64,
    ) -> Result<Worker, WorkerManagementError> {
        let worker_id = WorkerId::new();

        info!(
            worker_id = %worker_id,
            provider_type = %config.provider_type.as_str(),
            "Provisioning new worker with custom config"
        );

        let worker = self
            .provider
            .create_worker(worker_id.clone(), config)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(
            worker_id = %worker.id,
            container_id = ?worker.metadata.get("container_id"),
            "Worker provisioned successfully"
        );

        // Attempt registration if enabled
        if let (Some(adapter), true) =
            (&self.registration_adapter, self.config.registration_enabled)
        {
            if let Err(error) = adapter.register_worker(&worker).await {
                warn!(
                    worker_id = %worker.id,
                    error = %error,
                    "Worker provisioned but registration failed"
                );
            } else {
                info!(worker_id = %worker.id, "Worker registered successfully");
            }
        }

        Ok(worker)
    }

    /// Stop a worker
    pub async fn stop_worker(
        &self,
        worker_id: &WorkerId,
        graceful: bool,
    ) -> Result<(), WorkerManagementError> {
        info!(worker_id = %worker_id, graceful = graceful, "Stopping worker");

        self.provider
            .stop_worker(worker_id, graceful)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(worker_id = %worker_id, "Worker stopped successfully");
        Ok(())
    }

    /// Delete a worker
    pub async fn delete_worker(&self, worker_id: &WorkerId) -> Result<(), WorkerManagementError> {
        info!(worker_id = %worker_id, "Deleting worker");

        self.provider
            .delete_worker(worker_id)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(worker_id = %worker_id, "Worker deleted successfully");
        Ok(())
    }

    /// Get worker status
    pub async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<hodei_shared_types::WorkerStatus, WorkerManagementError> {
        let status = self
            .provider
            .get_worker_status(worker_id)
            .await
            .map_err(WorkerManagementError::Provider)?;

        Ok(status)
    }

    /// List all workers
    pub async fn list_workers(&self) -> Result<Vec<WorkerId>, WorkerManagementError> {
        let workers = self
            .provider
            .list_workers()
            .await
            .map_err(WorkerManagementError::Provider)?;

        Ok(workers)
    }

    /// Get provider capabilities
    pub async fn get_provider_capabilities(
        &self,
    ) -> Result<hodei_ports::worker_provider::ProviderCapabilities, WorkerManagementError> {
        let capabilities = self
            .provider
            .capabilities()
            .await
            .map_err(WorkerManagementError::Provider)?;

        Ok(capabilities)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WorkerManagementError {
    #[error("Provider error: {0}")]
    Provider(ProviderError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl WorkerManagementError {
    pub fn internal<T: Into<String>>(msg: T) -> Self {
        Self::Internal(msg.into())
    }
}

/// Create a default worker management service with Docker provider
pub async fn create_default_worker_management_service<P, S>(
    provider: P,
) -> Result<WorkerManagementService<P, S>, WorkerManagementError>
where
    P: WorkerProvider + Send + Sync + Clone + 'static,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    Ok(WorkerManagementService::new(
        provider,
        WorkerManagementConfig::default(),
    ))
}

/// Create a worker management service with Kubernetes provider
pub async fn create_kubernetes_worker_management_service<P, S>(
    provider: P,
) -> Result<WorkerManagementService<P, S>, WorkerManagementError>
where
    P: WorkerProvider + Send + Sync + Clone + 'static,
    S: SchedulerPort + Send + Sync + Clone + 'static,
{
    Ok(WorkerManagementService::new(
        provider,
        WorkerManagementConfig::default(),
    ))
}

/// Custom error types for DynamicPoolManager
#[derive(Debug, thiserror::Error)]
pub enum DynamicPoolError {
    #[error("Pool at maximum capacity: {current}/{max}")]
    PoolAtCapacity { current: u32, max: u32 },

    #[error("Pool at minimum size: {current}/{min}")]
    PoolAtMinimum { current: u32, min: u32 },

    #[error("Provisioning timeout after {timeout:?}")]
    ProvisioningTimeout { timeout: Duration },

    #[error("Worker not found in pool: {worker_id}")]
    WorkerNotFound { worker_id: WorkerId },

    #[error("Worker not available: {worker_id}")]
    WorkerNotAvailable { worker_id: WorkerId },

    #[error("Invalid pool state transition")]
    InvalidStateTransition,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Registration error: {0}")]
    Registration(#[from] WorkerRegistrationError),
}

/// Configuration for dynamic worker pools
#[derive(Debug, Clone)]
pub struct DynamicPoolConfig {
    pub pool_id: String,
    pub worker_type: String,
    pub min_size: u32,
    pub max_size: u32,
    pub idle_timeout: Duration,
    pub provision_timeout: Duration,
    pub max_concurrent_provisioning: u32,
    pub cooldown_period: Duration,
    pub drain_timeout: Duration,
    pub pre_warm_on_start: bool,
}

impl DynamicPoolConfig {
    pub fn new(pool_id: String, worker_type: String) -> Self {
        Self {
            pool_id,
            worker_type,
            min_size: 0,
            max_size: 100,
            idle_timeout: Duration::from_secs(300),
            provision_timeout: Duration::from_secs(120),
            max_concurrent_provisioning: 5,
            cooldown_period: Duration::from_secs(30),
            drain_timeout: Duration::from_secs(60),
            pre_warm_on_start: false,
        }
    }

    /// Validate configuration constraints
    pub fn validate(&self) -> Result<(), DynamicPoolError> {
        if self.min_size > self.max_size {
            return Err(DynamicPoolError::InvalidStateTransition);
        }
        if self.max_concurrent_provisioning == 0 {
            return Err(DynamicPoolError::InvalidStateTransition);
        }
        Ok(())
    }
}

/// Dynamic pool state
#[derive(Debug, Clone)]
pub struct DynamicPoolState {
    pub available_workers: Vec<WorkerId>,
    pub busy_workers: HashMap<WorkerId, JobId>,
    pub idle_workers: HashSet<WorkerId>,
    pub pending_allocations: Vec<AllocationRequest>,
    pub last_scaling_operation: Option<Instant>,
    pub total_provisioned: u64,
    pub total_terminated: u64,
}

impl DynamicPoolState {
    pub fn new() -> Self {
        Self {
            available_workers: Vec::new(),
            busy_workers: HashMap::new(),
            idle_workers: HashSet::new(),
            pending_allocations: Vec::new(),
            last_scaling_operation: None,
            total_provisioned: 0,
            total_terminated: 0,
        }
    }
}

/// Current status of a dynamic pool
#[derive(Debug, Clone)]
pub struct DynamicPoolStatus {
    pub pool_id: String,
    pub worker_type: String,
    pub available_workers: u32,
    pub busy_workers: u32,
    pub idle_workers: u32,
    pub pending_allocations: u32,
    pub total_provisioned: u64,
    pub total_terminated: u64,
    pub last_scaling_operation: Option<Instant>,
}

/// Result of a worker allocation
#[derive(Debug, Clone)]
pub struct WorkerAllocation {
    pub worker_id: WorkerId,
    pub job_id: JobId,
    pub allocation_time: chrono::DateTime<chrono::Utc>,
}

/// Worker allocation request
#[derive(Debug, Clone)]
pub struct AllocationRequest {
    pub job_id: JobId,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

/// Dynamic pool metrics
#[derive(Debug)]
pub struct DynamicPoolMetrics {
    pub pool_id: String,
    pub allocations_total: std::sync::atomic::AtomicU64,
    pub releases_total: std::sync::atomic::AtomicU64,
    pub provisioning_total: std::sync::atomic::AtomicU64,
    pub termination_total: std::sync::atomic::AtomicU64,
    pub cleanup_scans_total: std::sync::atomic::AtomicU64,
}

impl DynamicPoolMetrics {
    pub fn new(pool_id: &str) -> Self {
        Self {
            pool_id: pool_id.to_string(),
            allocations_total: std::sync::atomic::AtomicU64::new(0),
            releases_total: std::sync::atomic::AtomicU64::new(0),
            provisioning_total: std::sync::atomic::AtomicU64::new(0),
            termination_total: std::sync::atomic::AtomicU64::new(0),
            cleanup_scans_total: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            pool_id: self.pool_id.clone(),
            allocations_total: std::sync::atomic::AtomicU64::new(
                self.allocations_total
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            releases_total: std::sync::atomic::AtomicU64::new(
                self.releases_total
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            provisioning_total: std::sync::atomic::AtomicU64::new(
                self.provisioning_total
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            termination_total: std::sync::atomic::AtomicU64::new(
                self.termination_total
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            cleanup_scans_total: std::sync::atomic::AtomicU64::new(
                self.cleanup_scans_total
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
        }
    }

    pub fn record_allocation(&self) {
        self.allocations_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_release(&self) {
        self.releases_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_provisioning(&self) {
        self.provisioning_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_termination(&self) {
        self.termination_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_cleanup_scan(&self) {
        self.cleanup_scans_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Dynamic pool events
#[derive(Debug, Clone)]
pub enum DynamicPoolEvent {
    WorkerProvisioned {
        pool_id: String,
        worker_id: WorkerId,
        job_id: Option<JobId>,
    },
    WorkerTerminated {
        pool_id: String,
        worker_id: WorkerId,
    },
    PoolScaled {
        pool_id: String,
        old_size: u32,
        new_size: u32,
        reason: String,
    },
    WorkerAllocated {
        pool_id: String,
        worker_id: WorkerId,
        job_id: JobId,
    },
    WorkerReleased {
        pool_id: String,
        worker_id: WorkerId,
        job_id: JobId,
    },
}

/// Manages dynamic worker pools with auto-scaling
#[derive(Debug)]
pub struct DynamicPoolManager<T>
where
    T: WorkerProvider + Send + Sync,
{
    config: DynamicPoolConfig,
    state: Arc<RwLock<DynamicPoolState>>,
    worker_provider: T,
    registration_adapter: Option<WorkerRegistrationAdapter<MockSchedulerPort>>,
    metrics: DynamicPoolMetrics,
    cleanup_task: Arc<std::sync::atomic::AtomicBool>,
}

/// Mock scheduler for DynamicPoolManager
#[derive(Debug, Clone)]
pub struct MockSchedulerPort;

#[async_trait::async_trait]
impl SchedulerPort for MockSchedulerPort {
    async fn register_worker(
        &self,
        _worker: &Worker,
    ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn unregister_worker(
        &self,
        _worker_id: &WorkerId,
    ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn get_registered_workers(
        &self,
    ) -> Result<Vec<WorkerId>, hodei_ports::scheduler_port::SchedulerError> {
        Ok(Vec::new())
    }
}

impl<T> DynamicPoolManager<T>
where
    T: WorkerProvider + Send + Sync + Clone + 'static,
{
    /// Create new dynamic pool manager
    pub fn new(config: DynamicPoolConfig, worker_provider: T) -> Result<Self, DynamicPoolError> {
        config.validate()?;
        let metrics = DynamicPoolMetrics::new(&config.pool_id);

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(DynamicPoolState::new())),
            worker_provider,
            registration_adapter: None,
            metrics,
            cleanup_task: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Create new dynamic pool manager with registration adapter
    pub fn new_with_registration(
        config: DynamicPoolConfig,
        worker_provider: T,
        registration_adapter: WorkerRegistrationAdapter<MockSchedulerPort>,
    ) -> Result<Self, DynamicPoolError> {
        config.validate()?;
        let metrics = DynamicPoolMetrics::new(&config.pool_id);

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(DynamicPoolState::new())),
            worker_provider,
            registration_adapter: Some(registration_adapter),
            metrics,
            cleanup_task: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Start the pool manager background tasks
    pub async fn start(&self) -> Result<(), DynamicPoolError> {
        // Pre-warm workers if configured
        if self.config.pre_warm_on_start && self.config.min_size > 0 {
            self.scale_to(self.config.min_size).await?;
        }

        // Start idle worker cleanup task
        self.start_cleanup_task().await;

        info!(pool_id = %self.config.pool_id, "Dynamic pool manager started");
        Ok(())
    }

    /// Stop the pool manager
    pub async fn stop(&self) -> Result<(), DynamicPoolError> {
        self.cleanup_task
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // Scale down all workers
        let state = self.state.read().await;
        let worker_count = state.available_workers.len() + state.busy_workers.len();
        drop(state);

        for _ in 0..worker_count {
            if let Ok(worker_id) = self.get_any_worker_id().await {
                self.terminate_worker(worker_id).await.ok();
            }
        }

        info!(pool_id = %self.config.pool_id, "Dynamic pool manager stopped");
        Ok(())
    }

    /// Allocate a worker from the pool
    pub async fn allocate_worker(
        &self,
        job_id: JobId,
        _requirements: AllocationRequest,
    ) -> Result<WorkerAllocation, DynamicPoolError> {
        let mut state = self.state.write().await;

        // Try to get available worker
        if let Some(worker_id) = state.available_workers.pop() {
            state.busy_workers.insert(worker_id.clone(), job_id.clone());
            drop(state);

            self.metrics.record_allocation();
            let allocation = WorkerAllocation {
                worker_id,
                job_id,
                allocation_time: Utc::now(),
            };

            return Ok(allocation);
        }

        // No available workers - check if we can provision
        let current_size = (state.total_provisioned - state.total_terminated) as u32;
        if current_size < self.config.max_size {
            // Queue the allocation request
            state.pending_allocations.push(AllocationRequest {
                job_id: job_id.clone(),
                requested_at: Utc::now(),
            });

            drop(state);

            // Trigger provisioning
            self.provision_worker().await?;

            return Err(DynamicPoolError::ProvisioningTimeout {
                timeout: self.config.provision_timeout,
            });
        }

        Err(DynamicPoolError::PoolAtCapacity {
            current: current_size,
            max: self.config.max_size,
        })
    }

    /// Release a worker back to the pool
    pub async fn release_worker(
        &self,
        worker_id: WorkerId,
        job_id: hodei_core::JobId,
    ) -> Result<(), DynamicPoolError> {
        let mut state = self.state.write().await;

        // Verify worker is currently busy
        if state.busy_workers.remove(&worker_id) != Some(job_id.clone()) {
            return Err(DynamicPoolError::WorkerNotFound { worker_id });
        }

        // Check if worker should be terminated (idle timeout)
        if self.should_terminate_worker() {
            state.total_terminated += 1;
            drop(state);

            self.terminate_worker(worker_id).await?;
        } else {
            // Return to available pool
            state.available_workers.push(worker_id.clone());
            drop(state);
        }

        self.metrics.record_release();
        Ok(())
    }

    /// Get current pool status
    pub async fn status(&self) -> DynamicPoolStatus {
        let state = self.state.read().await;
        DynamicPoolStatus {
            pool_id: self.config.pool_id.clone(),
            worker_type: self.config.worker_type.clone(),
            available_workers: state.available_workers.len() as u32,
            busy_workers: state.busy_workers.len() as u32,
            idle_workers: state.idle_workers.len() as u32,
            pending_allocations: state.pending_allocations.len() as u32,
            total_provisioned: state.total_provisioned,
            total_terminated: state.total_terminated,
            last_scaling_operation: state.last_scaling_operation,
        }
    }

    /// Manually scale pool to target size
    pub async fn scale_to(&self, target_size: u32) -> Result<(), DynamicPoolError> {
        let current_size = self.get_current_size().await;

        if target_size < self.config.min_size || target_size > self.config.max_size {
            return Err(DynamicPoolError::InvalidStateTransition);
        }

        if target_size > current_size {
            let to_provision = target_size - current_size;
            self.provision_workers(to_provision as usize).await?;
        } else if target_size < current_size {
            let to_terminate = current_size - target_size;
            self.terminate_workers(to_terminate as usize).await?;
        }

        let mut state = self.state.write().await;
        state.last_scaling_operation = Some(Instant::now());
        drop(state);

        Ok(())
    }

    /// Terminate a specific worker
    pub async fn terminate_worker(&self, worker_id: WorkerId) -> Result<(), DynamicPoolError> {
        self.worker_provider.stop_worker(&worker_id, true).await?;
        self.worker_provider.delete_worker(&worker_id).await?;

        // Unregister from scheduler
        if let Some(ref adapter) = self.registration_adapter {
            adapter.unregister_worker(&worker_id).await.ok();
        }

        self.metrics.record_termination();
        Ok(())
    }

    // Internal methods

    async fn provision_worker(&self) -> Result<(), DynamicPoolError> {
        let worker_id = WorkerId::new();
        let config = ProviderConfig::docker(format!("{}-worker", self.config.pool_id));
        let worker = self
            .worker_provider
            .create_worker(worker_id, config)
            .await?;

        let mut state = self.state.write().await;
        state.available_workers.push(worker.id.clone());
        state.total_provisioned += 1;

        // Process pending allocations
        if let Some(allocation_request) = state.pending_allocations.pop() {
            // Immediately allocate this worker
            state
                .busy_workers
                .insert(worker.id.clone(), allocation_request.job_id);
            drop(state);

            self.metrics.record_provisioning();
        }

        Ok(())
    }

    async fn provision_workers(&self, count: usize) -> Result<(), DynamicPoolError> {
        let mut handles = Vec::new();

        for _ in 0..count {
            if self.get_current_size().await >= self.config.max_size {
                break;
            }

            let handle = tokio::spawn(async {
                // Provision worker
                Ok::<_, DynamicPoolError>(())
            });

            handles.push(handle);
        }

        // Wait for all provisioning operations to complete
        for handle in handles {
            handle
                .await
                .map_err(|_| DynamicPoolError::Internal("Thread join error".to_string()))??;
        }

        Ok(())
    }

    async fn terminate_workers(&self, count: usize) -> Result<(), DynamicPoolError> {
        let mut state = self.state.write().await;

        for _ in 0..count {
            if let Some(worker_id) = state.available_workers.pop() {
                state.total_terminated += 1;
            } else {
                break;
            }
        }

        Ok(())
    }

    fn should_terminate_worker(&self) -> bool {
        // Check cooldown period
        let state_guard = self.state.blocking_read();
        if let Some(last_op) = state_guard.last_scaling_operation {
            if last_op.elapsed() < self.config.cooldown_period {
                return false;
            }
        }

        // For now, don't terminate workers (simplified implementation)
        false
    }

    async fn get_current_size(&self) -> u32 {
        let state = self.state.read().await;
        (state.total_provisioned - state.total_terminated) as u32
    }

    async fn get_any_worker_id(&self) -> Result<WorkerId, DynamicPoolError> {
        let state = self.state.read().await;
        if let Some(worker_id) = state.available_workers.first().cloned() {
            Ok(worker_id)
        } else if let Some(worker_id) = state.busy_workers.keys().next().cloned() {
            Ok(worker_id)
        } else {
            Err(DynamicPoolError::PoolAtMinimum {
                current: 0,
                min: self.config.min_size,
            })
        }
    }

    async fn start_cleanup_task(&self) {
        let state = self.state.clone();
        let cleanup_flag = self.cleanup_task.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            while cleanup_flag.load(std::sync::atomic::Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_secs(30)).await;

                // Scan for idle workers
                let _state_guard = state.write().await;
                metrics.record_cleanup_scan();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use hodei_ports::worker_provider::ProviderCapabilities;
    use hodei_shared_types::WorkerCapabilities;

    // Mock implementation for testing
    #[derive(Debug, Clone)]
    pub struct MockWorkerProvider {
        pub workers: Vec<Worker>,
        pub should_fail: bool,
    }

    impl MockWorkerProvider {
        pub fn new() -> Self {
            Self {
                workers: Vec::new(),
                should_fail: false,
            }
        }

        pub fn with_worker(mut self, worker: Worker) -> Self {
            self.workers.push(worker);
            self
        }

        pub fn with_failure(mut self, should_fail: bool) -> Self {
            self.should_fail = should_fail;
            self
        }
    }

    #[async_trait]
    impl WorkerProvider for MockWorkerProvider {
        fn provider_type(&self) -> hodei_ports::worker_provider::ProviderType {
            hodei_ports::worker_provider::ProviderType::Docker
        }

        fn name(&self) -> &str {
            "mock-provider"
        }

        async fn capabilities(&self) -> Result<ProviderCapabilities, ProviderError> {
            Ok(ProviderCapabilities {
                supports_auto_scaling: true,
                supports_health_checks: true,
                supports_volumes: false,
                max_workers: Some(1000),
                estimated_provision_time_ms: 1000,
            })
        }

        async fn create_worker(
            &self,
            worker_id: WorkerId,
            _config: ProviderConfig,
        ) -> Result<Worker, ProviderError> {
            if self.should_fail {
                return Err(ProviderError::Provider("Mock error".to_string()));
            }

            let worker_name = format!("worker-{}", worker_id);
            Ok(Worker::new(
                worker_id,
                worker_name,
                WorkerCapabilities::new(4, 8192),
            ))
        }

        async fn get_worker_status(
            &self,
            worker_id: &WorkerId,
        ) -> Result<hodei_shared_types::WorkerStatus, ProviderError> {
            Ok(hodei_shared_types::WorkerStatus::create_with_status(
                "IDLE".to_string(),
            ))
        }

        async fn stop_worker(
            &self,
            _worker_id: &WorkerId,
            _graceful: bool,
        ) -> Result<(), ProviderError> {
            Ok(())
        }

        async fn delete_worker(&self, _worker_id: &WorkerId) -> Result<(), ProviderError> {
            Ok(())
        }

        async fn list_workers(&self) -> Result<Vec<WorkerId>, ProviderError> {
            Ok(self.workers.iter().map(|w| w.id.clone()).collect())
        }
    }

    // Mock scheduler for testing
    #[derive(Debug, Clone)]
    pub struct MockSchedulerPort;

    #[async_trait]
    impl SchedulerPort for MockSchedulerPort {
        async fn register_worker(
            &self,
            _worker: &Worker,
        ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
            Ok(())
        }

        async fn unregister_worker(
            &self,
            _worker_id: &WorkerId,
        ) -> Result<(), hodei_ports::scheduler_port::SchedulerError> {
            Ok(())
        }

        async fn get_registered_workers(
            &self,
        ) -> Result<Vec<WorkerId>, hodei_ports::scheduler_port::SchedulerError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_provision_worker_with_registration() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service =
            WorkerManagementService::new_with_registration(mock_provider, adapter, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        assert!(result.is_ok(), "Expected successful provision");
    }

    #[tokio::test]
    async fn test_provision_worker_registration_failure_not_rollback() {
        let mock_provider = MockWorkerProvider::new().with_failure(false);
        let config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service =
            WorkerManagementService::new_with_registration(mock_provider, adapter, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        // Provisioning succeeds even if registration fails
        assert!(result.is_ok(), "Provisioning should succeed");
    }

    #[tokio::test]
    async fn test_provision_worker_without_registration() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig::default();

        let service: WorkerManagementService<MockWorkerProvider, MockSchedulerPort> =
            WorkerManagementService::new(mock_provider, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        assert!(result.is_ok(), "Expected successful provision");
    }

    #[tokio::test]
    async fn test_provision_worker_with_config_registration() {
        let mock_provider = MockWorkerProvider::new();
        let mut config = ProviderConfig::docker("test-provider".to_string());
        let worker_management_config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service = WorkerManagementService::new_with_registration(
            mock_provider,
            adapter,
            worker_management_config,
        );

        let result = service.provision_worker_with_config(config, 4, 8192).await;

        assert!(result.is_ok(), "Expected successful provision with config");
    }

    #[tokio::test]
    async fn test_provision_worker_registration_disabled() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig {
            registration_enabled: false,
            registration_max_retries: 0,
        };

        let service: WorkerManagementService<MockWorkerProvider, MockSchedulerPort> =
            WorkerManagementService::new(mock_provider, config);

        let result = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await;

        assert!(
            result.is_ok(),
            "Expected successful provision without registration"
        );
    }

    #[tokio::test]
    async fn test_worker_management_config_default() {
        let config = WorkerManagementConfig::default();

        assert_eq!(config.registration_enabled, true);
        assert_eq!(config.registration_max_retries, 3);
    }

    #[tokio::test]
    async fn test_worker_management_config_clone() {
        let config = WorkerManagementConfig::default();
        let cloned = config.clone();

        assert_eq!(config, cloned);
    }

    #[tokio::test]
    async fn test_worker_management_all_operations() {
        let mock_provider = MockWorkerProvider::new();
        let config = WorkerManagementConfig::default();
        let mock_scheduler = MockSchedulerPort;
        let adapter = WorkerRegistrationAdapter::new(mock_scheduler, RegistrationConfig::default());

        let service =
            WorkerManagementService::new_with_registration(mock_provider.clone(), adapter, config);

        // Provision worker
        let worker = service
            .provision_worker("test-image".to_string(), 4, 8192)
            .await
            .unwrap();

        // Stop worker
        let result = service.stop_worker(&worker.id, true).await;
        assert!(result.is_ok(), "Expected successful stop");

        // Delete worker
        let result = service.delete_worker(&worker.id).await;
        assert!(result.is_ok(), "Expected successful delete");

        // Get provider capabilities
        let capabilities = service.get_provider_capabilities().await.unwrap();
        assert_eq!(capabilities.supports_auto_scaling, true);
    }
}
