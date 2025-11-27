//! Pipeline Execution Repository Port
//!
//! Defines the interface for persisting and retrieving pipeline executions.

use async_trait::async_trait;
use hodei_core::{
    Result,
    pipeline::PipelineStepId,
    pipeline_execution::{
        ExecutionId, ExecutionStatus, PipelineExecution, StepExecution, StepExecutionId,
        StepExecutionStatus,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Repository error types
#[derive(thiserror::Error, Debug)]
pub enum PipelineExecutionRepositoryError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Pipeline Execution Repository Port
#[async_trait]
pub trait PipelineExecutionRepository: Send + Sync {
    /// Save a pipeline execution
    async fn save_execution(&self, execution: &PipelineExecution) -> Result<()>;

    /// Get a pipeline execution by ID
    async fn get_execution(&self, execution_id: &ExecutionId) -> Result<Option<PipelineExecution>>;

    /// Get all executions for a pipeline
    async fn get_executions_by_pipeline(
        &self,
        pipeline_id: &hodei_core::PipelineId,
    ) -> Result<Vec<PipelineExecution>>;

    /// Update pipeline execution status
    async fn update_execution_status(
        &self,
        execution_id: &ExecutionId,
        status: ExecutionStatus,
    ) -> Result<()>;

    /// Update step execution status
    async fn update_step_status(
        &self,
        execution_id: &ExecutionId,
        step_id: &PipelineStepId,
        status: StepExecutionStatus,
    ) -> Result<()>;

    /// Update a step execution
    async fn update_step(&self, execution_id: &ExecutionId, step: &StepExecution) -> Result<()>;

    /// Delete a pipeline execution
    async fn delete_execution(&self, execution_id: &ExecutionId) -> Result<()>;

    /// Get all active (PENDING or RUNNING) executions
    async fn get_active_executions(&self) -> Result<Vec<PipelineExecution>>;
}
