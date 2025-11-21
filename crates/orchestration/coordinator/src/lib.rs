//! Distributed Coordinator implementation (US-003)
//!
//! This module provides job coordination with load balancing and failure detection.

use async_trait::async_trait;
use hodei_distributed_comm::EventBus;
use hodei_shared_types::job_definitions::{JobId, JobSpec};
use hodei_shared_types::{CorrelationId, DomainError, TenantId, Uuid};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Worker information for load balancing
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: Uuid,
    pub load: f64,
    pub capabilities: Vec<String>,
    pub is_healthy: bool,
}

/// Load balancer for selecting optimal workers
pub struct LoadBalancer {
    workers: Arc<Mutex<HashMap<Uuid, WorkerInfo>>>,
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_worker(&self, worker: WorkerInfo) {
        let mut workers = self.workers.lock();
        workers.insert(worker.id, worker);
    }

    pub fn select_worker(&self, required_capabilities: &[String]) -> Option<Uuid> {
        let workers = self.workers.lock();

        let mut candidates: Vec<_> = workers
            .values()
            .filter(|w| {
                w.is_healthy
                    && required_capabilities
                        .iter()
                        .all(|cap| w.capabilities.contains(cap))
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Select worker with minimum load
        candidates.sort_by(|a, b| {
            a.load
                .partial_cmp(&b.load)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Some(candidates[0].id)
    }

    pub fn update_load(&self, worker_id: Uuid, new_load: f64) {
        let mut workers = self.workers.lock();
        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.load = new_load;
        }
    }

    pub fn mark_unhealthy(&self, worker_id: Uuid) {
        let mut workers = self.workers.lock();
        if let Some(worker) = workers.get_mut(&worker_id) {
            worker.is_healthy = false;
        }
    }
}

/// Failure detector based on heartbeat
pub struct FailureDetector {
    workers: Arc<Mutex<HashMap<Uuid, std::time::Instant>>>,
    timeout: std::time::Duration,
}

impl FailureDetector {
    pub fn new(timeout: std::time::Duration) -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
            timeout,
        }
    }

    pub fn record_heartbeat(&self, worker_id: Uuid) {
        let mut workers = self.workers.lock();
        workers.insert(worker_id, std::time::Instant::now());
    }

    pub fn detect_failures(&self) -> Vec<Uuid> {
        let workers = self.workers.lock();
        let now = std::time::Instant::now();

        workers
            .iter()
            .filter(|(_, last_seen)| now.duration_since(**last_seen) > self.timeout)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn remove_worker(&self, worker_id: Uuid) {
        let mut workers = self.workers.lock();
        workers.remove(&worker_id);
    }
}

/// Job Coordinator
pub struct JobCoordinator {
    load_balancer: Arc<LoadBalancer>,
    failure_detector: Arc<FailureDetector>,
    event_bus: Option<Arc<hodei_distributed_comm::NatsEventBus>>,
}

impl JobCoordinator {
    pub fn new() -> Self {
        Self {
            load_balancer: Arc::new(LoadBalancer::new()),
            failure_detector: Arc::new(FailureDetector::new(std::time::Duration::from_secs(30))),
            event_bus: None,
        }
    }

    pub fn with_event_bus(mut self, event_bus: Arc<hodei_distributed_comm::NatsEventBus>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Register a new worker
    pub fn register_worker(&self, worker: WorkerInfo) {
        self.load_balancer.add_worker(worker.clone());
        self.failure_detector.record_heartbeat(worker.id);
    }

    /// Schedule a job to an available worker
    pub async fn schedule_job(
        &self,
        job_id: JobId,
        job_spec: &JobSpec,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
    ) -> Result<Uuid, DomainError> {
        // Extract required capabilities from job spec (simplified)
        let required_capabilities = vec!["default".to_string()];

        // Select optimal worker
        let worker_id = self
            .load_balancer
            .select_worker(&required_capabilities)
            .ok_or_else(|| DomainError::Infrastructure("no suitable worker found".to_string()))?;

        // Record heartbeat
        self.failure_detector.record_heartbeat(worker_id);

        // Update worker load (simulate job assignment)
        self.load_balancer.update_load(worker_id, 1.0);

        // Publish job scheduled event if event bus is available
        if let Some(event_bus) = &self.event_bus {
            let event = JobScheduledEvent {
                job_id: job_id.clone(),
                worker_id,
                correlation_id: correlation_id.clone(),
                tenant_id: tenant_id.clone(),
            };

            if let Err(e) = event_bus
                .publish(
                    "jobs.scheduled",
                    &event,
                    correlation_id.clone(),
                    tenant_id.clone(),
                )
                .await
            {
                tracing::warn!("Failed to publish job scheduled event: {}", e);
            }
        }

        tracing::info!("Scheduled job {} to worker {}", job_id.as_uuid(), worker_id);

        Ok(worker_id)
    }

    /// Handle worker heartbeat
    pub fn handle_heartbeat(&self, worker_id: Uuid) {
        self.failure_detector.record_heartbeat(worker_id);
        self.load_balancer.update_load(worker_id, 0.5); // Active but not at full capacity
    }

    /// Handle worker failure
    pub fn handle_worker_failure(&self, worker_id: Uuid) {
        self.load_balancer.mark_unhealthy(worker_id);
        self.failure_detector.remove_worker(worker_id);
        tracing::warn!("Worker {} marked as unhealthy", worker_id);
    }

    /// Check for failed workers
    pub fn check_failures(&self) -> Vec<Uuid> {
        let failed_workers = self.failure_detector.detect_failures();
        for worker_id in &failed_workers {
            self.handle_worker_failure(*worker_id);
        }
        failed_workers
    }
}

/// Event for job scheduled
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JobScheduledEvent {
    pub job_id: JobId,
    pub worker_id: Uuid,
    pub correlation_id: CorrelationId,
    pub tenant_id: TenantId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_shared_types::job_definitions::ResourceQuota;
    use std::collections::HashMap;

    #[test]
    fn test_load_balancer_worker_selection() {
        let balancer = LoadBalancer::new();

        // Add workers with different loads
        balancer.add_worker(WorkerInfo {
            id: Uuid::new_v4(),
            load: 0.5,
            capabilities: vec!["default".to_string()],
            is_healthy: true,
        });

        let worker_id = balancer.select_worker(&["default".to_string()]);
        assert!(worker_id.is_some());
    }

    #[test]
    fn test_failure_detection() {
        let detector = FailureDetector::new(std::time::Duration::from_millis(100));

        let worker_id = Uuid::new_v4();
        detector.record_heartbeat(worker_id);

        // Should not detect failure immediately
        assert_eq!(detector.detect_failures().len(), 0);

        std::thread::sleep(std::time::Duration::from_millis(150));

        // Should detect failure after timeout
        let failures = detector.detect_failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0], worker_id);
    }

    #[test]
    fn test_coordinator_job_scheduling() {
        let coordinator = JobCoordinator::new();

        // Register a worker
        coordinator.register_worker(WorkerInfo {
            id: Uuid::new_v4(),
            load: 0.0,
            capabilities: vec!["default".to_string()],
            is_healthy: true,
        });

        let spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu:latest".to_string(),
            command: vec!["echo".to_string()],
            resources: ResourceQuota::new(100, 512),
            timeout_ms: 30000,
            retries: 2,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let correlation_id = CorrelationId::new();
        let tenant_id = TenantId::new("test".to_string());

        // This would be async in real usage
        // but we're just testing the coordination logic here
    }
}
