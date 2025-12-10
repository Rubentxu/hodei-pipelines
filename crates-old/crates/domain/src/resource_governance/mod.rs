//! Resource Governance Bounded Context
//!
//! This module handles all resource governance related functionality,
//! including resource pools, quotas, and global resource control.

pub mod entities;
pub mod value_objects;
pub mod domain_services;

pub use domain_services::controller::{
    AllocationResult, GRCConfig, GRCMetrics, GlobalResourceController, PoolSelectionResult,
};
pub use entities::domain::*;
pub use domain_services::repository::ResourcePoolRepository;
