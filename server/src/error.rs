//! Error handling for gRPC services
//!
//! This module provides structured error types and mappings for gRPC services,
//! ensuring consistent error handling across the system.

use chrono::Utc;
use hodei_core::{JobId, WorkerId};
use hodei_ports::scheduler_port::SchedulerError;
use tonic::Status;
use tracing::{error, info, warn};

/// Result type alias for gRPC operations
pub type GrpcResult<T> = std::result::Result<T, GrpcError>;

/// Structured error type for gRPC operations
#[derive(thiserror::Error, Debug)]
pub enum GrpcError {
    #[error("Worker not found: {0}")]
    WorkerNotFound(WorkerId),

    #[error("Job not found: {0}")]
    JobNotFound(JobId),

    #[error("Invalid capability format: {0}")]
    InvalidCapability(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Scheduler error: {0}")]
    Scheduler(#[from] SchedulerError),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl GrpcError {
    /// Convert error to tonic::Status with logging and metrics
    pub fn to_status(&self) -> Status {
        let error_type = self.error_type();

        // Record metrics
        // In a real application, we'd use global metrics instance
        // For now, just log

        // Structured logging
        match self {
            GrpcError::WorkerNotFound(worker_id) => {
                warn!(worker_id = %worker_id, error_type, "Worker not found");
                Status::not_found("Worker not registered or not found")
            }
            GrpcError::JobNotFound(job_id) => {
                warn!(job_id = %job_id, error_type, "Job not found");
                Status::not_found("Job not found")
            }
            GrpcError::InvalidCapability(msg) => {
                warn!(error_type, details = %msg, "Invalid capability provided");
                Status::invalid_argument(msg)
            }
            GrpcError::InvalidRequest(msg) => {
                warn!(error_type, details = %msg, "Invalid request");
                Status::invalid_argument(msg)
            }
            GrpcError::Scheduler(e) => {
                error!(error_type = "scheduler_error", scheduler_error = %e, "Scheduler error occurred");
                match e {
                    SchedulerError::WorkerNotFound(worker_id) => {
                        Status::not_found("Worker not found")
                    }
                    SchedulerError::Validation(msg) | SchedulerError::Config(msg) => {
                        Status::invalid_argument(msg)
                    }
                    SchedulerError::NoEligibleWorkers => {
                        Status::resource_exhausted("No eligible workers available")
                    }
                    SchedulerError::Internal(msg)
                    | SchedulerError::RegistrationFailed(msg)
                    | SchedulerError::JobRepository(msg)
                    | SchedulerError::WorkerRepository(msg)
                    | SchedulerError::WorkerClient(msg)
                    | SchedulerError::EventBus(msg)
                    | SchedulerError::ClusterState(msg) => Status::internal(msg),
                }
            }
            GrpcError::Internal(msg) => {
                error!(error_type, details = %msg, "Internal error");
                Status::internal(msg)
            }
        }
    }

    /// Get error type for metrics and logging
    pub fn error_type(&self) -> &'static str {
        match self {
            GrpcError::WorkerNotFound(_) => "worker_not_found",
            GrpcError::JobNotFound(_) => "job_not_found",
            GrpcError::InvalidCapability(_) => "invalid_capability",
            GrpcError::InvalidRequest(_) => "invalid_request",
            GrpcError::Scheduler(_) => "scheduler_error",
            GrpcError::Internal(_) => "internal_error",
        }
    }
}

impl From<GrpcError> for Status {
    fn from(error: GrpcError) -> Self {
        error.to_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::WorkerId;

    #[test]
    fn test_worker_not_found_to_status() {
        let worker_id = WorkerId::new();
        let error = GrpcError::WorkerNotFound(worker_id.clone());
        let status: Status = error.clone().into();

        assert_eq!(status.code(), tonic::Code::NotFound);
        assert!(status.message().contains("Worker not found"));
    }

    #[test]
    fn test_job_not_found_to_status() {
        let job_id = JobId::new();
        let error = GrpcError::JobNotFound(job_id.clone());
        let status: Status = error.into();

        assert_eq!(status.code(), tonic::Code::NotFound);
        assert!(status.message().contains("Job not found"));
    }

    #[test]
    fn test_invalid_capability_to_status() {
        let error = GrpcError::InvalidCapability("invalid format".to_string());
        let status: Status = error.into();

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("Invalid capability"));
    }

    #[test]
    fn test_scheduler_worker_not_found_to_status() {
        let worker_id = WorkerId::new();
        let scheduler_error = SchedulerError::worker_not_found(worker_id);
        let error = GrpcError::Scheduler(scheduler_error);
        let status: Status = error.into();

        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[test]
    fn test_scheduler_validation_to_status() {
        let scheduler_error = SchedulerError::validation_failed("Invalid input".to_string());
        let error = GrpcError::Scheduler(scheduler_error);
        let status: Status = error.into();

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn test_error_type() {
        let worker_id = WorkerId::new();
        assert_eq!(
            GrpcError::WorkerNotFound(worker_id).error_type(),
            "worker_not_found"
        );

        let job_id = JobId::new();
        assert_eq!(GrpcError::JobNotFound(job_id).error_type(), "job_not_found");

        assert_eq!(
            GrpcError::InvalidCapability("test".to_string()).error_type(),
            "invalid_capability"
        );

        assert_eq!(
            GrpcError::Internal("test".to_string()).error_type(),
            "internal_error"
        );
    }

    #[test]
    fn test_error_display() {
        let worker_id = WorkerId::new();
        let error = GrpcError::WorkerNotFound(worker_id.clone());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Worker not found"));
        assert!(error_str.contains(&worker_id.to_string()));

        let job_id = JobId::new();
        let error = GrpcError::JobNotFound(job_id.clone());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Job not found"));
        assert!(error_str.contains(&job_id.to_string()));
    }
}
