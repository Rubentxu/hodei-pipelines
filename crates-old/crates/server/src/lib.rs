//! Hodei Server Library
//!
//! This module contains the core server implementation for the Hodei job orchestration system.

pub mod alerting_system;
pub mod api_docs;
pub mod audit_logs_compliance;
pub mod bootstrap;
pub mod dtos;
pub mod error;
pub mod grpc;
pub mod handlers;
pub mod identity_access;
pub mod live_metrics_api;
pub mod observability;
pub mod pipeline_execution;
pub mod realtime_status_api;
pub mod resource_governance;
pub mod scheduling;
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
