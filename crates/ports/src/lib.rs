//! Ports - Abstraction Layer
//!
//! This crate defines ports (traits) that represent the interfaces
//! needed by the application layer. These are implemented by adapters
//! in the infrastructure layer.

pub use hodei_pipelines_domain::WorkerStatus;

pub mod shared;
pub mod identity_access;
pub mod observability;
pub mod pipeline_execution;
pub mod resource_governance;
pub mod scheduling;

// Re-export event_bus from shared for backward compatibility
pub use crate::shared::event_bus::{EventBusError, EventPublisher, EventSubscriber, SystemEvent};
pub mod event_bus {
    pub use super::shared::event_bus::*;
}

// Re-export scheduling modules for backward compatibility
pub mod worker_provider {
    pub use super::scheduling::worker_provider::*;
}

pub mod scheduler_port {
    pub use super::scheduling::scheduler_port::*;
}

pub mod worker_template {
    pub use super::scheduling::worker_template::*;
}
pub use crate::identity_access::rbac_repository::{PermissionRepository, RoleRepository};
pub use crate::pipeline_execution::execution_repository::PipelineExecutionRepository;
pub use crate::pipeline_execution::job_repository::{JobFilter, JobRepository, JobRepositoryError};
#[cfg(feature = "testing")]
pub use crate::pipeline_execution::pipeline_repository::MockPipelineRepository;
pub use crate::pipeline_execution::pipeline_repository::{
    PipelineRepository, PipelineRepositoryError,
};
pub use crate::resource_governance::resource_pool::{
    AllocationStatus, PoolResourceAllocation, ResourceAllocationRequest, ResourcePool,
    ResourcePoolConfig, ResourcePoolRepository, ResourcePoolStatus, ResourcePoolType,
};
pub use crate::resource_governance::{
    BudgetPeriod, BudgetQuota, GovernanceError, PoolCapacity, PoolConfiguration, PoolQuery,
    PoolSelector, PoolStatus, RateLimit, ResourceAllocation, ResourcePoolInfo, ResourceQuota,
    ResourceType, SortPreference,
};
pub use crate::scheduling::scheduler_port::{SchedulerError, SchedulerPort};
pub use crate::scheduling::worker_client::{WorkerClient, WorkerClientError};
pub use crate::scheduling::worker_provider::{
    ProviderConfig, ProviderFactoryTrait, ProviderType, WorkerProvider,
};
pub use crate::scheduling::worker_registration::{WorkerRegistrationError, WorkerRegistrationPort};
pub use crate::scheduling::worker_repository::{WorkerRepository, WorkerRepositoryError};
pub use crate::scheduling::worker_template::{
    ContainerSpec, DockerTemplateGenerator, EnvVar, KubernetesTemplateGenerator, NetworkConfig,
    PortMapping, ResourceRequirements, SecurityContext, TemplateError, TemplateMetadata, ToolSpec,
    VolumeMount, WorkerTemplate, WorkerTemplateGenerator,
};
