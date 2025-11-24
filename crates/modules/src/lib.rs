//! Application Modules
//!
//! This crate contains the application layer (use cases) that orchestrates
//! the domain entities through the ports.

pub mod orchestrator;
pub mod cost_tracking;
pub mod queue_assignment;
pub mod queue_scaling_integration;
pub mod resource_pool;
pub mod scaling_policies;
pub mod scheduler;
pub mod worker_management;

pub use crate::orchestrator::{OrchestratorConfig, OrchestratorModule};
pub use crate::cost_tracking::{
    CostAlert, CostAlertType, CostAlerts, CostReportingPeriod, CostSummary, CostTrackingError,
    CostTrackingService, JobCost, WorkerCost,
};
pub use crate::queue_assignment::{
    AssignmentRequest, AssignmentResult, QueueAssignmentEngine, QueueError, QueuePriority,
    QueueType, SchedulingPolicy,
};
pub use crate::queue_scaling_integration::{
    QueueScalingConfig, QueueScalingError, QueueScalingEvent, QueueScalingIntegration,
    QueueScalingStats,
};
pub use crate::resource_pool::{
    ResourcePoolService, ResourcePoolServiceError, create_docker_resource_pool,
    create_kubernetes_resource_pool,
};
pub use crate::scaling_policies::{
    CooldownManager, CpuUtilizationScalingPolicy, CustomScalingPolicy, QueueDepthScalingPolicy,
    ScalingAction, ScalingContext, ScalingDecision, ScalingEngine, ScalingMetrics,
    ScalingPolicyEnum,
};
pub use crate::scheduler::state_machine::{
    SchedulingContext, SchedulingState, SchedulingStateMachine,
};
pub use crate::scheduler::{SchedulerConfig, SchedulerModule};
pub use crate::worker_management::{
    WorkerManagementConfig, WorkerManagementError, WorkerManagementService,
    create_default_worker_management_service, create_kubernetes_worker_management_service,
};
