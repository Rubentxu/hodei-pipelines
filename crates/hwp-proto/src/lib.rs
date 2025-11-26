//! Hodei Worker Protocol (HWP) Protobuf Definitions
//!
//! This crate contains the Protocol Buffer definitions for the HWP protocol,
//! which is used for communication between the Hodei Pipelines server and worker agents.

pub mod pb {
    tonic::include_proto!("hwp");
}

pub use pb::{
    AgentMessage,
    // Artifact upload messages
    ArtifactChunk,
    AssignJobRequest,
    CancelJobRequest,
    Empty,
    FinalizeUploadRequest,
    FinalizeUploadResponse,
    GetWorkerStatusRequest,
    HeartbeatRequest,
    InitiateUploadRequest,
    InitiateUploadResponse,
    JobAccepted,
    JobResult,
    JobSpec,
    LogEntry,
    ResourceQuota,
    ResourceUsage,
    ResumeUploadRequest,
    ResumeUploadResponse,
    ServerMessage,
    UploadArtifactRequest,
    UploadArtifactResponse,
    WorkerRegistration,
    WorkerStatus,
};

pub use pb::worker_service_client::WorkerServiceClient;
pub use pb::worker_service_server::{WorkerService, WorkerServiceServer};

// Re-export for convenience
pub use pb::agent_message::Payload as AgentPayload;
pub use pb::server_message::Payload as ServerPayload;
