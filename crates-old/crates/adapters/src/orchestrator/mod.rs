//! Orchestrator Bounded Context Adapters
//!
//! This module contains implementations (adapters) for the Orchestrator bounded context,
//! which coordinates between different bounded contexts in the system.

pub mod facade_adapter;

pub use facade_adapter::FacadeOrchestratorAdapter;
