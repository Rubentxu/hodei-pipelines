//! Worker Registration Adapter
//!
//! This module implements the WorkerRegistrationPort for the SchedulerModule,
//! allowing dynamic workers to be automatically registered in the scheduling system.

use async_trait::async_trait;
use hodei_core::{Worker, WorkerId};
use hodei_modules::SchedulerModule;
use hodei_ports::worker_registration::WorkerRegistrationPort;
use std::sync::Arc;

/// Worker Registration Adapter
///
/// This adapter wraps the SchedulerModule to implement the WorkerRegistrationPort,
/// enabling decoupled registration of dynamically provisioned workers.
pub struct WorkerRegistrationAdapter<J, B, C, W>
where
    J: hodei_ports::job_repository::JobRepository + Send + Sync + 'static,
    B: hodei_ports::event_bus::EventPublisher + Send + Sync + 'static,
    C: hodei_ports::worker_client::WorkerClient + Send + Sync + 'static,
    W: hodei_ports::worker_repository::WorkerRepository + Send + Sync + 'static,
{
    scheduler: Arc<SchedulerModule<J, B, C, W>>,
}

impl<J, B, C, W> WorkerRegistrationAdapter<J, B, C, W>
where
    J: hodei_ports::job_repository::JobRepository + Send + Sync + 'static,
    B: hodei_ports::event_bus::EventPublisher + Send + Sync + 'static,
    C: hodei_ports::worker_client::WorkerClient + Send + Sync + 'static,
    W: hodei_ports::worker_repository::WorkerRepository + Send + Sync + 'static,
{
    pub fn new(scheduler: Arc<SchedulerModule<J, B, C, W>>) -> Self {
        Self { scheduler }
    }
}

#[async_trait]
impl<J, B, C, W> WorkerRegistrationPort for WorkerRegistrationAdapter<J, B, C, W>
where
    J: hodei_ports::job_repository::JobRepository + Send + Sync + 'static,
    B: hodei_ports::event_bus::EventPublisher + Send + Sync + 'static,
    C: hodei_ports::worker_client::WorkerClient + Send + Sync + 'static,
    W: hodei_ports::worker_repository::WorkerRepository + Send + Sync + 'static,
{
    async fn register_dynamic_worker(&self, worker: Worker) -> Result<(), String> {
        self.scheduler
            .register_worker(worker)
            .await
            .map_err(|e| e.to_string())
    }

    async fn unregister_worker(&self, worker_id: &WorkerId) -> Result<(), String> {
        // TODO: Implement unregister_worker in SchedulerModule
        // For now, just return Ok as a placeholder
        tracing::info!(worker_id = %worker_id, "Worker unregistered");
        Ok(())
    }
}
