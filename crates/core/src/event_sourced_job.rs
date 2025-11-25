//! Event-Sourced Job Aggregate
//!
//! This module provides an event-sourced implementation of the Job aggregate.
//! State changes are recorded as domain events, allowing full audit trails
//! and temporal queries.

pub use crate::error::DomainError;
pub use crate::job_definitions::{ExecResult, JobId, JobSpec, JobState, ResourceQuota};

use crate::Result;
use crate::events::{DomainEvent, EventSourcedAggregate};
use crate::job_events::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Event-Sourced Job Aggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSourcedJob {
    pub id: JobId,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub image: String,
    pub state: JobState,
    pub tenant_id: Option<String>,
    pub worker_id: Option<Uuid>,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
    pub version: u64,
}

impl EventSourcedJob {
    /// Create a new job (generates JobCreatedEvent)
    pub fn create(
        id: JobId,
        tenant_id: Option<String>,
        name: String,
        command: String,
        image: String,
    ) -> (Self, JobCreatedEvent) {
        let now = Utc::now();
        let job = Self {
            id,
            name: name.clone(),
            description: None,
            command: command.clone(),
            image: image.clone(),
            state: JobState::new(JobState::PENDING.to_string()).unwrap(),
            tenant_id: tenant_id.clone(),
            worker_id: None,
            retry_count: 0,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            metadata: HashMap::new(),
            version: 1,
        };

        let event = JobCreatedEvent::new(id.0, tenant_id.unwrap_or_default(), name, command, image);

        (job, event)
    }

    /// Start the job
    pub fn start(&mut self) -> Result<JobStartedEvent, DomainError> {
        if !matches!(self.state, JobState { inner: ref s } if s == JobState::PENDING) {
            return Err(DomainError::InvalidStateTransition(format!(
                "Cannot start job in state {}",
                self.state.inner
            )));
        }

        let event = JobStartedEvent::new(self.id.0, self.worker_id);
        self.apply_start(&event);
        Ok(event)
    }

    /// Complete the job
    pub fn complete(
        &mut self,
        exit_code: i32,
        duration_ms: u64,
        output: Option<String>,
    ) -> Result<JobCompletedEvent, DomainError> {
        if !matches!(self.state, JobState { inner: ref s } if s == JobState::RUNNING) {
            return Err(DomainError::InvalidStateTransition(format!(
                "Cannot complete job in state {}",
                self.state.inner
            )));
        }

        let event = JobCompletedEvent::new(self.id.0, exit_code, duration_ms, output);
        self.apply_complete(&event);
        Ok(event)
    }

    /// Fail the job
    pub fn fail(&mut self, error_message: String) -> Result<JobFailedEvent, DomainError> {
        if !matches!(self.state, JobState { inner: ref s } if s == JobState::RUNNING) {
            return Err(DomainError::InvalidStateTransition(format!(
                "Cannot fail job in state {}",
                self.state.inner
            )));
        }

        let event = JobFailedEvent::new(self.id.0, error_message, self.retry_count);
        self.apply_fail(&event);
        Ok(event)
    }

    /// Retry the job
    pub fn retry(&mut self, reason: String) -> Result<JobRetriedEvent, DomainError> {
        if !matches!(self.state, JobState { inner: ref s } if s == JobState::FAILED) {
            return Err(DomainError::InvalidStateTransition(format!(
                "Cannot retry job in state {}",
                self.state.inner
            )));
        }

        let event = JobRetriedEvent::new(self.id.0, self.retry_count + 1, reason);
        self.apply_retry(&event);
        Ok(event)
    }

    /// Cancel the job
    pub fn cancel(&mut self, reason: String) -> Result<JobCancelledEvent, DomainError> {
        if matches!(self.state, JobState { inner: ref s } if s == JobState::COMPLETED || s == JobState::FAILED)
        {
            return Err(DomainError::InvalidStateTransition(format!(
                "Cannot cancel job in state {}",
                self.state.inner
            )));
        }

        let event = JobCancelledEvent::new(self.id.0, reason);
        self.apply_cancel(&event);
        Ok(event)
    }

    /// Update metadata
    pub fn update_metadata(
        &mut self,
        metadata: HashMap<String, String>,
    ) -> Result<JobMetadataUpdatedEvent, DomainError> {
        let event = JobMetadataUpdatedEvent::new(self.id.0, metadata.clone());
        self.apply_metadata_update(&event);
        Ok(event)
    }

    // Event application methods (mutating state)
    fn apply_start(&mut self, event: &JobStartedEvent) {
        self.state = JobState::new(JobState::RUNNING.to_string()).unwrap();
        self.started_at = Some(event.started_at);
        self.updated_at = event.started_at;
        self.version = event.version();
    }

    fn apply_complete(&mut self, event: &JobCompletedEvent) {
        self.state = JobState::new(JobState::COMPLETED.to_string()).unwrap();
        self.completed_at = Some(event.completed_at);
        self.updated_at = event.completed_at;
        self.version = event.version();
    }

    fn apply_fail(&mut self, event: &JobFailedEvent) {
        self.state = JobState::new(JobState::FAILED.to_string()).unwrap();
        self.updated_at = event.failed_at;
        self.retry_count = event.retry_count;
        self.version = event.version();
    }

    fn apply_retry(&mut self, event: &JobRetriedEvent) {
        self.state = JobState::new(JobState::PENDING.to_string()).unwrap();
        self.updated_at = event.retry_at;
        self.retry_count = event.retry_count;
        self.version = event.version();
    }

    fn apply_cancel(&mut self, event: &JobCancelledEvent) {
        self.state = JobState::new(JobState::CANCELLED.to_string()).unwrap();
        self.updated_at = event.cancelled_at;
        self.version = event.version();
    }

    fn apply_metadata_update(&mut self, event: &JobMetadataUpdatedEvent) {
        self.metadata = event.metadata.clone();
        self.updated_at = event.updated_at;
        self.version = event.version();
    }
}

// Implement EventSourcedAggregate for EventSourcedJob
impl EventSourcedAggregate for EventSourcedJob {
    type Event = Box<dyn DomainEvent>;

    fn version(&self) -> u64 {
        self.version
    }

    fn take_uncommitted_events(&mut self) -> Vec<Self::Event> {
        // In a real implementation, this would return uncommitted events
        // For now, we return an empty vector
        Vec::new()
    }

    fn mark_events_as_committed(&mut self) {
        // This would be called after successfully saving events
    }

    fn load_from_events(&mut self, events: &[Self::Event]) {
        for event in events {
            // Reconstruct state by applying events
            self.apply_event(event.as_ref());
        }
    }
}

impl EventSourcedJob {
    fn apply_event(&mut self, event: &dyn DomainEvent) {
        match event.event_type() {
            "JobCreated" => {
                if let Some(job_event) = event.downcast_ref::<JobCreatedEvent>() {
                    self.name = job_event.name.clone();
                    self.command = job_event.command.clone();
                    self.image = job_event.image.clone();
                    self.tenant_id = Some(job_event.tenant_id.clone());
                    self.created_at = job_event.created_at;
                    self.updated_at = job_event.created_at;
                    self.version = job_event.version();
                }
            }
            "JobStarted" => {
                if let Some(job_event) = event.downcast_ref::<JobStartedEvent>() {
                    self.apply_start(job_event);
                }
            }
            "JobCompleted" => {
                if let Some(job_event) = event.downcast_ref::<JobCompletedEvent>() {
                    self.apply_complete(job_event);
                }
            }
            "JobFailed" => {
                if let Some(job_event) = event.downcast_ref::<JobFailedEvent>() {
                    self.apply_fail(job_event);
                }
            }
            "JobRetried" => {
                if let Some(job_event) = event.downcast_ref::<JobRetriedEvent>() {
                    self.apply_retry(job_event);
                }
            }
            "JobCancelled" => {
                if let Some(job_event) = event.downcast_ref::<JobCancelledEvent>() {
                    self.apply_cancel(job_event);
                }
            }
            "JobMetadataUpdated" => {
                if let Some(job_event) = event.downcast_ref::<JobMetadataUpdatedEvent>() {
                    self.apply_metadata_update(job_event);
                }
            }
            _ => {
                // Unknown event type - log and ignore
                println!("Unknown event type: {}", event.event_type());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_definitions::{JobId, JobSpec};

    #[test]
    fn test_create_job() {
        let id = JobId::new();
        let (job, event) = EventSourcedJob::create(
            id,
            Some("tenant-1".to_string()),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        assert_eq!(job.name, "Test Job");
        assert_eq!(job.state.inner, JobState::PENDING);
        assert_eq!(event.event_type(), "JobCreated");
    }

    #[test]
    fn test_start_job() {
        let id = JobId::new();
        let (mut job, _) = EventSourcedJob::create(
            id,
            Some("tenant-1".to_string()),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        let start_event = job.start().unwrap();
        assert_eq!(job.state.inner, JobState::RUNNING);
        assert!(job.started_at.is_some());
    }

    #[test]
    fn test_complete_job() {
        let id = JobId::new();
        let (mut job, _) = EventSourcedJob::create(
            id,
            Some("tenant-1".to_string()),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        job.start().unwrap();
        let complete_event = job.complete(0, 5000, Some("Success".to_string())).unwrap();

        assert_eq!(job.state.inner, JobState::COMPLETED);
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_retry_job() {
        let id = JobId::new();
        let (mut job, _) = EventSourcedJob::create(
            id,
            Some("tenant-1".to_string()),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        job.start().unwrap();
        job.fail("Test error".to_string()).unwrap();
        let retry_event = job.retry("Retry reason".to_string()).unwrap();

        assert_eq!(job.state.inner, JobState::PENDING);
        assert_eq!(job.retry_count, 1);
    }

    #[test]
    fn test_cancel_job() {
        let id = JobId::new();
        let (mut job, _) = EventSourcedJob::create(
            id,
            Some("tenant-1".to_string()),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        let cancel_event = job.cancel("User request".to_string()).unwrap();
        assert_eq!(job.state.inner, JobState::CANCELLED);
    }

    #[test]
    fn test_update_metadata() {
        let id = JobId::new();
        let (mut job, _) = EventSourcedJob::create(
            id,
            Some("tenant-1".to_string()),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        let update_event = job.update_metadata(metadata.clone()).unwrap();

        assert_eq!(job.metadata, metadata);
    }
}
