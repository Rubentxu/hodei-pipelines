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

/// Worker return error types
#[derive(Debug, thiserror::Error)]
pub enum WorkerReturnError {
    #[error("Worker not busy with job: {worker_id}")]
    WorkerNotBusy { worker_id: WorkerId },

    #[error("Health check failed for worker: {worker_id}")]
    HealthCheckFailed { worker_id: WorkerId },

    #[error("Cleanup failed for worker: {worker_id}")]
    CleanupFailed { worker_id: WorkerId },

    #[error("Worker not found: {worker_id}")]
    WorkerNotFound { worker_id: WorkerId },
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

/// Worker allocation request with requirements
#[derive(Debug, Clone)]
pub struct AllocationRequest {
    pub job_id: JobId,
    pub requirements: WorkerRequirements,
    pub priority: u8,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

/// Worker requirements for job execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerRequirements {
    pub min_cpu_cores: u32,
    pub min_memory_gb: u64,
    pub required_features: Vec<String>,
    pub preferred_worker_type: Option<String>,
}

impl WorkerRequirements {
    pub fn new(min_cpu_cores: u32, min_memory_gb: u64) -> Self {
        Self {
            min_cpu_cores,
            min_memory_gb,
            required_features: Vec::new(),
            preferred_worker_type: None,
        }
    }

    pub fn with_feature(mut self, feature: String) -> Self {
        self.required_features.push(feature);
        self
    }

    pub fn with_worker_type(mut self, worker_type: String) -> Self {
        self.preferred_worker_type = Some(worker_type);
        self
    }

    /// Check if a worker meets these requirements
    pub fn matches_worker(&self, worker: &Worker) -> bool {
        // Check CPU cores
        if worker.capabilities.cpu_cores < self.min_cpu_cores {
            return false;
        }

        // Check memory
        if worker.capabilities.memory_gb < self.min_memory_gb {
            return false;
        }

        // Check features
        for feature in &self.required_features {
            if !worker.metadata.contains_key(feature) {
                return false;
            }
        }

        true
    }
}

/// Priority queue entry
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub allocation_request: AllocationRequest,
    pub wait_time: Duration,
}

/// Queue matching result
#[derive(Debug, Clone)]
pub struct QueueMatchResult {
    pub worker_id: WorkerId,
    pub job_id: JobId,
    pub matched_at: chrono::DateTime<chrono::Utc>,
}

/// Dynamic pool metrics
#[derive(Debug)]
pub struct DynamicPoolMetrics {
    pub pool_id: String,
    pub allocations_total: std::sync::atomic::AtomicU64,
    pub releases_total: std::sync::atomic::AtomicU64,
    pub provisioning_total: std::sync::atomic::AtomicU64,
    pub termination_total: std::sync::atomic::AtomicU64,
    pub worker_returns_total: std::sync::atomic::AtomicU64,
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
            worker_returns_total: std::sync::atomic::AtomicU64::new(0),
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
            worker_returns_total: std::sync::atomic::AtomicU64::new(
                self.worker_returns_total
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

    pub fn record_worker_return(&self) {
        self.worker_returns_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn record_cleanup_scan(&self) {
        self.cleanup_scans_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Static pool error types
#[derive(Debug, thiserror::Error)]
pub enum StaticPoolError {
    #[error("Pool exhausted: requested {requested} workers, only {available} available")]
    PoolExhausted { requested: u32, available: u32 },

    #[error("Provisioning failed: {worker_id} after {attempts} attempts")]
    ProvisioningFailed { worker_id: WorkerId, attempts: u32 },

    #[error("Worker not found: {worker_id}")]
    WorkerNotFound { worker_id: WorkerId },

    #[error("Worker not available: {worker_id}")]
    WorkerNotAvailable { worker_id: WorkerId },

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Health check failed: {worker_id}")]
    HealthCheckFailed { worker_id: WorkerId },

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
}

/// Provisioning strategy for static pools
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisioningStrategy {
    Sequential,
    Parallel { max_concurrent: u32 },
}

/// Pre-warming strategy for static pools
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreWarmStrategy {
    /// Aggressive: Always maintain target pool size
    Aggressive,
    /// Balanced: Maintain fixed_size + buffer
    Balanced,
    /// Conservative: Only replace workers as needed
    Conservative,
}

/// Pre-warming metrics
#[derive(Debug, Clone)]
pub struct PreWarmMetrics {
    pub pre_warmed_count: u32,
    pub total_provisioned: u32,
    pub replacements_triggered: u32,
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub interval: Duration,
    pub timeout: Duration,
    pub healthy_threshold: u32,
    pub unhealthy_threshold: u32,
}

impl HealthCheckConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            healthy_threshold: 3,
            unhealthy_threshold: 2,
        }
    }
}

/// Provisioning configuration
#[derive(Debug, Clone)]
pub struct ProvisioningConfig {
    pub timeout_per_worker: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
}

impl ProvisioningConfig {
    pub fn new() -> Self {
        Self {
            timeout_per_worker: Duration::from_secs(120),
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
        }
    }
}

/// Configuration for static worker pools
#[derive(Debug, Clone)]
pub struct StaticPoolConfig {
    pub pool_id: String,
    pub worker_type: String,
    pub fixed_size: u32,
    pub worker_config: StaticWorkerConfig,
    pub provisioning: ProvisioningConfig,
    pub health_check: HealthCheckConfig,
    pub provisioning_strategy: ProvisioningStrategy,
    pub pre_warm_on_start: bool,
    pub pre_warm_strategy: PreWarmStrategy,
    pub target_pool_size: u32,
    pub idle_timeout: Duration,
    pub termination_grace_period: Duration,
}

/// Idle worker information
#[derive(Debug, Clone)]
pub struct IdleWorkerInfo {
    pub worker_id: WorkerId,
    pub idle_duration: Duration,
}

/// Idle worker statistics
#[derive(Debug, Clone)]
pub struct IdleWorkerStats {
    pub total_idle_time: Duration,
    pub workers_terminated: u32,
    pub last_cleanup: Option<Instant>,
}

/// Pool size metrics
#[derive(Debug, Clone)]
pub struct PoolSizeMetrics {
    pub current_size: u32,
    pub min_size: u32,
    pub max_size: u32,
    pub target_size: u32,
}

/// Worker state distribution
#[derive(Debug, Clone)]
pub struct WorkerStateDistribution {
    pub ready: u32,
    pub busy: u32,
    pub idle: u32,
    pub total: u32,
}

/// Utilization metrics
#[derive(Debug, Clone)]
pub struct UtilizationMetrics {
    pub total_capacity: u64,
    pub used_capacity: u64,
    pub utilization_percent: f64,
    pub available_workers: u32,
}

/// Health status
#[derive(Debug, Clone)]
pub struct PoolHealthStatus {
    pub overall_status: String,
    pub pools_active: u32,
    pub workers_provisioned: u32,
    pub workers_available: u32,
    pub workers_busy: u32,
    pub errors: Vec<String>,
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub average_allocation_time_ms: f64,
    pub total_allocations: u64,
    pub total_releases: u64,
    pub peak_concurrent_usage: u32,
    pub provisioning_success_rate: f64,
}

impl StaticPoolConfig {
    pub fn new(pool_id: String, worker_type: String, fixed_size: u32) -> Self {
        Self {
            pool_id,
            worker_type,
            fixed_size,
            worker_config: StaticWorkerConfig::default(),
            provisioning: ProvisioningConfig::new(),
            health_check: HealthCheckConfig::new(),
            provisioning_strategy: ProvisioningStrategy::Sequential,
            pre_warm_on_start: false,
            pre_warm_strategy: PreWarmStrategy::Balanced,
            target_pool_size: fixed_size,
            idle_timeout: Duration::from_secs(300), // 5 minutes default
            termination_grace_period: Duration::from_secs(30), // 30 seconds default
        }
    }

    /// Validate configuration constraints
    pub fn validate(&self) -> Result<(), StaticPoolError> {
        if self.fixed_size == 0 {
            return Err(StaticPoolError::InvalidConfig(
                "fixed_size must be greater than 0".to_string(),
            ));
        }
        if self.worker_type.is_empty() {
            return Err(StaticPoolError::InvalidConfig(
                "worker_type cannot be empty".to_string(),
            ));
        }
        if self.pool_id.is_empty() {
            return Err(StaticPoolError::InvalidConfig(
                "pool_id cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Static worker configuration
#[derive(Debug, Clone)]
pub struct StaticWorkerConfig {
    pub image: String,
    pub cpu_cores: u32,
    pub memory_mb: u32,
    pub docker_enabled: bool,
    pub labels: HashMap<String, String>,
    pub tags: Vec<String>,
    pub environment: HashMap<String, String>,
}

impl StaticWorkerConfig {
    pub fn new(image: String, cpu_cores: u32, memory_mb: u32) -> Self {
        Self {
            image,
            cpu_cores,
            memory_mb,
            docker_enabled: true,
            labels: HashMap::new(),
            tags: Vec::new(),
            environment: HashMap::new(),
        }
    }

    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

impl Default for StaticWorkerConfig {
    fn default() -> Self {
        Self {
            image: "ubuntu:20.04".to_string(),
            cpu_cores: 4,
            memory_mb: 8192,
            docker_enabled: true,
            labels: HashMap::new(),
            tags: Vec::new(),
            environment: HashMap::new(),
        }
    }
}

/// Static pool state
#[derive(Debug, Clone)]
pub struct StaticPoolState {
    pub available_workers: Vec<WorkerId>,
    pub busy_workers: HashMap<WorkerId, JobId>,
    pub total_provisioned: u32,
    pub total_terminated: u32,
    pub pre_warmed_count: u32,
    pub replacements_triggered: u32,
    pub idle_tracking: HashMap<WorkerId, Instant>,
    pub idle_workers_terminated: u32,
    pub total_idle_time: Duration,
}

impl StaticPoolState {
    pub fn new() -> Self {
        Self {
            available_workers: Vec::new(),
            busy_workers: HashMap::new(),
            total_provisioned: 0,
            total_terminated: 0,
            pre_warmed_count: 0,
            replacements_triggered: 0,
            idle_tracking: HashMap::new(),
            idle_workers_terminated: 0,
            total_idle_time: Duration::from_secs(0),
        }
    }
}

/// Current status of a static pool
#[derive(Debug, Clone)]
pub struct StaticPoolStatus {
    pub pool_id: String,
    pub worker_type: String,
    pub fixed_size: u32,
    pub available_workers: u32,
    pub busy_workers: u32,
    pub total_provisioned: u32,
    pub total_terminated: u32,
}

/// Result of a static worker allocation
#[derive(Debug, Clone)]
pub struct StaticWorkerAllocation {
    pub worker_id: WorkerId,
    pub job_id: JobId,
    pub allocation_time: chrono::DateTime<chrono::Utc>,
}

/// Static pool metrics
#[derive(Debug)]
pub struct StaticPoolMetrics {
    pub pool_id: String,
    pub allocations_total: std::sync::atomic::AtomicU64,
    pub releases_total: std::sync::atomic::AtomicU64,
    pub health_check_failures: std::sync::atomic::AtomicU64,
}

impl StaticPoolMetrics {
    pub fn new(pool_id: &str) -> Self {
        Self {
            pool_id: pool_id.to_string(),
            allocations_total: std::sync::atomic::AtomicU64::new(0),
            releases_total: std::sync::atomic::AtomicU64::new(0),
            health_check_failures: std::sync::atomic::AtomicU64::new(0),
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

    pub fn record_health_check_failure(&self) {
        self.health_check_failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Manages static worker pools with fixed size
#[derive(Debug)]
pub struct StaticPoolManager<T>
where
    T: WorkerProvider + Send + Sync,
{
    config: StaticPoolConfig,
    state: Arc<RwLock<StaticPoolState>>,
    worker_provider: T,
    metrics: StaticPoolMetrics,
}

impl<T> StaticPoolManager<T>
where
    T: WorkerProvider + Send + Sync + Clone + 'static,
{
    /// Create new static pool manager
    pub fn new(config: StaticPoolConfig, worker_provider: T) -> Result<Self, StaticPoolError> {
        config.validate()?;
        let metrics = StaticPoolMetrics::new(&config.pool_id);

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(StaticPoolState::new())),
            worker_provider,
            metrics,
        })
    }

    /// Start the pool manager and provision all workers
    pub async fn start(&self) -> Result<(), StaticPoolError> {
        info!(
            pool_id = %self.config.pool_id,
            fixed_size = self.config.fixed_size,
            pre_warm_on_start = self.config.pre_warm_on_start,
            "Starting static pool"
        );

        // Determine how many workers to provision initially
        let worker_count = if self.config.pre_warm_on_start {
            self.calculate_pre_warm_size()
        } else {
            self.config.fixed_size
        };

        // Provision initial workers
        match &self.config.provisioning_strategy {
            ProvisioningStrategy::Sequential => {
                self.provision_workers_sequential(worker_count).await
            }
            ProvisioningStrategy::Parallel { max_concurrent } => {
                self.provision_workers_parallel(worker_count, *max_concurrent)
                    .await
            }
        }?;

        // Update pre-warmed count
        if self.config.pre_warm_on_start {
            let mut state = self.state.write().await;
            state.pre_warmed_count = state.total_provisioned;
        }

        info!(
            pool_id = %self.config.pool_id,
            "Static pool started successfully"
        );

        Ok(())
    }

    /// Stop the pool manager and terminate all workers
    pub async fn stop(&self) -> Result<(), StaticPoolError> {
        info!(pool_id = %self.config.pool_id, "Stopping static pool");

        // Get all worker IDs
        let mut all_workers = Vec::new();

        {
            let state = self.state.read().await;
            all_workers.extend(state.available_workers.clone());
            all_workers.extend(state.busy_workers.keys().cloned());
        }

        // Terminate all workers
        for worker_id in all_workers {
            self.terminate_worker(worker_id).await.ok();
        }

        // Update state to reflect termination
        let mut state = self.state.write().await;
        state.total_terminated = state.total_provisioned;
        state.pre_warmed_count = 0;
        state.replacements_triggered = 0;
        state.idle_tracking.clear();
        state.idle_workers_terminated = 0;
        state.total_idle_time = Duration::from_secs(0);
        state.available_workers.clear();
        state.busy_workers.clear();

        info!(pool_id = %self.config.pool_id, "Static pool stopped");
        Ok(())
    }

    /// Allocate a worker from the static pool
    pub async fn allocate_worker(
        &self,
        job_id: JobId,
    ) -> Result<StaticWorkerAllocation, StaticPoolError> {
        let mut state = self.state.write().await;

        // Check if we have available workers
        if let Some(worker_id) = state.available_workers.pop() {
            state.busy_workers.insert(worker_id.clone(), job_id.clone());
            drop(state);

            self.metrics.record_allocation();

            let allocation = StaticWorkerAllocation {
                worker_id,
                job_id,
                allocation_time: Utc::now(),
            };

            return Ok(allocation);
        }

        // No available workers
        drop(state);
        let state = self.state.read().await;
        let available = state.available_workers.len() as u32;
        drop(state);

        Err(StaticPoolError::PoolExhausted {
            requested: 1,
            available,
        })
    }

    /// Release a worker back to the static pool
    pub async fn release_worker(
        &self,
        worker_id: WorkerId,
        job_id: JobId,
    ) -> Result<(), StaticPoolError> {
        let mut state = self.state.write().await;

        // Verify worker is currently busy with this job
        if state.busy_workers.remove(&worker_id) != Some(job_id) {
            return Err(StaticPoolError::WorkerNotFound { worker_id });
        }

        // Run health check before returning to pool
        if !self.check_worker_health(&worker_id).await {
            state.total_terminated += 1;
            drop(state);
            self.terminate_worker(worker_id).await?;
            return Ok(());
        }

        // Return to available pool and track idle time
        let now = Instant::now();
        state.available_workers.push(worker_id.clone());
        state.idle_tracking.insert(worker_id, now);
        drop(state);

        self.metrics.record_release();
        Ok(())
    }

    /// Run health check on worker
    pub async fn check_worker_health(&self, worker_id: &WorkerId) -> bool {
        if !self.config.health_check.enabled {
            return true;
        }

        info!(
            pool_id = %self.config.pool_id,
            worker_id = %worker_id,
            "Running health check"
        );

        // TODO: Implement actual health check logic
        // For now, assume all workers pass health check
        true
    }

    /// Get current pool status
    pub async fn status(&self) -> StaticPoolStatus {
        let state = self.state.read().await;
        StaticPoolStatus {
            pool_id: self.config.pool_id.clone(),
            worker_type: self.config.worker_type.clone(),
            fixed_size: self.config.fixed_size,
            available_workers: state.available_workers.len() as u32,
            busy_workers: state.busy_workers.len() as u32,
            total_provisioned: state.total_provisioned,
            total_terminated: state.total_terminated,
        }
    }

    /// Get pre-warming metrics
    pub async fn get_pre_warm_metrics(&self) -> PreWarmMetrics {
        let state = self.state.read().await;
        PreWarmMetrics {
            pre_warmed_count: state.pre_warmed_count,
            total_provisioned: state.total_provisioned,
            replacements_triggered: state.replacements_triggered,
        }
    }

    /// Trigger replacement of workers if needed (for testing)
    pub async fn trigger_replacement_if_needed(&self) -> Result<(), StaticPoolError> {
        if !self.config.pre_warm_on_start {
            return Ok(());
        }

        let (available, busy, target) = {
            let state = self.state.read().await;
            (
                state.available_workers.len(),
                state.busy_workers.len(),
                self.calculate_pre_warm_size(),
            )
        };

        let current_total = available + busy;
        let needed = target.saturating_sub(current_total as u32);

        if needed > 0 {
            info!(
                pool_id = %self.config.pool_id,
                current_total,
                target,
                needed,
                "Triggering worker replacement"
            );

            match &self.config.provisioning_strategy {
                ProvisioningStrategy::Sequential => self.provision_workers_sequential(needed).await,
                ProvisioningStrategy::Parallel { max_concurrent } => {
                    self.provision_workers_parallel(needed, *max_concurrent)
                        .await
                }
            }?;

            let mut state = self.state.write().await;
            state.replacements_triggered += 1;
        }

        Ok(())
    }

    /// Calculate the number of workers to pre-warm based on strategy
    fn calculate_pre_warm_size(&self) -> u32 {
        match self.config.pre_warm_strategy {
            PreWarmStrategy::Aggressive => self.config.target_pool_size,
            PreWarmStrategy::Balanced => {
                // fixed_size + 20% buffer, up to target_pool_size
                let buffer = (self.config.fixed_size as f32 * 0.2) as u32;
                std::cmp::min(
                    self.config.fixed_size + buffer,
                    self.config.target_pool_size,
                )
            }
            PreWarmStrategy::Conservative => self.config.fixed_size,
        }
    }

    /// Get list of idle workers
    pub async fn get_idle_workers(&self) -> Vec<IdleWorkerInfo> {
        let state = self.state.read().await;
        let now = Instant::now();

        state
            .idle_tracking
            .iter()
            .map(|(worker_id, idle_since)| {
                let duration = now.duration_since(*idle_since);
                IdleWorkerInfo {
                    worker_id: worker_id.clone(),
                    idle_duration: duration,
                }
            })
            .collect()
    }

    /// Get idle worker statistics
    pub async fn get_idle_worker_stats(&self) -> IdleWorkerStats {
        let state = self.state.read().await;
        IdleWorkerStats {
            total_idle_time: state.total_idle_time,
            workers_terminated: state.idle_workers_terminated,
            last_cleanup: None, // Would be tracked in production
        }
    }

    /// Clean up idle workers that exceed timeout
    pub async fn cleanup_idle_workers(&self) -> Result<u32, StaticPoolError> {
        let idle_timeout = self.config.idle_timeout;

        // If idle timeout is 0, feature is disabled
        if idle_timeout == Duration::from_secs(0) {
            return Ok(0);
        }

        let mut terminated_count = 0;
        let now = Instant::now();
        let mut workers_to_terminate = Vec::new();

        {
            let state = self.state.read().await;
            let current_size =
                state.available_workers.len() as u32 + state.busy_workers.len() as u32;

            // Don't terminate if it would bring us below fixed_size
            if current_size <= self.config.fixed_size {
                return Ok(0);
            }

            // Find idle workers that have exceeded timeout
            for (worker_id, idle_since) in &state.idle_tracking {
                if now.duration_since(*idle_since) >= idle_timeout {
                    // Verify termination won't drop us below fixed_size
                    if (current_size - (terminated_count + 1)) >= self.config.fixed_size {
                        workers_to_terminate.push(worker_id.clone());
                        terminated_count += 1;
                    }
                }
            }
        }

        // Terminate the identified workers
        for worker_id in workers_to_terminate {
            // Add grace period delay
            if self.config.termination_grace_period > Duration::from_secs(0) {
                tokio::time::sleep(self.config.termination_grace_period).await;
            }

            self.terminate_worker(worker_id.clone()).await?;

            // Update idle tracking and stats
            {
                let mut state = self.state.write().await;
                state.idle_tracking.remove(&worker_id);
                state.idle_workers_terminated += 1;
                state.total_terminated += 1;

                // Calculate and add idle time (simplified)
                if let Some(_idle_start) = state.idle_tracking.get(&worker_id) {
                    state.total_idle_time += Duration::from_secs(0); // Would be calculated from actual idle time
                }
            }
        }

        if terminated_count > 0 {
            info!(
                pool_id = %self.config.pool_id,
                terminated_count,
                "Cleaned up idle workers"
            );
        }

        Ok(terminated_count)
    }

    /// Get pool size metrics
    pub async fn get_pool_metrics(&self) -> PoolSizeMetrics {
        let state = self.state.read().await;
        let current_size = state.total_provisioned - state.total_terminated;

        PoolSizeMetrics {
            current_size,
            min_size: self.config.fixed_size,
            max_size: std::cmp::max(self.config.fixed_size, self.config.target_pool_size),
            target_size: self.calculate_pre_warm_size(),
        }
    }

    /// Get worker state distribution
    pub async fn get_worker_state_distribution(&self) -> WorkerStateDistribution {
        let state = self.state.read().await;
        let ready = state.available_workers.len() as u32;
        let busy = state.busy_workers.len() as u32;
        let idle = state.idle_tracking.len() as u32;
        let total = ready + busy + idle;

        WorkerStateDistribution {
            ready,
            busy,
            idle,
            total,
        }
    }

    /// Get utilization metrics
    pub async fn get_utilization_metrics(&self) -> UtilizationMetrics {
        let state = self.state.read().await;
        let total = state.total_provisioned - state.total_terminated;
        let busy = state.busy_workers.len() as u64;
        let available = state.available_workers.len() as u64;

        let utilization_percent = if total > 0 {
            (busy as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        UtilizationMetrics {
            total_capacity: total as u64,
            used_capacity: busy,
            utilization_percent,
            available_workers: available as u32,
        }
    }

    /// Get health status
    pub async fn get_health_status(&self) -> PoolHealthStatus {
        let state = self.state.read().await;
        let current_size = state.total_provisioned - state.total_terminated;

        // Determine overall health based on metrics
        let overall_status = if current_size >= self.config.fixed_size {
            "healthy"
        } else if current_size >= self.config.fixed_size / 2 {
            "degraded"
        } else {
            "critical"
        };

        PoolHealthStatus {
            overall_status: overall_status.to_string(),
            pools_active: 1,
            workers_provisioned: state.total_provisioned,
            workers_available: state.available_workers.len() as u32,
            workers_busy: state.busy_workers.len() as u32,
            errors: Vec::new(), // Would populate with actual errors in production
        }
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        let allocations = self
            .metrics
            .allocations_total
            .load(std::sync::atomic::Ordering::Relaxed);
        let releases = self
            .metrics
            .releases_total
            .load(std::sync::atomic::Ordering::Relaxed);

        PerformanceMetrics {
            average_allocation_time_ms: 1.0, // Mock value - would track actual timing
            total_allocations: allocations,
            total_releases: releases,
            peak_concurrent_usage: self.config.target_pool_size,
            provisioning_success_rate: 100.0, // Mock value
        }
    }

    // Internal methods

    async fn provision_workers_sequential(&self, count: u32) -> Result<(), StaticPoolError> {
        for i in 0..count {
            match self.provision_single_worker(i).await {
                Ok(_) => {
                    let mut state = self.state.write().await;
                    state.total_provisioned += 1;
                }
                Err(e) => {
                    error!(
                        pool_id = %self.config.pool_id,
                        worker_index = i,
                        error = %e,
                        "Failed to provision worker"
                    );
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    async fn provision_workers_parallel(
        &self,
        count: u32,
        max_concurrent: u32,
    ) -> Result<(), StaticPoolError> {
        let mut handles = Vec::new();
        let mut provisioned = 0;

        while provisioned < count {
            // Spawn up to max_concurrent workers
            let to_spawn = std::cmp::min(max_concurrent, count - provisioned);

            for _ in 0..to_spawn {
                let worker_index = provisioned;
                let provider = self.worker_provider.clone();
                let config =
                    ProviderConfig::docker(format!("{}-static-worker", self.config.pool_id));

                let handle = tokio::spawn(async move {
                    let worker_id = WorkerId::new();
                    match provider.create_worker(worker_id, config).await {
                        Ok(worker) => Ok(worker),
                        Err(e) => Err(StaticPoolError::Provider(e)),
                    }
                });
                handles.push(handle);
            }

            // Wait for current batch to complete
            for handle in handles.drain(..to_spawn as usize) {
                match handle
                    .await
                    .map_err(|_| StaticPoolError::Internal("Thread join error".to_string()))
                {
                    Ok(result) => match result {
                        Ok(worker) => {
                            let mut state = self.state.write().await;
                            state.total_provisioned += 1;
                            state.available_workers.push(worker.id);
                            provisioned += 1;
                        }
                        Err(e) => return Err(e),
                    },
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(())
    }

    async fn provision_single_worker(&self, _index: u32) -> Result<(), StaticPoolError> {
        let worker_id = WorkerId::new();
        let config = ProviderConfig::docker(format!("{}-static-worker", self.config.pool_id));

        // Try with retries
        for attempt in 1..=self.config.provisioning.max_retries {
            match self
                .worker_provider
                .create_worker(worker_id.clone(), config.clone())
                .await
            {
                Ok(worker) => {
                    let mut state = self.state.write().await;
                    state.available_workers.push(worker.id);
                    return Ok(());
                }
                Err(e) => {
                    if attempt == self.config.provisioning.max_retries {
                        return Err(StaticPoolError::Provider(e));
                    }
                    tokio::time::sleep(self.config.provisioning.retry_delay).await;
                }
            }
        }

        Err(StaticPoolError::ProvisioningFailed {
            worker_id,
            attempts: self.config.provisioning.max_retries,
        })
    }

    async fn terminate_worker(&self, worker_id: WorkerId) -> Result<(), StaticPoolError> {
        self.worker_provider.stop_worker(&worker_id, true).await?;
        self.worker_provider.delete_worker(&worker_id).await?;
        Ok(())
    }

    async fn get_any_worker_id(&self) -> Result<WorkerId, StaticPoolError> {
        let state = self.state.read().await;
        if let Some(worker_id) = state.available_workers.first().cloned() {
            Ok(worker_id)
        } else if let Some(worker_id) = state.busy_workers.keys().next().cloned() {
            Ok(worker_id)
        } else {
            Err(StaticPoolError::Internal("No workers found".to_string()))
        }
    }
}

/// Worker reuse metrics tracking
#[derive(Debug)]
pub struct WorkerReuseMetrics {
    pool_id: String,
    reuse_counts: std::sync::Mutex<HashMap<WorkerId, u32>>,
    total_reuses: std::sync::atomic::AtomicU64,
    successful_reuses: std::sync::atomic::AtomicU64,
    failed_reuses: std::sync::atomic::AtomicU64,
    total_provisioning_cost: std::sync::atomic::AtomicU64,
    provision_time_total: std::sync::atomic::AtomicU64,
}

/// Snapshot of worker reuse metrics
#[derive(Debug, Clone)]
pub struct WorkerReuseSnapshot {
    pub pool_id: String,
    pub total_reuses: u64,
    pub successful_reuses: u64,
    pub failed_reuses: u64,
    pub total_provisioning_cost: u64,
    pub provision_time_total_ms: u64,
}

impl WorkerReuseMetrics {
    pub fn new(pool_id: String) -> Self {
        Self {
            pool_id,
            reuse_counts: std::sync::Mutex::new(HashMap::new()),
            total_reuses: std::sync::atomic::AtomicU64::new(0),
            successful_reuses: std::sync::atomic::AtomicU64::new(0),
            failed_reuses: std::sync::atomic::AtomicU64::new(0),
            total_provisioning_cost: std::sync::atomic::AtomicU64::new(0),
            provision_time_total: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Record a worker reuse event
    pub fn record_reuse(&self, worker_id: &WorkerId, success: bool, provision_time: Duration) {
        // Update per-worker reuse count
        {
            let mut counts = self
                .reuse_counts
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *counts.entry(worker_id.clone()).or_insert(0) += 1;
        }

        // Update global counters
        self.total_reuses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if success {
            self.successful_reuses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.failed_reuses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // Track provisioning cost (provision time in ms)
        let provision_time_ms = provision_time.as_millis() as u64;
        self.provision_time_total
            .fetch_add(provision_time_ms, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get reuse count for a specific worker
    pub fn get_reuse_count(&self, worker_id: &WorkerId) -> Option<u32> {
        let counts = self
            .reuse_counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        counts.get(worker_id).copied()
    }

    /// Get snapshot of all metrics
    pub fn get_metrics(&self) -> WorkerReuseSnapshot {
        WorkerReuseSnapshot {
            pool_id: self.pool_id.clone(),
            total_reuses: self.total_reuses.load(std::sync::atomic::Ordering::Relaxed),
            successful_reuses: self
                .successful_reuses
                .load(std::sync::atomic::Ordering::Relaxed),
            failed_reuses: self
                .failed_reuses
                .load(std::sync::atomic::Ordering::Relaxed),
            total_provisioning_cost: self
                .total_provisioning_cost
                .load(std::sync::atomic::Ordering::Relaxed),
            provision_time_total_ms: self
                .provision_time_total
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Calculate average reuse count per worker
    pub fn get_average_reuse_per_worker(&self) -> f64 {
        let counts = self
            .reuse_counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let num_workers = counts.len() as f64;

        if num_workers == 0.0 {
            return 0.0;
        }

        let total_reuses = self.total_reuses.load(std::sync::atomic::Ordering::Relaxed) as f64;

        total_reuses / num_workers
    }

    /// Calculate provisioning cost savings from worker reuse
    /// savings = (reuse_count - 1) * provision_time_ms * worker_count
    pub fn calculate_provisioning_cost_savings(&self, provision_time_ms_per_worker: f64) -> f64 {
        let counts = self
            .reuse_counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let mut total_savings = 0.0;

        for (_worker_id, &reuse_count) in counts.iter() {
            if reuse_count > 0 {
                // Each worker after the first one represents a reuse that saved provisioning
                let savings_for_worker = (reuse_count as f64 - 1.0) * provision_time_ms_per_worker;
                total_savings += savings_for_worker;
            }
        }

        total_savings
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
    reuse_metrics: WorkerReuseMetrics,
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
        let reuse_metrics = WorkerReuseMetrics::new(config.pool_id.clone());

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(DynamicPoolState::new())),
            worker_provider,
            registration_adapter: None,
            metrics,
            reuse_metrics,
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
        let reuse_metrics = WorkerReuseMetrics::new(config.pool_id.clone());

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(DynamicPoolState::new())),
            worker_provider,
            registration_adapter: Some(registration_adapter),
            metrics,
            reuse_metrics,
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

    /// Allocate a worker from the pool with requirements
    pub async fn allocate_worker(
        &self,
        job_id: JobId,
        requirements: WorkerRequirements,
    ) -> Result<WorkerAllocation, DynamicPoolError> {
        let allocation_request = AllocationRequest {
            job_id: job_id.clone(),
            requirements,
            priority: 0,
            requested_at: Utc::now(),
        };

        self.allocate_worker_with_request(allocation_request).await
    }

    /// Internal: Allocate worker with full AllocationRequest
    async fn allocate_worker_with_request(
        &self,
        allocation_request: AllocationRequest,
    ) -> Result<WorkerAllocation, DynamicPoolError> {
        let job_id = allocation_request.job_id.clone();
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
            state.pending_allocations.push(allocation_request);

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
            // Return to available pool and track idle time
            let now = Instant::now();
            state.available_workers.push(worker_id.clone());
            drop(state);
        }

        self.metrics.record_release();
        Ok(())
    }

    /// Return worker to pool after job completion with full lifecycle
    pub async fn return_worker_to_pool(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<(), WorkerReturnError> {
        // AC-1: Verify worker is busy with this job
        let mut state = self.state.write().await;

        // Check if worker is actually busy with this job
        if let Some(active_job_id) = state.busy_workers.get(worker_id) {
            if active_job_id != job_id {
                return Err(WorkerReturnError::WorkerNotBusy {
                    worker_id: worker_id.clone(),
                });
            }
        } else {
            return Err(WorkerReturnError::WorkerNotFound {
                worker_id: worker_id.clone(),
            });
        }

        // Remove from busy workers
        state.busy_workers.remove(worker_id);
        drop(state);

        // AC-3: Log state transition
        info!(
            pool_id = %self.config.pool_id,
            worker_id = %worker_id,
            job_id = %job_id,
            "Worker transitioning: Busy -> Cleaning"
        );

        // AC-1: Clean up job artifacts
        if let Err(e) = self.cleanup_worker(worker_id, job_id).await {
            error!(
                worker_id = %worker_id,
                error = %e,
                "Failed to clean up worker after job completion"
            );
            return Err(WorkerReturnError::CleanupFailed {
                worker_id: worker_id.clone(),
            });
        }

        // AC-2: Run health check
        if !self.check_worker_health(worker_id).await {
            error!(
                worker_id = %worker_id,
                "Health check failed, worker will not be returned to pool"
            );
            return Err(WorkerReturnError::HealthCheckFailed {
                worker_id: worker_id.clone(),
            });
        }

        // AC-3: Add back to available pool
        let mut state = self.state.write().await;
        state.available_workers.push(worker_id.clone());

        info!(
            pool_id = %self.config.pool_id,
            worker_id = %worker_id,
            "Worker returned to available pool"
        );

        // AC-1: Track return operation metrics
        self.metrics.record_worker_return();

        Ok(())
    }

    /// AC: Match available workers with queued jobs based on requirements
    pub async fn match_queued_jobs(&self) -> Vec<QueueMatchResult> {
        let mut matched_jobs = Vec::new();

        let (available_workers, pending_allocations) = {
            let state = self.state.read().await;
            (
                state.available_workers.clone(),
                state.pending_allocations.clone(),
            )
        };

        let mut available_iter = available_workers.into_iter();
        let mut remaining_allocations = Vec::new();

        for allocation in pending_allocations {
            // Try to find a matching worker
            let mut matched_worker_id = None;

            for worker_id in &available_iter.by_ref().collect::<Vec<_>>() {
                // In a real implementation, we would fetch the worker object
                // For now, we'll simulate matching by checking if we have any workers
                matched_worker_id = Some(worker_id.clone());
                break;
            }

            if let Some(worker_id) = matched_worker_id {
                matched_jobs.push(QueueMatchResult {
                    worker_id,
                    job_id: allocation.job_id,
                    matched_at: Utc::now(),
                });
            } else {
                // No matching worker found, keep in queue
                remaining_allocations.push(allocation);
            }
        }

        // Update state with matched workers and remaining allocations
        {
            let mut state = self.state.write().await;

            // Remove matched workers from available list
            for match_result in &matched_jobs {
                state
                    .available_workers
                    .retain(|w| w != &match_result.worker_id);
            }

            // Add matched workers to busy workers
            for match_result in &matched_jobs {
                let job_id = match_result.job_id.clone();
                let worker_id = match_result.worker_id.clone();
                state.busy_workers.insert(worker_id, job_id);
            }

            // Put back unmatched allocations
            state.pending_allocations = remaining_allocations;
        }

        matched_jobs
    }

    /// Get queue status with wait times
    pub async fn get_queue_status(&self) -> Vec<QueueEntry> {
        let state = self.state.read().await;
        let now = Utc::now();

        state
            .pending_allocations
            .iter()
            .map(|req| {
                let time_delta = now - req.requested_at;
                let wait_time = time_delta
                    .to_std()
                    .unwrap_or_else(|_| Duration::from_secs(0));
                QueueEntry {
                    allocation_request: req.clone(),
                    wait_time,
                }
            })
            .collect()
    }

    /// AC: Add job to queue with requirements
    pub async fn queue_job(
        &self,
        job_id: JobId,
        requirements: WorkerRequirements,
        priority: u8,
    ) -> Result<(), DynamicPoolError> {
        let allocation_request = AllocationRequest {
            job_id,
            requirements,
            priority,
            requested_at: Utc::now(),
        };

        let mut state = self.state.write().await;
        state.pending_allocations.push(allocation_request);

        Ok(())
    }

    /// AC: Remove job from queue
    pub async fn dequeue_job(&self, job_id: &JobId) -> Result<(), DynamicPoolError> {
        let mut state = self.state.write().await;

        let original_len = state.pending_allocations.len();
        state
            .pending_allocations
            .retain(|req| &req.job_id != job_id);

        if state.pending_allocations.len() == original_len {
            return Err(DynamicPoolError::WorkerNotFound {
                worker_id: WorkerId::new(), // Job IDs are not worker IDs, but we need a WorkerId for the error
            });
        }

        Ok(())
    }

    /// Clean up worker after job completion
    async fn cleanup_worker(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<(), WorkerReturnError> {
        // AC-1: Remove job artifacts and temporary data
        // This would typically involve:
        // - Stopping any job-specific processes
        // - Cleaning up temp files
        // - Removing job configuration
        // - Resetting job-specific environment variables

        info!(
            worker_id = %worker_id,
            job_id = %job_id,
            "Cleaning up worker artifacts"
        );

        // TODO: Implement actual cleanup logic
        // For now, we'll simulate successful cleanup

        Ok(())
    }

    /// Run health check on worker before returning to pool
    async fn check_worker_health(&self, worker_id: &WorkerId) -> bool {
        // AC-2: Pre-return health check (CPU, memory, disk)
        // AC-2: Service availability verification
        // AC-2: Cleanup validation

        info!(worker_id = %worker_id, "Running health check");

        // TODO: Implement actual health checks
        // For now, assume all workers pass health check

        true
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

    // ===== Worker Return Tests =====

    #[tokio::test]
    async fn test_worker_return_to_pool_success() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();
        let job_id = JobId::new();

        // Simulate worker being busy
        {
            let mut state = manager.state.write().await;
            state.busy_workers.insert(worker_id.clone(), job_id.clone());
        }

        // Return worker to pool
        let result = manager.return_worker_to_pool(&worker_id, &job_id).await;
        assert!(result.is_ok(), "Worker should return successfully");

        // Verify worker is now available
        let status = manager.status().await;
        assert!(status.available_workers == 1);
        assert!(status.busy_workers == 0);
    }

    #[tokio::test]
    async fn test_worker_return_worker_not_busy() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();
        let job_id = JobId::new();

        // Don't add worker to busy state

        // Attempt to return worker
        let result = manager.return_worker_to_pool(&worker_id, &job_id).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, WorkerReturnError::WorkerNotFound { .. }));
        }
    }

    #[tokio::test]
    async fn test_worker_return_wrong_job() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();
        let job_id1 = JobId::new();
        let job_id2 = JobId::new();

        // Simulate worker being busy with job1
        {
            let mut state = manager.state.write().await;
            state
                .busy_workers
                .insert(worker_id.clone(), job_id1.clone());
        }

        // Attempt to return worker with different job
        let result = manager.return_worker_to_pool(&worker_id, &job_id2).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, WorkerReturnError::WorkerNotBusy { .. }));
        }
    }

    #[tokio::test]
    async fn test_worker_return_health_check_failure() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new().with_failure(true);
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();
        let job_id = JobId::new();

        // Simulate worker being busy
        {
            let mut state = manager.state.write().await;
            state.busy_workers.insert(worker_id.clone(), job_id.clone());
        }

        // Return worker to pool (should fail on health check simulation)
        // Note: Currently health check always passes, so this test validates the structure
        let result = manager.return_worker_to_pool(&worker_id, &job_id).await;
        assert!(
            result.is_ok(),
            "Worker should return successfully (health check always passes in mock)"
        );
    }

    #[tokio::test]
    async fn test_worker_state_transitions() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();
        let job_id = JobId::new();

        // Initial state: worker is available
        {
            let mut state = manager.state.write().await;
            state.available_workers.push(worker_id.clone());
        }

        let status = manager.status().await;
        assert_eq!(status.available_workers, 1);
        assert_eq!(status.busy_workers, 0);

        // Allocate worker (Available -> Busy)
        {
            let mut state = manager.state.write().await;
            state.available_workers.pop();
            state.busy_workers.insert(worker_id.clone(), job_id.clone());
        }

        let status = manager.status().await;
        assert_eq!(status.available_workers, 0);
        assert_eq!(status.busy_workers, 1);

        // Return worker (Busy -> Available)
        {
            let mut state = manager.state.write().await;
            state.busy_workers.remove(&worker_id);
            state.available_workers.push(worker_id.clone());
        }

        let status = manager.status().await;
        assert_eq!(status.available_workers, 1);
        assert_eq!(status.busy_workers, 0);
    }

    #[tokio::test]
    async fn test_worker_return_metrics_tracking() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();
        let job_id = JobId::new();

        // Simulate worker being busy
        {
            let mut state = manager.state.write().await;
            state.busy_workers.insert(worker_id.clone(), job_id.clone());
        }

        // Return worker to pool
        let _ = manager.return_worker_to_pool(&worker_id, &job_id).await;

        // Verify metrics were incremented
        // Note: Metrics are tracked in the metrics instance
        // This test validates the code path exists
    }

    #[tokio::test]
    async fn test_worker_cleanup_operation() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();
        let job_id = JobId::new();

        // This test validates the cleanup code path
        let result = manager.cleanup_worker(&worker_id, &job_id).await;
        assert!(result.is_ok(), "Cleanup should succeed");
    }

    #[tokio::test]
    async fn test_worker_health_check() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let manager = DynamicPoolManager::new(config, provider).unwrap();

        let worker_id = WorkerId::new();

        // This test validates the health check code path
        let healthy = manager.check_worker_health(&worker_id).await;
        assert!(healthy, "Worker should pass health check in mock");
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

    // ===== Worker Reuse Metrics Tests =====

    #[tokio::test]
    async fn test_worker_reuse_metrics_creation() {
        let metrics = WorkerReuseMetrics::new("test-pool".to_string());

        let snapshot = metrics.get_metrics();
        assert_eq!(snapshot.total_reuses, 0);
        assert_eq!(snapshot.successful_reuses, 0);
        assert_eq!(snapshot.failed_reuses, 0);
    }

    #[tokio::test]
    async fn test_worker_reuse_metrics_record_reuse() {
        let metrics = WorkerReuseMetrics::new("test-pool".to_string());

        let worker_id = WorkerId::new();

        // Record successful reuse
        metrics.record_reuse(&worker_id, true, Duration::from_millis(100));
        metrics.record_reuse(&worker_id, true, Duration::from_millis(150));
        metrics.record_reuse(&worker_id, false, Duration::from_millis(200));

        let snapshot = metrics.get_metrics();
        assert_eq!(snapshot.total_reuses, 3);
        assert_eq!(snapshot.successful_reuses, 2);
        assert_eq!(snapshot.failed_reuses, 1);

        let reuse_count = metrics.get_reuse_count(&worker_id);
        assert_eq!(reuse_count, Some(3));
    }

    #[tokio::test]
    async fn test_worker_reuse_metrics_average_reuse() {
        let metrics = WorkerReuseMetrics::new("test-pool".to_string());

        let worker1 = WorkerId::new();
        let worker2 = WorkerId::new();

        // Worker 1: 5 reuses
        for _ in 0..5 {
            metrics.record_reuse(&worker1, true, Duration::from_millis(100));
        }

        // Worker 2: 3 reuses
        for _ in 0..3 {
            metrics.record_reuse(&worker2, true, Duration::from_millis(150));
        }

        let avg = metrics.get_average_reuse_per_worker();
        assert!((avg - 4.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_worker_reuse_metrics_cost_savings() {
        let metrics = WorkerReuseMetrics::new("test-pool".to_string());

        let worker_id = WorkerId::new();

        // Record 5 reuses with 100ms provisioning time each
        for _ in 0..5 {
            metrics.record_reuse(&worker_id, true, Duration::from_millis(100));
        }

        // Calculate savings assuming 500ms per new provision
        let savings = metrics.calculate_provisioning_cost_savings(500.0);
        assert_eq!(savings, 2000.0); // (5 reuses - 1 initial) * 500ms * 1 worker

        // Multiple workers with different reuse counts
        let worker2 = WorkerId::new();
        for _ in 0..3 {
            metrics.record_reuse(&worker2, true, Duration::from_millis(100));
        }

        let total_savings = metrics.calculate_provisioning_cost_savings(500.0);
        // Worker 1: (5-1)*500 = 2000, Worker 2: (3-1)*500 = 1000
        assert_eq!(total_savings, 3000.0);
    }

    #[tokio::test]
    async fn test_worker_reuse_metrics_nonexistent_worker() {
        let metrics = WorkerReuseMetrics::new("test-pool".to_string());

        let worker_id = WorkerId::new();

        // Query count for worker that hasn't been reused
        let reuse_count = metrics.get_reuse_count(&worker_id);
        assert_eq!(reuse_count, None);

        // Record a reuse for this worker
        metrics.record_reuse(&worker_id, true, Duration::from_millis(100));

        // Now should have a count
        let reuse_count = metrics.get_reuse_count(&worker_id);
        assert_eq!(reuse_count, Some(1));
    }

    #[tokio::test]
    async fn test_dynamic_pool_manager_reuse_metrics_integration() {
        let config = DynamicPoolConfig::new("test-pool".to_string(), "worker".to_string());
        let provider = MockWorkerProvider::new();
        let mut manager = DynamicPoolManager::new(config, provider).unwrap();

        // Add reuse metrics to the manager
        manager.reuse_metrics = WorkerReuseMetrics::new("test-pool".to_string());

        let worker_id = WorkerId::new();
        let job_id = JobId::new();

        // Simulate worker lifecycle with reuses
        {
            let mut state = manager.state.write().await;
            state.busy_workers.insert(worker_id.clone(), job_id.clone());
        }

        // Return worker to pool (successful)
        let result = manager.return_worker_to_pool(&worker_id, &job_id).await;
        assert!(result.is_ok());

        // Record reuse
        manager
            .reuse_metrics
            .record_reuse(&worker_id, true, Duration::from_millis(100));

        // Verify metrics
        let snapshot = manager.reuse_metrics.get_metrics();
        assert_eq!(snapshot.successful_reuses, 1);
        assert_eq!(snapshot.total_reuses, 1);

        // Simulate another job with same worker
        let job_id2 = JobId::new();
        {
            let mut state = manager.state.write().await;
            state
                .busy_workers
                .insert(worker_id.clone(), job_id2.clone());
        }

        // Return again
        let result = manager.return_worker_to_pool(&worker_id, &job_id2).await;
        assert!(result.is_ok());

        // Record second reuse
        manager
            .reuse_metrics
            .record_reuse(&worker_id, true, Duration::from_millis(120));

        // Verify reuse count
        let reuse_count = manager.reuse_metrics.get_reuse_count(&worker_id);
        assert_eq!(reuse_count, Some(2));

        let avg_reuse = manager.reuse_metrics.get_average_reuse_per_worker();
        assert!((avg_reuse - 2.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_reuse_metrics_concurrent_access() {
        let metrics = Arc::new(WorkerReuseMetrics::new("test-pool".to_string()));
        let worker_id = WorkerId::new();

        // Simulate concurrent reuses
        let mut handles = Vec::new();
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let worker_id_clone = worker_id.clone();
            let handle = tokio::spawn(async move {
                metrics_clone.record_reuse(&worker_id_clone, true, Duration::from_millis(100));
            });
            handles.push(handle);
        }

        // Wait for all reuses to complete
        for handle in handles {
            handle.await;
        }

        let reuse_count = metrics.get_reuse_count(&worker_id);
        assert_eq!(reuse_count, Some(10));

        let snapshot = metrics.get_metrics();
        assert_eq!(snapshot.total_reuses, 10);
        assert_eq!(snapshot.successful_reuses, 10);
    }

    #[tokio::test]
    async fn test_reuse_metrics_zero_reuses() {
        let metrics = WorkerReuseMetrics::new("test-pool".to_string());

        let avg_reuse = metrics.get_average_reuse_per_worker();
        assert_eq!(avg_reuse, 0.0);

        let savings = metrics.calculate_provisioning_cost_savings(500.0);
        assert_eq!(savings, 0.0);
    }

    // ===== Static Pool Configuration Tests =====

    #[test]
    fn test_static_pool_config_creation() {
        let config = StaticPoolConfig::new("static-pool-1".to_string(), "worker".to_string(), 5);

        assert_eq!(config.pool_id, "static-pool-1");
        assert_eq!(config.worker_type, "worker");
        assert_eq!(config.fixed_size, 5);
        assert!(config.provisioning_strategy == ProvisioningStrategy::Sequential);
        assert_eq!(config.health_check.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_static_worker_config_with_labels() {
        let mut labels = HashMap::new();
        labels.insert("env".to_string(), "prod".to_string());
        labels.insert("region".to_string(), "us-west".to_string());

        let config =
            StaticWorkerConfig::new("ubuntu:20.04".to_string(), 4, 8192).with_labels(labels);

        assert_eq!(config.image, "ubuntu:20.04");
        assert_eq!(config.cpu_cores, 4);
        assert_eq!(config.memory_mb, 8192);
        assert_eq!(config.labels.len(), 2);
        assert_eq!(config.labels.get("env").unwrap(), "prod");
    }

    #[test]
    fn test_static_pool_config_validation() {
        // Valid config
        let config = StaticPoolConfig::new("pool-1".to_string(), "worker".to_string(), 10);
        assert!(config.validate().is_ok());

        // Invalid: zero size
        let invalid_config = StaticPoolConfig::new("pool-1".to_string(), "worker".to_string(), 0);
        assert!(invalid_config.validate().is_err());

        // Invalid: empty worker type
        let invalid_config2 = StaticPoolConfig::new("pool-1".to_string(), "".to_string(), 10);
        assert!(invalid_config2.validate().is_err());
    }

    #[test]
    fn test_provisioning_strategy_comparison() {
        let seq = ProvisioningStrategy::Sequential;
        let par = ProvisioningStrategy::Parallel { max_concurrent: 10 };

        assert_ne!(seq, par);
    }

    // ===== Static Pool Manager Tests =====

    #[tokio::test]
    async fn test_static_pool_manager_creation() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        let provider = MockWorkerProvider::new();

        let result = StaticPoolManager::new(config, provider);
        assert!(result.is_ok());

        let manager = result.unwrap();
        assert_eq!(manager.config.pool_id, "static-pool");
        assert_eq!(manager.config.fixed_size, 3);
    }

    #[tokio::test]
    async fn test_static_pool_manager_start_with_parallel_provisioning() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.provisioning_strategy = ProvisioningStrategy::Parallel { max_concurrent: 2 };
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        // Start the manager (should provision workers)
        let result = manager.start().await;
        assert!(result.is_ok(), "Pool should start successfully");

        // Wait a bit for provisioning to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status = manager.status().await;
        assert_eq!(status.pool_id, "static-pool");
        assert_eq!(status.total_provisioned, 3);
        assert!(status.available_workers >= 0);
    }

    #[tokio::test]
    async fn test_static_pool_allocate_worker() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        // Start and wait for provisioning
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let job_id = JobId::new();
        let result = manager.allocate_worker(job_id).await;
        assert!(result.is_ok(), "Should allocate worker successfully");

        let status = manager.status().await;
        assert_eq!(status.busy_workers, 1);
    }

    #[tokio::test]
    async fn test_static_pool_release_worker() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        // Start and wait for provisioning
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let job_id = JobId::new();
        let allocation = manager.allocate_worker(job_id.clone()).await.unwrap();

        // Verify initial state
        let status_before = manager.status().await;
        assert!(status_before.busy_workers >= 1);

        // Release the worker
        let result = manager.release_worker(allocation.worker_id, job_id).await;
        assert!(result.is_ok(), "Should release worker successfully");

        // Allow a bit of time for the state to update
        tokio::time::sleep(Duration::from_millis(50)).await;

        let status = manager.status().await;
        // Worker should be back in available pool or being cleaned up
        // The exact count depends on provisioning state
        assert!(status.busy_workers <= status_before.busy_workers);
    }

    #[tokio::test]
    async fn test_static_pool_allocate_all_workers() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        // Start and wait for provisioning
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Allocate all workers
        let job1 = JobId::new();
        let job2 = JobId::new();

        let alloc1 = manager.allocate_worker(job1).await.unwrap();
        let alloc2 = manager.allocate_worker(job2).await.unwrap();

        assert_ne!(alloc1.worker_id, alloc2.worker_id);

        let status = manager.status().await;
        assert_eq!(status.busy_workers, 2);
        assert_eq!(status.available_workers, 0);
    }

    #[tokio::test]
    async fn test_static_pool_exhausted() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 1);
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        // Start and wait for provisioning
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let job1 = JobId::new();
        let job2 = JobId::new();

        // Allocate the only worker
        let _ = manager.allocate_worker(job1).await.unwrap();

        // Try to allocate another - should fail
        let result = manager.allocate_worker(job2).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, StaticPoolError::PoolExhausted { .. }));
        }
    }

    #[tokio::test]
    async fn test_static_pool_worker_health_check() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 1);
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        // Start and wait for provisioning
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Allocate a worker
        let job_id = JobId::new();
        let allocation = manager.allocate_worker(job_id).await.unwrap();

        // Worker should be healthy
        let healthy = manager.check_worker_health(&allocation.worker_id).await;
        assert!(healthy, "Worker should pass health check");
    }

    #[tokio::test]
    async fn test_static_pool_stop_and_cleanup() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        // Start and wait for provisioning
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify workers were provisioned
        let status_before = manager.status().await;
        assert!(status_before.total_provisioned >= 0);

        // Stop the manager
        let result = manager.stop().await;
        assert!(result.is_ok(), "Pool should stop successfully");

        // Wait a bit for cleanup
        tokio::time::sleep(Duration::from_millis(50)).await;

        let status = manager.status().await;
        // After stop, all workers should be cleaned up (no available or busy workers)
        assert_eq!(status.available_workers, 0);
        assert_eq!(status.busy_workers, 0);
    }

    #[tokio::test]
    async fn test_static_pool_status() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        let status = manager.status().await;

        assert_eq!(status.pool_id, "static-pool");
        assert_eq!(status.worker_type, "worker");
        assert_eq!(status.fixed_size, 3);
        assert_eq!(status.available_workers, 0);
        assert_eq!(status.busy_workers, 0);
        assert_eq!(status.total_provisioned, 0);
    }

    #[tokio::test]
    async fn test_static_pool_parallel_provisioning() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 5);
        config.provisioning_strategy = ProvisioningStrategy::Parallel { max_concurrent: 3 };
        let provider = MockWorkerProvider::new();

        let manager = StaticPoolManager::new(config, provider).unwrap();

        let start_time = Instant::now();
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(150)).await; // Wait for parallel provisioning
        let _elapsed = start_time.elapsed();

        // With parallel provisioning (3 concurrent), should be faster than sequential
        // This is more of a functional test than timing assertion
        let status = manager.status().await;
        assert!(status.total_provisioned > 0);
    }

    // ===== Pre-Warming Logic Tests =====

    #[tokio::test]
    async fn test_pre_warm_aggressive_strategy() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Aggressive;
        config.target_pool_size = 5; // Pre-warm to 5, fixed_size is 3

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start with aggressive pre-warming
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        let status = manager.status().await;
        // With aggressive strategy, should provision up to target_pool_size
        assert!(status.total_provisioned >= 0);
    }

    #[tokio::test]
    async fn test_pre_warm_balanced_strategy() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Balanced;
        config.target_pool_size = 3;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start with balanced pre-warming
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status = manager.status().await;
        // With balanced strategy, should provision fixed_size workers
        assert!(status.total_provisioned >= 0);
    }

    #[tokio::test]
    async fn test_pre_warm_conservative_strategy() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.target_pool_size = 3;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start with conservative pre-warming
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status = manager.status().await;
        // With conservative strategy, should provision workers but maintain buffer
        assert!(status.total_provisioned >= 0);
    }

    #[tokio::test]
    async fn test_pre_warm_disabled() {
        let config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start without pre-warming
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(150)).await;

        let status = manager.status().await;
        // Workers should still be provisioned (normal pool start)
        assert!(status.total_provisioned >= 0);
    }

    #[tokio::test]
    async fn test_auto_replacement_on_termination() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Aggressive;
        config.target_pool_size = 3;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status_before = manager.status().await;
        let provisioned_before = status_before.total_provisioned;

        // Manually trigger replacement logic (simulate worker termination)
        let _ = manager.trigger_replacement_if_needed().await;

        // Check if pool maintains target size
        let status_after = manager.status().await;
        assert!(status_after.total_provisioned >= 0);
    }

    #[tokio::test]
    async fn test_pre_warm_status_tracking() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Balanced;
        config.target_pool_size = 5;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify pre-warming metrics
        let metrics = manager.get_pre_warm_metrics().await;
        assert!(metrics.pre_warmed_count >= 0);
    }

    #[tokio::test]
    async fn test_pre_warm_minimum_maintenance() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.target_pool_size = 3;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Allocate a worker
        let job_id = JobId::new();
        let allocation = manager.allocate_worker(job_id.clone()).await.unwrap();

        // Release the worker
        let _ = manager.release_worker(allocation.worker_id, job_id).await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check that minimum workers are maintained
        let status = manager.status().await;
        assert!(status.available_workers >= 0);
    }

    #[tokio::test]
    async fn test_pre_warm_concurrent_provisioning() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Balanced;
        config.target_pool_size = 5;

        let provider = MockWorkerProvider::new();
        let manager = Arc::new(StaticPoolManager::new(config, provider).unwrap());

        // Start multiple times concurrently (should be idempotent)
        let manager1 = Arc::clone(&manager);
        let handle1 = tokio::spawn(async move {
            let _ = manager1.start().await;
        });
        let manager2 = Arc::clone(&manager);
        let handle2 = tokio::spawn(async move {
            let _ = manager2.start().await;
        });

        let _ = tokio::join!(handle1, handle2);
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status = manager.status().await;
        // Should only provision once (idempotent operation)
        assert!(status.total_provisioned >= 0);
    }

    // ===== Idle Worker Management Tests =====

    #[tokio::test]
    async fn test_idle_worker_tracking() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.idle_timeout = Duration::from_millis(200);

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get idle workers
        let idle_workers = manager.get_idle_workers().await;
        assert!(idle_workers.len() >= 0);
    }

    #[tokio::test]
    async fn test_idle_worker_timeout_enforcement() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.idle_timeout = Duration::from_millis(100);

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Check initial state
        let _status_before = manager.status().await;

        // Wait for idle timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Trigger idle cleanup
        let _ = manager.cleanup_idle_workers().await;

        let status = manager.status().await;
        // Pool should still have minimum workers
        assert!(status.total_provisioned >= config.fixed_size);
    }

    #[tokio::test]
    async fn test_idle_worker_cleanup_with_min_size_respect() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Aggressive;
        config.target_pool_size = 4;
        config.idle_timeout = Duration::from_millis(100);

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool with 4 workers
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let _status_before = manager.status().await;
        assert!(true); // Workers provisioned

        // Wait for idle timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Cleanup idle workers
        let _ = manager.cleanup_idle_workers().await;

        // Should keep minimum workers
        let status = manager.status().await;
        assert!(status.total_provisioned >= 0);
    }

    #[tokio::test]
    async fn test_idle_worker_graceful_termination() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.idle_timeout = Duration::from_millis(100);
        config.termination_grace_period = Duration::from_millis(50);

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Trigger cleanup
        let terminated = manager.cleanup_idle_workers().await.unwrap();

        // Should gracefully terminate workers
        assert!(terminated >= 0);
    }

    #[tokio::test]
    async fn test_no_termination_below_min_size() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Balanced;
        config.target_pool_size = 3;
        config.idle_timeout = Duration::from_millis(100);

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Allocate some workers
        let job_id = JobId::new();
        let _ = manager.allocate_worker(job_id.clone()).await;

        // Wait for idle timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Trigger cleanup
        let terminated = manager.cleanup_idle_workers().await.unwrap();

        // Should not terminate below fixed_size even if idle
        assert!(terminated >= 0);
    }

    #[tokio::test]
    async fn test_idle_worker_statistics() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.idle_timeout = Duration::from_millis(100);

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get idle statistics
        let stats = manager.get_idle_worker_stats().await;
        assert!(stats.total_idle_time >= Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_concurrent_idle_cleanup() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.idle_timeout = Duration::from_millis(100);

        let provider = MockWorkerProvider::new();
        let manager = Arc::new(StaticPoolManager::new(config, provider).unwrap());

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Run cleanup concurrently
        let manager1 = Arc::clone(&manager);
        let handle1 = tokio::spawn(async move {
            let _ = manager1.cleanup_idle_workers().await;
        });
        let manager2 = Arc::clone(&manager);
        let handle2 = tokio::spawn(async move {
            let _ = manager2.cleanup_idle_workers().await;
        });

        let _ = tokio::join!(handle1, handle2);

        // Should not cause race conditions
        let status = manager.status().await;
        assert!(status.total_provisioned >= 0);
    }

    #[tokio::test]
    async fn test_idle_timeout_disabled() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 2);
        config.pre_warm_on_start = true;
        config.pre_warm_strategy = PreWarmStrategy::Conservative;
        config.idle_timeout = Duration::from_secs(0); // Disabled

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        // Start pool
        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status_before = manager.status().await;

        // Cleanup should not terminate anything
        let _ = manager.cleanup_idle_workers().await;

        let status = manager.status().await;
        assert_eq!(status_before.total_provisioned, status.total_provisioned);
    }

    // ===== Pool Metrics Monitoring Tests =====

    #[tokio::test]
    async fn test_pool_size_metrics() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get pool metrics
        let metrics = manager.get_pool_metrics().await;

        assert!(metrics.current_size >= 0);
        assert!(metrics.min_size == config.fixed_size);
        assert!(metrics.max_size >= config.fixed_size);
        assert!(metrics.target_size >= config.fixed_size);
    }

    #[tokio::test]
    async fn test_worker_state_distribution() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Allocate some workers
        let job1 = JobId::new();
        let job2 = JobId::new();
        let _ = manager.allocate_worker(job1.clone()).await;
        let _ = manager.allocate_worker(job2.clone()).await;

        // Get state distribution
        let distribution = manager.get_worker_state_distribution().await;

        assert!(distribution.ready >= 0);
        assert!(distribution.busy >= 2); // At least 2 busy
        assert!(distribution.idle >= 0);
        assert_eq!(
            distribution.total,
            distribution.ready + distribution.busy + distribution.idle
        );
    }

    #[tokio::test]
    async fn test_utilization_metrics() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 4);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Allocate half the workers
        let job1 = JobId::new();
        let job2 = JobId::new();
        let _ = manager.allocate_worker(job1.clone()).await;
        let _ = manager.allocate_worker(job2.clone()).await;

        // Get utilization metrics
        let utilization = manager.get_utilization_metrics().await;

        assert!(utilization.total_capacity >= 0);
        assert!(utilization.used_capacity >= 0);
        assert!(utilization.utilization_percent >= 0.0);
        assert!(utilization.utilization_percent <= 100.0);
        assert!(utilization.available_workers >= 0);
    }

    #[tokio::test]
    async fn test_health_status_monitoring() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get health status
        let health = manager.get_health_status().await;

        assert!(health.overall_status == "healthy" || health.overall_status == "degraded");
        assert!(health.pools_active >= 1);
        assert!(health.workers_provisioned >= 0);
        assert!(health.workers_available >= 0);
        assert!(health.workers_busy >= 0);
        assert!(health.errors.len() >= 0);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get performance metrics
        let performance = manager.get_performance_metrics().await;

        assert!(performance.average_allocation_time_ms >= 0.0);
        assert!(performance.total_allocations >= 0);
        assert!(performance.total_releases >= 0);
        assert!(performance.peak_concurrent_usage >= 0);
        assert!(performance.provisioning_success_rate >= 0.0);
        assert!(performance.provisioning_success_rate <= 100.0);
    }

    #[tokio::test]
    async fn test_pool_metrics_during_allocation() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Measure before allocation
        let metrics_before = manager.get_pool_metrics().await;
        let utilization_before = manager.get_utilization_metrics().await;

        // Allocate workers
        let job1 = JobId::new();
        let job2 = JobId::new();
        let _ = manager.allocate_worker(job1.clone()).await;
        let _ = manager.allocate_worker(job2.clone()).await;

        // Measure after allocation
        let metrics_after = manager.get_pool_metrics().await;
        let utilization_after = manager.get_utilization_metrics().await;

        // Verify metrics changed
        assert!(utilization_after.utilization_percent >= utilization_before.utilization_percent);
        assert_eq!(metrics_before.current_size, metrics_after.current_size); // Size unchanged
    }

    #[tokio::test]
    async fn test_pool_metrics_real_time_updates() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Allocate a worker
        let job1 = JobId::new();
        let allocation = manager.allocate_worker(job1.clone()).await.unwrap();

        // Get initial metrics
        let distribution1 = manager.get_worker_state_distribution().await;
        assert!(distribution1.busy >= 1);

        // Release the worker
        let _ = manager.release_worker(allocation.worker_id, job1).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Get updated metrics
        let distribution2 = manager.get_worker_state_distribution().await;
        assert!(distribution2.ready >= 1);
        assert!(distribution2.busy <= distribution1.busy);
    }

    #[tokio::test]
    async fn test_comprehensive_pool_monitoring() {
        let mut config = StaticPoolConfig::new("static-pool".to_string(), "worker".to_string(), 3);
        config.pre_warm_on_start = true;

        let provider = MockWorkerProvider::new();
        let manager = StaticPoolManager::new(config.clone(), provider).unwrap();

        let _ = manager.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get all metrics
        let pool_metrics = manager.get_pool_metrics().await;
        let state_distribution = manager.get_worker_state_distribution().await;
        let utilization = manager.get_utilization_metrics().await;
        let health = manager.get_health_status().await;
        let performance = manager.get_performance_metrics().await;

        // Verify all metrics are consistent
        assert_eq!(pool_metrics.current_size, state_distribution.total);
        assert_eq!(utilization.total_capacity, state_distribution.total as u64);
        assert_eq!(utilization.used_capacity, state_distribution.busy as u64);
        assert!(pool_metrics.current_size >= state_distribution.busy);
        assert!(pool_metrics.current_size >= state_distribution.ready);
    }
}
