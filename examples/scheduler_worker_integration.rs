//! Scheduler and Worker Lifecycle Integration Example
//!
//! This example demonstrates how to integrate the Kubernetes-style scheduler
//! framework with the worker lifecycle management system for the Hodei Jobs
//! distributed CI/CD platform.
//!
//! Run with: cargo run --example scheduler_worker_integration

use hodei_scheduler::backend::{ComputeResource, KubernetesBackend, SchedulerBackend};
use hodei_scheduler::integration::SchedulerWorkerIntegration;
use hodei_scheduler::types::{
    BackendSpecific, BackendType, Job, JobMetadata, JobPriority, JobSpec, KubernetesNodeSpecific,
    LabelSelector, LabelSelectorOperator, NodeAffinity, NodeLocation, NodeSelector,
    ResourceRequirements, WeightedLabelSelector, WorkerNode,
};
use hodei_shared_types::TenantId;
use hodei_worker_lifecycle::{Worker, WorkerCapabilities, WorkerManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Scheduler-Worker Lifecycle Integration Example\n");

    // Step 1: Create Worker Lifecycle Manager
    println!("📋 Step 1: Creating Worker Lifecycle Manager");
    let worker_manager = Arc::new(WorkerManager::new(
        None,                    // No coordinator needed for this example
        Duration::from_secs(30), // Health check interval
        Duration::from_secs(10), // Heartbeat timeout
    ));

    // Step 2: Create Scheduler Backend (Mock for example)
    println!("☸️  Step 2: Creating Mock Scheduler Backend");
    let backend = Arc::new(KubernetesBackend::new());

    // Step 3: Create the Integration Coordinator
    println!("🔗 Step 3: Creating Scheduler-Worker Integration Coordinator");
    let integration = SchedulerWorkerIntegration::new(backend.clone(), worker_manager.clone());

    // Step 4: Start the integration
    println!("▶️  Starting integration coordinator...");
    integration.start().await?;
    println!("✅ Integration coordinator started successfully\n");

    // Step 5: Register workers in lifecycle manager
    println!("👥 Step 5: Registering Workers");
    let worker_ids = register_workers(&worker_manager).await?;
    println!("✅ Registered {} workers\n", worker_ids.len());

    // Step 6: Create a sample job
    println!("📝 Step 6: Creating Sample Job");
    let job = create_sample_job();
    println!(
        "✅ Created job: {} (priority: {})\n",
        job.metadata.name, job.spec.priority
    );

    // Step 7: Find suitable workers for the job
    println!("🔍 Step 7: Finding Suitable Workers");
    match integration.find_suitable_workers(&job).await {
        Ok(workers) => {
            println!("✅ Found {} suitable workers:", workers.len());
            for worker in &workers {
                println!("   - Worker ID: {}", worker.id);
                println!(
                    "     CPU: {:.1} cores, Memory: {} bytes",
                    worker.resources.cpu_cores, worker.resources.memory_bytes
                );
            }
            println!();

            // Step 8: Bind job to a worker
            if let Some(first_worker) = workers.first() {
                println!("🔗 Step 8: Binding Job to Worker");
                let job_id = job.metadata.id;
                let worker_id = first_worker.id;

                integration.bind_job_to_worker(&job_id, &worker_id).await?;
                println!(
                    "✅ Successfully bound job {} to worker {}\n",
                    job_id, worker_id
                );

                // Step 9: Verify binding
                println!("✅ Step 9: Verifying Job Binding");
                assert!(integration.is_job_bound(&job_id));
                assert_eq!(integration.get_job_worker(&job_id), Some(worker_id));

                let binding_map = integration.get_job_to_worker_map();
                println!("   Active bindings: {}", binding_map.len());
                println!("   Job {} is bound to worker {}\n", job_id, worker_id);

                // Step 10: Simulate job completion
                println!("✅ Step 10: Simulating Job Completion");
                integration
                    .unbind_job_from_worker(&job_id, &worker_id)
                    .await?;
                println!("✅ Job unbound from worker\n");
            }
        }
        Err(e) => {
            println!("❌ No suitable workers found: {}", e);
        }
    }

    // Step 11: Test worker failure handling
    println!("❌ Step 11: Testing Worker Failure Handling");
    if let Some(first_worker_id) = worker_ids.first() {
        println!("   Simulating failure for worker: {}", first_worker_id);
        integration.handle_worker_failed(*first_worker_id).await?;
        println!("✅ Worker failure handled successfully\n");
    }

    // Step 12: Get worker status
    println!("📊 Step 12: Getting Worker Status Summary");
    let worker_summaries = integration.get_workers_summary().await;
    println!("   Total workers: {}", worker_summaries.len());

    for summary in worker_summaries {
        println!(
            "   - Worker {}: state={}, load={:.2}, jobs={}",
            summary.worker_id, summary.state, summary.load, summary.jobs_running
        );
    }
    println!();

    // Step 13: Stop the integration
    println!("⏹️  Step 13: Stopping Integration Coordinator");
    integration.stop().await?;
    println!("✅ Integration coordinator stopped successfully\n");

    println!("✨ Example completed successfully!");

    Ok(())
}

/// Register sample workers in the lifecycle manager
async fn register_workers(
    worker_manager: &Arc<WorkerManager>,
) -> Result<Vec<Uuid>, Box<dyn std::error::Error>> {
    let mut worker_ids = Vec::new();

    // Worker 1: High-capacity node
    let worker1_id = Uuid::new_v4();
    worker_manager.register_worker(Worker::new(
        worker1_id,
        WorkerCapabilities {
            cpu_cores: 16,
            memory_gb: 64,
            has_gpu: true,
            gpu_count: Some(2),
            specialized_hardware: vec!["nvme-ssd".to_string()],
            container_runtime: "containerd".to_string(),
        },
        10, // max_jobs
        None,
    ))?;
    worker_ids.push(worker1_id);
    println!("   ✅ Registered worker 1 (ID: {})", worker1_id);

    // Worker 2: Medium-capacity node
    let worker2_id = Uuid::new_v4();
    worker_manager.register_worker(Worker::new(
        worker2_id,
        WorkerCapabilities {
            cpu_cores: 8,
            memory_gb: 32,
            has_gpu: false,
            gpu_count: None,
            specialized_hardware: vec![],
            container_runtime: "docker".to_string(),
        },
        5, // max_jobs
        None,
    ))?;
    worker_ids.push(worker2_id);
    println!("   ✅ Registered worker 2 (ID: {})", worker2_id);

    // Worker 3: GPU node
    let worker3_id = Uuid::new_v4();
    worker_manager.register_worker(Worker::new(
        worker3_id,
        WorkerCapabilities {
            cpu_cores: 4,
            memory_gb: 16,
            has_gpu: true,
            gpu_count: Some(1),
            specialized_hardware: vec!["cuda".to_string()],
            container_runtime: "docker".to_string(),
        },
        3, // max_jobs
        None,
    ))?;
    worker_ids.push(worker3_id);
    println!("   ✅ Registered worker 3 (ID: {})", worker3_id);

    Ok(worker_ids)
}

/// Create a sample job with resource requirements
fn create_sample_job() -> Job {
    Job {
        metadata: JobMetadata {
            id: Uuid::new_v4(),
            name: "ci-pipeline-build".to_string(),
            namespace: "default".to_string(),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("app".to_string(), "hodei-jobs".to_string());
                labels.insert("tier".to_string(), "ci".to_string());
                labels
            },
            created_at: chrono::Utc::now(),
        },
        spec: JobSpec {
            resource_requirements: Some(ResourceRequirements {
                cpu_cores: Some(4.0),
                memory_bytes: Some(8_000_000_000), // 8 GB
                gpu_count: Some(1),
                ephemeral_storage: Some(20_000_000_000), // 20 GB
            }),
            priority: JobPriority::High,
            node_selector: Some(NodeSelector {
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("node-type".to_string(), "compute".to_string());
                    labels
                },
            }),
            affinity: Some(NodeAffinity {
                required_during_scheduling: vec![LabelSelector {
                    key: "rack".to_string(),
                    operator: LabelSelectorOperator::In,
                    values: Some(vec!["rack-a".to_string(), "rack-b".to_string()]),
                }],
                preferred_during_scheduling: vec![WeightedLabelSelector {
                    selector: LabelSelector {
                        key: "gpu".to_string(),
                        operator: LabelSelectorOperator::Exists,
                        values: None,
                    },
                    weight: 10,
                }],
            }),
            tolerations: vec![],
            max_retries: 3,
        },
    }
}
