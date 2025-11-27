//! Pipeline Domain Entity
//!
//! This module contains the Pipeline aggregate root and related value objects.

use crate::specifications::Specification;
use crate::{DomainError, Result};
// use daggy::{Dag, NodeIndex};
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
    pub fn new(name: String, job_spec: crate::job::JobSpec, timeout_ms: u64) -> Self {
        Self {
            id: PipelineStepId::new(),
            name,
            job_spec,
            depends_on: vec![],
            timeout_ms,
        }
    }

    pub fn with_dependency(mut self, step_id: PipelineStepId) -> Self {
        self.depends_on.push(step_id);
        self
    }

    /// Validate step using individual validation rules
    /// (Pipeline-level validation happens when creating a Pipeline)
    pub fn validate(&self) -> std::result::Result<(), DomainError> {
        use crate::pipeline_step_specifications::{
            NoSelfDependencySpec, StepNameNotEmptySpec, ValidTimeoutSpec,
        };

        // Individual step validation (name, timeout, no self-dependency)
        let name_spec = StepNameNotEmptySpec::new();
        let timeout_spec = ValidTimeoutSpec::new();
        let no_self_spec = NoSelfDependencySpec::new();

        if !name_spec.is_satisfied_by(self) {
            return Err(DomainError::Validation(
                "Step name cannot be empty".to_string(),
            ));
        }

        if !timeout_spec.is_satisfied_by(self) {
            return Err(DomainError::Validation(format!(
                "Step '{}' timeout must be greater than 0",
                self.name
            )));
        }

        if !no_self_spec.is_satisfied_by(self) {
            return Err(DomainError::Validation(format!(
                "Step '{}' cannot depend on itself",
                self.name
            )));
        }

        Ok(())
    }
}

/// Builder for PipelineStep using Builder Pattern
/// Provides fluent interface for step construction with validation
pub struct PipelineStepBuilder {
    name: Option<String>,
    job_spec: Option<crate::job::JobSpec>,
    timeout_ms: Option<u64>,
    depends_on: Vec<PipelineStepId>,
}

impl PipelineStepBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            name: None,
            job_spec: None,
            timeout_ms: Some(300000), // Default 5 minutes
            depends_on: vec![],
        }
    }

    /// Set step name (required)
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set job spec (required)
    pub fn job_spec(mut self, job_spec: crate::job::JobSpec) -> Self {
        self.job_spec = Some(job_spec);
        self
    }

    /// Set timeout in milliseconds (optional, default: 300000)
    pub fn timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Add a dependency (can be called multiple times)
    pub fn depends_on(mut self, step_id: PipelineStepId) -> Self {
        self.depends_on.push(step_id);
        self
    }

    /// Add multiple dependencies
    pub fn depends_on_many(mut self, step_ids: Vec<PipelineStepId>) -> Self {
        self.depends_on.extend(step_ids);
        self
    }

    /// Build the PipelineStep with validation
    /// Returns error if required fields are missing or validation fails
    pub fn build(self) -> Result<PipelineStep> {
        let name = self
            .name
            .ok_or_else(|| DomainError::Validation("Step name is required".to_string()))?;

        let job_spec = self
            .job_spec
            .ok_or_else(|| DomainError::Validation("Job spec is required".to_string()))?;

        let timeout_ms = self
            .timeout_ms
            .ok_or_else(|| DomainError::Validation("Timeout is required".to_string()))?;

        let step = PipelineStep {
            id: PipelineStepId::new(),
            name,
            job_spec,
            depends_on: self.depends_on,
            timeout_ms,
        };

        // Validate using specifications
        step.validate()?;

        Ok(step)
    }
}

impl Default for PipelineStepBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline status - Value Object (Enum)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStatus {
    PENDING,
    RUNNING,
    SUCCESS,
    FAILED,
    CANCELLED,
}

impl PipelineStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PENDING => "PENDING",
            Self::RUNNING => "RUNNING",
            Self::SUCCESS => "SUCCESS",
            Self::FAILED => "FAILED",
            Self::CANCELLED => "CANCELLED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::SUCCESS | Self::FAILED | Self::CANCELLED)
    }

    /// Create from string (for backward compatibility)
    pub fn from_str(status: &str) -> Result<Self> {
        match status {
            "PENDING" => Ok(Self::PENDING),
            "RUNNING" => Ok(Self::RUNNING),
            "SUCCESS" => Ok(Self::SUCCESS),
            "FAILED" => Ok(Self::FAILED),
            "CANCELLED" => Ok(Self::CANCELLED),
            _ => Err(DomainError::Validation(format!(
                "invalid pipeline status: {}",
                status
            ))),
        }
    }
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for PipelineStatus {
    fn from(s: String) -> Self {
        Self::from_str(&s).expect("valid status")
    }
}

impl From<&str> for PipelineStatus {
    fn from(s: &str) -> Self {
        Self::from_str(s).expect("valid status")
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
            status: PipelineStatus::PENDING,
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
        self.status = PipelineStatus::RUNNING;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn complete(&mut self) -> std::result::Result<(), DomainError> {
        self.status = PipelineStatus::SUCCESS;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn fail(&mut self) -> std::result::Result<(), DomainError> {
        self.status = PipelineStatus::FAILED;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, PipelineStatus::RUNNING)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            PipelineStatus::SUCCESS | PipelineStatus::FAILED | PipelineStatus::CANCELLED
        )
    }
}

/// Check if the DAG has cycles using DFS
/// Optimized to O(n) by pre-building lookup indices
fn has_cycle(steps: &[PipelineStep]) -> std::result::Result<bool, DomainError> {
    // Pre-build lookup table: step_id -> step_index (O(n))
    let step_lookup: HashMap<PipelineStepId, usize> = steps
        .iter()
        .enumerate()
        .map(|(idx, step)| (step.id.clone(), idx))
        .collect();

    let mut visited = HashMap::new();
    let mut rec_stack = HashMap::new();

    // For each step, check if it's part of a cycle (O(n))
    for step_id in step_lookup.keys() {
        if !visited.contains_key(step_id) {
            if has_cycle_dfs(step_id, steps, &step_lookup, &mut visited, &mut rec_stack)? {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

fn has_cycle_dfs(
    step_id: &PipelineStepId,
    steps: &[PipelineStep],
    step_lookup: &HashMap<PipelineStepId, usize>,
    visited: &mut HashMap<PipelineStepId, bool>,
    rec_stack: &mut HashMap<PipelineStepId, bool>,
) -> std::result::Result<bool, DomainError> {
    visited.insert(step_id.clone(), true);
    rec_stack.insert(step_id.clone(), true);

    // O(1) lookup instead of O(n) iteration
    let step_idx = step_lookup
        .get(step_id)
        .ok_or_else(|| DomainError::Validation(format!("Step not found: {}", step_id)))?;

    let step = &steps[*step_idx];

    for dep_id in &step.depends_on {
        let is_visited = visited.get(dep_id).copied().unwrap_or(false);
        let in_recursion = rec_stack.get(dep_id).copied().unwrap_or(false);

        if !is_visited {
            if has_cycle_dfs(dep_id, steps, step_lookup, visited, rec_stack)? {
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
/// Optimized to O(n) by pre-building lookup indices
fn topological_sort(
    steps: &[PipelineStep],
) -> std::result::Result<Vec<PipelineStepId>, DomainError> {
    // Pre-build lookup table for O(1) index access
    let step_lookup: HashMap<PipelineStepId, usize> = steps
        .iter()
        .enumerate()
        .map(|(idx, step)| (step.id.clone(), idx))
        .collect();

    // Build adjacency list and in-degrees in O(n) time
    let mut in_degree = vec![0u32; steps.len()];
    let mut graph = vec![Vec::new(); steps.len()];

    // Process dependencies: for each edge dep -> step
    for (step_idx, step) in steps.iter().enumerate() {
        for dep_id in &step.depends_on {
            // O(1) lookup instead of O(n)
            let dep_idx = *step_lookup
                .get(dep_id)
                .ok_or_else(|| DomainError::Validation(format!("Step not found: {}", dep_id)))?;

            // Edge: dep -> step
            graph[dep_idx].push(step_idx);
            in_degree[step_idx] += 1;
        }
    }

    // Find nodes with in-degree 0
    let mut queue: Vec<usize> = in_degree
        .iter()
        .enumerate()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(idx, _)| idx)
        .collect();

    let mut result = Vec::new();

    // Process queue
    while let Some(step_idx) = queue.pop() {
        // Convert index back to step_id
        let step_id = &steps[step_idx].id;
        result.push(step_id.clone());

        // For each node that this step points to
        for &neighbor in &graph[step_idx] {
            in_degree[neighbor] -= 1;

            if in_degree[neighbor] == 0 {
                queue.push(neighbor);
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

        let step = PipelineStep::new("step1".to_string(), job_spec, 300000);
        let pipeline =
            Pipeline::new(PipelineId::new(), "test-pipeline".to_string(), vec![step]).unwrap();

        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.steps.len(), 1);
        assert!(matches!(pipeline.status, PipelineStatus::PENDING));
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

        let step1 = PipelineStep::new("step1".to_string(), job_spec.clone(), 300000);
        let step2 = PipelineStep::new("step2".to_string(), job_spec, 300000);

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

        let mut step1 = PipelineStep::new("step1".to_string(), job_spec.clone(), 300000);
        let mut step2 = PipelineStep::new("step2".to_string(), job_spec.clone(), 300000);
        let mut step3 = PipelineStep::new("step3".to_string(), job_spec, 300000);

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

        let mut step = PipelineStep::new("step1".to_string(), job_spec, 300000);
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

        let mut step = PipelineStep::new("step1".to_string(), job_spec, 300000);
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

        let step1 = PipelineStep::new("step1".to_string(), job_spec.clone(), 300000);
        let step2 = PipelineStep::new("step2".to_string(), job_spec.clone(), 300000);

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

        let step = PipelineStep::new("step1".to_string(), job_spec, 300000);
        let mut pipeline =
            Pipeline::new(PipelineId::new(), "test-pipeline".to_string(), vec![step]).unwrap();

        assert!(matches!(pipeline.status, PipelineStatus::PENDING));

        pipeline.start().unwrap();
        assert!(matches!(pipeline.status, PipelineStatus::RUNNING));

        pipeline.complete().unwrap();
        assert!(pipeline.is_terminal());
    }

    #[test]
    fn test_pipeline_step_builder_basic() {
        let job_spec = crate::job::JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let step = PipelineStepBuilder::new()
            .name("test-step")
            .job_spec(job_spec.clone())
            .build()
            .unwrap();

        assert_eq!(step.name, "test-step");
        assert_eq!(step.job_spec.name, "test-job");
        assert_eq!(step.timeout_ms, 300000); // default
        assert!(step.depends_on.is_empty());
    }

    #[test]
    fn test_pipeline_step_builder_with_custom_timeout() {
        let job_spec = crate::job::JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let step = PipelineStepBuilder::new()
            .name("test-step")
            .job_spec(job_spec.clone())
            .timeout(600000)
            .build()
            .unwrap();

        assert_eq!(step.timeout_ms, 600000);
    }

    #[test]
    fn test_pipeline_step_builder_with_dependencies() {
        let job_spec = crate::job::JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let step1_id = PipelineStepId::new();

        // Create a pipeline with both steps
        let step1 = PipelineStepBuilder::new()
            .name("dependency-step")
            .job_spec(job_spec.clone())
            .build()
            .unwrap();

        let step2 = PipelineStepBuilder::new()
            .name("dependent-step")
            .job_spec(job_spec.clone())
            .depends_on(step1.id.clone())
            .build()
            .unwrap();

        assert_eq!(step2.depends_on.len(), 1);
        assert_eq!(step2.depends_on[0], step1.id);
    }

    #[test]
    fn test_pipeline_step_builder_with_multiple_dependencies() {
        let job_spec = crate::job::JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let step1 = PipelineStepBuilder::new()
            .name("dep1")
            .job_spec(job_spec.clone())
            .build()
            .unwrap();

        let step2 = PipelineStepBuilder::new()
            .name("dep2")
            .job_spec(job_spec.clone())
            .build()
            .unwrap();

        let step3 = PipelineStepBuilder::new()
            .name("main-step")
            .job_spec(job_spec.clone())
            .depends_on(step1.id.clone())
            .depends_on(step2.id.clone())
            .build()
            .unwrap();

        assert_eq!(step3.depends_on.len(), 2);
    }

    #[test]
    fn test_pipeline_step_builder_fails_without_name() {
        let job_spec = crate::job::JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        let result = PipelineStepBuilder::new()
            .job_spec(job_spec.clone())
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_step_builder_fails_without_job_spec() {
        let result = PipelineStepBuilder::new().name("test-step").build();

        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_step_builder_fluent_interface() {
        let job_spec = crate::job::JobSpec {
            name: "test-job".to_string(),
            image: "ubuntu".to_string(),
            command: vec!["echo".to_string()],
            resources: crate::job::ResourceQuota::default(),
            timeout_ms: 30000,
            retries: 3,
            env: HashMap::new(),
            secret_refs: vec![],
        };

        // Test that builder methods return self for chaining
        let mut builder = PipelineStepBuilder::new();
        builder = builder.name("test-step");
        builder = builder.job_spec(job_spec.clone());
        builder = builder.timeout(600000);

        let step = builder.build().unwrap();
        assert_eq!(step.name, "test-step");
        assert_eq!(step.timeout_ms, 600000);
    }
}
