//! Worker Registration Port
//!
//! This module defines the port for dynamically registering workers
//! in the scheduling system.

use async_trait::async_trait;
use hodei_core::{Worker, WorkerId};

/// Worker registration port
/// 
/// This trait allows WorkerManagementService to register dynamically
/// provisioned workers in the scheduling system without depending
/// on the concrete Scheduler implementation.
#[async_trait]
pub trait WorkerRegistrationPort: Send + Sync {
    /// Register a dynamically provisioned worker
    async fn register_dynamic_worker(&self, worker: Worker) -> Result<(), String>;

    /// Unregister a worker (e.g., when stopping/deleting)
    async fn unregister_worker(&self, worker_id: &WorkerId) -> Result<(), String>;
}
