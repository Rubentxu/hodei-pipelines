//! Queueing Domain Types
//!
//! Core types for queue management in the domain layer.

use std::collections::HashMap;
use std::time::Instant;

/// Represents a job in the queue
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedJob {
    pub job_id: String,
    pub tenant_id: String,
    pub priority: u8,
    pub weight: f64,
    pub arrival_time: Instant,
    pub virtual_finish_time: f64,
}

/// Queue state for policy decision making
#[derive(Debug, Clone)]
pub struct QueueState {
    pub current_size: usize,
    pub max_capacity: usize,
    pub tenant_loads: HashMap<String, TenantLoad>,
    pub global_virtual_time: f64,
}

/// Tenant load information for fair sharing decisions
#[derive(Debug, Clone)]
pub struct TenantLoad {
    pub tenant_id: String,
    pub active_jobs: usize,
    pub total_weight: f64,
    pub last_allocation: Option<Instant>,
}

impl QueuedJob {
    pub fn new(job_id: String, tenant_id: String, priority: u8, weight: f64) -> Self {
        Self {
            job_id,
            tenant_id,
            priority,
            weight,
            arrival_time: Instant::now(),
            virtual_finish_time: 0.0,
        }
    }
}

impl QueueState {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            current_size: 0,
            max_capacity,
            tenant_loads: HashMap::new(),
            global_virtual_time: 0.0,
        }
    }

    pub fn is_at_capacity(&self) -> bool {
        self.current_size >= self.max_capacity
    }

    pub fn update_tenant_load(&mut self, tenant_id: String, load: TenantLoad) {
        self.tenant_loads.insert(tenant_id, load);
    }
}
