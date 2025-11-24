//! Resource Pool Port
//!
//! This module defines the port for managing resource pools that provide
//! workers on demand, allowing flexible resource allocation and auto-scaling.

use async_trait::async_trait;
use hodei_core::{Worker, WorkerId};
use hodei_shared_types::{ResourceQuota, WorkerStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource pool type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourcePoolType {
    /// Docker-based pool using containers
    Docker,
    /// Kubernetes-based pool using Pods
    Kubernetes,
    /// Cloud VMs pool
    Cloud,
    /// Static pool with pre-provisioned workers
    Static,
}

/// Resource pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePoolConfig {
    pub pool_type: ResourcePoolType,
    pub name: String,
    pub provider_name: String,
    pub min_size: u32,
    pub max_size: u32,
    pub default_resources: ResourceQuota,
    pub tags: HashMap<String, String>,
}

/// Resource pool status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePoolStatus {
    pub name: String,
    pub pool_type: ResourcePoolType,
    pub total_capacity: u32,
    pub available_capacity: u32,
    pub active_workers: u32,
    pub pending_requests: u32,
}

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
pub struct ResourceAllocation {
    pub request_id: String,
    pub worker_id: WorkerId,
    pub allocation_id: String,
    pub status: AllocationStatus,
}

/// Allocation status
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        &self,
        request: ResourceAllocationRequest,
    ) -> Result<ResourceAllocation, String>;

    /// Release allocated resources
    async fn release_resources(
        &self,
        allocation_id: &str,
    ) -> Result<(), String>;

    /// List active allocations
    async fn list_allocations(&self) -> Result<Vec<ResourceAllocation>, String>;

    /// Scale pool to target size
    async fn scale_to(&self, target_size: u32) -> Result<(), String>;

    /// Get available workers in pool
    async fn list_workers(&self) -> Result<Vec<WorkerId>, String>;
}
