//! Pipeline Domain Entity
//!
//! This module contains the Pipeline aggregate root and related value objects.

use crate::{DomainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Pipeline identifier - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct PipelineId(pub Uuid);

impl PipelineId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for PipelineId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pipeline step - Value Object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineStep {
    pub name: String,
    pub job_spec: crate::job::JobSpec,
    pub depends_on: Vec<String>,
    pub timeout_ms: u64,
}

impl PipelineStep {
    pub fn new(name: String, job_spec: crate::job::JobSpec) -> Self {
        Self {
            name,
            job_spec,
            depends_on: vec![],
            timeout_ms: 300000, // 5 minutes default
        }
    }

    pub fn with_dependency(mut self, step_name: String) -> Self {
        self.depends_on.push(step_name);
        self
    }
}

/// Pipeline status - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineStatus(String);

impl PipelineStatus {
    pub const PENDING: &'static str = "PENDING";
    pub const RUNNING: &'static str = "RUNNING";
    pub const SUCCESS: &'static str = "SUCCESS";
    pub const FAILED: &'static str = "FAILED";
    pub const CANCELLED: &'static str = "CANCELLED";

    pub fn new(status: String) -> Result<Self> {
        match status.as_str() {
            Self::PENDING | Self::RUNNING | Self::SUCCESS | Self::FAILED | Self::CANCELLED => {
                Ok(Self(status))
            }
            _ => Err(DomainError::Validation(format!(
                "invalid pipeline status: {}",
                status
            ))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.0.as_str(),
            Self::SUCCESS | Self::FAILED | Self::CANCELLED
        )
    }
}

impl From<String> for PipelineStatus {
    fn from(s: String) -> Self {
        Self::new(s).expect("valid status")
    }
}

/// Pipeline aggregate root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: PipelineId,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<PipelineStep>,
    pub status: PipelineStatus,
    pub variables: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tenant_id: Option<String>,
    pub workflow_definition: serde_json::Value,
}

impl Pipeline {
    pub fn new(id: PipelineId, name: String, steps: Vec<PipelineStep>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            steps,
            status: PipelineStatus::new(PipelineStatus::PENDING.to_string()).unwrap(),
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
            tenant_id: None,
            workflow_definition: serde_json::Value::Null,
        }
    }

    pub fn add_step(&mut self, step: PipelineStep) {
        self.steps.push(step);
        self.updated_at = chrono::Utc::now();
    }

    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    pub fn start(&mut self) -> Result<()> {
        let new_status = PipelineStatus::new(PipelineStatus::RUNNING.to_string())?;
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn complete(&mut self) -> Result<()> {
        let new_status = PipelineStatus::new(PipelineStatus::SUCCESS.to_string())?;
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn fail(&mut self) -> Result<()> {
        let new_status = PipelineStatus::new(PipelineStatus::FAILED.to_string())?;
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.status.as_str() == PipelineStatus::RUNNING
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            PipelineStatus::SUCCESS | PipelineStatus::FAILED | PipelineStatus::CANCELLED
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        let job_spec = crate::job::JobSpec {
            name: "step1".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let step = PipelineStep::new("step1".to_string(), job_spec);
        let pipeline = Pipeline::new(PipelineId::new(), "test-pipeline".to_string(), vec![step]);

        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.steps.len(), 1);
        assert_eq!(pipeline.status.as_str(), PipelineStatus::PENDING);
    }

    #[test]
    fn test_pipeline_status_transition() {
        let job_spec = crate::job::JobSpec {
            name: "step1".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let step = PipelineStep::new("step1".to_string(), job_spec);
        let mut pipeline =
            Pipeline::new(PipelineId::new(), "test-pipeline".to_string(), vec![step]);

        assert_eq!(pipeline.status.as_str(), PipelineStatus::PENDING);

        pipeline.start().unwrap();
        assert_eq!(pipeline.status.as_str(), PipelineStatus::RUNNING);

        pipeline.complete().unwrap();
        assert!(pipeline.is_terminal());
    }
}
