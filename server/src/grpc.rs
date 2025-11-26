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
    AgentMessage, AgentPayload, AssignJobRequest, CancelJobRequest, Empty, JobAccepted, JobResult,
    LogEntry, ServerMessage, ServerPayload, WorkerRegistration, WorkerService, WorkerStatus,
};

use hodei_core::WorkerCapabilities;
use hodei_core::{Worker, WorkerId};
use hodei_modules::SchedulerModule;
use hodei_ports::job_repository::JobRepository;
use hodei_ports::worker_client::WorkerClient;
use hodei_ports::worker_repository::WorkerRepository;
use hodei_ports::{event_bus::EventPublisher, scheduler_port::SchedulerError};
use hwp_proto::pb::server_message;

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
        info!("Registering worker: {}", req.worker_id);

        // Map capabilities
        // TODO: Parse capabilities from string list
        let capabilities = WorkerCapabilities::new(4, 8192); // Default for now

        let worker = Worker::new(WorkerId::new(), req.worker_id.clone(), capabilities);

        match self.scheduler.register_worker(&worker).await {
            Ok(_) => Ok(Response::new(WorkerStatus {
                worker_id: req.worker_id,
                state: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            })),
            Err(e) => {
                error!("Failed to register worker: {}", e);
                Err(Status::internal("Failed to register worker"))
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
}
