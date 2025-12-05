//! Orchestrator Bounded Context
//!
//! This bounded context is responsible for coordinating pipeline execution,
//! managing dependencies, and handling resilience policies for distributed
//! pipeline workflows.
//!
//! # Architecture
//!
//! The orchestrator follows Hexagonal Architecture (Ports & Adapters):
//!
//! - **Application Layer**: Use cases for pipeline coordination
//! - **Domain Layer**: Core orchestration logic and business rules
//! - **Ports**: Trait definitions for external dependencies
//! - **Adapters**: Concrete implementations

pub mod services;

/// Re-export common types for convenience
pub use services::pipeline_execution_orchestrator::{
    ConcreteOrchestrator, PipelineExecutionConfig, PipelineExecutionOrchestrator,
    PipelineExecutionService, PipelineService,
};

/// Re-export pipeline CRUD types
pub use services::pipeline_crud::{
    CreatePipelineRequest, ExecutePipelineRequest, ListPipelinesFilter, ListPipelinesResponse,
    PipelineCrudConfig, PipelineCrudError, PipelineCrudService, PipelineSummary,
    UpdatePipelineRequest,
};
