//! Job mapper for database persistence
//!
//! This module provides the mapping layer between domain objects and database rows,
//! reducing Feature Envy in the repository adapters.

use crate::{Job, JobId, JobSpec, JobState, ResourceQuota};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

/// Database row representation for Job entity
#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: JobId,
    pub name: String,
    pub description: Option<String>,
    pub spec_json: Value,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub tenant_id: Option<String>,
    pub result: Value,
}

/// Mapper trait for Job entity
pub trait JobMapper {
    /// Convert domain Job to database row
    fn to_row(&self, job: &Job) -> JobRow;

    /// Convert database row to domain Job
    fn from_row(&self, row: JobRow) -> Result<Job, String>;

    /// Generate UPDATE query and parameters for partial updates
    fn to_update_params(&self, job: &Job) -> (String, Vec<String>);
}

/// SQLx-based Job mapper implementation
pub struct SqlxJobMapper;

impl SqlxJobMapper {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqlxJobMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl JobMapper for SqlxJobMapper {
    fn to_row(&self, job: &Job) -> JobRow {
        JobRow {
            id: job.id.clone(),
            name: job.name.as_ref().clone(),
            description: job.description.as_ref().map(|cow| cow.as_ref().to_string()),
            spec_json: serde_json::to_value(job.spec.as_ref()).unwrap_or_else(|_| Value::Null),
            state: job.state.to_string(),
            created_at: job.created_at,
            updated_at: job.updated_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            tenant_id: job.tenant_id.as_ref().map(|arc| arc.as_ref().clone()),
            result: job.result.clone(),
        }
    }

    fn from_row(&self, row: JobRow) -> Result<Job, String> {
        let job_spec = serde_json::from_value::<JobSpec>(row.spec_json.clone())
            .map_err(|e| format!("Failed to parse job spec: {}", e))?;

        let job_state =
            JobState::new(row.state).map_err(|e| format!("Invalid job state: {}", e))?;

        Ok(Job {
            id: row.id,
            name: Arc::new(row.name),
            description: row.description.map(|s| Cow::Owned(s)),
            spec: Arc::new(job_spec),
            state: job_state,
            created_at: row.created_at,
            updated_at: row.updated_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            tenant_id: row.tenant_id.map(Arc::new),
            result: row.result,
        })
    }

    fn to_update_params(&self, job: &Job) -> (String, Vec<String>) {
        let mut updates = Vec::new();
        let mut params = Vec::new();

        updates.push(format!("name = ${}", params.len() + 1));
        params.push(job.name.as_ref().clone());

        updates.push(format!("description = ${}", params.len() + 1));
        params.push(
            job.description
                .as_ref()
                .map(|cow| cow.as_ref().to_string())
                .unwrap_or_default(),
        );

        updates.push(format!("spec = ${}", params.len() + 1));
        params.push(serde_json::to_string(job.spec.as_ref()).unwrap_or_else(|_| "{}".to_string()));

        updates.push(format!("state = ${}", params.len() + 1));
        params.push(job.state.to_string());

        updates.push(format!("updated_at = ${}", params.len() + 1));
        params.push(job.updated_at.to_rfc3339());

        if let Some(started) = job.started_at {
            updates.push(format!("started_at = ${}", params.len() + 1));
            params.push(started.to_rfc3339());
        }

        if let Some(completed) = job.completed_at {
            updates.push(format!("completed_at = ${}", params.len() + 1));
            params.push(completed.to_rfc3339());
        }

        if let Some(tenant) = &job.tenant_id {
            updates.push(format!("tenant_id = ${}", params.len() + 1));
            params.push(tenant.as_ref().clone());
        }

        let query = format!(
            "UPDATE jobs SET {} WHERE id = ${}",
            updates.join(", "),
            params.len() + 1
        );
        params.push(job.id.to_string());

        (query, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Job, JobId, JobState, ResourceQuota};

    fn create_test_job() -> Job {
        Job {
            id: JobId::new(),
            name: Arc::new("test-job".to_string()),
            description: Some(Cow::Owned("Test job description".to_string())),
            spec: Arc::new(JobSpec {
                name: "test-job".to_string(),
                image: "ubuntu:latest".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
                resources: ResourceQuota::default(),
                timeout_ms: 300000,
                retries: 3,
                env: HashMap::new(),
                secret_refs: Vec::new(),
            }),
            state: JobState::new("PENDING".to_string()).unwrap(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            tenant_id: Some(Arc::new("test-tenant".to_string())),
            result: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_to_row_from_row() {
        let mapper = SqlxJobMapper;
        let job = create_test_job();

        let row = mapper.to_row(&job);
        assert_eq!(row.name, *job.name);
        assert_eq!(row.state, job.state.to_string());

        let recovered_job = mapper.from_row(row).unwrap();
        assert_eq!(recovered_job.name, job.name);
        assert_eq!(recovered_job.spec.name, job.spec.name);
    }

    #[test]
    fn test_to_update_params() {
        let mapper = SqlxJobMapper;
        let job = create_test_job();

        let (query, params) = mapper.to_update_params(&job);

        assert!(query.contains("UPDATE jobs"));
        assert!(query.contains("WHERE id"));
        assert!(query.contains("name = $1"));
        assert!(!params.is_empty()); // Should have at least some parameters
    }
}
