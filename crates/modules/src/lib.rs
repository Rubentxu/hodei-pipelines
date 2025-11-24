//! Application Modules
//!
//! This crate contains the application layer (use cases) that orchestrates
//! the domain entities through the ports.

pub mod orchestrator;
pub mod scheduler;

pub use crate::orchestrator::{OrchestratorConfig, OrchestratorModule};
pub use crate::scheduler::state_machine::{
    SchedulingContext, SchedulingState, SchedulingStateMachine,
};
pub use crate::scheduler::{SchedulerConfig, SchedulerModule};
