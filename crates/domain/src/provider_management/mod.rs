//! Provider Management Bounded Context
//!
//! Manages provider registration, capabilities, and lifecycle
//! - Provider aggregate root
//! - Provider configuration value objects
//! - Provider repository port
//! - Provider registry service

pub mod entities;
pub mod repositories;
pub mod services;
pub mod use_cases;
pub mod value_objects;

// Re-exports
pub use entities::Provider;
pub use repositories::ProviderRepository;
pub use services::ProviderRegistry;
pub use use_cases::{ListProvidersUseCase, RegisterProviderUseCase};
pub use value_objects::ProviderConfig;
