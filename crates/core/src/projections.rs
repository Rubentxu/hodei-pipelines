//! Event Projections and Read Models
//!
//! This module provides infrastructure for building read models from domain events.
//! Projections are used to create denormalized views optimized for queries.
//!
//! Key concepts:
//! - Projector: Consumes events and updates read models
//! - ReadModel: Denormalized representation of aggregate state
//! - JobStatusProjection: Read model for job status queries

use crate::events::{DomainEvent, EventStore};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Trait for event projectors
#[async_trait::async_trait]
pub trait Projector: Send + Sync {
    /// Process an event and update the read model
    async fn project(&mut self, event: &Box<dyn DomainEvent>) -> Result<(), ProjectionError>;

    /// Get the name of this projector
    fn name(&self) -> &'static str;
}

/// Trait for read models
pub trait ReadModel: Send + Sync {
    /// Get the aggregate ID this read model represents
    fn aggregate_id(&self) -> Uuid;

    /// Get the current version of the read model
    fn version(&self) -> u64;
}

/// Job Status Read Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusProjection {
    pub job_id: Uuid,
    pub job_name: String,
    pub current_state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time_ms: Option<u64>,
    pub retry_count: u32,
    pub tenant_id: Option<String>,
}

impl ReadModel for JobStatusProjection {
    fn aggregate_id(&self) -> Uuid {
        self.job_id
    }

    fn version(&self) -> u64 {
        // Calculate version based on timestamps (simplified)
        self.updated_at.timestamp() as u64
    }
}

impl JobStatusProjection {
    pub fn new(job_id: Uuid, job_name: String) -> Self {
        let now = Utc::now();
        Self {
            job_id,
            job_name,
            current_state: "PENDING".to_string(),
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            execution_time_ms: None,
            retry_count: 0,
            tenant_id: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.current_state == "RUNNING"
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.current_state.as_str(),
            "SUCCESS" | "FAILED" | "CANCELLED"
        )
    }

    pub fn execution_duration(&self) -> Option<std::time::Duration> {
        if let Some(started) = self.started_at
            && let Some(completed) = self.completed_at {
                return Some(
                    completed
                        .signed_duration_since(started)
                        .to_std()
                        .unwrap_or_default(),
                );
            }
        None
    }
}

/// Job Events for projection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCreatedEvent {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub version: u64,
    pub job_name: String,
    pub tenant_id: Option<String>,
}

impl DomainEvent for JobCreatedEvent {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn event_type(&self) -> &'static str {
        "JobCreated"
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn serialize(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn as_trait_object(&self) -> Box<dyn DomainEvent> {
        Box::new(Self {
            event_id: self.event_id,
            aggregate_id: self.aggregate_id,
            occurred_at: self.occurred_at,
            version: self.version,
            job_name: self.job_name.clone(),
            tenant_id: self.tenant_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobScheduledEvent {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub version: u64,
}

impl DomainEvent for JobScheduledEvent {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn event_type(&self) -> &'static str {
        "JobScheduled"
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn serialize(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn as_trait_object(&self) -> Box<dyn DomainEvent> {
        Box::new(Self {
            event_id: self.event_id,
            aggregate_id: self.aggregate_id,
            occurred_at: self.occurred_at,
            version: self.version,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStartedEvent {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub version: u64,
}

impl DomainEvent for JobStartedEvent {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn event_type(&self) -> &'static str {
        "JobStarted"
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn serialize(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn as_trait_object(&self) -> Box<dyn DomainEvent> {
        Box::new(Self {
            event_id: self.event_id,
            aggregate_id: self.aggregate_id,
            occurred_at: self.occurred_at,
            version: self.version,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCompletedEvent {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub version: u64,
    pub final_state: String,
    pub execution_time_ms: u64,
}

impl DomainEvent for JobCompletedEvent {
    fn event_id(&self) -> Uuid {
        self.event_id
    }

    fn event_type(&self) -> &'static str {
        "JobCompleted"
    }

    fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn serialize(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn as_trait_object(&self) -> Box<dyn DomainEvent> {
        Box::new(Self {
            event_id: self.event_id,
            aggregate_id: self.aggregate_id,
            occurred_at: self.occurred_at,
            version: self.version,
            final_state: self.final_state.clone(),
            execution_time_ms: self.execution_time_ms,
        })
    }
}

/// Job Status Projector
pub struct JobStatusProjector {
    projections: HashMap<Uuid, JobStatusProjection>,
}

impl JobStatusProjector {
    pub fn new() -> Self {
        Self {
            projections: HashMap::new(),
        }
    }

    /// Get a projection by job ID
    pub fn get(&self, job_id: &Uuid) -> Option<&JobStatusProjection> {
        self.projections.get(job_id)
    }

    /// Get all active jobs (non-terminal states)
    pub fn get_active_jobs(&self) -> Vec<&JobStatusProjection> {
        self.projections
            .values()
            .filter(|p| !p.is_terminal())
            .collect()
    }

    /// Get all completed jobs
    pub fn get_completed_jobs(&self) -> Vec<&JobStatusProjection> {
        self.projections
            .values()
            .filter(|p| p.is_terminal())
            .collect()
    }
}

impl Default for JobStatusProjector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Projector for JobStatusProjector {
    async fn project(&mut self, event: &Box<dyn DomainEvent>) -> Result<(), ProjectionError> {
        match event.event_type() {
            "JobCreated" => {
                // Use serialize() method from DomainEvent trait
                if let Ok(event_data) = event.serialize()
                    && let Ok(job_event) = serde_json::from_value::<JobCreatedEvent>(event_data) {
                        let projection =
                            JobStatusProjection::new(job_event.aggregate_id, job_event.job_name);
                        if let Some(_tenant) = job_event.tenant_id {
                            // Update projection with tenant
                        }
                        self.projections.insert(job_event.aggregate_id, projection);
                    }
            }
            "JobScheduled" => {
                if let Some(proj) = self.projections.get_mut(&event.aggregate_id()) {
                    proj.current_state = "SCHEDULED".to_string();
                    proj.updated_at = Utc::now();
                }
            }
            "JobStarted" => {
                if let Some(proj) = self.projections.get_mut(&event.aggregate_id()) {
                    proj.current_state = "RUNNING".to_string();
                    proj.started_at = Some(Utc::now());
                    proj.updated_at = Utc::now();
                }
            }
            "JobCompleted" => {
                if let Ok(event_data) = event.serialize()
                    && let Ok(completed_event) =
                        serde_json::from_value::<JobCompletedEvent>(event_data)
                        && let Some(proj) = self.projections.get_mut(&event.aggregate_id()) {
                            proj.current_state = completed_event.final_state;
                            proj.completed_at = Some(Utc::now());
                            proj.execution_time_ms = Some(completed_event.execution_time_ms);
                            proj.updated_at = Utc::now();
                        }
            }
            _ => {
                // Ignore unknown event types
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "JobStatusProjector"
    }
}

/// Projection error types
#[derive(thiserror::Error, Debug)]
pub enum ProjectionError {
    #[error("Event deserialization error: {0}")]
    DeserializationError(String),

    #[error("Projection update error: {0}")]
    UpdateError(String),
}

/// Build projections from an event store
pub async fn build_projections_from_events<T: EventStore + Send + Sync>(
    event_store: Arc<T>,
    _from_version: Option<u64>,
) -> Result<JobStatusProjector, ProjectionError> {
    

    let mut projector = JobStatusProjector::new();

    // Get all job-related events (in a real implementation, you'd filter by aggregate type)
    let all_events = event_store
        .load_events_by_type("JobCreated", None)
        .await
        .map_err(|e| ProjectionError::DeserializationError(e.to_string()))?;

    // In a real implementation, you'd also get JobScheduled, JobStarted, JobCompleted events
    // For simplicity, we're just handling JobCreated here

    for event in all_events {
        projector
            .project(&event)
            .await
            .map_err(|e| ProjectionError::UpdateError(format!("Failed to project event: {}", e)))?;
    }

    Ok(projector)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_job_status_projection_creation() {
        let job_id = Uuid::new_v4();
        let projection = JobStatusProjection::new(job_id, "test-job".to_string());

        assert_eq!(projection.job_id, job_id);
        assert_eq!(projection.job_name, "test-job");
        assert_eq!(projection.current_state, "PENDING");
        assert!(!projection.is_running());
        assert!(!projection.is_terminal());
        assert_eq!(projection.retry_count, 0);
    }

    #[test]
    fn test_job_status_projection_is_running() {
        let job_id = Uuid::new_v4();
        let mut projection = JobStatusProjection::new(job_id, "test-job".to_string());

        projection.current_state = "RUNNING".to_string();
        assert!(projection.is_running());
    }

    #[test]
    fn test_job_status_projection_is_terminal() {
        let job_id = Uuid::new_v4();

        for state in &["SUCCESS", "FAILED", "CANCELLED"] {
            let mut projection = JobStatusProjection::new(job_id, "test-job".to_string());
            projection.current_state = state.to_string();
            assert!(
                projection.is_terminal(),
                "State {} should be terminal",
                state
            );
        }

        // Non-terminal states should not be terminal
        let mut projection = JobStatusProjection::new(job_id, "test-job".to_string());
        projection.current_state = "RUNNING".to_string();
        assert!(!projection.is_terminal());
    }

    #[test]
    fn test_job_status_projection_execution_duration() {
        let job_id = Uuid::new_v4();
        let mut projection = JobStatusProjection::new(job_id, "test-job".to_string());

        // No execution time when not started
        assert!(projection.execution_duration().is_none());

        // Set started and completed times
        let started = Utc::now();
        let completed = started + chrono::Duration::seconds(10);
        projection.started_at = Some(started);
        projection.completed_at = Some(completed);

        let duration = projection.execution_duration().unwrap();
        assert_eq!(duration.as_secs(), 10);
    }

    #[tokio::test]
    async fn test_job_status_projector_projects_job_created() {
        let mut projector = JobStatusProjector::new();
        let job_id = Uuid::new_v4();

        let event = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "test-job".to_string(),
            tenant_id: Some("tenant-1".to_string()),
        };

        projector.project(&event.as_trait_object()).await.unwrap();

        let projection = projector.get(&job_id).unwrap();
        assert_eq!(projection.job_id, job_id);
        assert_eq!(projection.job_name, "test-job");
        assert_eq!(projection.current_state, "PENDING");
    }

    #[tokio::test]
    async fn test_job_status_projector_projects_job_scheduled() {
        let mut projector = JobStatusProjector::new();
        let job_id = Uuid::new_v4();

        // First create the job
        let created_event = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "test-job".to_string(),
            tenant_id: None,
        };
        projector
            .project(&created_event.as_trait_object())
            .await
            .unwrap();

        // Then schedule it
        let scheduled_event = JobScheduledEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job_id,
            occurred_at: Utc::now(),
            version: 1,
        };
        projector
            .project(&scheduled_event.as_trait_object())
            .await
            .unwrap();

        let projection = projector.get(&job_id).unwrap();
        assert_eq!(projection.current_state, "SCHEDULED");
    }

    #[tokio::test]
    async fn test_job_status_projector_projects_full_lifecycle() {
        let mut projector = JobStatusProjector::new();
        let job_id = Uuid::new_v4();

        // Create
        let created = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "test-job".to_string(),
            tenant_id: None,
        };
        projector.project(&created.as_trait_object()).await.unwrap();

        // Schedule
        let scheduled = JobScheduledEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job_id,
            occurred_at: Utc::now(),
            version: 1,
        };
        projector
            .project(&scheduled.as_trait_object())
            .await
            .unwrap();

        // Start
        let started = JobStartedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job_id,
            occurred_at: Utc::now(),
            version: 2,
        };
        projector.project(&started.as_trait_object()).await.unwrap();

        // Complete
        let completed = JobCompletedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job_id,
            occurred_at: Utc::now(),
            version: 3,
            final_state: "SUCCESS".to_string(),
            execution_time_ms: 5000,
        };
        projector
            .project(&completed.as_trait_object())
            .await
            .unwrap();

        let projection = projector.get(&job_id).unwrap();
        assert_eq!(projection.current_state, "SUCCESS");
        assert!(projection.is_terminal());
        assert_eq!(projection.execution_time_ms, Some(5000));
    }

    #[tokio::test]
    async fn test_job_status_projector_get_active_jobs() {
        let mut projector = JobStatusProjector::new();

        let job1_id = Uuid::new_v4();
        let job2_id = Uuid::new_v4();
        let job3_id = Uuid::new_v4();

        // Create job1 and complete it
        let job1_created = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job1_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "completed-job".to_string(),
            tenant_id: None,
        };
        projector
            .project(&job1_created.as_trait_object())
            .await
            .unwrap();

        let job1_completed = JobCompletedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job1_id,
            occurred_at: Utc::now(),
            version: 1,
            final_state: "SUCCESS".to_string(),
            execution_time_ms: 1000,
        };
        projector
            .project(&job1_completed.as_trait_object())
            .await
            .unwrap();

        // Create job2 and leave it running
        let job2_created = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job2_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "running-job".to_string(),
            tenant_id: None,
        };
        projector
            .project(&job2_created.as_trait_object())
            .await
            .unwrap();

        let job2_started = JobStartedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job2_id,
            occurred_at: Utc::now(),
            version: 1,
        };
        projector
            .project(&job2_started.as_trait_object())
            .await
            .unwrap();

        // Create job3 and leave it pending
        let job3_created = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job3_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "pending-job".to_string(),
            tenant_id: None,
        };
        projector
            .project(&job3_created.as_trait_object())
            .await
            .unwrap();

        let active_jobs = projector.get_active_jobs();
        assert_eq!(active_jobs.len(), 2);

        let active_names: Vec<&str> = active_jobs.iter().map(|p| p.job_name.as_str()).collect();
        assert!(active_names.contains(&"running-job"));
        assert!(active_names.contains(&"pending-job"));
        assert!(!active_names.contains(&"completed-job"));
    }

    #[tokio::test]
    async fn test_job_status_projector_get_completed_jobs() {
        let mut projector = JobStatusProjector::new();

        let job1_id = Uuid::new_v4();
        let job2_id = Uuid::new_v4();

        // Create and complete job1
        let job1_created = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job1_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "completed-job-1".to_string(),
            tenant_id: None,
        };
        projector
            .project(&job1_created.as_trait_object())
            .await
            .unwrap();

        let job1_completed = JobCompletedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job1_id,
            occurred_at: Utc::now(),
            version: 1,
            final_state: "SUCCESS".to_string(),
            execution_time_ms: 1000,
        };
        projector
            .project(&job1_completed.as_trait_object())
            .await
            .unwrap();

        // Create and complete job2 with FAILED
        let job2_created = JobCreatedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job2_id,
            occurred_at: Utc::now(),
            version: 0,
            job_name: "completed-job-2".to_string(),
            tenant_id: None,
        };
        projector
            .project(&job2_created.as_trait_object())
            .await
            .unwrap();

        let job2_completed = JobCompletedEvent {
            event_id: Uuid::new_v4(),
            aggregate_id: job2_id,
            occurred_at: Utc::now(),
            version: 1,
            final_state: "FAILED".to_string(),
            execution_time_ms: 2000,
        };
        projector
            .project(&job2_completed.as_trait_object())
            .await
            .unwrap();

        let completed_jobs = projector.get_completed_jobs();
        assert_eq!(completed_jobs.len(), 2);

        let completed_names: Vec<&str> =
            completed_jobs.iter().map(|p| p.job_name.as_str()).collect();
        assert!(completed_names.contains(&"completed-job-1"));
        assert!(completed_names.contains(&"completed-job-2"));
    }
}
