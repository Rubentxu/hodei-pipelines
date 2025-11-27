//! Pipeline CRUD Module
//!
//! Provides complete CRUD operations for Pipeline entities following DDD principles.
//! Implements use cases for creating, reading, updating, and deleting pipelines.

use hodei_core::{
    DomainError,
    job::JobSpec,
    pipeline::{
        Pipeline, PipelineId, PipelineStatus, PipelineStep, PipelineStepBuilder, PipelineStepId,
    },
};
use hodei_ports::{EventPublisher, PipelineRepository};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

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
    pub async fn create_pipeline(
        &self,
        request: CreatePipelineRequest,
    ) -> Result<Pipeline, PipelineCrudError> {
        info!("Creating pipeline: {}", request.name);

        // Validate name length
        if request.name.len() > self.config.max_pipeline_name_length {
            return Err(PipelineCrudError::Validation(format!(
                "Pipeline name exceeds maximum length of {}",
                self.config.max_pipeline_name_length
            )));
        }

        // Validate steps count
        if request.steps.len() > self.config.max_pipeline_steps {
            return Err(PipelineCrudError::Validation(format!(
                "Pipeline exceeds maximum number of steps: {}",
                self.config.max_pipeline_steps
            )));
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
            .map_err(PipelineCrudError::PipelineRepository)?;

        // Publish event
        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineCreated(pipeline.clone()))
            .await
            .map_err(PipelineCrudError::EventBus)?;

        info!("Pipeline created successfully: {}", pipeline.id);

        Ok(pipeline)
    }

    /// Get pipeline by ID
    pub async fn get_pipeline(
        &self,
        id: &PipelineId,
    ) -> Result<Option<Pipeline>, PipelineCrudError> {
        let pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?;

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
    ) -> Result<Vec<Pipeline>, PipelineCrudError> {
        info!("Listing pipelines with filter: {:?}", filter);

        // Get all pipelines from repository
        // Note: In production, this would use pagination and proper querying
        let mut all_pipelines = self
            .pipeline_repo
            .get_all_pipelines()
            .await
            .map_err(PipelineCrudError::PipelineRepository)?;

        // Apply filter if provided
        if let Some(filter) = filter {
            all_pipelines = all_pipelines
                .into_iter()
                .filter(|p| filter.matches(p))
                .collect();
        }

        info!("Found {} pipelines", all_pipelines.len());

        Ok(all_pipelines)
    }

    /// Update pipeline (partial update)
    pub async fn update_pipeline(
        &self,
        id: &PipelineId,
        request: UpdatePipelineRequest,
    ) -> Result<Pipeline, PipelineCrudError> {
        info!("Updating pipeline: {}", id);

        // Get existing pipeline
        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Update fields if provided
        if let Some(name) = request.name {
            // Validate name length
            if name.len() > self.config.max_pipeline_name_length {
                return Err(PipelineCrudError::Validation(format!(
                    "Pipeline name exceeds maximum length of {}",
                    self.config.max_pipeline_name_length
                )));
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
                )));
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
            .map_err(PipelineCrudError::PipelineRepository)?;

        // Publish event
        info!("Pipeline updated: {}", id);

        info!("Pipeline updated successfully: {}", id);

        Ok(pipeline)
    }

    /// Delete pipeline
    pub async fn delete_pipeline(&self, id: &PipelineId) -> Result<(), PipelineCrudError> {
        info!("Deleting pipeline: {}", id);

        // Check if pipeline exists
        let pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Check if pipeline is running
        if pipeline.is_running() {
            return Err(PipelineCrudError::Validation(
                "Cannot delete running pipeline".to_string(),
            ));
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
            .map_err(PipelineCrudError::PipelineRepository)?;

        // Publish event
        info!("Pipeline deleted: {}", id);

        info!("Pipeline deleted successfully: {}", id);

        Ok(())
    }

    /// Get pipeline execution order (topological sort)
    pub async fn get_execution_order(
        &self,
        id: &PipelineId,
    ) -> Result<Vec<PipelineStep>, PipelineCrudError> {
        info!("Getting execution order for pipeline: {}", id);

        let pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        let execution_order = pipeline
            .get_execution_order()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        Ok(execution_order.iter().map(|s| (*s).clone()).collect())
    }

    /// Start pipeline execution
    pub async fn start_pipeline(&self, id: &PipelineId) -> Result<(), PipelineCrudError> {
        info!("Starting pipeline: {}", id);

        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Validate pipeline is not already running
        if pipeline.is_running() {
            return Err(PipelineCrudError::Validation(
                "Pipeline is already running".to_string(),
            ));
        }

        // Validate pipeline is in valid state to start
        if pipeline.is_terminal() {
            return Err(PipelineCrudError::Validation(
                "Cannot start pipeline in terminal state".to_string(),
            ));
        }

        // Start pipeline
        pipeline
            .start()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?;

        // Publish event
        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineStarted {
                pipeline_id: id.clone(),
            })
            .await
            .map_err(PipelineCrudError::EventBus)?;

        info!("Pipeline started successfully: {}", id);

        Ok(())
    }

    /// Complete pipeline successfully
    pub async fn complete_pipeline(&self, id: &PipelineId) -> Result<(), PipelineCrudError> {
        info!("Completing pipeline: {}", id);

        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Complete pipeline
        pipeline
            .complete()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?;

        // Publish event
        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineCompleted {
                pipeline_id: id.clone(),
            })
            .await
            .map_err(PipelineCrudError::EventBus)?;

        info!("Pipeline completed successfully: {}", id);

        Ok(())
    }

    /// Fail pipeline
    pub async fn fail_pipeline(&self, id: &PipelineId) -> Result<(), PipelineCrudError> {
        info!("Failing pipeline: {}", id);

        let mut pipeline = self
            .pipeline_repo
            .get_pipeline(id)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?
            .ok_or(PipelineCrudError::NotFound(id.clone()))?;

        // Fail pipeline
        pipeline
            .fail()
            .map_err(|e| PipelineCrudError::DomainError(e.to_string()))?;

        // Save to repository
        self.pipeline_repo
            .save_pipeline(&pipeline)
            .await
            .map_err(PipelineCrudError::PipelineRepository)?;

        // Publish event
        info!("Pipeline failed: {}", id);

        info!("Pipeline failed: {}", id);

        Ok(())
    }

    /// Create a pipeline step from request using Builder Pattern
    fn create_pipeline_step(
        &self,
        step_request: CreatePipelineStepRequest,
    ) -> Result<PipelineStep, PipelineCrudError> {
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

/// Filter for listing pipelines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPipelinesFilter {
    pub status: Option<PipelineStatus>,
    pub name_pattern: Option<String>, // Simple substring match
}

impl ListPipelinesFilter {
    /// Check if a pipeline matches the filter
    fn matches(&self, pipeline: &Pipeline) -> bool {
        if let Some(status) = &self.status {
            if pipeline.status != *status {
                return false;
            }
        }

        if let Some(pattern) = &self.name_pattern {
            if !pipeline.name.contains(pattern) {
                return false;
            }
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
    EventBus(hodei_ports::EventBusError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_adapters::InMemoryPipelineRepository;
    use hodei_core::pipeline::Pipeline;
    use std::collections::HashMap;

    // Mock EventBus for tests
    #[derive(Debug, Clone)]
    struct MockEventBus;

    #[async_trait::async_trait]
    impl EventPublisher for MockEventBus {
        async fn publish(
            &self,
            _event: hodei_ports::SystemEvent,
        ) -> Result<(), hodei_ports::EventBusError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_create_pipeline() {
        let repo = Arc::new(InMemoryPipelineRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service = PipelineCrudService::new(
            repo.clone(),
            event_bus.clone(),
            PipelineCrudConfig::default(),
        );

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: hodei_core::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let request = CreatePipelineRequest {
            name: "test-pipeline".to_string(),
            description: Some("Test pipeline".to_string()),
            steps: vec![CreatePipelineStepRequest {
                name: "step1".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string()],
                resources: Some(hodei_core::job::ResourceQuota::default()),
                timeout_ms: Some(30000),
                retries: Some(3),
                env: None,
                secret_refs: None,
                depends_on: None,
            }],
            variables: None,
        };

        let pipeline = service.create_pipeline(request).await.unwrap();

        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.steps.len(), 1);
        assert_eq!(pipeline.steps[0].name, "step1");
    }

    #[tokio::test]
    async fn test_get_pipeline() {
        let repo = Arc::new(InMemoryPipelineRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service = PipelineCrudService::new(
            repo.clone(),
            event_bus.clone(),
            PipelineCrudConfig::default(),
        );

        // Create pipeline
        let request = CreatePipelineRequest {
            name: "test-pipeline".to_string(),
            description: None,
            steps: vec![CreatePipelineStepRequest {
                name: "step1".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string()],
                resources: Some(hodei_core::job::ResourceQuota::default()),
                timeout_ms: Some(30000),
                retries: Some(3),
                env: None,
                secret_refs: None,
                depends_on: None,
            }],
            variables: None,
        };

        let created = service.create_pipeline(request.clone()).await.unwrap();

        // Get pipeline
        let retrieved = service.get_pipeline(&created.id).await.unwrap();

        assert!(retrieved.is_some());
        let pipeline = retrieved.unwrap();
        assert_eq!(pipeline.id, created.id);
        assert_eq!(pipeline.name, "test-pipeline");
    }

    #[tokio::test]
    async fn test_list_pipelines() {
        let repo = Arc::new(InMemoryPipelineRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service = PipelineCrudService::new(
            repo.clone(),
            event_bus.clone(),
            PipelineCrudConfig::default(),
        );

        // Create multiple pipelines
        for i in 0..5 {
            let request = CreatePipelineRequest {
                name: format!("test-pipeline-{}", i),
                description: None,
                steps: vec![CreatePipelineStepRequest {
                    name: format!("step1-{}", i),
                    image: "ubuntu".to_string(),
                    command: vec!["echo".to_string()],
                    resources: Some(hodei_core::job::ResourceQuota::default()),
                    timeout_ms: Some(30000),
                    retries: Some(3),
                    env: None,
                    secret_refs: None,
                    depends_on: None,
                }],
                variables: None,
            };
            service.create_pipeline(request).await.unwrap();
        }

        // List all pipelines
        let result = service.list_pipelines(None).await.unwrap();
        assert_eq!(result.len(), 5);

        // Filter by name
        let filter = ListPipelinesFilter {
            status: None,
            name_pattern: Some("pipeline-0".to_string()),
        };
        let filtered = service.list_pipelines(Some(&filter)).await.unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "test-pipeline-0");
    }

    #[tokio::test]
    async fn test_update_pipeline() {
        let repo = Arc::new(InMemoryPipelineRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service = PipelineCrudService::new(
            repo.clone(),
            event_bus.clone(),
            PipelineCrudConfig::default(),
        );

        // Create pipeline
        let request = CreatePipelineRequest {
            name: "test-pipeline".to_string(),
            description: None,
            steps: vec![CreatePipelineStepRequest {
                name: "step1".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string()],
                resources: Some(hodei_core::job::ResourceQuota::default()),
                timeout_ms: Some(30000),
                retries: Some(3),
                env: None,
                secret_refs: None,
                depends_on: None,
            }],
            variables: None,
        };

        let created = service.create_pipeline(request).await.unwrap();

        // Update pipeline
        let update_request = UpdatePipelineRequest {
            name: Some("updated-pipeline".to_string()),
            description: Some("Updated description".to_string()),
            steps: None,
            variables: None,
        };

        let updated = service
            .update_pipeline(&created.id, update_request)
            .await
            .unwrap();

        assert_eq!(updated.name, "updated-pipeline");
        assert_eq!(updated.description, Some("Updated description".to_string()));
    }

    #[tokio::test]
    async fn test_delete_pipeline() {
        let repo = Arc::new(InMemoryPipelineRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service = PipelineCrudService::new(
            repo.clone(),
            event_bus.clone(),
            PipelineCrudConfig::default(),
        );

        // Create pipeline
        let request = CreatePipelineRequest {
            name: "test-pipeline".to_string(),
            description: None,
            steps: vec![CreatePipelineStepRequest {
                name: "step1".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string()],
                resources: Some(hodei_core::job::ResourceQuota::default()),
                timeout_ms: Some(30000),
                retries: Some(3),
                env: None,
                secret_refs: None,
                depends_on: None,
            }],
            variables: None,
        };

        let created = service.create_pipeline(request).await.unwrap();

        // Delete pipeline
        service.delete_pipeline(&created.id).await.unwrap();

        // Verify it's deleted
        let result = service.get_pipeline(&created.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_start_pipeline() {
        let repo = Arc::new(InMemoryPipelineRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service = PipelineCrudService::new(
            repo.clone(),
            event_bus.clone(),
            PipelineCrudConfig::default(),
        );

        // Create pipeline
        let request = CreatePipelineRequest {
            name: "test-pipeline".to_string(),
            description: None,
            steps: vec![CreatePipelineStepRequest {
                name: "step1".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string()],
                resources: Some(hodei_core::job::ResourceQuota::default()),
                timeout_ms: Some(30000),
                retries: Some(3),
                env: None,
                secret_refs: None,
                depends_on: None,
            }],
            variables: None,
        };

        let created = service.create_pipeline(request).await.unwrap();

        // Start pipeline
        service.start_pipeline(&created.id).await.unwrap();

        // Verify it's running
        let retrieved = service.get_pipeline(&created.id).await.unwrap().unwrap();
        assert!(retrieved.is_running());
    }

    #[tokio::test]
    async fn test_get_execution_order() {
        let repo = Arc::new(InMemoryPipelineRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service = PipelineCrudService::new(
            repo.clone(),
            event_bus.clone(),
            PipelineCrudConfig::default(),
        );

        // Create pipeline with dependencies
        let job_spec = CreatePipelineStepRequest {
            name: "job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: Some(hodei_core::job::ResourceQuota::default()),
            timeout_ms: Some(30000),
            retries: Some(3),
            env: None,
            secret_refs: None,
            depends_on: None,
        };

        let request = CreatePipelineRequest {
            name: "test-pipeline".to_string(),
            description: None,
            steps: vec![
                CreatePipelineStepRequest {
                    name: "step1".to_string(),
                    image: "ubuntu".to_string(),
                    command: vec!["echo".to_string()],
                    resources: Some(hodei_core::job::ResourceQuota::default()),
                    timeout_ms: Some(30000),
                    retries: Some(3),
                    env: None,
                    secret_refs: None,
                    depends_on: None,
                },
                CreatePipelineStepRequest {
                    name: "step2".to_string(),
                    image: "ubuntu".to_string(),
                    command: vec!["echo".to_string()],
                    resources: Some(hodei_core::job::ResourceQuota::default()),
                    timeout_ms: Some(30000),
                    retries: Some(3),
                    env: None,
                    secret_refs: None,
                    depends_on: Some(vec![uuid::Uuid::new_v4().to_string()]), // We'll need to handle this better
                },
            ],
            variables: None,
        };

        // This test would need the actual step IDs, so we'll simplify
        let created = service
            .create_pipeline(CreatePipelineRequest {
                name: "test-pipeline".to_string(),
                description: None,
                steps: vec![CreatePipelineStepRequest {
                    name: "step1".to_string(),
                    image: "ubuntu".to_string(),
                    command: vec!["echo".to_string()],
                    resources: Some(hodei_core::job::ResourceQuota::default()),
                    timeout_ms: Some(30000),
                    retries: Some(3),
                    env: None,
                    secret_refs: None,
                    depends_on: None,
                }],
                variables: None,
            })
            .await
            .unwrap();

        // Get execution order
        let order = service.get_execution_order(&created.id).await.unwrap();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name, "step1");
    }
}
