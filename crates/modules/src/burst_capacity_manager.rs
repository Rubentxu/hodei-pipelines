//! Burst Capacity Management Module
//!
//! This module provides burst capacity management for tenants, allowing
//! temporary quota exceedance during high demand periods.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

use crate::multi_tenancy_quota_manager::{MultiTenancyQuotaManager, TenantId, TenantUsage};

type Result<T> = std::result::Result<T, BurstError>;

/// Burst capacity configuration
#[derive(Debug, Clone)]
pub struct BurstCapacityConfig {
    pub enabled: bool,
    pub default_multiplier: f64,
    pub max_burst_duration: Duration,
    pub burst_cooldown: Duration,
    pub global_burst_pool_ratio: f64, // % of total capacity reserved for bursts
    pub max_concurrent_bursts: u32,
    pub burst_cost_multiplier: f64,
    pub enable_burst_queuing: bool,
}

/// Burst session information
#[derive(Debug, Clone)]
pub struct BurstSession {
    pub tenant_id: TenantId,
    pub start_time: DateTime<Utc>,
    pub expiry_time: DateTime<Utc>,
    pub requested_resources: BurstResourceRequest,
    pub burst_multiplier: f64,
    pub status: BurstStatus,
    pub cost_accrued: f64,
}

/// Resource request for burst
#[derive(Debug, Clone)]
pub struct BurstResourceRequest {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub worker_count: u32,
}

/// Burst status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BurstStatus {
    Active,
    Queued,
    Expired,
    Terminated,
}

/// Burst capacity decision
#[derive(Debug, Clone)]
pub struct BurstDecision {
    pub allowed: bool,
    pub reason: String,
    pub allocated_multiplier: f64,
    pub max_duration: Duration,
    pub cost_impact: f64,
}

/// Burst capacity statistics
#[derive(Debug, Clone)]
pub struct BurstStats {
    pub total_burst_sessions: u64,
    pub active_burst_sessions: u64,
    pub queued_burst_requests: u64,
    pub expired_bursts: u64,
    pub average_burst_duration: f64,
    pub total_burst_cost: f64,
    pub burst_success_rate: f64,
    pub global_burst_capacity_used: f64,
}

/// Burst capacity manager
#[derive(Debug, Clone)]
pub struct BurstCapacityManager {
    config: BurstCapacityConfig,
    quota_manager: MultiTenancyQuotaManager,
    active_sessions: HashMap<TenantId, BurstSession>,
    queued_sessions: Vec<BurstSession>,
    stats: BurstStats,
}

/// Burst error types
#[derive(Debug, thiserror::Error)]
pub enum BurstError {
    #[error("Burst not allowed for tenant {0}")]
    BurstNotAllowed(String),

    #[error("Insufficient burst capacity")]
    InsufficientBurstCapacity,

    #[error("Burst session not found: {0}")]
    SessionNotFound(String),

    #[error("Burst cooldown active for tenant {0}")]
    BurstCooldownActive(String),

    #[error("Maximum burst duration exceeded")]
    MaxBurstDurationExceeded,
}

impl BurstCapacityManager {
    /// Create a new burst capacity manager
    pub fn new(config: BurstCapacityConfig, quota_manager: MultiTenancyQuotaManager) -> Self {
        Self {
            config,
            quota_manager,
            active_sessions: HashMap::new(),
            queued_sessions: Vec::new(),
            stats: BurstStats {
                total_burst_sessions: 0,
                active_burst_sessions: 0,
                queued_burst_requests: 0,
                expired_bursts: 0,
                average_burst_duration: 0.0,
                total_burst_cost: 0.0,
                burst_success_rate: 0.0,
                global_burst_capacity_used: 0.0,
            },
        }
    }

    /// Request burst capacity for a tenant
    pub async fn request_burst_capacity(
        &mut self,
        tenant_id: &str,
        burst_request: BurstResourceRequest,
        requested_multiplier: f64,
    ) -> Result<BurstDecision> {
        // Check if burst is enabled globally
        if !self.config.enabled {
            return Ok(BurstDecision {
                allowed: false,
                reason: "Burst capacity is disabled".to_string(),
                allocated_multiplier: 0.0,
                max_duration: Duration::from_secs(0),
                cost_impact: 0.0,
            });
        }

        // Check if tenant is already in burst
        if self.active_sessions.contains_key(tenant_id) {
            return Ok(BurstDecision {
                allowed: false,
                reason: "Tenant already in burst session".to_string(),
                allocated_multiplier: 0.0,
                max_duration: Duration::from_secs(0),
                cost_impact: 0.0,
            });
        }

        // Get tenant usage
        let usage = self
            .quota_manager
            .get_tenant_usage(tenant_id)
            .await
            .ok_or_else(|| BurstError::BurstNotAllowed(tenant_id.to_string()))?;

        // Check burst cooldown
        if let Some(last_burst) = usage.last_burst {
            let elapsed = Utc::now().signed_duration_since(last_burst);
            if let Ok(cooldown) = ChronoDuration::from_std(self.config.burst_cooldown)
                && elapsed < cooldown {
                    return Ok(BurstDecision {
                        allowed: false,
                        reason: "Burst cooldown period active".to_string(),
                        allocated_multiplier: 0.0,
                        max_duration: Duration::from_secs(0),
                        cost_impact: 0.0,
                    });
                }
        }

        // Check maximum burst duration
        if burst_request.cpu_cores * requested_multiplier as u32 > 1000 {
            return Err(BurstError::MaxBurstDurationExceeded);
        }

        // Determine burst allocation
        let allocated_multiplier = self
            .determine_burst_multiplier(&burst_request, requested_multiplier, &usage)
            .await?;

        // Calculate cost impact
        let cost_impact = self.calculate_burst_cost(&burst_request, allocated_multiplier);

        // Create burst session
        let tenant_id_str = tenant_id.to_string();
        let burst_duration = self.config.max_burst_duration;
        let session = BurstSession {
            tenant_id: tenant_id_str.clone(),
            start_time: Utc::now(),
            expiry_time: Utc::now()
                + ChronoDuration::from_std(burst_duration)
                    .unwrap_or_else(|_| ChronoDuration::seconds(3600)),
            requested_resources: burst_request.clone(),
            burst_multiplier: allocated_multiplier,
            status: BurstStatus::Active,
            cost_accrued: 0.0,
        };

        // Track active session
        self.active_sessions
            .insert(tenant_id_str.clone(), session.clone());
        self.stats.total_burst_sessions += 1;
        self.stats.active_burst_sessions += 1;

        info!(
            "Burst capacity granted to tenant {} with multiplier {}",
            tenant_id_str, allocated_multiplier
        );

        Ok(BurstDecision {
            allowed: true,
            reason: "Burst capacity allocated".to_string(),
            allocated_multiplier,
            max_duration: self.config.max_burst_duration,
            cost_impact,
        })
    }

    /// End burst session for a tenant
    pub async fn end_burst_session(&mut self, tenant_id: &str) -> Result<()> {
        let tenant_id_str = tenant_id.to_string();
        if let Some(session) = self.active_sessions.remove(&tenant_id_str) {
            // Calculate final cost
            let final_cost = self.calculate_session_cost(&session);
            self.stats.total_burst_cost += final_cost;

            self.stats.active_burst_sessions = self.stats.active_burst_sessions.saturating_sub(1);

            info!(
                "Burst session ended for tenant {}. Final cost: ${:.2}",
                tenant_id, final_cost
            );

            Ok(())
        } else {
            Err(BurstError::SessionNotFound(tenant_id_str))
        }
    }

    /// Get active burst sessions
    pub fn get_active_sessions(&self) -> Vec<&BurstSession> {
        self.active_sessions.values().collect()
    }

    /// Get burst statistics
    pub fn get_stats(&self) -> BurstStats {
        self.stats.clone()
    }

    /// Check if a tenant is in burst
    pub fn is_in_burst(&self, tenant_id: &str) -> bool {
        let tenant_id_str = tenant_id.to_string();
        self.active_sessions.contains_key(&tenant_id_str)
    }

    /// Get remaining burst time for a tenant
    pub fn get_remaining_burst_time(&self, tenant_id: &str) -> Option<Duration> {
        let tenant_id_str = tenant_id.to_string();
        if let Some(session) = self.active_sessions.get(&tenant_id_str) {
            let remaining = session.expiry_time - Utc::now();
            remaining.to_std().ok()
        } else {
            None
        }
    }

    /// Process queued burst requests
    pub async fn process_queued_bursts(&mut self) -> Result<()> {
        if !self.config.enable_burst_queuing {
            return Ok(());
        }

        // Process queued requests (simplified: process first in queue)
        while let Some(session) = self.queued_sessions.pop() {
            if self.active_sessions.len() >= self.config.max_concurrent_bursts as usize {
                // Re-queue if at capacity
                self.queued_sessions.push(session);
                break;
            }

            // Grant burst to queued tenant
            self.active_sessions
                .insert(session.tenant_id.clone(), session.clone());
            self.stats.active_burst_sessions += 1;
            self.stats.queued_burst_requests = self.stats.queued_burst_requests.saturating_sub(1);

            info!(
                "Granted burst capacity to queued tenant: {}",
                session.tenant_id
            );
        }

        Ok(())
    }

    /// Clean up expired burst sessions
    pub fn cleanup_expired_sessions(&mut self) -> u32 {
        let mut expired_count = 0;
        let now = Utc::now();

        self.active_sessions.retain(|tenant_id, session| {
            if now >= session.expiry_time {
                expired_count += 1;
                self.stats.expired_bursts += 1;
                self.stats.active_burst_sessions =
                    self.stats.active_burst_sessions.saturating_sub(1);
                warn!("Burst session expired for tenant: {}", tenant_id);
                false
            } else {
                true
            }
        });

        expired_count
    }

    /// Determine appropriate burst multiplier
    async fn determine_burst_multiplier(
        &self,
        _request: &BurstResourceRequest,
        requested: f64,
        _usage: &TenantUsage,
    ) -> Result<f64> {
        // Respect minimum/maximum limits
        let multiplier = requested.max(1.0).min(self.config.default_multiplier);

        // Check if tenant has burst policy
        // In a real implementation, we'd get the tenant's burst policy from quota_manager
        // For now, use default multiplier
        Ok(multiplier)
    }

    /// Calculate burst cost
    fn calculate_burst_cost(&self, request: &BurstResourceRequest, multiplier: f64) -> f64 {
        let base_cost = request.cpu_cores as f64 * 0.05; // $0.05 per core-hour
        
        base_cost * multiplier * self.config.burst_cost_multiplier
    }

    /// Calculate session cost
    fn calculate_session_cost(&self, session: &BurstSession) -> f64 {
        let duration_hours =
            (session.expiry_time - session.start_time).num_seconds() as f64 / 3600.0;
        let base_cost = session.requested_resources.cpu_cores as f64 * 0.05;
        base_cost * duration_hours * session.burst_multiplier * self.config.burst_cost_multiplier
    }
}

impl Default for BurstCapacityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_multiplier: 1.5,
            max_burst_duration: Duration::from_secs(3600), // 1 hour
            burst_cooldown: Duration::from_secs(1800),     // 30 minutes
            global_burst_pool_ratio: 0.2,                  // 20% of capacity
            max_concurrent_bursts: 10,
            burst_cost_multiplier: 1.5, // 50% premium
            enable_burst_queuing: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_burst_request(cpu_cores: u32) -> BurstResourceRequest {
        BurstResourceRequest {
            cpu_cores,
            memory_mb: 256,
            worker_count: 5,
        }
    }

    #[tokio::test]
    async fn test_burst_manager_creation() {
        let config = BurstCapacityConfig::default();
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );

        let manager = BurstCapacityManager::new(config, quota_manager);

        assert!(!manager.is_in_burst("tenant-1"));
        let stats = manager.get_stats();
        assert_eq!(stats.active_burst_sessions, 0);
    }

    #[tokio::test]
    async fn test_request_burst_capacity() {
        let config = BurstCapacityConfig::default();
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );

        // Register tenant FIRST
        let quota = crate::multi_tenancy_quota_manager::TenantQuota {
            tenant_id: "tenant-1".to_string(),
            limits: crate::multi_tenancy_quota_manager::QuotaLimits {
                max_cpu_cores: 100,
                max_memory_mb: 1024,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 50,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
            pool_access: HashMap::new(),
            burst_policy: crate::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 2.0,
                burst_duration: Duration::from_secs(300),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: crate::multi_tenancy_quota_manager::BillingTier::Standard,
            quota_type: crate::multi_tenancy_quota_manager::QuotaType::SoftLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        quota_manager.register_tenant(quota).await.unwrap();

        let mut manager = BurstCapacityManager::new(config, quota_manager.clone());

        // Request burst capacity
        let burst_request = create_burst_request(50);
        let decision = manager
            .request_burst_capacity("tenant-1", burst_request, 1.5)
            .await
            .unwrap();

        assert!(decision.allowed);
        assert_eq!(decision.allocated_multiplier, 1.5);
        assert!(manager.is_in_burst("tenant-1"));

        let stats = manager.get_stats();
        assert_eq!(stats.active_burst_sessions, 1);
    }

    #[tokio::test]
    async fn test_end_burst_session() {
        let config = BurstCapacityConfig::default();
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );

        // Register tenant
        let quota = crate::multi_tenancy_quota_manager::TenantQuota {
            tenant_id: "tenant-1".to_string(),
            limits: crate::multi_tenancy_quota_manager::QuotaLimits {
                max_cpu_cores: 100,
                max_memory_mb: 1024,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 50,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
            pool_access: HashMap::new(),
            burst_policy: crate::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 2.0,
                burst_duration: Duration::from_secs(300),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: crate::multi_tenancy_quota_manager::BillingTier::Standard,
            quota_type: crate::multi_tenancy_quota_manager::QuotaType::SoftLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        quota_manager.register_tenant(quota).await.unwrap();

        let mut manager = BurstCapacityManager::new(config, quota_manager.clone());

        // Start burst
        let burst_request = create_burst_request(50);
        manager
            .request_burst_capacity("tenant-1", burst_request, 1.5)
            .await
            .unwrap();

        // End burst
        manager.end_burst_session("tenant-1").await.unwrap();

        assert!(!manager.is_in_burst("tenant-1"));
        let stats = manager.get_stats();
        assert_eq!(stats.active_burst_sessions, 0);
    }

    #[tokio::test]
    async fn test_duplicate_burst_request() {
        let config = BurstCapacityConfig::default();
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );

        // Register tenant
        let quota = crate::multi_tenancy_quota_manager::TenantQuota {
            tenant_id: "tenant-1".to_string(),
            limits: crate::multi_tenancy_quota_manager::QuotaLimits {
                max_cpu_cores: 100,
                max_memory_mb: 1024,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 50,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
            pool_access: HashMap::new(),
            burst_policy: crate::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 2.0,
                burst_duration: Duration::from_secs(300),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: crate::multi_tenancy_quota_manager::BillingTier::Standard,
            quota_type: crate::multi_tenancy_quota_manager::QuotaType::SoftLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        quota_manager.register_tenant(quota).await.unwrap();

        let mut manager = BurstCapacityManager::new(config, quota_manager.clone());

        // First burst request (should succeed)
        let burst_request = create_burst_request(50);
        let decision1 = manager
            .request_burst_capacity("tenant-1", burst_request.clone(), 1.5)
            .await
            .unwrap();
        assert!(decision1.allowed);

        // Second burst request (should fail - already in burst)
        let decision2 = manager
            .request_burst_capacity("tenant-1", burst_request, 1.5)
            .await
            .unwrap();
        assert!(!decision2.allowed);
        assert!(decision2.reason.contains("already in burst"));
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let config = BurstCapacityConfig {
            max_burst_duration: Duration::from_millis(100), // Very short
            ..Default::default()
        };
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );

        // Register tenant
        let quota = crate::multi_tenancy_quota_manager::TenantQuota {
            tenant_id: "tenant-1".to_string(),
            limits: crate::multi_tenancy_quota_manager::QuotaLimits {
                max_cpu_cores: 100,
                max_memory_mb: 1024,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 50,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
            pool_access: HashMap::new(),
            burst_policy: crate::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 2.0,
                burst_duration: Duration::from_secs(300),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: crate::multi_tenancy_quota_manager::BillingTier::Standard,
            quota_type: crate::multi_tenancy_quota_manager::QuotaType::SoftLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        quota_manager.register_tenant(quota).await.unwrap();

        let mut manager = BurstCapacityManager::new(config, quota_manager.clone());

        // Start burst
        let burst_request = create_burst_request(50);
        manager
            .request_burst_capacity("tenant-1", burst_request, 1.5)
            .await
            .unwrap();

        // Wait for session to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Cleanup expired sessions
        let expired_count = manager.cleanup_expired_sessions();
        assert_eq!(expired_count, 1);
        assert!(!manager.is_in_burst("tenant-1"));
    }
}

// Error conversion to DomainError
impl From<BurstError> for hodei_core::DomainError {
    fn from(err: BurstError) -> Self {
        hodei_core::DomainError::Infrastructure(err.to_string())
    }
}
