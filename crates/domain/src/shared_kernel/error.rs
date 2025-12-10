//! Domain Error Types
//!
//! Centralized error handling for the domain layer

/// Result type for domain operations
pub type DomainResult<T> = std::result::Result<T, DomainError>;

/// Main domain error enum
#[derive(thiserror::Error, Debug)]
pub enum DomainError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),
}
