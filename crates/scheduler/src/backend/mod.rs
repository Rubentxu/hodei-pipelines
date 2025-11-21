//! Scheduler Backend Abstraction Module
//!
//! This module provides the backend abstraction layer for supporting multiple
//! execution backends (Kubernetes, Docker, Cloud VMs, etc.). Each backend
//! implements the SchedulerBackend trait to provide uniform interface.

use crate::types::*;
use crate::{SchedulerError, WorkerId, JobId};
use std::sync::Arc;
use async_trait::async_trait;

/// Scheduler backend trait for multi-backend support
#[async_trait]
pub trait SchedulerBackend: Send + Sync {
    /// Get backend type
    fn backend_type(&self) -> BackendType;

    /// List all available nodes
    async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError>;

    /// Get specific node by ID
    async fn get_node(&self, id: &WorkerId) -> Result<WorkerNode, SchedulerError>;

    /// Bind job to node
    async fn bind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError>;

    /// Unbind job from node
    async fn unbind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError>;

    /// Get node status
    async fn get_node_status(&self, id: &WorkerId) -> Result<WorkerStatus, SchedulerError>;
}

/// Compute resources of a worker node
#[derive(Debug, Clone, PartialEq)]
pub struct ComputeResource {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub gpu_count: u32,
}

impl ComputeResource {
    /// Check if node has enough resources
    pub fn has_resources(&self, requirements: &ResourceRequirements) -> bool {
        if let Some(cpu) = requirements.cpu_cores {
            if self.cpu_cores < cpu {
                return false;
            }
        }

        if let Some(memory) = requirements.memory_bytes {
            if self.memory_bytes < memory {
                return false;
            }
        }

        if let Some(gpu) = requirements.gpu_count {
            if self.gpu_count < gpu {
                return false;
            }
        }

        true
    }

    /// Calculate utilization percentage (0.0 to 1.0)
    pub fn utilization(&self, requirements: &ResourceRequirements) -> f64 {
        let mut max_util: f64 = 0.0;

        if let Some(cpu) = requirements.cpu_cores {
            max_util = max_util.max(cpu / self.cpu_cores);
        }

        if let Some(memory) = requirements.memory_bytes {
            let mem_util = memory as f64 / self.memory_bytes as f64;
            max_util = max_util.max(mem_util);
        }

        if let Some(gpu) = requirements.gpu_count {
            if self.gpu_count > 0 {
                let gpu_util = gpu as f64 / self.gpu_count as f64;
                max_util = max_util.max(gpu_util);
            }
        }

        max_util.min(1.0)
    }
}

/// Kubernetes backend adapter
pub struct KubernetesBackend {
    // In production: would have Kubernetes client/API connector
}

impl KubernetesBackend {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SchedulerBackend for KubernetesBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Kubernetes
    }

    async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError> {
        // TODO: Connect to Kubernetes API
        // For now: return empty list
        Ok(vec![])
    }

    async fn get_node(&self, id: &WorkerId) -> Result<WorkerNode, SchedulerError> {
        // TODO: Implement Kubernetes API call
        Err(SchedulerError::WorkerNotFound(*id))
    }

    async fn bind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError> {
        // TODO: Implement Kubernetes Pod binding
        tracing::info!("Binding job {} to Kubernetes node {}", job_id, node_id);
        Ok(())
    }

    async fn unbind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError> {
        tracing::info!("Unbinding job {} from Kubernetes node {}", job_id, node_id);
        Ok(())
    }

    async fn get_node_status(&self, id: &WorkerId) -> Result<WorkerStatus, SchedulerError> {
        // TODO: Get status from Kubernetes API
        Err(SchedulerError::WorkerNotFound(*id))
    }
}

/// Docker backend adapter
pub struct DockerBackend {
    // In production: would have Docker client connector
}

impl DockerBackend {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SchedulerBackend for DockerBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Docker
    }

    async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError> {
        // TODO: Connect to Docker API
        Ok(vec![])
    }

    async fn get_node(&self, id: &WorkerId) -> Result<WorkerNode, SchedulerError> {
        Err(SchedulerError::WorkerNotFound(*id))
    }

    async fn bind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError> {
        tracing::info!("Binding job {} to Docker node {}", job_id, node_id);
        Ok(())
    }

    async fn unbind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError> {
        tracing::info!("Unbinding job {} from Docker node {}", job_id, node_id);
        Ok(())
    }

    async fn get_node_status(&self, id: &WorkerId) -> Result<WorkerStatus, SchedulerError> {
        Err(SchedulerError::WorkerNotFound(*id))
    }
}

/// Cloud VM backend adapter
pub struct CloudVmBackend {
    // In production: would have cloud provider client (AWS, Azure, GCP)
}

impl CloudVmBackend {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SchedulerBackend for CloudVmBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::CloudVm
    }

    async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError> {
        // TODO: Connect to cloud provider API
        Ok(vec![])
    }

    async fn get_node(&self, id: &WorkerId) -> Result<WorkerNode, SchedulerError> {
        Err(SchedulerError::WorkerNotFound(*id))
    }

    async fn bind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError> {
        tracing::info!("Binding job {} to Cloud VM {}", job_id, node_id);
        Ok(())
    }

    async fn unbind_job(&self, job_id: &JobId, node_id: &WorkerId) -> Result<(), SchedulerError> {
        tracing::info!("Unbinding job {} from Cloud VM {}", job_id, node_id);
        Ok(())
    }

    async fn get_node_status(&self, id: &WorkerId) -> Result<WorkerStatus, SchedulerError> {
        Err(SchedulerError::WorkerNotFound(*id))
    }
}

/// Backend registry for managing multiple backends
pub struct BackendRegistry {
    backends: dashmap::DashMap<BackendType, Arc<dyn SchedulerBackend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            backends: dashmap::DashMap::new(),
        }
    }

    /// Register backend
    pub fn register(&self, backend: Arc<dyn SchedulerBackend>) {
        let backend_type = backend.backend_type();
        self.backends.insert(backend_type, backend);
    }

    /// Get backend by type
    pub fn get(&self, backend_type: BackendType) -> Option<Arc<dyn SchedulerBackend>> {
        self.backends.get(&backend_type).map(|entry| entry.clone())
    }

    /// List all registered backend types
    pub fn list_types(&self) -> Vec<BackendType> {
        self.backends.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_compute_resource_has_resources() {
        let resource = ComputeResource {
            cpu_cores: 8.0,
            memory_bytes: 16_000_000_000,
            gpu_count: 2,
        };

        let requirements = ResourceRequirements {
            cpu_cores: Some(4.0),
            memory_bytes: Some(8_000_000_000),
            gpu_count: Some(1),
            ephemeral_storage: None,
        };

        assert!(resource.has_resources(&requirements));

        // Insufficient resources
        let insufficient_requirements = ResourceRequirements {
            cpu_cores: Some(16.0),
            memory_bytes: Some(8_000_000_000),
            gpu_count: Some(1),
            ephemeral_storage: None,
        };

        assert!(!resource.has_resources(&insufficient_requirements));
    }

    #[test]
    fn test_compute_resource_utilization() {
        let resource = ComputeResource {
            cpu_cores: 8.0,
            memory_bytes: 16_000_000_000,
            gpu_count: 2,
        };

        let requirements = ResourceRequirements {
            cpu_cores: Some(4.0),
            memory_bytes: Some(8_000_000_000),
            gpu_count: Some(1),
            ephemeral_storage: None,
        };

        let utilization = resource.utilization(&requirements);
        assert!(utilization > 0.0 && utilization <= 1.0);
    }
}
