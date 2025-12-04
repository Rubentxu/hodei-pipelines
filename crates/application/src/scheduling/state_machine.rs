//! State Machine for Scheduling Process
//!
//! This module eliminates Temporal Coupling in the scheduler by making state
//! transitions explicit and validated, allowing for better testability and
//! flexibility in the scheduling process.

use crate::scheduling::{SchedulerModule, WorkerNode};
use hodei_pipelines_domain::{Job, Result, Worker};

/// Scheduling state to eliminate temporal coupling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulingState {
    Idle,
    Collecting,
    Matching,
    Optimizing,
    Committing,
    Completed,
    Failed(String),
}

/// Context for scheduling state machine
#[derive(Debug)]
pub struct SchedulingContext {
    pub job: Option<Job>,
    pub eligible_workers: Vec<Worker>,
    pub selected_worker: Option<Worker>,
    pub cluster_state: Vec<WorkerNode>,
}

impl Default for SchedulingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulingContext {
    pub fn new() -> Self {
        Self {
            job: None,
            eligible_workers: Vec::new(),
            selected_worker: None,
            cluster_state: Vec::new(),
        }
    }
}

/// State Machine for scheduling process
pub struct SchedulingStateMachine {
    current_state: SchedulingState,
    context: SchedulingContext,
}

impl SchedulingStateMachine {
    pub fn new() -> Self {
        SchedulingStateMachine {
            current_state: SchedulingState::Idle,
            context: SchedulingContext::new(),
        }
    }

    pub fn get_state(&self) -> &SchedulingState {
        &self.current_state
    }

    pub fn get_context(&self) -> &SchedulingContext {
        &self.context
    }

    /// Transition to Collecting state
    pub async fn collect<R, E, W, WR>(
        &mut self,
        scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<()>
    where
        R: hodei_pipelines_ports::JobRepository
            + hodei_pipelines_ports::PipelineRepository
            + hodei_pipelines_ports::RoleRepository
            + Send
            + Sync
            + 'static,
        E: hodei_pipelines_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_pipelines_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_pipelines_ports::WorkerRepository + Send + Sync + 'static,
    {
        self.validate_state(&[SchedulingState::Idle])?;

        self.current_state = SchedulingState::Collecting;

        // Collect cluster state
        let cluster_workers = scheduler.cluster_state.get_all_workers().await;
        self.context.cluster_state = cluster_workers;

        // If we have a job, collect eligible workers
        if let Some(job) = &self.context.job {
            let workers = scheduler.find_eligible_workers(job).await?;
            self.context.eligible_workers = workers;
        }

        self.current_state = SchedulingState::Matching;
        Ok(())
    }

    /// Transition to Matching state
    pub async fn match_jobs<R, E, W, WR>(
        &mut self,
        scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<()>
    where
        R: hodei_pipelines_ports::JobRepository
            + hodei_pipelines_ports::PipelineRepository
            + hodei_pipelines_ports::RoleRepository
            + Send
            + Sync
            + 'static,
        E: hodei_pipelines_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_pipelines_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_pipelines_ports::WorkerRepository + Send + Sync + 'static,
    {
        self.validate_state(&[SchedulingState::Matching])?;

        if let Some(job) = &self.context.job
            && !self.context.eligible_workers.is_empty()
        {
            let selected_worker = scheduler
                .select_best_worker(&self.context.eligible_workers, job)
                .await?;
            self.context.selected_worker = Some(selected_worker);
        }

        self.current_state = SchedulingState::Optimizing;
        Ok(())
    }

    /// Transition to Optimizing state
    pub async fn optimize<R, E, W, WR>(
        &mut self,
        _scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<()>
    where
        R: hodei_pipelines_ports::JobRepository
            + hodei_pipelines_ports::PipelineRepository
            + hodei_pipelines_ports::RoleRepository
            + Send
            + Sync
            + 'static,
        E: hodei_pipelines_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_pipelines_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_pipelines_ports::WorkerRepository + Send + Sync + 'static,
    {
        self.validate_state(&[SchedulingState::Optimizing])?;

        // In this implementation, optimization happens during worker selection
        // This state is kept for extensibility

        self.current_state = SchedulingState::Committing;
        Ok(())
    }

    /// Transition to Committing state
    pub async fn commit<R, E, W, WR>(
        &mut self,
        scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<()>
    where
        R: hodei_pipelines_ports::JobRepository
            + hodei_pipelines_ports::PipelineRepository
            + hodei_pipelines_ports::RoleRepository
            + Send
            + Sync
            + 'static,
        E: hodei_pipelines_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_pipelines_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_pipelines_ports::WorkerRepository + Send + Sync + 'static,
    {
        self.validate_state(&[SchedulingState::Committing])?;

        if let (Some(job), Some(worker)) = (&self.context.job, &self.context.selected_worker) {
            // Reserve worker
            if scheduler.reserve_worker(worker, &job.id).await? {
                // Update job state
                scheduler
                    .job_repo
                    .compare_and_swap_status(
                        &job.id,
                        &hodei_pipelines_domain::pipeline_execution::value_objects::job_definitions::JobState::Pending.as_str(),
                        &hodei_pipelines_domain::pipeline_execution::value_objects::job_definitions::JobState::Scheduled.as_str(),
                    )
                    .await
                    .map_err(|e| hodei_pipelines_domain::DomainError::Infrastructure(e.to_string()))?;

                // Assign job to worker
                scheduler
                    .worker_client
                    .assign_job(&worker.id, &job.id, &job.spec)
                    .await
                    .map_err(|e| hodei_pipelines_domain::DomainError::Infrastructure(e.to_string()))?;

                self.current_state = SchedulingState::Completed;
            } else {
                return Err(hodei_pipelines_domain::DomainError::Infrastructure(
                    "No eligible workers found".to_string(),
                ));
            }
        } else {
            return Err(hodei_pipelines_domain::DomainError::Infrastructure(
                "No eligible workers found".to_string(),
            ));
        }

        Ok(())
    }

    /// Complete the scheduling cycle
    pub async fn complete<R, E, W, WR>(
        &mut self,
        scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<()>
    where
        R: hodei_pipelines_ports::JobRepository
            + hodei_pipelines_ports::PipelineRepository
            + hodei_pipelines_ports::RoleRepository
            + Send
            + Sync
            + 'static,
        E: hodei_pipelines_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_pipelines_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_pipelines_ports::WorkerRepository + Send + Sync + 'static,
    {
        self.collect(scheduler).await?;
        self.match_jobs(scheduler).await?;
        self.optimize(scheduler).await?;
        self.commit(scheduler).await?;
        Ok(())
    }

    /// Execute only the matching phase (for testing or partial execution)
    pub async fn discover_matches<R, E, W, WR>(
        &mut self,
        scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<Option<Worker>>
    where
        R: hodei_pipelines_ports::JobRepository
            + hodei_pipelines_ports::PipelineRepository
            + hodei_pipelines_ports::RoleRepository
            + Send
            + Sync
            + 'static,
        E: hodei_pipelines_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_pipelines_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_pipelines_ports::WorkerRepository + Send + Sync + 'static,
    {
        // Can only match if we have a job and are in valid state
        if self.context.job.is_none() {
            return Err(hodei_pipelines_domain::DomainError::Validation(
                "No job set in context".to_string(),
            ));
        }

        // Validate we can reach Matching state
        self.validate_state(&[
            SchedulingState::Idle,
            SchedulingState::Collecting,
            SchedulingState::Matching,
        ])?;

        // Force collection if needed
        if matches!(self.current_state, SchedulingState::Idle) {
            self.collect(scheduler).await?;
        } else if matches!(self.current_state, SchedulingState::Collecting) {
            self.current_state = SchedulingState::Matching;
        }

        // Get matching workers
        self.match_jobs(scheduler).await?;

        Ok(self.context.selected_worker.clone())
    }

    fn validate_state(&self, allowed_states: &[SchedulingState]) -> Result<()> {
        if !allowed_states.contains(&self.current_state) {
            return Err(hodei_pipelines_domain::DomainError::Validation(format!(
                "Invalid state transition from {:?} to allowed states {:?}",
                self.current_state, allowed_states
            )));
        }
        Ok(())
    }

    /// Set job in context
    pub fn set_job(&mut self, job: Job) {
        self.context.job = Some(job);
    }

    /// Reset state machine to initial state
    pub fn reset(&mut self) {
        self.current_state = SchedulingState::Idle;
        self.context = SchedulingContext::new();
    }
}

impl Default for SchedulingStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
