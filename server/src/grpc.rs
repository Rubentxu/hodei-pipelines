use chrono;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{
    Stream, StreamExt,
    wrappers::{ReceiverStream, UnboundedReceiverStream},
};
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, warn};

use hwp_proto::{
    AgentMessage,
    AgentPayload,
    // Artifact upload types
    ArtifactChunk,
    AssignJobRequest,
    CancelJobRequest,
    Empty,
    FinalizeUploadRequest,
    FinalizeUploadResponse,
    InitiateUploadRequest,
    InitiateUploadResponse,
    JobAccepted,
    JobResult,
    LogEntry,
    ResumeUploadRequest,
    ResumeUploadResponse,
    ServerMessage,
    ServerPayload,
    UploadArtifactResponse,
    WorkerRegistration,
    WorkerService,
    WorkerStatus,
};

use hodei_core::WorkerCapabilities;
use hodei_core::{Worker, WorkerId};
use hodei_modules::SchedulerModule;
use hodei_ports::job_repository::JobRepository;
use hodei_ports::worker_client::WorkerClient;
use hodei_ports::worker_repository::WorkerRepository;
use hodei_ports::{event_bus::EventPublisher, scheduler_port::SchedulerError};
use hwp_proto::pb::server_message;

use crate::error::{GrpcError, GrpcResult};
use hodei_core::JobId;

pub struct HwpService {
    scheduler: Arc<dyn hodei_ports::scheduler_port::SchedulerPort + Send + Sync>,
}

impl HwpService {
    pub fn new(
        scheduler: Arc<dyn hodei_ports::scheduler_port::SchedulerPort + Send + Sync>,
    ) -> Self {
        Self { scheduler }
    }
}

#[tonic::async_trait]
impl WorkerService for HwpService {
    type JobStreamStream = Pin<Box<dyn Stream<Item = Result<ServerMessage, Status>> + Send>>;

    async fn register_worker(
        &self,
        request: Request<WorkerRegistration>,
    ) -> Result<Response<WorkerStatus>, Status> {
        let req = request.into_inner();
        let worker_id = req.worker_id.clone();
        info!("Registering worker: {}", worker_id);

        // Validate input
        if worker_id.trim().is_empty() {
            return Err(Status::invalid_argument("Worker ID cannot be empty"));
        }

        // Map capabilities (for US-02.5, we can validate them better)
        // TODO: Parse capabilities from string list using US-02.1 implementation
        let capabilities = WorkerCapabilities::new(4, 8192); // Default for now

        let worker = Worker::new(WorkerId::new(), worker_id.clone(), capabilities);

        match self.scheduler.register_worker(&worker).await {
            Ok(_) => {
                info!(worker_id, "Worker registered successfully");
                Ok(Response::new(WorkerStatus {
                    worker_id,
                    state: "IDLE".to_string(),
                    current_jobs: vec![],
                    last_heartbeat: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                }))
            }
            Err(e) => {
                let grpc_error: GrpcError = e.into();
                warn!(
                    worker_id,
                    error_type = grpc_error.error_type(),
                    error = %grpc_error,
                    "Failed to register worker"
                );
                // Convert to Status and return
                Err(grpc_error.to_status())
            }
        }
    }

    async fn assign_job(
        &self,
        _request: Request<AssignJobRequest>,
    ) -> Result<Response<JobAccepted>, Status> {
        Err(Status::unimplemented("Use JobStream for assignment"))
    }

    async fn stream_logs(
        &self,
        _request: Request<Streaming<LogEntry>>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("Use JobStream for logs"))
    }

    async fn cancel_job(
        &self,
        _request: Request<CancelJobRequest>,
    ) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("Use JobStream for cancellation"))
    }

    async fn get_worker_status(
        &self,
        request: Request<hwp_proto::GetWorkerStatusRequest>,
    ) -> Result<Response<WorkerStatus>, Status> {
        let req = request.into_inner();
        info!("Getting worker status: {}", req.worker_id);

        Ok(Response::new(WorkerStatus {
            worker_id: req.worker_id,
            state: "IDLE".to_string(),
            current_jobs: vec![],
            last_heartbeat: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<hwp_proto::HeartbeatRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        info!("Heartbeat from worker: {}", req.worker_id);

        Ok(Response::new(Empty {}))
    }

    async fn job_stream(
        &self,
        request: Request<Streaming<AgentMessage>>,
    ) -> Result<Response<Self::JobStreamStream>, Status> {
        // Extract worker_id from request metadata for US-02.4: Bidirectional Job Streaming
        let worker_id = request
            .metadata()
            .get("worker-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| WorkerId::from_uuid(uuid::Uuid::parse_str(s).unwrap_or_default()))
            .unwrap_or_else(|| {
                warn!("No worker-id in metadata, generating temporary ID");
                WorkerId::new()
            });

        let mut inbound = request.into_inner();
        let (tx, rx) = mpsc::unbounded_channel::<Result<ServerMessage, SchedulerError>>();

        info!(
            "Establishing bidirectional stream for worker: {}",
            worker_id
        );

        // Register the transmitter with the scheduler BEFORE spawning the task (US-02.4)
        let scheduler = self.scheduler.clone();
        if let Err(e) = scheduler.register_transmitter(&worker_id, tx).await {
            error!(
                "Failed to register transmitter for worker {}: {}",
                worker_id, e
            );
            return Err(Status::internal("Failed to establish bidirectional stream"));
        }

        // Spawn a task to handle incoming messages from the agent (US-02.4: Bidirectional streaming)
        let scheduler_clone = self.scheduler.clone();
        let worker_id_clone = worker_id.clone();
        tokio::spawn(async move {
            while let Some(result) = inbound.next().await {
                match result {
                    Ok(msg) => {
                        if let Some(payload) = msg.payload {
                            match payload {
                                AgentPayload::JobAccepted(accepted) => {
                                    info!(
                                        "Job accepted by worker {}: {}",
                                        worker_id_clone, accepted.job_id
                                    );
                                    // Update job status in scheduler/repo
                                    // TODO: Update job state to SCHEDULED or RUNNING
                                }
                                AgentPayload::LogEntry(log) => {
                                    info!(
                                        "Log from worker {} job {}: {}",
                                        worker_id_clone, log.job_id, log.data
                                    );
                                    // Forward log to event bus or storage
                                    // TODO: Publish log event to event bus
                                }
                                AgentPayload::JobResult(res) => {
                                    info!(
                                        "Job result from worker {} for job {}: exit_code={}",
                                        worker_id_clone, res.job_id, res.exit_code
                                    );
                                    // Complete job in scheduler
                                    // TODO: Update job state to COMPLETED or FAILED
                                    // TODO: Publish job completed event
                                }
                                _ => {
                                    info!(
                                        "Received unhandled payload type from worker {}",
                                        worker_id_clone
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Stream error from worker {}: {}", worker_id_clone, e);
                        break;
                    }
                }
            }
            warn!("Agent stream ended for worker {}", worker_id_clone);

            // Cleanup: unregister transmitter when stream ends (US-02.4)
            if let Err(e) = scheduler_clone
                .unregister_transmitter(&worker_id_clone)
                .await
            {
                error!(
                    "Failed to unregister transmitter for worker {}: {}",
                    worker_id_clone, e
                );
            }
        });

        let output_stream = UnboundedReceiverStream::new(rx).map(|result| {
            result.map_err(|e| {
                // Convert SchedulerError to tonic::Status
                match e {
                    SchedulerError::WorkerNotFound(_) => Status::not_found("Worker not found"),
                    SchedulerError::Validation(msg) => Status::invalid_argument(msg),
                    SchedulerError::Config(msg) => Status::failed_precondition(msg),
                    SchedulerError::NoEligibleWorkers => {
                        Status::resource_exhausted("No eligible workers")
                    }
                    SchedulerError::Internal(msg)
                    | SchedulerError::RegistrationFailed(msg)
                    | SchedulerError::JobRepository(msg)
                    | SchedulerError::WorkerRepository(msg)
                    | SchedulerError::WorkerClient(msg)
                    | SchedulerError::EventBus(msg)
                    | SchedulerError::ClusterState(msg) => Status::internal(msg),
                }
            })
        });
        Ok(Response::new(
            Box::pin(output_stream) as Self::JobStreamStream
        ))
    }

    async fn upload_artifact(
        &self,
        _request: Request<Streaming<ArtifactChunk>>,
    ) -> Result<Response<UploadArtifactResponse>, Status> {
        // TODO: Implement actual artifact upload handling
        Err(Status::unimplemented(
            "Artifact upload not yet implemented on server",
        ))
    }

    async fn initiate_upload(
        &self,
        _request: Request<InitiateUploadRequest>,
    ) -> Result<Response<InitiateUploadResponse>, Status> {
        // TODO: Implement actual upload initiation
        let upload_id = format!("upload-{}", uuid::Uuid::new_v4());
        Ok(Response::new(InitiateUploadResponse {
            upload_id,
            accepted: true,
            error_message: "".to_string(),
        }))
    }

    async fn resume_upload(
        &self,
        _request: Request<ResumeUploadRequest>,
    ) -> Result<Response<ResumeUploadResponse>, Status> {
        // TODO: Implement actual resume logic
        Ok(Response::new(ResumeUploadResponse {
            upload_id: "resumed-upload-id".to_string(),
            can_resume: true,
            error_message: "".to_string(),
            next_expected_chunk: 0,
        }))
    }

    async fn finalize_upload(
        &self,
        request: Request<FinalizeUploadRequest>,
    ) -> Result<Response<FinalizeUploadResponse>, Status> {
        let req = request.into_inner();
        info!("Finalizing upload for artifact: {}", req.artifact_id);

        Ok(Response::new(FinalizeUploadResponse {
            artifact_id: req.artifact_id,
            success: true,
            error_message: "".to_string(),
            server_checksum: req.checksum,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use hodei_core::{Job, JobSpec, JobState, ResourceQuota};
    use hodei_ports::event_bus::{LogEntry, MockEventPublisher, StreamType, SystemEvent};
    use hodei_ports::job_repository::MockJobRepository;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_job_accepted_updates_state_to_running() {
        let job_repo = Arc::new(MockJobRepository);
        let event_bus = Arc::new(MockEventPublisher);

        let job_id = JobId::new();
        let worker_id = WorkerId::new();

        let job = Job::new(
            job_id.clone(),
            JobSpec {
                name: "test-job".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
                resources: ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: 0,
                },
                timeout_ms: 60000,
                retries: 0,
                env: std::collections::HashMap::new(),
                secret_refs: vec![],
            },
        )
        .unwrap();

        job_repo.save_job(&job).await.unwrap();

        // Update job state to SCHEDULED before calling handle_job_accepted
        job_repo
            .compare_and_swap_status(&job_id, "PENDING", "SCHEDULED")
            .await
            .unwrap();

        let scheduler = hodei_modules::SchedulerBuilder::new()
            .job_repository(job_repo.clone())
            .event_bus(event_bus.clone())
            .worker_client(Arc::new(hodei_ports::MockWorkerClient))
            .worker_repository(Arc::new(hodei_ports::MockWorkerRepository))
            .build()
            .unwrap();

        // Call the method under test
        let result = scheduler.handle_job_accepted(&job_id, &worker_id).await;

        assert!(result.is_ok(), "handle_job_accepted should succeed");

        // Verify state was updated to RUNNING
        let updated_job = job_repo.get_job(&job_id).await.unwrap().unwrap();
        assert_eq!(
            updated_job.state,
            JobState::RUNNING,
            "Job state should be RUNNING"
        );
        assert!(
            updated_job.started_at.is_some(),
            "Start time should be recorded"
        );
    }

    #[tokio::test]
    async fn test_log_entry_published_to_event_bus() {
        let job_repo = Arc::new(MockJobRepository);
        let event_bus = Arc::new(MockEventPublisher);

        let job_id = JobId::new();
        let worker_id = WorkerId::new();

        let log_data = b"Test log output".to_vec();

        let scheduler = hodei_modules::SchedulerBuilder::new()
            .job_repository(job_repo.clone())
            .event_bus(event_bus.clone())
            .worker_client(Arc::new(hodei_ports::MockWorkerClient))
            .worker_repository(Arc::new(hodei_ports::MockWorkerRepository))
            .build()
            .unwrap();

        // Call the method under test
        let result = scheduler
            .handle_log_entry(&job_id, &worker_id, log_data.clone(), StreamType::Stdout)
            .await;

        assert!(result.is_ok(), "handle_log_entry should succeed");

        // Verify event was published
        assert_eq!(
            event_bus.published_events.len(),
            1,
            "One event should be published"
        );

        if let Some(published_event) = event_bus.published_events.first() {
            assert!(
                matches!(published_event, SystemEvent::LogChunkReceived(_)),
                "Should be LogChunkReceived event"
            );

            if let SystemEvent::LogChunkReceived(log) = published_event {
                assert_eq!(log.job_id, job_id, "Job ID should match");
                assert_eq!(log.data, log_data, "Log data should match");
                assert_eq!(
                    log.stream_type,
                    StreamType::Stdout,
                    "Stream type should match"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_job_result_updates_state_to_completed() {
        let job_repo = Arc::new(MockJobRepository);
        let event_bus = Arc::new(MockEventPublisher);

        let job_id = JobId::new();
        let worker_id = WorkerId::new();

        let mut job = Job::new(
            job_id.clone(),
            JobSpec {
                name: "test-job".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
                resources: ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: 0,
                },
                timeout_ms: 60000,
                retries: 0,
                env: std::collections::HashMap::new(),
                secret_refs: vec![],
            },
        )
        .unwrap();

        job.start().unwrap();
        job_repo.save_job(&job).await.unwrap();

        // Update job state to RUNNING before calling handle_job_result
        job_repo
            .compare_and_swap_status(&job_id, "PENDING", "RUNNING")
            .await
            .unwrap();
        job_repo
            .set_job_start_time(&job_id, Utc::now())
            .await
            .unwrap();

        let scheduler = hodei_modules::SchedulerBuilder::new()
            .job_repository(job_repo.clone())
            .event_bus(event_bus.clone())
            .worker_client(Arc::new(hodei_ports::MockWorkerClient))
            .worker_repository(Arc::new(hodei_ports::MockWorkerRepository))
            .build()
            .unwrap();

        // Call the method under test with exit code 0 (success)
        let result = scheduler.handle_job_result(&job_id, &worker_id, 0).await;

        assert!(result.is_ok(), "handle_job_result should succeed");

        // Verify job state was updated to COMPLETED
        let updated_job = job_repo.get_job(&job_id).await.unwrap().unwrap();
        assert!(
            updated_job.finished_at.is_some(),
            "Finish time should be recorded"
        );
        assert!(
            updated_job.duration.is_some(),
            "Duration should be calculated"
        );
    }

    #[tokio::test]
    async fn test_job_result_with_nonzero_exit_code_updates_state_to_failed() {
        let job_repo = Arc::new(MockJobRepository);
        let event_bus = Arc::new(MockEventPublisher);

        let job_id = JobId::new();
        let worker_id = WorkerId::new();

        let mut job = Job::new(
            job_id.clone(),
            JobSpec {
                name: "test-job".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["false".to_string()],
                resources: ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: 0,
                },
                timeout_ms: 60000,
                retries: 0,
                env: std::collections::HashMap::new(),
                secret_refs: vec![],
            },
        )
        .unwrap();

        job.start().unwrap();
        job_repo.save_job(&job).await.unwrap();

        // Update job state to RUNNING before calling handle_job_result
        job_repo
            .compare_and_swap_status(&job_id, "PENDING", "RUNNING")
            .await
            .unwrap();
        job_repo
            .set_job_start_time(&job_id, Utc::now())
            .await
            .unwrap();

        let scheduler = hodei_modules::SchedulerBuilder::new()
            .job_repository(job_repo.clone())
            .event_bus(event_bus.clone())
            .worker_client(Arc::new(hodei_ports::MockWorkerClient))
            .worker_repository(Arc::new(hodei_ports::MockWorkerRepository))
            .build()
            .unwrap();

        // Call the method under test with non-zero exit code (failure)
        let result = scheduler.handle_job_result(&job_id, &worker_id, 1).await;

        assert!(result.is_ok(), "handle_job_result should succeed");

        // Verify job state was updated to FAILED
        let updated_job = job_repo.get_job(&job_id).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_job_completed_event_published_with_metrics() {
        let job_repo = Arc::new(MockJobRepository);
        let event_bus = Arc::new(MockEventPublisher);

        let job_id = JobId::new();
        let worker_id = WorkerId::new();

        let mut job = Job::new(
            job_id.clone(),
            JobSpec {
                name: "test-job".to_string(),
                image: "ubuntu".to_string(),
                command: vec!["sleep".to_string(), "1".to_string()],
                resources: ResourceQuota {
                    cpu_m: 1000,
                    memory_mb: 512,
                    gpu: 0,
                },
                timeout_ms: 60000,
                retries: 0,
                env: std::collections::HashMap::new(),
                secret_refs: vec![],
            },
        )
        .unwrap();

        job.start().unwrap();
        let start_time = Utc::now();
        job_repo.save_job(&job).await.unwrap();

        // Update job state to RUNNING and set start time
        job_repo
            .compare_and_swap_status(&job_id, "PENDING", "RUNNING")
            .await
            .unwrap();
        job_repo
            .set_job_start_time(&job_id, start_time)
            .await
            .unwrap();

        // Simulate some execution time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let scheduler = hodei_modules::SchedulerBuilder::new()
            .job_repository(job_repo.clone())
            .event_bus(event_bus.clone())
            .worker_client(Arc::new(hodei_ports::MockWorkerClient))
            .worker_repository(Arc::new(hodei_ports::MockWorkerRepository))
            .build()
            .unwrap();

        // Call the method under test
        let result = scheduler.handle_job_result(&job_id, &worker_id, 0).await;

        assert!(result.is_ok(), "handle_job_result should succeed");

        // Verify JobCompleted event was published
        assert_eq!(
            event_bus.published_events.len(),
            1,
            "One event should be published"
        );

        if let Some(published_event) = event_bus.published_events.first() {
            match published_event {
                SystemEvent::JobCompleted {
                    job_id: id,
                    exit_code,
                } => {
                    assert_eq!(id, &job_id, "Job ID should match");
                    assert_eq!(exit_code, &0, "Exit code should be 0");
                }
                _ => panic!("Expected JobCompleted event, got {:?}", published_event),
            }
        }
    }
}
