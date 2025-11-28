//! Multi-Tenancy Quota Manager Module
//!
//! This module provides comprehensive multi-tenancy support with per-tenant
//! resource quotas, fair share scheduling, and billing integration.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info};

/// Tenant identifier type
pub type TenantId = String;

/// Pool identifier type
pub type PoolId = String;

/// CPU cores type
pub type CpuCores = u32;

/// Memory in megabytes
pub type MemoryMB = u64;

/// Worker count
pub type WorkerCount = u32;

/// Job count
pub type JobCount = u64;

/// Resource request from a tenant
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    pub tenant_id: TenantId,
    pub pool_id: PoolId,
    pub cpu_cores: CpuCores,
    pub memory_mb: MemoryMB,
    pub worker_count: WorkerCount,
    pub estimated_duration: Duration,
    pub priority: JobPriority,
}

/// Job priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Critical,
    High,
    Normal,
    Low,
    Batch,
}

/// Billing tier for cost calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingTier {
    Free,
    Standard,
    Premium,
    Enterprise,
}

/// Quota types for different enforcement strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaType {
    HardLimit, // Strict enforcement, no exceptions
    SoftLimit, // Allow bursts with tracking
    FairShare, // Dynamic based on pool capacity
}

/// Burst policy for handling quota exceedance
#[derive(Debug, Clone)]
pub struct BurstPolicy {
    pub allowed: bool,
    pub max_burst_multiplier: f64,
    pub burst_duration: Duration,
    pub cooldown_period: Duration,
    pub max_bursts_per_day: u32,
}

/// Per-pool quota configuration
#[derive(Debug, Clone)]
pub struct PoolQuota {
    pub pool_id: PoolId,
    pub max_cpu_cores: Option<CpuCores>,
    pub max_memory_mb: Option<MemoryMB>,
    pub max_workers: Option<WorkerCount>,
    pub priority_boost: u8,
}

/// Tenant quota limits
#[derive(Debug, Clone)]
pub struct QuotaLimits {
    pub max_cpu_cores: CpuCores,
    pub max_memory_mb: MemoryMB,
    pub max_concurrent_workers: WorkerCount,
    pub max_concurrent_jobs: u32,
    pub max_daily_cost: f64,
    pub max_monthly_jobs: JobCount,
}

/// Tenant quota configuration
#[derive(Debug, Clone)]
pub struct TenantQuota {
    pub tenant_id: TenantId,
    pub limits: QuotaLimits,
    pub pool_access: HashMap<PoolId, PoolQuota>,
    pub burst_policy: BurstPolicy,
    pub billing_tier: BillingTier,
    pub quota_type: QuotaType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Usage tracking for a tenant
#[derive(Debug, Clone)]
pub struct TenantUsage {
    pub tenant_id: TenantId,
    pub current_cpu_cores: CpuCores,
    pub current_memory_mb: MemoryMB,
    pub current_workers: WorkerCount,
    pub current_jobs: u32,
    pub daily_cost: f64,
    pub monthly_jobs: JobCount,
    pub last_updated: DateTime<Utc>,
    pub burst_count_today: u32,
    pub last_burst: Option<DateTime<Utc>>,
}

/// Quota violation reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaViolationReason {
    HardLimitExceeded,
    ConcurrentLimitReached,
    DailyCostExceeded,
    MonthlyJobsExceeded,
    PoolAccessDenied,
    InsufficientBurstCapacity,
}

impl std::fmt::Display for QuotaViolationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaViolationReason::HardLimitExceeded => write!(f, "Hard limit exceeded"),
            QuotaViolationReason::ConcurrentLimitReached => write!(f, "Concurrent limit reached"),
            QuotaViolationReason::DailyCostExceeded => write!(f, "Daily cost exceeded"),
            QuotaViolationReason::MonthlyJobsExceeded => write!(f, "Monthly jobs exceeded"),
            QuotaViolationReason::PoolAccessDenied => write!(f, "Pool access denied"),
            QuotaViolationReason::InsufficientBurstCapacity => {
                write!(f, "Insufficient burst capacity")
            }
        }
    }
}

/// Quota decision result
#[derive(Debug, Clone)]
pub enum QuotaDecision {
    Allow {
        reason: Option<String>,
    },
    Deny {
        reason: QuotaViolationReason,
    },
    Queue {
        reason: QuotaViolationReason,
        estimated_wait: Duration,
    },
}

/// Fair share allocation decision
#[derive(Debug, Clone)]
pub struct FairShareDecision {
    pub allowed: bool,
    pub allocated_fraction: f64,
    pub weight: f64,
    pub position_in_queue: usize,
}

/// Quota manager statistics
#[derive(Debug, Clone)]
pub struct QuotaStats {
    pub total_tenants: u64,
    pub active_tenants: u64,
    pub quota_violations_today: u64,
    pub queued_requests: u64,
    pub average_check_latency_ms: f64,
    pub total_burst_allowances_used: u64,
}

/// Multi-tenancy quota manager
#[derive(Debug, Clone)]
pub struct MultiTenancyQuotaManager {
    pub quotas: Arc<RwLock<HashMap<TenantId, TenantQuota>>>,
    pub usage: Arc<RwLock<HashMap<TenantId, TenantUsage>>>,
    usage_history: Arc<RwLock<Vec<UsageEvent>>>,
    pub stats: Arc<RwLock<QuotaStats>>,
    pub config: QuotaManagerConfig,
}

/// Usage event for tracking and audit
#[derive(Debug, Clone)]
pub struct UsageEvent {
    pub tenant_id: TenantId,
    pub pool_id: PoolId,
    pub event_type: UsageEventType,
    pub timestamp: DateTime<Utc>,
    pub cpu_cores: CpuCores,
    pub memory_mb: MemoryMB,
    pub cost: f64,
}

/// Usage event types
#[derive(Debug, Clone)]
pub enum UsageEventType {
    Allocation,
    Deallocation,
    BurstStart,
    BurstEnd,
    QuotaExceeded,
}

/// Quota manager configuration
#[derive(Debug, Clone)]
pub struct QuotaManagerConfig {
    pub enable_burst: bool,
    pub default_burst_multiplier: f64,
    pub metrics_retention_days: u32,
    pub check_timeout: Duration,
    pub fair_share_window: Duration,
}

/// Quota error types
#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(TenantId),

    #[error("Pool not found: {0}")]
    PoolNotFound(PoolId),

    #[error("Quota violation: {0}")]
    QuotaViolation(QuotaViolationReason),

    #[error("Invalid quota configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Usage tracking error: {0}")]
    UsageTrackingError(String),

    #[error("Enforcement error: {0}")]
    EnforcementError(String),

    #[error("Fair share calculation error: {0}")]
    FairShareError(String),
}

impl MultiTenancyQuotaManager {
    /// Create a new quota manager
    pub fn new(config: QuotaManagerConfig) -> Self {
        Self {
            quotas: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
            usage_history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(QuotaStats {
                total_tenants: 0,
                active_tenants: 0,
                quota_violations_today: 0,
                queued_requests: 0,
                average_check_latency_ms: 0.0,
                total_burst_allowances_used: 0,
            })),
            config,
        }
    }

    /// Register a new tenant with quota configuration
    pub async fn register_tenant(&self, quota: TenantQuota) -> Result<(), QuotaError> {
        let mut quotas = self.quotas.write().await;

        let tenant_id = quota.tenant_id.clone();
        if quotas.contains_key(&tenant_id) {
            return Err(QuotaError::InvalidConfiguration(format!(
                "Tenant {} already exists",
                tenant_id
            )));
        }

        // Initialize usage tracking
        let initial_usage = TenantUsage {
            tenant_id: tenant_id.clone(),
            current_cpu_cores: 0,
            current_memory_mb: 0,
            current_workers: 0,
            current_jobs: 0,
            daily_cost: 0.0,
            monthly_jobs: 0,
            last_updated: Utc::now(),
            burst_count_today: 0,
            last_burst: None,
        };

        let mut usage = self.usage.write().await;
        usage.insert(tenant_id.clone(), initial_usage);

        quotas.insert(tenant_id.clone(), quota);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_tenants += 1;

        info!("Registered tenant: {}", tenant_id);
        Ok(())
    }

    /// Check if a resource request meets quota requirements
    pub async fn check_quota(
        &self,
        tenant_id: &str,
        resource_request: &ResourceRequest,
    ) -> Result<QuotaDecision, QuotaError> {
        let start_time = Instant::now();

        let quotas = self.quotas.read().await;
        let quota = quotas
            .get(tenant_id)
            .ok_or_else(|| QuotaError::TenantNotFound(tenant_id.to_string()))?;

        let mut usage = self.usage.write().await;
        let tenant_usage = usage.get_mut(tenant_id).ok_or_else(|| {
            QuotaError::UsageTrackingError(format!("No usage tracking for tenant {}", tenant_id))
        })?;

        // Update last accessed time
        tenant_usage.last_updated = Utc::now();

        // Check pool access
        if !quota.pool_access.is_empty() {
            if !quota.pool_access.contains_key(&resource_request.pool_id) {
                return Ok(QuotaDecision::Deny {
                    reason: QuotaViolationReason::PoolAccessDenied,
                });
            }
        }

        // Check hard limits for CPU
        let projected_cpu = tenant_usage.current_cpu_cores + resource_request.cpu_cores;
        if projected_cpu > quota.limits.max_cpu_cores {
            return Ok(QuotaDecision::Deny {
                reason: QuotaViolationReason::HardLimitExceeded,
            });
        }

        // Check hard limits for memory
        let projected_memory = tenant_usage.current_memory_mb + resource_request.memory_mb;
        if projected_memory > quota.limits.max_memory_mb {
            return Ok(QuotaDecision::Deny {
                reason: QuotaViolationReason::HardLimitExceeded,
            });
        }

        // Check concurrent workers limit
        let projected_workers = tenant_usage.current_workers + resource_request.worker_count;
        if projected_workers > quota.limits.max_concurrent_workers {
            return Ok(QuotaDecision::Queue {
                reason: QuotaViolationReason::ConcurrentLimitReached,
                estimated_wait: self.estimate_wait_time(tenant_usage, &quota).await,
            });
        }

        // Check concurrent jobs limit
        if tenant_usage.current_jobs + 1 > quota.limits.max_concurrent_jobs {
            return Ok(QuotaDecision::Queue {
                reason: QuotaViolationReason::ConcurrentLimitReached,
                estimated_wait: self.estimate_wait_time(tenant_usage, &quota).await,
            });
        }

        // For soft limits, allow burst
        if quota.quota_type == QuotaType::SoftLimit
            && resource_request.cpu_cores > quota.limits.max_cpu_cores
        {
            let burst_decision = self
                .check_burst_capacity(tenant_id, &quota, tenant_usage)
                .await?;
            if burst_decision.allowed {
                tenant_usage.burst_count_today += 1;
                tenant_usage.last_burst = Some(Utc::now());

                // Record burst usage event
                self.record_usage_event(
                    tenant_id,
                    &resource_request.pool_id,
                    UsageEventType::BurstStart,
                    resource_request.cpu_cores,
                    resource_request.memory_mb,
                    0.0,
                )
                .await;

                return Ok(QuotaDecision::Allow {
                    reason: Some("Burst allocation".to_string()),
                });
            }
        }

        // All checks passed
        Ok(QuotaDecision::Allow { reason: None })
    }

    /// Allocate resources for a tenant
    pub async fn allocate_resources(
        &self,
        tenant_id: &str,
        resource_request: &ResourceRequest,
    ) -> Result<(), QuotaError> {
        let quotas = self.quotas.read().await;
        let quota = quotas
            .get(tenant_id)
            .ok_or_else(|| QuotaError::TenantNotFound(tenant_id.to_string()))?;

        let mut usage = self.usage.write().await;
        let tenant_usage = usage.get_mut(tenant_id).ok_or_else(|| {
            QuotaError::UsageTrackingError(format!("No usage tracking for tenant {}", tenant_id))
        })?;

        // Update usage
        tenant_usage.current_cpu_cores += resource_request.cpu_cores;
        tenant_usage.current_memory_mb += resource_request.memory_mb;
        tenant_usage.current_workers += resource_request.worker_count;
        tenant_usage.current_jobs += 1;

        // Estimate cost
        let estimated_cost = self.calculate_cost(&quota, &resource_request);
        tenant_usage.daily_cost += estimated_cost;

        // Record usage event
        self.record_usage_event(
            tenant_id,
            &resource_request.pool_id,
            UsageEventType::Allocation,
            resource_request.cpu_cores,
            resource_request.memory_mb,
            estimated_cost,
        )
        .await;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.active_tenants += 1;

        info!(
            "Allocated resources for tenant {}: {} CPU, {} MB",
            tenant_id, resource_request.cpu_cores, resource_request.memory_mb
        );
        Ok(())
    }

    /// Deallocate resources from a tenant
    pub async fn deallocate_resources(
        &self,
        tenant_id: &str,
        resource_request: &ResourceRequest,
    ) -> Result<(), QuotaError> {
        let mut usage = self.usage.write().await;
        let tenant_usage = usage.get_mut(tenant_id).ok_or_else(|| {
            QuotaError::UsageTrackingError(format!("No usage tracking for tenant {}", tenant_id))
        })?;

        // Update usage
        tenant_usage.current_cpu_cores = tenant_usage
            .current_cpu_cores
            .saturating_sub(resource_request.cpu_cores);
        tenant_usage.current_memory_mb = tenant_usage
            .current_memory_mb
            .saturating_sub(resource_request.memory_mb);
        tenant_usage.current_workers = tenant_usage
            .current_workers
            .saturating_sub(resource_request.worker_count);
        tenant_usage.current_jobs = tenant_usage.current_jobs.saturating_sub(1);

        // Record usage event
        self.record_usage_event(
            tenant_id,
            &resource_request.pool_id,
            UsageEventType::Deallocation,
            resource_request.cpu_cores,
            resource_request.memory_mb,
            0.0,
        )
        .await;

        info!(
            "Deallocated resources for tenant {}: {} CPU, {} MB",
            tenant_id, resource_request.cpu_cores, resource_request.memory_mb
        );
        Ok(())
    }

    /// Get tenant usage information
    pub async fn get_tenant_usage(&self, tenant_id: &str) -> Option<TenantUsage> {
        let usage = self.usage.read().await;
        usage.get(tenant_id).cloned()
    }

    /// Get all tenant usage
    pub async fn get_all_tenant_usage(&self) -> HashMap<TenantId, TenantUsage> {
        let usage = self.usage.read().await;
        usage.clone()
    }

    /// Get quota statistics
    pub async fn get_stats(&self) -> QuotaStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Update tenant quota
    pub async fn update_quota(
        &self,
        tenant_id: &str,
        new_quota: TenantQuota,
    ) -> Result<(), QuotaError> {
        let mut quotas = self.quotas.write().await;
        let tenant_id_str = tenant_id.to_string();
        let quota = quotas
            .get_mut(&tenant_id_str)
            .ok_or_else(|| QuotaError::TenantNotFound(tenant_id_str.clone()))?;

        *quota = new_quota;
        quota.updated_at = Utc::now();

        info!("Updated quota for tenant: {}", tenant_id);
        Ok(())
    }

    /// Reset daily usage for all tenants
    pub async fn reset_daily_usage(&self) -> Result<(), QuotaError> {
        let mut usage = self.usage.write().await;
        for (_, tenant_usage) in usage.iter_mut() {
            tenant_usage.daily_cost = 0.0;
            tenant_usage.burst_count_today = 0;
        }

        info!("Reset daily usage for all tenants");
        Ok(())
    }

    /// Check if burst capacity is available
    async fn check_burst_capacity(
        &self,
        tenant_id: &str,
        quota: &TenantQuota,
        usage: &TenantUsage,
    ) -> Result<BurstCapacityDecision, QuotaError> {
        if !self.config.enable_burst || !quota.burst_policy.allowed {
            return Ok(BurstCapacityDecision {
                allowed: false,
                reason: "Burst not enabled".to_string(),
            });
        }

        // Check burst count limit
        if usage.burst_count_today >= quota.burst_policy.max_bursts_per_day {
            return Ok(BurstCapacityDecision {
                allowed: false,
                reason: "Maximum bursts per day reached".to_string(),
            });
        }

        // Check cooldown period
        if let Some(last_burst) = usage.last_burst {
            let elapsed = Utc::now().signed_duration_since(last_burst);
            if let Ok(cooldown_duration) =
                chrono::Duration::from_std(quota.burst_policy.cooldown_period)
            {
                if elapsed < cooldown_duration {
                    return Ok(BurstCapacityDecision {
                        allowed: false,
                        reason: "Burst cooldown period active".to_string(),
                    });
                }
            }
        }

        Ok(BurstCapacityDecision {
            allowed: true,
            reason: "Burst capacity available".to_string(),
        })
    }

    /// Calculate estimated wait time
    async fn estimate_wait_time(&self, usage: &TenantUsage, quota: &TenantQuota) -> Duration {
        let cpu_utilization = usage.current_cpu_cores as f64 / quota.limits.max_cpu_cores as f64;
        let worker_utilization =
            usage.current_workers as f64 / quota.limits.max_concurrent_workers as f64;

        let max_utilization = cpu_utilization.max(worker_utilization);
        let remaining_capacity = 1.0 - max_utilization;

        if remaining_capacity <= 0.0 {
            Duration::from_secs(300) // 5 minutes max wait
        } else {
            let estimated_wait_secs = (remaining_capacity * 60.0) as u64;
            Duration::from_secs(estimated_wait_secs.max(30))
        }
    }

    /// Calculate cost for a resource request
    fn calculate_cost(&self, quota: &TenantQuota, request: &ResourceRequest) -> f64 {
        let cpu_cost_per_core = match quota.billing_tier {
            BillingTier::Free => 0.0,
            BillingTier::Standard => 0.05,
            BillingTier::Premium => 0.04,
            BillingTier::Enterprise => 0.03,
        };

        let memory_cost_per_gb = match quota.billing_tier {
            BillingTier::Free => 0.0,
            BillingTier::Standard => 0.01,
            BillingTier::Premium => 0.008,
            BillingTier::Enterprise => 0.006,
        };

        let duration_hours = request.estimated_duration.as_secs_f64() / 3600.0;
        let memory_gb = request.memory_mb as f64 / 1024.0;

        (request.cpu_cores as f64 * cpu_cost_per_core * duration_hours)
            + (memory_gb * memory_cost_per_gb * duration_hours)
    }

    /// Record a usage event
    async fn record_usage_event(
        &self,
        tenant_id: &str,
        pool_id: &PoolId,
        event_type: UsageEventType,
        cpu_cores: CpuCores,
        memory_mb: MemoryMB,
        cost: f64,
    ) {
        let event = UsageEvent {
            tenant_id: tenant_id.to_string(),
            pool_id: pool_id.clone(),
            event_type,
            timestamp: Utc::now(),
            cpu_cores,
            memory_mb,
            cost,
        };

        let mut history = self.usage_history.write().await;
        history.push(event);

        // Keep history within limits
        let max_history = 10000;
        if history.len() > max_history {
            let overflow = history.len() - max_history;
            history.drain(0..overflow);
        }
    }
}

/// Burst capacity decision
#[derive(Debug, Clone)]
struct BurstCapacityDecision {
    pub allowed: bool,
    pub reason: String,
}

/// Create default quota manager configuration
impl Default for QuotaManagerConfig {
    fn default() -> Self {
        Self {
            enable_burst: true,
            default_burst_multiplier: 1.5,
            metrics_retention_days: 30,
            check_timeout: Duration::from_millis(100),
            fair_share_window: Duration::from_secs(60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_quota(tenant_id: &str) -> TenantQuota {
        TenantQuota {
            tenant_id: tenant_id.to_string(),
            limits: QuotaLimits {
                max_cpu_cores: 100,
                max_memory_mb: 1024,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 50,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
            pool_access: HashMap::new(),
            burst_policy: BurstPolicy {
                allowed: true,
                max_burst_multiplier: 1.5,
                burst_duration: Duration::from_secs(300),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: BillingTier::Standard,
            quota_type: QuotaType::HardLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_request(tenant_id: &str, pool_id: &str) -> ResourceRequest {
        ResourceRequest {
            tenant_id: tenant_id.to_string(),
            pool_id: pool_id.to_string(),
            cpu_cores: 10,
            memory_mb: 256,
            worker_count: 5,
            estimated_duration: Duration::from_secs(3600),
            priority: JobPriority::Normal,
        }
    }

    #[tokio::test]
    async fn test_register_tenant() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let quota = create_test_quota("tenant-1");

        let result = manager.register_tenant(quota).await;
        assert!(result.is_ok());

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_tenants, 1);
    }

    #[tokio::test]
    async fn test_check_quota_allocation_allowed() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let quota = create_test_quota("tenant-1");
        manager.register_tenant(quota).await.unwrap();

        let request = create_test_request("tenant-1", "pool-1");
        let decision = manager.check_quota("tenant-1", &request).await.unwrap();

        match decision {
            QuotaDecision::Allow { .. } => {}
            _ => panic!("Expected Allow decision"),
        }
    }

    #[tokio::test]
    async fn test_check_quota_hard_limit_exceeded() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let quota = create_test_quota("tenant-1");
        manager.register_tenant(quota).await.unwrap();

        let request = ResourceRequest {
            tenant_id: "tenant-1".to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores: 200, // Exceeds limit
            memory_mb: 256,
            worker_count: 5,
            estimated_duration: Duration::from_secs(3600),
            priority: JobPriority::Normal,
        };

        let decision = manager.check_quota("tenant-1", &request).await.unwrap();

        match decision {
            QuotaDecision::Deny { reason } => {
                assert_eq!(reason, QuotaViolationReason::HardLimitExceeded);
            }
            _ => panic!("Expected Deny decision"),
        }
    }

    #[tokio::test]
    async fn test_allocate_and_deallocate_resources() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let quota = create_test_quota("tenant-1");
        manager.register_tenant(quota).await.unwrap();

        let request = create_test_request("tenant-1", "pool-1");

        // Allocate resources
        manager
            .allocate_resources("tenant-1", &request)
            .await
            .unwrap();

        let usage = manager.get_tenant_usage("tenant-1").await.unwrap();
        assert_eq!(usage.current_cpu_cores, 10);
        assert_eq!(usage.current_memory_mb, 256);
        assert_eq!(usage.current_workers, 5);
        assert_eq!(usage.current_jobs, 1);

        // Deallocate resources
        manager
            .deallocate_resources("tenant-1", &request)
            .await
            .unwrap();

        let usage = manager.get_tenant_usage("tenant-1").await.unwrap();
        assert_eq!(usage.current_cpu_cores, 0);
        assert_eq!(usage.current_memory_mb, 0);
        assert_eq!(usage.current_workers, 0);
        assert_eq!(usage.current_jobs, 0);
    }

    #[tokio::test]
    async fn test_concurrent_limit_exceeded() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let mut quota = create_test_quota("tenant-1");
        quota.limits.max_concurrent_workers = 3;
        quota.limits.max_concurrent_jobs = 2;
        manager.register_tenant(quota).await.unwrap();

        let request1 = create_test_request("tenant-1", "pool-1");
        let request2 = create_test_request("tenant-1", "pool-1");
        let request3 = create_test_request("tenant-1", "pool-1");

        // Allocate first two requests
        manager
            .allocate_resources("tenant-1", &request1)
            .await
            .unwrap();
        manager
            .allocate_resources("tenant-1", &request2)
            .await
            .unwrap();

        // Third request should be queued
        let decision = manager.check_quota("tenant-1", &request3).await.unwrap();

        match decision {
            QuotaDecision::Queue { .. } => {}
            _ => panic!("Expected Queue decision"),
        }
    }

    #[tokio::test]
    async fn test_update_quota() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let quota = create_test_quota("tenant-1");
        manager.register_tenant(quota).await.unwrap();

        let mut new_quota = create_test_quota("tenant-1");
        new_quota.limits.max_cpu_cores = 200;

        manager.update_quota("tenant-1", new_quota).await.unwrap();

        let request = ResourceRequest {
            tenant_id: "tenant-1".to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores: 150,
            memory_mb: 256,
            worker_count: 5,
            estimated_duration: Duration::from_secs(3600),
            priority: JobPriority::Normal,
        };

        let decision = manager.check_quota("tenant-1", &request).await.unwrap();

        match decision {
            QuotaDecision::Allow { .. } => {}
            _ => panic!("Expected Allow decision for updated quota"),
        }
    }

    #[tokio::test]
    async fn test_reset_daily_usage() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let quota = create_test_quota("tenant-1");
        manager.register_tenant(quota).await.unwrap();

        let request = create_test_request("tenant-1", "pool-1");
        manager
            .allocate_resources("tenant-1", &request)
            .await
            .unwrap();

        let usage_before = manager.get_tenant_usage("tenant-1").await.unwrap();
        assert!(usage_before.daily_cost > 0.0);

        manager.reset_daily_usage().await.unwrap();

        let usage_after = manager.get_tenant_usage("tenant-1").await.unwrap();
        assert_eq!(usage_after.daily_cost, 0.0);
    }

    #[tokio::test]
    async fn test_tenant_not_found() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());
        let request = create_test_request("tenant-1", "pool-1");

        let result = manager.check_quota("tenant-1", &request).await;

        assert!(matches!(result, Err(QuotaError::TenantNotFound(_))));
    }

    #[tokio::test]
    async fn test_cost_calculation() {
        let manager = MultiTenancyQuotaManager::new(QuotaManagerConfig::default());

        let quota = create_test_quota("tenant-1");
        let request = ResourceRequest {
            tenant_id: "tenant-1".to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores: 4,
            memory_mb: 512,
            worker_count: 2,
            estimated_duration: Duration::from_secs(3600), // 1 hour
            priority: JobPriority::Normal,
        };

        let cost = manager.calculate_cost(&quota, &request);

        // 4 cores * $0.05/hour * 1 hour + 0.5 GB * $0.01/hour * 1 hour
        assert!(cost > 0.0);
        assert!(cost < 1.0);
    }
}

// Error conversion to DomainError
impl From<QuotaError> for hodei_core::DomainError {
    fn from(err: QuotaError) -> Self {
        hodei_core::DomainError::Infrastructure(err.to_string())
    }
}
