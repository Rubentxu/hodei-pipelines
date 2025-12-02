use hodei_pipelines_core::{
    job_definitions::{JobSpec, ResourceQuota},
    pipeline::{Pipeline, PipelineStep},
    pipeline_execution::{PipelineExecution, StepExecution, StepExecutionStatus},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

// --- Common ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResourceQuotaDto {
    pub cpu_m: u64,
    pub memory_mb: u64,
    pub gpu: Option<u8>,
}

impl From<ResourceQuota> for ResourceQuotaDto {
    fn from(q: ResourceQuota) -> Self {
        Self {
            cpu_m: q.cpu_m,
            memory_mb: q.memory_mb,
            gpu: q.gpu,
        }
    }
}

impl From<ResourceQuotaDto> for ResourceQuota {
    fn from(dto: ResourceQuotaDto) -> Self {
        Self {
            cpu_m: dto.cpu_m,
            memory_mb: dto.memory_mb,
            gpu: dto.gpu,
        }
    }
}

// --- Pipeline ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub steps: Vec<PipelineStepDto>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineStepDto {
    pub id: Uuid,
    pub name: String,
    pub job_spec: JobSpecDto,
    pub dependencies: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobSpecDto {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub resources: ResourceQuotaDto,
    pub timeout_ms: u64,
    pub retries: u8,
    pub env: HashMap<String, String>,
    pub secret_refs: Vec<String>,
}

impl From<Pipeline> for PipelineDto {
    fn from(p: Pipeline) -> Self {
        Self {
            id: p.id.0,
            name: p.name,
            description: p.description,
            status: p.status.as_str().to_string(),
            steps: p.steps.into_iter().map(Into::into).collect(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineSummaryDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub step_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Pipeline> for PipelineSummaryDto {
    fn from(p: Pipeline) -> Self {
        Self {
            id: p.id.0,
            name: p.name,
            description: p.description,
            status: p.status.as_str().to_string(),
            step_count: p.steps.len(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

impl From<PipelineStep> for PipelineStepDto {
    fn from(s: PipelineStep) -> Self {
        Self {
            id: s.id.0,
            name: s.name,
            job_spec: s.job_spec.into(),
            dependencies: s.depends_on.into_iter().map(|id| id.0).collect(),
        }
    }
}

impl From<JobSpec> for JobSpecDto {
    fn from(s: JobSpec) -> Self {
        Self {
            name: s.name,
            image: s.image,
            command: s.command,
            resources: s.resources.into(),
            timeout_ms: s.timeout_ms,
            retries: s.retries,
            env: s.env,
            secret_refs: s.secret_refs,
        }
    }
}

impl From<JobSpecDto> for JobSpec {
    fn from(dto: JobSpecDto) -> Self {
        Self {
            name: dto.name,
            image: dto.image,
            command: dto.command,
            resources: dto.resources.into(),
            timeout_ms: dto.timeout_ms,
            retries: dto.retries,
            env: dto.env,
            secret_refs: dto.secret_refs,
        }
    }
}

// --- Execution ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineExecutionDto {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub steps: Vec<StepExecutionDto>,
    pub variables: HashMap<String, String>,
    pub tenant_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StepExecutionDto {
    pub step_execution_id: Uuid,
    pub step_id: Uuid,
    pub status: String,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u8,
    pub error_message: Option<String>,
    pub logs: Vec<String>,
}

impl From<PipelineExecution> for PipelineExecutionDto {
    fn from(e: PipelineExecution) -> Self {
        Self {
            id: e.id.0,
            pipeline_id: e.pipeline_id.0,
            status: e.status.as_str().to_string(),
            started_at: e.started_at,
            completed_at: e.completed_at,
            steps: e.steps.into_iter().map(Into::into).collect(),
            variables: e.variables,
            tenant_id: e.tenant_id,
            correlation_id: e.correlation_id,
        }
    }
}

impl From<StepExecution> for StepExecutionDto {
    fn from(s: StepExecution) -> Self {
        Self {
            step_execution_id: s.step_execution_id.0,
            step_id: s.step_id.0,
            status: s.status.as_str().to_string(),
            started_at: s.started_at,
            completed_at: s.completed_at,
            retry_count: s.retry_count,
            error_message: s.error_message,
            logs: s.logs,
        }
    }
}

// --- Create Requests ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePipelineRequestDto {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<CreatePipelineStepRequestDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePipelineStepRequestDto {
    pub name: String,
    pub job_spec: JobSpecDto,
    pub dependencies: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePipelineRequestDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub steps: Option<Vec<CreatePipelineStepRequestDto>>,
    pub variables: Option<HashMap<String, String>>,
}

// --- DAG Visualizer ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DagNodeDto {
    pub id: Uuid,
    pub name: String,
    pub status: Option<String>,
    pub position: Option<DagPositionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DagEdgeDto {
    pub source: Uuid,
    pub target: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DagPositionDto {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DagStructureDto {
    pub pipeline_id: Uuid,
    pub nodes: Vec<DagNodeDto>,
    pub edges: Vec<DagEdgeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StepDetailsDto {
    pub id: Uuid,
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub timeout_ms: u64,
    pub retries: u32,
    pub environment: HashMap<String, String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionLogsDto {
    pub execution_id: Uuid,
    pub step_executions: Vec<StepExecutionLogsDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StepExecutionLogsDto {
    pub step_id: Uuid,
    pub step_name: String,
    pub status: String,
    pub logs: Vec<LogEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogEntryDto {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stream_type: String,
    pub content: String,
}

// --- List Responses ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListPipelinesResponseDto {
    #[serde(rename = "items")]
    pub pipelines: Vec<PipelineSummaryDto>,
    pub total: usize,
}

impl From<Vec<Pipeline>> for ListPipelinesResponseDto {
    fn from(mut pipelines: Vec<Pipeline>) -> Self {
        let total = pipelines.len();
        let pipelines_summary = pipelines.drain(..).map(|p| p.into()).collect();
        Self {
            pipelines: pipelines_summary,
            total,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutePipelineRequestDto {
    pub environment: Option<String>,
    pub branch: Option<String>,
    pub parameters: Option<HashMap<String, String>>,
    pub variables: Option<HashMap<String, String>>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutePipelineResponseDto {
    pub execution_id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
}

impl From<PipelineExecution> for ExecutePipelineResponseDto {
    fn from(execution: PipelineExecution) -> Self {
        Self {
            execution_id: execution.id.0,
            pipeline_id: execution.pipeline_id.0,
            status: execution.status.as_str().to_string(),
        }
    }
}

// --- Execution Details ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionDetailsDto {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub current_step: Option<String>,
    pub stages: Vec<StageExecutionDto>,
    pub variables: HashMap<String, String>,
    pub tenant_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StageExecutionDto {
    pub step_id: String,
    pub step_name: String,
    pub status: StepExecutionStatusDto,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u8,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum StepExecutionStatusDto {
    PENDING,
    RUNNING,
    COMPLETED,
    FAILED,
    SKIPPED,
}

impl From<StepExecutionStatus> for StepExecutionStatusDto {
    fn from(status: StepExecutionStatus) -> Self {
        match status {
            StepExecutionStatus::PENDING => Self::PENDING,
            StepExecutionStatus::RUNNING => Self::RUNNING,
            StepExecutionStatus::COMPLETED => Self::COMPLETED,
            StepExecutionStatus::FAILED => Self::FAILED,
            StepExecutionStatus::SKIPPED => Self::SKIPPED,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionListItemDto {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub status: String,
    pub trigger: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionHistoryDto {
    pub executions: Vec<ExecutionListItemDto>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelExecutionResponseDto {
    pub id: Uuid,
    pub status: String,
    pub canceled_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryExecutionResponseDto {
    pub original_execution_id: Uuid,
    pub new_execution_id: Uuid,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// --- Resource Pools ---

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ResourcePoolTypeDto {
    Docker,
    Kubernetes,
    Cloud,
    Static,
}

impl From<hodei_pipelines_ports::resource_pool::ResourcePoolType> for ResourcePoolTypeDto {
    fn from(t: hodei_pipelines_ports::resource_pool::ResourcePoolType) -> Self {
        match t {
            hodei_pipelines_ports::resource_pool::ResourcePoolType::Docker => Self::Docker,
            hodei_pipelines_ports::resource_pool::ResourcePoolType::Kubernetes => Self::Kubernetes,
            hodei_pipelines_ports::resource_pool::ResourcePoolType::Cloud => Self::Cloud,
            hodei_pipelines_ports::resource_pool::ResourcePoolType::Static => Self::Static,
        }
    }
}

impl From<ResourcePoolTypeDto> for hodei_pipelines_ports::resource_pool::ResourcePoolType {
    fn from(t: ResourcePoolTypeDto) -> Self {
        match t {
            ResourcePoolTypeDto::Docker => Self::Docker,
            ResourcePoolTypeDto::Kubernetes => Self::Kubernetes,
            ResourcePoolTypeDto::Cloud => Self::Cloud,
            ResourcePoolTypeDto::Static => Self::Static,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResourcePoolConfigDto {
    pub pool_type: ResourcePoolTypeDto,
    pub name: String,
    pub provider_name: String,
    pub min_size: u32,
    pub max_size: u32,
    pub default_resources: ResourceQuotaDto,
    pub tags: HashMap<String, String>,
}

impl From<hodei_pipelines_ports::resource_pool::ResourcePoolConfig> for ResourcePoolConfigDto {
    fn from(c: hodei_pipelines_ports::resource_pool::ResourcePoolConfig) -> Self {
        Self {
            pool_type: c.pool_type.into(),
            name: c.name,
            provider_name: c.provider_name,
            min_size: c.min_size,
            max_size: c.max_size,
            default_resources: c.default_resources.into(),
            tags: c.tags,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResourcePoolStatusDto {
    pub name: String,
    pub pool_type: ResourcePoolTypeDto,
    pub total_capacity: u32,
    pub available_capacity: u32,
    pub active_workers: u32,
    pub pending_requests: u32,
}

impl From<hodei_pipelines_ports::resource_pool::ResourcePoolStatus> for ResourcePoolStatusDto {
    fn from(s: hodei_pipelines_ports::resource_pool::ResourcePoolStatus) -> Self {
        Self {
            name: s.name,
            pool_type: s.pool_type.into(),
            total_capacity: s.total_capacity,
            available_capacity: s.available_capacity,
            active_workers: s.active_workers,
            pending_requests: s.pending_requests,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePoolRequestDto {
    pub pool_type: ResourcePoolTypeDto,
    pub name: String,
    pub provider_name: String,
    pub min_size: u32,
    pub max_size: u32,
    pub default_resources: ResourceQuotaDto,
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePoolRequestDto {
    pub name: Option<String>,
    pub min_size: Option<u32>,
    pub max_size: Option<u32>,
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResourcePoolResponseDto {
    pub id: String,
    pub config: ResourcePoolConfigDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkerCapabilitiesDto {
    pub cpu_cores: u32,
    pub memory_gb: u64,
    pub gpu: Option<u8>,
    pub features: Vec<String>,
    pub labels: HashMap<String, String>,
    pub max_concurrent_jobs: u32,
}

impl From<hodei_pipelines_core::worker_messages::WorkerCapabilities> for WorkerCapabilitiesDto {
    fn from(c: hodei_pipelines_core::worker_messages::WorkerCapabilities) -> Self {
        Self {
            cpu_cores: c.cpu_cores,
            memory_gb: c.memory_gb,
            gpu: c.gpu,
            features: c.features,
            labels: c.labels,
            max_concurrent_jobs: c.max_concurrent_jobs,
        }
    }
}

impl From<WorkerCapabilitiesDto> for hodei_pipelines_core::worker_messages::WorkerCapabilities {
    fn from(c: WorkerCapabilitiesDto) -> Self {
        Self {
            cpu_cores: c.cpu_cores,
            memory_gb: c.memory_gb,
            gpu: c.gpu,
            features: c.features,
            labels: c.labels,
            max_concurrent_jobs: c.max_concurrent_jobs,
        }
    }
}
