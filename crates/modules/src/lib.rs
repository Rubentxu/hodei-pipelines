//! Application Modules
//!
//! This crate contains the application layer (use cases) that orchestrates
//! the domain entities through the ports.

pub mod artifact_management;
pub mod auto_scaling_engine;
pub mod burst_capacity_manager;
pub mod cooldown_management;
pub mod cost_optimization;
pub mod cost_tracking;
pub mod metrics_collection;
pub mod multi_tenancy_quota_manager;
pub mod orchestrator;
pub mod pipeline_crud;
pub mod pool_lifecycle;
pub mod queue_assignment;
pub mod queue_prioritization;
pub mod queue_scaling_integration;
pub mod quota_enforcement;
pub mod rbac;
pub mod resource_pool;
pub mod resource_pool_metrics_collector;
pub mod scaling_policies;
pub mod scaling_triggers;
pub mod scheduler;
pub mod sla_tracking;
pub mod weighted_fair_queuing;
pub mod worker_management;

pub use crate::artifact_management::{
    ArtifactError, ArtifactId, ArtifactManagementConfig, ArtifactManagementService,
    ArtifactMetadata, ArtifactRepository, ChunkInfo, StorageBackend, StorageError, StorageProvider,
    UploadId, UploadStatus,
};
pub use crate::auto_scaling_engine::{
    AutoScalingPolicy, AutoScalingPolicyEngine, EvaluationContext, MetricsSnapshot,
    ScaleAction as AutoScalingAction, ScalingConstraints, ScalingDecision as AutoScalingDecision,
    ScalingStrategy, ScalingTrigger,
};
pub use crate::burst_capacity_manager::{
    BurstCapacityConfig, BurstCapacityManager, BurstDecision, BurstError, BurstResourceRequest,
    BurstSession, BurstStats, BurstStatus,
};
pub use crate::cooldown_management::{
    AdvancedCooldownManager, CooldownConfig, CooldownEvent, CooldownStats, CooldownType,
    OverrideConfig, OverrideReason,
};
pub use crate::cost_optimization::{
    CostAnalysisPeriod, CostBreakdown, CostEfficiencyMetrics, CostOptimizationEngine,
    CostOptimizationError, OptimizationRecommendation, OptimizationReport, UtilizationAnalysis,
};
pub use crate::cost_tracking::{
    CostAlert, CostAlertType, CostAlerts, CostReportingPeriod, CostSummary, CostTrackingError,
    CostTrackingService, JobCost, WorkerCost,
};
pub use crate::metrics_collection::{
    AggregatedMetric, AggregationWindow, MetricType, MetricValue, MetricsCollector, MetricsConfig,
    MetricsError, RealTimeSnapshot,
};
pub use crate::multi_tenancy_quota_manager::{
    BillingTier, BurstPolicy, FairShareDecision, JobPriority, MultiTenancyQuotaManager, PoolQuota,
    QuotaDecision, QuotaError, QuotaLimits, QuotaManagerConfig, QuotaStats, QuotaType,
    QuotaViolationReason, ResourceRequest, TenantId, TenantQuota, TenantUsage,
};
pub use crate::orchestrator::{OrchestratorConfig, OrchestratorModule};
pub use crate::pipeline_crud::{
    CreatePipelineRequest, CreatePipelineStepRequest, ListPipelinesFilter, ListPipelinesResponse,
    PipelineCrudConfig, PipelineCrudError, PipelineCrudService, PipelineSummary,
    UpdatePipelineRequest,
};
pub use crate::pool_lifecycle::{
    HealthCheckResult, InMemoryStateStore, LifecycleError, PoolConfig, PoolEvent,
    PoolLifecycleState, PoolState, ResourcePoolLifecycleManager,
};
pub use crate::queue_assignment::{
    AssignmentRequest, AssignmentResult, DeadLetterQueue, FIFOStandardQueue, QueueAssignmentEngine,
    QueueError, QueuePriority, QueueType, SchedulingPolicy,
};
pub use crate::queue_prioritization::{
    FairShareAllocation, PreemptionCandidate, PrioritizationInfo, PrioritizationStats,
    PrioritizationStrategy,
};
pub use crate::queue_scaling_integration::{
    QueueScalingConfig, QueueScalingError, QueueScalingEvent, QueueScalingIntegration,
    QueueScalingStats,
};
pub use crate::rbac::{
    CreatePermissionRequest, CreateRoleRequest, ListRolesResponse, RbacConfig, RbacError,
    RoleBasedAccessControlService, UpdateRoleRequest,
};
pub use crate::resource_pool::{
    ResourcePoolService, ResourcePoolServiceError, create_docker_resource_pool,
    create_kubernetes_resource_pool,
};
pub use crate::resource_pool_metrics_collector::{
    AggregatedPoolMetrics, CostMetrics, HealthMetrics, InMemoryMetricsStore, JobMetrics,
    MetricsError as ResourcePoolMetricsError, MetricsStore, PerPoolTenantMetrics, PerTenantMetrics,
    PoolMetrics, PoolSizeMetrics, PrometheusMetricsExporter, ResourcePoolMetricsCollector,
    TenantMetrics, WorkerStateMetrics,
};
pub use crate::scaling_policies::{
    CooldownManager, CpuUtilizationScalingPolicy, CustomScalingPolicy, QueueDepthScalingPolicy,
    ScalingAction, ScalingContext, ScalingDecision, ScalingEngine, ScalingMetrics,
    ScalingPolicyEnum,
};
pub use crate::scaling_triggers::{
    ComparisonOperator, CompositeOperator, CompositeTrigger, EventBasedTrigger, EventCondition,
    ScaleDirection, TimeBasedTrigger, Trigger, TriggerEvaluationConfig, TriggerEvaluationEngine,
    TriggerEvaluationResult, TriggerStats, TriggerType,
};
pub use crate::scheduler::state_machine::{
    SchedulingContext, SchedulingState, SchedulingStateMachine,
};
pub use crate::scheduler::{SchedulerConfig, SchedulerModule};
pub use crate::sla_tracking::{
    PriorityAdjustment, SLALevel, SLAMetricsSnapshot, SLAStats, SLATracker, SLAViolationAlert,
    SLAViolationEvent,
};
pub use crate::weighted_fair_queuing::{
    StarvationDetection, TenantWeight, WFQAllocation, WFQConfig, WFQError, WFQQueueEntry, WFQStats,
    WeightContext, WeightStrategy, WeightedFairQueueingEngine,
};
pub use crate::worker_management::{
    WorkerManagementConfig, WorkerManagementError, WorkerManagementService,
    create_default_worker_management_service, create_kubernetes_worker_management_service,
};
