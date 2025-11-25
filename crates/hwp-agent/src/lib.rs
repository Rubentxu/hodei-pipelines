//! HWP Agent - Lightweight Worker Agent for Hodei Pipelines
//!
//! This crate implements the HWP (Hodei Worker Protocol) agent that runs
//! inside containers/VMs and connects to the Hodei server via gRPC.
//!
//! Key features:
//! - Binary size <5MB (static linking)
//! - PTY support for colored logs
//! - Intelligent log buffering
//! - Real-time resource monitoring
//! - Auto-reconnection
//! - Secure bootstrapping

pub mod artifacts;
pub mod config;
pub mod connection;
pub mod executor;
pub mod logging;
pub mod monitor;

pub use config::Config;
pub use connection::Client;
pub use executor::{JobExecutor, JobHandle, ProcessInfo};
pub use logging::{LogBuffer, LogStreamer};
pub use monitor::ResourceMonitor;

/// Agent result type
pub type Result<T> = std::result::Result<T, AgentError>;

/// Agent error types
#[derive(thiserror::Error, Debug)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Worker client error: {0}")]
    WorkerClient(#[from] hodei_ports::WorkerClientError),

    #[error("Other error: {0}")]
    Other(String),
}
