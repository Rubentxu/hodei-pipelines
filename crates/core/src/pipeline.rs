//! Pipeline Domain Entity
//!
//! This module contains the Pipeline aggregate root and related value objects.

use crate::{DomainError, Result};
// use daggy::{Dag, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
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

/// Pipeline step identifier - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct PipelineStepId(pub Uuid);

impl PipelineStepId {
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

impl Default for PipelineStepId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PipelineStepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PipelineStepId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Pipeline step - Value Object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineStep {
    pub id: PipelineStepId,
    pub name: String,
    pub job_spec: crate::job::JobSpec,
    pub depends_on: Vec<PipelineStepId>,
    pub timeout_ms: u64,
}

impl PipelineStep {
    pub fn new(name: String, job_spec: crate::job::JobSpec) -> Self {
        Self {
            id: PipelineStepId::new(),
            name,
            job_spec,
            depends_on: vec![],
            timeout_ms: 300000, // 5 minutes default
        }
    }

    pub fn with_dependency(mut self, step_id: PipelineStepId) -> Self {
        self.depends_on.push(step_id);
        self
    }

    /// Validate step has no self-dependencies
    pub fn validate(&self) -> std::result::Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Step name cannot be empty".to_string(),
            ));
        }

        if self.depends_on.contains(&self.id) {
            return Err(DomainError::Validation(format!(
                "Step '{}' cannot depend on itself",
                self.name
            )));
        }

        if self.timeout_ms == 0 {
            return Err(DomainError::Validation(format!(
                "Step '{}' timeout must be greater than 0",
                self.name
            )));
        }

        Ok(())
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

/// Pipeline aggregate root with DAG validation
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
    /// Create a new pipeline with DAG validation
    pub fn new(
        id: PipelineId,
        name: String,
        steps: Vec<PipelineStep>,
    ) -> std::result::Result<Self, DomainError> {
        // Validate all steps first
        for step in &steps {
            step.validate()?;
        }

        // Validate that DAG is acyclic using manual cycle detection
        let cycle_check = has_cycle(&steps)?;
        if cycle_check {
            return Err(DomainError::Validation(
                "Circular dependency detected in pipeline".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        Ok(Self {
            id,
            name,
            description: None,
            steps,
            status: PipelineStatus::new(PipelineStatus::PENDING.to_string())?,
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
            tenant_id: None,
            workflow_definition: serde_json::Value::Null,
        })
    }

    /// Get execution order based on DAG topology
    pub fn get_execution_order(&self) -> std::result::Result<Vec<&PipelineStep>, DomainError> {
        // Get topological order using manual algorithm
        let topological_order = topological_sort(&self.steps)?;

        Ok(topological_order
            .into_iter()
            .map(|step_id| self.steps.iter().find(|step| step.id == step_id).unwrap())
            .collect())
    }

    pub fn add_step(&mut self, step: PipelineStep) {
        self.steps.push(step);
        self.updated_at = chrono::Utc::now();
    }

    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    pub fn start(&mut self) -> std::result::Result<(), DomainError> {
        let new_status = PipelineStatus::new(PipelineStatus::RUNNING.to_string())?;
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn complete(&mut self) -> std::result::Result<(), DomainError> {
        let new_status = PipelineStatus::new(PipelineStatus::SUCCESS.to_string())?;
        self.status = new_status;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn fail(&mut self) -> std::result::Result<(), DomainError> {
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

/// Check if the DAG has cycles using DFS
fn has_cycle(steps: &[PipelineStep]) -> std::result::Result<bool, DomainError> {
    let mut visited = HashMap::new();
    let mut rec_stack = HashMap::new();

    for step in steps {
        if !visited.contains_key(&step.id) {
            if has_cycle_dfs(&step.id, steps, &mut visited, &mut rec_stack)? {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn has_cycle_dfs(
    step_id: &PipelineStepId,
    steps: &[PipelineStep],
    visited: &mut HashMap<PipelineStepId, bool>,
    rec_stack: &mut HashMap<PipelineStepId, bool>,
) -> std::result::Result<bool, DomainError> {
    visited.insert(step_id.clone(), true);
    rec_stack.insert(step_id.clone(), true);

    let step = steps
        .iter()
        .find(|s| s.id == *step_id)
        .ok_or_else(|| DomainError::Validation(format!("Step not found: {}", step_id)))?;

    for dep_id in &step.depends_on {
        let is_visited = visited.get(dep_id).copied().unwrap_or(false);
        let in_recursion = rec_stack.get(dep_id).copied().unwrap_or(false);

        if !is_visited {
            if has_cycle_dfs(dep_id, steps, visited, rec_stack)? {
                return Ok(true);
            }
        } else if in_recursion {
            return Ok(true);
        }
    }

    rec_stack.insert(step_id.clone(), false);
    Ok(false)
}

/// Perform topological sort using Kahn's algorithm
fn topological_sort(
    steps: &[PipelineStep],
) -> std::result::Result<Vec<PipelineStepId>, DomainError> {
    // Build adjacency list
    let mut in_degree = HashMap::new();
    let mut graph = HashMap::new();

    // Initialize in-degrees
    for step in steps {
        in_degree.insert(step.id.clone(), 0);
        graph.insert(step.id.clone(), vec![]);
    }

    // Build graph and calculate in-degrees
    for step in steps {
        for dep_id in &step.depends_on {
            // Edge: dep -> step
            graph
                .entry(dep_id.clone())
                .and_modify(|edges| edges.push(step.id.clone()));
            in_degree.entry(step.id.clone()).and_modify(|deg| *deg += 1);
        }
    }

    // Find nodes with in-degree 0
    let mut queue: Vec<_> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(step_id, _)| step_id.clone())
        .collect();

    let mut result = Vec::new();

    // Process queue
    while let Some(step_id) = queue.pop() {
        result.push(step_id.clone());

        // For each node that this step points to
        if let Some(neighbors) = graph.get(&step_id) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(&neighbor) {
                    *deg -= 1;

                    if *deg == 0 {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }
    }

    // If we haven't visited all nodes, there's a cycle
    if result.len() != steps.len() {
        return Err(DomainError::Validation(
            "Circular dependency detected in pipeline".to_string(),
        ));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_pipeline_creation() {
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
        let pipeline =
            Pipeline::new(PipelineId::new(), "test-pipeline".to_string(), vec![step]).unwrap();

        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.steps.len(), 1);
        assert_eq!(pipeline.status.as_str(), PipelineStatus::PENDING);
    }

    #[test]
    fn test_pipeline_with_dependencies() {
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

        let step1 = PipelineStep::new("step1".to_string(), job_spec.clone());
        let step2 = PipelineStep::new("step2".to_string(), job_spec);

        // This should work - simple dependency
        let pipeline = Pipeline::new(
            PipelineId::new(),
            "test-pipeline".to_string(),
            vec![step1.clone(), step2.clone()],
        )
        .unwrap();

        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.steps.len(), 2);
    }

    #[test]
    fn test_circular_dependency_detection() {
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

        let mut step1 = PipelineStep::new("step1".to_string(), job_spec.clone());
        let mut step2 = PipelineStep::new("step2".to_string(), job_spec.clone());
        let mut step3 = PipelineStep::new("step3".to_string(), job_spec);

        // Create circular dependency: step1 -> step2 -> step3 -> step1
        step1.depends_on.push(step2.id.clone());
        step2.depends_on.push(step3.id.clone());
        step3.depends_on.push(step1.id.clone());

        let result = Pipeline::new(
            PipelineId::new(),
            "test-pipeline".to_string(),
            vec![step1, step2, step3],
        );

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Circular dependency"));
        }
    }

    #[test]
    fn test_self_dependency_detection() {
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

        let mut step = PipelineStep::new("step1".to_string(), job_spec);
        step.depends_on.push(step.id.clone());

        let result = step.validate();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("cannot depend on itself"));
        }
    }

    #[test]
    fn test_step_timeout_validation() {
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

        let mut step = PipelineStep::new("step1".to_string(), job_spec);
        step.timeout_ms = 0; // Set invalid timeout directly

        let result = step.validate();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("timeout must be greater than 0"));
        }
    }

    #[test]
    fn test_execution_order() {
        let job_spec = crate::job::JobSpec {
            name: "step".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let step1 = PipelineStep::new("step1".to_string(), job_spec.clone());
        let step2 = PipelineStep::new("step2".to_string(), job_spec.clone());

        // Make step2 depend on step1
        let mut step2_with_dep = step2.clone();
        step2_with_dep.depends_on.push(step1.id.clone());

        let pipeline = Pipeline::new(
            PipelineId::new(),
            "test-pipeline".to_string(),
            vec![step1.clone(), step2_with_dep],
        )
        .unwrap();

        let execution_order = pipeline.get_execution_order().unwrap();
        assert_eq!(execution_order.len(), 2);

        // step1 should come before step2 in the execution order
        assert_eq!(execution_order[0].name, "step1");
        assert_eq!(execution_order[1].name, "step2");
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
            Pipeline::new(PipelineId::new(), "test-pipeline".to_string(), vec![step]).unwrap();

        assert_eq!(pipeline.status.as_str(), PipelineStatus::PENDING);

        pipeline.start().unwrap();
        assert_eq!(pipeline.status.as_str(), PipelineStatus::RUNNING);

        pipeline.complete().unwrap();
        assert!(pipeline.is_terminal());
    }
}
