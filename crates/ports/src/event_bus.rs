//! Event Bus Port - Zero-copy event communication
//!
//! Defines interfaces for high-performance in-memory event bus.

use async_trait::async_trait;
use hodei_core::PipelineId;
use hodei_core::{JobId, JobSpec, WorkerId};
use std::sync::Arc;

/// Zero-copy log entry (Arc wrapper for data)
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub job_id: JobId,
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

/// System events for inter-component communication
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Job created event
    JobCreated(JobSpec),

    /// Job scheduled event
    JobScheduled { job_id: JobId, worker_id: WorkerId },

    /// Job started event
    JobStarted { job_id: JobId, worker_id: WorkerId },

    /// Job completed event
    JobCompleted { job_id: JobId, exit_code: i32 },

    /// Job failed event
    JobFailed { job_id: JobId, error: String },

    /// Worker connected event
    WorkerConnected { worker_id: WorkerId },

    /// Worker disconnected event
    WorkerDisconnected { worker_id: WorkerId },

    /// Worker heartbeat event
    WorkerHeartbeat {
        worker_id: WorkerId,
        timestamp: std::time::SystemTime,
    },

    /// Log chunk received event (zero-copy via Arc)
    LogChunkReceived(LogEntry),

    /// Pipeline created event (zero-copy via Arc)
    PipelineCreated(Arc<hodei_core::Pipeline>),

    /// Pipeline started event
    PipelineStarted { pipeline_id: PipelineId },

    /// Pipeline completed event
    PipelineCompleted { pipeline_id: PipelineId },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_publisher_trait_exists() {
        // This test verifies the trait exists and compiles
        let _publisher: Option<Box<dyn EventPublisher + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[tokio::test]
    async fn test_event_subscriber_trait_exists() {
        // This test verifies the trait exists and compiles
        let _subscriber: Option<Box<dyn EventSubscriber + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[test]
    fn test_event_bus_error_constructors() {
        // Test error constructors
        let _full = EventBusError::Full(100);
        let _dropped = EventBusError::Dropped;
        let _closed = EventBusError::Closed;
        let _internal = EventBusError::Internal("error".to_string());
    }

    #[test]
    fn test_event_bus_error_display() {
        let full = EventBusError::Full(100);
        let dropped = EventBusError::Dropped;
        let closed = EventBusError::Closed;
        let internal = EventBusError::Internal("Test error".to_string());

        assert!(full.to_string().contains("Bus full"));
        assert!(dropped.to_string().contains("Subscriber dropped"));
        assert!(closed.to_string().contains("Channel closed"));
        assert!(internal.to_string().contains("Internal error"));
    }
}
