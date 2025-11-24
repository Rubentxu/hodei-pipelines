//! Application Modules
//!
//! This crate contains the application layer (use cases) that orchestrates
//! the domain entities through the ports.

pub mod orchestrator;
pub mod resource_pool;
pub mod scheduler;
pub mod worker_management;

pub use crate::orchestrator::{OrchestratorConfig, OrchestratorModule};
pub use crate::resource_pool::{
    ResourcePoolService, ResourcePoolServiceError, create_docker_resource_pool,
    create_kubernetes_resource_pool,
};
pub use crate::scheduler::state_machine::{
    SchedulingContext, SchedulingState, SchedulingStateMachine,
};
pub use crate::scheduler::{SchedulerConfig, SchedulerModule};
pub use crate::worker_management::{
    WorkerManagementConfig, WorkerManagementError, WorkerManagementService,
    create_default_worker_management_service, create_kubernetes_worker_management_service,
};
