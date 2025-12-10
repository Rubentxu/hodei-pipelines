//! Pipeline Execution Integration Service
//!
//! This service integrates WorkerProvisioner, PipelineStepExecutor, and ToolManager
//! to provide a complete pipeline execution flow with ephemeral workers.

use async_trait::async_trait;
use hodei_pipelines_domain::{
    DomainError, ExecutionId, Pipeline, PipelineExecution, PipelineId, Result,
};
use hodei_pipelines_ports::{
    EventPublisher, JobRepository, PipelineExecutionRepository, PipelineRepository,
    scheduling::worker_provisioner::{
        ProvisionedWorker, WorkerAllocationRequest, WorkerProvisioner,
    },
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{error, info, warn};

use super::{
    ephemeral_worker_handle::ProvisionedWorkerHandleBuilder,
    pipeline_step_executor::{PipelineStepExecutor, ToolManager},
};

/// Configuration for pipeline execution integration
#[derive(Debug, Clone)]
pub struct PipelineExecutionIntegrationConfig {
    /// Base directory for workspace creation
    pub workspace_base_dir: PathBuf,

    /// Whether to install tools automatically
    pub auto_install_tools: bool,

    /// Whether to clean up workspace after execution
    pub cleanup_workspace: bool,

    /// Maximum concurrent pipeline executions
    pub max_concurrent_executions: usize,
}

impl Default for PipelineExecutionIntegrationConfig {
    fn default() -> Self {
        Self {
            workspace_base_dir: PathBuf::from("/tmp/hodei-workspaces"),
            auto_install_tools: true,
            cleanup_workspace: false,
            max_concurrent_executions: 10,
        }
    }
}

/// Pipeline Execution Integration Service
pub struct PipelineExecutionIntegrationService<W, T, R, J, P, E>
where
    W: WorkerProvisioner + Send + Sync + 'static,
    T: ToolManager + Send + Sync + 'static,
    R: PipelineExecutionRepository + Send + Sync + 'static,
    J: JobRepository + Send + Sync + 'static,
    P: PipelineRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
{
    worker_provisioner: Arc<W>,
    tool_manager: Arc<T>,
    execution_repo: Arc<R>,
    #[allow(dead_code)]
    job_repo: Arc<J>,
    pipeline_repo: Arc<P>,
    event_bus: Arc<E>,
    config: PipelineExecutionIntegrationConfig,
}

impl<W, T, R, J, P, E> PipelineExecutionIntegrationService<W, T, R, J, P, E>
where
    W: WorkerProvisioner + Send + Sync + 'static,
    T: ToolManager + Send + Sync + 'static,
    R: PipelineExecutionRepository + Send + Sync + 'static,
    J: JobRepository + Send + Sync + 'static,
    P: PipelineRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
{
    /// Create a new integration service
    pub fn new(
        worker_provisioner: Arc<W>,
        tool_manager: Arc<T>,
        execution_repo: Arc<R>,
        job_repo: Arc<J>,
        pipeline_repo: Arc<P>,
        event_bus: Arc<E>,
        config: PipelineExecutionIntegrationConfig,
    ) -> Self {
        Self {
            worker_provisioner,
            tool_manager,
            execution_repo,
            job_repo,
            pipeline_repo,
            event_bus,
            config,
        }
    }

    /// Execute a complete pipeline with ephemeral worker provisioning
    pub async fn execute_pipeline_with_ephemeral_worker(
        &self,
        pipeline_id: PipelineId,
        variables: HashMap<String, String>,
        tenant_id: Option<String>,
        correlation_id: Option<String>,
        priority: u8,
    ) -> Result<ExecutionId> {
        info!("Starting pipeline execution with ephemeral worker: {}", pipeline_id);

        // 1. Get pipeline from repository
        let pipeline = self
            .pipeline_repo
            .get_pipeline(&pipeline_id)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get pipeline: {}", e)))?
            .ok_or_else(|| DomainError::NotFound(format!("Pipeline not found: {}", pipeline_id)))?;

        // 2. Calculate resource requirements
        let resource_requirements = self
            .worker_provisioner
            .calculate_resource_requirements(&pipeline)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to calculate resources: {}", e)))?;

        // 3. Create worker allocation request
        let allocation_request = WorkerAllocationRequest {
            pipeline: pipeline.clone(),
            resource_requirements: resource_requirements.clone(),
            tenant_id: tenant_id.clone(),
            priority,
            required_labels: vec![], // Can be extended based on pipeline requirements
            preferred_labels: vec![],
        };

        // 4. Provision ephemeral worker
        let provisioned_worker = self
            .worker_provisioner
            .provision_worker_for_pipeline(allocation_request)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to provision worker: {}", e)))?;

        info!(
            "Worker provisioned for pipeline {}: {}",
            pipeline_id, provisioned_worker.worker_id
        );

        // 5. Create pipeline execution record
        let step_ids: Vec<_> = pipeline.steps.iter().map(|step| step.id.clone()).collect();
        let execution = PipelineExecution::new(
            pipeline_id.clone(),
            step_ids,
            variables,
            tenant_id,
            correlation_id,
        );

        let execution_id = execution.id.clone();

        // 6. Save initial execution state
        self.execution_repo
            .save_execution(&execution)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to save execution: {}", e)))?;

        // 7. Start async execution
        let worker_provisioner = self.worker_provisioner.clone();
        let tool_manager = self.tool_manager.clone();
        let execution_repo = self.execution_repo.clone();
        let event_bus = self.event_bus.clone();
        let config = self.config.clone();
        let pipeline_clone = pipeline.clone();
        let provisioned_worker_arc = Arc::new(provisioned_worker);

        tokio::spawn(async move {
            if let Err(e) = Self::execute_pipeline_in_worker(
                worker_provisioner,
                tool_manager,
                execution_repo,
                event_bus,
                config,
                pipeline_clone,
                provisioned_worker_arc,
                execution_id,
            )
            .await
            {
                error!("Pipeline execution failed: {}", e);
            }
        });

        info!(
            "Pipeline execution started: {} (execution: {})",
            pipeline_id, execution_id
        );

        Ok(execution_id)
    }

    /// Internal method to execute pipeline in provisioned worker
    async fn execute_pipeline_in_worker<W2, T2, R2, E2>(
        _worker_provisioner: Arc<W2>,
        tool_manager: Arc<T2>,
        execution_repo: Arc<R2>,
        event_bus: Arc<E2>,
        config: PipelineExecutionIntegrationConfig,
        pipeline: Pipeline,
        provisioned_worker: Arc<ProvisionedWorker>,
        execution_id: ExecutionId,
    ) -> Result<()>
    where
        W2: WorkerProvisioner + Send + Sync + 'static,
        T2: ToolManager + Send + Sync + 'static,
        R2: PipelineExecutionRepository + Send + Sync + 'static,
        E2: EventPublisher + Send + Sync + 'static,
    {
        info!(
            "Starting pipeline execution in worker: {} (execution: {})",
            provisioned_worker.worker_id, execution_id
        );

        // 1. Get execution from repository
        let mut execution = execution_repo
            .get_execution(&execution_id)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get execution: {}", e)))?
            .ok_or_else(|| DomainError::NotFound(format!("Execution not found: {}", execution_id)))?;

        // 2. Create workspace directory
        let workspace_dir = config.workspace_base_dir.join(format!("execution-{}", execution_id));
        fs::create_dir_all(&workspace_dir)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to create workspace: {}", e)))?;

        // 3. Create ephemeral worker handle
        let worker_handle = ProvisionedWorkerHandleBuilder::new(provisioned_worker.clone())
            .with_tool_manager(tool_manager.clone())
            .build();

        // 4. Create pipeline step executor
        let step_executor = PipelineStepExecutor::new(
            Arc::new(worker_handle),
            tool_manager,
            workspace_dir.clone(),
        );

        // 5. Execute pipeline
        let execution_result = step_executor
            .execute_pipeline(&pipeline, &mut execution)
            .await?;

        // 6. Update execution state
        execution_repo
            .save_execution(&execution)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to save execution result: {}", e)))?;

        // 7. Publish completion event
        // Note: Currently only PipelineExecutionStarted is available in SystemEvent
        // We'll publish a PipelineCompleted event instead
        let event = if execution_result.success {
            hodei_pipelines_ports::SystemEvent::PipelineCompleted {
                pipeline_id: pipeline.id,
            }
        } else {
            hodei_pipelines_ports::SystemEvent::JobFailed {
                job_id: hodei_pipelines_domain::JobId::new(),
                error: format!("Pipeline failed at step {:?}", execution_result.failed_step),
            }
        };

        event_bus
            .publish(event)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to publish event: {}", e)))?;

        // 8. Cleanup workspace if configured
        if config.cleanup_workspace {
            if let Err(e) = fs::remove_dir_all(&workspace_dir).await {
                warn!("Failed to cleanup workspace: {}", e);
            }
        }

        info!(
            "Pipeline execution completed: {} (success: {})",
            execution_id, execution_result.success
        );

        Ok(())
    }

    /// Get execution status
    pub async fn get_execution_status(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<PipelineExecution>> {
        self.execution_repo
            .get_execution(execution_id)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get execution: {}", e)))
    }

    /// Cancel a running pipeline execution
    pub async fn cancel_execution(&self, execution_id: &ExecutionId) -> Result<()> {
        info!("Cancelling pipeline execution: {}", execution_id);

        // TODO: Implement cancellation logic
        // This would need to signal the running execution to stop
        // and cleanup the provisioned worker

        warn!("Execution cancellation not yet implemented for: {}", execution_id);
        Ok(())
    }
}

/// Simplified API trait for pipeline execution
#[async_trait]
pub trait PipelineExecutionService: Send + Sync {
    /// Execute a pipeline with ephemeral worker
    async fn execute_pipeline(
        &self,
        pipeline_id: PipelineId,
        variables: HashMap<String, String>,
        tenant_id: Option<String>,
        correlation_id: Option<String>,
    ) -> Result<ExecutionId>;

    /// Get execution status
    async fn get_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<PipelineExecution>>;

    /// Cancel execution
    async fn cancel_execution(&self, execution_id: &ExecutionId) -> Result<()>;
}

#[async_trait]
impl<W, T, R, J, P, E> PipelineExecutionService
    for PipelineExecutionIntegrationService<W, T, R, J, P, E>
where
    W: WorkerProvisioner + Send + Sync + 'static,
    T: ToolManager + Send + Sync + 'static,
    R: PipelineExecutionRepository + Send + Sync + 'static,
    J: JobRepository + Send + Sync + 'static,
    P: PipelineRepository + Send + Sync + 'static,
    E: EventPublisher + Send + Sync + 'static,
{
    async fn execute_pipeline(
        &self,
        pipeline_id: PipelineId,
        variables: HashMap<String, String>,
        tenant_id: Option<String>,
        correlation_id: Option<String>,
    ) -> Result<ExecutionId> {
        self.execute_pipeline_with_ephemeral_worker(
            pipeline_id,
            variables,
            tenant_id,
            correlation_id,
            5, // Default priority
        )
        .await
    }

    async fn get_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<PipelineExecution>> {
        self.get_execution_status(execution_id).await
    }

    async fn cancel_execution(&self, execution_id: &ExecutionId) -> Result<()> {
        self.cancel_execution(execution_id).await
    }
}