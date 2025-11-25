//! Job Domain Events
//!
//! This module defines all domain events for the Job aggregate.
//! These events represent state changes that can be replayed for event sourcing.

use crate::events::DomainEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Event: JobCreated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCreatedEvent {
    pub job_id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub command: String,
    pub image: String,
    pub created_at: DateTime<Utc>,
}

impl JobCreatedEvent {
    pub fn new(
        job_id: Uuid,
        tenant_id: String,
        name: String,
        command: String,
        image: String,
    ) -> Self {
        Self {
            job_id,
            tenant_id,
            name,
            command,
            image,
            created_at: Utc::now(),
        }
    }
}

impl DomainEvent for JobCreatedEvent {
    fn event_id(&self) -> Uuid {
        // Use job_id as event_id for simplicity
        self.job_id
    }

    fn event_type(&self) -> &'static str {
        "JobCreated"
    }

    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn version(&self) -> u64 {
        1
    }
}

/// Event: JobStarted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStartedEvent {
    pub job_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub worker_id: Option<Uuid>,
}

impl JobStartedEvent {
    pub fn new(job_id: Uuid, worker_id: Option<Uuid>) -> Self {
        Self {
            job_id,
            started_at: Utc::now(),
            worker_id,
        }
    }
}

impl DomainEvent for JobStartedEvent {
    fn event_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    fn event_type(&self) -> &'static str {
        "JobStarted"
    }

    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    fn version(&self) -> u64 {
        2
    }
}

/// Event: JobCompleted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCompletedEvent {
    pub job_id: Uuid,
    pub completed_at: DateTime<Utc>,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub output: Option<String>,
}

impl JobCompletedEvent {
    pub fn new(job_id: Uuid, exit_code: i32, duration_ms: u64, output: Option<String>) -> Self {
        Self {
            job_id,
            completed_at: Utc::now(),
            exit_code,
            duration_ms,
            output,
        }
    }
}

impl DomainEvent for JobCompletedEvent {
    fn event_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    fn event_type(&self) -> &'static str {
        "JobCompleted"
    }

    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.completed_at
    }

    fn version(&self) -> u64 {
        3
    }
}

/// Event: JobFailed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobFailedEvent {
    pub job_id: Uuid,
    pub failed_at: DateTime<Utc>,
    pub error_message: String,
    pub retry_count: u32,
}

impl JobFailedEvent {
    pub fn new(job_id: Uuid, error_message: String, retry_count: u32) -> Self {
        Self {
            job_id,
            failed_at: Utc::now(),
            error_message,
            retry_count,
        }
    }
}

impl DomainEvent for JobFailedEvent {
    fn event_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    fn event_type(&self) -> &'static str {
        "JobFailed"
    }

    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.failed_at
    }

    fn version(&self) -> u64 {
        4
    }
}

/// Event: JobRetried
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRetriedEvent {
    pub job_id: Uuid,
    pub retry_at: DateTime<Utc>,
    pub retry_count: u32,
    pub reason: String,
}

impl JobRetriedEvent {
    pub fn new(job_id: Uuid, retry_count: u32, reason: String) -> Self {
        Self {
            job_id,
            retry_at: Utc::now(),
            retry_count,
            reason,
        }
    }
}

impl DomainEvent for JobRetriedEvent {
    fn event_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    fn event_type(&self) -> &'static str {
        "JobRetried"
    }

    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.retry_at
    }

    fn version(&self) -> u64 {
        5
    }
}

/// Event: JobCancelled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCancelledEvent {
    pub job_id: Uuid,
    pub cancelled_at: DateTime<Utc>,
    pub reason: String,
}

impl JobCancelledEvent {
    pub fn new(job_id: Uuid, reason: String) -> Self {
        Self {
            job_id,
            cancelled_at: Utc::now(),
            reason,
        }
    }
}

impl DomainEvent for JobCancelledEvent {
    fn event_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    fn event_type(&self) -> &'static str {
        "JobCancelled"
    }

    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.cancelled_at
    }

    fn version(&self) -> u64 {
        6
    }
}

/// Event: JobMetadataUpdated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadataUpdatedEvent {
    pub job_id: Uuid,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl JobMetadataUpdatedEvent {
    pub fn new(job_id: Uuid, metadata: HashMap<String, String>) -> Self {
        Self {
            job_id,
            updated_at: Utc::now(),
            metadata,
        }
    }
}

impl DomainEvent for JobMetadataUpdatedEvent {
    fn event_id(&self) -> Uuid {
        Uuid::new_v4()
    }

    fn event_type(&self) -> &'static str {
        "JobMetadataUpdated"
    }

    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn version(&self) -> u64 {
        7
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DomainEvent;

    #[test]
    fn test_job_created_event() {
        let job_id = Uuid::new_v4();
        let event = JobCreatedEvent::new(
            job_id,
            "tenant-1".to_string(),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        assert_eq!(event.event_id(), job_id);
        assert_eq!(event.event_type(), "JobCreated");
        assert_eq!(event.aggregate_id(), job_id);
        assert_eq!(event.version(), 1);
    }

    #[test]
    fn test_job_started_event() {
        let job_id = Uuid::new_v4();
        let event = JobStartedEvent::new(job_id, None);

        assert_eq!(event.event_type(), "JobStarted");
        assert_eq!(event.aggregate_id(), job_id);
        assert_eq!(event.version(), 2);
    }

    #[test]
    fn test_job_completed_event() {
        let job_id = Uuid::new_v4();
        let event = JobCompletedEvent::new(job_id, 0, 5000, Some("Success".to_string()));

        assert_eq!(event.event_type(), "JobCompleted");
        assert_eq!(event.aggregate_id(), job_id);
        assert_eq!(event.version(), 3);
        assert_eq!(event.exit_code, 0);
    }

    #[test]
    fn test_job_failed_event() {
        let job_id = Uuid::new_v4();
        let event = JobFailedEvent::new(job_id, "Memory error".to_string(), 1);

        assert_eq!(event.event_type(), "JobFailed");
        assert_eq!(event.aggregate_id(), job_id);
        assert_eq!(event.version(), 4);
        assert_eq!(event.retry_count, 1);
    }
}
