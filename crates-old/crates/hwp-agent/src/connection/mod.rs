//! Connection management module
//!
//! This module handles the gRPC connection to the Hodei server,
//! authentication, and bidirectional streaming.

pub mod auth;
pub mod grpc_client;

pub use grpc_client::Client;
