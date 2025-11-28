//! Weighted Fair Queuing (WFQ) Module
//!
//! This module implements weighted fair queuing for multi-tenant environments,
//! ensuring fair resource allocation based on tenant weights and preventing
//! resource starvation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::multi_tenancy_quota_manager::{
    BillingTier, ResourceRequest, TenantId, TenantQuota, TenantUsage,
};

/// Tenant weight configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightStrategy {
    /// Weight based on billing tier
    BillingTier,
    /// Weight based on quota size
    QuotaBased,
    /// Weight based on usage history
    UsageHistory,
    /// Custom weight per tenant
    Custom,
}

/// Tenant weight configuration
#[derive(Debug, Clone)]
pub struct TenantWeight {
    pub tenant_id: TenantId,
    pub weight: f64,
    pub strategy: WeightStrategy,
    pub min_weight: f64,
    pub max_weight: f64,
    pub last_updated: DateTime<Utc>,
}

/// Resource allocation for WFQ scheduling
#[derive(Debug, Clone)]
pub struct WFQAllocation {
    pub tenant_id: TenantId,
    pub allocated_cpu: u32,
    pub allocated_memory: u64,
    pub allocated_workers: u32,
    pub weight: f64,
    pub virtual_time: f64,
    pub finish_time: f64,
}

/// WFQ queue entry
#[derive(Debug, Clone)]
pub struct WFQQueueEntry {
    pub tenant_id: TenantId,
    pub request: ResourceRequest,
    pub arrival_time: DateTime<Utc>,
    pub virtual_finish_time: f64,
    pub weight: f64,
    pub packet_size: u64, // Resource units
}

/// Weight calculation context
#[derive(Debug, Clone)]
pub struct WeightContext {
    pub tenant_quota: TenantQuota,
    pub tenant_usage: TenantUsage,
    pub pool_capacity: u64,
    pub total_weights: f64,
    pub current_time: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WFQConfig {
    /// Global virtual time for fair queuing
    pub enable_virtual_time: bool,
    /// Minimum weight for any tenant
    pub min_weight: f64,
    /// Maximum weight for any tenant
    pub max_weight: f64,
    /// Weight calculation strategy
    pub default_strategy: WeightStrategy,
    /// Starvation prevention threshold (0.0 to 1.0)
    pub starvation_threshold: f64,
    /// Weight adjustment interval
    pub weight_update_interval: Duration,
    /// Default packet size for resource calculation
    pub default_packet_size: u64,
    /// Enable dynamic weight adjustment
    pub enable_dynamic_weights: bool,
    /// Starvation detection window
    pub starvation_window: Duration,
    /// Fair share calculation window
    pub fair_share_window: Duration,
}

/// WFQ statistics
#[derive(Debug, Clone)]
pub struct WFQStats {
    pub total_allocations: u64,
    pub total_tenants: u64,
    pub active_tenants: u64,
    pub starvation_events: u64,
    pub weight_adjustments: u64,
    pub average_wait_time_ms: f64,
    pub fairness_index: f64, // 0.0 to 1.0 (1.0 = perfectly fair)
    pub virtual_time: f64,
    pub queue_depth: u64,
}

/// Starvation detection
#[derive(Debug, Clone)]
pub struct StarvationDetection {
    pub tenant_id: TenantId,
    pub first_seen: DateTime<Utc>,
    pub last_allocation: Option<DateTime<Utc>>,
    pub waiting_time: Duration,
    pub allocation_count: u64,
    pub is_starving: bool,
}

/// Weighted Fair Queuing Engine
#[derive(Debug)]
pub struct WeightedFairQueueingEngine {
    config: WFQConfig,
    tenant_weights: Arc<RwLock<HashMap<TenantId, TenantWeight>>>,
    active_flows: Arc<RwLock<HashMap<TenantId, WFQAllocation>>>,
    pending_queue: Arc<RwLock<VecDeque<WFQQueueEntry>>>,
    global_virtual_time: Arc<RwLock<f64>>,
    stats: Arc<RwLock<WFQStats>>,
    starvation_monitor: Arc<RwLock<HashMap<TenantId, StarvationDetection>>>,
}

/// WFQ error types
#[derive(Debug, thiserror::Error)]
pub enum WFQError {
    #[error("Tenant not found: {0}")]
    TenantNotFound(TenantId),

    #[error("Invalid weight configuration: {0}")]
    InvalidWeight(String),

    #[error("Queue overflow")]
    QueueOverflow,

    #[error("Starvation detected for tenant: {0}")]
    StarvationDetected(TenantId),

    #[error("No capacity available")]
    NoCapacity,

    #[error("Weight calculation error: {0}")]
    WeightCalculationError(String),
}

impl WeightedFairQueueingEngine {
    /// Create a new WFQ engine
    pub fn new(config: WFQConfig) -> Self {
        Self {
            config,
            tenant_weights: Arc::new(RwLock::new(HashMap::new())),
            active_flows: Arc::new(RwLock::new(HashMap::new())),
            pending_queue: Arc::new(RwLock::new(VecDeque::new())),
            global_virtual_time: Arc::new(RwLock::new(0.0)),
            stats: Arc::new(RwLock::new(WFQStats {
                total_allocations: 0,
                total_tenants: 0,
                active_tenants: 0,
                starvation_events: 0,
                weight_adjustments: 0,
                average_wait_time_ms: 0.0,
                fairness_index: 1.0,
                virtual_time: 0.0,
                queue_depth: 0,
            })),
            starvation_monitor: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tenant with the WFQ engine
    pub async fn register_tenant(
        &self,
        tenant_quota: TenantQuota,
        usage: &TenantUsage,
    ) -> Result<(), WFQError> {
        let weight = self.calculate_initial_weight(&tenant_quota, usage)?;
        let tenant_weight = TenantWeight {
            tenant_id: tenant_quota.tenant_id.clone(),
            weight,
            strategy: self.config.default_strategy,
            min_weight: self.config.min_weight,
            max_weight: self.config.max_weight,
            last_updated: Utc::now(),
        };

        let mut weights = self.tenant_weights.write().await;
        weights.insert(tenant_quota.tenant_id.clone(), tenant_weight);

        // Initialize starvation detection
        let mut starvation = self.starvation_monitor.write().await;
        starvation.insert(
            tenant_quota.tenant_id.clone(),
            StarvationDetection {
                tenant_id: tenant_quota.tenant_id.clone(),
                first_seen: Utc::now(),
                last_allocation: None,
                waiting_time: Duration::from_secs(0),
                allocation_count: 0,
                is_starving: false,
            },
        );

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_tenants += 1;

        info!("Registered tenant in WFQ: {}", tenant_quota.tenant_id);
        Ok(())
    }

    /// Enqueue a resource request
    pub async fn enqueue_request(&self, request: ResourceRequest) -> Result<(), WFQError> {
        let weights = self.tenant_weights.read().await;
        let weight = weights
            .get(&request.tenant_id)
            .ok_or_else(|| WFQError::TenantNotFound(request.tenant_id.clone()))?;

        let global_time = self.global_virtual_time.write().await;
        let packet_size = self.calculate_packet_size(&request);

        // Calculate virtual finish time
        let virtual_finish_time = *global_time + (packet_size as f64 / weight.weight);

        let mut queue = self.pending_queue.write().await;
        if queue.len() >= 10000 {
            return Err(WFQError::QueueOverflow);
        }

        // Create entry
        let entry = WFQQueueEntry {
            tenant_id: request.tenant_id.clone(),
            request: request.clone(),
            arrival_time: Utc::now(),
            virtual_finish_time,
            weight: weight.weight,
            packet_size,
        };

        // Insert in order of virtual finish time (priority queue behavior)
        let mut inserted = false;
        for (index, existing) in queue.iter().enumerate() {
            if entry.virtual_finish_time < existing.virtual_finish_time {
                queue.insert(index, entry.clone());
                inserted = true;
                break;
            }
        }

        if !inserted {
            queue.push_back(entry);
        }

        // Update starvation monitor
        self.update_starvation_monitor(&request.tenant_id).await;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.queue_depth = queue.len() as u64;

        debug!("Enqueued request for tenant: {}", request.tenant_id);
        Ok(())
    }

    /// Dequeue the next request for scheduling
    pub async fn dequeue_next(&self) -> Option<WFQQueueEntry> {
        let mut queue = self.pending_queue.write().await;

        if queue.is_empty() {
            return None;
        }

        // Get entry with minimum virtual finish time
        let entry = queue.pop_front()?;

        // Update global virtual time
        let mut global_time = self.global_virtual_time.write().await;
        *global_time = entry.virtual_finish_time;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.virtual_time = entry.virtual_finish_time;
        stats.queue_depth = queue.len() as u64;

        // Update starvation monitor
        self.update_allocation(&entry.tenant_id).await;

        debug!(
            "Dequeued request for tenant: {} (VFT: {})",
            entry.tenant_id, entry.virtual_finish_time
        );

        Some(entry)
    }

    /// Get fair share allocation for all active tenants
    pub async fn get_fair_share_allocation(&self, pool_capacity: u64) -> Vec<WFQAllocation> {
        let weights = self.tenant_weights.read().await;
        let mut allocations = Vec::new();

        let total_weight: f64 = weights.values().map(|w| w.weight).sum();
        if total_weight == 0.0 {
            return allocations;
        }

        let global_time = self.global_virtual_time.read().await;

        for (tenant_id, weight) in weights.iter() {
            let allocated = (pool_capacity as f64 * weight.weight / total_weight) as u64;

            let allocation = WFQAllocation {
                tenant_id: tenant_id.clone(),
                allocated_cpu: (allocated / 1000) as u32, // Simplified: 1 unit = 1000 CPU
                allocated_memory: (allocated * 100) as u64, // Simplified: 1 unit = 100MB
                allocated_workers: (allocated / 100) as u32,
                weight: weight.weight,
                virtual_time: *global_time,
                finish_time: *global_time
                    + (self.config.default_packet_size as f64 / weight.weight),
            };

            allocations.push(allocation);
        }

        allocations
    }

    /// Update tenant weight dynamically
    pub async fn update_tenant_weight(
        &self,
        tenant_id: &str,
        new_weight: f64,
    ) -> Result<(), WFQError> {
        let mut weights = self.tenant_weights.write().await;
        let tenant_weight = weights
            .get_mut(tenant_id)
            .ok_or_else(|| WFQError::TenantNotFound(tenant_id.to_string()))?;

        // Clamp weight to valid range
        let clamped_weight = new_weight
            .max(self.config.min_weight)
            .min(self.config.max_weight);

        tenant_weight.weight = clamped_weight;
        tenant_weight.last_updated = Utc::now();

        // Update stats
        let mut stats = self.stats.write().await;
        stats.weight_adjustments += 1;

        info!(
            "Updated weight for tenant {}: {}",
            tenant_id, clamped_weight
        );
        Ok(())
    }

    /// Recalculate weights based on current strategy
    pub async fn recalculate_weights(
        &self,
        contexts: HashMap<TenantId, WeightContext>,
    ) -> Result<(), WFQError> {
        let mut weights = self.tenant_weights.write().await;

        for (tenant_id, context) in contexts {
            if let Some(weight_config) = weights.get_mut(&tenant_id) {
                let new_weight = self.calculate_weight(&context, weight_config.strategy)?;
                let clamped_weight = new_weight
                    .max(self.config.min_weight)
                    .min(self.config.max_weight);

                weight_config.weight = clamped_weight;
                weight_config.last_updated = Utc::now();
            }
        }

        // Update stats
        let mut stats = self.stats.write().await;
        stats.weight_adjustments += weights.len() as u64;

        info!("Recalculated weights for {} tenants", weights.len());
        Ok(())
    }

    /// Get current fairness index
    pub async fn calculate_fairness_index(&self) -> f64 {
        let weights = self.tenant_weights.read().await;

        if weights.is_empty() {
            return 1.0;
        }

        let values: Vec<f64> = weights.values().map(|w| w.weight).collect();
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        let variance: f64 =
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

        // Jain's fairness index
        let sum: f64 = values.iter().sum();
        let sum_squared: f64 = values.iter().map(|v| v * v).sum();

        if sum_squared == 0.0 {
            return 1.0;
        }

        let jain_index = (sum * sum) / (values.len() as f64 * sum_squared);
        1.0 - variance / (mean * mean).max(0.001) // Normalized fairness
    }

    /// Detect and handle starvation
    pub async fn detect_and_handle_starvation(&mut self) -> Vec<TenantId> {
        let mut starving_tenants = Vec::new();
        let mut starvation = self.starvation_monitor.write().await;
        let now = Utc::now();

        for (tenant_id, detection) in starvation.iter_mut() {
            // Check if tenant has been waiting too long
            if detection.waiting_time > self.config.starvation_window {
                detection.is_starving = true;
                starving_tenants.push(tenant_id.clone());

                // Increase weight for starving tenant
                if let Some(weight_config) = self.tenant_weights.write().await.get_mut(tenant_id) {
                    weight_config.weight = (weight_config.weight * 1.5).min(self.config.max_weight);
                }

                info!("Starvation detected for tenant: {}", tenant_id);
            }
        }

        // Update stats
        if !starving_tenants.is_empty() {
            let mut stats = self.stats.write().await;
            stats.starvation_events += starving_tenants.len() as u64;
        }

        starving_tenants
    }

    /// Get WFQ statistics
    pub async fn get_stats(&self) -> WFQStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Get tenant weight
    pub async fn get_tenant_weight(&self, tenant_id: &str) -> Option<f64> {
        let weights = self.tenant_weights.read().await;
        weights.get(tenant_id).map(|w| w.weight)
    }

    /// Get all tenant weights
    pub async fn get_all_weights(&self) -> HashMap<TenantId, f64> {
        let weights = self.tenant_weights.read().await;
        weights.iter().map(|(k, v)| (k.clone(), v.weight)).collect()
    }

    /// Get queue depth
    pub async fn get_queue_depth(&self) -> u64 {
        let queue = self.pending_queue.read().await;
        queue.len() as u64
    }

    /// Clear queue (emergency use only)
    pub async fn clear_queue(&self) {
        let mut queue = self.pending_queue.write().await;
        queue.clear();

        let mut stats = self.stats.write().await;
        stats.queue_depth = 0;

        warn!("WFQ queue cleared");
    }

    /// Calculate initial weight for a tenant
    fn calculate_initial_weight(
        &self,
        quota: &TenantQuota,
        usage: &TenantUsage,
    ) -> Result<f64, WFQError> {
        match self.config.default_strategy {
            WeightStrategy::BillingTier => match quota.billing_tier {
                BillingTier::Free => Ok(1.0),
                BillingTier::Standard => Ok(2.0),
                BillingTier::Premium => Ok(3.0),
                BillingTier::Enterprise => Ok(5.0),
            },
            WeightStrategy::QuotaBased => {
                let total_quota = quota.limits.max_cpu_cores as f64
                    + (quota.limits.max_memory_mb as f64 / 1024.0);
                Ok((total_quota / 100.0).max(1.0))
            }
            WeightStrategy::UsageHistory => {
                let usage_ratio = if quota.limits.max_cpu_cores > 0 {
                    usage.current_cpu_cores as f64 / quota.limits.max_cpu_cores as f64
                } else {
                    0.0
                };
                Ok((1.0 + usage_ratio).min(self.config.max_weight))
            }
            WeightStrategy::Custom => Ok(2.0), // Default custom weight
        }
    }

    /// Calculate weight based on context
    fn calculate_weight(
        &self,
        context: &WeightContext,
        strategy: WeightStrategy,
    ) -> Result<f64, WFQError> {
        match strategy {
            WeightStrategy::BillingTier => match context.tenant_quota.billing_tier {
                BillingTier::Free => Ok(1.0),
                BillingTier::Standard => Ok(2.0),
                BillingTier::Premium => Ok(3.0),
                BillingTier::Enterprise => Ok(5.0),
            },
            WeightStrategy::QuotaBased => {
                let total_quota = context.tenant_quota.limits.max_cpu_cores as f64
                    + (context.tenant_quota.limits.max_memory_mb as f64 / 1024.0);
                Ok((total_quota / 100.0).max(1.0))
            }
            WeightStrategy::UsageHistory => {
                let quota = context.tenant_quota.limits.max_cpu_cores;
                if quota == 0 {
                    return Err(WFQError::WeightCalculationError(
                        "Zero quota for usage history calculation".to_string(),
                    ));
                }

                let usage_ratio = context.tenant_usage.current_cpu_cores as f64 / quota as f64;
                Ok((1.0 - usage_ratio * 0.5).max(self.config.min_weight))
            }
            WeightStrategy::Custom => {
                // Would need external weight source
                Ok(2.0)
            }
        }
    }

    /// Calculate packet size for resource request
    fn calculate_packet_size(&self, request: &ResourceRequest) -> u64 {
        // Simplified: sum of all requested resources
        (request.cpu_cores as u64 * 1000)
            + (request.memory_mb / 100)
            + (request.worker_count as u64 * 100)
    }

    /// Update starvation monitor for a tenant
    async fn update_starvation_monitor(&self, tenant_id: &str) {
        let mut starvation = self.starvation_monitor.write().await;
        if let Some(detection) = starvation.get_mut(tenant_id) {
            let now = Utc::now();
            if detection.last_allocation.is_none() {
                // First time in queue
                detection.first_seen = now;
            }
            detection.waiting_time = now
                .signed_duration_since(detection.first_seen)
                .to_std()
                .unwrap_or_default();
        }
    }

    /// Update allocation for a tenant
    async fn update_allocation(&self, tenant_id: &str) {
        let mut starvation = self.starvation_monitor.write().await;
        if let Some(detection) = starvation.get_mut(tenant_id) {
            detection.last_allocation = Some(Utc::now());
            detection.allocation_count += 1;
            detection.waiting_time = Duration::from_secs(0);
            detection.is_starving = false;
        }
    }
}

impl Default for WFQConfig {
    fn default() -> Self {
        Self {
            enable_virtual_time: true,
            min_weight: 0.5,
            max_weight: 10.0,
            default_strategy: WeightStrategy::QuotaBased,
            starvation_threshold: 0.1,
            weight_update_interval: Duration::from_secs(60),
            default_packet_size: 1500,
            enable_dynamic_weights: true,
            starvation_window: Duration::from_secs(300),
            fair_share_window: Duration::from_secs(120),
        }
    }
}

