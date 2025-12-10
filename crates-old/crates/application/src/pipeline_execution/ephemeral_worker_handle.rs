//! Ephemeral Worker Handle Adapter
//!
//! This module provides an adapter that implements EphemeralWorkerHandle
//! for workers provisioned by WorkerProvisioner, enabling integration with
//! PipelineStepExecutor.

use async_trait::async_trait;
use hodei_pipelines_domain::{DomainError, Result, WorkerId};
use hodei_pipelines_ports::scheduling::worker_provisioner::ProvisionedWorker;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;
use tracing::info;

use super::pipeline_step_executor::{EphemeralWorkerHandle, StepExecutionResult};

/// Adapter that implements EphemeralWorkerHandle for a provisioned worker
#[derive(Debug, Clone)]
pub struct ProvisionedWorkerHandle {
    worker_id: WorkerId,
    #[allow(dead_code)]
    worker: Arc<ProvisionedWorker>,
}

impl ProvisionedWorkerHandle {
    /// Create a new handle for a provisioned worker
    pub fn new(worker: Arc<ProvisionedWorker>) -> Self {
        Self {
            worker_id: worker.worker_id.clone(),
            worker,
        }
    }
}

#[async_trait]
impl EphemeralWorkerHandle for ProvisionedWorkerHandle {
    async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&Path>,
        env_vars: Option<&[(String, String)]>,
    ) -> Result<StepExecutionResult> {
        info!(
            worker_id = %self.worker_id,
            command = command,
            args = ?args,
            "Executing command in provisioned worker"
        );

        // In a real implementation, this would communicate with the actual worker
        // (e.g., via Docker exec, Kubernetes exec, or worker agent API)
        // For now, we'll simulate execution with a local process

        let start_time = std::time::Instant::now();

        // Build the full command
        let mut cmd = Command::new(command);

        // Add arguments
        cmd.args(args);

        // Set working directory if provided
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Add environment variables
        if let Some(vars) = env_vars {
            for (key, value) in vars {
                cmd.env(key, value);
            }
        }

        // Add worker-specific environment variables
        cmd.env("WORKER_ID", self.worker_id.to_string());
        cmd.env("PIPELINE_WORKER", "true");

        // Execute command
        let output = cmd.output().await.map_err(|e| {
            DomainError::Infrastructure(format!("Failed to execute command: {}", e))
        })?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let exit_code = output.status.code();
        let success = output.status.success();

        // Collect logs
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let mut logs = Vec::new();

        if !stdout.is_empty() {
            logs.push(format!("STDOUT:\n{}", stdout));
        }
        if !stderr.is_empty() {
            logs.push(format!("STDERR:\n{}", stderr));
        }

        // If no logs were captured, add a default message
        if logs.is_empty() {
            logs.push("Command executed (no output)".to_string());
        }

        info!(
            worker_id = %self.worker_id,
            command = command,
            exit_code = ?exit_code,
            success = success,
            duration_ms = duration_ms,
            "Command execution completed"
        );

        Ok(StepExecutionResult {
            step_id: self.worker_id.to_string(),
            success,
            exit_code,
            duration_ms,
            logs,
        })
    }

    fn worker_id(&self) -> &WorkerId {
        &self.worker_id
    }

    async fn is_healthy(&self) -> Result<bool> {
        // In a real implementation, check worker health via provider API
        // For now, assume provisioned workers are healthy
        Ok(true)
    }
}

/// Builder for creating ProvisionedWorkerHandle with proper configuration
pub struct ProvisionedWorkerHandleBuilder {
    worker: Arc<ProvisionedWorker>,
    tool_manager: Option<Arc<dyn super::pipeline_step_executor::ToolManager>>,
}

impl ProvisionedWorkerHandleBuilder {
    /// Create a new builder for a provisioned worker
    pub fn new(worker: Arc<ProvisionedWorker>) -> Self {
        Self {
            worker,
            tool_manager: None,
        }
    }

    /// Set tool manager for tool installation in worker
    pub fn with_tool_manager(mut self, tool_manager: Arc<dyn super::pipeline_step_executor::ToolManager>) -> Self {
        self.tool_manager = Some(tool_manager);
        self
    }

    /// Build the EphemeralWorkerHandle
    pub fn build(self) -> ProvisionedWorkerHandle {
        // Note: Currently tool_manager is not used in the handle,
        // but could be used for pre-execution tool installation
        ProvisionedWorkerHandle::new(self.worker)
    }
}