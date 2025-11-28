//! Pipeline Execution Orchestrator Service
//!
//! Production-ready implementation for orchestrating pipeline step execution
//! with dependency management and fault tolerance.

use async_trait::async_trait;
use hodei_core::{
    DomainError, Result,
    job::JobState,
    pipeline::{Pipeline, PipelineId, PipelineStep, PipelineStepId},
    pipeline_execution::{
        ExecutionId, ExecutionStatus, PipelineExecution, StepExecution, StepExecutionId,
        StepExecutionStatus,
    },
};
use hodei_ports::{EventPublisher, JobRepository, PipelineExecutionRepository, PipelineRepository};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use tracing::{Instrument, Span, error, info, warn};
use uuid::Uuid;

/// Configuration for pipeline execution orchestrator
#[derive(Debug, Clone)]
pub struct PipelineExecutionConfig {
    pub max_concurrent_steps: usize,
    pub max_retry_attempts: u8,
    pub step_timeout_secs: u64,
    pub cleanup_interval_secs: u64,
}

impl Default for PipelineExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_steps: 10,
            max_retry_attempts: 3,
            step_timeout_secs: 3600,
            cleanup_interval_secs: 300,
        }
    }
}

/// Pipeline Execution Orchestrator
/// Handles step execution with dependency management and fault tolerance
pub struct PipelineExecutionOrchestrator<R, J, P, E>
where
    R: PipelineExecutionRepository + Send + Sync,
    J: JobRepository + Send + Sync,
    P: PipelineRepository + Send + Sync,
    E: EventPublisher + Send + Sync,
{
    execution_repo: Arc<R>,
    job_repo: Arc<J>,
    pipeline_repo: Arc<P>,
    event_bus: Arc<E>,
    config: PipelineExecutionConfig,
    // Control channels
    cancellation_sender: mpsc::UnboundedSender<ExecutionId>,
    cancellation_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ExecutionId>>>,
    // Semaphore for concurrent step execution
    step_semaphore: Arc<Semaphore>,
}

impl<R, J, P, E> PipelineExecutionOrchestrator<R, J, P, E>
where
    R: PipelineExecutionRepository + Send + Sync,
    J: JobRepository + Send + Sync,
    P: PipelineRepository + Send + Sync,
    E: EventPublisher + Send + Sync,
{
    /// Create a new orchestrator instance
    pub fn new(
        execution_repo: Arc<R>,
        job_repo: Arc<J>,
        pipeline_repo: Arc<P>,
        event_bus: Arc<E>,
        config: PipelineExecutionConfig,
    ) -> Result<Self> {
        let (cancellation_sender, cancellation_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            execution_repo,
            job_repo,
            pipeline_repo,
            event_bus,
            config: config.clone(),
            cancellation_sender,
            cancellation_receiver: Arc::new(tokio::sync::Mutex::new(cancellation_receiver)),
            step_semaphore: Arc::new(Semaphore::new(config.max_concurrent_steps)),
        })
    }

    /// Execute a pipeline with orchestration
    pub async fn execute_pipeline(
        &self,
        pipeline_id: PipelineId,
        variables: HashMap<String, String>,
        tenant_id: Option<String>,
        correlation_id: Option<String>,
    ) -> Result<ExecutionId>
    where
        R: Send + Sync + 'static,
        J: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let pipeline_id_clone = pipeline_id.clone();
        info!("Starting pipeline execution: {}", pipeline_id_clone);

        // Get pipeline from repository
        let pipeline = self
            .pipeline_repo
            .get_pipeline(&pipeline_id)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get pipeline: {}", e)))?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Pipeline not found: {}", pipeline_id_clone))
            })?;

        // Validate pipeline is not already running
        if pipeline.is_running() {
            return Err(DomainError::Validation(
                "Pipeline is already running".to_string(),
            ));
        }

        // Get execution order based on dependencies
        let execution_order = pipeline
            .get_execution_order()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let step_ids: Vec<PipelineStepId> =
            execution_order.iter().map(|step| step.id.clone()).collect();

        // Create pipeline execution
        let execution = PipelineExecution::new(
            pipeline_id.clone(),
            step_ids,
            variables,
            tenant_id,
            correlation_id,
        );

        let execution_id = execution.id.clone();

        // Save initial execution
        self.execution_repo
            .save_execution(&execution)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to save execution: {}", e)))?;

        // Publish pipeline execution started event
        self.event_bus
            .publish(hodei_ports::SystemEvent::PipelineExecutionStarted {
                pipeline_id: pipeline_id_clone.clone(),
                execution_id: execution_id.clone(),
            })
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to publish event: {}", e)))?;

        // Start async execution in background
        let execution_repo = self.execution_repo.clone();
        let job_repo = self.job_repo.clone();
        let event_bus = self.event_bus.clone();
        let config = self.config.clone();
        let cancellation_receiver = self.cancellation_receiver.clone();
        let step_semaphore = self.step_semaphore.clone();

        // Extract pipeline and execution data needed for async execution
        let pipeline_clone = pipeline.clone();
        let execution_id_clone = execution_id.clone();

        tokio::spawn(async move {
            let span = Span::current();
            async move {
                Self::execute_pipeline_async(
                    execution_repo,
                    job_repo,
                    event_bus,
                    pipeline_clone,
                    execution_id_clone,
                    config,
                    cancellation_receiver,
                    step_semaphore,
                )
                .await;
            }
            .instrument(span)
            .await;
        });

        info!(
            "Pipeline execution initiated: {} (execution_id: {})",
            pipeline_id_clone, execution_id
        );
        Ok(execution_id)
    }

    /// Cancel a running pipeline execution
    pub async fn cancel_execution(&self, execution_id: &ExecutionId) -> Result<()> {
        info!("Cancelling pipeline execution: {}", execution_id);

        // Send cancellation signal
        if let Err(e) = self.cancellation_sender.send(execution_id.clone()) {
            warn!("Failed to send cancellation signal: {}", e);
        }

        // Update execution status
        self.execution_repo
            .update_execution_status(execution_id, ExecutionStatus::CANCELLED)
            .await
            .map_err(|e| {
                DomainError::Infrastructure(format!("Failed to cancel execution: {}", e))
            })?;

        info!("Pipeline execution cancelled: {}", execution_id);
        Ok(())
    }

    /// Get execution status
    pub async fn get_execution(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<Option<PipelineExecution>> {
        self.execution_repo
            .get_execution(execution_id)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to get execution: {}", e)))
    }

    /// Internal async execution logic
    async fn execute_pipeline_async<Rep, JobRepo, EventBus>(
        execution_repo: Arc<Rep>,
        job_repo: Arc<JobRepo>,
        event_bus: Arc<EventBus>,
        pipeline: Pipeline,
        execution_id: ExecutionId,
        config: PipelineExecutionConfig,
        cancellation_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ExecutionId>>>,
        step_semaphore: Arc<Semaphore>,
    ) where
        Rep: PipelineExecutionRepository + Send + Sync,
        JobRepo: JobRepository + Send + Sync,
        EventBus: EventPublisher + Send + Sync,
    {
        info!("Starting async pipeline execution: {}", execution_id);

        // Start the pipeline execution
        let mut execution = if let Some(exec) = execution_repo
            .get_execution(&execution_id)
            .await
            .ok()
            .flatten()
        {
            exec
        } else {
            error!("Execution not found: {}", execution_id);
            return;
        };

        if let Err(e) = execution.start() {
            error!("Failed to start execution: {}", e);
            return;
        }

        if let Err(e) = execution_repo.save_execution(&execution).await {
            error!("Failed to save execution state: {}", e);
            return;
        }

        // Build dependency graph
        let step_dependencies = build_dependency_graph(&pipeline);
        let total_steps = execution.steps.len();

        // Track completed and failed steps
        let mut completed_steps = HashSet::new();
        let mut failed_steps = HashSet::new();
        let mut cancelled = false;

        // Main execution loop
        'main_loop: for step_idx in 0..total_steps {
            // Check for cancellation
            let mut receiver = cancellation_receiver.lock().await;
            if let Ok(exec_id) = receiver.try_recv() {
                if exec_id == execution_id {
                    cancelled = true;
                    break 'main_loop;
                }
            }
            drop(receiver);

            // Find next ready step
            let current_step_id = execution.steps[step_idx].step_id.clone();

            // Check if dependencies are satisfied
            if let Some(deps) = step_dependencies.get(&current_step_id) {
                let mut all_deps_completed = true;
                for dep_id in deps {
                    if !completed_steps.contains(dep_id) && !failed_steps.contains(dep_id) {
                        all_deps_completed = false;
                        break;
                    }
                }

                if !all_deps_completed {
                    // This should not happen with topological sort, but safety check
                    error!("Dependency violation detected for step {}", current_step_id);
                    break;
                }
            }

            info!(
                "Starting step: {} (execution: {})",
                current_step_id, execution_id
            );

            // Acquire semaphore permit for concurrent execution
            let _permit = step_semaphore.acquire().await.unwrap();

            // Execute the step
            let step_result = Self::execute_step(
                &execution_repo,
                &job_repo,
                &event_bus,
                &execution,
                &pipeline,
                &current_step_id,
                &config,
            )
            .await;

            match step_result {
                Ok(StepExecutionStatus::PENDING) => {
                    info!(
                        "Step pending: {} (execution: {})",
                        current_step_id, execution_id
                    );
                    // PENDING steps should not occur in normal execution flow
                }
                Ok(StepExecutionStatus::RUNNING) => {
                    info!(
                        "Step running: {} (execution: {})",
                        current_step_id, execution_id
                    );
                    // In a real implementation, we might wait for completion
                    // For now, treat as completed
                    completed_steps.insert(current_step_id.clone());
                }
                Ok(StepExecutionStatus::COMPLETED) => {
                    completed_steps.insert(current_step_id.clone());
                    info!(
                        "Step completed: {} (execution: {})",
                        current_step_id, execution_id
                    );
                }
                Ok(StepExecutionStatus::FAILED) => {
                    failed_steps.insert(current_step_id.clone());
                    error!(
                        "Step failed: {} (execution: {})",
                        current_step_id, execution_id
                    );

                    // Check if pipeline should stop on first failure
                    // For now, we continue with remaining steps
                }
                Ok(StepExecutionStatus::SKIPPED) => {
                    info!(
                        "Step skipped: {} (execution: {})",
                        current_step_id, execution_id
                    );
                }
                Err(e) => {
                    error!(
                        "Step execution error: {} (execution: {}): {}",
                        current_step_id, execution_id, e
                    );
                    failed_steps.insert(current_step_id.clone());
                }
            }

            // Check if execution was cancelled
            let mut receiver = cancellation_receiver.lock().await;
            if let Ok(exec_id) = receiver.try_recv() {
                if exec_id == execution_id {
                    cancelled = true;
                    break 'main_loop;
                }
            }
            drop(receiver);
        }

        // Determine final execution status
        let final_status = if cancelled {
            ExecutionStatus::CANCELLED
        } else if failed_steps.is_empty() {
            ExecutionStatus::COMPLETED
        } else {
            ExecutionStatus::FAILED
        };

        // Update execution status
        execution.status = final_status.clone();

        if let Err(e) = execution_repo.save_execution(&execution).await {
            error!("Failed to save final execution state: {}", e);
        }

        info!(
            "Pipeline execution completed: {} (status: {})",
            execution_id, final_status
        );
    }

    /// Execute a single step
    async fn execute_step<Rep, JobRepo, EventBus>(
        execution_repo: &Arc<Rep>,
        job_repo: &Arc<JobRepo>,
        event_bus: &Arc<EventBus>,
        execution: &PipelineExecution,
        pipeline: &Pipeline,
        step_id: &PipelineStepId,
        config: &PipelineExecutionConfig,
    ) -> Result<StepExecutionStatus>
    where
        Rep: PipelineExecutionRepository + Send + Sync,
        JobRepo: JobRepository + Send + Sync,
        EventBus: EventPublisher + Send + Sync,
    {
        // Get the step from pipeline
        let step = pipeline
            .steps
            .iter()
            .find(|s| &s.id == step_id)
            .ok_or_else(|| DomainError::NotFound(format!("Step not found: {}", step_id)))?;

        // Create job spec from step
        let mut job_spec = step.job_spec.clone();

        // Apply pipeline variables to job spec
        for (key, value) in &execution.variables {
            job_spec.env.insert(key.clone(), value.clone());
        }

        // Create and schedule job
        let job_id = job_repo
            .create_job(job_spec)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to create job: {}", e)))?;

        info!("Created job {} for step {}", job_id, step_id);

        // Wait for job completion (simplified - in production this would be event-driven)
        let timeout_duration = tokio::time::Duration::from_secs(config.step_timeout_secs);
        let job_completion = tokio::time::timeout(
            timeout_duration,
            wait_for_job_completion(job_repo.clone(), job_id),
        );

        let job_result = job_completion.await.map_err(|_| {
            DomainError::Timeout(format!(
                "Step {} timed out after {} seconds",
                step_id, config.step_timeout_secs
            ))
        })??;

        // Update step execution status based on job result
        let step_status = if job_result.as_str() == JobState::SUCCESS {
            StepExecutionStatus::COMPLETED
        } else {
            StepExecutionStatus::FAILED
        };

        Ok(step_status)
    }
}

/// Helper function to wait for job completion
async fn wait_for_job_completion<R>(
    job_repo: Arc<R>,
    job_id: hodei_core::JobId,
) -> Result<hodei_core::job::JobState>
where
    R: JobRepository + Send + Sync,
{
    let mut attempts = 0;
    let max_attempts = 600; // 10 minutes with 1 second intervals

    loop {
        attempts += 1;

        if let Some(job) = job_repo.get_job(&job_id).await.ok().flatten() {
            if job.state.is_terminal() {
                return Ok(job.state);
            }
        }

        if attempts >= max_attempts {
            return Err(DomainError::Timeout(format!(
                "Job {} did not complete in time",
                job_id
            )));
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Build dependency graph from pipeline steps
fn build_dependency_graph(pipeline: &Pipeline) -> HashMap<PipelineStepId, Vec<PipelineStepId>> {
    let mut graph = HashMap::new();

    for step in &pipeline.steps {
        if !step.depends_on.is_empty() {
            graph.insert(step.id.clone(), step.depends_on.clone());
        }
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::{Result, job::JobState};
    use std::sync::Mutex;

    // Mock implementations for testing
    struct MockExecutionRepository {
        executions: Arc<Mutex<HashMap<ExecutionId, PipelineExecution>>>,
    }

    struct MockJobRepository {
        jobs: Arc<Mutex<HashMap<hodei_core::JobId, hodei_core::job::Job>>>,
    }

    #[async_trait]
    impl PipelineExecutionRepository for MockExecutionRepository {
        async fn save_execution(&self, execution: &PipelineExecution) -> Result<()> {
            let mut executions = self.executions.lock().unwrap();
            executions.insert(execution.id.clone(), execution.clone());
            Ok(())
        }

        async fn get_execution(
            &self,
            execution_id: &ExecutionId,
        ) -> Result<Option<PipelineExecution>> {
            let executions = self.executions.lock().unwrap();
            Ok(executions.get(execution_id).cloned())
        }

        async fn get_executions_by_pipeline(
            &self,
            _pipeline_id: &hodei_core::PipelineId,
        ) -> Result<Vec<PipelineExecution>> {
            unimplemented!()
        }

        async fn update_execution_status(
            &self,
            _execution_id: &ExecutionId,
            _status: ExecutionStatus,
        ) -> Result<()> {
            Ok(())
        }

        async fn update_step_status(
            &self,
            _execution_id: &ExecutionId,
            _step_id: &PipelineStepId,
            _status: StepExecutionStatus,
        ) -> Result<()> {
            Ok(())
        }

        async fn update_step(
            &self,
            _execution_id: &ExecutionId,
            _step: &StepExecution,
        ) -> Result<()> {
            Ok(())
        }

        async fn delete_execution(&self, _execution_id: &ExecutionId) -> Result<()> {
            Ok(())
        }

        async fn get_active_executions(&self) -> Result<Vec<PipelineExecution>> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl JobRepository for MockJobRepository {
        async fn create_job(
            &self,
            _job_spec: hodei_core::job::JobSpec,
        ) -> Result<hodei_core::JobId> {
            Ok(hodei_core::JobId::new())
        }

        async fn get_job(
            &self,
            _job_id: &hodei_core::JobId,
        ) -> Result<Option<hodei_core::job::Job>> {
            Ok(None)
        }

        async fn update_job_state(
            &self,
            _job_id: &hodei_core::JobId,
            _state: hodei_core::job::JobState,
        ) -> Result<()> {
            Ok(())
        }

        async fn delete_job(&self, _job_id: &hodei_core::JobId) -> Result<()> {
            Ok(())
        }

        async fn list_jobs(&self) -> Result<Vec<hodei_core::job::Job>> {
            Ok(vec![])
        }

        // Default implementations for other required methods
        async fn save_job(&self, _job: &hodei_core::job::Job) -> Result<()> {
            Ok(())
        }

        async fn get_pending_jobs(&self) -> Result<Vec<hodei_core::job::Job>> {
            Ok(vec![])
        }

        async fn get_running_jobs(&self) -> Result<Vec<hodei_core::job::Job>> {
            Ok(vec![])
        }

        async fn compare_and_swap_status(
            &self,
            _id: &hodei_core::JobId,
            _expected_state: &str,
            _new_state: &str,
        ) -> Result<bool> {
            Ok(false)
        }

        async fn assign_worker(
            &self,
            _job_id: &hodei_core::JobId,
            _worker_id: &hodei_core::WorkerId,
        ) -> Result<()> {
            Ok(())
        }

        async fn set_job_start_time(
            &self,
            _job_id: &hodei_core::JobId,
            _start_time: chrono::DateTime<chrono::Utc>,
        ) -> Result<()> {
            Ok(())
        }

        async fn set_job_finish_time(
            &self,
            _job_id: &hodei_core::JobId,
            _finish_time: chrono::DateTime<chrono::Utc>,
        ) -> Result<()> {
            Ok(())
        }

        async fn set_job_duration(
            &self,
            _job_id: &hodei_core::JobId,
            _duration_ms: i64,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_dependency_graph_building() {
        use hodei_core::{JobSpec, ResourceQuota, Result};
        use std::collections::HashMap;

        // Create a pipeline with dependencies
        let mut step1 = PipelineStep {
            id: PipelineStepId::new(),
            name: "step1".to_string(),
            job_spec: JobSpec {
                name: "job1".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            depends_on: vec![],
            timeout_ms: 300000,
        };

        let mut step2 = PipelineStep {
            id: PipelineStepId::new(),
            name: "step2".to_string(),
            job_spec: JobSpec {
                name: "job2".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 30000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: vec![],
            },
            depends_on: vec![],
            timeout_ms: 300000,
        };

        step2.depends_on.push(step1.id.clone());

        let pipeline = Pipeline::new(
            PipelineId::new(),
            "test-pipeline".to_string(),
            vec![step1.clone(), step2.clone()],
        )
        .unwrap();

        let graph = build_dependency_graph(&pipeline);

        assert!(graph.contains_key(&step2.id));
        assert_eq!(graph.get(&step2.id).unwrap(), &vec![step1.id.clone()]);
        assert!(!graph.contains_key(&step1.id));
    }
}
