//! Event Bus Port - Network-transmittable event communication
//!
//! Defines interfaces for event bus that supports both in-memory and distributed (NATS) implementations.

use async_trait::async_trait;
use hodei_pipelines_core::{ExecutionId, PipelineId};
use hodei_pipelines_core::{JobId, JobSpec, WorkerId};
use serde::{Deserialize, Serialize};

/// Log entry (cloneable data for network transmission)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub job_id: JobId,
    pub data: Vec<u8>,
    pub stream_type: StreamType,
    pub sequence: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StreamType {
    Stdout,
    Stderr,
}

/// System events for inter-component communication
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        timestamp: chrono::DateTime<chrono::Utc>,
        resource_usage: hodei_pipelines_core::ResourceUsage,
    },

    /// Log chunk received event
    LogChunkReceived(LogEntry),

    /// Pipeline created event
    PipelineCreated(hodei_pipelines_core::Pipeline),

    /// Pipeline started event
    PipelineStarted { pipeline_id: PipelineId },

    /// Pipeline completed event
    PipelineCompleted { pipeline_id: PipelineId },

    /// Pipeline execution started event
    PipelineExecutionStarted {
        pipeline_id: PipelineId,
        execution_id: ExecutionId,
    },
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
    use chrono::Utc;
    use hodei_pipelines_core::{JobId, JobSpec, ResourceQuota, WorkerId};
    use std::collections::HashMap;

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

    // ============= TDD Tests for JSON Serialization (US-NATS-01) =============

    #[test]
    fn test_system_event_job_created_serialization() {
        // Create a simple JobSpec
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "alpine:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 0,
            env,
            secret_refs: vec![],
        };

        let event = SystemEvent::JobCreated(job_spec.clone());

        // Test: Should serialize to JSON
        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "JobCreated should serialize to JSON: {:?}",
            result.err()
        );

        let json = result.unwrap();
        assert!(json.contains("JobCreated"));

        // Test: Should deserialize back
        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&json);
        assert!(deserialized.is_ok(), "Should deserialize from JSON");

        if let Ok(SystemEvent::JobCreated(deserialized_spec)) = deserialized {
            assert_eq!(deserialized_spec.name, job_spec.name);
            assert_eq!(deserialized_spec.image, job_spec.image);
        }
    }

    #[test]
    fn test_system_event_job_scheduled_serialization() {
        let event = SystemEvent::JobScheduled {
            job_id: JobId::new(),
            worker_id: WorkerId::new(),
        };

        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "JobScheduled should serialize: {:?}",
            result.err()
        );

        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&result.unwrap());
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }

    #[test]
    fn test_system_event_job_completed_serialization() {
        let event = SystemEvent::JobCompleted {
            job_id: JobId::new(),
            exit_code: 0,
        };

        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "JobCompleted should serialize: {:?}",
            result.err()
        );

        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&result.unwrap());
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }

    #[test]
    fn test_system_event_job_failed_serialization() {
        let event = SystemEvent::JobFailed {
            job_id: JobId::new(),
            error: "Something went wrong".to_string(),
        };

        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "JobFailed should serialize: {:?}",
            result.err()
        );

        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&result.unwrap());
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }

    #[test]
    fn test_system_event_worker_connected_serialization() {
        let event = SystemEvent::WorkerConnected {
            worker_id: WorkerId::new(),
        };

        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "WorkerConnected should serialize: {:?}",
            result.err()
        );

        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&result.unwrap());
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }

    #[test]
    fn test_system_event_worker_heartbeat_serialization() {
        let event = SystemEvent::WorkerHeartbeat {
            worker_id: WorkerId::new(),
            timestamp: chrono::Utc::now(),
            resource_usage: hodei_pipelines_core::ResourceUsage {
                cpu_usage_m: 100,
                memory_usage_mb: 1024,
                active_jobs: 1,
                disk_read_mb: 0.0,
                disk_write_mb: 0.0,
                network_sent_mb: 0.0,
                network_received_mb: 0.0,
                gpu_utilization_percent: 0.0,
                timestamp: chrono::Utc::now().timestamp_millis(),
            },
        };

        // Test: Should serialize to JSON
        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "WorkerHeartbeat should serialize: {:?}",
            result.err()
        );

        let json = result.unwrap();
        assert!(json.contains("WorkerHeartbeat"));

        // Test: Should deserialize back
        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&json);
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }

    #[test]
    fn test_system_event_log_chunk_received_serialization() {
        let log_entry = LogEntry {
            job_id: JobId::new(),
            data: vec![104, 101, 108, 108, 111], // "hello" in bytes
            stream_type: StreamType::Stdout,
            sequence: 1,
            timestamp: Utc::now(),
        };

        let event = SystemEvent::LogChunkReceived(log_entry);

        // Test: Should serialize to JSON
        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "LogChunkReceived should serialize: {:?}",
            result.err()
        );

        let json = result.unwrap();
        assert!(json.contains("LogChunkReceived"));

        // Test: Should deserialize back
        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&json);
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }

    #[test]
    fn test_system_event_pipeline_created_serialization() {
        use hodei_pipelines_core::{Pipeline, PipelineId};

        let pipeline = Pipeline::new(PipelineId::new(), "test-pipeline".to_string(), vec![])
            .expect("Failed to create pipeline");

        let event = SystemEvent::PipelineCreated(pipeline);

        // Test: Should serialize to JSON
        let result = serde_json::to_string(&event);
        assert!(
            result.is_ok(),
            "PipelineCreated should serialize: {:?}",
            result.err()
        );

        let json = result.unwrap();
        assert!(json.contains("PipelineCreated"));

        // Test: Should deserialize back
        let deserialized: Result<SystemEvent, _> = serde_json::from_str(&json);
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }
}
