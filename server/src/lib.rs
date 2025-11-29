//! Hodei Server Library
//!
//! This module contains the core server implementation for the Hodei job orchestration system.

pub mod api_docs;
pub mod bootstrap;
pub mod error;
pub mod execution_api;
pub mod grpc;
pub mod handlers;
pub mod logs_api;
pub mod observability_api;
pub mod pipeline_api;
pub mod resource_pool_crud;
pub mod terminal;

// API Router module for shared routes
pub mod api_router;

pub use bootstrap::{
    BootstrapError, Result as BootstrapResult, ServerComponents, initialize_server,
};
pub use grpc::HwpService;

// Re-export create_api_router for testing
pub use crate::api_router::create_api_router;

// Re-export handlers
pub use crate::handlers::{health_check, server_status};
