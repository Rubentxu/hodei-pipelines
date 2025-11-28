//! Pipeline CRUD Module
//!
//! Provides complete CRUD operations for Pipeline entities following DDD principles.
//! Implements use cases for creating, reading, updating, and deleting pipelines.

use hodei_core::{
    Result,
    job::JobSpec,
    pipeline::{
        Pipeline, PipelineId, PipelineStatus, PipelineStep, PipelineStepBuilder, PipelineStepId,
    },
    pipeline_execution::PipelineExecution,
};
use hodei_ports::{EventBusError, EventPublisher, PipelineRepository, SystemEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Configuration for Pipeline CRUD operations
#[derive(Debug, Clone)]
pub struct PipelineCrudConfig {
    pub max_pipeline_steps: usize,
    pub max_pipeline_name_length: usize,
    pub default_timeout_ms: u64,
}

impl Default for PipelineCrudConfig {
    fn default() -> Self {
        Self {
            max_pipeline_steps: 100,
            max_pipeline_name_length: 255,
            default_timeout_ms: 300000,
        }
    }
}

/// Pipeline CRUD Service - Application layer use cases
/// Implements complete CRUD operations for Pipeline entities
pub struct PipelineCrudService<R, E>
where
    R: PipelineRepository,
    E: EventPublisher,
{
    pipeline_repo: Arc<R>,
    event_bus: Arc<E>,
    config: PipelineCrudConfig,
}

impl<R, E> Clone for PipelineCrudService<R, E>
where
    R: PipelineRepository,
    E: EventPublisher,
{
    fn clone(&self) -> Self {
        Self {
            pipeline_repo: self.pipeline_repo.clone(),
            event_bus: self.event_bus.clone(),
            config: self.config.clone(),
        }
    }
}

impl<R, E> PipelineCrudService<R, E>
where
    R: PipelineRepository,
    E: EventPublisher,
{
    /// Create a new Pipeline CRUD service instance
    pub fn new(pipeline_repo: Arc<R>, event_bus: Arc<E>, config: PipelineCrudConfig) -> Self {
        Self {
            pipeline_repo,
            event_bus,
            config,
        }
    }

    /// Create a new pipeline with validation
    /// Uses Builder Pattern for step creation
    pub async fn create_pipeline(&self, request: CreatePipelineRequest) -> Result<Pipeline> {
        info!("Creating pipeline: {}", request.name);

        // Validate name length
        if request.name.len() > self.config.max_pipeline_name_length {
            return Err(PipelineCrudError::Validation(format!(
                "Pipeline name exceeds maximum length of {}",
                self.config.max_pipeline_name_length
            ))
            .into());
        }

        // Validate steps count
        if request.steps.len() > self.config.max_pipeline_steps {
            return Err(PipelineCrudError::Validation(format!(
                "Pipeline exceeds maximum number of steps: {}",
                self.config.max_pipeline_steps
            ))
            .into());
        }

        // Create steps using Builder Pattern
        let mut steps = Vec::with_capacity(request.steps.len());
        for step_request in request.steps {
            let step = self.create_pipeline_step(step_request)?;
            steps.push(step);
        }

        // Create pipeline with domain validation
        let pipeline = Pipeline::new(PipelineId::new(), request.name, steps)
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Publish event
        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineCreated(pipeline.clone()))
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        info!("Pipeline created successfully: {}", pipeline.id);

        Ok(pipeline)
    }

    /// Get pipeline by ID
    pub async fn get_pipeline(&self, id: &PipelineId) -> Result<Option<Pipeline>> {
        let pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        if let Some(ref p) = pipeline {
            info!("Retrieved pipeline: {}", p.id);
        } else {
            warn!("Pipeline not found: {}", id);
        }

        Ok(pipeline)
    }

    /// List all pipelines with optional filtering
    pub async fn list_pipelines(
        &self,
        filter: Option<&ListPipelinesFilter>,
    ) -> Result<Vec<Pipeline>> {
        info!("Listing pipelines with filter: {:?}", filter);

        // Get all pipelines from repository
        // Note: In production, this would use pagination and proper querying
        let mut all_pipelines = self
            .pipeline_repo
            .get_all_pipelines()
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Apply filter if provided
        if let Some(filter) = filter {
            all_pipelines.retain(|p| filter.matches(p));
        }

        info!("Found {} pipelines", all_pipelines.len());

        Ok(all_pipelines)
    }

    /// Update pipeline (partial update)
    pub async fn update_pipeline(
        &self,
        id: &PipelineId,
        request: UpdatePipelineRequest,
    ) -> Result<Pipeline> {
        info!("Updating pipeline: {}", id);

        // Get existing pipeline
        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Update fields if provided
        if let Some(name) = request.name {
            // Validate name length
            if name.len() > self.config.max_pipeline_name_length {
                return Err(PipelineCrudError::Validation(format!(
                    "Pipeline name exceeds maximum length of {}",
                    self.config.max_pipeline_name_length
                ))
                .into());
            }
            pipeline.name = name;
        }

        if let Some(description) = request.description {
            pipeline.description = Some(description);
        }

        if let Some(variables) = request.variables {
            pipeline.variables = variables;
        }

        // Update steps if provided
        if let Some(steps) = request.steps {
            // Validate steps count
            if steps.len() > self.config.max_pipeline_steps {
                return Err(PipelineCrudError::Validation(format!(
                    "Pipeline exceeds maximum number of steps: {}",
                    self.config.max_pipeline_steps
                ))
                .into());
            }

            // Validate DAG is acyclic
            let mut new_steps = Vec::with_capacity(steps.len());
            for step_request in steps {
                let step = self.create_pipeline_step(step_request)?;
                new_steps.push(step);
            }

            // Validate DAG
            let _ = Pipeline::new(PipelineId::new(), pipeline.name.clone(), new_steps.clone())
                .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

            pipeline.steps = new_steps;
        }

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Publish event
        info!("Pipeline updated: {}", id);

        info!("Pipeline updated successfully: {}", id);

        Ok(pipeline)
    }

    /// Delete pipeline
    pub async fn delete_pipeline(&self, id: &PipelineId) -> Result<()> {
        info!("Deleting pipeline: {}", id);

        // Check if pipeline exists
        let pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Check if pipeline is running
        if pipeline.is_running() {
            return Err(PipelineCrudError::Validation(
                "Cannot delete running pipeline".to_string(),
            )
            .into());
        }

        // Check if pipeline is in terminal state
        if pipeline.is_terminal() {
            // Allow deletion of terminal pipelines with force flag
            // For now, we'll allow it
            info!("Deleting terminal pipeline: {}", id);
        }

        // Delete from repository
        self.pipeline_repo
            .delete_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Publish event
        info!("Pipeline deleted: {}", id);

        info!("Pipeline deleted successfully: {}", id);

        Ok(())
    }

    /// Get pipeline execution order (topological sort)
    pub async fn get_execution_order(&self, id: &PipelineId) -> Result<Vec<PipelineStep>> {
        info!("Getting execution order for pipeline: {}", id);

        let pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        let execution_order = pipeline
            .get_execution_order()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        Ok(execution_order.iter().map(|s| (*s).clone()).collect())
    }

    /// Start pipeline execution
    pub async fn start_pipeline(&self, id: &PipelineId) -> Result<()> {
        info!("Starting pipeline: {}", id);

        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Validate pipeline is not already running
        if pipeline.is_running() {
            return Err(
                PipelineCrudError::Validation("Pipeline is already running".to_string()).into(),
            );
        }

        // Validate pipeline is in valid state to start
        if pipeline.is_terminal() {
            return Err(PipelineCrudError::Validation(
                "Cannot start pipeline in terminal state".to_string(),
            )
            .into());
        }

        // Start pipeline
        pipeline
            .start()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Publish event
        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineStarted {
                pipeline_id: id.clone(),
            })
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        info!("Pipeline started successfully: {}", id);

        Ok(())
    }

    /// Complete pipeline successfully
    pub async fn complete_pipeline(&self, id: &PipelineId) -> Result<()> {
        info!("Completing pipeline: {}", id);

        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Complete pipeline
        pipeline
            .complete()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Publish event
        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineCompleted {
                pipeline_id: id.clone(),
            })
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        info!("Pipeline completed successfully: {}", id);

        Ok(())
    }

    /// Fail pipeline
    pub async fn fail_pipeline(&self, id: &PipelineId) -> Result<()> {
        info!("Failing pipeline: {}", id);

        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Fail pipeline
        pipeline
            .fail()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Publish event
        info!("Pipeline failed: {}", id);

        info!("Pipeline failed: {}", id);

        Ok(())
    }

    /// Execute pipeline with orchestration of steps and dependencies
    pub async fn execute_pipeline(
        &self,
        request: ExecutePipelineRequest,
    ) -> Result<PipelineExecution> {
        info!("Executing pipeline: {}", request.pipeline_id);

        // Get pipeline from repository
        let pipeline = self
            .pipeline_repo
            .get_pipeline(&request.pipeline_id)
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?
            .ok_or(PipelineCrudError::NotFound(request.pipeline_id.clone()))?;

        // Validate pipeline is not already running
        if pipeline.is_running() {
            return Err(
                PipelineCrudError::Validation("Pipeline is already running".to_string()).into(),
            );
        }

        // Validate pipeline is in valid state to execute
        if pipeline.is_terminal() {
            return Err(PipelineCrudError::Validation(
                "Cannot execute pipeline in terminal state".to_string(),
            )
            .into());
        }

        // Get execution order based on dependencies
        let execution_order = pipeline
            .get_execution_order()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Extract step IDs in execution order
        let step_ids: Vec<PipelineStepId> =
            execution_order.iter().map(|step| step.id.clone()).collect();

        // Create pipeline execution with all steps
        let variables = request.variables.unwrap_or_default();
        let execution = PipelineExecution::new(
            pipeline.id.clone(),
            step_ids,
            variables,
            request.tenant_id,
            request.correlation_id,
        );

        // Publish pipeline execution started event
        self.event_bus
            .publish(SystemEvent::PipelineExecutionStarted {
                pipeline_id: pipeline.id.clone(),
                execution_id: execution.id.clone(),
            })
            .await
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        info!(
            "Pipeline execution started: {} (execution_id: {})",
            request.pipeline_id, execution.id
        );

        // TODO: In a real implementation, this would trigger async step execution
        // For now, we just return the execution instance

        Ok(execution)
    }

    /// Create a pipeline step from request using Builder Pattern
    fn create_pipeline_step(
        &self,
        step_request: CreatePipelineStepRequest,
    ) -> Result<PipelineStep> {
        // Convert JobSpec
        let job_spec = JobSpec {
            name: step_request.name.clone(),
            image: step_request.image,
            command: step_request.command,
            resources: step_request.resources.unwrap_or_default(),
            timeout_ms: step_request
                .timeout_ms
                .unwrap_or(self.config.default_timeout_ms),
            retries: step_request.retries.unwrap_or(0) as u8,
            env: step_request.env.unwrap_or_default(),
            secret_refs: step_request.secret_refs.unwrap_or_default(),
        };

        // Create step using Builder Pattern
        let mut step_builder = PipelineStepBuilder::new()
            .name(step_request.name)
            .job_spec(job_spec);

        // Add timeout if provided
        if let Some(timeout_ms) = step_request.timeout_ms {
            step_builder = step_builder.timeout(timeout_ms);
        }

        // Add dependencies
        for dep_id_str in step_request.depends_on.unwrap_or_default() {
            let dep_id: PipelineStepId = dep_id_str.parse().map_err(|_| {
                PipelineCrudError::Validation(format!("Invalid step ID: {}", dep_id_str))
            })?;
            step_builder = step_builder.depends_on(dep_id);
        }

        let step = step_builder
            .build()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        Ok(step)
    }
}

// ========== DTOs (Data Transfer Objects) ==========

/// Request to create a new pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipelineRequest {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<CreatePipelineStepRequest>,
    pub variables: Option<HashMap<String, String>>,
}

/// Request to create a pipeline step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipelineStepRequest {
    pub name: String,
    pub image: String,
    pub command: Vec<String>,
    pub resources: Option<hodei_core::job::ResourceQuota>,
    pub timeout_ms: Option<u64>,
    pub retries: Option<u32>,
    pub env: Option<HashMap<String, String>>,
    pub secret_refs: Option<Vec<String>>,
    pub depends_on: Option<Vec<String>>, // Step IDs
}

/// Request to update a pipeline (partial update)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePipelineRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub steps: Option<Vec<CreatePipelineStepRequest>>,
    pub variables: Option<HashMap<String, String>>,
}

/// Request to execute a pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutePipelineRequest {
    pub pipeline_id: PipelineId,
    pub variables: Option<HashMap<String, String>>,
    pub tenant_id: Option<String>,
    pub correlation_id: Option<String>,
}

/// Filter for listing pipelines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPipelinesFilter {
    pub status: Option<PipelineStatus>,
    pub name_pattern: Option<String>, // Simple substring match
}

impl ListPipelinesFilter {
    /// Check if a pipeline matches the filter
    fn matches(&self, pipeline: &Pipeline) -> bool {
        if let Some(status) = &self.status
            && pipeline.status != *status
        {
            return false;
        }

        if let Some(pattern) = &self.name_pattern
            && !pipeline.name.contains(pattern)
        {
            return false;
        }

        true
    }
}

/// Response for listing pipelines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPipelinesResponse {
    pub pipelines: Vec<PipelineSummary>,
    pub total: usize,
}

/// Summary representation of a pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummary {
    pub id: PipelineId,
    pub name: String,
    pub description: Option<String>,
    pub status: PipelineStatus,
    pub step_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Pipeline> for PipelineSummary {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id,
            name: pipeline.name,
            description: pipeline.description,
            status: pipeline.status,
            step_count: pipeline.steps.len(),
            created_at: pipeline.created_at,
            updated_at: pipeline.updated_at,
        }
    }
}

// ========== Error Types ==========

#[derive(thiserror::Error, Debug)]
pub enum PipelineCrudError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Domain error: {0}")]
    DomainError(String),

    #[error("Pipeline not found: {0}")]
    NotFound(PipelineId),

    #[error("Pipeline repository error: {0}")]
    PipelineRepository(hodei_ports::PipelineRepositoryError),

    #[error("Event bus error: {0}")]
    EventBus(EventBusError),
}

// Convert PipelineCrudError to DomainError
impl From<PipelineCrudError> for hodei_core::DomainError {
    fn from(err: PipelineCrudError) -> Self {
        hodei_core::DomainError::Other(err.to_string())
    }
}
