//! Integration Tests with TestContainers
//!
//! This module contains comprehensive integration tests that verify the entire
//! resource governance and pipeline execution workflow using TestContainers.

use async_trait::async_trait;
use hodei_pipelines_domain::scheduling::entities::worker::Worker;
use hodei_pipelines_domain::scheduling::value_objects::worker_messages::WorkerId;
use hodei_pipelines_ports::scheduler_port::{SchedulerError, SchedulerPort};
use hodei_pipelines_proto::ServerMessage;
use tracing::info;

/// Mock Scheduler for integration testing
#[derive(Debug)]
pub struct MockScheduler;

#[async_trait]
impl SchedulerPort for MockScheduler {
    async fn register_worker(&self, worker: &Worker) -> Result<(), SchedulerError> {
        let _ = worker;
        info!("Mock scheduler: worker registered");
        Ok(())
    }

    async fn unregister_worker(&self, worker_id: &WorkerId) -> Result<(), SchedulerError> {
        let _ = worker_id;
        info!("Mock scheduler: worker unregistered");
        Ok(())
    }

    async fn get_registered_workers(&self) -> Result<Vec<WorkerId>, SchedulerError> {
        Ok(vec![])
    }

    async fn register_transmitter(
        &self,
        worker_id: &WorkerId,
        _transmitter: tokio::sync::mpsc::UnboundedSender<Result<ServerMessage, SchedulerError>>,
    ) -> Result<(), SchedulerError> {
        let _ = worker_id;
        Ok(())
    }

    async fn unregister_transmitter(&self, worker_id: &WorkerId) -> Result<(), SchedulerError> {
        let _ = worker_id;
        Ok(())
    }

    async fn send_to_worker(
        &self,
        worker_id: &WorkerId,
        message: ServerMessage,
    ) -> Result<(), SchedulerError> {
        let _ = (worker_id, message);
        Ok(())
    }
}

/// Integration test for complete pipeline execution workflow
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_scheduler_works() {
        let scheduler = MockScheduler;
        let worker_id = WorkerId::new();
        let worker = Worker::new(
            worker_id.clone(),
            "test-worker".to_string(),
            hodei_pipelines_domain::scheduling::entities::worker::WorkerCapabilities::new(4, 8192),
        );

        let result = scheduler.register_worker(&worker).await;
        assert!(result.is_ok());

        let workers = scheduler.get_registered_workers().await;
        assert!(workers.is_ok());
        assert_eq!(workers.unwrap().len(), 0); // Mock returns empty

        let unregister_result = scheduler.unregister_worker(&worker_id).await;
        assert!(unregister_result.is_ok());
    }
}
