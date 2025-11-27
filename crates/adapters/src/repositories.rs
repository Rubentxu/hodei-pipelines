//! In-Memory Repository Implementations
//!
//! Temporarily disabled - needs update to match new trait signatures
//! These structures are kept for future reference but not exported

use async_trait::async_trait;
use hodei_core::{Job, JobId, Pipeline, PipelineId, Worker, WorkerId};
use hodei_ports::{JobRepository, PipelineRepository, WorkerRepository};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// In-memory job repository - Temporarily disabled
#[cfg(feature = "inmemory")]
pub struct InMemoryJobRepository {
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
}

#[cfg(feature = "inmemory")]
impl InMemoryJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(feature = "inmemory")]
impl Default for InMemoryJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory worker repository - Temporarily disabled
#[cfg(feature = "inmemory")]
pub struct InMemoryWorkerRepository {
    workers: Arc<RwLock<HashMap<WorkerId, Worker>>>,
}

#[cfg(feature = "inmemory")]
impl InMemoryWorkerRepository {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(feature = "inmemory")]
impl Default for InMemoryWorkerRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory pipeline repository - Temporarily disabled
#[cfg(feature = "inmemory")]
pub struct InMemoryPipelineRepository {
    pipelines: Arc<RwLock<HashMap<PipelineId, Pipeline>>>,
}

#[cfg(feature = "inmemory")]
impl InMemoryPipelineRepository {
    pub fn new() -> Self {
        Self {
            pipelines: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(feature = "inmemory")]
impl Default for InMemoryPipelineRepository {
    fn default() -> Self {
        Self::new()
    }
}
