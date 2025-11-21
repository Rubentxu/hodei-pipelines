# Scheduler Integration Documentation

## Overview

The Scheduler-Worker Lifecycle Integration (US-012) bridges the Kubernetes-style scheduler framework with the worker lifecycle management system, enabling:

- **Real-time worker state synchronization** between the scheduler and lifecycle manager
- **Event-driven scheduling** based on worker lifecycle events (registration, failure, deregistration)
- **Automatic job rescheduling** on worker failures
- **Coordinated preemption** and job cleanup
- **Health monitoring** and recovery mechanisms

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Scheduler Framework                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Queue     │  │  Pipeline   │  │  Selection  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Backend    │  │  Affinity   │  │  Multi-Sched│         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│           SchedulerWorkerIntegration Coordinator             │
│  - Event handlers (LoadBalancer, Preemption, Metrics)      │
│  - Job-to-Worker mapping                                    │
│  - Worker lifecycle synchronization                         │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│                Worker Lifecycle Manager                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Register   │  │  Heartbeat  │  │  Failure    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Health    │  │  Draining   │  │   Cleanup   │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. SchedulerWorkerIntegration

The main coordinator that manages the integration between scheduler and worker lifecycle:

```rust
pub struct SchedulerWorkerIntegration {
    scheduler_backend: Arc<dyn SchedulerBackend>,
    worker_manager: Arc<WorkerManager>,
    event_handlers: Arc<Mutex<Vec<Box<dyn EventHandler>>>>,
    job_to_worker_map: Arc<Mutex<HashMap<JobId, WorkerId>>>,
    shutdown_tx: broadcast::Sender<()>,
}
```

### 2. Event Handlers

Three built-in event handlers for different concerns:

- **LoadBalancingHandler**: Handles load balancing when workers join/leave
- **PreemptionHandler**: Manages job preemption during failures
- **MetricsHandler**: Collects integration metrics

### 3. Job-to-Worker Mapping

Maintains a thread-safe mapping of jobs to their assigned workers:

```rust
let binding_map = integration.get_job_to_worker_map();
let worker_id = integration.get_job_worker(&job_id)?;
```

## Usage Example

### Basic Integration Setup

```rust
use hodei_scheduler::backend::KubernetesBackend;
use hodei_scheduler::integration::SchedulerWorkerIntegration;
use hodei_worker_lifecycle::WorkerManager;

async fn setup_integration() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create worker lifecycle manager
    let worker_manager = Arc::new(WorkerManager::new(
        None,
        Duration::from_secs(30), // Health check interval
        Duration::from_secs(10), // Heartbeat timeout
    ));

    // 2. Create scheduler backend
    let backend = Arc::new(KubernetesBackend::new(
        "default".to_string(),
        "https://kubernetes.default.svc:443".to_string(),
    ));

    // 3. Create integration coordinator
    let integration = SchedulerWorkerIntegration::new(
        backend,
        worker_manager,
    );

    // 4. Start the integration
    integration.start().await?;

    // 5. Use the integration...
    
    Ok(())
}
```

### Finding Suitable Workers

```rust
let job = create_sample_job();

// Find workers that match job requirements
let suitable_workers = integration.find_suitable_workers(&job).await?;

for worker in suitable_workers {
    println!("Found worker: {} with {} cores",
        worker.id, worker.resources.cpu_cores);
}
```

### Binding Jobs to Workers

```rust
let job_id = job.metadata.id;
let worker_id = chosen_worker.id;

// Record the binding
integration.bind_job_to_worker(&job_id, &worker_id).await?;

// Verify binding
assert!(integration.is_job_bound(&job_id));
```

### Handling Worker Events

```rust
// Worker registration
integration.handle_worker_registered(worker_id).await?;

// Worker heartbeat
integration.handle_worker_heartbeat(worker_id, 0.5, 2).await?;

// Worker failure (triggers automatic job rescheduling)
integration.handle_worker_failed(worker_id).await?;
```

### Worker Status Monitoring

```rust
// Get all worker summaries
let worker_summaries = integration.get_workers_summary().await;

for summary in worker_summaries {
    println!("Worker {}: state={}, load={:.2}",
        summary.worker_id,
        summary.state,
        summary.load
    );
}
```

## Worker Lifecycle States

The integration maintains the following worker lifecycle states:

- **AVAILABLE**: Worker is ready to accept new jobs
- **RUNNING**: Worker is currently executing jobs
- **UNHEALTHY**: Worker has failed and is not accepting new jobs
- **DRAINING**: Worker is completing existing jobs before shutdown

## Event Flow

### 1. Worker Registration Flow

```
Worker registers → Lifecycle Manager → Integration Coordinator →
Event Handlers → Update Backend → Trigger Job Retry
```

### 2. Worker Failure Flow

```
Worker fails → Lifecycle Manager detects → Integration Coordinator →
Get bound jobs → Reschedule jobs → Update mappings
```

### 3. Job Binding Flow

```
Scheduler selects worker → Integration binds job →
Record in mapping → Backend binds job → Verify binding
```

## Performance Considerations

- **Worker State Sync**: Periodic synchronization between lifecycle manager and backend
- **Event Handler Overhead**: Minimal overhead with async event processing
- **Job Mapping**: O(1) lookup for job-to-worker bindings using HashMap
- **Lock Contention**: Fine-grained locking to minimize contention

## Error Handling

The integration provides comprehensive error handling:

```rust
pub enum IntegrationError {
    SchedulerError(#[from] crate::SchedulerError),
    WorkerLifecycleError(String),
    WorkerNotFound(WorkerId),
    JobNotBound,
    Integration(String),
}
```

## Testing

The integration includes comprehensive tests:

```bash
cargo test -p hodei-scheduler --lib integration
```

Tests cover:
- Integration creation and shutdown
- Worker registration handling
- Heartbeat processing
- Job binding tracking
- Worker failure handling
- Suitable worker discovery

## Best Practices

1. **Always start the integration** before using it
2. **Register workers in lifecycle manager** before scheduling
3. **Handle worker events promptly** to avoid stale state
4. **Monitor worker status** for capacity planning
5. **Graceful shutdown** by calling `stop()`

## Example Applications

See the complete example at:
`examples/scheduler_worker_integration.rs`

Run with:
```bash
cargo run --example scheduler_worker_integration
```

This example demonstrates:
- Setting up the integration
- Registering workers
- Creating and scheduling jobs
- Handling failures
- Monitoring worker status
- Cleanup and shutdown

## Related Documentation

- [Scheduler Framework](crate::scheduler)
- [Worker Lifecycle Management](../orchestration/worker-lifecycle)
- [Pipeline Documentation](crate::pipeline)
- [Selection Strategies](crate::selection)
- [Affinity Rules](crate::affinity)
