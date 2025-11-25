//! Error types shared across the system

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
}

impl DomainError {
    pub fn invalid_state_transition(from: &str, to: &str) -> Self {
        Self::InvalidStateTransition {
            from: from.to_string(),
            to: to.to_string(),
        }
    }
}
