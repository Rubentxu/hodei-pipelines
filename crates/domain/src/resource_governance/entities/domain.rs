//! Resource Governance Types
//!
//! This module defines the core types for resource governance,
//! including ComputePools, ResourceRequests, TenantQuotas, and related
//! types for managing compute resources across multiple providers.

use crate::ResourceQuota;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Resource pool configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourcePoolConfig {
    pub provider_type: ProviderType,
    pub name: String,
    pub provider_name: String,
    pub min_size: u32,
    pub max_size: u32,
    pub default_resources: ResourceQuota,
    pub tags: HashMap<String, String>,
}

impl Default for ResourcePoolConfig {
    fn default() -> Self {
        Self {
            provider_type: ProviderType::Docker,
            name: "default-pool".to_string(),
            provider_name: "default-provider".to_string(),
            min_size: 0,
            max_size: 10,
            default_resources: ResourceQuota::new(1000, 1024),
            tags: HashMap::new(),
        }
    }
}

/// Resource pool status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourcePoolStatus {
    pub name: String,
    pub provider_type: ProviderType,
    pub total_capacity: u32,
    pub available_capacity: u32,
    pub active_workers: u32,
    pub pending_requests: u32,
}

/// Unique identifier for a ComputePool
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoolId(pub String);

impl PoolId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for PoolId {
    fn from(s: String) -> Self {
        PoolId(s)
    }
}

impl From<&str> for PoolId {
    fn from(s: &str) -> Self {
        PoolId(s.to_string())
    }
}

impl fmt::Display for PoolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a ResourceRequest
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId(s.to_string())
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Provider type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    Kubernetes,
    Docker,
    CloudVM,
    BareMetal,
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderType::Kubernetes => write!(f, "Kubernetes"),
            ProviderType::Docker => write!(f, "Docker"),
            ProviderType::CloudVM => write!(f, "CloudVM"),
            ProviderType::BareMetal => write!(f, "BareMetal"),
        }
    }
}

/// Pool capacity information
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoolCapacity {
    pub cpu_millicores: u64, // 1000m = 1 core
    pub memory_mb: u64,
    pub gpu_count: u32,
    pub max_workers: u32,
    pub active_workers: u32,
    pub storage_gb: Option<u64>,
}

impl PoolCapacity {
    /// Check if this capacity can accommodate another capacity
    pub fn can_accommodate(&self, other: &PoolCapacity) -> bool {
        self.cpu_millicores >= other.cpu_millicores
            && self.memory_mb >= other.memory_mb
            && self.gpu_count >= other.gpu_count
            && self.active_workers + other.active_workers <= self.max_workers
    }

    /// Calculate available capacity after subtracting used capacity
    pub fn available(&self, used: &PoolCapacity, reserved: &PoolCapacity) -> PoolCapacity {
        PoolCapacity {
            cpu_millicores: self
                .cpu_millicores
                .saturating_sub(used.cpu_millicores)
                .saturating_sub(reserved.cpu_millicores),
            memory_mb: self
                .memory_mb
                .saturating_sub(used.memory_mb)
                .saturating_sub(reserved.memory_mb),
            gpu_count: self
                .gpu_count
                .saturating_sub(used.gpu_count)
                .saturating_sub(reserved.gpu_count),
            max_workers: self.max_workers,
            active_workers: self
                .active_workers
                .saturating_sub(used.active_workers)
                .saturating_sub(reserved.active_workers),
            storage_gb: match (self.storage_gb, used.storage_gb, reserved.storage_gb) {
                (Some(total), Some(used_gb), Some(reserved_gb)) => {
                    Some(total.saturating_sub(used_gb).saturating_sub(reserved_gb))
                }
                _ => None,
            },
        }
    }

    /// Calculate utilization percentage based on CPU
    pub fn utilization_percentage(&self) -> f64 {
        if self.cpu_millicores == 0 {
            return 0.0;
        }
        (self.active_workers as f64 / self.max_workers as f64) * 100.0
    }
}

/// Cost configuration for a compute pool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostConfig {
    pub cpu_hour_cents: u64,        // Cost per CPU core per hour (in cents)
    pub memory_gb_hour_cents: u64,  // Cost per GB memory per hour (in cents)
    pub gpu_hour_cents: u64,        // Cost per GPU per hour (in cents)
    pub storage_gb_hour_cents: u64, // Cost per GB storage per hour (in cents)
    pub base_hourly_cents: u64,     // Base hourly cost (in cents)
}

/// Pool status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolStatus {
    Active,   // Accepting new allocations
    Paused,   // Temporarily paused
    Draining, // Draining existing allocations
    Offline,  // Completely offline
}

impl PoolStatus {
    /// Check if pool is accepting new allocations
    pub fn is_accepting_allocations(&self) -> bool {
        matches!(self, PoolStatus::Active)
    }

    /// Check if pool is healthy and operational
    pub fn is_healthy(&self) -> bool {
        matches!(self, PoolStatus::Active)
    }
}

/// Compute Pool representing a cluster or resource pool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputePool {
    pub id: PoolId,
    pub name: String,
    pub provider_type: ProviderType,

    // Capacity tracking
    pub total_capacity: PoolCapacity,
    pub used_capacity: PoolCapacity,
    pub reserved_capacity: PoolCapacity,

    // Metadata
    pub labels: HashMap<String, String>,

    // Cost information
    pub cost_config: Option<CostConfig>,

    // State
    pub status: PoolStatus,
}

/// Builder for ComputePool using Builder Pattern
pub struct ComputePoolBuilder {
    id: Option<PoolId>,
    name: Option<String>,
    provider_type: Option<ProviderType>,
    total_capacity: Option<PoolCapacity>,
    labels: Option<HashMap<String, String>>,
    cost_config: Option<CostConfig>,
    status: Option<PoolStatus>,
}

impl ComputePoolBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            name: None,
            provider_type: None,
            total_capacity: None,
            labels: None,
            cost_config: None,
            status: None,
        }
    }

    pub fn id(mut self, id: PoolId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn provider_type(mut self, provider_type: ProviderType) -> Self {
        self.provider_type = Some(provider_type);
        self
    }

    pub fn total_capacity(mut self, capacity: PoolCapacity) -> Self {
        self.total_capacity = Some(capacity);
        self
    }

    pub fn labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn cost_config(mut self, cost_config: CostConfig) -> Self {
        self.cost_config = Some(cost_config);
        self
    }

    pub fn status(mut self, status: PoolStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn build(self) -> Result<ComputePool, String> {
        let id = self.id.ok_or_else(|| "id is required".to_string())?;
        let name = self.name.ok_or_else(|| "name is required".to_string())?;
        let provider_type = self
            .provider_type
            .ok_or_else(|| "provider_type is required".to_string())?;
        let total_capacity = self
            .total_capacity
            .ok_or_else(|| "total_capacity is required".to_string())?;

        let mut labels = self.labels.unwrap_or_default();
        // Add provider_type as implicit label
        labels.insert("provider_type".to_string(), provider_type.to_string());

        let status = self.status.unwrap_or(PoolStatus::Active);

        Ok(ComputePool {
            id,
            name,
            provider_type,
            total_capacity: total_capacity.clone(),
            used_capacity: PoolCapacity {
                cpu_millicores: 0,
                memory_mb: 0,
                gpu_count: 0,
                max_workers: total_capacity.max_workers,
                active_workers: 0,
                storage_gb: total_capacity.storage_gb,
            },
            reserved_capacity: PoolCapacity {
                cpu_millicores: 0,
                memory_mb: 0,
                gpu_count: 0,
                max_workers: 0,
                active_workers: 0,
                storage_gb: None,
            },
            labels,
            cost_config: self.cost_config,
            status,
        })
    }
}

impl ComputePool {
    /// Create a new builder for ComputePool
    pub fn builder() -> ComputePoolBuilder {
        ComputePoolBuilder::new()
    }

    /// Get available capacity (total - used - reserved)
    pub fn available_capacity(&self) -> PoolCapacity {
        self.total_capacity
            .available(&self.used_capacity, &self.reserved_capacity)
    }

    /// Calculate utilization percentage
    pub fn utilization_percentage(&self) -> f64 {
        if self.total_capacity.max_workers == 0 {
            return 0.0;
        }
        (self.used_capacity.active_workers as f64 / self.total_capacity.max_workers as f64) * 100.0
    }

    /// Check if pool can accommodate a resource request
    pub fn can_accommodate(&self, cpu_millicores: u64, memory_mb: u64, gpu_count: u32) -> bool {
        let available = self.available_capacity();
        available.cpu_millicores >= cpu_millicores
            && available.memory_mb >= memory_mb
            && available.gpu_count >= gpu_count
    }

    /// Calculate cost per hour for given capacity
    pub fn calculate_cost_per_hour(&self, capacity: &PoolCapacity) -> Option<f64> {
        let cost_config = self.cost_config.as_ref()?;

        let cpu_cost =
            (capacity.cpu_millicores as f64 / 1000.0) * (cost_config.cpu_hour_cents as f64 / 100.0);
        let memory_cost = (capacity.memory_mb as f64 / 1024.0)
            * (cost_config.memory_gb_hour_cents as f64 / 100.0);
        let gpu_cost = capacity.gpu_count as f64 * (cost_config.gpu_hour_cents as f64 / 100.0);
        let storage_cost = capacity.storage_gb.unwrap_or(0) as f64
            * (cost_config.storage_gb_hour_cents as f64 / 100.0);
        let base_cost = cost_config.base_hourly_cents as f64 / 100.0;

        Some(cpu_cost + memory_cost + gpu_cost + storage_cost + base_cost)
    }

    /// Check if pool has a specific label
    pub fn has_label(&self, key: &str, value: &str) -> bool {
        self.labels.get(key).map(|v| v == value).unwrap_or(false)
    }

    /// Check if pool matches required labels
    pub fn matches_labels(&self, required_labels: &HashMap<String, String>) -> bool {
        for (key, value) in required_labels {
            if self.labels.get(key) != Some(value) {
                return false;
            }
        }
        true
    }

    /// Update pool status
    pub fn set_status(&mut self, status: PoolStatus) {
        self.status = status;
    }

    /// Pause the pool
    pub fn pause(&mut self) {
        self.status = PoolStatus::Paused;
    }

    /// Resume the pool
    pub fn resume(&mut self) {
        self.status = PoolStatus::Active;
    }

    /// Drain the pool
    pub fn drain(&mut self) {
        self.status = PoolStatus::Draining;
    }

    /// Mark pool as offline
    pub fn mark_offline(&mut self) {
        self.status = PoolStatus::Offline;
    }

    /// Update capacity (e.g., from autoscaling)
    pub fn update_capacity(&mut self, new_capacity: PoolCapacity) {
        self.total_capacity = new_capacity;
    }
}

/// Resource request for scheduling
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub request_id: RequestId,
    pub pipeline_id: Option<String>,
    pub job_id: Option<String>,

    // Resources required
    pub cpu_millicores: u64,
    pub memory_mb: u64,
    pub gpu_count: Option<u32>,

    // Labels for pool selection
    pub required_labels: HashMap<String, String>,
    pub preferred_labels: HashMap<String, String>,

    // Multi-tenancy
    pub tenant_id: Option<String>,
    pub priority: u8,
}

/// Builder for ResourceRequest
pub struct ResourceRequestBuilder {
    request_id: Option<RequestId>,
    pipeline_id: Option<String>,
    job_id: Option<String>,
    cpu_millicores: Option<u64>,
    memory_mb: Option<u64>,
    gpu_count: Option<u32>,
    required_labels: Option<HashMap<String, String>>,
    preferred_labels: Option<HashMap<String, String>>,
    tenant_id: Option<String>,
    priority: Option<u8>,
}

impl ResourceRequestBuilder {
    pub fn new() -> Self {
        Self {
            request_id: None,
            pipeline_id: None,
            job_id: None,
            cpu_millicores: None,
            memory_mb: None,
            gpu_count: None,
            required_labels: None,
            preferred_labels: None,
            tenant_id: None,
            priority: None,
        }
    }

    pub fn request_id(mut self, id: RequestId) -> Self {
        self.request_id = Some(id);
        self
    }

    pub fn pipeline_id(mut self, id: String) -> Self {
        self.pipeline_id = Some(id);
        self
    }

    pub fn job_id(mut self, id: String) -> Self {
        self.job_id = Some(id);
        self
    }

    pub fn cpu_millicores(mut self, cpu: u64) -> Self {
        self.cpu_millicores = Some(cpu);
        self
    }

    pub fn memory_mb(mut self, memory: u64) -> Self {
        self.memory_mb = Some(memory);
        self
    }

    pub fn gpu_count(mut self, gpu: Option<u32>) -> Self {
        self.gpu_count = gpu;
        self
    }

    pub fn required_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.required_labels = Some(labels);
        self
    }

    pub fn preferred_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.preferred_labels = Some(labels);
        self
    }

    pub fn tenant_id(mut self, id: String) -> Self {
        self.tenant_id = Some(id);
        self
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn build(self) -> Result<ResourceRequest, String> {
        let request_id = self
            .request_id
            .ok_or_else(|| "request_id is required".to_string())?;
        let cpu_millicores = self
            .cpu_millicores
            .ok_or_else(|| "cpu_millicores is required".to_string())?;
        let memory_mb = self
            .memory_mb
            .ok_or_else(|| "memory_mb is required".to_string())?;

        Ok(ResourceRequest {
            request_id,
            pipeline_id: self.pipeline_id,
            job_id: self.job_id,
            cpu_millicores,
            memory_mb,
            gpu_count: self.gpu_count,
            required_labels: self.required_labels.unwrap_or_default(),
            preferred_labels: self.preferred_labels.unwrap_or_default(),
            tenant_id: self.tenant_id,
            priority: self.priority.unwrap_or(5),
        })
    }
}

impl ResourceRequest {
    /// Create a new builder for ResourceRequest
    pub fn builder() -> ResourceRequestBuilder {
        ResourceRequestBuilder::new()
    }

    /// Check if this request can be satisfied by available capacity
    pub fn can_be_satisfied_by(&self, capacity: &PoolCapacity) -> bool {
        capacity.cpu_millicores >= self.cpu_millicores
            && capacity.memory_mb >= self.memory_mb
            && self.gpu_count.map_or(true, |gpu| capacity.gpu_count >= gpu)
    }
}

/// Tenant quota for multi-tenancy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TenantQuota {
    pub tenant_id: String,
    pub max_cpu_cores: u32,
    pub max_memory_mb: u64,
    pub max_gpus: Option<u32>,
    pub max_concurrent_jobs: u32,
    pub max_daily_cost: Option<f64>,
    pub max_monthly_cost: Option<f64>,
    pub allowed_pools: Vec<PoolId>,
}

/// Builder for TenantQuota
pub struct TenantQuotaBuilder {
    tenant_id: Option<String>,
    max_cpu_cores: Option<u32>,
    max_memory_mb: Option<u64>,
    max_gpus: Option<u32>,
    max_concurrent_jobs: Option<u32>,
    max_daily_cost: Option<f64>,
    max_monthly_cost: Option<f64>,
    allowed_pools: Option<Vec<PoolId>>,
}

impl TenantQuotaBuilder {
    pub fn new() -> Self {
        Self {
            tenant_id: None,
            max_cpu_cores: None,
            max_memory_mb: None,
            max_gpus: None,
            max_concurrent_jobs: None,
            max_daily_cost: None,
            max_monthly_cost: None,
            allowed_pools: None,
        }
    }

    pub fn tenant_id(mut self, id: String) -> Self {
        self.tenant_id = Some(id);
        self
    }

    pub fn max_cpu_cores(mut self, cores: u32) -> Self {
        self.max_cpu_cores = Some(cores);
        self
    }

    pub fn max_memory_mb(mut self, memory: u64) -> Self {
        self.max_memory_mb = Some(memory);
        self
    }

    pub fn max_gpus(mut self, gpus: Option<u32>) -> Self {
        self.max_gpus = gpus;
        self
    }

    pub fn max_concurrent_jobs(mut self, jobs: u32) -> Self {
        self.max_concurrent_jobs = Some(jobs);
        self
    }

    pub fn max_daily_cost(mut self, cost: Option<f64>) -> Self {
        self.max_daily_cost = cost;
        self
    }

    pub fn max_monthly_cost(mut self, cost: Option<f64>) -> Self {
        self.max_monthly_cost = cost;
        self
    }

    pub fn allowed_pools(mut self, pools: Vec<PoolId>) -> Self {
        self.allowed_pools = Some(pools);
        self
    }

    pub fn build(self) -> Result<TenantQuota, String> {
        let tenant_id = self
            .tenant_id
            .ok_or_else(|| "tenant_id is required".to_string())?;
        let max_cpu_cores = self
            .max_cpu_cores
            .ok_or_else(|| "max_cpu_cores is required".to_string())?;
        let max_memory_mb = self
            .max_memory_mb
            .ok_or_else(|| "max_memory_mb is required".to_string())?;
        let max_concurrent_jobs = self
            .max_concurrent_jobs
            .ok_or_else(|| "max_concurrent_jobs is required".to_string())?;

        Ok(TenantQuota {
            tenant_id,
            max_cpu_cores,
            max_memory_mb,
            max_gpus: self.max_gpus,
            max_concurrent_jobs,
            max_daily_cost: self.max_daily_cost,
            max_monthly_cost: self.max_monthly_cost,
            allowed_pools: self.allowed_pools.unwrap_or_default(),
        })
    }
}

impl TenantQuota {
    /// Create a new builder for TenantQuota
    pub fn builder() -> TenantQuotaBuilder {
        TenantQuotaBuilder::new()
    }

    /// Check if a resource request is within quota limits
    pub fn check_within_limits(&self, request: &ResourceRequest) -> Result<(), String> {
        let required_cores = ((request.cpu_millicores + 999) / 1000) as u32; // Convert to cores

        if required_cores > self.max_cpu_cores {
            return Err(format!(
                "CPU request {} exceeds quota {} cores",
                required_cores, self.max_cpu_cores
            ));
        }

        if request.memory_mb > self.max_memory_mb {
            return Err(format!(
                "Memory request {}MB exceeds quota {}MB",
                request.memory_mb, self.max_memory_mb
            ));
        }

        if let Some(max_gpus) = self.max_gpus {
            if let Some(request_gpus) = request.gpu_count {
                if request_gpus > max_gpus {
                    return Err(format!(
                        "GPU request {} exceeds quota {} GPUs",
                        request_gpus, max_gpus
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if tenant has access to a specific pool
    pub fn check_pool_access(&self, pool_id: &PoolId) -> Result<(), String> {
        if !self.allowed_pools.contains(pool_id) {
            return Err(format!(
                "Pool {} not allowed for tenant {}",
                pool_id, self.tenant_id
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_capacity_can_accommodate() {
        let total = PoolCapacity {
            cpu_millicores: 8000,
            memory_mb: 16384,
            gpu_count: 4,
            max_workers: 50,
            active_workers: 0,
            storage_gb: Some(500),
        };

        let request = PoolCapacity {
            cpu_millicores: 4000,
            memory_mb: 8192,
            gpu_count: 2,
            max_workers: 25,
            active_workers: 0,
            storage_gb: Some(250),
        };

        assert!(total.can_accommodate(&request));

        let big_request = PoolCapacity {
            cpu_millicores: 16000,
            memory_mb: 32768,
            gpu_count: 8,
            max_workers: 100,
            active_workers: 0,
            storage_gb: Some(1000),
        };

        assert!(!total.can_accommodate(&big_request));
    }
}
