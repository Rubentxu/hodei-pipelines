//! Resource Governance System
//!
//! This module defines the types and structures for tracking and managing
//! resources across multiple resource pools. It provides the foundation for
//! the Global Resource Controller (GRC) to make informed decisions about
//! resource allocation.

pub mod resource_pool;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Errors that can occur during resource governance operations.
#[derive(Error, Debug)]
pub enum GovernanceError {
    #[error("Resource pool not found: {pool_id}")]
    PoolNotFound { pool_id: String },

    #[error(
        "Insufficient capacity in pool {pool_id}: requested {requested:?}, available {available:?}"
    )]
    InsufficientCapacity {
        pool_id: String,
        requested: ResourceQuota,
        available: ResourceQuota,
    },

    #[error("Allocation not found: {allocation_id}")]
    AllocationNotFound { allocation_id: String },

    #[error("Resource validation failed: {message}")]
    Validation { message: String },

    #[error("Budget exceeded for pool {pool_id}: budget {budget}, used {used}")]
    BudgetExceeded {
        pool_id: String,
        budget: BudgetQuota,
        used: BudgetQuota,
    },

    #[error("Pool is paused: {pool_id}")]
    PoolPaused { pool_id: String },

    #[error("Rate limit exceeded for pool {pool_id}")]
    RateLimitExceeded { pool_id: String },
}

/// Resource types tracked by the governance system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// CPU cores (e.g., "2.0", "2000m")
    Cpu,
    /// Memory in bytes or human-readable format
    Memory,
    /// Ephemeral storage in bytes
    EphemeralStorage,
    /// GPU count or type
    Gpu,
    /// Custom resource type
    Custom(String),
}

/// Resource quantity with unit information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceQuantity {
    /// Amount of the resource
    pub amount: f64,
    /// Unit (e.g., "cores", "Gi", "bytes")
    pub unit: String,
}

impl ResourceQuantity {
    /// Create a new resource quantity
    pub fn new(amount: f64, unit: &str) -> Self {
        Self {
            amount,
            unit: unit.to_string(),
        }
    }

    /// Check if this quantity is greater than or equal to another
    pub fn is_at_least(&self, other: &ResourceQuantity) -> bool {
        self.amount >= other.amount && self.unit == other.unit
    }

    /// Check if this quantity is less than or equal to another
    pub fn is_at_most(&self, other: &ResourceQuantity) -> bool {
        self.amount <= other.amount && self.unit == other.unit
    }
}

/// Complete resource quota specification
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// CPU cores
    pub cpu: Option<ResourceQuantity>,
    /// Memory
    pub memory: Option<ResourceQuantity>,
    /// Ephemeral storage
    pub storage: Option<ResourceQuantity>,
    /// GPU count
    pub gpu: Option<u32>,
    /// Custom resources
    pub custom: HashMap<String, ResourceQuantity>,
}

impl ResourceQuota {
    /// Create an empty resource quota
    pub fn new() -> Self {
        Self::default()
    }

    /// Add CPU to the quota
    pub fn with_cpu(mut self, amount: f64, unit: &str) -> Self {
        self.cpu = Some(ResourceQuantity::new(amount, unit));
        self
    }

    /// Add memory to the quota
    pub fn with_memory(mut self, amount: f64, unit: &str) -> Self {
        self.memory = Some(ResourceQuantity::new(amount, unit));
        self
    }

    /// Add storage to the quota
    pub fn with_storage(mut self, amount: f64, unit: &str) -> Self {
        self.storage = Some(ResourceQuantity::new(amount, unit));
        self
    }

    /// Add GPU to the quota
    pub fn with_gpu(mut self, count: u32) -> Self {
        self.gpu = Some(count);
        self
    }

    /// Add a custom resource
    pub fn with_custom(mut self, name: &str, amount: f64, unit: &str) -> Self {
        self.custom
            .insert(name.to_string(), ResourceQuantity::new(amount, unit));
        self
    }

    /// Check if this quota has sufficient resources for another quota
    pub fn can_accommodate(&self, other: &ResourceQuota) -> bool {
        if let (Some(self_cpu), Some(other_cpu)) = (&self.cpu, &other.cpu)
            && !self_cpu.is_at_least(other_cpu) {
                return false;
            }
        if let (Some(self_mem), Some(other_mem)) = (&self.memory, &other.memory)
            && !self_mem.is_at_least(other_mem) {
                return false;
            }
        if let (Some(self_storage), Some(other_storage)) = (&self.storage, &other.storage)
            && !self_storage.is_at_least(other_storage) {
                return false;
            }
        if let (Some(self_gpu), Some(other_gpu)) = (&self.gpu, &other.gpu)
            && self_gpu < other_gpu {
                return false;
            }
        // Check custom resources
        for (name, required) in &other.custom {
            if let Some(available) = self.custom.get(name) {
                if !available.is_at_least(required) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    /// Add two quotas together
    pub fn add(&self, other: &ResourceQuota) -> ResourceQuota {
        let mut result = self.clone();

        if let Some(cpu) = &other.cpu {
            result.cpu = Some(cpu.clone());
        }
        if let Some(memory) = &other.memory {
            result.memory = Some(memory.clone());
        }
        if let Some(storage) = &other.storage {
            result.storage = Some(storage.clone());
        }
        if other.gpu.is_some() {
            result.gpu = other.gpu;
        }

        for (name, qty) in &other.custom {
            result.custom.insert(name.clone(), qty.clone());
        }

        result
    }

    /// Subtract one quota from another
    pub fn subtract(&self, other: &ResourceQuota) -> ResourceQuota {
        let mut result = self.clone();

        if other.cpu.is_some() {
            result.cpu = None;
        }
        if other.memory.is_some() {
            result.memory = None;
        }
        if other.storage.is_some() {
            result.storage = None;
        }
        if other.gpu.is_some() {
            result.gpu = None;
        }

        for name in other.custom.keys() {
            result.custom.remove(name);
        }

        result
    }
}

/// Budget quota for cost tracking
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BudgetQuota {
    /// Total budget amount
    pub total: f64,
    /// Amount already used
    pub used: f64,
    /// Currency code (e.g., "USD", "EUR")
    pub currency: String,
    /// Budget period (hourly, daily, monthly)
    pub period: BudgetPeriod,
}

impl std::fmt::Display for BudgetQuota {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} {}/month", self.total, self.currency)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum BudgetPeriod {
    Hourly,
    Daily,
    Weekly,
    #[default]
    Monthly,
    Yearly,
}


impl BudgetQuota {
    /// Create a new budget quota
    pub fn new(total: f64, currency: &str, period: BudgetPeriod) -> Self {
        Self {
            total,
            used: 0.0,
            currency: currency.to_string(),
            period,
        }
    }

    /// Check if budget has remaining capacity
    pub fn has_remaining(&self) -> bool {
        self.used < self.total
    }

    /// Get remaining budget
    pub fn remaining(&self) -> f64 {
        self.total - self.used
    }

    /// Calculate usage percentage
    pub fn usage_percentage(&self) -> f64 {
        if self.total == 0.0 {
            0.0
        } else {
            (self.used / self.total) * 100.0
        }
    }

    /// Check if adding an amount would exceed budget
    pub fn would_exceed(&self, amount: f64) -> bool {
        self.used + amount > self.total
    }
}

/// Pool status for governance decisions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolStatus {
    /// Pool is active and accepting allocations
    Active,
    /// Pool is paused (no new allocations)
    Paused,
    /// Pool is draining (existing allocations continue)
    Draining,
    /// Pool is offline
    Offline,
}

/// Pool capacity information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolCapacity {
    /// Total capacity of the pool
    pub total: ResourceQuota,
    /// Currently allocated resources
    pub allocated: ResourceQuota,
    /// Available capacity (total - allocated)
    pub available: ResourceQuota,
    /// Reserved capacity (for burst or priority jobs)
    pub reserved: ResourceQuota,
    /// Pool status
    pub status: PoolStatus,
}

impl PoolCapacity {
    /// Create a new pool capacity
    pub fn new(total: ResourceQuota) -> Self {
        Self {
            total: total.clone(),
            allocated: ResourceQuota::new(),
            available: total,
            reserved: ResourceQuota::new(),
            status: PoolStatus::Active,
        }
    }

    /// Check if pool can accommodate a resource request
    pub fn can_accommodate(&self, request: &ResourceQuota) -> bool {
        self.status == PoolStatus::Active && self.available.can_accommodate(request)
    }

    /// Get the effective available capacity (excluding reserved)
    pub fn effective_available(&self) -> ResourceQuota {
        self.available.subtract(&self.reserved)
    }

    /// Calculate utilization percentage
    pub fn utilization_percentage(&self) -> f64 {
        let mut total_cpu = 0.0;
        let mut allocated_cpu = 0.0;

        if let Some(cpu) = &self.total.cpu {
            total_cpu += cpu.amount;
        }
        if let Some(cpu) = &self.allocated.cpu {
            allocated_cpu += cpu.amount;
        }

        if total_cpu == 0.0 {
            0.0
        } else {
            (allocated_cpu / total_cpu) * 100.0
        }
    }
}

/// Pool label selector for filtering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolSelector {
    /// Required labels (all must match)
    pub required_labels: HashMap<String, String>,
    /// Preferred labels (best effort match)
    pub preferred_labels: HashMap<String, String>,
    /// Excluded labels (must not match)
    pub excluded_labels: HashMap<String, String>,
}

impl PoolSelector {
    /// Create a new pool selector
    pub fn new() -> Self {
        Self {
            required_labels: HashMap::new(),
            preferred_labels: HashMap::new(),
            excluded_labels: HashMap::new(),
        }
    }

    /// Add a required label
    pub fn with_required_label(mut self, key: &str, value: &str) -> Self {
        self.required_labels
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Add a preferred label
    pub fn with_preferred_label(mut self, key: &str, value: &str) -> Self {
        self.preferred_labels
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Add an excluded label
    pub fn with_excluded_label(mut self, key: &str, value: &str) -> Self {
        self.excluded_labels
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Check if a pool's labels match this selector
    pub fn matches(&self, pool_labels: &HashMap<String, String>) -> bool {
        // Check required labels
        for (key, value) in &self.required_labels {
            if pool_labels.get(key) != Some(value) {
                return false;
            }
        }

        // Check excluded labels
        for (key, value) in &self.excluded_labels {
            if pool_labels.get(key) == Some(value) {
                return false;
            }
        }

        true
    }

    /// Calculate match score (for preference ordering)
    pub fn match_score(&self, pool_labels: &HashMap<String, String>) -> usize {
        let mut score: usize = 0;

        // Required labels must match - if any don't match, score is 0
        for (key, value) in &self.required_labels {
            if pool_labels.get(key) != Some(value) {
                return 0;
            }
            score += 10;
        }

        // Preferred labels get bonus points
        for (key, value) in &self.preferred_labels {
            if pool_labels.get(key) == Some(value) {
                score += 5;
            }
        }

        // Excluded labels subtract points
        for (key, value) in &self.excluded_labels {
            if pool_labels.get(key) == Some(value) {
                score = score.saturating_sub(5);
            }
        }

        score
    }
}

impl Default for PoolSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource allocation tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Unique allocation ID
    pub id: String,
    /// Pool where resources are allocated
    pub pool_id: String,
    /// Allocated resources
    pub resources: ResourceQuota,
    /// Allocation timestamp
    pub created_at: SystemTime,
    /// Allocation expiry (if temporary)
    pub expires_at: Option<SystemTime>,
    /// Allocation metadata
    pub metadata: HashMap<String, String>,
}

impl ResourceAllocation {
    /// Create a new resource allocation
    pub fn new(id: String, pool_id: String, resources: ResourceQuota) -> Self {
        Self {
            id,
            pool_id,
            resources,
            created_at: SystemTime::now(),
            expires_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if allocation has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }

    /// Get allocation age
    pub fn age(&self) -> Duration {
        self.created_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    }
}

/// Resource pool information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcePoolInfo {
    /// Pool ID
    pub id: String,
    /// Pool name
    pub name: String,
    /// Pool type (K8s, Docker, VM, etc.)
    pub pool_type: String,
    /// Pool capacity
    pub capacity: PoolCapacity,
    /// Pool labels for filtering
    pub labels: HashMap<String, String>,
    /// Pool configuration
    pub config: PoolConfiguration,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last updated timestamp
    pub updated_at: SystemTime,
}

/// Pool configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolConfiguration {
    /// Maximum concurrent allocations
    pub max_concurrent_allocations: Option<u32>,
    /// Allocation timeout (how long to wait for resources)
    pub allocation_timeout: Option<Duration>,
    /// Budget limits
    pub budget: Option<BudgetQuota>,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimit>,
    /// Overcommit factor (e.g., 1.0 = no overcommit, 1.5 = 50% overcommit)
    pub overcommit_factor: Option<f64>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum allocations per minute
    pub per_minute: Option<u32>,
    /// Maximum allocations per hour
    pub per_hour: Option<u32>,
    /// Maximum allocations per day
    pub per_day: Option<u32>,
}

impl RateLimit {
    /// Create a new rate limit
    pub fn new() -> Self {
        Self {
            per_minute: None,
            per_hour: None,
            per_day: None,
        }
    }

    /// Set per-minute limit
    pub fn per_minute(mut self, limit: u32) -> Self {
        self.per_minute = Some(limit);
        self
    }

    /// Set per-hour limit
    pub fn per_hour(mut self, limit: u32) -> Self {
        self.per_hour = Some(limit);
        self
    }

    /// Set per-day limit
    pub fn per_day(mut self, limit: u32) -> Self {
        self.per_day = Some(limit);
        self
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self::new()
    }
}

/// Query for finding suitable resource pools
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolQuery {
    /// Resource requirements
    pub resource_requirements: ResourceQuota,
    /// Pool selector with label filters
    pub selector: PoolSelector,
    /// Budget constraints (optional)
    pub budget_constraint: Option<BudgetQuota>,
    /// Maximum acceptable cost
    pub max_cost_per_hour: Option<f64>,
    /// Sort order preference
    pub sort_by: SortPreference,
    /// Maximum pools to return
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortPreference {
    /// Prefer pools with most available capacity
    MostCapacity,
    /// Prefer pools with lowest cost
    LowestCost,
    /// Prefer pools with highest utilization (for efficiency)
    HighestUtilization,
    /// Prefer pools with specific labels
    LabelMatch,
    /// Use scoring algorithm (bin packing + cost + labels + load balancing)
    ScoringAlgorithm,
}

impl Default for SortPreference {
    fn default() -> Self {
        Self::MostCapacity
    }
}

impl PoolQuery {
    /// Create a new pool query
    pub fn new(resource_requirements: ResourceQuota) -> Self {
        Self {
            resource_requirements,
            selector: PoolSelector::new(),
            budget_constraint: None,
            max_cost_per_hour: None,
            sort_by: SortPreference::MostCapacity,
            limit: None,
        }
    }

    /// Add a required label
    pub fn with_required_label(mut self, key: &str, value: &str) -> Self {
        self.selector = self.selector.with_required_label(key, value);
        self
    }

    /// Set budget constraint
    pub fn with_budget(mut self, budget: BudgetQuota) -> Self {
        self.budget_constraint = Some(budget);
        self
    }

    /// Set maximum cost per hour
    pub fn with_max_cost_per_hour(mut self, cost: f64) -> Self {
        self.max_cost_per_hour = Some(cost);
        self
    }

    /// Set sort preference
    pub fn with_sort_by(mut self, sort_by: SortPreference) -> Self {
        self.sort_by = sort_by;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_quota_creation() {
        let quota = ResourceQuota::new()
            .with_cpu(2.0, "cores")
            .with_memory(4.0, "Gi")
            .with_gpu(1);

        assert_eq!(quota.cpu.unwrap().amount, 2.0);
        assert_eq!(quota.memory.unwrap().amount, 4.0);
        assert_eq!(quota.gpu, Some(1));
    }

    #[test]
    fn test_resource_quota_accommodation() {
        let total = ResourceQuota::new()
            .with_cpu(4.0, "cores")
            .with_memory(8.0, "Gi");

        let request = ResourceQuota::new()
            .with_cpu(2.0, "cores")
            .with_memory(4.0, "Gi");

        assert!(total.can_accommodate(&request));

        let big_request = ResourceQuota::new()
            .with_cpu(8.0, "cores")
            .with_memory(16.0, "Gi");

        assert!(!total.can_accommodate(&big_request));
    }

    #[test]
    fn test_pool_selector_matching() {
        let selector = PoolSelector::new()
            .with_required_label("environment", "production")
            .with_preferred_label("region", "us-east");

        let mut pool_labels = HashMap::new();
        pool_labels.insert("environment".to_string(), "production".to_string());
        pool_labels.insert("region".to_string(), "us-east".to_string());

        assert!(selector.matches(&pool_labels));
        assert_eq!(selector.match_score(&pool_labels), 15);
    }

    #[test]
    fn test_budget_quota() {
        let budget = BudgetQuota::new(1000.0, "USD", BudgetPeriod::Monthly);
        assert_eq!(budget.remaining(), 1000.0);
        assert_eq!(budget.usage_percentage(), 0.0);

        // Simulate usage
        let budget_after = BudgetQuota {
            total: 1000.0,
            used: 500.0,
            currency: "USD".to_string(),
            period: BudgetPeriod::Monthly,
        };

        assert_eq!(budget_after.remaining(), 500.0);
        assert_eq!(budget_after.usage_percentage(), 50.0);
    }

    #[test]
    fn test_pool_capacity() {
        let total = ResourceQuota::new()
            .with_cpu(8.0, "cores")
            .with_memory(16.0, "Gi");

        let mut capacity = PoolCapacity::new(total);
        assert_eq!(capacity.utilization_percentage(), 0.0);

        // Allocate resources
        capacity.allocated = ResourceQuota::new()
            .with_cpu(4.0, "cores")
            .with_memory(8.0, "Gi");

        assert_eq!(capacity.utilization_percentage(), 50.0);
    }
}
