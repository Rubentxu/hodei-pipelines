//! Pipeline Execution Domain Entity
//!
//! This module contains the PipelineExecution aggregate root and related value objects
//! for executing pipelines with step orchestration and dependency management.

use crate::pipeline::{PipelineId, PipelineStepId};
use crate::{DomainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Execution identifier - Value Object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[schema(value_type = String)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct ExecutionId(pub uuid::Uuid);

impl ExecutionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Step execution identifier - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct StepExecutionId(pub Uuid);

impl StepExecutionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for StepExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StepExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pipeline execution status - Value Object (Enum)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    PENDING,
    RUNNING,
    COMPLETED,
    FAILED,
    CANCELLED,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PENDING => "PENDING",
            Self::RUNNING => "RUNNING",
            Self::COMPLETED => "COMPLETED",
            Self::FAILED => "FAILED",
            Self::CANCELLED => "CANCELLED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::COMPLETED | Self::FAILED | Self::CANCELLED)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::PENDING | Self::RUNNING)
    }

    /// Create from string (for backward compatibility)
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(status: &str) -> Result<Self> {
        match status {
            "PENDING" => Ok(Self::PENDING),
            "RUNNING" => Ok(Self::RUNNING),
            "COMPLETED" => Ok(Self::COMPLETED),
            "FAILED" => Ok(Self::FAILED),
            "CANCELLED" => Ok(Self::CANCELLED),
            _ => Err(DomainError::Validation(format!(
                "invalid execution status: {}",
                status
            ))),
        }
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for ExecutionStatus {
    fn from(s: String) -> Self {
        Self::from_str(&s).expect("valid status")
    }
}

impl From<&str> for ExecutionStatus {
    fn from(s: &str) -> Self {
        Self::from_str(s).expect("valid status")
    }
}

/// Step execution status - Value Object (Enum)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepExecutionStatus {
    PENDING,
    RUNNING,
    COMPLETED,
    FAILED,
    SKIPPED,
}

impl StepExecutionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PENDING => "PENDING",
            Self::RUNNING => "RUNNING",
            Self::COMPLETED => "COMPLETED",
            Self::FAILED => "FAILED",
            Self::SKIPPED => "SKIPPED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::COMPLETED | Self::FAILED | Self::SKIPPED)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::PENDING | Self::RUNNING)
    }

    /// Create from string (for backward compatibility)
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(status: &str) -> Result<Self> {
        match status {
            "PENDING" => Ok(Self::PENDING),
            "RUNNING" => Ok(Self::RUNNING),
            "COMPLETED" => Ok(Self::COMPLETED),
            "FAILED" => Ok(Self::FAILED),
            "SKIPPED" => Ok(Self::SKIPPED),
            _ => Err(DomainError::Validation(format!(
                "invalid step execution status: {}",
                status
            ))),
        }
    }
}

impl std::fmt::Display for StepExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for StepExecutionStatus {
    fn from(s: String) -> Self {
        Self::from_str(&s).expect("valid status")
    }
}

impl From<&str> for StepExecutionStatus {
    fn from(s: &str) -> Self {
        Self::from_str(s).expect("valid status")
    }
}

/// Step execution data - Value Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecution {
    pub step_execution_id: StepExecutionId,
    pub step_id: PipelineStepId,
    pub status: StepExecutionStatus,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u8,
    pub error_message: Option<String>,
    pub logs: Vec<String>,
}

impl StepExecution {
    pub fn new(step_id: PipelineStepId) -> Self {
        Self {
            step_execution_id: StepExecutionId::new(),
            step_id,
            status: StepExecutionStatus::PENDING,
            started_at: None,
            completed_at: None,
            retry_count: 0,
            error_message: None,
            logs: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.status = StepExecutionStatus::RUNNING;
        self.started_at = Some(chrono::Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = StepExecutionStatus::COMPLETED;
        self.completed_at = Some(chrono::Utc::now());
    }

    pub fn fail(&mut self, error_message: String) {
        self.status = StepExecutionStatus::FAILED;
        self.completed_at = Some(chrono::Utc::now());
        self.error_message = Some(error_message);
    }

    pub fn skip(&mut self) {
        self.status = StepExecutionStatus::SKIPPED;
        self.completed_at = Some(chrono::Utc::now());
    }

    pub fn retry(&mut self) {
        self.retry_count += 1;
        self.status = StepExecutionStatus::PENDING;
        self.started_at = None;
        self.completed_at = None;
        self.error_message = None;
    }

    pub fn add_log(&mut self, log: String) {
        self.logs.push(log);
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, StepExecutionStatus::RUNNING)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, StepExecutionStatus::COMPLETED)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, StepExecutionStatus::FAILED)
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self.status, StepExecutionStatus::SKIPPED)
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

/// Pipeline execution aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineExecution {
    pub id: ExecutionId,
    pub pipeline_id: PipelineId,
    pub status: ExecutionStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub steps: Vec<StepExecution>,
    pub variables: HashMap<String, String>,
    pub tenant_id: Option<String>,
    pub correlation_id: Option<String>,
}

impl PipelineExecution {
    /// Create a new pipeline execution
    pub fn new(
        pipeline_id: PipelineId,
        steps: Vec<PipelineStepId>,
        variables: HashMap<String, String>,
        tenant_id: Option<String>,
        correlation_id: Option<String>,
    ) -> Self {
        let step_executions = steps.into_iter().map(StepExecution::new).collect();

        Self {
            id: ExecutionId::new(),
            pipeline_id,
            status: ExecutionStatus::PENDING,
            started_at: chrono::Utc::now(),
            completed_at: None,
            steps: step_executions,
            variables,
            tenant_id,
            correlation_id,
        }
    }

    /// Start the pipeline execution
    pub fn start(&mut self) -> Result<()> {
        if !matches!(self.status, ExecutionStatus::PENDING) {
            return Err(DomainError::Validation(
                "Pipeline execution can only be started from PENDING state".to_string(),
            ));
        }

        self.status = ExecutionStatus::RUNNING;
        Ok(())
    }

    /// Complete the pipeline execution
    pub fn complete(&mut self) -> Result<()> {
        // Verify all steps are in terminal state
        let all_completed = self.steps.iter().all(|step| step.is_terminal());
        let any_failed = self.steps.iter().any(|step| step.is_failed());

        if !all_completed {
            return Err(DomainError::Validation(
                "Cannot complete pipeline execution with pending steps".to_string(),
            ));
        }

        if any_failed {
            return Err(DomainError::Validation(
                "Cannot complete pipeline execution with failed steps".to_string(),
            ));
        }

        self.status = ExecutionStatus::COMPLETED;
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Fail the pipeline execution
    pub fn fail(&mut self) -> Result<()> {
        if matches!(
            self.status,
            ExecutionStatus::COMPLETED | ExecutionStatus::FAILED | ExecutionStatus::CANCELLED
        ) {
            return Err(DomainError::Validation(
                "Cannot fail a pipeline execution that is already in terminal state".to_string(),
            ));
        }

        self.status = ExecutionStatus::FAILED;
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Cancel the pipeline execution
    pub fn cancel(&mut self) -> Result<()> {
        if matches!(
            self.status,
            ExecutionStatus::COMPLETED | ExecutionStatus::FAILED | ExecutionStatus::CANCELLED
        ) {
            return Err(DomainError::Validation(
                "Cannot cancel a pipeline execution that is already in terminal state".to_string(),
            ));
        }

        self.status = ExecutionStatus::CANCELLED;
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Get a step execution by step ID
    pub fn get_step_execution(&self, step_id: &PipelineStepId) -> Option<&StepExecution> {
        self.steps.iter().find(|step| &step.step_id == step_id)
    }

    /// Get a mutable step execution by step ID
    pub fn get_step_execution_mut(
        &mut self,
        step_id: &PipelineStepId,
    ) -> Option<&mut StepExecution> {
        self.steps.iter_mut().find(|step| &step.step_id == step_id)
    }

    /// Start a specific step execution
    pub fn start_step(&mut self, step_id: &PipelineStepId) -> Result<()> {
        let step = self
            .get_step_execution_mut(step_id)
            .ok_or_else(|| DomainError::Validation(format!("Step {} not found", step_id)))?;

        if !matches!(step.status, StepExecutionStatus::PENDING) {
            return Err(DomainError::Validation(format!(
                "Step {} can only be started from PENDING state",
                step_id
            )));
        }

        step.start();
        Ok(())
    }

    /// Complete a specific step execution
    pub fn complete_step(&mut self, step_id: &PipelineStepId) -> Result<()> {
        let step = self
            .get_step_execution_mut(step_id)
            .ok_or_else(|| DomainError::Validation(format!("Step {} not found", step_id)))?;

        if !matches!(step.status, StepExecutionStatus::RUNNING) {
            return Err(DomainError::Validation(format!(
                "Step {} can only be completed from RUNNING state",
                step_id
            )));
        }

        step.complete();
        Ok(())
    }

    /// Fail a specific step execution
    pub fn fail_step(&mut self, step_id: &PipelineStepId, error_message: String) -> Result<()> {
        let step = self
            .get_step_execution_mut(step_id)
            .ok_or_else(|| DomainError::Validation(format!("Step {} not found", step_id)))?;

        if !matches!(
            step.status,
            StepExecutionStatus::RUNNING | StepExecutionStatus::PENDING
        ) {
            return Err(DomainError::Validation(format!(
                "Step {} can only be failed from RUNNING or PENDING state",
                step_id
            )));
        }

        step.fail(error_message);
        Ok(())
    }

    /// Skip a specific step execution
    pub fn skip_step(&mut self, step_id: &PipelineStepId) -> Result<()> {
        let step = self
            .get_step_execution_mut(step_id)
            .ok_or_else(|| DomainError::Validation(format!("Step {} not found", step_id)))?;

        if !matches!(step.status, StepExecutionStatus::PENDING) {
            return Err(DomainError::Validation(format!(
                "Step {} can only be skipped from PENDING state",
                step_id
            )));
        }

        step.skip();
        Ok(())
    }

    /// Retry a specific step execution
    pub fn retry_step(&mut self, step_id: &PipelineStepId, max_retries: u8) -> Result<()> {
        let step = self
            .get_step_execution_mut(step_id)
            .ok_or_else(|| DomainError::Validation(format!("Step {} not found", step_id)))?;

        if !matches!(step.status, StepExecutionStatus::FAILED) {
            return Err(DomainError::Validation(
                "Only FAILED steps can be retried".to_string(),
            ));
        }

        if step.retry_count >= max_retries {
            return Err(DomainError::Validation(format!(
                "Step {} has exceeded maximum retry count",
                step_id
            )));
        }

        step.retry();
        Ok(())
    }

    /// Add a log to a specific step execution
    pub fn add_step_log(&mut self, step_id: &PipelineStepId, log: String) -> Result<()> {
        let step = self
            .get_step_execution_mut(step_id)
            .ok_or_else(|| DomainError::Validation(format!("Step {} not found", step_id)))?;

        step.add_log(log);
        Ok(())
    }

    /// Get all pending steps
    pub fn get_pending_steps(&self) -> Vec<&StepExecution> {
        self.steps
            .iter()
            .filter(|step| matches!(step.status, StepExecutionStatus::PENDING))
            .collect()
    }

    /// Get all running steps
    pub fn get_running_steps(&self) -> Vec<&StepExecution> {
        self.steps.iter().filter(|step| step.is_running()).collect()
    }

    /// Get all completed steps
    pub fn get_completed_steps(&self) -> Vec<&StepExecution> {
        self.steps
            .iter()
            .filter(|step| step.is_completed())
            .collect()
    }

    /// Get all failed steps
    pub fn get_failed_steps(&self) -> Vec<&StepExecution> {
        self.steps.iter().filter(|step| step.is_failed()).collect()
    }

    /// Check if pipeline execution is running
    pub fn is_running(&self) -> bool {
        matches!(self.status, ExecutionStatus::RUNNING)
    }

    /// Check if pipeline execution is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, ExecutionStatus::COMPLETED)
    }

    /// Check if pipeline execution is failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, ExecutionStatus::FAILED)
    }

    /// Check if pipeline execution is cancelled
    pub fn is_cancelled(&self) -> bool {
        matches!(self.status, ExecutionStatus::CANCELLED)
    }

    /// Check if pipeline execution is in terminal state
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Check if pipeline execution is active (not terminal)
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Get execution duration
    pub fn get_duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed - self.started_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_execution_id_generation() {
        let id = ExecutionId::new();
        assert!(!id.as_uuid().is_nil());
    }

    #[test]
    fn test_step_execution_creation() {
        let step_id = PipelineStepId::new();
        let step_exec = StepExecution::new(step_id.clone());

        assert_eq!(step_exec.step_id, step_id);
        assert!(matches!(step_exec.status, StepExecutionStatus::PENDING));
        assert!(!step_exec.is_terminal());
        assert_eq!(step_exec.retry_count, 0);
    }

    #[test]
    fn test_step_execution_start() {
        let step_id = PipelineStepId::new();
        let mut step_exec = StepExecution::new(step_id);

        step_exec.start();
        assert!(matches!(step_exec.status, StepExecutionStatus::RUNNING));
        assert!(step_exec.started_at.is_some());
    }

    #[test]
    fn test_step_execution_complete() {
        let step_id = PipelineStepId::new();
        let mut step_exec = StepExecution::new(step_id);

        step_exec.start();
        step_exec.complete();

        assert!(matches!(step_exec.status, StepExecutionStatus::COMPLETED));
        assert!(step_exec.completed_at.is_some());
        assert!(step_exec.is_terminal());
    }

    #[test]
    fn test_step_execution_fail() {
        let step_id = PipelineStepId::new();
        let mut step_exec = StepExecution::new(step_id);

        step_exec.start();
        step_exec.fail("Test error".to_string());

        assert!(matches!(step_exec.status, StepExecutionStatus::FAILED));
        assert!(step_exec.completed_at.is_some());
        assert_eq!(step_exec.error_message, Some("Test error".to_string()));
        assert!(step_exec.is_terminal());
    }

    #[test]
    fn test_step_execution_retry() {
        let step_id = PipelineStepId::new();
        let mut step_exec = StepExecution::new(step_id);

        step_exec.start();
        step_exec.fail("Test error".to_string());
        step_exec.retry();

        assert!(matches!(step_exec.status, StepExecutionStatus::PENDING));
        assert_eq!(step_exec.retry_count, 1);
        assert!(step_exec.completed_at.is_none());
    }

    #[test]
    fn test_pipeline_execution_creation() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new(), PipelineStepId::new()];
        let variables = HashMap::new();

        let exec = PipelineExecution::new(
            pipeline_id.clone(),
            steps.clone(),
            variables,
            Some("tenant1".to_string()),
            Some("corr123".to_string()),
        );

        assert_eq!(exec.pipeline_id, pipeline_id);
        assert!(matches!(exec.status, ExecutionStatus::PENDING));
        assert_eq!(exec.steps.len(), 2);
        assert!(
            exec.steps
                .iter()
                .all(|s| matches!(s.status, StepExecutionStatus::PENDING))
        );
        assert_eq!(exec.tenant_id, Some("tenant1".to_string()));
        assert_eq!(exec.correlation_id, Some("corr123".to_string()));
    }

    #[test]
    fn test_pipeline_execution_start() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        exec.start().unwrap();
        assert!(matches!(exec.status, ExecutionStatus::RUNNING));
    }

    #[test]
    fn test_pipeline_execution_start_fails_when_not_pending() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        exec.start().unwrap();
        let result = exec.start();
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_execution_complete() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        exec.start().unwrap();

        // Start and complete the step
        let step_id = exec.steps[0].step_id.clone();
        exec.start_step(&step_id).unwrap();
        exec.complete_step(&step_id).unwrap();

        // Now complete the pipeline
        exec.complete().unwrap();
        assert!(matches!(exec.status, ExecutionStatus::COMPLETED));
        assert!(exec.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_execution_complete_fails_with_pending_steps() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        exec.start().unwrap();
        let result = exec.complete();
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_execution_complete_fails_with_failed_steps() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        exec.start().unwrap();

        // Fail the step
        let step_id = exec.steps[0].step_id.clone();
        exec.fail_step(&step_id, "Test error".to_string()).unwrap();

        // Try to complete the pipeline
        let result = exec.complete();
        assert!(result.is_err());
    }

    #[test]
    fn test_start_step() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        exec.start().unwrap();
        let step_id = exec.steps[0].step_id.clone();
        exec.start_step(&step_id).unwrap();

        assert!(matches!(exec.steps[0].status, StepExecutionStatus::RUNNING));
    }

    #[test]
    fn test_start_step_fails_with_invalid_state() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        let step_id = exec.steps[0].step_id.clone();
        exec.start_step(&step_id).unwrap();

        // Try to start again
        let result = exec.start_step(&step_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_step_log() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        let step_id = exec.steps[0].step_id.clone();
        exec.add_step_log(&step_id, "Test log".to_string()).unwrap();

        assert_eq!(exec.steps[0].logs.len(), 1);
        assert_eq!(exec.steps[0].logs[0], "Test log");
    }

    #[test]
    fn test_get_step_execution() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new(), PipelineStepId::new()];
        let variables = HashMap::new();

        let exec = PipelineExecution::new(pipeline_id, steps.clone(), variables, None, None);

        let found = exec.get_step_execution(&steps[0]).unwrap();
        assert_eq!(found.step_id, steps[0]);
    }

    #[test]
    fn test_get_pending_steps() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new(), PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps.clone(), variables, None, None);

        assert_eq!(exec.get_pending_steps().len(), 2);

        // Start one step
        exec.start_step(&steps[0]).unwrap();
        assert_eq!(exec.get_pending_steps().len(), 1);
    }

    #[test]
    fn test_skip_step() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        let step_id = exec.steps[0].step_id.clone();
        exec.skip_step(&step_id).unwrap();

        assert!(matches!(exec.steps[0].status, StepExecutionStatus::SKIPPED));
    }

    #[test]
    fn test_cancel_execution() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        exec.start().unwrap();
        exec.cancel().unwrap();

        assert!(matches!(exec.status, ExecutionStatus::CANCELLED));
        assert!(exec.is_terminal());
    }

    #[test]
    fn test_retry_step_within_limits() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        let step_id = exec.steps[0].step_id.clone();
        exec.start_step(&step_id).unwrap();
        exec.fail_step(&step_id, "Test error".to_string()).unwrap();

        // Retry the step
        exec.retry_step(&step_id, 3).unwrap();

        assert_eq!(exec.steps[0].retry_count, 1);
        assert!(matches!(exec.steps[0].status, StepExecutionStatus::PENDING));
    }

    #[test]
    fn test_retry_step_exceeds_max_retries() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        let step_id = exec.steps[0].step_id.clone();
        exec.start_step(&step_id).unwrap();
        exec.fail_step(&step_id, "Test error".to_string()).unwrap();

        // Retry twice (max_retries = 3)
        exec.retry_step(&step_id, 3).unwrap();
        exec.start_step(&step_id).unwrap();
        exec.fail_step(&step_id, "Test error".to_string()).unwrap();
        exec.retry_step(&step_id, 3).unwrap();

        // Try to retry again (should exceed max_retries)
        let result = exec.retry_step(&step_id, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_execution_duration() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let mut exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        assert!(exec.get_duration().is_none());

        exec.start().unwrap();
        let step_id = exec.steps[0].step_id.clone();
        exec.start_step(&step_id).unwrap();
        exec.complete_step(&step_id).unwrap();
        exec.complete().unwrap();

        assert!(exec.get_duration().is_some());
    }

    #[test]
    fn test_is_running_and_active() {
        let pipeline_id = PipelineId::new();
        let steps = vec![PipelineStepId::new()];
        let variables = HashMap::new();

        let exec = PipelineExecution::new(pipeline_id, steps, variables, None, None);

        assert!(!exec.is_running());
        assert!(exec.is_active());
    }

    #[test]
    fn test_status_terminal_check() {
        assert!(ExecutionStatus::COMPLETED.is_terminal());
        assert!(ExecutionStatus::FAILED.is_terminal());
        assert!(ExecutionStatus::CANCELLED.is_terminal());
        assert!(!ExecutionStatus::PENDING.is_terminal());
        assert!(!ExecutionStatus::RUNNING.is_terminal());

        assert!(ExecutionStatus::PENDING.is_active());
        assert!(ExecutionStatus::RUNNING.is_active());
        assert!(!ExecutionStatus::COMPLETED.is_active());
        assert!(!ExecutionStatus::FAILED.is_active());
        assert!(!ExecutionStatus::CANCELLED.is_active());
    }
}
