//! Global Resource Controller (GRC)
//!
//! The GRC is responsible for tracking and managing resources across all
//! registered resource pools. It provides a centralized view of available
//! capacity and makes informed decisions about where to allocate resources
//! based on capacity, labels, and budget constraints.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::global_resource_controller::{
    BudgetPeriod, BudgetQuota, GovernanceError, PoolCapacity, PoolConfiguration, PoolQuery,
    PoolSelector, PoolStatus, RateLimit, ResourceAllocation, ResourcePoolInfo, ResourceQuota,
    ResourceType, SortPreference,
};
use crate::scheduler_port::{SchedulerError, SchedulerPort};

/// Global Resource Controller - Central resource management system
#[derive(Debug)]
pub struct GlobalResourceController<SP: SchedulerPort> {
    /// Registered resource pools
    pools: Arc<RwLock<HashMap<String, ResourcePoolInfo>>>,

    /// Active allocations
    allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,

    /// Scheduler port for coordination
    scheduler: Arc<SP>,

    /// Metrics collector for monitoring
    metrics: Arc<RwLock<ResourceControllerMetrics>>,

    /// Configuration
    config: GRCConfig,
}

/// GRC configuration
#[derive(Debug, Clone)]
pub struct GRCConfig {
    /// Default allocation timeout
    pub default_allocation_timeout: std::time::Duration,
    /// Default overcommit factor
    pub default_overcommit_factor: f64,
    /// Enable budget enforcement
    pub enable_budget_enforcement: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Metrics collection interval
    pub metrics_interval: std::time::Duration,
    /// Pool health check interval
    pub health_check_interval: std::time::Duration,
}

impl Default for GRCConfig {
    fn default() -> Self {
        Self {
            default_allocation_timeout: std::time::Duration::from_secs(300),
            default_overcommit_factor: 1.0,
            enable_budget_enforcement: true,
            enable_rate_limiting: true,
            metrics_interval: std::time::Duration::from_secs(60),
            health_check_interval: std::time::Duration::from_secs(30),
        }
    }
}

/// Metrics for the Global Resource Controller
#[derive(Debug, Default)]
pub struct ResourceControllerMetrics {
    /// Total pools registered
    pub total_pools: usize,
    /// Active pools
    pub active_pools: usize,
    /// Total capacity across all pools
    pub total_capacity: ResourceQuota,
    /// Total allocated resources
    pub total_allocated: ResourceQuota,
    /// Total available resources
    pub total_available: ResourceQuota,
    /// Number of active allocations
    pub active_allocations: usize,
    /// Allocation rate (per minute)
    pub allocation_rate: u32,
    /// Average allocation time
    pub avg_allocation_time_ms: u64,
    /// Budget utilization
    pub budget_utilization: HashMap<String, f64>,
}

impl<SP: SchedulerPort + Send + Sync> GlobalResourceController<SP> {
    /// Create a new Global Resource Controller
    pub fn new(scheduler: Arc<SP>, config: GRCConfig) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            allocations: Arc::new(RwLock::new(HashMap::new())),
            scheduler: Arc::new(scheduler),
            metrics: Arc::new(RwLock::new(ResourceControllerMetrics::default())),
            config,
        }
    }

    /// Register a new resource pool
    pub async fn register_pool(&self, pool: ResourcePoolInfo) -> Result<(), GovernanceError> {
        let mut pools = self.pools.write().await;

        // Check if pool already exists
        if pools.contains_key(&pool.id) {
            return Err(GovernanceError::PoolNotFound {
                pool_id: pool.id.clone(),
            });
        }

        // Validate pool configuration
        self.validate_pool_config(&pool)?;

        // Register the pool
        pools.insert(pool.id.clone(), pool);

        info!("Registered resource pool: {}", pool.id);

        // Update metrics
        self.update_metrics().await;

        Ok(())
    }

    /// Unregister a resource pool
    pub async fn unregister_pool(&self, pool_id: &str) -> Result<(), GovernanceError> {
        let mut pools = self.pools.write().await;

        if !pools.contains_key(pool_id) {
            return Err(GovernanceError::PoolNotFound {
                pool_id: pool_id.to_string(),
            });
        }

        // Check if pool has active allocations
        let allocations = self.allocations.read().await;
        let has_allocations = allocations.values().any(|alloc| alloc.pool_id == pool_id);

        if has_allocations {
            // Set pool to draining instead of removing
            if let Some(pool) = pools.get_mut(pool_id) {
                pool.capacity.status = PoolStatus::Draining;
                info!("Set pool to draining: {}", pool_id);
            }
        } else {
            pools.remove(pool_id);
            info!("Unregistered resource pool: {}", pool_id);
        }

        // Update metrics
        self.update_metrics().await;

        Ok(())
    }

    /// Find suitable resource pools based on query
    pub async fn find_pools(
        &self,
        query: PoolQuery,
    ) -> Result<Vec<ResourcePoolInfo>, GovernanceError> {
        let pools = self.pools.read().await;

        // Filter pools by selector and capacity
        let mut suitable_pools: Vec<_> = pools
            .values()
            .filter(|pool| {
                // Check if pool matches label selector
                query.selector.matches(&pool.labels)
                    && pool.capacity.can_accommodate(&query.resource_requirements)
                    && pool.capacity.status == PoolStatus::Active
            })
            .cloned()
            .collect();

        // Sort pools by preference
        self.sort_pools(&mut suitable_pools, &query.sort_by);

        // Apply limit
        if let Some(limit) = query.limit {
            suitable_pools.truncate(limit);
        }

        debug!("Found {} suitable pools for query", suitable_pools.len());

        Ok(suitable_pools)
    }

    /// Allocate resources from a pool
    pub async fn allocate_resources(
        &self,
        allocation_id: String,
        pool_id: &str,
        resources: ResourceQuota,
    ) -> Result<ResourceAllocation, GovernanceError> {
        let mut pools = self.pools.write().await;
        let mut allocations = self.allocations.write().await;

        // Get pool
        let pool = pools
            .get_mut(pool_id)
            .ok_or_else(|| GovernanceError::PoolNotFound {
                pool_id: pool_id.to_string(),
            })?;

        // Check pool status
        if pool.capacity.status != PoolStatus::Active {
            return Err(GovernanceError::PoolPaused {
                pool_id: pool_id.to_string(),
            });
        }

        // Check budget if enabled
        if self.config.enable_budget_enforcement {
            if let Some(budget) = &pool.config.budget {
                if budget.would_exceed(0.0) {
                    return Err(GovernanceError::BudgetExceeded {
                        pool_id: pool_id.to_string(),
                        budget: budget.clone(),
                        used: budget.clone(),
                    });
                }
            }
        }

        // Check capacity
        if !pool.capacity.can_accommodate(&resources) {
            return Err(GovernanceError::InsufficientCapacity {
                pool_id: pool_id.to_string(),
                requested: resources.clone(),
                available: pool.capacity.available.clone(),
            });
        }

        // Create allocation
        let allocation =
            ResourceAllocation::new(allocation_id, pool_id.to_string(), resources.clone());

        // Update pool capacity
        pool.capacity.allocated = pool.capacity.allocated.add(&resources);
        pool.capacity.available = pool.capacity.total.subtract(&pool.capacity.allocated);

        // Update pool's budget if applicable
        if let Some(budget) = &mut pool.config.budget {
            // This is a simplified model - in reality, you'd track cost per allocation
            budget.used += 0.0;
        }

        // Store allocation
        allocations.insert(allocation.id.clone(), allocation.clone());

        info!("Allocated resources from pool {}: {:?}", pool_id, resources);

        Ok(allocation)
    }

    /// Release resources from a pool
    pub async fn release_resources(&self, allocation_id: &str) -> Result<(), GovernanceError> {
        let mut pools = self.pools.write().await;
        let mut allocations = self.allocations.write().await;

        // Get allocation
        let allocation = allocations.remove(allocation_id).ok_or_else(|| {
            GovernanceError::AllocationNotFound {
                allocation_id: allocation_id.to_string(),
            }
        })?;

        // Get pool
        let pool =
            pools
                .get_mut(&allocation.pool_id)
                .ok_or_else(|| GovernanceError::PoolNotFound {
                    pool_id: allocation.pool_id.clone(),
                })?;

        // Update pool capacity
        pool.capacity.allocated = pool.capacity.allocated.subtract(&allocation.resources);
        pool.capacity.available = pool.capacity.total.subtract(&pool.capacity.allocated);

        // Update pool's budget if applicable
        if let Some(budget) = &mut pool.config.budget {
            // This is a simplified model - in reality, you'd track cost per allocation
            budget.used -= 0.0;
        }

        info!(
            "Released resources from pool {}: {:?}",
            allocation.pool_id, allocation.resources
        );

        Ok(())
    }

    /// Get pool status
    pub async fn get_pool_status(&self, pool_id: &str) -> Result<PoolCapacity, GovernanceError> {
        let pools = self.pools.read().await;
        let pool = pools
            .get(pool_id)
            .ok_or_else(|| GovernanceError::PoolNotFound {
                pool_id: pool_id.to_string(),
            })?;

        Ok(pool.capacity.clone())
    }

    /// Get all pools
    pub async fn get_all_pools(&self) -> Vec<ResourcePoolInfo> {
        let pools = self.pools.read().await;
        pools.values().cloned().collect()
    }

    /// Get active allocations
    pub async fn get_active_allocations(&self) -> Vec<ResourceAllocation> {
        let allocations = self.allocations.read().await;
        allocations.values().cloned().collect()
    }

    /// Get controller metrics
    pub async fn get_metrics(&self) -> ResourceControllerMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Update pool status (pause, resume, etc.)
    pub async fn update_pool_status(
        &self,
        pool_id: &str,
        status: PoolStatus,
    ) -> Result<(), GovernanceError> {
        let mut pools = self.pools.write().await;

        let pool = pools
            .get_mut(pool_id)
            .ok_or_else(|| GovernanceError::PoolNotFound {
                pool_id: pool_id.to_string(),
            })?;

        pool.capacity.status = status.clone();
        pool.updated_at = std::time::SystemTime::now();

        info!("Updated pool status: {} -> {:?}", pool_id, status);

        Ok(())
    }

    /// Clean up expired allocations
    pub async fn cleanup_expired_allocations(&self) -> Result<u32, GovernanceError> {
        let mut pools = self.pools.write().await;
        let mut allocations = self.allocations.write().await;

        let now = std::time::SystemTime::now();
        let mut cleaned = 0;

        // Find expired allocations
        let expired_ids: Vec<String> = allocations
            .values()
            .filter(|alloc| {
                if let Some(expires_at) = alloc.expires_at {
                    now > expires_at
                } else {
                    false
                }
            })
            .map(|alloc| alloc.id.clone())
            .collect();

        // Release expired allocations
        for allocation_id in expired_ids {
            if let Some(allocation) = allocations.remove(&allocation_id) {
                // Update pool capacity
                if let Some(pool) = pools.get_mut(&allocation.pool_id) {
                    pool.capacity.allocated =
                        pool.capacity.allocated.subtract(&allocation.resources);
                    pool.capacity.available =
                        pool.capacity.total.subtract(&pool.capacity.allocated);
                }
                cleaned += 1;
            }
        }

        if cleaned > 0 {
            info!("Cleaned up {} expired allocations", cleaned);
            self.update_metrics().await;
        }

        Ok(cleaned)
    }

    /// Validate pool configuration
    fn validate_pool_config(&self, pool: &ResourcePoolInfo) -> Result<(), GovernanceError> {
        // Check if total capacity is non-negative
        if pool.capacity.total.cpu.is_some() {
            let cpu = pool.capacity.total.cpu.as_ref().unwrap();
            if cpu.amount < 0.0 {
                return Err(GovernanceError::Validation {
                    message: "CPU amount cannot be negative".to_string(),
                });
            }
        }

        if pool.capacity.total.memory.is_some() {
            let memory = pool.capacity.total.memory.as_ref().unwrap();
            if memory.amount < 0.0 {
                return Err(GovernanceError::Validation {
                    message: "Memory amount cannot be negative".to_string(),
                });
            }
        }

        // Check overcommit factor
        if let Some(factor) = pool.config.overcommit_factor {
            if factor < 1.0 {
                return Err(GovernanceError::Validation {
                    message: "Overcommit factor must be >= 1.0".to_string(),
                });
            }
        }

        // Check budget validity
        if let Some(budget) = &pool.config.budget {
            if budget.total < 0.0 {
                return Err(GovernanceError::Validation {
                    message: "Budget total cannot be negative".to_string(),
                });
            }
            if budget.used > budget.total {
                return Err(GovernanceError::Validation {
                    message: "Used budget cannot exceed total budget".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Sort pools by preference
    fn sort_pools(&self, pools: &mut [ResourcePoolInfo], sort_by: &SortPreference) {
        match sort_by {
            SortPreference::MostCapacity => {
                pools.sort_by(|a, b| {
                    b.capacity
                        .available
                        .cpu
                        .as_ref()
                        .unwrap()
                        .amount
                        .partial_cmp(&a.capacity.available.cpu.as_ref().unwrap().amount)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortPreference::LowestCost => {
                // This would require cost information in pool config
                // For now, just leave as-is
            }
            SortPreference::HighestUtilization => {
                pools.sort_by(|a, b| {
                    a.capacity
                        .utilization_percentage()
                        .partial_cmp(&b.capacity.utilization_percentage())
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .reverse()
                });
            }
            SortPreference::LabelMatch => {
                // This is already handled by the selector matching
            }
        }
    }

    /// Update controller metrics
    async fn update_metrics(&self) {
        let pools = self.pools.read().await;
        let allocations = self.allocations.read().await;

        let mut metrics = ResourceControllerMetrics::default();

        // Count pools
        metrics.total_pools = pools.len();
        metrics.active_pools = pools
            .values()
            .filter(|p| p.capacity.status == PoolStatus::Active)
            .count();

        // Aggregate capacity
        for pool in pools.values() {
            metrics.total_capacity = metrics.total_capacity.add(&pool.capacity.total);
            metrics.total_allocated = metrics.total_allocated.add(&pool.capacity.allocated);
            metrics.total_available = metrics.total_available.add(&pool.capacity.available);
        }

        // Count allocations
        metrics.active_allocations = allocations.len();

        // Update budget utilization
        for pool in pools.values() {
            if let Some(budget) = &pool.config.budget {
                metrics
                    .budget_utilization
                    .insert(pool.id.clone(), budget.usage_percentage());
            }
        }

        *self.metrics.write().await = metrics;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler_port::MockSchedulerPort;

    fn create_test_pool(id: &str, total_cpu: f64, total_memory: f64) -> ResourcePoolInfo {
        let total = ResourceQuota::new()
            .with_cpu(total_cpu, "cores")
            .with_memory(total_memory, "Gi");

        ResourcePoolInfo {
            id: id.to_string(),
            name: format!("Pool {}", id),
            pool_type: "kubernetes".to_string(),
            capacity: PoolCapacity::new(total),
            labels: HashMap::new(),
            config: PoolConfiguration {
                max_concurrent_allocations: None,
                allocation_timeout: None,
                budget: None,
                rate_limit: None,
                overcommit_factor: None,
            },
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_register_pool() {
        let scheduler = Arc::new(MockSchedulerPort::new());
        let grc = GlobalResourceController::new(scheduler, GRCConfig::default());

        let pool = create_test_pool("pool-1", 8.0, 16.0);

        let result = grc.register_pool(pool.clone()).await;

        assert!(result.is_ok());

        let pools = grc.get_all_pools().await;
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].id, "pool-1");
    }

    #[tokio::test]
    async fn test_allocate_resources() {
        let scheduler = Arc::new(MockSchedulerPort::new());
        let grc = GlobalResourceController::new(scheduler, GRCConfig::default());

        let pool = create_test_pool("pool-1", 8.0, 16.0);
        grc.register_pool(pool).await.unwrap();

        let request = ResourceQuota::new()
            .with_cpu(2.0, "cores")
            .with_memory(4.0, "Gi");

        let result = grc
            .allocate_resources("alloc-1".to_string(), "pool-1", request.clone())
            .await;

        assert!(result.is_ok());

        let status = grc.get_pool_status("pool-1").await.unwrap();
        assert_eq!(status.allocated.cpu.as_ref().unwrap().amount, 2.0);
        assert_eq!(status.available.cpu.as_ref().unwrap().amount, 6.0);
    }

    #[tokio::test]
    async fn test_insufficient_capacity() {
        let scheduler = Arc::new(MockSchedulerPort::new());
        let grc = GlobalResourceController::new(scheduler, GRCConfig::default());

        let pool = create_test_pool("pool-1", 4.0, 8.0);
        grc.register_pool(pool).await.unwrap();

        let request = ResourceQuota::new()
            .with_cpu(8.0, "cores")
            .with_memory(16.0, "Gi");

        let result = grc
            .allocate_resources("alloc-1".to_string(), "pool-1", request)
            .await;

        assert!(matches!(
            result,
            Err(GovernanceError::InsufficientCapacity { .. })
        ));
    }

    #[tokio::test]
    async fn test_release_resources() {
        let scheduler = Arc::new(MockSchedulerPort::new());
        let grc = GlobalResourceController::new(scheduler, GRCConfig::default());

        let pool = create_test_pool("pool-1", 8.0, 16.0);
        grc.register_pool(pool).await.unwrap();

        let request = ResourceQuota::new()
            .with_cpu(2.0, "cores")
            .with_memory(4.0, "Gi");

        grc.allocate_resources("alloc-1".to_string(), "pool-1", request)
            .await
            .unwrap();

        let result = grc.release_resources("alloc-1").await;

        assert!(result.is_ok());

        let status = grc.get_pool_status("pool-1").await.unwrap();
        assert_eq!(status.allocated.cpu.as_ref().unwrap().amount, 0.0);
        assert_eq!(status.available.cpu.as_ref().unwrap().amount, 8.0);
    }

    #[tokio::test]
    async fn test_pool_filtering() {
        let scheduler = Arc::new(MockSchedulerPort::new());
        let grc = GlobalResourceController::new(scheduler, GRCConfig::default());

        // Register pools
        let mut pool1 = create_test_pool("pool-1", 8.0, 16.0);
        pool1
            .labels
            .insert("region".to_string(), "us-east".to_string());

        let mut pool2 = create_test_pool("pool-2", 4.0, 8.0);
        pool2
            .labels
            .insert("region".to_string(), "us-west".to_string());

        grc.register_pool(pool1).await.unwrap();
        grc.register_pool(pool2).await.unwrap();

        // Query for us-east pools
        let query = PoolQuery::new(ResourceQuota::new().with_cpu(2.0, "cores"))
            .with_required_label("region", "us-east");

        let pools = grc.find_pools(query).await.unwrap();

        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].id, "pool-1");
    }

    #[tokio::test]
    async fn test_pool_status_update() {
        let scheduler = Arc::new(MockSchedulerPort::new());
        let grc = GlobalResourceController::new(scheduler, GRCConfig::default());

        let pool = create_test_pool("pool-1", 8.0, 16.0);
        grc.register_pool(pool).await.unwrap();

        let result = grc.update_pool_status("pool-1", PoolStatus::Paused).await;

        assert!(result.is_ok());

        let status = grc.get_pool_status("pool-1").await.unwrap();
        assert_eq!(status.status, PoolStatus::Paused);
    }
}
