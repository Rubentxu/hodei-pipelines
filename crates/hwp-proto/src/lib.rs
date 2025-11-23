//! Hodei Worker Protocol (HWP) Protobuf Definitions
//!
//! This crate contains the Protocol Buffer definitions for the HWP protocol,
//! which is used for communication between the Hodei Jobs server and worker agents.

pub mod pb {
    tonic::include_proto!("hwp");
}

pub use pb::{
    AssignJobRequest, CancelJobRequest, Empty, JobAccepted, JobResult, LogEntry,
    WorkerRegistration, WorkerStatus,
};

pub use pb::worker_service_server::{WorkerService, WorkerServiceServer};
