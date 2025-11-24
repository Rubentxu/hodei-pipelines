//! Hodei Worker Protocol (HWP) Protobuf Definitions
//!
//! This crate contains the Protocol Buffer definitions for the HWP protocol,
//! which is used for communication between the Hodei Jobs server and worker agents.

pub mod pb {
    tonic::include_proto!("hwp");
}

pub use pb::{
    AgentMessage, AssignJobRequest, CancelJobRequest, Empty, GetWorkerStatusRequest,
    HeartbeatRequest, JobAccepted, JobResult, JobSpec, LogEntry, ResourceQuota, ResourceUsage,
    ServerMessage, WorkerRegistration, WorkerStatus,
};

pub use pb::worker_service_client::WorkerServiceClient;
pub use pb::worker_service_server::{WorkerService, WorkerServiceServer};

// Re-export for convenience
pub use pb::agent_message::Payload as AgentPayload;
pub use pb::server_message::Payload as ServerPayload;
