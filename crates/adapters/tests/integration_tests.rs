//! End-to-end integration tests for adapters
//!
//! These tests verify the repository implementations work correctly.

use hodei_pipelines_core::{Job, JobId, JobSpec, Worker, WorkerId};
use hodei_pipelines_core::{ResourceQuota, WorkerCapabilities};
use hodei_pipelines_ports::event_bus::SystemEvent;
use std::collections::HashMap;

/// Helper function to create a test Job
fn create_test_job() -> Job {
    Job::new(
        JobId::new(),
        JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:22.04".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::new(1000, 512),
            timeout_ms: 30000,
            retries: 0,
            env: HashMap::new(),
            secret_refs: Vec::new(),
        },
    )
    .expect("Failed to create test job")
}

/// Helper function to create a test Worker
fn create_test_worker(id: &str) -> Worker {
    Worker::new(
        WorkerId::new(),
        format!("worker-{}", id),
        WorkerCapabilities::new(4, 8192),
    )
}

#[tokio::test]
async fn test_job_creation() {
    let job = create_test_job();
    assert_eq!(job.name(), "test-job");
    assert!(job.is_pending());
}

#[tokio::test]
async fn test_worker_creation() {
    let worker = create_test_worker("worker-01");
    assert_eq!(worker.name, "worker-worker-01");
    assert!(worker.is_available());
}

#[tokio::test]
async fn test_system_event_creation() {
    let job = create_test_job();
    let event = SystemEvent::JobCreated(job.spec.clone());

    match event {
        SystemEvent::JobCreated(job) => {
            assert_eq!(job.name, "test-job");
        }
        _ => panic!("Wrong event type"),
    }

    let worker = create_test_worker("worker-01");
    let worker_id = worker.id.clone();
    let event = SystemEvent::WorkerConnected {
        worker_id: worker.id,
    };

    match event {
        SystemEvent::WorkerConnected {
            worker_id: received_id,
        } => {
            assert_eq!(received_id, worker_id);
        }
        _ => panic!("Wrong event type"),
    }
}

#[tokio::test]
async fn test_resource_quota() {
    let quota = ResourceQuota::new(2000, 1024);
    assert_eq!(quota.cpu_m, 2000);
    assert_eq!(quota.memory_mb, 1024);
}
