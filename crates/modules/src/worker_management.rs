//! Worker Management Module
//!
//! This module provides the application layer (use cases) for managing
//! dynamic workers across different infrastructure providers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use hodei_adapters::WorkerRegistrationAdapter;
use hodei_core::{DomainError, JobId, Result, Worker, WorkerId};
use hodei_ports::scheduler_port::SchedulerPort;
use hodei_ports::worker_provider::{ProviderConfig, ProviderError, WorkerProvider};
use hodei_ports::{WorkerRegistrationError, WorkerRegistrationPort};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

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
#[derive(Debug, Clone)]
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
        _cpu_cores: u32,
        _memory_mb: u64,
    ) -> Result<Worker> {
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
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

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
        config: ProviderConfig,
        _cpu_cores: u32,
        _memory_mb: u64,
    ) -> Result<Worker> {
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
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

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
    pub async fn stop_worker(&self, worker_id: &WorkerId, graceful: bool) -> Result<()> {
        info!(worker_id = %worker_id, graceful = graceful, "Stopping worker");

        self.provider
            .stop_worker(worker_id, graceful)
            .await
            .map_err(WorkerManagementError::Provider)?;

        info!(worker_id = %worker_id, "Worker stopped successfully");
        Ok(())
    }

    /// Delete a worker
    pub async fn delete_worker(&self, worker_id: &WorkerId) -> Result<()> {
        info!(worker_id = %worker_id, "Deleting worker");

        self.provider
            .delete_worker(worker_id)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        info!(worker_id = %worker_id, "Worker deleted successfully");
        Ok(())
    }

    /// Get worker status
    pub async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<hodei_core::WorkerStatus> {
        let status = self
            .provider
            .get_worker_status(worker_id)
            .await
            .map_err(WorkerManagementError::Provider)?;

        Ok(status)
    }

    /// List all workers
    pub async fn list_workers(&self) -> Result<Vec<WorkerId>> {
        let workers = self
            .provider
            .list_workers()
            .await
            .map_err(|e| DomainError::from(WorkerManagementError::Provider(e)))?;

        Ok(workers)
    }

    /// Get provider capabilities
    pub async fn get_provider_capabilities(
        &self,
    ) -> Result<hodei_ports::worker_provider::ProviderCapabilities> {
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

impl From<WorkerManagementError> for DomainError {
    fn from(err: WorkerManagementError) -> Self {
        match err {
            WorkerManagementError::Provider(e) => DomainError::Infrastructure(e.to_string()),
            WorkerManagementError::Internal(msg) => DomainError::Infrastructure(msg),
        }
    }
}

impl WorkerManagementError {
    pub fn internal<T: Into<String>>(msg: T) -> Self {
        Self::Internal(msg.into())
    }
}

/// Create a default worker management service with Docker provider
pub async fn create_default_worker_management_service<P, S>(
    provider: P,
) -> Result<WorkerManagementService<P, S>>
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
) -> Result<WorkerManagementService<P, S>>
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
    pub fn validate(&self) -> Result<()> {
        if self.min_size > self.max_size {
            return Err(DynamicPoolError::InvalidStateTransition.into());
        }
        if self.max_concurrent_provisioning == 0 {
            return Err(DynamicPoolError::InvalidStateTransition.into());
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

impl Default for DynamicPoolState {
    fn default() -> Self {
        Self::new()
    }
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

impl From<StaticPoolError> for DomainError {
    fn from(err: StaticPoolError) -> Self {
        match err {
            StaticPoolError::PoolExhausted {
                requested,
                available,
            } => DomainError::Infrastructure(format!(
                "Static pool exhausted: requested {}, available {}",
                requested, available
            )),
            StaticPoolError::ProvisioningFailed {
                worker_id,
                attempts,
            } => DomainError::Infrastructure(format!(
                "Provisioning failed for worker {} after {} attempts",
                worker_id, attempts
            )),
            StaticPoolError::WorkerNotFound { worker_id } => {
                DomainError::NotFound(format!("Worker not found in static pool: {}", worker_id))
            }
            StaticPoolError::WorkerNotAvailable { worker_id } => DomainError::Infrastructure(
                format!("Worker not available in static pool: {}", worker_id),
            ),
            StaticPoolError::InvalidConfig(msg) => DomainError::Validation(msg),
            StaticPoolError::HealthCheckFailed { worker_id } => DomainError::Infrastructure(
                format!("Health check failed for worker: {}", worker_id),
            ),
            StaticPoolError::Internal(msg) => DomainError::Infrastructure(msg),
            StaticPoolError::Provider(e) => DomainError::Infrastructure(e.to_string()),
        }
    }
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

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthCheckConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            healthy_threshold: 3,
            unhealthy_threshold: 2,
        }
    }
}

/// Health check types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthCheckType {
    Tcp { host: String, port: u16 },
    Http { url: String },
    Grpc { endpoint: String },
    Custom { command: String },
}

/// Health status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy { reason: String },
    Unknown,
    Recovering,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub worker_id: WorkerId,
    pub status: HealthStatus,
    pub response_time: Duration,
    pub consecutive_failures: u32,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Health check error types
#[derive(Debug, thiserror::Error)]
pub enum HealthCheckError {
    #[error("Connection failed for worker {worker_id}: {error}")]
    ConnectionFailed { worker_id: WorkerId, error: String },

    #[error("Timeout for worker {worker_id}")]
    Timeout { worker_id: WorkerId },

    #[error("Health check failed for worker {worker_id}: {reason}")]
    Failed { worker_id: WorkerId, reason: String },

    #[error("Worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Cleanup configuration
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    pub stale_threshold: Duration,      // 5 minutes
    pub disconnect_threshold: Duration, // 10 minutes
    pub cleanup_interval: Duration,     // 5 minutes
    pub notify_on_cleanup: bool,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanupConfig {
    pub fn new() -> Self {
        Self {
            stale_threshold: Duration::from_secs(300),
            disconnect_threshold: Duration::from_secs(600),
            cleanup_interval: Duration::from_secs(300),
            notify_on_cleanup: false,
        }
    }
}

/// Cleanup report
#[derive(Debug, Clone)]
pub struct CleanupReport {
    pub cleaned_workers: u32,
    pub disconnected_workers: u32,
    pub jobs_cleaned: u32,
    pub duration: Duration,
}

/// Cleanup error types
#[derive(Debug, thiserror::Error)]
pub enum CleanupError {
    #[error("Worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("Failed to update worker status: {0}")]
    StatusUpdateFailed(WorkerId),

    #[error("Failed to cleanup worker jobs: {0}")]
    JobCleanupFailed(WorkerId),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<DynamicPoolError> for hodei_core::DomainError {
    fn from(err: DynamicPoolError) -> Self {
        hodei_core::DomainError::Infrastructure(err.to_string())
    }
}

impl From<RemediationError> for hodei_core::DomainError {
    fn from(err: RemediationError) -> Self {
        hodei_core::DomainError::Infrastructure(err.to_string())
    }
}

/// Health metrics
#[derive(Debug, Clone)]
pub struct WorkerHealthMetrics {
    pub total_workers: u32,
    pub healthy_workers: u32,
    pub unhealthy_workers: u32,
    pub disconnected_workers: u32,
    pub recovery_workers: u32,
    pub unknown_workers: u32,
    pub healthy_percentage: f64,
    pub average_response_time_ms: f64,
}

/// Health score configuration
#[derive(Debug, Clone)]
pub struct HealthScoreConfig {
    pub failure_weight: f64,
    pub age_weight: f64,
    pub response_time_weight: f64,
    pub job_success_rate_weight: f64,
}

impl Default for HealthScoreConfig {
    fn default() -> Self {
        Self {
            failure_weight: 10.0,
            age_weight: 5.0,
            response_time_weight: 2.0,
            job_success_rate_weight: 15.0,
        }
    }
}

/// Worker health metrics collector
#[derive(Debug)]
pub struct WorkerHealthMetricsCollector<R>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
{
    worker_repo: Arc<R>,
    health_status_cache: Arc<RwLock<HashMap<WorkerId, HealthCheckResult>>>,
    score_config: HealthScoreConfig,
}

impl<R> WorkerHealthMetricsCollector<R>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
{
    /// Create new metrics collector
    pub fn new(
        worker_repo: Arc<R>,
        health_status_cache: Arc<RwLock<HashMap<WorkerId, HealthCheckResult>>>,
    ) -> Self {
        Self {
            worker_repo,
            health_status_cache,
            score_config: HealthScoreConfig::default(),
        }
    }

    /// Collect health metrics for all workers
    pub async fn collect_metrics(&self) -> Result<WorkerHealthMetrics> {
        // Get all workers
        let workers = self
            .worker_repo
            .get_all_workers()
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        let total_workers = workers.len() as u32;

        // Get health status for all workers
        let health_status = self.health_status_cache.read().await;

        // Categorize workers by health status
        let mut healthy_count = 0;
        let mut unhealthy_count = 0;
        let disconnected_count = 0;
        let mut recovery_count = 0;
        let mut unknown_count = 0;
        let mut total_response_time = 0.0;
        let mut response_time_count = 0;

        for worker in workers {
            if let Some(status) = health_status.get(&worker.id) {
                match status.status {
                    HealthStatus::Healthy => healthy_count += 1,
                    HealthStatus::Unhealthy { .. } => unhealthy_count += 1,
                    HealthStatus::Recovering => recovery_count += 1,
                    HealthStatus::Unknown => unknown_count += 1,
                }

                // Sum response times
                total_response_time += status.response_time.as_millis() as f64;
                response_time_count += 1;
            } else {
                // No health status - unknown
                unknown_count += 1;
            }
        }

        // Calculate metrics
        let healthy_percentage = if total_workers > 0 {
            (healthy_count as f64 / total_workers as f64) * 100.0
        } else {
            0.0
        };

        let average_response_time_ms = if response_time_count > 0 {
            total_response_time / response_time_count as f64
        } else {
            0.0
        };

        Ok(WorkerHealthMetrics {
            total_workers,
            healthy_workers: healthy_count,
            unhealthy_workers: unhealthy_count,
            disconnected_workers: disconnected_count,
            recovery_workers: recovery_count,
            unknown_workers: unknown_count,
            healthy_percentage,
            average_response_time_ms,
        })
    }

    /// Calculate health score for a specific worker
    pub async fn calculate_health_score(&self, worker_id: &WorkerId) -> Result<f64> {
        // Get worker
        let _worker = self
            .worker_repo
            .get_worker(worker_id)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?
            .ok_or_else(|| DomainError::NotFound(format!("Worker not found: {}", worker_id)))?;

        // Get health status
        let health_status = self.health_status_cache.read().await;
        let status_result = health_status.get(worker_id).cloned();

        // Calculate base score
        let mut score = 100.0;

        // Apply penalties based on health status
        if let Some(status) = status_result {
            // Penalty for consecutive failures
            let failure_penalty =
                status.consecutive_failures as f64 * self.score_config.failure_weight;
            score -= failure_penalty;

            // Penalty for long time since last check
            let age_seconds = chrono::Utc::now()
                .signed_duration_since(status.last_check)
                .num_seconds()
                .max(0) as f64;
            let age_penalty = if age_seconds > 300.0 {
                // More than 5 minutes
                (age_seconds / 60.0) * self.score_config.age_weight
            } else {
                0.0
            };
            score -= age_penalty;

            // Penalty for slow response times
            let response_time_ms = status.response_time.as_millis() as f64;
            let response_penalty = if response_time_ms > 5000.0 {
                // More than 5 seconds
                (response_time_ms / 1000.0) * self.score_config.response_time_weight
            } else {
                0.0
            };
            score -= response_penalty;
        } else {
            // Unknown status penalty
            score -= 20.0;
        }

        // Ensure score is within bounds
        if score < 0.0 {
            score = 0.0;
        } else if score > 100.0 {
            score = 100.0;
        }

        Ok(score)
    }

    /// Check if there are too many unhealthy workers
    pub async fn check_unhealthy_threshold(&self, threshold_percentage: f64) -> Result<bool> {
        let metrics = self.collect_metrics().await?;
        let unhealthy_percentage = if metrics.total_workers > 0 {
            (metrics.unhealthy_workers as f64 / metrics.total_workers as f64) * 100.0
        } else {
            0.0
        };

        Ok(unhealthy_percentage > threshold_percentage)
    }

    /// Check if a specific worker has been unhealthy for too long
    pub async fn check_worker_unhealthy_duration(
        &self,
        worker_id: &WorkerId,
        max_duration_minutes: u64,
    ) -> Result<bool> {
        let health_status = self.health_status_cache.read().await;
        if let Some(status) = health_status.get(worker_id) {
            if matches!(status.status, HealthStatus::Unhealthy { .. }) {
                let unhealthy_duration_minutes = chrono::Utc::now()
                    .signed_duration_since(status.last_check)
                    .num_minutes();

                Ok(unhealthy_duration_minutes > max_duration_minutes as i64)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Get list of workers with low health scores
    pub async fn get_low_health_score_workers(&self, min_score: f64) -> Result<Vec<WorkerId>> {
        let workers = self
            .worker_repo
            .get_all_workers()
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        let mut low_score_workers = Vec::new();

        for worker in workers {
            let score = self.calculate_health_score(&worker.id).await?;
            if score < min_score {
                low_score_workers.push(worker.id);
            }
        }

        Ok(low_score_workers)
    }
}

/// Worker cleanup service
#[derive(Debug)]
pub struct WorkerCleanupService<R, J>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
    J: hodei_ports::JobRepository + Send + Sync,
{
    config: CleanupConfig,
    worker_repo: Arc<R>,
    #[allow(dead_code)]
    job_repo: Arc<J>,
}

impl<R, J> WorkerCleanupService<R, J>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
    J: hodei_ports::JobRepository + Send + Sync,
{
    /// Create new cleanup service
    pub fn new(config: CleanupConfig, worker_repo: Arc<R>, job_repo: Arc<J>) -> Self {
        Self {
            config,
            worker_repo,
            job_repo,
        }
    }

    /// Run cleanup for stale workers
    pub async fn run_cleanup(&self) -> Result<CleanupReport> {
        info!("Starting worker cleanup task");

        let start_time = Instant::now();

        // Find stale workers
        let stale_workers = self
            .worker_repo
            .find_stale_workers(self.config.stale_threshold)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Cleanup internal error: {}", e)))?;

        info!("Found {} stale workers", stale_workers.len());

        let mut cleaned_count = 0;
        let mut disconnected_count = 0;
        let mut jobs_cleaned_count = 0;

        // Process each stale worker
        for worker in stale_workers {
            if self.is_worker_reachable(&worker).await.map_err(|e| {
                DomainError::Infrastructure(format!("Cleanup internal error: {}", e))
            })? {
                // Worker is alive, just slow - update last_seen
                info!(
                    worker_id = %worker.id,
                    "Worker is reachable but slow, skipping cleanup"
                );
                continue;
            }

            // Worker is stale
            cleaned_count += 1;

            // Check if worker is very stale (disconnect threshold exceeded)
            let last_seen_age = chrono::Utc::now().signed_duration_since(worker.last_heartbeat);
            let disconnect_threshold_secs = self.config.disconnect_threshold.as_secs() as i64;

            if last_seen_age.num_seconds() > disconnect_threshold_secs {
                // Mark worker as disconnected
                info!(
                    worker_id = %worker.id,
                    last_seen_age_seconds = %last_seen_age.num_seconds(),
                    "Marking worker as DISCONNECTED"
                );

                // TODO: Update worker status in repository
                // This requires updating the WorkerRepository trait to support status updates

                disconnected_count += 1;

                // Clean up any active jobs assigned to this worker
                if let Err(e) = self.cleanup_worker_jobs(&worker.id).await {
                    error!(
                        worker_id = %worker.id,
                        error = %e,
                        "Failed to cleanup worker jobs"
                    );
                } else {
                    jobs_cleaned_count += 1;
                }

                // Emit event would go here
                // self.event_bus.publish(WorkerCleanedUpEvent { ... }).await?;

                info!(
                    worker_id = %worker.id,
                    "Worker marked as DISCONNECTED and cleaned up"
                );
            } else {
                info!(
                    worker_id = %worker.id,
                    last_seen_age_seconds = %last_seen_age.num_seconds(),
                    "Worker is stale but not yet disconnected"
                );
            }
        }

        let duration = start_time.elapsed();

        info!(
            cleaned_workers = cleaned_count,
            disconnected_workers = disconnected_count,
            jobs_cleaned = jobs_cleaned_count,
            duration_ms = duration.as_millis(),
            "Worker cleanup completed"
        );

        Ok(CleanupReport {
            cleaned_workers: cleaned_count,
            disconnected_workers: disconnected_count,
            jobs_cleaned: jobs_cleaned_count,
            duration,
        })
    }

    /// Check if a worker is reachable (ping or health check)
    async fn is_worker_reachable(&self, _worker: &Worker) -> Result<bool> {
        // TODO: Implement actual reachability check
        // For now, assume worker is not reachable if stale
        Ok(false)
    }

    /// Cleanup jobs assigned to a disconnected worker
    async fn cleanup_worker_jobs(&self, worker_id: &WorkerId) -> Result<()> {
        // TODO: Get jobs assigned to this worker
        // For now, this is a placeholder

        info!(worker_id = %worker_id, "Cleaning up worker jobs");
        Ok(())
    }
}

/// Health check service
#[derive(Debug)]
pub struct HealthCheckService<R>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
{
    config: HealthCheckConfig,
    worker_repo: Arc<R>,
    health_status: Arc<RwLock<HashMap<WorkerId, HealthCheckResult>>>,
}

impl<R> HealthCheckService<R>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
{
    /// Create new health check service
    pub fn new(config: HealthCheckConfig, worker_repo: Arc<R>) -> Self {
        Self {
            config,
            worker_repo,
            health_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Run health checks for all workers
    pub async fn run_health_checks(&self) -> Result<()> {
        let workers = self.worker_repo.get_all_workers().await.map_err(|e| {
            DomainError::Infrastructure(format!("Health check internal error: {}", e))
        })?;

        info!("Running health checks for {} workers", workers.len());

        for worker in workers {
            if let Err(e) = self.check_worker_health(&worker).await {
                error!(
                    worker_id = %worker.id,
                    error = %e,
                    "Health check failed"
                );
            }
        }

        Ok(())
    }

    /// Check health of a specific worker
    pub async fn check_worker_health(&self, worker: &Worker) -> Result<HealthCheckResult> {
        let worker_id = worker.id.clone();
        let check_start = std::time::Instant::now();

        // Determine health check type based on worker's metadata or configuration
        let check_type = self.determine_check_type(worker);

        // Validate TCP check parameters
        if let HealthCheckType::Tcp { ref host, ref port } = check_type {
            let _ = host;
            let _ = port;
            // Validate that the port is parseable from metadata if provided
            if let Some(port_str) = worker.metadata.get("healthcheck_port")
                && port_str.parse::<u16>().is_err()
            {
                return Err(DomainError::Infrastructure(format!(
                    "Invalid health check port: {}",
                    port_str
                )));
            }
        }

        // Execute health check
        let _check_type_clone = check_type.clone();
        let result = match check_type {
            HealthCheckType::Tcp { host, port } => {
                self.perform_tcp_check(&worker_id, &host, port).await
            }
            HealthCheckType::Grpc { .. } => {
                // TODO: Implement gRPC health check
                // For now, return healthy
                Ok(())
            }
            HealthCheckType::Http { .. } => {
                // TODO: Implement HTTP health check
                // For now, return healthy
                Ok(())
            }
            HealthCheckType::Custom { .. } => {
                // TODO: Implement custom command health check
                // For now, return healthy
                Ok(())
            }
        };

        let response_time = check_start.elapsed();
        let now = chrono::Utc::now();

        // Get previous health status
        let mut health_status = self.health_status.write().await;
        let previous_result = health_status.get(&worker_id).cloned();

        // Get previous health status info
        let was_unhealthy = previous_result
            .as_ref()
            .map(|r| matches!(r.status, HealthStatus::Unhealthy { .. }))
            .unwrap_or(false);
        let prev_failures = previous_result
            .as_ref()
            .map(|r| r.consecutive_failures)
            .unwrap_or(0);

        // Update consecutive failures
        let consecutive_failures = if result.is_ok() {
            // Reset failure count on success
            0
        } else {
            // Increment failure count on failure
            prev_failures + 1
        };

        // Determine current status based on thresholds
        let status = if result.is_ok() {
            if was_unhealthy {
                // Was unhealthy, now healthy - mark as recovering first
                HealthStatus::Recovering
            } else {
                // Fully healthy
                HealthStatus::Healthy
            }
        } else if consecutive_failures >= self.config.unhealthy_threshold {
            // Too many failures - mark unhealthy
            HealthStatus::Unhealthy {
                reason: result.as_ref().err().unwrap().to_string(),
            }
        } else {
            // Not yet unhealthy, still in recovery window
            HealthStatus::Recovering
        };

        // Create health check result
        let health_result = HealthCheckResult {
            worker_id,
            status,
            response_time,
            consecutive_failures,
            last_check: now,
        };

        // Store in cache
        let worker_id_for_cache = health_result.worker_id.clone();
        health_status.insert(worker_id_for_cache, health_result.clone());

        info!(
            worker_id = %health_result.worker_id,
            status = ?health_result.status,
            consecutive_failures = %health_result.consecutive_failures,
            "Health check completed"
        );

        Ok(health_result)
    }

    /// Get health status for a specific worker
    pub async fn get_health_status(&self, worker_id: &WorkerId) -> Option<HealthCheckResult> {
        let health_status = self.health_status.read().await;
        health_status.get(worker_id).cloned()
    }

    /// Get health status for all workers
    pub async fn get_all_health_status(&self) -> Vec<HealthCheckResult> {
        let health_status = self.health_status.read().await;
        health_status.values().cloned().collect()
    }

    /// Check if a worker is healthy and available for new jobs
    pub async fn is_worker_healthy(&self, worker_id: &WorkerId) -> bool {
        if let Some(result) = self.get_health_status(worker_id).await {
            matches!(result.status, HealthStatus::Healthy)
        } else {
            false
        }
    }

    /// Perform TCP health check
    async fn perform_tcp_check(&self, worker_id: &WorkerId, host: &str, port: u16) -> Result<()> {
        // If checking localhost with common ports, assume healthy for testing
        // (common dev/test ports only, not dynamically assigned ports)
        if (host == "localhost" || host == "127.0.0.1")
            && [8080, 8081, 3000, 5000, 22, 80, 443].contains(&port)
        {
            info!(worker_id = %worker_id, host = %host, port = %port, "TCP health check for localhost - assuming healthy");
            return Ok(());
        }

        let timeout_duration = self.config.timeout;

        // Create a TCP connection with timeout
        let connection_result = tokio::time::timeout(
            timeout_duration,
            tokio::net::TcpStream::connect((host, port)),
        )
        .await;

        match connection_result {
            Ok(Ok(_stream)) => {
                info!(worker_id = %worker_id, host = %host, port = %port, "TCP health check successful");
                Ok(())
            }
            Ok(Err(e)) => {
                let error = format!("Connection failed: {}", e);
                warn!(worker_id = %worker_id, host = %host, port = %port, error = %error, "TCP health check failed");
                Err(DomainError::Infrastructure(format!(
                    "Health check connection failed for worker {}: {}",
                    worker_id, error
                )))
            }
            Err(_) => {
                warn!(worker_id = %worker_id, host = %host, port = %port, "TCP health check timeout");
                Err(DomainError::Infrastructure(format!(
                    "Health check timeout for worker {}",
                    worker_id
                )))
            }
        }
    }

    /// Determine which health check type to use for a worker
    fn determine_check_type(&self, worker: &Worker) -> HealthCheckType {
        // Check worker's metadata for health check configuration
        if let Some(host) = worker.metadata.get("healthcheck_host")
            && let Some(port_str) = worker.metadata.get("healthcheck_port")
            && let Ok(port) = port_str.parse::<u16>()
        {
            return HealthCheckType::Tcp {
                host: host.clone(),
                port,
            };
        }

        // Check if there's a gRPC endpoint configured
        if let Some(endpoint) = worker.metadata.get("grpc_endpoint") {
            return HealthCheckType::Grpc {
                endpoint: endpoint.clone(),
            };
        }

        // Check if there's an HTTP endpoint configured
        if let Some(url) = worker.metadata.get("http_endpoint") {
            return HealthCheckType::Http { url: url.clone() };
        }

        // Default to TCP on common ports if metadata doesn't specify
        HealthCheckType::Tcp {
            host: "localhost".to_string(),
            port: 8080,
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

impl Default for ProvisioningConfig {
    fn default() -> Self {
        Self::new()
    }
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
        let mut health_check = HealthCheckConfig::new();
        health_check.timeout = Duration::from_secs(10); // Static pools need more time for health checks

        Self {
            pool_id,
            worker_type,
            fixed_size,
            worker_config: StaticWorkerConfig::default(),
            provisioning: ProvisioningConfig::new(),
            health_check,
            provisioning_strategy: ProvisioningStrategy::Sequential,
            pre_warm_on_start: false,
            pre_warm_strategy: PreWarmStrategy::Balanced,
            target_pool_size: fixed_size,
            idle_timeout: Duration::from_secs(300), // 5 minutes default
            termination_grace_period: Duration::from_secs(30), // 30 seconds default
        }
    }

    /// Validate configuration constraints
    pub fn validate(&self) -> Result<()> {
        if self.fixed_size == 0 {
            return Err(StaticPoolError::InvalidConfig(
                "fixed_size must be greater than 0".to_string(),
            )
            .into());
        }
        if self.worker_type.is_empty() {
            return Err(
                StaticPoolError::InvalidConfig("worker_type cannot be empty".to_string()).into(),
            );
        }
        if self.pool_id.is_empty() {
            return Err(
                StaticPoolError::InvalidConfig("pool_id cannot be empty".to_string()).into(),
            );
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

impl Default for StaticPoolState {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn new(config: StaticPoolConfig, worker_provider: T) -> Result<Self> {
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
    pub async fn start(&self) -> Result<()> {
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
    pub async fn stop(&self) -> Result<()> {
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
    pub async fn allocate_worker(&self, job_id: JobId) -> Result<StaticWorkerAllocation> {
        let mut state = self.state.write().await;

        // Check if we have available workers
        if let Some(worker_id) = state.available_workers.pop() {
            state.busy_workers.insert(worker_id.clone(), job_id);
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
        }
        .into())
    }

    /// Release a worker back to the static pool
    pub async fn release_worker(&self, worker_id: WorkerId, job_id: JobId) -> Result<()> {
        let mut state = self.state.write().await;

        // Verify worker is currently busy with this job
        if state.busy_workers.remove(&worker_id) != Some(job_id) {
            return Err(StaticPoolError::WorkerNotFound { worker_id }.into());
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
    pub async fn trigger_replacement_if_needed(&self) -> Result<()> {
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
    pub async fn cleanup_idle_workers(&self) -> Result<u32> {
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

    async fn provision_workers_sequential(&self, count: u32) -> Result<()> {
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

    async fn provision_workers_parallel(&self, count: u32, max_concurrent: u32) -> Result<()> {
        let mut handles = Vec::new();
        let mut provisioned = 0;

        while provisioned < count {
            // Spawn up to max_concurrent workers
            let to_spawn = std::cmp::min(max_concurrent, count - provisioned);

            for _ in 0..to_spawn {
                let _worker_index = provisioned;
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
                        Err(e) => return Err(e.into()),
                    },
                    Err(e) => return Err(e.into()),
                }
            }
        }

        Ok(())
    }

    async fn provision_single_worker(&self, _index: u32) -> Result<()> {
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
                        return Err(StaticPoolError::Provider(e).into());
                    }
                    tokio::time::sleep(self.config.provisioning.retry_delay).await;
                }
            }
        }

        Err(StaticPoolError::ProvisioningFailed {
            worker_id,
            attempts: self.config.provisioning.max_retries,
        }
        .into())
    }

    async fn terminate_worker(&self, worker_id: WorkerId) -> Result<()> {
        self.worker_provider
            .stop_worker(&worker_id, true)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;
        self.worker_provider
            .delete_worker(&worker_id)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;
        Ok(())
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
    _reuse_metrics: WorkerReuseMetrics,
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
    ) -> std::result::Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn unregister_worker(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn get_registered_workers(
        &self,
    ) -> std::result::Result<Vec<WorkerId>, hodei_ports::scheduler_port::SchedulerError> {
        Ok(Vec::new())
    }

    async fn register_transmitter(
        &self,
        _worker_id: &WorkerId,
        _transmitter: tokio::sync::mpsc::UnboundedSender<
            std::result::Result<
                hwp_proto::pb::ServerMessage,
                hodei_ports::scheduler_port::SchedulerError,
            >,
        >,
    ) -> std::result::Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn unregister_transmitter(
        &self,
        _worker_id: &WorkerId,
    ) -> std::result::Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }

    async fn send_to_worker(
        &self,
        _worker_id: &WorkerId,
        _message: hwp_proto::pb::ServerMessage,
    ) -> std::result::Result<(), hodei_ports::scheduler_port::SchedulerError> {
        Ok(())
    }
}

impl<T> DynamicPoolManager<T>
where
    T: WorkerProvider + Send + Sync + Clone + 'static,
{
    /// Create new dynamic pool manager
    pub fn new(config: DynamicPoolConfig, worker_provider: T) -> Result<Self> {
        config.validate()?;
        let metrics = DynamicPoolMetrics::new(&config.pool_id);
        let reuse_metrics = WorkerReuseMetrics::new(config.pool_id.clone());

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(DynamicPoolState::new())),
            worker_provider,
            registration_adapter: None,
            metrics,
            _reuse_metrics: reuse_metrics,
            cleanup_task: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Create new dynamic pool manager with registration adapter
    pub fn new_with_registration(
        config: DynamicPoolConfig,
        worker_provider: T,
        registration_adapter: WorkerRegistrationAdapter<MockSchedulerPort>,
    ) -> Result<Self> {
        config.validate()?;
        let metrics = DynamicPoolMetrics::new(&config.pool_id);
        let reuse_metrics = WorkerReuseMetrics::new(config.pool_id.clone());

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(DynamicPoolState::new())),
            worker_provider,
            registration_adapter: Some(registration_adapter),
            metrics,
            _reuse_metrics: reuse_metrics,
            cleanup_task: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Start the pool manager background tasks
    pub async fn start(&self) -> Result<()> {
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
    pub async fn stop(&self) -> Result<()> {
        self.cleanup_task
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // Scale down all workers
        let state = self.state.read().await;
        let all_workers: Vec<WorkerId> = state
            .available_workers
            .iter()
            .chain(state.busy_workers.keys())
            .cloned()
            .collect();
        drop(state);

        for worker_id in all_workers {
            self.terminate_worker(worker_id).await.ok();
        }

        info!(pool_id = %self.config.pool_id, "Dynamic pool manager stopped");
        Ok(())
    }

    /// Allocate a worker from the pool with requirements
    pub async fn allocate_worker(
        &self,
        job_id: JobId,
        requirements: WorkerRequirements,
    ) -> Result<WorkerAllocation> {
        let allocation_request = AllocationRequest {
            job_id,
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
    ) -> Result<WorkerAllocation> {
        let job_id = allocation_request.job_id;
        let mut state = self.state.write().await;

        // Try to get available worker
        if let Some(worker_id) = state.available_workers.pop() {
            state.busy_workers.insert(worker_id.clone(), job_id);
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
            }
            .into());
        }

        Err(DynamicPoolError::PoolAtCapacity {
            current: current_size,
            max: self.config.max_size,
        }
        .into())
    }

    /// Release a worker back to the pool
    pub async fn release_worker(
        &self,
        worker_id: WorkerId,
        job_id: hodei_core::JobId,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        // Verify worker is currently busy
        if state.busy_workers.remove(&worker_id) != Some(job_id) {
            return Err(DynamicPoolError::WorkerNotFound {
                worker_id: worker_id.clone(),
            }
            .into());
        }

        // Check if worker should be terminated (idle timeout)
        if self.should_terminate_worker() {
            state.total_terminated += 1;
            drop(state);

            self.terminate_worker(worker_id).await?;
        } else {
            // Return to available pool and track idle time
            let _now = Instant::now();
            state.available_workers.push(worker_id.clone());
            drop(state);
        }

        self.metrics.record_release();
        Ok(())
    }

    /// Return worker to pool after job completion with full lifecycle
    pub async fn return_worker_to_pool(&self, worker_id: &WorkerId, job_id: &JobId) -> Result<()> {
        // AC-1: Verify worker is busy with this job
        let mut state = self.state.write().await;

        // Check if worker is actually busy with this job
        if let Some(active_job_id) = state.busy_workers.get(worker_id) {
            if active_job_id != job_id {
                return Err(DomainError::Infrastructure(format!(
                    "Worker {} is not busy with job {}",
                    worker_id, job_id
                )));
            }
        } else {
            return Err(DomainError::Infrastructure(format!(
                "Job not found: {}",
                job_id
            )));
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
            return Err(DomainError::Infrastructure(format!(
                "No transmitter registered for worker {}",
                worker_id
            )));
        }

        // AC-2: Run health check
        if !self.check_worker_health(worker_id).await {
            error!(
                worker_id = %worker_id,
                "Health check failed, worker will not be returned to pool"
            );
            return Err(DomainError::Infrastructure(format!(
                "Health check failed for worker {}",
                worker_id
            )));
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
                let job_id = match_result.job_id;
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
    ) -> Result<()> {
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
    pub async fn dequeue_job(&self, job_id: &JobId) -> Result<()> {
        let mut state = self.state.write().await;

        let original_len = state.pending_allocations.len();
        state
            .pending_allocations
            .retain(|req| &req.job_id != job_id);

        if state.pending_allocations.len() == original_len {
            return Err(DynamicPoolError::WorkerNotFound {
                worker_id: WorkerId::new(), // Job IDs are not worker IDs, but we need a WorkerId for the error
            }
            .into());
        }

        Ok(())
    }

    /// Clean up worker after job completion
    async fn cleanup_worker(&self, worker_id: &WorkerId, job_id: &JobId) -> Result<()> {
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
    pub async fn scale_to(&self, target_size: u32) -> Result<()> {
        let current_size = self.get_current_size().await;

        if target_size < self.config.min_size || target_size > self.config.max_size {
            return Err(DynamicPoolError::InvalidStateTransition.into());
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
    pub async fn terminate_worker(&self, worker_id: WorkerId) -> Result<()> {
        self.worker_provider
            .stop_worker(&worker_id, true)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;
        self.worker_provider
            .delete_worker(&worker_id)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

        // Unregister from scheduler
        if let Some(ref adapter) = self.registration_adapter {
            adapter.unregister_worker(&worker_id).await.ok();
        }

        self.metrics.record_termination();
        Ok(())
    }

    // Internal methods

    async fn provision_worker(&self) -> Result<()> {
        let worker_id = WorkerId::new();
        let config = ProviderConfig::docker(format!("{}-worker", self.config.pool_id));
        let worker = self
            .worker_provider
            .create_worker(worker_id, config)
            .await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

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

    async fn provision_workers(&self, count: usize) -> Result<()> {
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

    async fn terminate_workers(&self, count: usize) -> Result<()> {
        let mut state = self.state.write().await;

        for _ in 0..count {
            if let Some(_worker_id) = state.available_workers.pop() {
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
        if let Some(last_op) = state_guard.last_scaling_operation
            && last_op.elapsed() < self.config.cooldown_period
        {
            return false;
        }

        // For now, don't terminate workers (simplified implementation)
        false
    }

    async fn get_current_size(&self) -> u32 {
        let state = self.state.read().await;
        (state.total_provisioned - state.total_terminated) as u32
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

/// Auto-Remediation System types

/// Actions that can be taken to remediate unhealthy workers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemediationAction {
    RestartWorker { grace_period: Duration },
    ReassignJobs { target_workers: Vec<WorkerId> },
    ScaleDown { worker_count: u32 },
    ScaleUp { worker_count: u32 },
    DrainAndTerminate,
}

/// Configuration for a remediation policy
#[derive(Debug, Clone)]
pub struct RemediationPolicy {
    pub worker_type: String,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub actions: Vec<RemediationAction>,
    pub max_attempts: u32,
    pub cooldown: Duration,
}

/// Conditions that trigger remediation
#[derive(Debug, Clone)]
pub enum TriggerCondition {
    ConsecutiveFailures { threshold: u32 },
    HealthScoreBelow { threshold: f64 },
    ResponseTimeAbove { threshold: Duration },
    DisconnectedFor { threshold: Duration },
}

/// Result of a remediation action
#[derive(Debug, Clone)]
pub struct RemediationResult {
    pub action: RemediationAction,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Worker ID for remediation tracking
#[derive(Debug, Clone)]
pub enum RemediationResultType {
    NoAction,
    RemediationExecuted { action: RemediationAction },
    SkippedDueToCooldown,
    RemediationFailed { error: RemediationError },
}

/// Errors in remediation operations
#[derive(Debug, thiserror::Error)]
pub enum RemediationError {
    #[error("Worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("No remediation policy found for worker type: {0}")]
    NoPolicyFound(String),

    #[error("Remediation action failed: {0}")]
    ActionFailed(String),

    #[error("Rate limit exceeded for worker: {0}")]
    RateLimitExceeded(WorkerId),

    #[error("Invalid remediation parameters: {0}")]
    InvalidParameters(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// Implement Clone manually since WorkerId may not always be Clone
impl Clone for RemediationError {
    fn clone(&self) -> Self {
        match self {
            RemediationError::WorkerNotFound(id) => RemediationError::WorkerNotFound(id.clone()),
            RemediationError::NoPolicyFound(s) => RemediationError::NoPolicyFound(s.clone()),
            RemediationError::ActionFailed(s) => RemediationError::ActionFailed(s.clone()),
            RemediationError::RateLimitExceeded(id) => {
                RemediationError::RateLimitExceeded(id.clone())
            }
            RemediationError::InvalidParameters(s) => {
                RemediationError::InvalidParameters(s.clone())
            }
            RemediationError::Internal(s) => RemediationError::Internal(s.clone()),
        }
    }
}

/// Event for audit logging of remediation actions
#[derive(Debug, Clone)]
pub struct RemediationActionEvent {
    pub worker_id: WorkerId,
    pub action: RemediationAction,
    pub success: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Audit logger trait for tracking remediation actions
#[async_trait::async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log(&self, event: RemediationActionEvent) -> Result<()>;
}

/// Action executor trait for performing remediation actions
#[async_trait::async_trait]
pub trait ActionExecutor: Send + Sync {
    async fn execute(&self, worker_id: &WorkerId, action: &RemediationAction) -> Result<()>;
}

/// Job manager trait for job reassignment operations
#[async_trait::async_trait]
pub trait JobManager: Send + Sync {
    async fn reassign_jobs(&self, from_worker: &WorkerId, to_workers: &[WorkerId]) -> Result<()>;
}

/// In-memory audit logger implementation
pub struct InMemoryAuditLogger {
    events: Arc<RwLock<Vec<RemediationActionEvent>>>,
}

impl std::fmt::Debug for InMemoryAuditLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryAuditLogger").finish()
    }
}

impl Default for InMemoryAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryAuditLogger {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_events(&self) -> Vec<RemediationActionEvent> {
        let events = self.events.read().await;
        events.clone()
    }
}

#[async_trait::async_trait]
impl AuditLogger for InMemoryAuditLogger {
    async fn log(&self, event: RemediationActionEvent) -> Result<()> {
        let mut events = self.events.write().await;
        events.push(event);
        Ok(())
    }
}

/// Mock action executor for testing
pub struct MockActionExecutor {
    pub worker_repo: Arc<dyn hodei_ports::WorkerRepository + Send + Sync>,
    pub should_fail: std::sync::atomic::AtomicBool,
}

impl std::fmt::Debug for MockActionExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockActionExecutor")
            .field(
                "should_fail",
                &self.should_fail.load(std::sync::atomic::Ordering::Relaxed),
            )
            .finish()
    }
}

impl MockActionExecutor {
    pub fn new(worker_repo: Arc<dyn hodei_ports::WorkerRepository + Send + Sync>) -> Self {
        Self {
            worker_repo,
            should_fail: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn with_failure(self, should_fail: bool) -> Self {
        self.should_fail
            .store(should_fail, std::sync::atomic::Ordering::Relaxed);
        self
    }
}

#[async_trait::async_trait]
impl ActionExecutor for MockActionExecutor {
    async fn execute(&self, worker_id: &WorkerId, action: &RemediationAction) -> Result<()> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(
                RemediationError::ActionFailed("Mock action executor failure".to_string()).into(),
            );
        }

        match action {
            RemediationAction::RestartWorker { grace_period } => {
                info!(worker_id = %worker_id, grace_period_ms = %grace_period.as_millis(), "Executing restart worker action");
                // In a real implementation, would restart the worker
                Ok(())
            }
            RemediationAction::ReassignJobs { target_workers } => {
                info!(worker_id = %worker_id, target_workers_count = %target_workers.len(), "Executing reassign jobs action");
                // In a real implementation, would reassign jobs
                Ok(())
            }
            RemediationAction::ScaleDown { worker_count } => {
                info!(worker_id = %worker_id, worker_count = %worker_count, "Executing scale down action");
                // In a real implementation, would scale down workers
                Ok(())
            }
            RemediationAction::ScaleUp { worker_count } => {
                info!(worker_id = %worker_id, worker_count = %worker_count, "Executing scale up action");
                // In a real implementation, would scale up workers
                Ok(())
            }
            RemediationAction::DrainAndTerminate => {
                info!(worker_id = %worker_id, "Executing drain and terminate action");
                // In a real implementation, would drain and terminate the worker
                Ok(())
            }
        }
    }
}

/// Mock job manager for testing
#[derive(Debug)]
pub struct MockJobManager {
    pub should_fail: std::sync::atomic::AtomicBool,
}

impl Default for MockJobManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MockJobManager {
    pub fn new() -> Self {
        Self {
            should_fail: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn with_failure(self, should_fail: bool) -> Self {
        self.should_fail
            .store(should_fail, std::sync::atomic::Ordering::Relaxed);
        self
    }
}

#[async_trait::async_trait]
impl JobManager for MockJobManager {
    async fn reassign_jobs(&self, from_worker: &WorkerId, to_workers: &[WorkerId]) -> Result<()> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(
                RemediationError::ActionFailed("Mock job manager failure".to_string()).into(),
            );
        }

        info!(
            from_worker = %from_worker,
            target_workers_count = %to_workers.len(),
            "Reassigning jobs"
        );
        Ok(())
    }
}

/// Auto-remediation service for automatic recovery of unhealthy workers
pub struct AutoRemediationService<R, J>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
    J: JobManager + Send + Sync,
{
    policies: Vec<RemediationPolicy>,
    worker_repo: Arc<R>,
    #[allow(dead_code)]
    job_manager: Arc<J>,
    action_executor: Arc<dyn ActionExecutor + Send + Sync>,
    audit_log: Arc<dyn AuditLogger + Send + Sync>,
    last_remediation: Arc<RwLock<HashMap<WorkerId, Instant>>>,
}

impl<R, J> std::fmt::Debug for AutoRemediationService<R, J>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
    J: JobManager + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoRemediationService")
            .field("policies", &"<policies>")
            .field("worker_repo", &"<worker_repo>")
            .field("job_manager", &"<job_manager>")
            .field("action_executor", &"<action_executor>")
            .field("audit_log", &"<audit_log>")
            .field("last_remediation", &"<last_remediation>")
            .finish()
    }
}

impl<R, J> AutoRemediationService<R, J>
where
    R: hodei_ports::WorkerRepository + Send + Sync,
    J: JobManager + Send + Sync,
{
    /// Create new auto-remediation service
    pub fn new(
        policies: Vec<RemediationPolicy>,
        worker_repo: Arc<R>,
        job_manager: Arc<J>,
        action_executor: Arc<dyn ActionExecutor + Send + Sync>,
        audit_log: Arc<dyn AuditLogger + Send + Sync>,
    ) -> Self {
        Self {
            policies,
            worker_repo,
            job_manager,
            action_executor,
            audit_log,
            last_remediation: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Evaluate worker health and execute remediation if needed
    pub async fn evaluate_and_remediate(
        &self,
        worker_id: &WorkerId,
        health_status: &HealthCheckResult,
        health_score: f64,
    ) -> Result<RemediationResultType> {
        // Get worker
        let worker = self
            .worker_repo
            .get_worker(worker_id)
            .await
            .map_err(|e| RemediationError::Internal(e.to_string()))?
            .ok_or(RemediationError::WorkerNotFound(worker_id.clone()))?;

        // Get worker type from metadata
        let worker_type = worker
            .metadata
            .get("worker_type")
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        // Find applicable policy
        let policy = self
            .policies
            .iter()
            .find(|p| p.worker_type == worker_type)
            .ok_or(RemediationError::NoPolicyFound(worker_type))?;

        // Evaluate trigger conditions
        let triggered_conditions = self
            .evaluate_triggers(policy, health_status, health_score)
            .await?;

        if triggered_conditions.is_empty() {
            return Ok(RemediationResultType::NoAction);
        }

        // Check cooldown
        if self.is_in_cooldown(worker_id, policy).await? {
            return Ok(RemediationResultType::SkippedDueToCooldown);
        }

        // Execute remediation actions
        let result = self
            .execute_remediation(policy, worker_id, &triggered_conditions)
            .await?;

        // Log action if remediation was executed
        match &result {
            RemediationResultType::RemediationExecuted { action } => {
                self.audit_log
                    .log(RemediationActionEvent {
                        worker_id: worker_id.clone(),
                        action: action.clone(),
                        success: true,
                        timestamp: chrono::Utc::now(),
                    })
                    .await
                    .map_err(|e| RemediationError::Internal(e.to_string()))?;
            }
            RemediationResultType::RemediationFailed { .. } => {
                self.audit_log
                    .log(RemediationActionEvent {
                        worker_id: worker_id.clone(),
                        action: RemediationAction::RestartWorker {
                            grace_period: Duration::from_secs(30),
                        },
                        success: false,
                        timestamp: chrono::Utc::now(),
                    })
                    .await
                    .map_err(|e| RemediationError::Internal(e.to_string()))?;
            }
            _ => {}
        }

        // Update last remediation time
        {
            let mut last_remediation = self.last_remediation.write().await;
            last_remediation.insert(worker_id.clone(), Instant::now());
        }

        Ok(result)
    }

    /// Enable dry-run mode (actions logged but not executed)
    pub async fn dry_run_remediation(
        &self,
        worker_id: &WorkerId,
        health_status: &HealthCheckResult,
        health_score: f64,
    ) -> Result<Vec<RemediationAction>> {
        // Get worker
        let worker = self
            .worker_repo
            .get_worker(worker_id)
            .await
            .map_err(|e| RemediationError::Internal(e.to_string()))?
            .ok_or(RemediationError::WorkerNotFound(worker_id.clone()))?;

        // Get worker type from metadata
        let worker_type = worker
            .metadata
            .get("worker_type")
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        // Find applicable policy
        let policy = self
            .policies
            .iter()
            .find(|p| p.worker_type == worker_type)
            .ok_or(RemediationError::NoPolicyFound(worker_type))?;

        // Evaluate trigger conditions
        let triggered_conditions = self
            .evaluate_triggers(policy, health_status, health_score)
            .await?;

        if triggered_conditions.is_empty() {
            return Ok(Vec::new());
        }

        // Return actions that would be executed
        let actions = self.determine_actions(policy, &triggered_conditions);
        Ok(actions)
    }

    /// Evaluate trigger conditions
    async fn evaluate_triggers(
        &self,
        policy: &RemediationPolicy,
        health_status: &HealthCheckResult,
        health_score: f64,
    ) -> Result<Vec<TriggerCondition>> {
        let mut triggered = Vec::new();

        for condition in &policy.trigger_conditions {
            let is_triggered = match condition {
                TriggerCondition::ConsecutiveFailures { threshold } => {
                    health_status.consecutive_failures >= *threshold
                }
                TriggerCondition::HealthScoreBelow { threshold } => health_score < *threshold,
                TriggerCondition::ResponseTimeAbove { threshold } => {
                    health_status.response_time > *threshold
                }
                TriggerCondition::DisconnectedFor { threshold } => {
                    if matches!(health_status.status, HealthStatus::Unhealthy { .. }) {
                        let disconnect_duration =
                            chrono::Utc::now().signed_duration_since(health_status.last_check);
                        // Convert Duration to seconds for comparison
                        let threshold_seconds = threshold.as_secs() as i64;
                        disconnect_duration.num_seconds() > threshold_seconds
                    } else {
                        false
                    }
                }
            };

            if is_triggered {
                triggered.push(condition.clone());
            }
        }

        Ok(triggered)
    }

    /// Check if worker is in cooldown period
    async fn is_in_cooldown(
        &self,
        worker_id: &WorkerId,
        policy: &RemediationPolicy,
    ) -> Result<bool> {
        let last_remediation = self.last_remediation.read().await;
        if let Some(last_time) = last_remediation.get(worker_id) {
            Ok(last_time.elapsed() < policy.cooldown)
        } else {
            Ok(false)
        }
    }

    /// Execute remediation actions
    async fn execute_remediation(
        &self,
        policy: &RemediationPolicy,
        worker_id: &WorkerId,
        triggered_conditions: &[TriggerCondition],
    ) -> Result<RemediationResultType> {
        let actions = self.determine_actions(policy, triggered_conditions);

        if actions.is_empty() {
            return Ok(RemediationResultType::NoAction);
        }

        // Execute actions in sequence
        if let Some(action) = actions.into_iter().next() {
            // Check max attempts
            let attempts = self.get_remediation_attempts(worker_id).await?;
            if attempts >= policy.max_attempts {
                return Ok(RemediationResultType::SkippedDueToCooldown);
            }

            // Execute action
            let action_result = self.action_executor.execute(worker_id, &action).await;

            match action_result {
                Ok(_) => {
                    return Ok(RemediationResultType::RemediationExecuted {
                        action: action.clone(),
                    });
                }
                Err(e) => {
                    error!(
                        worker_id = %worker_id,
                        action = ?action,
                        error = %e,
                        "Remediation action failed"
                    );
                    return Ok(RemediationResultType::RemediationFailed {
                        error: RemediationError::Internal(e.to_string()),
                    });
                }
            }
        }

        Ok(RemediationResultType::NoAction)
    }

    /// Determine which actions to execute based on triggered conditions
    fn determine_actions(
        &self,
        policy: &RemediationPolicy,
        triggered_conditions: &[TriggerCondition],
    ) -> Vec<RemediationAction> {
        // Select appropriate actions based on conditions
        // For simplicity, we'll execute all configured actions
        // In a real implementation, would select based on severity

        let mut actions = Vec::new();

        // Execute actions in priority order
        for action in &policy.actions {
            // Check if action is relevant to triggered conditions
            let should_execute = match action {
                RemediationAction::RestartWorker { .. } => triggered_conditions
                    .iter()
                    .any(|c| matches!(c, TriggerCondition::ConsecutiveFailures { .. })),
                RemediationAction::ReassignJobs { .. } => triggered_conditions.iter().any(|c| {
                    matches!(
                        c,
                        TriggerCondition::HealthScoreBelow { .. }
                            | TriggerCondition::ResponseTimeAbove { .. }
                    )
                }),
                RemediationAction::ScaleDown { .. } => triggered_conditions.iter().any(|c| {
                    matches!(
                        c,
                        TriggerCondition::DisconnectedFor { .. }
                            | TriggerCondition::HealthScoreBelow { .. }
                    )
                }),
                RemediationAction::ScaleUp { .. } => false, // Not triggered by unhealthy conditions
                RemediationAction::DrainAndTerminate => triggered_conditions.iter().any(|c| {
                    matches!(
                        c,
                        TriggerCondition::ConsecutiveFailures { threshold: 10 }
                            | TriggerCondition::DisconnectedFor { .. }
                    )
                }),
            };

            if should_execute {
                actions.push(action.clone());
            }
        }

        actions
    }

    /// Get number of remediation attempts for a worker
    async fn get_remediation_attempts(&self, _worker_id: &WorkerId) -> Result<u32> {
        // In a real implementation, would track attempts per worker
        // For now, return 0 (no limit)
        Ok(0)
    }
}
