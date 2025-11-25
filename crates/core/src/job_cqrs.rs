//! Job Commands and Queries for CQRS
//!
//! This module defines all commands and queries for Job management.

use crate::Result;
use crate::cqrs::{Command, Query};
use crate::job_definitions::{JobId, JobState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Command: CreateJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobCommand {
    pub job_id: JobId,
    pub tenant_id: Option<String>,
    pub name: String,
    pub command: String,
    pub image: String,
    pub correlation_id: Option<Uuid>,
}

impl Command for CreateJobCommand {
    type AggregateId = JobId;

    fn aggregate_id(&self) -> Self::AggregateId {
        self.job_id.clone()
    }

    fn correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
}

/// Command: StartJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartJobCommand {
    pub job_id: JobId,
    pub worker_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
}

impl Command for StartJobCommand {
    type AggregateId = JobId;

    fn aggregate_id(&self) -> Self::AggregateId {
        self.job_id.clone()
    }

    fn correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
}

/// Command: CompleteJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteJobCommand {
    pub job_id: JobId,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub output: Option<String>,
    pub correlation_id: Option<Uuid>,
}

impl Command for CompleteJobCommand {
    type AggregateId = JobId;

    fn aggregate_id(&self) -> Self::AggregateId {
        self.job_id.clone()
    }

    fn correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
}

/// Command: FailJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailJobCommand {
    pub job_id: JobId,
    pub error_message: String,
    pub correlation_id: Option<Uuid>,
}

impl Command for FailJobCommand {
    type AggregateId = JobId;

    fn aggregate_id(&self) -> Self::AggregateId {
        self.job_id.clone()
    }

    fn correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
}

/// Command: RetryJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryJobCommand {
    pub job_id: JobId,
    pub reason: String,
    pub correlation_id: Option<Uuid>,
}

impl Command for RetryJobCommand {
    type AggregateId = JobId;

    fn aggregate_id(&self) -> Self::AggregateId {
        self.job_id.clone()
    }

    fn correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
}

/// Command: CancelJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelJobCommand {
    pub job_id: JobId,
    pub reason: String,
    pub correlation_id: Option<Uuid>,
}

impl Command for CancelJobCommand {
    type AggregateId = JobId;

    fn aggregate_id(&self) -> Self::AggregateId {
        self.job_id.clone()
    }

    fn correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
}

/// Command: UpdateJobMetadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateJobMetadataCommand {
    pub job_id: JobId,
    pub metadata: HashMap<String, String>,
    pub correlation_id: Option<Uuid>,
}

impl Command for UpdateJobMetadataCommand {
    type AggregateId = JobId;

    fn aggregate_id(&self) -> Self::AggregateId {
        self.job_id.clone()
    }

    fn correlation_id(&self) -> Option<Uuid> {
        self.correlation_id
    }
}

// Queries

/// Query: GetJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetJobQuery {
    pub job_id: JobId,
}

impl Query for GetJobQuery {
    type Result = Option<JobView>;
}

/// Query: ListJobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListJobsQuery {
    pub tenant_id: Option<String>,
    pub state: Option<JobState>,
    pub limit: usize,
}

impl Query for ListJobsQuery {
    type Result = Vec<JobView>;
}

/// Query: GetJobHistory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetJobHistoryQuery {
    pub job_id: JobId,
}

impl Query for GetJobHistoryQuery {
    type Result = Vec<JobEventView>;
}

/// Read model: JobView
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobView {
    pub job_id: JobId,
    pub name: String,
    pub command: String,
    pub image: String,
    pub state: JobState,
    pub tenant_id: Option<String>,
    pub worker_id: Option<Uuid>,
    pub retry_count: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: HashMap<String, String>,
}

/// Read model: JobEventView
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEventView {
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub version: u64,
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_job_command() {
        let cmd = CreateJobCommand {
            job_id: JobId::new(),
            tenant_id: Some("tenant-1".to_string()),
            name: "Test Job".to_string(),
            command: "echo hello".to_string(),
            image: "ubuntu:latest".to_string(),
            correlation_id: Some(Uuid::new_v4()),
        };

        assert_eq!(cmd.name, "Test Job");
        assert!(cmd.correlation_id.is_some());
    }

    #[test]
    fn test_start_job_command() {
        let cmd = StartJobCommand {
            job_id: JobId::new(),
            worker_id: Some(Uuid::new_v4()),
            correlation_id: Some(Uuid::new_v4()),
        };

        assert!(cmd.worker_id.is_some());
    }

    #[test]
    fn test_get_job_query() {
        let query = GetJobQuery {
            job_id: JobId::new(),
        };

        assert!(matches!(query, GetJobQuery { .. }));
    }

    #[test]
    fn test_job_view_creation() {
        let view = JobView {
            job_id: JobId::new(),
            name: "Test Job".to_string(),
            command: "echo hello".to_string(),
            image: "ubuntu:latest".to_string(),
            state: JobState::new(JobState::PENDING.to_string()).unwrap(),
            tenant_id: Some("tenant-1".to_string()),
            worker_id: None,
            retry_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            metadata: HashMap::new(),
        };

        assert_eq!(view.name, "Test Job");
        assert_eq!(view.metadata.len(), 0);
    }
}
