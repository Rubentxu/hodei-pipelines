use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, warn};

use hwp_proto::{
    AgentMessage, AgentPayload, AssignJobRequest, CancelJobRequest, Empty, JobAccepted, JobResult,
    LogEntry, ServerMessage, ServerPayload, WorkerRegistration, WorkerService, WorkerStatus,
};

use hodei_core::{Worker, WorkerCapabilities, WorkerId};
use hodei_modules::SchedulerModule;
use hodei_ports::bus::EventBus;
use hodei_ports::repository::{JobRepository, WorkerRepository};
use hodei_ports::worker::WorkerClient;

pub struct HwpService<J, B, C, W>
where
    J: JobRepository + Send + Sync + 'static,
    B: EventBus + Send + Sync + 'static,
    C: WorkerClient + Send + Sync + 'static,
    W: WorkerRepository + Send + Sync + 'static,
{
    scheduler: Arc<SchedulerModule<J, B, C, W>>,
}

impl<J, B, C, W> HwpService<J, B, C, W>
where
    J: JobRepository + Send + Sync + 'static,
    B: EventBus + Send + Sync + 'static,
    C: WorkerClient + Send + Sync + 'static,
    W: WorkerRepository + Send + Sync + 'static,
{
    pub fn new(scheduler: Arc<SchedulerModule<J, B, C, W>>) -> Self {
        Self { scheduler }
    }
}

#[tonic::async_trait]
impl<J, B, C, W> WorkerService for HwpService<J, B, C, W>
where
    J: JobRepository + Send + Sync + 'static,
    B: EventBus + Send + Sync + 'static,
    C: WorkerClient + Send + Sync + 'static,
    W: WorkerRepository + Send + Sync + 'static,
{
    type StreamLogsStream = Pin<Box<dyn Stream<Item = Result<Empty, Status>> + Send>>;
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

        match self.scheduler.register_worker(worker).await {
            Ok(_) => Ok(Response::new(WorkerStatus {
                worker_id: req.worker_id,
                state: "IDLE".to_string(),
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

    async fn job_stream(
        &self,
        request: Request<Streaming<AgentMessage>>,
    ) -> Result<Response<Self::JobStreamStream>, Status> {
        let mut inbound = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        // Spawn a task to handle incoming messages from the agent
        let scheduler = self.scheduler.clone();
        tokio::spawn(async move {
            while let Some(result) = inbound.next().await {
                match result {
                    Ok(msg) => {
                        if let Some(payload) = msg.payload {
                            match payload {
                                AgentPayload::JobAccepted(accepted) => {
                                    info!("Job accepted: {}", accepted.job_id);
                                    // Update job status in scheduler/repo
                                }
                                AgentPayload::LogEntry(log) => {
                                    info!("Log from {}: {}", log.job_id, log.data);
                                    // Forward log to event bus or storage
                                }
                                AgentPayload::JobResult(res) => {
                                    info!(
                                        "Job result for {}: exit_code={}",
                                        res.job_id, res.exit_code
                                    );
                                    // Complete job in scheduler
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        error!("Stream error: {}", e);
                        break;
                    }
                }
            }
            warn!("Agent stream ended");
        });

        // Return the outbound stream (server -> agent)
        // In a real implementation, we would register this tx with the scheduler
        // so it can send AssignJob requests to this specific worker.
        // For now, we just return an empty stream or keep it open.

        // TODO: Register tx with Scheduler to enable pushing jobs

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::JobStreamStream
        ))
    }

    async fn connect_worker(
        &self,
        request: Request<Streaming<AgentMessage>>,
    ) -> Result<Response<Self::JobStreamStream>, Status> {
        // This is the new method we added in the proto for bidirectional streaming
        // It should behave similarly to job_stream but might be the primary entry point
        self.job_stream(request).await
    }
}
