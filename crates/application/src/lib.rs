//! Application Modules
//!
//! This crate contains the application layer (use cases) that orchestrates
//! the domain entities through the ports.

pub mod identity_access;
pub mod observability;
pub mod pipeline_execution;
pub mod resource_governance;
pub mod scheduling;

pub use crate::pipeline_execution::artifact_management::{
    ArtifactError, ArtifactId, ArtifactManagementConfig, ArtifactManagementService,
    ArtifactMetadata, ArtifactRepository, ChunkInfo, StorageBackend, StorageError, StorageProvider,
    UploadId, UploadStatus,
};
pub use crate::resource_governance::auto_scaling_engine::{
    AutoScalingPolicy, AutoScalingPolicyEngine, EvaluationContext, MetricsSnapshot,
    ScaleAction as AutoScalingAction, ScalingConstraints, ScalingDecision as AutoScalingDecision,
    ScalingStrategy, ScalingTrigger,
};
pub use crate::resource_governance::burst_capacity_manager::{
    BurstCapacityConfig, BurstCapacityManager, BurstDecision, BurstError, BurstResourceRequest,
    BurstSession, BurstStats, BurstStatus,
};
pub use crate::resource_governance::cooldown_management::{
    AdvancedCooldownManager, CooldownConfig, CooldownEvent, CooldownStats, CooldownType,
    OverrideConfig, OverrideReason,
};
pub use crate::resource_governance::cost_optimization::{
    CostAnalysisPeriod, CostBreakdown, CostEfficiencyMetrics, CostOptimizationEngine,
    CostOptimizationError, OptimizationRecommendation, OptimizationReport, UtilizationAnalysis,
};
pub use crate::resource_governance::cost_tracking::{
    CostAlert, CostAlertType, CostAlerts, CostReportingPeriod, CostSummary, CostTrackingError,
    CostTrackingService, JobCost, WorkerCost,
};

pub use crate::identity_access::multi_tenancy_quota_manager::{
    BillingTier, BurstPolicy, FairShareDecision, JobPriority, MultiTenancyQuotaManager, PoolQuota,
    QuotaDecision, QuotaError, QuotaLimits, QuotaManagerConfig, QuotaStats, QuotaType,
    QuotaViolationReason, ResourceRequest, TenantId, TenantQuota, TenantUsage,
};
pub use crate::identity_access::rbac::{
    CreatePermissionRequest, CreateRoleRequest, ListRolesResponse, RbacConfig, RbacError,
    RoleBasedAccessControlService, UpdateRoleRequest,
};
pub use crate::observability::metrics_collection::{
    AggregatedMetric, AggregationWindow, MetricType, MetricValue, MetricsCollector, MetricsConfig,
    MetricsError, RealTimeSnapshot,
};
pub use crate::observability::sla_tracking::{
    PriorityAdjustment, SLALevel, SLAMetricsSnapshot, SLAStats, SLATracker, SLAViolationAlert,
    SLAViolationEvent,
};
pub use crate::pipeline_execution::orchestrator::{OrchestratorConfig, OrchestratorModule};
pub use crate::pipeline_execution::pipeline_crud::{
    CreatePipelineRequest, CreatePipelineStepRequest, ExecutePipelineRequest, ListPipelinesFilter,
    ListPipelinesResponse, PipelineCrudConfig, PipelineCrudError, PipelineCrudService,
    PipelineSummary, UpdatePipelineRequest,
};
pub use crate::pipeline_execution::pipeline_execution_orchestrator::{
    ConcreteOrchestrator, PipelineExecutionConfig, PipelineExecutionOrchestrator,
    PipelineExecutionService, PipelineService,
};
pub use crate::resource_governance::pool_lifecycle::{
    HealthCheckResult, InMemoryStateStore, LifecycleError, PoolConfig, PoolEvent,
    PoolLifecycleState, PoolState, ResourcePoolLifecycleManager,
};
pub use crate::resource_governance::resource_pool::{
    ResourcePoolService, ResourcePoolServiceError, create_docker_resource_pool,
    create_kubernetes_resource_pool,
};
pub use crate::resource_governance::resource_pool_metrics_collector::{
    AggregatedPoolMetrics, CostMetrics, HealthMetrics, InMemoryMetricsStore, JobMetrics,
    MetricsError as ResourcePoolMetricsError, MetricsStore, PerPoolTenantMetrics, PerTenantMetrics,
    PoolMetrics, PoolSizeMetrics, PrometheusMetricsExporter, ResourcePoolMetricsCollector,
    TenantMetrics, WorkerStateMetrics,
};
pub use crate::resource_governance::scaling_policies::{
    CooldownManager, CpuUtilizationScalingPolicy, CustomScalingPolicy, QueueDepthScalingPolicy,
    ScalingAction, ScalingContext, ScalingDecision, ScalingEngine, ScalingMetrics,
    ScalingPolicyEnum,
};
pub use crate::resource_governance::scaling_triggers::{
    ComparisonOperator, CompositeOperator, CompositeTrigger, EventBasedTrigger, EventCondition,
    ScaleDirection, TimeBasedTrigger, Trigger, TriggerEvaluationConfig, TriggerEvaluationEngine,
    TriggerEvaluationResult, TriggerStats, TriggerType,
};
pub use crate::scheduling::queue_assignment::{
    AssignmentRequest, AssignmentResult, DeadLetterQueue, FIFOStandardQueue, QueueAssignmentEngine,
    QueueError, QueuePriority, QueueType, SchedulingPolicy,
};
pub use crate::scheduling::queue_prioritization::{
    FairShareAllocation, PreemptionCandidate, PrioritizationInfo, PrioritizationStats,
    PrioritizationStrategy,
};
pub use crate::scheduling::queue_scaling_integration::{
    QueueScalingConfig, QueueScalingError, QueueScalingEvent, QueueScalingIntegration,
    QueueScalingStats,
};
pub use crate::scheduling::state_machine::{
    SchedulingContext, SchedulingState, SchedulingStateMachine,
};
pub use crate::scheduling::{SchedulerConfig, SchedulerModule};
pub use crate::scheduling::weighted_fair_queuing::{
    StarvationDetection, TenantWeight, WFQAllocation, WFQConfig, WFQError, WFQQueueEntry, WFQStats,
    WeightContext, WeightStrategy, WeightedFairQueueingEngine,
};
pub use crate::scheduling::worker_management::{
    WorkerManagementConfig, WorkerManagementError, WorkerManagementService,
    create_default_worker_management_service, create_kubernetes_worker_management_service,
};
