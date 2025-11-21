//! Domain layer - Core business logic
//!
//! This layer contains entities, value objects, and domain services.
//! It is independent of infrastructure concerns.

pub mod job_entity;

pub use job_entity::{Job, JobEvent};

/// Domain services
pub mod services {
    use hodei_shared_types::DomainError;

    /// Job validation service
    pub struct JobValidator;

    impl JobValidator {
        pub fn validate_name(name: &str) -> Result<(), DomainError> {
            if name.trim().is_empty() {
                return Err(DomainError::Validation(
                    "job name cannot be empty".to_string(),
                ));
            }
            Ok(())
        }

        pub fn validate_timeout(timeout_ms: u64) -> Result<(), DomainError> {
            if timeout_ms == 0 {
                return Err(DomainError::Validation(
                    "timeout must be greater than 0".to_string(),
                ));
            }
            Ok(())
        }
    }
}
