# Kubernetes-Style Scheduler Design for Hodei Jobs

## Overview

This document outlines the design for a Kubernetes-inspired scheduler for the Hodei Jobs CI/CD system. The scheduler will replace AI-based scheduling with proven, predictable algorithms similar to those used in Kubernetes.

## Multi-Backend Core Architecture

The scheduler is designed with a flexible abstraction layer supporting multiple execution backends:
- **Kubernetes**: Pods, Nodes, Clusters
- **Docker**: Standalone containers
- **Cloud VMs**: AWS EC2, Azure VMs, GCP Compute
- **Bare Metal**: Physical servers
- **Serverless**: Lambda, Azure Functions
- **HPC**: SLURM, PBS clusters

### Unified Scheduler Framework

```
┌─────────────────────────────────────────────────────────────┐
│               Multi-Backend Scheduler                        │
├─────────────────────────────────────────────────────────────┤
│  BACKEND ABSTRACTION LAYER                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │K8s Backend  │  │Docker       │  │Cloud VM     │         │
│  │Adapter      │  │Backend      │  │Backend      │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│  SCHEDULING PIPELINE                                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ 1. INFORM: Watch job queues from all backends           │ │
│  │ 2. FILTER: Cross-backend feasibility check              │ │
│  │ 3. SCORE: Rank backends and nodes                       │ │
│  │ 4. BIND: Assign to best backend/node                    │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│  BACKEND COORDINATION                                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │Backend      │  │Cross-Backend│  │Cost         │         │
│  │Registry     │  │Health Mon.  │  │Optimization │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

### Backend Abstraction Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              SchedulerBackend Trait                          │
├─────────────────────────────────────────────────────────────┤
│  Core Interface                                             │
│  - list_nodes() → Vec<WorkerNode>                          │
│  - get_node(id) → WorkerNode                               │
│  - bind_job(job_id, node_id) → Result<()>                  │
│  - get_job_status(job_id) → JobStatus                      │
│                                                               │
│  Backend Type                                               │
│  - Kubernetes, Docker, CloudVM, BareMetal, Serverless      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Concrete Implementations                       │
├─────────────────────────────────────────────────────────────┤
│  K8sSchedulerBackend      │  DockerSchedulerBackend         │
│  - Uses K8s API           │  - Uses Docker API              │
│  - Pod scheduling         │  - Container scheduling         │
│  - Node/Pod resources     │  - Host resources               │
│                           │                                 │
│  CloudVmSchedulerBackend  │  BareMetalSchedulerBackend      │
│  - Cloud APIs             │  - IPMI/Redfish                 │
│  - Instance types         │  - Physical resources           │
│  - Region/Zone aware      │  - Rack aware                   │
└─────────────────────────────────────────────────────────────┘
```

## Scheduling Strategies

### 1. Priority and Preemption

Jobs can have priorities (High, Medium, Low), and high-priority jobs can preempt lower-priority jobs.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobPriority {
    Critical,  // System critical jobs
    High,      // Production deployments
    Medium,    // Regular CI/CD jobs
    Low,       // Background jobs
    Batch,     // Low-priority batch processing
}
```

**Preemption Rules**:
- Critical jobs can preempt any other job
- High jobs can preempt Medium, Low, Batch
- Medium jobs can preempt Low, Batch
- Low and Batch cannot preempt anyone

### 2. Node Selection Criteria

#### Resource Requests and Limits

Each job specifies resource requirements:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: Option<f64>,      // e.g., 2.0 cores
    pub memory_bytes: Option<u64>,   // e.g., 4GB
    pub ephemeral_storage: Option<u64>,
    pub gpu_count: Option<u32>,
}
```

#### Node Affinity and Selectors

Jobs can specify where they should run:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSelector {
    pub labels: HashMap<String, String>,  // key: "arch", value: "amd64"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAffinity {
    pub required_during_scheduling: Vec<LabelSelector>,  // Must match
    pub preferred_during_scheduling: Vec<WeightedLabelSelector>,  // Soft constraints
}
```

### 3. Inter-job Affinity

Control which jobs should be co-located or separated:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PodAffinityTerm {
    // Jobs with matching labels should be placed together
    InSameRegion(Vec<LabelSelector>),
    
    // Jobs should NOT be in the same region
    InDifferentRegions(Vec<LabelSelector>),
    
    // Jobs should be in a specific number of regions
    InNRegions(Vec<LabelSelector>, u32),
}
```

### 4. Taints and Tolerations

Allow workers to be dedicated to specific types of jobs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Taint {
    pub key: String,          // e.g., "dedicated"
    pub operator: TaintOperator,
    pub value: String,        // e.g., "gpu-worker"
    pub effect: TaintEffect,  // NoSchedule, PreferNoSchedule, NoExecute
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toleration {
    pub key: String,
    pub operator: TaintOperator,
    pub value: String,
    pub effect: TaintEffect,
}
```

**Taint Effects**:
- `NoSchedule`: Jobs without matching toleration cannot be scheduled
- `PreferNoSchedule`: Scheduler tries to avoid scheduling without toleration
- `NoExecute`: Jobs are evicted if they don't have matching toleration

## Scheduling Algorithms

### 1. Queue Management Strategies

#### FIFO (First In, First Out)
Jobs are scheduled in the order they arrive.

#### Priority Queue
Jobs are ordered by priority, with preemption support.

#### Fair Queuing
Jobs are distributed fairly across tenants/users.

```rust
#[derive(Debug, Clone)]
pub enum QueueStrategy {
    Fifo,
    Priority,
    Fair {
        tenant_key: String,  // Group by tenant
        weights: HashMap<String, u32>,
    },
}
```

### 2. Worker Selection Algorithms

#### Least Loaded
Select the worker with the lowest current load.

#### Resource Balance
Select worker that best balances resource utilization across the cluster.

#### Bin Packing (Best Fit)
Pack jobs efficiently to minimize resource fragmentation.

#### Round Robin
Distribute jobs evenly across available workers.

### 3. Scoring Functions

Workers receive scores from multiple criteria, combined with weights:

```rust
#[derive(Debug, Clone)]
pub struct ScoringCriteria {
    pub resource_utilization: f64,      // Weight: 40
    pub load_balancing: f64,            // Weight: 30
    pub affinity_bonus: f64,            // Weight: 20
    pub network_proximity: f64,         // Weight: 10
}

impl Worker {
    fn calculate_score(&self, criteria: &ScoringCriteria) -> f64 {
        let resource_score = self.resource_score();      // 0-100
        let load_score = self.load_score();              // 0-100
        let affinity_score = self.affinity_score();      // 0-100
        
        (resource_score * criteria.resource_utilization) +
        (load_score * criteria.load_balancing) +
        (affinity_score * criteria.affinity_bonus)
    }
}
```

## Job Lifecycle with Scheduler

```
┌─────────────┐
│   Job       │
│  Created    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Job       │
│  Pending    │◄────── Scheduler watches this
└──────┬──────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐
│  Scheduler  │────►│  Find Best  │
│ Evaluation  │     │  Worker     │
└──────┬──────┘     └──────┬──────┘
       │                   │
       ▼                   ▼
┌─────────────┐     ┌─────────────┐
│   Worker    │     │   Job       │
│  Assigned   │     │ Scheduled   │
└──────┬──────┘     └─────────────┘
       │
       ▼
┌─────────────┐
│   Job       │
│  Running    │
└─────────────┘
```

## Failure Handling

### Scheduling Failures

1. **No Suitable Worker**: Job remains pending with reason
2. **Preemption**: Lower priority job is moved to pending
3. **Timeout**: Job fails to schedule within SLA

### Recovery Strategies

1. **Retry with Relaxed Constraints**: Try scheduling without soft constraints
2. **Preemption Trigger**: Promote job priority to preempt others
3. **Backoff**: Exponential backoff for repeated failures

## Configuration

```rust
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub queue_strategy: QueueStrategy,
    pub worker_selection: WorkerSelectionStrategy,
    pub preemption_enabled: bool,
    pub timeout_seconds: u64,
    pub scoring_weights: ScoringCriteria,
}

#[derive(Debug, Clone)]
pub enum WorkerSelectionStrategy {
    LeastLoaded,
    ResourceBalance,
    BinPacking,
    RoundRobin,
    Custom(String),  // Plugin-based strategy
}
```

## Extensibility

The scheduler supports custom plugins:

```rust
#[async_trait]
pub trait SchedulingPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    
    async fn filter(&self, job: &Job, workers: Vec<Worker>) -> Vec<Worker>;
    
    async fn score(&self, job: &Job, worker: &Worker) -> f64;
    
    async fn on_job_scheduled(&self, job: &Job, worker: &Worker) -> Result<()>;
}
```

## Multiple Schedulers

Like Kubernetes, support for multiple schedulers:

```rust
pub struct SchedulerInstance {
    pub name: String,
    pub config: SchedulerConfig,
    pub plugins: Vec<Box<dyn SchedulingPlugin>>,
    pub namespace: Option<String>,  // Scheduler scope
}
```

## Monitoring and Metrics

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerMetrics {
    pub scheduling_latency_ms: u64,
    pub queue_wait_time_ms: u64,
    pub preemption_count: u64,
    pub scheduling_failures: u64,
    pub worker_utilization: HashMap<String, f64>,
}
```

## Implementation Plan

### Phase 1: Core Scheduler
1. Job queue management (Priority + FIFO)
2. Basic worker selection (Least Loaded)
3. Resource requirements checking
4. Simple affinity rules

### Phase 2: Advanced Features
1. Preemption support
2. Taints and tolerations
3. Multiple queue strategies
4. Scoring framework

### Phase 3: Extensibility
1. Plugin system
2. Custom algorithms
3. Multiple schedulers
4. Fine-grained affinity rules

## Benefits over AI-based Scheduling

1. **Predictability**: Deterministic scheduling decisions
2. **Debuggability**: Clear reasons for scheduling choices
3. **Performance**: O(log n) queue operations, O(n) filtering
4. **Control**: Fine-grained control over placement
5. **Proven**: Battle-tested in Kubernetes production
6. **Simplicity**: Easier to reason about and debug
7. **Resource Efficiency**: Optimal bin-packing algorithms

## References

- Kubernetes Scheduler: https://kubernetes.io/docs/concepts/scheduling-eviction/scheduling-framework/
- Scheduler Architecture: https://kubernetes.io/docs/reference/command-line-tools-reference/kube-scheduler/
- Scheduling Profiles: https://kubernetes.io/docs/reference/scheduling/config/#profiles
