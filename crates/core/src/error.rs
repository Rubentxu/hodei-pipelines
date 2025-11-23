//! Domain Error Types
//!
//! These are pure domain errors without infrastructure concerns.

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum DomainError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Domain error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, DomainError>;
