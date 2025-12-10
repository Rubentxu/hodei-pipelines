//! Pipeline Step Executor Module
//!
//! This module provides the PipelineStepExecutor that executes pipeline steps
//! sequentially in the same ephemeral worker with a shared workspace.

use async_trait::async_trait;
use hodei_pipelines_domain::{
    DomainError, ExecutionId, Pipeline, PipelineExecution, Result, WorkerId,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tracing::{error, info, warn};

/// Tool Manager interface for installing and managing tools in workers
#[async_trait]
pub trait ToolManager: Send + Sync {
    /// Install required tools for a pipeline
    async fn install_tools(
        &self,
        workspace_path: &Path,
        tools: &[ToolRequirement],
    ) -> Result<()>;

    /// Check if a tool is already installed
    async fn is_tool_installed(
        &self,
        workspace_path: &Path,
        tool: &ToolRequirement,
    ) -> Result<bool>;
}

/// Tool requirement specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRequirement {
    pub name: String,
    pub version: String,
    pub plugin: Option<String>,
}

/// Execution error types
#[derive(thiserror::Error, Debug)]
pub enum ExecutionError {
    #[error("Workspace preparation failed: {0}")]
    WorkspacePreparation(String),

    #[error("Tool installation failed: {0}")]
    ToolInstallation(String),

    #[error("Step execution failed: {0}")]
    StepExecution(String),

    #[error("Worker communication failed: {0}")]
    WorkerCommunication(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

/// Result of a single step execution
#[derive(Debug, Clone)]
pub struct StepExecutionResult {
    pub step_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub logs: Vec<String>,
}

/// Result of a complete pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineExecutionResult {
    pub execution_id: ExecutionId,
    pub success: bool,
    pub steps_results: Vec<StepExecutionResult>,
    pub failed_step: Option<usize>,
    pub total_duration_ms: u64,
    pub workspace_path: Option<PathBuf>,
}

/// Ephemeral worker handle for executing steps
#[async_trait]
pub trait EphemeralWorkerHandle: Send + Sync {
    /// Execute a command in the worker
    async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&Path>,
        env_vars: Option<&[(String, String)]>,
    ) -> Result<StepExecutionResult>;

    /// Get worker ID
    fn worker_id(&self) -> &WorkerId;

    /// Check if worker is healthy
    async fn is_healthy(&self) -> Result<bool>;
}

/// Pipeline Step Executor
#[derive(Debug)]
pub struct PipelineStepExecutor<W, T> {
    worker_handle: Arc<W>,
    tool_manager: Arc<T>,
    base_workspace_dir: PathBuf,
}

impl<W, T> PipelineStepExecutor<W, T>
where
    W: EphemeralWorkerHandle + Send + Sync,
    T: ToolManager + Send + Sync,
{
    /// Create a new pipeline step executor
    pub fn new(worker_handle: Arc<W>, tool_manager: Arc<T>, base_workspace_dir: PathBuf) -> Self {
        Self {
            worker_handle,
            tool_manager,
            base_workspace_dir,
        }
    }

    /// Execute a complete pipeline with all its steps
    pub async fn execute_pipeline(
        &self,
        pipeline: &Pipeline,
        execution: &mut PipelineExecution,
    ) -> Result<PipelineExecutionResult> {
        info!(
            execution_id = %execution.id,
            pipeline_id = %execution.pipeline_id,
            steps_count = %pipeline.steps.len(),
            "Starting pipeline execution"
        );

        let start_time = std::time::Instant::now();

        // 1. Prepare workspace
        let workspace_path = self.prepare_workspace(execution).await?;

        // 2. Install required tools
        let required_tools = self.extract_required_tools(pipeline);
        if !required_tools.is_empty() {
            self.install_required_tools(&workspace_path, &required_tools)
                .await?;
        }

        // Start pipeline execution
        execution.start().map_err(ExecutionError::Domain)?;

        let mut steps_results = Vec::new();

        // 3. Execute steps sequentially in topolical order
        for (index, step) in pipeline.steps.iter().enumerate() {
            info!(
                execution_id = %execution.id,
                step_id = %step.id,
                step_index = index,
                "Starting step execution"
            );

            // Update step status to running
            if let Some(step_execution) = execution.get_step_execution_mut(&step.id) {
                step_execution.start();
            }

            let step_start = std::time::Instant::now();

            // Execute step
            let step_result = self
                .execute_step(&workspace_path, step, execution, index)
                .await?;

            steps_results.push(step_result);

            let step_duration = step_start.elapsed().as_millis() as u64;
            info!(
                execution_id = %execution.id,
                step_id = %step.id,
                step_index = index,
                duration_ms = step_duration,
                "Step execution completed"
            );

            // If step failed, fail the pipeline
            if let Some(last_result) = steps_results.last() {
                if !last_result.success {
                    // Mark step as failed in execution
                    if let Some(step_execution) = execution.get_step_execution_mut(&step.id) {
                        step_execution.fail(format!(
                            "Step execution failed with exit code: {:?}",
                            last_result.exit_code
                        ));
                    }

                    execution.fail().map_err(ExecutionError::Domain)?;

                    let total_duration = start_time.elapsed().as_millis() as u64;

                    return Ok(PipelineExecutionResult {
                        execution_id: execution.id,
                        success: false,
                        steps_results,
                        failed_step: Some(index),
                        total_duration_ms: total_duration,
                        workspace_path: Some(workspace_path),
                    });
                }
            }

            // Mark step as completed
            if let Some(step_execution) = execution.get_step_execution_mut(&step.id) {
                step_execution.complete();
            }
        }

        // Complete pipeline execution
        execution.complete().map_err(ExecutionError::Domain)?;

        let total_duration = start_time.elapsed().as_millis() as u64;

        info!(
            execution_id = %execution.id,
            total_duration_ms = total_duration,
            "Pipeline execution completed successfully"
        );

        Ok(PipelineExecutionResult {
            execution_id: execution.id,
            success: true,
            steps_results,
            failed_step: None,
            total_duration_ms: total_duration,
            workspace_path: Some(workspace_path),
        })
    }

    /// Prepare workspace for execution
    async fn prepare_workspace(
        &self,
        execution: &PipelineExecution,
    ) -> Result<PathBuf> {
        let workspace_path = self
            .base_workspace_dir
            .join(format!("execution-{}", execution.id));

        info!(
            execution_id = %execution.id,
            workspace_path = %workspace_path.display(),
            "Preparing workspace"
        );

        // Create workspace directory
        fs::create_dir_all(&workspace_path)
            .await
            .map_err(|e| ExecutionError::WorkspacePreparation(e.to_string()))?;

        // Create subdirectories
        let subdirs = ["logs", "artifacts", "cache"];
        for subdir in &subdirs {
            let subdir_path = workspace_path.join(subdir);
            fs::create_dir_all(&subdir_path)
                .await
                .map_err(|e| ExecutionError::WorkspacePreparation(e.to_string()))?;
        }

        // Write execution metadata
        let metadata = format!(
            "execution_id: {}\npipeline_id: {}\nstarted_at: {}\n",
            execution.id, execution.pipeline_id, execution.started_at
        );
        fs::write(workspace_path.join("metadata.txt"), metadata)
            .await
            .map_err(|e| ExecutionError::WorkspacePreparation(e.to_string()))?;

        info!(
            workspace_path = %workspace_path.display(),
            "Workspace prepared successfully"
        );

        Ok(workspace_path)
    }

    /// Extract required tools from pipeline
    fn extract_required_tools(&self, _pipeline: &Pipeline) -> Vec<ToolRequirement> {
        // TODO: Implement tool extraction from pipeline steps
        // Currently PipelineStep and JobSpec don't have a 'tools' field
        // This needs to be implemented when tool configuration is added to the domain
        Vec::new()
    }

    /// Install required tools in workspace
    async fn install_required_tools(
        &self,
        workspace_path: &Path,
        tools: &[ToolRequirement],
    ) -> Result<()> {
        info!(
            tools_count = %tools.len(),
            workspace_path = %workspace_path.display(),
            "Installing required tools"
        );

        for tool in tools {
            // Check if already installed
            match self
                .tool_manager
                .is_tool_installed(workspace_path, tool)
                .await
            {
                Ok(true) => {
                    info!(
                        tool_name = %tool.name,
                        tool_version = %tool.version,
                        "Tool already installed, skipping"
                    );
                    continue;
                }
                Ok(false) => {
                    // Proceed with installation
                }
                Err(e) => {
                    warn!(
                        tool_name = %tool.name,
                        error = %e,
                        "Failed to check tool installation status"
                    );
                }
            }

            // Install tool
            self.tool_manager
                .install_tools(workspace_path, &[tool.clone()])
                .await
                .map_err(|e| ExecutionError::ToolInstallation(e.to_string()))?;

            info!(
                tool_name = %tool.name,
                tool_version = %tool.version,
                "Tool installed successfully"
            );
        }

        Ok(())
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        workspace_path: &Path,
        step: &hodei_pipelines_domain::pipeline_execution::entities::pipeline::PipelineStep,
        execution: &mut PipelineExecution,
        step_index: usize,
    ) -> Result<StepExecutionResult> {
        // Build command from job_spec
        // job_spec.command is a Vec<String>, we need to extract the first element as command
        // and the rest as args
        let command_vec = step.job_spec.command.clone();
        if command_vec.is_empty() {
            return Err(DomainError::Validation("Command cannot be empty".to_string()));
        }
        let command = command_vec[0].clone();
        let args = command_vec[1..].to_vec();
        let working_dir = workspace_path.to_path_buf();

        // Prepare environment variables
        let mut env_vars = Vec::new();

        // Add pipeline variables
        for (key, value) in &execution.variables {
            env_vars.push((key.clone(), value.clone()));
        }

        // Add step-specific environment variables from job_spec
        for (key, value) in &step.job_spec.env {
            env_vars.push((key.clone(), value.clone()));
        }

        // Execute command in worker
        let step_result = self
            .worker_handle
            .execute_command(&command, &args, Some(&working_dir), Some(&env_vars))
            .await
            .map_err(|e| ExecutionError::StepExecution(e.to_string()))?;

        // Write step logs
        let log_file = workspace_path
            .join("logs")
            .join(format!("step-{}.log", step_index));
        let logs_content = step_result.logs.join("\n");
        fs::write(&log_file, logs_content)
            .await
            .map_err(|e| ExecutionError::StepExecution(e.to_string()))?;

        Ok(step_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_pipelines_domain::pipeline_execution::entities::pipeline::{Pipeline, PipelineId, PipelineStep, PipelineStepId};
    use hodei_pipelines_domain::pipeline_execution::job_definitions;
    use std::collections::HashMap;
    use tempfile::TempDir;

    // Mock implementations for testing
    struct MockWorkerHandle {
        worker_id: WorkerId,
    }

    #[async_trait]
    impl EphemeralWorkerHandle for MockWorkerHandle {
        async fn execute_command(
            &self,
            _command: &str,
            _args: &[String],
            _working_dir: Option<&Path>,
            _env_vars: Option<&[(String, String)]>,
        ) -> Result<StepExecutionResult> {
            Ok(StepExecutionResult {
                step_id: self.worker_id.to_string(),
                success: true,
                exit_code: Some(0),
                duration_ms: 100,
                logs: vec!["Step executed successfully".to_string()],
            })
        }

        fn worker_id(&self) -> &WorkerId {
            &self.worker_id
        }

        async fn is_healthy(&self) -> Result<bool> {
            Ok(true)
        }
    }

    struct MockToolManager;

    #[async_trait]
    impl ToolManager for MockToolManager {
        async fn install_tools(
            &self,
            _workspace_path: &Path,
            _tools: &[ToolRequirement],
        ) -> Result<()> {
            Ok(())
        }

        async fn is_tool_installed(
            &self,
            _workspace_path: &Path,
            _tool: &ToolRequirement,
        ) -> Result<bool> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn test_execute_pipeline_success() {
        // Create test pipeline
        let pipeline_id = PipelineId::new();
        let steps = vec![
            PipelineStep {
                id: PipelineStepId::new(),
                name: "step1".to_string(),
                job_spec: job_definitions::JobSpec {
                    name: "step1".to_string(),
                    image: "alpine".to_string(),
                    command: vec!["echo".to_string(), "Hello".to_string()],
                    resources: job_definitions::ResourceQuota {
                        cpu_m: 1000,
                        memory_mb: 1024,
                        gpu: None,
                    },
                    timeout_ms: 1000,
                    retries: 0,
                    env: HashMap::new(),
                    secret_refs: vec![],
                },
                depends_on: vec![],
                timeout_ms: 1000,
            },
            PipelineStep {
                id: PipelineStepId::new(),
                name: "step2".to_string(),
                job_spec: job_definitions::JobSpec {
                    name: "step2".to_string(),
                    image: "alpine".to_string(),
                    command: vec!["echo".to_string(), "World".to_string()],
                    resources: job_definitions::ResourceQuota {
                        cpu_m: 1000,
                        memory_mb: 1024,
                        gpu: None,
                    },
                    timeout_ms: 1000,
                    retries: 0,
                    env: HashMap::new(),
                    secret_refs: vec![],
                },
                depends_on: vec![],
                timeout_ms: 1000,
            },
        ];

        let pipeline = Pipeline::new(pipeline_id.clone(), "test-pipeline".to_string(), steps).unwrap();

        // Create execution
        let mut execution = PipelineExecution::new(
            pipeline_id,
            pipeline.steps.iter().map(|s| s.id.clone()).collect(),
            HashMap::new(),
            None,
            None,
        );

        // Create executor
        let temp_dir = TempDir::new().unwrap();
        let base_workspace = temp_dir.path().to_path_buf();

        let worker_handle = Arc::new(MockWorkerHandle {
            worker_id: WorkerId::new(),
        });
        let tool_manager = Arc::new(MockToolManager);

        let executor = PipelineStepExecutor::new(worker_handle, tool_manager, base_workspace);

        // Execute pipeline
        let result = executor
            .execute_pipeline(&pipeline, &mut execution)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.steps_results.len(), 2);
        assert!(result.failed_step.is_none());
        assert!(execution.is_completed());
    }

    #[tokio::test]
    async fn test_prepare_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let base_workspace = temp_dir.path().to_path_buf();

        let worker_handle = Arc::new(MockWorkerHandle {
            worker_id: WorkerId::new(),
        });
        let tool_manager = Arc::new(MockToolManager);

        let executor = PipelineStepExecutor::new(worker_handle, tool_manager, base_workspace);

        let _execution_id = ExecutionId::new();
        let pipeline_id = PipelineId::new();

        let execution = PipelineExecution::new(pipeline_id, vec![], HashMap::new(), None, None);

        let workspace_path = executor.prepare_workspace(&execution).await.unwrap();

        // Check workspace was created
        assert!(workspace_path.exists());
        assert!(workspace_path.join("logs").exists());
        assert!(workspace_path.join("artifacts").exists());
        assert!(workspace_path.join("cache").exists());
        assert!(workspace_path.join("metadata.txt").exists());
    }

    #[test]
    fn test_extract_required_tools() {
        // TODO: Update test when tool configuration is added to PipelineStep
        // Currently PipelineStep doesn't have a 'tools' field
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStep {
            id: PipelineStepId::new(),
            name: "step1".to_string(),
            job_spec: job_definitions::JobSpec {
                name: "test".to_string(),
                image: "alpine".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
                resources: job_definitions::ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 1024,
                    gpu: None,
                },
                timeout_ms: 1000,
                retries: 0,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            depends_on: vec![],
            timeout_ms: 1000,
        }];

        let pipeline = Pipeline::new(pipeline_id, "test-pipeline".to_string(), steps).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let base_workspace = temp_dir.path().to_path_buf();

        let worker_handle = Arc::new(MockWorkerHandle {
            worker_id: WorkerId::new(),
        });
        let tool_manager = Arc::new(MockToolManager);

        let executor = PipelineStepExecutor::new(worker_handle, tool_manager, base_workspace);

        let tools = executor.extract_required_tools(&pipeline);
        assert_eq!(tools.len(), 0); // No tools field in PipelineStep yet
    }
}

impl From<ExecutionError> for hodei_pipelines_domain::DomainError {
    fn from(error: ExecutionError) -> Self {
        hodei_pipelines_domain::DomainError::Infrastructure(error.to_string())
    }
}
