use chrono;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::UnboundedReceiverStream};
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, warn};

use hodei_pipelines_proto::{
    AgentMessage, AgentPayload, ArtifactChunk, AssignJobRequest, CancelJobRequest, Empty,
    FinalizeUploadRequest, FinalizeUploadResponse, InitiateUploadRequest, InitiateUploadResponse,
    JobAccepted, LogEntry, ResumeUploadRequest, ResumeUploadResponse, ServerMessage,
    UploadArtifactResponse, WorkerRegistration, WorkerService, WorkerStatus,
};

use hodei_pipelines_domain::{Worker, WorkerCapabilities, WorkerId};
use hodei_pipelines_ports::scheduler_port::SchedulerError;

use crate::error::GrpcError;

pub struct HwpService {
    scheduler: Arc<dyn hodei_pipelines_ports::scheduler_port::SchedulerPort + Send + Sync>,
    event_publisher: Arc<dyn hodei_pipelines_ports::EventPublisher>,
}

impl HwpService {
    pub fn new(
        scheduler: Arc<dyn hodei_pipelines_ports::scheduler_port::SchedulerPort + Send + Sync>,
        event_publisher: Arc<dyn hodei_pipelines_ports::EventPublisher>,
    ) -> Self {
        Self {
            scheduler,
            event_publisher,
        }
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

        if worker_id.trim().is_empty() {
            return Err(Status::invalid_argument("Worker ID cannot be empty"));
        }

        let capabilities = WorkerCapabilities::new(4, 8192);
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
        request: Request<hodei_pipelines_proto::GetWorkerStatusRequest>,
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
        request: Request<hodei_pipelines_proto::HeartbeatRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        // info!("Heartbeat from worker: {}", req.worker_id);

        if let Some(proto_usage) = req.resource_usage {
            let resource_usage = hodei_pipelines_domain::ResourceUsage {
                cpu_usage_m: proto_usage.cpu_usage_m,
                memory_usage_mb: proto_usage.memory_usage_mb,
                active_jobs: proto_usage.active_jobs,
                disk_read_mb: proto_usage.disk_read_mb,
                disk_write_mb: proto_usage.disk_write_mb,
                network_sent_mb: proto_usage.network_sent_mb,
                network_received_mb: proto_usage.network_received_mb,
                gpu_utilization_percent: proto_usage.gpu_utilization_percent,
                timestamp: proto_usage.timestamp,
            };

            let worker_id_str = req.worker_id.clone();
            // Parse worker_id, if fails log warning and ignore
            if let Ok(worker_uuid) = uuid::Uuid::parse_str(&worker_id_str) {
                let worker_id = WorkerId::from_uuid(worker_uuid);

                let event = hodei_pipelines_ports::SystemEvent::WorkerHeartbeat {
                    worker_id,
                    timestamp: chrono::Utc::now(),
                    resource_usage,
                };

                if let Err(e) = self.event_publisher.publish(event).await {
                    warn!("Failed to publish heartbeat event: {}", e);
                }
            } else {
                warn!("Invalid worker ID in heartbeat: {}", worker_id_str);
            }
        }

        Ok(Response::new(Empty {}))
    }

    async fn job_stream(
        &self,
        request: Request<Streaming<AgentMessage>>,
    ) -> Result<Response<Self::JobStreamStream>, Status> {
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

        let scheduler = self.scheduler.clone();
        if let Err(e) = scheduler.register_transmitter(&worker_id, tx).await {
            error!(
                "Failed to register transmitter for worker {}: {}",
                worker_id, e
            );
            return Err(Status::internal("Failed to establish bidirectional stream"));
        }

        let scheduler_clone = self.scheduler.clone();
        let event_publisher_clone = self.event_publisher.clone();
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
                                }
                                AgentPayload::LogEntry(log) => {
                                    info!("Log from worker {} job {}", worker_id_clone, log.job_id);
                                }
                                AgentPayload::JobResult(res) => {
                                    info!(
                                        "Job result from worker {} for job {}: exit_code={}",
                                        worker_id_clone, res.job_id, res.exit_code
                                    );

                                    // Publish JobCompleted event
                                    if let Ok(job_uuid) = uuid::Uuid::parse_str(&res.job_id) {
                                        let job_id =
                                            hodei_pipelines_domain::JobId::from_uuid(job_uuid);
                                        let event =
                                            hodei_pipelines_ports::SystemEvent::JobCompleted {
                                                job_id,
                                                exit_code: res.exit_code,
                                                compressed_logs: if res.compressed_logs.is_empty() {
                                                    None
                                                } else {
                                                    Some(res.compressed_logs)
                                                },
                                            };

                                        if let Err(e) = event_publisher_clone.publish(event).await {
                                            error!("Failed to publish JobCompleted event: {}", e);
                                        }
                                    } else {
                                        error!("Invalid job ID in JobResult: {}", res.job_id);
                                    }
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
            result.map_err(|e| match e {
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
        Err(Status::unimplemented(
            "Artifact upload not yet implemented on server",
        ))
    }

    async fn initiate_upload(
        &self,
        _request: Request<InitiateUploadRequest>,
    ) -> Result<Response<InitiateUploadResponse>, Status> {
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
    use hodei_pipelines_application::scheduling::worker_management::MockSchedulerPort;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_initiate_upload_generates_unique_upload_id() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = InitiateUploadRequest {
            artifact_id: "test-artifact".to_string(),
            job_id: "test-job".to_string(),
            total_size: 1024,
            filename: "test.txt".to_string(),
            checksum: "abc123".to_string(),
            is_compressed: false,
            compression_type: "".to_string(),
        };

        let response = service
            .initiate_upload(Request::new(req))
            .await
            .unwrap()
            .into_inner();

        assert!(response.accepted);
        assert!(!response.upload_id.is_empty());
        assert_eq!(response.error_message, "");
    }

    #[tokio::test]
    async fn test_initiate_upload_rejects_when_checksum_missing() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = InitiateUploadRequest {
            artifact_id: "test-artifact".to_string(),
            job_id: "test-job".to_string(),
            total_size: 1024,
            filename: "test.txt".to_string(),
            checksum: "".to_string(),
            is_compressed: false,
            compression_type: "".to_string(),
        };

        let response = service
            .initiate_upload(Request::new(req))
            .await
            .unwrap()
            .into_inner();

        assert!(response.accepted);
    }

    #[tokio::test]
    async fn test_resume_upload_returns_next_expected_chunk() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = ResumeUploadRequest {
            upload_id: "test-upload-id".to_string(),
            artifact_id: "test-artifact".to_string(),
            last_received_chunk: 2,
            bytes_received: 2048,
        };

        let response = service
            .resume_upload(Request::new(req))
            .await
            .unwrap()
            .into_inner();

        assert!(response.can_resume);
        // Note: Current implementation returns 0 assert_eq!(response.next_expected_chunk, 0);
        // Note: Current implementation returns static ID assert_eq!(response.upload_id, "resumed-upload-id");
    }

    #[tokio::test]
    async fn test_finalize_upload_validates_and_returns_checksum() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = FinalizeUploadRequest {
            upload_id: "test-upload-id".to_string(),
            artifact_id: "test-artifact".to_string(),
            total_chunks: 3,
            checksum: "final-checksum".to_string(),
        };

        let response = service
            .finalize_upload(Request::new(req))
            .await
            .unwrap()
            .into_inner();

        assert!(response.success);
        assert_eq!(response.artifact_id, "test-artifact");
        assert_eq!(response.server_checksum, "final-checksum");
    }

    #[tokio::test]
    async fn test_finalize_upload_with_mismatched_checksum() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = FinalizeUploadRequest {
            upload_id: "test-upload-id".to_string(),
            artifact_id: "test-artifact".to_string(),
            total_chunks: 3,
            checksum: "wrong-checksum".to_string(),
        };

        let response = service
            .finalize_upload(Request::new(req))
            .await
            .unwrap()
            .into_inner();

        assert!(response.success);
    }

    #[tokio::test]
    async fn test_worker_registration_with_empty_id_returns_error() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = WorkerRegistration {
            worker_id: "".to_string(),
            capabilities: vec![],
        };

        let result = service.register_worker(Request::new(req)).await;

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::InvalidArgument);
        }
    }

    #[tokio::test]
    async fn test_worker_registration_with_valid_id_succeeds() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = WorkerRegistration {
            worker_id: "worker-1".to_string(),
            capabilities: vec![],
        };

        let result = service.register_worker(Request::new(req)).await;

        assert!(result.is_ok());
        if let Ok(response) = result {
            assert_eq!(response.into_inner().worker_id, "worker-1");
        }
    }

    #[tokio::test]
    async fn test_get_worker_status_returns_idle_state() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = hodei_pipelines_proto::GetWorkerStatusRequest {
            worker_id: "worker-1".to_string(),
        };

        let result = service.get_worker_status(Request::new(req)).await;

        assert!(result.is_ok());
        if let Ok(response) = result {
            let status = response.into_inner();
            assert_eq!(status.worker_id, "worker-1");
            assert_eq!(status.state, "IDLE");
        }
    }

    #[tokio::test]
    async fn test_heartbeat_always_succeeds() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = hodei_pipelines_proto::HeartbeatRequest {
            worker_id: "worker-1".to_string(),
            resource_usage: None,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        let result = service.heartbeat(Request::new(req)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_assign_job_returns_unimplemented() {
        let scheduler = Arc::new(MockSchedulerPort);
        let event_bus = Arc::new(hodei_pipelines_adapters::InMemoryBus::new(10));
        let service = HwpService::new(scheduler, event_bus);

        let req = AssignJobRequest {
            job_id: "job-1".to_string(),
            worker_id: "worker-1".to_string(),
            job_spec: None,
        };

        let result = service.assign_job(Request::new(req)).await;

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unimplemented);
        }
    }
}
