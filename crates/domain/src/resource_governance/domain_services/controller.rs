//! Global Resource Controller (GRC)
//!
//! The GRC is responsible for tracking and managing resources across all
//! registered resource pools. It provides a centralized view of available
//! capacity and makes informed decisions about where to allocate resources
//! based on capacity, labels, and budget constraints.

use crate::resource_governance::{
    ComputePool, PoolCapacity, PoolId, PoolStatus, ResourcePoolRepository, ResourceRequest,
    TenantQuota,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{error, info};

/// Configuration for the Global Resource Controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GRCConfig {
    pub default_allocation_timeout_secs: u64,
    pub enable_quota_enforcement: bool,
    pub enable_cost_tracking: bool,
    pub max_allocation_wait_secs: u64,
}

impl Default for GRCConfig {
    fn default() -> Self {
        Self {
            default_allocation_timeout_secs: 300,
            enable_quota_enforcement: true,
            enable_cost_tracking: true,
            max_allocation_wait_secs: 60,
        }
    }
}

/// Resource allocation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllocationResult {
    pub allocation_id: String,
    pub pool_id: PoolId,
    pub resources: PoolCapacity,
    pub estimated_cost_per_hour: Option<f64>,
    pub expires_at: Option<i64>,
}

/// Pool selection result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolSelectionResult {
    pub selected_pool: ComputePool,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Metrics for the Global Resource Controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GRCMetrics {
    pub total_pools: usize,
    pub active_pools: usize,
    pub total_capacity: PoolCapacity,
    pub total_allocated: PoolCapacity,
    pub total_available: PoolCapacity,
    pub active_allocations: usize,
    pub average_allocation_latency_ms: u64,
    pub queue_depth: usize,
}

/// Global Resource Controller with interior mutability
#[derive(Clone)]
pub struct GlobalResourceController {
    config: GRCConfig,
    repository: Arc<dyn ResourcePoolRepository>,
    inner: Arc<RwLock<GRCInner>>,
}

/// Inner state of GRC protected by RwLock
struct GRCInner {
    pools: HashMap<PoolId, ComputePool>,
    tenant_quotas: HashMap<String, TenantQuota>,
    active_tenant_jobs: HashMap<String, u32>, // Track concurrent jobs per tenant
    tenant_resource_usage: HashMap<String, PoolCapacity>, // Track allocated resources per tenant
    metrics: GRCMetrics,
}

impl std::fmt::Debug for GlobalResourceController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.read().unwrap();
        f.debug_struct("GlobalResourceController")
            .field("config", &self.config)
            .field("pools", &inner.pools)
            .field("tenant_quotas", &inner.tenant_quotas)
            .field("metrics", &inner.metrics)
            .finish()
    }
}

impl GlobalResourceController {
    /// Create a new GRC instance
    pub fn new(config: GRCConfig, repository: Arc<dyn ResourcePoolRepository>) -> Self {
        Self {
            config,
            repository,
            inner: Arc::new(RwLock::new(GRCInner {
                pools: HashMap::new(),
                tenant_quotas: HashMap::new(),
                active_tenant_jobs: HashMap::new(),
                tenant_resource_usage: HashMap::new(),
                metrics: GRCMetrics {
                    total_pools: 0,
                    active_pools: 0,
                    total_capacity: PoolCapacity {
                        cpu_millicores: 0,
                        memory_mb: 0,
                        gpu_count: 0,
                        max_workers: 0,
                        active_workers: 0,
                        storage_gb: None,
                    },
                    total_allocated: PoolCapacity {
                        cpu_millicores: 0,
                        memory_mb: 0,
                        gpu_count: 0,
                        max_workers: 0,
                        active_workers: 0,
                        storage_gb: None,
                    },
                    total_available: PoolCapacity {
                        cpu_millicores: 0,
                        memory_mb: 0,
                        gpu_count: 0,
                        max_workers: 0,
                        active_workers: 0,
                        storage_gb: None,
                    },
                    active_allocations: 0,
                    average_allocation_latency_ms: 0,
                    queue_depth: 0,
                },
            })),
        }
    }

    /// Initialize GRC by loading pools from repository
    pub async fn initialize(&self) -> Result<(), String> {
        info!("Initializing Global Resource Controller");
        match self.repository.list().await {
            Ok(configs) => {
                let mut inner = self.inner.write().unwrap();
                for config in configs {
                    let pool = ComputePool::builder()
                        .id(PoolId(config.name.clone()))
                        .name(config.name.clone())
                        .provider_type(config.provider_type)
                        .total_capacity(PoolCapacity {
                            cpu_millicores: config.max_size as u64 * config.default_resources.cpu_m,
                            memory_mb: config.max_size as u64 * config.default_resources.memory_mb,
                            gpu_count: config.max_size
                                * config.default_resources.gpu.unwrap_or(0) as u32,
                            max_workers: config.max_size,
                            active_workers: 0,
                            storage_gb: None, // Config doesn't have storage yet
                        })
                        .labels(config.tags)
                        .build()
                        .map_err(|e| format!("Failed to build pool from config: {}", e))?;

                    Self::register_pool_inner(&mut inner, pool)?;
                }
                info!("Loaded {} pools from repository", inner.pools.len());
                Ok(())
            }
            Err(e) => {
                error!("Failed to load pools from repository: {}", e);
                Err(format!("Failed to load pools: {}", e))
            }
        }
    }

    /// Register a compute pool
    pub fn register_pool(&self, pool: ComputePool) -> Result<(), String> {
        let mut inner = self.inner.write().unwrap();
        Self::register_pool_inner(&mut inner, pool)
    }

    /// Inner implementation of register_pool
    fn register_pool_inner(inner: &mut GRCInner, pool: ComputePool) -> Result<(), String> {
        if inner.pools.contains_key(&pool.id) {
            return Err(format!("Pool {} already exists", pool.id));
        }

        inner.pools.insert(pool.id.clone(), pool);
        Self::update_metrics_inner(inner);

        Ok(())
    }

    /// Unregister a compute pool
    pub fn unregister_pool(&self, pool_id: &PoolId) -> Result<(), String> {
        let mut inner = self.inner.write().unwrap();
        if !inner.pools.contains_key(pool_id) {
            return Err(format!("Pool {} not found", pool_id));
        }

        inner.pools.remove(pool_id);
        Self::update_metrics_inner(&mut inner);

        Ok(())
    }

    /// Register a tenant quota
    pub fn register_tenant_quota(
        &self,
        tenant_id: String,
        quota: TenantQuota,
    ) -> Result<(), String> {
        let mut inner = self.inner.write().unwrap();
        inner.tenant_quotas.insert(tenant_id, quota);
        Ok(())
    }

    /// Find candidate pools that can satisfy a resource request
    pub fn find_candidate_pools(
        &self,
        request: &ResourceRequest,
    ) -> Result<Vec<ComputePool>, String> {
        let inner = self.inner.read().unwrap();
        let mut candidates: Vec<ComputePool> = inner
            .pools
            .values()
            .filter(|pool| {
                // Pool must be active
                if pool.status != PoolStatus::Active {
                    return false;
                }
                // Must match required labels
                if !pool.matches_labels(&request.required_labels) {
                    return false;
                }
                // Must have sufficient capacity
                pool.can_accommodate(
                    request.cpu_millicores,
                    request.memory_mb,
                    request.gpu_count.unwrap_or(0),
                )
            })
            .cloned()
            .collect();

        // Sort by score (best first)
        candidates.sort_by(|a, b| {
            let score_a = self.calculate_pool_score_inner(&inner, a, request);
            let score_b = self.calculate_pool_score_inner(&inner, b, request);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(candidates)
    }

    /// Select the best pool for a resource request
    pub async fn select_best_pool(
        &self,
        pools: &[ComputePool],
        request: &ResourceRequest,
    ) -> Result<ComputePool, String> {
        if pools.is_empty() {
            return Err("No pools available".to_string());
        }

        let inner = self.inner.read().unwrap();
        let mut best_pool = pools[0].clone();
        let mut best_score = self.calculate_pool_score_inner(&inner, &best_pool, request);

        for pool in pools {
            let score = self.calculate_pool_score_inner(&inner, pool, request);
            if score > best_score {
                best_pool = pool.clone();
                best_score = score;
            }
        }

        Ok(best_pool)
    }

    /// Allocate resources from a pool
    pub fn allocate_resources(
        &self,
        allocation_id: String,
        pool_id: PoolId,
        request: ResourceRequest,
    ) -> Result<AllocationResult, String> {
        let mut inner = self.inner.write().unwrap();

        // Get pool
        let pool = inner
            .pools
            .get_mut(&pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?;

        // Check pool status
        if pool.status != PoolStatus::Active {
            return Err(format!("Pool {} is not active", pool_id));
        }

        // Check capacity
        let requested_capacity = PoolCapacity {
            cpu_millicores: request.cpu_millicores,
            memory_mb: request.memory_mb,
            gpu_count: request.gpu_count.unwrap_or(0),
            max_workers: 1,
            active_workers: 1,
            storage_gb: None,
        };

        let available = pool
            .total_capacity
            .available(&pool.used_capacity, &pool.reserved_capacity);

        if !available.can_accommodate(&requested_capacity) {
            return Err(format!("Pool {} has insufficient capacity", pool_id));
        }

        // Update pool capacity
        pool.used_capacity = PoolCapacity {
            cpu_millicores: pool.used_capacity.cpu_millicores + requested_capacity.cpu_millicores,
            memory_mb: pool.used_capacity.memory_mb + requested_capacity.memory_mb,
            gpu_count: pool.used_capacity.gpu_count + requested_capacity.gpu_count,
            max_workers: pool.used_capacity.max_workers,
            active_workers: pool.used_capacity.active_workers + 1,
            storage_gb: pool.used_capacity.storage_gb,
        };

        // Calculate cost
        let estimated_cost = if self.config.enable_cost_tracking {
            pool.calculate_cost_per_hour(&requested_capacity)
        } else {
            None
        };

        Self::update_metrics_inner(&mut inner);

        Ok(AllocationResult {
            allocation_id,
            pool_id,
            resources: requested_capacity,
            estimated_cost_per_hour: estimated_cost,
            expires_at: None,
        })
    }

    /// Release resources back to the pool
    pub fn release_resources(&self, allocation: AllocationResult) -> Result<(), String> {
        let mut inner = self.inner.write().unwrap();
        let pool = inner
            .pools
            .get_mut(&allocation.pool_id)
            .ok_or_else(|| format!("Pool {} not found", allocation.pool_id))?;

        // Update pool capacity
        pool.used_capacity = PoolCapacity {
            cpu_millicores: pool
                .used_capacity
                .cpu_millicores
                .saturating_sub(allocation.resources.cpu_millicores),
            memory_mb: pool
                .used_capacity
                .memory_mb
                .saturating_sub(allocation.resources.memory_mb),
            gpu_count: pool
                .used_capacity
                .gpu_count
                .saturating_sub(allocation.resources.gpu_count),
            max_workers: pool.used_capacity.max_workers,
            active_workers: pool.used_capacity.active_workers.saturating_sub(1),
            storage_gb: pool.used_capacity.storage_gb,
        };

        Self::update_metrics_inner(&mut inner);

        Ok(())
    }

    /// Allocate resources with quota enforcement
    pub fn allocate_with_quota_check(
        &self,
        allocation_id: String,
        pool_id: PoolId,
        request: ResourceRequest,
    ) -> Result<AllocationResult, String> {
        let mut inner = self.inner.write().unwrap();

        // Check tenant quota if enabled
        if self.config.enable_quota_enforcement {
            if let Some(tenant_id) = &request.tenant_id {
                if let Some(quota) = inner.tenant_quotas.get(tenant_id) {
                    // Check pool access first
                    quota
                        .check_pool_access(&pool_id)
                        .map_err(|e| format!("Pool access denied: {}", e))?;

                    // Check concurrent job limit
                    let current_jobs = inner.active_tenant_jobs.get(tenant_id).unwrap_or(&0);
                    if *current_jobs >= quota.max_concurrent_jobs {
                        return Err(format!(
                            "Tenant {} has exceeded concurrent job limit of {} (currently {} active)",
                            tenant_id, quota.max_concurrent_jobs, current_jobs
                        ));
                    }

                    // Check cumulative resource usage (existing + new)
                    let requested_capacity = PoolCapacity {
                        cpu_millicores: request.cpu_millicores,
                        memory_mb: request.memory_mb,
                        gpu_count: request.gpu_count.unwrap_or(0),
                        max_workers: 1,
                        active_workers: 1,
                        storage_gb: None,
                    };

                    let current_usage = inner
                        .tenant_resource_usage
                        .get(tenant_id)
                        .unwrap_or(&PoolCapacity {
                            cpu_millicores: 0,
                            memory_mb: 0,
                            gpu_count: 0,
                            max_workers: 0,
                            active_workers: 0,
                            storage_gb: None,
                        })
                        .clone();

                    let total_usage = PoolCapacity {
                        cpu_millicores: current_usage.cpu_millicores
                            + requested_capacity.cpu_millicores,
                        memory_mb: current_usage.memory_mb + requested_capacity.memory_mb,
                        gpu_count: current_usage.gpu_count + requested_capacity.gpu_count,
                        max_workers: current_usage.max_workers,
                        active_workers: current_usage.active_workers + 1,
                        storage_gb: current_usage.storage_gb,
                    };

                    let required_cores = (total_usage.cpu_millicores + 999) / 1000;
                    if required_cores > quota.max_cpu_cores as u64 {
                        return Err(format!(
                            "CPU quota exceeded: {} cores required but limit is {} cores",
                            required_cores, quota.max_cpu_cores
                        ));
                    }

                    if total_usage.memory_mb > quota.max_memory_mb {
                        return Err(format!(
                            "Memory quota exceeded: {}MB required but limit is {}MB",
                            total_usage.memory_mb, quota.max_memory_mb
                        ));
                    }

                    if let Some(max_gpus) = quota.max_gpus {
                        if total_usage.gpu_count > max_gpus {
                            return Err(format!(
                                "GPU quota exceeded: {} GPUs required but limit is {}",
                                total_usage.gpu_count, max_gpus
                            ));
                        }
                    }

                    // Increment job count and update resource usage
                    *inner
                        .active_tenant_jobs
                        .entry(tenant_id.clone())
                        .or_insert(0) += 1;
                    inner.tenant_resource_usage
                        .insert(tenant_id.clone(), total_usage);
                }
            }
        }

        // Now allocate resources
        let pool = inner
            .pools
            .get_mut(&pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?;

        // Check pool status
        if pool.status != PoolStatus::Active {
            return Err(format!("Pool {} is not active", pool_id));
        }

        // Check capacity
        let requested_capacity = PoolCapacity {
            cpu_millicores: request.cpu_millicores,
            memory_mb: request.memory_mb,
            gpu_count: request.gpu_count.unwrap_or(0),
            max_workers: 1,
            active_workers: 1,
            storage_gb: None,
        };

        let available = pool
            .total_capacity
            .available(&pool.used_capacity, &pool.reserved_capacity);

        if !available.can_accommodate(&requested_capacity) {
            return Err(format!("Pool {} has insufficient capacity", pool_id));
        }

        // Update pool capacity
        pool.used_capacity = PoolCapacity {
            cpu_millicores: pool.used_capacity.cpu_millicores + requested_capacity.cpu_millicores,
            memory_mb: pool.used_capacity.memory_mb + requested_capacity.memory_mb,
            gpu_count: pool.used_capacity.gpu_count + requested_capacity.gpu_count,
            max_workers: pool.used_capacity.max_workers,
            active_workers: pool.used_capacity.active_workers + 1,
            storage_gb: pool.used_capacity.storage_gb,
        };

        // Calculate cost
        let estimated_cost = if self.config.enable_cost_tracking {
            pool.calculate_cost_per_hour(&requested_capacity)
        } else {
            None
        };

        Self::update_metrics_inner(&mut inner);

        Ok(AllocationResult {
            allocation_id,
            pool_id,
            resources: requested_capacity,
            estimated_cost_per_hour: estimated_cost,
            expires_at: None,
        })
    }

    /// Get pool status
    pub fn get_pool_status(&self, pool_id: &PoolId) -> Result<PoolCapacity, String> {
        let inner = self.inner.read().unwrap();
        let pool = inner
            .pools
            .get(pool_id)
            .ok_or_else(|| format!("Pool {} not found", pool_id))?;

        let available = pool.available_capacity();
        Ok(available)
    }

    /// Get all registered pools
    pub fn get_all_pools(&self) -> Vec<ComputePool> {
        let inner = self.inner.read().unwrap();
        inner.pools.values().cloned().collect()
    }

    /// Get GRC metrics
    pub fn get_metrics(&self) -> GRCMetrics {
        let inner = self.inner.read().unwrap();
        inner.metrics.clone()
    }

    /// Release tenant job slot
    pub fn release_tenant_job(&self, _allocation_id: &str, tenant_id: Option<&String>) {
        if let Some(tenant_id) = tenant_id {
            let mut inner = self.inner.write().unwrap();
            if let Some(count) = inner.active_tenant_jobs.get_mut(tenant_id) {
                if *count > 0 {
                    *count -= 1;
                    // Remove entry if count reaches 0 to keep HashMap clean
                    if *count == 0 {
                        inner.active_tenant_jobs.remove(tenant_id);
                    }
                }
            }

            // Also update resource usage tracking
            // Note: This is a simplified version. In production, you'd need to track
            // exactly which allocation_id maps to which resources for accurate decrement
        }
    }

    /// Calculate pool score based on multiple factors
    fn calculate_pool_score_inner(
        &self,
        inner: &GRCInner,
        pool: &ComputePool,
        request: &ResourceRequest,
    ) -> f64 {
        // Check if pool can accommodate the request
        if !pool.can_accommodate(
            request.cpu_millicores,
            request.memory_mb,
            request.gpu_count.unwrap_or(0),
        ) {
            return 0.0;
        }

        // 1. Bin Packing Score (35%) - Minimize resource waste
        let cpu_utilization = (pool.used_capacity.cpu_millicores + request.cpu_millicores) as f64
            / pool.total_capacity.cpu_millicores as f64;
        let bin_pack_score = if cpu_utilization >= 0.6 && cpu_utilization <= 0.85 {
            1.0 // Optimal utilization
        } else if cpu_utilization < 0.6 {
            0.7 + (cpu_utilization / 0.6) * 0.3 // Under-utilized penalty
        } else {
            0.3 // Over-utilized penalty
        };

        // 2. Cost Score (25%) - Minimize cost
        let cost_score = if let Some(cost_config) = &pool.cost_config {
            let cost_per_core_hour = cost_config.cpu_hour_cents as f64 / 100.0;
            1.0 / (1.0 + cost_per_core_hour / 10.0).max(0.1)
        } else {
            0.5 // Neutral score if no cost info
        };

        // 3. Label Match Score (20%) - Prefer pools with matching labels
        let label_score = if request.preferred_labels.is_empty() {
            1.0
        } else {
            let matching = request
                .preferred_labels
                .iter()
                .filter(|(k, v)| pool.has_label(k, v))
                .count();
            matching as f64 / request.preferred_labels.len() as f64
        };

        // 4. Load Balancing Score (15%) - Distribute load evenly
        let worker_utilization =
            pool.used_capacity.active_workers as f64 / pool.total_capacity.max_workers as f64;
        let load_score = 1.0 - worker_utilization;

        // 5. Health Score (5%) - Prefer healthy pools
        let health_score = if pool.status.is_healthy() {
            1.0
        } else {
            0.0 // Unhealthy pools get 0 health score
        };

        // Combined weighted score
        bin_pack_score * 0.35 + cost_score * 0.25 + label_score * 0.2 + load_score * 0.15 + health_score * 0.05
    }

    /// Update internal metrics
    fn update_metrics_inner(inner: &mut GRCInner) {
        inner.metrics.total_pools = inner.pools.len();
        inner.metrics.active_pools = inner
            .pools
            .values()
            .filter(|p| p.status == PoolStatus::Active)
            .count();

        // Aggregate capacity
        let mut total_capacity = PoolCapacity {
            cpu_millicores: 0,
            memory_mb: 0,
            gpu_count: 0,
            max_workers: 0,
            active_workers: 0,
            storage_gb: None,
        };

        let mut total_allocated = PoolCapacity {
            cpu_millicores: 0,
            memory_mb: 0,
            gpu_count: 0,
            max_workers: 0,
            active_workers: 0,
            storage_gb: None,
        };

        for pool in inner.pools.values() {
            total_capacity.cpu_millicores += pool.total_capacity.cpu_millicores;
            total_capacity.memory_mb += pool.total_capacity.memory_mb;
            total_capacity.gpu_count += pool.total_capacity.gpu_count;
            total_capacity.max_workers += pool.total_capacity.max_workers;
            total_capacity.active_workers += pool.total_capacity.active_workers;

            total_allocated.cpu_millicores += pool.used_capacity.cpu_millicores;
            total_allocated.memory_mb += pool.used_capacity.memory_mb;
            total_allocated.gpu_count += pool.used_capacity.gpu_count;
            total_allocated.max_workers += pool.used_capacity.max_workers;
            total_allocated.active_workers += pool.used_capacity.active_workers;
        }

        inner.metrics.total_capacity = total_capacity.clone();
        inner.metrics.total_allocated = total_allocated.clone();

        let reserved = PoolCapacity {
            cpu_millicores: 0,
            memory_mb: 0,
            gpu_count: 0,
            max_workers: 0,
            active_workers: 0,
            storage_gb: None,
        };

        inner.metrics.total_available = total_capacity.available(&total_allocated, &reserved);
        inner.metrics.active_allocations = total_allocated.active_workers as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_governance::*;

    struct MockResourcePoolRepository;

    #[async_trait::async_trait]
    impl ResourcePoolRepository for MockResourcePoolRepository {
        async fn save(&self, _config: ResourcePoolConfig) -> Result<(), String> {
            Ok(())
        }
        async fn get(&self, _name: &str) -> Result<Option<ResourcePoolConfig>, String> {
            Ok(None)
        }
        async fn list(&self) -> Result<Vec<ResourcePoolConfig>, String> {
            Ok(vec![])
        }
        async fn delete(&self, _name: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_pool_score_calculation() {
        let repo = Arc::new(MockResourcePoolRepository);
        let grc = GlobalResourceController::new(GRCConfig::default(), repo);

        let mut pool = ComputePool::builder()
            .id("test-pool".into())
            .name("Test Pool".to_string())
            .provider_type(ProviderType::Kubernetes)
            .total_capacity(PoolCapacity {
                cpu_millicores: 16000,
                memory_mb: 32768,
                gpu_count: 4,
                max_workers: 100,
                active_workers: 0,
                storage_gb: Some(1000),
            })
            .build()
            .unwrap();

        // Simulate partial utilization (50%)
        pool.used_capacity = PoolCapacity {
            cpu_millicores: 8000,
            memory_mb: 16384,
            gpu_count: 2,
            max_workers: 100,
            active_workers: 50,
            storage_gb: Some(1000),
        };

        let request = ResourceRequest::builder()
            .request_id("req-1".into())
            .cpu_millicores(2000)
            .memory_mb(4096)
            .priority(5)
            .build()
            .unwrap();

        // Need to register pool first
        let grc_clone = grc.clone();
        grc_clone.register_pool(pool).unwrap();

        let inner = grc.inner.read().unwrap();
        let pool_ref = inner.pools.get(&PoolId::from("test-pool")).unwrap();
        let score = grc.calculate_pool_score_inner(&inner, pool_ref, &request);

        // Score should be good for optimal utilization
        assert!(score > 0.5);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_allocation_and_release() {
        let repo = Arc::new(MockResourcePoolRepository);
        let grc = GlobalResourceController::new(GRCConfig::default(), repo);

        let pool = ComputePool::builder()
            .id("pool-1".into())
            .name("Pool 1".to_string())
            .provider_type(ProviderType::Kubernetes)
            .total_capacity(PoolCapacity {
                cpu_millicores: 16000,
                memory_mb: 32768,
                gpu_count: 4,
                max_workers: 100,
                active_workers: 0,
                storage_gb: Some(1000),
            })
            .build()
            .unwrap();

        grc.register_pool(pool).unwrap();

        let request = ResourceRequest::builder()
            .request_id("req-1".into())
            .cpu_millicores(4000)
            .memory_mb(8192)
            .priority(5)
            .build()
            .unwrap();

        // Allocate
        let allocation = grc
            .allocate_resources("alloc-1".to_string(), "pool-1".into(), request.clone())
            .unwrap();

        assert_eq!(allocation.resources.cpu_millicores, 4000);
        assert_eq!(allocation.resources.memory_mb, 8192);

        // Check pool capacity decreased
        let available = grc.get_pool_status(&PoolId::from("pool-1")).unwrap();
        assert_eq!(available.cpu_millicores, 12000); // 16000 - 4000

        // Release
        grc.release_resources(allocation).unwrap();

        // Check pool capacity restored
        let available = grc.get_pool_status(&PoolId::from("pool-1")).unwrap();
        assert_eq!(available.cpu_millicores, 16000);
    }
}