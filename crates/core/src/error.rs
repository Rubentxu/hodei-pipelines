//! Error types shared across the system
//!
//! This module defines comprehensive error types following DDD principles,
//! providing granular error categories for better error handling and debugging.

use thiserror::Error;

/// Base error type for the entire system
#[derive(Error, Debug)]
pub enum DomainError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("resource not found: {0}")]
    NotFound(String),

    #[error("concurrency error: {0}")]
    Concurrency(String),

    #[error("infrastructure error: {0}")]
    Infrastructure(String),

    #[error("authorization error: {0}")]
    Authorization(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("other error: {0}")]
    Other(String),
}

impl DomainError {
    pub fn invalid_state_transition(from: &str, to: &str) -> Self {
        Self::InvalidStateTransition {
            from: from.to_string(),
            to: to.to_string(),
        }
    }
}

/// Database-specific error types
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("query execution failed: {0}")]
    Query(String),

    #[error("transaction failed: {0}")]
    Transaction(String),

    #[error("migration failed: {0}")]
    Migration(String),

    #[error("constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("deadlock detected")]
    Deadlock,

    #[error("timeout waiting for connection")]
    ConnectionTimeout,

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("deserialization error: {0}")]
    Deserialization(String),
}

impl From<DatabaseError> for DomainError {
    fn from(err: DatabaseError) -> Self {
        DomainError::Infrastructure(err.to_string())
    }
}

/// Repository operation error types
#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("entity with ID {id} not found")]
    EntityNotFound { id: String },

    #[error("optimistic lock failed: expected version {expected} but found {actual}")]
    OptimisticLockError { expected: u64, actual: u64 },

    #[error("concurrent modification detected")]
    ConcurrentModification,

    #[error("entity already exists with ID {id}")]
    EntityAlreadyExists { id: String },

    #[error("invalid entity state: {0}")]
    InvalidEntityState(String),

    #[error("version conflict for entity with ID {id}")]
    VersionConflict { id: String },
}

impl From<RepositoryError> for DomainError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::EntityNotFound { .. } => DomainError::NotFound(err.to_string()),
            RepositoryError::OptimisticLockError { .. }
            | RepositoryError::ConcurrentModification => DomainError::Concurrency(err.to_string()),
            RepositoryError::EntityAlreadyExists { .. }
            | RepositoryError::InvalidEntityState { .. }
            | RepositoryError::VersionConflict { .. } => DomainError::Validation(err.to_string()),
        }
    }
}

/// Pipeline-specific error types
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("circular dependency detected in pipeline")]
    CircularDependency,

    #[error("step {step_id} not found in pipeline")]
    StepNotFound { step_id: String },

    #[error("invalid step dependency: {0}")]
    InvalidDependency(String),

    #[error("pipeline execution failed: {0}")]
    ExecutionFailed(String),

    #[error("invalid pipeline status transition from {from} to {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("pipeline step timeout exceeded")]
    StepTimeout,
}

impl From<PipelineError> for DomainError {
    fn from(err: PipelineError) -> Self {
        match err {
            PipelineError::CircularDependency | PipelineError::InvalidDependency(_) => {
                DomainError::Validation(err.to_string())
            }
            PipelineError::StepNotFound { .. } => DomainError::NotFound(err.to_string()),
            PipelineError::ExecutionFailed(_) | PipelineError::StepTimeout => {
                DomainError::Infrastructure(err.to_string())
            }
            PipelineError::InvalidStatusTransition { .. } => DomainError::InvalidStateTransition {
                from: "unknown".to_string(),
                to: "unknown".to_string(),
            },
        }
    }
}

/// Job-specific error types
#[derive(Error, Debug)]
pub enum JobError {
    #[error("job execution failed: {0}")]
    ExecutionFailed(String),

    #[error("job timeout exceeded")]
    Timeout,

    #[error("invalid job state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("job cancellation failed: {0}")]
    CancellationFailed(String),

    #[error("retry limit exceeded for job")]
    RetryLimitExceeded,

    #[error("job not assigned to any worker")]
    Unassigned,
}

impl From<JobError> for DomainError {
    fn from(err: JobError) -> Self {
        match err {
            JobError::InvalidStateTransition { .. } => DomainError::InvalidStateTransition {
                from: "unknown".to_string(),
                to: "unknown".to_string(),
            },
            JobError::ExecutionFailed(_) | JobError::Timeout | JobError::Unassigned => {
                DomainError::Infrastructure(err.to_string())
            }
            JobError::CancellationFailed(_) | JobError::RetryLimitExceeded => {
                DomainError::Validation(err.to_string())
            }
        }
    }
}

/// Worker-specific error types
#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("worker not reachable: {id}")]
    Unreachable { id: String },

    #[error("worker registration failed: {0}")]
    RegistrationFailed(String),

    #[error("worker {id} already registered")]
    AlreadyRegistered { id: String },

    #[error("worker unhealthy: {id}")]
    Unhealthy { id: String },

    #[error("worker capability mismatch: {0}")]
    CapabilityMismatch(String),

    #[error("worker pool exhausted")]
    PoolExhausted,
}

impl From<WorkerError> for DomainError {
    fn from(err: WorkerError) -> Self {
        match err {
            WorkerError::Unreachable { .. }
            | WorkerError::RegistrationFailed(_)
            | WorkerError::Unhealthy { .. }
            | WorkerError::PoolExhausted => DomainError::Infrastructure(err.to_string()),
            WorkerError::AlreadyRegistered { .. } => DomainError::Validation(err.to_string()),
            WorkerError::CapabilityMismatch(_) => DomainError::Authorization(err.to_string()),
        }
    }
}

/// Event bus-specific error types
#[derive(Error, Debug)]
pub enum EventBusError {
    #[error("publish failed: {0}")]
    PublishFailed(String),

    #[error("subscribe failed: {0}")]
    SubscribeFailed(String),

    #[error("connection lost to event bus")]
    ConnectionLost,

    #[error("event serialization failed: {0}")]
    Serialization(String),

    #[error("event deserialization failed: {0}")]
    Deserialization(String),

    #[error("subscription timeout")]
    SubscriptionTimeout,
}

impl From<EventBusError> for DomainError {
    fn from(err: EventBusError) -> Self {
        match err {
            EventBusError::ConnectionLost | EventBusError::PublishFailed(_) => {
                DomainError::Infrastructure(err.to_string())
            }
            EventBusError::SubscriptionTimeout => DomainError::Timeout(err.to_string()),
            EventBusError::Serialization(_) | EventBusError::Deserialization(_) => {
                DomainError::Other(err.to_string())
            }
            _ => DomainError::Infrastructure(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_error_creation() {
        let err = DomainError::invalid_state_transition("PENDING", "RUNNING");
        assert!(matches!(err, DomainError::InvalidStateTransition { .. }));

        let err = DomainError::Validation("test".to_string());
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn test_database_error_conversion() {
        let db_err = DatabaseError::Connection("test".to_string());
        let domain_err: DomainError = db_err.into();
        assert!(matches!(domain_err, DomainError::Infrastructure(_)));
    }

    #[test]
    fn test_repository_error_conversion() {
        let repo_err = RepositoryError::EntityNotFound {
            id: "test-id".to_string(),
        };
        let domain_err: DomainError = repo_err.into();
        assert!(matches!(domain_err, DomainError::NotFound(_)));
    }

    #[test]
    fn test_pipeline_error_conversion() {
        let pipeline_err = PipelineError::CircularDependency;
        let domain_err: DomainError = pipeline_err.into();
        assert!(matches!(domain_err, DomainError::Validation(_)));
    }

    #[test]
    fn test_job_error_conversion() {
        let job_err = JobError::Timeout;
        let domain_err: DomainError = job_err.into();
        assert!(matches!(domain_err, DomainError::Infrastructure(_)));
    }

    #[test]
    fn test_worker_error_conversion() {
        let worker_err = WorkerError::Unreachable {
            id: "worker-1".to_string(),
        };
        let domain_err: DomainError = worker_err.into();
        assert!(matches!(domain_err, DomainError::Infrastructure(_)));
    }
}
