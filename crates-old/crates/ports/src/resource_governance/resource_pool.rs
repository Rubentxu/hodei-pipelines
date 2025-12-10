//! Resource Pool Port
//!
//! This module defines the port for managing resource pools that provide
//! workers on demand, allowing flexible resource allocation and auto-scaling.

use async_trait::async_trait;
pub use hodei_pipelines_domain::resource_governance::{
    ProviderType as ResourcePoolType, ResourcePoolConfig, ResourcePoolRepository,
    ResourcePoolStatus,
};
use hodei_pipelines_domain::{ResourceQuota, Worker, WorkerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ResourcePoolType is replaced by ProviderType in core

/// Request to allocate resources from pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocationRequest {
    pub request_id: String,
    pub required_resources: ResourceQuota,
    pub labels: HashMap<String, String>,
    pub priority: u8, // 0-255, higher = more priority
}

/// Resource allocation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolResourceAllocation {
    pub request_id: String,
    pub worker_id: WorkerId,
    pub allocation_id: String,
    pub status: AllocationStatus,
}

/// Allocation status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum AllocationStatus {
    Pending,
    Allocated {
        worker: Worker,
        container_id: Option<String>,
    },
    Failed(String),
}

/// Resource pool port
#[async_trait]
pub trait ResourcePool: Send + Sync {
    /// Get pool configuration
    fn config(&self) -> &ResourcePoolConfig;

    /// Get pool status
    async fn status(&self) -> Result<ResourcePoolStatus, String>;

    /// Request resource allocation
    async fn allocate_resources(
        &mut self,
        request: ResourceAllocationRequest,
    ) -> Result<PoolResourceAllocation, String>;

    /// Release allocated resources
    async fn release_resources(&mut self, allocation_id: &str) -> Result<(), String>;

    /// List active allocations
    async fn list_allocations(&self) -> Result<Vec<PoolResourceAllocation>, String>;

    /// Scale pool to target size
    async fn scale_to(&mut self, target_size: u32) -> Result<(), String>;

    /// Get available workers in pool
    async fn list_workers(&self) -> Result<Vec<WorkerId>, String>;
}
