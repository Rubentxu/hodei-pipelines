//! Domain Services - Business Logic Services
//!
//! Domain Services contain business logic that doesn't naturally fit
//! within a single entity or value object.

pub mod priority_calculator;

// Re-export for convenience
pub use priority_calculator::PriorityCalculator;

use crate::{JobId, WorkerCapabilities, WorkerId};

/// WorkerNode - cluster state representation for domain services
#[derive(Debug, Clone)]
pub struct WorkerNode {
    pub id: WorkerId,
    pub capabilities: WorkerCapabilities,
    pub usage: ResourceUsage,
    pub reserved_jobs: Vec<JobId>,
    pub last_heartbeat: std::time::Instant,
}

impl WorkerNode {
    pub fn is_healthy(&self) -> bool {
        self.last_heartbeat.elapsed() < std::time::Duration::from_secs(30)
    }

    pub fn has_capacity(&self, required_cores: u32, required_memory_mb: u64) -> bool {
        self.capabilities.cpu_cores >= required_cores
            && self.capabilities.memory_gb * 1024 >= required_memory_mb
    }
}

/// Resource usage snapshot
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub io_percent: f64,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_mb: 0,
            io_percent: 0.0,
        }
    }

    pub fn update(&mut self, cpu_percent: f64, memory_mb: u64, io_percent: f64) {
        self.cpu_percent = cpu_percent;
        self.memory_mb = memory_mb;
        self.io_percent = io_percent;
    }
}
