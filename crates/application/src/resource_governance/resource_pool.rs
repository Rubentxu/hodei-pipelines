//! Resource Pool Module
//!
//! This module provides the application layer for managing resource pools
//! that automatically provision and scale workers based on demand.

use async_trait::async_trait;
use hodei_pipelines_adapters::DefaultProviderFactory;
use hodei_pipelines_domain::{DomainError, ResourceQuota, Result, WorkerId};
use hodei_pipelines_ports::{
    ProviderFactoryTrait,
    resource_governance::resource_pool::{
        AllocationStatus, PoolResourceAllocation, ResourceAllocationRequest, ResourcePool,
        ResourcePoolConfig, ResourcePoolStatus, ResourcePoolType,
    },
    worker_provider::{ProviderConfig, ProviderError, WorkerProvider},
    worker_template::WorkerTemplate,
};
use std::collections::{HashMap, VecDeque};
use tracing::{error, info, warn};

/// Resource Pool Service
///
/// Manages a pool of resources that can be allocated on demand.
/// Handles auto-scaling, queuing, and worker lifecycle management.
#[derive(Debug)]
pub struct ResourcePoolService {
    config: ResourcePoolConfig,
    provider: Box<dyn WorkerProvider + Send + Sync>,
    allocations: HashMap<String, PoolResourceAllocation>,
    pending_queue: VecDeque<ResourceAllocationRequest>,
    active_workers: HashMap<WorkerId, String>, // worker_id -> allocation_id
    _auto_scaling_enabled: bool,
}

impl ResourcePoolService {
    pub fn new(
        config: ResourcePoolConfig,
        provider: Box<dyn WorkerProvider + Send + Sync>,
    ) -> Self {
        Self {
            config,
            provider,
            allocations: HashMap::new(),
            pending_queue: VecDeque::new(),
            active_workers: HashMap::new(),
            _auto_scaling_enabled: true,
        }
    }

    /// Process pending queue and allocate if capacity available
    async fn process_queue(&mut self) -> Result<()> {
        if self.pending_queue.is_empty() {
            return Ok(());
        }

        // Check available capacity
        let available = self.get_available_capacity().await?;
        if available == 0 {
            return Ok(());
        }

        // Process pending requests
        let mut processed = 0;
        while let Some(request) = self.pending_queue.pop_front() {
            if processed >= available {
                // Re-queue if no more capacity
                self.pending_queue.push_front(request);
                break;
            }

            let request_id = request.request_id.clone();
            match self.allocate_internal(&request).await {
                Ok(allocation) => {
                    self.allocations
                        .insert(allocation.allocation_id.clone(), allocation);
                    processed += 1;
                    info!("Allocated resources for request {}", request_id);
                }
                Err(e) => {
                    error!("Failed to allocate resources: {}", e);
                    // Mark as failed
                    let failed_allocation = PoolResourceAllocation {
                        request_id,
                        worker_id: WorkerId::new(),
                        allocation_id: format!("failed-{}", uuid::Uuid::new_v4()),
                        status: AllocationStatus::Failed(e.to_string()),
                    };
                    self.allocations
                        .insert(failed_allocation.allocation_id.clone(), failed_allocation);
                }
            }
        }

        Ok(())
    }

    /// Internal allocation logic
    async fn allocate_internal(
        &self,
        request: &ResourceAllocationRequest,
    ) -> Result<PoolResourceAllocation> {
        // Create worker with requested resources
        let worker_id = WorkerId::new();

        // Create a basic worker template
        let template = hodei_pipelines_ports::worker_template::WorkerTemplate::new(
            "pool-worker",
            "1.0.0",
            "hwp-agent:latest",
        )
        .with_cpu("2")
        .with_memory("4Gi");

        let mut config = ProviderConfig::docker(format!("pool-worker-{}", worker_id), template);

        // Set resources in environment or metadata
        config = config.with_image("hwp-agent:latest".to_string());

        match self.provider.create_worker(worker_id.clone(), config).await {
            Ok(worker) => Ok(PoolResourceAllocation {
                request_id: request.request_id.clone(),
                worker_id: worker_id.clone(),
                allocation_id: format!("alloc-{}", uuid::Uuid::new_v4()),
                status: AllocationStatus::Allocated {
                    worker,
                    container_id: None,
                },
            }),
            Err(e) => Err(DomainError::Infrastructure(format!(
                "Provider error: {}",
                e
            ))),
        }
    }

    /// Get available capacity
    async fn get_available_capacity(&self) -> Result<u32> {
        let status = self
            .status()
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Internal error: {}", e)))?;
        Ok(status
            .available_capacity
            .saturating_sub(self.allocations.len() as u32))
    }
}

#[async_trait]
impl ResourcePool for ResourcePoolService {
    fn config(&self) -> &ResourcePoolConfig {
        &self.config
    }

    async fn status(&self) -> std::result::Result<ResourcePoolStatus, String> {
        let worker_count = self.provider.list_workers().await.unwrap_or_default().len();
        let available = self.config.max_size.saturating_sub(worker_count as u32);

        Ok(ResourcePoolStatus {
            name: self.config.name.clone(),
            provider_type: self.config.provider_type.clone(),
            total_capacity: self.config.max_size,
            available_capacity: available,
            active_workers: worker_count as u32,
            pending_requests: self.pending_queue.len() as u32,
        })
    }

    async fn allocate_resources(
        &mut self,
        request: ResourceAllocationRequest,
    ) -> std::result::Result<PoolResourceAllocation, String> {
        info!(
            pool_name = %self.config.name,
            request_id = %request.request_id,
            "Allocating resources"
        );

        let available = self.get_available_capacity().await.unwrap_or(0);

        if available > 0 {
            // Allocate immediately
            let allocation = self
                .allocate_internal(&request)
                .await
                .map_err(|e| ResourcePoolServiceError::Internal(e.to_string()).to_string())?;
            self.allocations
                .insert(allocation.allocation_id.clone(), allocation.clone());

            if let AllocationStatus::Allocated { ref worker, .. } = allocation.status {
                self.active_workers
                    .insert(worker.id.clone(), allocation.allocation_id.clone());
            }

            // Process queue if needed
            let _ = self.process_queue().await;

            Ok(allocation)
        } else {
            // Queue request
            let request_id = request.request_id.clone();
            self.pending_queue.push_back(request);
            let allocation = PoolResourceAllocation {
                request_id,
                worker_id: WorkerId::new(),
                allocation_id: format!("pending-{}", uuid::Uuid::new_v4()),
                status: AllocationStatus::Pending,
            };
            self.allocations
                .insert(allocation.allocation_id.clone(), allocation.clone());
            Ok(allocation)
        }
    }

    async fn release_resources(&mut self, allocation_id: &str) -> std::result::Result<(), String> {
        if let Some(allocation) = self.allocations.remove(allocation_id) {
            if let AllocationStatus::Allocated { ref worker, .. } = allocation.status {
                // Stop and delete worker
                if let Err(e) = self.provider.stop_worker(&worker.id, true).await {
                    warn!("Failed to stop worker {}: {}", worker.id, e);
                }

                self.active_workers.remove(&worker.id);

                info!(
                    pool_name = %self.config.name,
                    allocation_id = allocation_id,
                    "Released resources"
                );

                // Process queue after release
                let _ = self.process_queue().await;
            }
            Ok(())
        } else {
            Err("Allocation not found".to_string())
        }
    }

    async fn list_allocations(&self) -> std::result::Result<Vec<PoolResourceAllocation>, String> {
        Ok(self.allocations.values().cloned().collect())
    }

    async fn scale_to(&mut self, target_size: u32) -> std::result::Result<(), String> {
        info!(
            pool_name = %self.config.name,
            target_size = target_size,
            "Scaling pool"
        );

        // Note: In a real implementation, this would trigger provisioning/deprovisioning
        // For now, just update the max_size
        self.config.max_size = target_size;

        Ok(())
    }

    async fn list_workers(&self) -> std::result::Result<Vec<WorkerId>, String> {
        self.provider
            .list_workers()
            .await
            .map_err(|e| e.to_string())
    }
}

/// Resource pool service error
#[derive(thiserror::Error, Debug)]
pub enum ResourcePoolServiceError {
    #[error("Provider error: {0}")]
    Provider(ProviderError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<ProviderError> for ResourcePoolServiceError {
    fn from(e: ProviderError) -> Self {
        Self::Provider(e)
    }
}

/// Create a default Docker resource pool
pub async fn create_docker_resource_pool(
    name: String,
    min_size: u32,
    max_size: u32,
) -> Result<ResourcePoolService> {
    let config = ResourcePoolConfig {
        provider_type: ResourcePoolType::Docker,
        name,
        provider_name: "docker-provider".to_string(),
        min_size,
        max_size,
        default_resources: ResourceQuota {
            cpu_m: 1000,
            memory_mb: 2048,
            gpu: None,
        },
        tags: HashMap::new(),
    };

    let provider_config = ProviderConfig::docker(
        "docker-pool".to_string(),
        WorkerTemplate::new("docker-pool", "latest", "ubuntu:22.04"),
    );
    let factory = DefaultProviderFactory::new();
    let provider = factory
        .create_provider(provider_config)
        .await
        .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

    Ok(ResourcePoolService::new(config, provider))
}

/// Create a Kubernetes resource pool
pub async fn create_kubernetes_resource_pool(
    name: String,
    _namespace: String,
    min_size: u32,
    max_size: u32,
) -> Result<ResourcePoolService> {
    let config = ResourcePoolConfig {
        provider_type: ResourcePoolType::Kubernetes,
        name,
        provider_name: "k8s-provider".to_string(),
        min_size,
        max_size,
        default_resources: ResourceQuota {
            cpu_m: 1000,
            memory_mb: 2048,
            gpu: None,
        },
        tags: HashMap::new(),
    };

    let provider_config = ProviderConfig::kubernetes(
        "k8s-pool".to_string(),
        WorkerTemplate::new("k8s-pool", "latest", "ubuntu:22.04"),
    );
    let factory = DefaultProviderFactory::new();
    let provider = factory
        .create_provider(provider_config)
        .await
        .map_err(|e| DomainError::Infrastructure(e.to_string()))?;

    Ok(ResourcePoolService::new(config, provider))
}
