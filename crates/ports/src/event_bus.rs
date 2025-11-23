//! Event Bus Port - Zero-copy event communication
//!
//! Defines interfaces for high-performance in-memory event bus.

use async_trait::async_trait;
use std::sync::Arc;

/// Zero-copy log entry (Arc wrapper for data)
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub job_id: hodei_core::JobId,
    pub data: Arc<Vec<u8>>,
    pub stream_type: StreamType,
    pub sequence: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum StreamType {
    Stdout,
    Stderr,
}

/// Zero-copy system events (Arc for efficient cloning)
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Job created event (zero-copy via Arc)
    JobCreated(Arc<hodei_core::JobSpec>),

    /// Job scheduled event
    JobScheduled {
        job_id: hodei_core::JobId,
        worker_id: hodei_core::WorkerId,
    },

    /// Job started event
    JobStarted {
        job_id: hodei_core::JobId,
        worker_id: hodei_core::WorkerId,
    },

    /// Job completed event
    JobCompleted {
        job_id: hodei_core::JobId,
        exit_code: i32,
    },

    /// Job failed event
    JobFailed {
        job_id: hodei_core::JobId,
        error: String,
    },

    /// Worker connected event
    WorkerConnected { worker_id: hodei_core::WorkerId },

    /// Worker disconnected event
    WorkerDisconnected { worker_id: hodei_core::WorkerId },

    /// Worker heartbeat event
    WorkerHeartbeat {
        worker_id: hodei_core::WorkerId,
        timestamp: std::time::SystemTime,
    },

    /// Log chunk received event (zero-copy via Arc)
    LogChunkReceived(LogEntry),

    /// Pipeline created event (zero-copy via Arc)
    PipelineCreated(Arc<hodei_core::Pipeline>),

    /// Pipeline started event
    PipelineStarted { pipeline_id: hodei_core::PipelineId },

    /// Pipeline completed event
    PipelineCompleted { pipeline_id: hodei_core::PipelineId },
}

/// Event bus error types
#[derive(thiserror::Error, Debug)]
pub enum EventBusError {
    #[error("Bus full (capacity: {0})")]
    Full(usize),

    #[error("Subscriber dropped")]
    Dropped,

    #[error("Channel closed")]
    Closed,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Alias for compatibility
pub type BusError = EventBusError;

/// Event publisher port
#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: SystemEvent) -> Result<(), EventBusError>;

    async fn publish_batch(&self, events: Vec<SystemEvent>) -> Result<(), EventBusError> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

/// Event receiver wrapper
#[derive(Debug)]
pub struct EventReceiver {
    pub receiver: tokio::sync::broadcast::Receiver<SystemEvent>,
}

impl EventReceiver {
    pub async fn recv(&mut self) -> Result<SystemEvent, EventBusError> {
        self.receiver
            .recv()
            .await
            .map_err(|_| EventBusError::Dropped)
    }

    pub fn try_recv(&mut self) -> Result<SystemEvent, EventBusError> {
        self.receiver.try_recv().map_err(|_| EventBusError::Dropped)
    }
}

/// Event subscriber port
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn subscribe(&self) -> Result<EventReceiver, EventBusError>;
}
