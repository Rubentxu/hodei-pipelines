//! Orchestrator Bounded Context Ports
//!
//! This module defines the interfaces (ports) for the Orchestrator bounded context,
//! which coordinates between different bounded contexts in the system.
//!
//! # Architecture
//!
//! The Orchestrator bounded context acts as a director, coordinating:
//! - Pipeline execution
//! - Scheduling operations
//! - Resource governance
//! - Policy enforcement
//!
//! # Usage Example
//! ```ignore
//! use hodei_pipelines_ports::orchestrator::PipelineCoordinationPort;
//! use hodei_pipelines_domain::pipeline_execution::entities::pipeline::PipelineId;
//!
//! // In your implementation:
//! let coordinator: Box<dyn PipelineCoordinationPort> = // ... get implementation
//! let execution_id = coordinator.coordinate_pipeline_execution(
//!     PipelineId::new(),
//!     ExecutionContext::default()
//! ).await?;
//! ```

pub mod conflict_port;
pub mod coordination_port;
pub mod policy_port;

pub use conflict_port::ConflictResolutionPort;
pub use coordination_port::{
    CoordinationResult, CoordinationStatus, ExecutionContext, PipelineCoordinationPort,
};
pub use policy_port::ResiliencePolicyPort;
