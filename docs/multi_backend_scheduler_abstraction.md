# Scheduler Abstraction: Multi-Backend Support Design

## Overview

The scheduler must be designed with a flexible abstraction layer that supports multiple execution backends:
- **Kubernetes**: Pods, DaemonSets, Nodes
- **Docker**: Standalone containers
- **Virtual Machines**: Cloud VMs (AWS EC2, Azure VMs, GCP), on-prem VMs
- **Bare Metal**: Physical servers
- **Serverless**: AWS Lambda, Azure Functions, etc.
- **HPC**: SLURM, PBS clusters
- **Edge Devices**: IoT, edge compute nodes

## Abstraction Layers

### 1. Compute Resource Abstraction

```rust
/// Unified representation of compute resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResource {
    pub cpu_cores: Option<f64>,
    pub cpu_architecture: String, // x86_64, ARM64, etc.
    pub memory_bytes: Option<u64>,
    pub ephemeral_storage: Option<u64>,
    pub gpu_count: Option<u32>,
    pub gpu_type: Option<String>, // Tesla V100, A100, etc.
    pub accelerators: Vec<Accelerator>, // TPU, FPGA, etc.
    pub custom_resources: HashMap<String, CustomResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomResource {
    pub resource_type: String, // "nvidia.com/t4", "amd/instinct", etc.
    pub quantity: u64,
}

/// Backend-specific resource format
pub enum ResourceFormat {
    Kubernetes {
        requests: ResourceRequirements,
        limits: ResourceRequirements,
    },
    Docker {
        cpu_limit: Option<f64>,
        memory_limit: Option<u64>,
    },
    CloudVm {
        instance_type: String,
    },
    Serverless {
        memory: u64,
        timeout: Duration,
    },
}
```

### 2. Node/Worker Abstraction

```rust
/// Unified worker/node representation
#[derive(Debug, Clone)]
pub struct WorkerNode {
    pub id: WorkerId,
    pub backend_type: BackendType,
    pub status: WorkerStatus,
    pub resources: ComputeResource,
    pub labels: HashMap<String, String>,
    pub taints: Vec<Taint>,
    pub backend_specific: BackendSpecific,
    pub location: NodeLocation,
}

#[derive(Debug, Clone)]
pub enum BackendType {
    Kubernetes,
    DockerStandalone,
    CloudVm {
        provider: CloudProvider,
        region: String,
        availability_zone: Option<String>,
    },
    BareMetal {
        datacenter: String,
        rack: Option<String>,
    },
    Serverless {
        provider: String,
        runtime: String,
    },
    HpcCluster {
        scheduler: HpcScheduler, // SLURM, PBS, etc.
        partition: String,
    },
}

#[derive(Debug, Clone)]
pub enum BackendSpecific {
    Kubernetes {
        node_name: String,
        kubelet_version: String,
        pod_capacity: u32,
        allocatable: ComputeResource,
    },
    DockerStandalone {
        docker_version: String,
        container_count: u32,
        max_containers: u32,
    },
    CloudVm {
        instance_id: String,
        instance_type: String,
        cloud_provider: CloudProvider,
        vpc: String,
        subnet: String,
    },
    Serverless {
        provider: String,
        region: String,
        function_name: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct NodeLocation {
    pub region: Option<String>,
    pub zone: Option<String>,
    pub datacenter: Option<String>,
    pub rack: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}
```

### 3. Job/Workload Abstraction

```rust
/// Unified job representation
#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub namespace: String,
    pub priority: JobPriority,
    pub resource_requirements: ComputeResource,
    pub backend_preferences: BackendPreferences,
    pub affinity_rules: Vec<AffinityRule>,
    pub constraints: Vec<SchedulingConstraint>,
    pub workload_type: WorkloadType,
    pub payload: JobPayload,
}

#[derive(Debug, Clone)]
pub enum BackendPreferences {
    Specific(BackendType), // Must run on this backend
    Preferred(Vec<BackendType>), // Prefer these backends
    Any, // Any backend is acceptable
}

#[derive(Debug, Clone)]
pub enum WorkloadType {
    LongRunning {
        max_duration: Option<Duration>,
        restart_policy: RestartPolicy,
    },
    Batch {
        estimated_duration: Duration,
        max_retries: u32,
    },
    Serverless {
        timeout: Duration,
        cold_start_acceptable: bool,
    },
    DataProcessing {
        input_data_size: Option<u64>,
        output_data_size: Option<u64>,
    },
    Interactive {
        max_execution_time: Duration,
        idle_timeout: Duration,
    },
}
```

### 4. Scheduler Backend Abstraction

```rust
/// Backend-specific scheduler implementation
#[async_trait]
pub trait SchedulerBackend: Send + Sync {
    /// Backend identifier
    fn backend_type(&self) -> BackendType;
    
    /// Discover and list all available nodes
    async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError>;
    
    /// Get node details
    async fn get_node(&self, node_id: &WorkerId) -> Result<WorkerNode, SchedulerError>;
    
    /// Check if node is schedulable
    fn is_node_schedulable(&self, node: &WorkerNode) -> bool;
    
    /// Reserve node for a job (pre-binding)
    async fn reserve_node(
        &self,
        node_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<Reservation, SchedulerError>;
    
    /// Bind job to node (actual assignment)
    async fn bind_job(
        &self,
        job_id: &JobId,
        node_id: &WorkerId,
    ) -> Result<(), SchedulerError>;
    
    /// Unbind job from node
    async fn unbind_job(
        &self,
        job_id: &JobId,
        node_id: &WorkerId,
    ) -> Result<(), SchedulerError>;
    
    /// Get job status
    async fn get_job_status(
        &self,
        job_id: &JobId,
    ) -> Result<JobStatus, SchedulerError>;
    
    /// Get node metrics
    async fn get_node_metrics(
        &self,
        node_id: &WorkerId,
    ) -> Result<NodeMetrics, SchedulerError>;
    
    /// Execute backend-specific command
    async fn execute_command(
        &self,
        node_id: &WorkerId,
        command: &str,
    ) -> Result<CommandResult, SchedulerError>;
}

/// Kubernetes Scheduler Backend Implementation
pub struct KubernetesSchedulerBackend {
    client: K8sClient,
    namespace: String,
}

#[async_trait]
impl SchedulerBackend for KubernetesSchedulerBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Kubernetes
    }
    
    async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError> {
        let nodes = self.client.list_nodes().await?;
        Ok(nodes.into_iter().map(|k8s_node| {
            WorkerNode {
                id: WorkerId::new(k8s_node.name),
                backend_type: BackendType::Kubernetes,
                status: convert_k8s_status(k8s_node.status),
                resources: convert_k8s_resources(&k8s_node.spec.capacity),
                labels: k8s_node.metadata.labels,
                taints: convert_k8s_taints(&k8s_node.spec.taints),
                backend_specific: BackendSpecific::Kubernetes {
                    node_name: k8s_node.name,
                    kubelet_version: k8s_node.status.node_info.kubelet_version,
                    pod_capacity: k8s_node.status.capacity.pods().unwrap_or(0) as u32,
                    allocatable: convert_k8s_resources(&k8s_node.status.allocatable),
                },
                location: extract_k8s_location(&k8s_node.metadata.labels),
            }
        }).collect())
    }
    
    // ... implement other methods
}

/// Docker Scheduler Backend Implementation
pub struct DockerSchedulerBackend {
    docker: DockerClient,
}

#[async_trait]
impl SchedulerBackend for DockerSchedulerBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::DockerStandalone
    }
    
    async fn list_nodes(&self) -> Result<Vec<WorkerNode>, SchedulerError> {
        let containers = self.docker.list_containers().await?;
        Ok(vec![WorkerNode {
            id: WorkerId::new("docker-host-1"),
            backend_type: BackendType::DockerStandalone,
            status: WorkerStatus::Running,
            resources: detect_docker_resources(),
            labels: HashMap::new(),
            taints: vec![],
            backend_specific: BackendSpecific::DockerStandalone {
                docker_version: self.docker.version().await?.version,
                container_count: containers.len() as u32,
                max_containers: 100, // Configurable
            },
            location: NodeLocation::default(),
        }])
    }
    
    // ... implement other methods
}
```

### 5. Multi-Backend Scheduler

```rust
/// Scheduler that manages multiple backends
pub struct MultiBackendScheduler {
    backends: HashMap<BackendType, Box<dyn SchedulerBackend>>,
    backend_preferences: BackendPreferencePolicy,
    fallback_chain: Vec<BackendType>,
    health_monitor: HealthMonitor,
}

impl MultiBackendScheduler {
    /// Add a backend to the scheduler
    pub fn add_backend(&mut self, backend: Box<dyn SchedulerBackend>) {
        let backend_type = backend.backend_type();
        self.backends.insert(backend_type, backend);
    }
    
    /// Schedule a job across all or specific backends
    pub async fn schedule_job(
        &self,
        job: &Job,
    ) -> Result<SchedulingResult, SchedulerError> {
        // Determine which backends to consider
        let candidate_backends = self.select_candidate_backends(job).await?;
        
        // Try scheduling on each backend in order
        for backend_type in candidate_backends {
            if let Some(backend) = self.backends.get(&backend_type) {
                // Check backend health
                if !self.health_monitor.is_backend_healthy(backend_type).await {
                    continue;
                }
                
                // Attempt scheduling
                match self.attempt_scheduling(backend, job).await {
                    Ok(result) => return Ok(result),
                    Err(SchedulerError::NoSuitableNodes) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        
        Err(SchedulerError::NoSuitableNodes)
    }
    
    /// Select candidate backends based on job preferences
    async fn select_candidate_backends(
        &self,
        job: &Job,
    ) -> Result<Vec<BackendType>, SchedulerError> {
        match &job.backend_preferences {
            BackendPreferences::Specific(backend_type) => {
                if self.backends.contains_key(backend_type) {
                    Ok(vec![backend_type.clone()])
                } else {
                    Err(SchedulerError::BackendNotAvailable(backend_type.clone()))
                }
            }
            BackendPreferences::Preferred(preferred) => {
                // Check which preferred backends are available
                let available: Vec<BackendType> = preferred
                    .iter()
                    .filter(|b| self.backends.contains_key(b))
                    .cloned()
                    .collect();
                
                if !available.is_empty() {
                    Ok(available)
                } else {
                    // Fallback to any available backend
                    self.get_all_available_backends()
                }
            }
            BackendPreferences::Any => self.get_all_available_backends(),
        }
    }
    
    /// Attempt scheduling on a specific backend
    async fn attempt_scheduling(
        &self,
        backend: &dyn SchedulerBackend,
        job: &Job,
    ) -> Result<SchedulingResult, SchedulerError> {
        let nodes = backend.list_nodes().await?;
        
        // Filter feasible nodes
        let feasible_nodes = self.filter_feasible_nodes(backend, job, &nodes)?;
        
        if feasible_nodes.is_empty() {
            return Err(SchedulerError::NoSuitableNodes);
        }
        
        // Score and rank nodes
        let scored_nodes = self.score_nodes(job, &feasible_nodes)?;
        
        // Select best node
        let best_node = scored_nodes
            .into_iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .ok_or(SchedulerError::NoSuitableNodes)?;
        
        // Reserve and bind
        backend.reserve_node(&best_node.node.id, &job.id).await?;
        backend.bind_job(&job.id, &best_node.node.id).await?;
        
        Ok(SchedulingResult {
            job_id: job.id.clone(),
            node_id: best_node.node.id,
            backend_type: backend.backend_type(),
            assigned_resources: best_node.node.resources.clone(),
            score: best_node.score,
            reasoning: best_node.reasoning,
        })
    }
}
```

### 6. Resource Conversion Utilities

```rust
/// Convert between backend-specific and unified resource formats
pub struct ResourceConverter;

impl ResourceConverter {
    /// Convert Kubernetes resources to ComputeResource
    pub fn k8s_to_compute(
        requests: &BTreeMap<String, resource::Quantity>,
        limits: &BTreeMap<String, resource::Quantity>,
    ) -> ComputeResource {
        ComputeResource {
            cpu_cores: parse_quantity(&requests.get("cpu").cloned()),
            memory_bytes: parse_quantity(&requests.get("memory").cloned()),
            ephemeral_storage: parse_quantity(&requests.get("ephemeral-storage").cloned()),
            gpu_count: parse_gpu_count(limits),
            gpu_type: parse_gpu_type(limits),
            accelerators: parse_accelerators(limits),
            custom_resources: parse_custom_resources(limits),
        }
    }
    
    /// Convert Docker configuration to ComputeResource
    pub fn docker_to_compute(
        cpu_limit: Option<f64>,
        memory_limit: Option<u64>,
    ) -> ComputeResource {
        ComputeResource {
            cpu_cores: cpu_limit,
            memory_bytes: memory_limit,
            ..Default::default()
        }
    }
    
    /// Convert cloud VM instance type to ComputeResource
    pub fn cloud_vm_to_compute(
        instance_type: &str,
        provider: CloudProvider,
    ) -> ComputeResource {
        match provider {
            CloudProvider::Aws => map_aws_instance_type(instance_type),
            CloudProvider::Azure => map_azure_instance_type(instance_type),
            CloudProvider::Gcp => map_gcp_instance_type(instance_type),
        }
    }
}
```

## Scheduling Policies for Multi-Backend

### Backend Affinity

```rust
#[derive(Debug, Clone)]
pub enum BackendAffinity {
    /// Job should run on specific backend type
    Required(BackendType),
    
    /// Job prefers specific backend types (soft constraint)
    Preferred(Vec<BackendType>, Vec<f64>), // backends and weights
    
    /// Job should avoid specific backend types
    Avoid(Vec<BackendType>),
    
    /// Job can run anywhere (no preference)
    Any,
}
```

### Cross-Backend Resource Scheduling

```rust
/// For jobs requiring resources across multiple backends
#[derive(Debug, Clone)]
pub struct DistributedJob {
    pub components: Vec<JobComponent>,
    pub execution_model: ExecutionModel,
}

#[derive(Debug, Clone)]
pub enum ExecutionModel {
    /// All components run on the same backend
    Colocated {
        preferred_backend: BackendType,
    },
    /// Components can run on different backends
    Distributed {
        allowed_backends: Vec<BackendType>,
        data_transfer_costs: HashMap<(BackendType, BackendType), f64>,
    },
    /// One component is primary, others are edge nodes
    HubAndSpoke {
        primary_backend: BackendType,
        edge_backends: Vec<BackendType>,
    },
}
```

## Implementation Strategy

### Phase 1: Core Abstraction
1. Define ComputeResource and WorkerNode abstractions
2. Implement SchedulerBackend trait
3. Start with Kubernetes backend
4. Create basic multi-backend orchestrator

### Phase 2: Additional Backends
1. Add Docker standalone backend
2. Add Cloud VM backend (AWS/Azure/GCP)
3. Add Bare metal backend
4. Implement resource conversion utilities

### Phase 3: Advanced Features
1. Serverless backend support
2. HPC cluster backend (SLURM)
3. Edge device backend
4. Multi-cloud federation
5. Backend-specific optimizations

### Phase 4: Orchestration
1. Cross-backend job distribution
2. Backend failover and recovery
3. Cost optimization across backends
4. Compliance and data residency
5. Multi-cloud disaster recovery

## Benefits

1. **Vendor Neutrality**: Not locked to a specific backend
2. **Flexibility**: Use the best tool for each job
3. **Cost Optimization**: Choose backend based on cost/performance
4. **Hybrid Deployment**: Mix on-prem, cloud, and edge
5. **Future-Proof**: Easy to add new backends
6. **Consistent Interface**: Same scheduler APIs regardless of backend

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_multi_backend_scheduling() {
        let mut scheduler = MultiBackendScheduler::new();
        
        // Add mock backends
        scheduler.add_backend(Box::new(MockKubernetesBackend::new()));
        scheduler.add_backend(Box::new(MockDockerBackend::new()));
        scheduler.add_backend(Box::new(MockCloudVmBackend::new()));
        
        // Create job that can run anywhere
        let job = JobBuilder::new()
            .backend_preference(BackendPreferences::Any)
            .resource_requirement(ComputeResource {
                cpu_cores: Some(4.0),
                memory_bytes: Some(8_000_000_000),
                ..Default::default()
            })
            .build();
        
        let result = scheduler.schedule_job(&job).await.unwrap();
        
        // Should schedule on one of the backends
        assert!(matches!(
            result.backend_type,
            BackendType::Kubernetes
                | BackendType::DockerStandalone
                | BackendType::CloudVm { .. }
        ));
    }
}
```

## Configuration Example

```yaml
scheduler:
  backends:
    - type: kubernetes
      kubeconfig: "~/.kube/config"
      namespace: "default"
      
    - type: docker_standalone
      socket_path: "/var/run/docker.sock"
      
    - type: cloud_vm
      provider: "aws"
      region: "us-east-1"
      credentials_profile: "default"
      
    - type: bare_metal
      api_endpoint: "https://baremetal.example.com/api"
      
  policies:
    backend_preference: "cost_optimized"  # or "performance", "locality"
    fallback_enabled: true
    health_check_interval: 30s
```

This abstraction ensures the scheduler works seamlessly across any execution backend, providing maximum flexibility for the Hodei Jobs CI/CD system.
