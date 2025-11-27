//! Ports - Abstraction Layer
//!
//! This crate defines ports (traits) that represent the interfaces
//! needed by the application layer. These are implemented by adapters
//! in the infrastructure layer.

pub use hodei_core::WorkerStatus;

pub mod event_bus;
pub mod job_repository;
pub mod pipeline_repository;
pub mod rbac_repository;
pub mod resource_pool;
pub mod scheduler_port;
pub mod security;
pub mod worker_client;
pub mod worker_provider;
pub mod worker_registration;
pub mod worker_repository;

pub use crate::event_bus::{EventBusError, EventPublisher, EventSubscriber, SystemEvent};
pub use crate::job_repository::{JobRepository, JobRepositoryError};
pub use crate::pipeline_repository::{PipelineRepository, PipelineRepositoryError};
pub use crate::rbac_repository::{PermissionRepository, RoleRepository};
pub use crate::resource_pool::{
    AllocationStatus, ResourceAllocation, ResourceAllocationRequest, ResourcePool,
    ResourcePoolConfig, ResourcePoolStatus, ResourcePoolType,
};
pub use crate::scheduler_port::{SchedulerError, SchedulerPort};
pub use crate::worker_client::{WorkerClient, WorkerClientError};
pub use crate::worker_provider::{
    ProviderConfig, ProviderFactoryTrait, ProviderType, WorkerProvider,
};
pub use crate::worker_registration::{WorkerRegistrationError, WorkerRegistrationPort};
pub use crate::worker_repository::{WorkerRepository, WorkerRepositoryError};
