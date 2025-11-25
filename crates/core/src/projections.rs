//! Projections for Read Models
//!
//! This module provides projections that transform events into denormalized
//! read models for efficient querying.

use crate::events::{DomainEvent, EventStore};
use crate::job_cqrs::{JobEventView, JobView};
use crate::job_definitions::{JobId, JobState};
use crate::job_events::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Projection for Job read model
pub struct JobProjection {
    event_store: Arc<dyn EventStore>,
}

impl JobProjection {
    pub fn new(event_store: Arc<dyn EventStore>) -> Self {
        Self { event_store }
    }

    /// Get a job by ID
    pub async fn get_job(&self, job_id: JobId) -> Result<Option<JobView>, ProjectionError> {
        let events = self
            .event_store
            .load_events(job_id.0, None)
            .await
            .map_err(|e| ProjectionError::EventStoreError(e.to_string()))?;

        if events.is_empty() {
            return Ok(None);
        }

        let view = self::build_job_view(job_id, &events)?;
        Ok(Some(view))
    }

    /// List jobs with optional filtering
    pub async fn list_jobs(
        &self,
        tenant_id: Option<String>,
        state: Option<JobState>,
        limit: usize,
    ) -> Result<Vec<JobView>, ProjectionError> {
        // For simplicity, we'll load all events and filter
        // In production, you'd want to maintain an index or use a separate read model
        let all_events = self.get_all_job_events().await?;

        // Group events by job ID
        let mut jobs_by_id: HashMap<Uuid, Vec<Box<dyn DomainEvent>>> = HashMap::new();
        for event_view in all_events {
            jobs_by_id
                .entry(event_view.event_id)
                .or_insert_with(Vec::new);
        }

        let mut views = Vec::new();
        for (job_id, events) in jobs_by_id {
            if let Ok(view) = self::build_job_view(JobId(job_id), &events) {
                // Apply filters
                if tenant_id.is_some() && view.tenant_id != tenant_id {
                    continue;
                }
                if state.is_some() && view.state != state.clone().unwrap() {
                    continue;
                }
                views.push(view);
            }
        }

        // Sort by created_at descending and limit
        views.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if views.len() > limit {
            views.truncate(limit);
        }

        Ok(views)
    }

    /// Get job event history
    pub async fn get_job_history(
        &self,
        job_id: JobId,
    ) -> Result<Vec<JobEventView>, ProjectionError> {
        let events = self
            .event_store
            .load_events(job_id.0, None)
            .await
            .map_err(|e| ProjectionError::EventStoreError(e.to_string()))?;

        let mut views = Vec::new();
        for event in events {
            let view = JobEventView {
                event_id: event.event_id(),
                event_type: event.event_type().to_string(),
                occurred_at: event.occurred_at(),
                version: event.version(),
                data: serde_json::to_value(event.as_ref())
                    .map_err(|e| ProjectionError::SerializationError(e.to_string()))?,
            };
            views.push(view);
        }

        // Sort by version
        views.sort_by(|a, b| a.version.cmp(&b.version));

        Ok(views)
    }

    /// Get all job events (for building projections)
    async fn get_all_job_events(&self) -> Result<Vec<JobEventView>, ProjectionError> {
        // In a real implementation, you'd want to query by event type
        // For now, we'll load all events and filter
        let events = self
            .event_store
            .load_events_by_type("JobCreated", None)
            .await
            .map_err(|e| ProjectionError::EventStoreError(e.to_string()))?;

        let mut views = Vec::new();
        for event in events {
            let view = JobEventView {
                event_id: event.event_id(),
                event_type: event.event_type().to_string(),
                occurred_at: event.occurred_at(),
                version: event.version(),
                data: serde_json::to_value(event.as_ref())
                    .map_err(|e| ProjectionError::SerializationError(e.to_string()))?,
            };
            views.push(view);
        }

        Ok(views)
    }
}

/// Build a JobView from events
fn build_job_view(
    job_id: JobId,
    events: &[Box<dyn DomainEvent>],
) -> Result<JobView, ProjectionError> {
    let mut name = "".to_string();
    let mut description = None;
    let mut command = "".to_string();
    let mut image = "".to_string();
    let mut state = JobState::new(JobState::PENDING.to_string()).unwrap();
    let mut tenant_id = None;
    let mut worker_id = None;
    let mut retry_count = 0;
    let mut created_at = Utc::now();
    let mut updated_at = Utc::now();
    let mut started_at = None;
    let mut completed_at = None;
    let mut metadata = HashMap::new();
    let mut version = 0;

    // Apply events in order
    for event in events {
        match event.event_type() {
            "JobCreated" => {
                if let Some(job_event) = event.downcast_ref::<JobCreatedEvent>() {
                    name = job_event.name.clone();
                    command = job_event.command.clone();
                    image = job_event.image.clone();
                    tenant_id = Some(job_event.tenant_id.clone());
                    created_at = job_event.created_at;
                    updated_at = job_event.created_at;
                    version = job_event.version();
                }
            }
            "JobStarted" => {
                if let Some(job_event) = event.downcast_ref::<JobStartedEvent>() {
                    state = JobState::new(JobState::RUNNING.to_string()).unwrap();
                    started_at = Some(job_event.started_at);
                    updated_at = job_event.started_at;
                    version = job_event.version();
                }
            }
            "JobCompleted" => {
                if let Some(job_event) = event.downcast_ref::<JobCompletedEvent>() {
                    state = JobState::new(JobState::COMPLETED.to_string()).unwrap();
                    completed_at = Some(job_event.completed_at);
                    updated_at = job_event.completed_at;
                    version = job_event.version();
                }
            }
            "JobFailed" => {
                if let Some(job_event) = event.downcast_ref::<JobFailedEvent>() {
                    state = JobState::new(JobState::FAILED.to_string()).unwrap();
                    updated_at = job_event.failed_at;
                    retry_count = job_event.retry_count;
                    version = job_event.version();
                }
            }
            "JobRetried" => {
                if let Some(job_event) = event.downcast_ref::<JobRetriedEvent>() {
                    state = JobState::new(JobState::PENDING.to_string()).unwrap();
                    updated_at = job_event.retry_at;
                    retry_count = job_event.retry_count;
                    version = job_event.version();
                }
            }
            "JobCancelled" => {
                if let Some(job_event) = event.downcast_ref::<JobCancelledEvent>() {
                    state = JobState::new(JobState::CANCELLED.to_string()).unwrap();
                    updated_at = job_event.cancelled_at;
                    version = job_event.version();
                }
            }
            "JobMetadataUpdated" => {
                if let Some(job_event) = event.downcast_ref::<JobMetadataUpdatedEvent>() {
                    metadata = job_event.metadata.clone();
                    updated_at = job_event.updated_at;
                    version = job_event.version();
                }
            }
            _ => {
                // Unknown event type - skip
            }
        }
    }

    Ok(JobView {
        job_id,
        name,
        command,
        image,
        state,
        tenant_id,
        worker_id,
        retry_count,
        created_at,
        updated_at,
        started_at,
        completed_at,
        metadata,
    })
}

/// Projection error types
#[derive(thiserror::Error, Debug)]
pub enum ProjectionError {
    #[error("Event store error: {0}")]
    EventStoreError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::InMemoryEventStore;
    use crate::job_definitions::JobId;

    #[tokio::test]
    #[cfg(feature = "dashmap")]
    async fn test_job_projection_get_job() {
        let store = Arc::new(InMemoryEventStore::new());
        let projection = JobProjection::new(store);

        let job_id = JobId::new();
        let (job, event) = EventSourcedJob::create(
            job_id.clone(),
            Some("tenant-1".to_string()),
            "Test Job".to_string(),
            "echo hello".to_string(),
            "ubuntu:latest".to_string(),
        );

        // Save the event
        store
            .save_events(job_id.0, &[Box::new(event)], 0)
            .await
            .unwrap();

        // Get the job through projection
        let result = projection.get_job(job_id.clone()).await.unwrap();
        assert!(result.is_some());

        let view = result.unwrap();
        assert_eq!(view.name, "Test Job");
        assert_eq!(view.tenant_id, Some("tenant-1".to_string()));
    }

    #[test]
    fn test_build_job_view() {
        let job_id = JobId::new();
        let events: Vec<Box<dyn DomainEvent>> = vec![
            Box::new(JobCreatedEvent::new(
                job_id.0,
                "tenant-1".to_string(),
                "Test Job".to_string(),
                "echo hello".to_string(),
                "ubuntu:latest".to_string(),
            )),
            Box::new(JobStartedEvent::new(job_id.0, None)),
        ];

        let view = build_job_view(job_id, &events).unwrap();
        assert_eq!(view.name, "Test Job");
        assert_eq!(view.state.inner, JobState::RUNNING);
        assert!(view.started_at.is_some());
    }
}
