//! State Machine for Scheduling Process
//!
//! This module eliminates Temporal Coupling in the scheduler by making state
//! transitions explicit and validated, allowing for better testability and
//! flexibility in the scheduling process.

use crate::scheduler::{
    ClusterState, ResourceUsage, SchedulerConfig, SchedulerError, SchedulerModule, Worker,
    WorkerNode,
};
use hodei_core::{Job, JobId};
use std::sync::Arc;

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
    ) -> Result<(), SchedulerError>
    where
        R: hodei_ports::JobRepository + Send + Sync + 'static,
        E: hodei_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_ports::WorkerRepository + Send + Sync + 'static,
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
    ) -> Result<(), SchedulerError>
    where
        R: hodei_ports::JobRepository + Send + Sync + 'static,
        E: hodei_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_ports::WorkerRepository + Send + Sync + 'static,
    {
        self.validate_state(&[SchedulingState::Matching])?;

        if let Some(job) = &self.context.job {
            if !self.context.eligible_workers.is_empty() {
                let selected_worker = scheduler
                    .select_best_worker(&self.context.eligible_workers, job)
                    .await?;
                self.context.selected_worker = Some(selected_worker);
            }
        }

        self.current_state = SchedulingState::Optimizing;
        Ok(())
    }

    /// Transition to Optimizing state
    pub async fn optimize<R, E, W, WR>(
        &mut self,
        scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<(), SchedulerError>
    where
        R: hodei_ports::JobRepository + Send + Sync + 'static,
        E: hodei_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_ports::WorkerRepository + Send + Sync + 'static,
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
    ) -> Result<(), SchedulerError>
    where
        R: hodei_ports::JobRepository + Send + Sync + 'static,
        E: hodei_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_ports::WorkerRepository + Send + Sync + 'static,
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
                        hodei_core::JobState::PENDING,
                        hodei_core::JobState::SCHEDULED,
                    )
                    .await
                    .map_err(SchedulerError::JobRepository)?;

                // Assign job to worker
                scheduler
                    .worker_client
                    .assign_job(&worker.id, &job.id, &job.spec)
                    .await
                    .map_err(SchedulerError::WorkerClient)?;

                self.current_state = SchedulingState::Completed;
            } else {
                return Err(SchedulerError::NoEligibleWorkers);
            }
        } else {
            return Err(SchedulerError::NoEligibleWorkers);
        }

        Ok(())
    }

    /// Complete the scheduling cycle
    pub async fn complete<R, E, W, WR>(
        &mut self,
        scheduler: &SchedulerModule<R, E, W, WR>,
    ) -> Result<(), SchedulerError>
    where
        R: hodei_ports::JobRepository + Send + Sync + 'static,
        E: hodei_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_ports::WorkerRepository + Send + Sync + 'static,
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
    ) -> Result<Option<Worker>, SchedulerError>
    where
        R: hodei_ports::JobRepository + Send + Sync + 'static,
        E: hodei_ports::EventPublisher + Send + Sync + 'static,
        W: hodei_ports::WorkerClient + Send + Sync + 'static,
        WR: hodei_ports::WorkerRepository + Send + Sync + 'static,
    {
        // Can only match if we have a job and are in valid state
        if self.context.job.is_none() {
            return Err(SchedulerError::Validation(
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

    fn validate_state(&self, allowed_states: &[SchedulingState]) -> Result<(), SchedulerError> {
        if !allowed_states.contains(&self.current_state) {
            return Err(SchedulerError::Validation(format!(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{SchedulerBuilder, SchedulerConfig};
    use hodei_core::Worker;
    use hodei_core::{Job, JobId, JobSpec, ResourceQuota};
    use hodei_ports::{
        EventPublisher, JobRepository, JobRepositoryError, WorkerClient, WorkerRepository,
    };
    use hodei_shared_types::{WorkerCapabilities, WorkerId, WorkerStatus};
    use std::sync::Arc;

    // Mock implementations
    #[derive(PartialEq, Clone)]
    struct MockJobRepository;
    #[derive(PartialEq, Clone)]
    struct MockEventBus;
    #[derive(PartialEq, Clone)]
    struct MockWorkerClient;
    #[derive(PartialEq, Clone)]
    struct MockWorkerRepository;

    #[async_trait::async_trait]
    impl JobRepository for MockJobRepository {
        async fn save_job(&self, _job: &Job) -> Result<(), JobRepositoryError> {
            Ok(())
        }

        async fn get_job(&self, _id: &JobId) -> Result<Option<Job>, JobRepositoryError> {
            Ok(None)
        }

        async fn get_pending_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
            Ok(vec![])
        }

        async fn get_running_jobs(&self) -> Result<Vec<Job>, JobRepositoryError> {
            Ok(vec![])
        }

        async fn delete_job(&self, _id: &JobId) -> Result<(), JobRepositoryError> {
            Ok(())
        }

        async fn compare_and_swap_status(
            &self,
            _id: &JobId,
            _expected: &str,
            _new: &str,
        ) -> Result<bool, JobRepositoryError> {
            Ok(true)
        }
    }

    #[async_trait::async_trait]
    impl EventPublisher for MockEventBus {
        async fn publish(
            &self,
            _event: hodei_ports::SystemEvent,
        ) -> Result<(), hodei_ports::EventBusError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl WorkerClient for MockWorkerClient {
        async fn assign_job(
            &self,
            _worker_id: &WorkerId,
            _job_id: &JobId,
            _job_spec: &JobSpec,
        ) -> Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }

        async fn cancel_job(
            &self,
            _worker_id: &WorkerId,
            _job_id: &JobId,
        ) -> Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }

        async fn get_worker_status(
            &self,
            _worker_id: &WorkerId,
        ) -> Result<WorkerStatus, hodei_ports::WorkerClientError> {
            Ok(WorkerStatus {
                worker_id: WorkerId::new(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().into(),
            })
        }

        async fn send_heartbeat(
            &self,
            _worker_id: &WorkerId,
        ) -> Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl WorkerRepository for MockWorkerRepository {
        async fn save_worker(
            &self,
            _worker: &Worker,
        ) -> Result<(), hodei_ports::WorkerRepositoryError> {
            Ok(())
        }

        async fn get_worker(
            &self,
            _id: &WorkerId,
        ) -> Result<Option<Worker>, hodei_ports::WorkerRepositoryError> {
            Ok(None)
        }

        async fn get_all_workers(&self) -> Result<Vec<Worker>, hodei_ports::WorkerRepositoryError> {
            Ok(vec![])
        }

        async fn delete_worker(
            &self,
            _id: &WorkerId,
        ) -> Result<(), hodei_ports::WorkerRepositoryError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_state_machine_initial_state() {
        let machine = SchedulingStateMachine::new();
        assert_eq!(*machine.get_state(), SchedulingState::Idle);
        assert!(machine.context.job.is_none());
    }

    #[tokio::test]
    async fn test_state_machine_set_job() {
        let mut machine = SchedulingStateMachine::new();
        let job = create_test_job();
        machine.set_job(job.clone());

        assert!(machine.context.job.is_some());
        assert_eq!(machine.context.job, Some(job));
    }

    #[tokio::test]
    async fn test_state_machine_reset() {
        let mut machine = SchedulingStateMachine::new();
        let job = create_test_job();
        machine.set_job(job);

        // Verify state changed
        assert!(machine.context.job.is_some());

        // Reset
        machine.reset();

        // Verify back to initial state
        assert_eq!(*machine.get_state(), SchedulingState::Idle);
        assert!(machine.context.job.is_none());
    }

    fn create_test_job() -> Job {
        Job::new(
            JobId::new(),
            JobSpec {
                name: "test-job".to_string(),
                image: "ubuntu:latest".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 300000,
                retries: 3,
                env: std::collections::HashMap::new(),
                secret_refs: Vec::new(),
            },
        )
        .unwrap()
        .with_description("Test job")
        .with_tenant("test")
    }
}
