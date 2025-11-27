//! PipelineStep validation specifications using the Specification Pattern
//!
//! This module provides comprehensive validation for PipelineStep entities
//! using the Specification pattern for flexible and composable validation rules.

use crate::pipeline::PipelineStep;
use crate::specifications::Specification;
use std::fmt;

/// Specification to ensure step name is not empty
#[derive(Debug, Clone, Copy)]
pub struct StepNameNotEmptySpec;

impl StepNameNotEmptySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StepNameNotEmptySpec {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<PipelineStep> for StepNameNotEmptySpec {
    fn is_satisfied_by(&self, candidate: &PipelineStep) -> bool {
        !candidate.name.trim().is_empty()
    }
}

impl fmt::Display for StepNameNotEmptySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Step name must not be empty")
    }
}

/// Specification to ensure timeout is valid
#[derive(Debug, Clone, Copy)]
pub struct ValidTimeoutSpec;

impl ValidTimeoutSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ValidTimeoutSpec {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<PipelineStep> for ValidTimeoutSpec {
    fn is_satisfied_by(&self, candidate: &PipelineStep) -> bool {
        candidate.timeout_ms > 0
    }
}

impl fmt::Display for ValidTimeoutSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timeout must be greater than 0")
    }
}

/// Specification to ensure no self-dependencies
#[derive(Debug, Clone, Copy)]
pub struct NoSelfDependencySpec;

impl NoSelfDependencySpec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoSelfDependencySpec {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<PipelineStep> for NoSelfDependencySpec {
    fn is_satisfied_by(&self, candidate: &PipelineStep) -> bool {
        !candidate.depends_on.contains(&candidate.id)
    }
}

impl fmt::Display for NoSelfDependencySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Step cannot depend on itself")
    }
}

/// Specification to ensure dependency exists in a pipeline
#[derive(Debug)]
pub struct DependencyExistsSpec {
    pipeline_step_ids: Vec<String>,
}

impl DependencyExistsSpec {
    pub fn new(pipeline_step_ids: Vec<String>) -> Self {
        Self { pipeline_step_ids }
    }

    pub fn with_step(mut self, step_id: &str) -> Self {
        self.pipeline_step_ids.push(step_id.to_string());
        self
    }
}

impl Specification<PipelineStep> for DependencyExistsSpec {
    fn is_satisfied_by(&self, candidate: &PipelineStep) -> bool {
        candidate
            .depends_on
            .iter()
            .all(|dep| self.pipeline_step_ids.contains(&dep.as_uuid().to_string()))
    }
}

impl fmt::Display for DependencyExistsSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "All dependencies must exist in the pipeline")
    }
}

/// Specification to ensure no circular dependencies
#[derive(Debug)]
pub struct NoCircularDependencySpec {
    pipeline_graph: Vec<(String, Vec<String>)>,
}

impl NoCircularDependencySpec {
    pub fn new(pipeline_graph: Vec<(String, Vec<String>)>) -> Self {
        Self { pipeline_graph }
    }

    pub fn with_step(mut self, step_id: &str, dependencies: Vec<&str>) -> Self {
        self.pipeline_graph.push((
            step_id.to_string(),
            dependencies.iter().map(|s| s.to_string()).collect(),
        ));
        self
    }
}

impl Specification<PipelineStep> for NoCircularDependencySpec {
    fn is_satisfied_by(&self, candidate: &PipelineStep) -> bool {
        // Simple circular dependency check using DFS
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        fn has_cycle(
            node: &str,
            graph: &[(String, Vec<String>)],
            visited: &mut std::collections::HashSet<String>,
            rec_stack: &mut std::collections::HashSet<String>,
        ) -> bool {
            if rec_stack.contains(node) {
                return true;
            }
            if visited.contains(node) {
                return false;
            }

            visited.insert(node.to_string());
            rec_stack.insert(node.to_string());

            for (step_id, deps) in graph {
                if step_id == node {
                    for dep in deps {
                        if has_cycle(dep, graph, visited, rec_stack) {
                            return true;
                        }
                    }
                }
            }

            rec_stack.remove(node);
            false
        }

        !has_cycle(
            &candidate.id.as_uuid().to_string(),
            &self.pipeline_graph,
            &mut visited,
            &mut rec_stack,
        )
    }
}

impl fmt::Display for NoCircularDependencySpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pipeline must not have circular dependencies")
    }
}

/// Complete validation specification for PipelineStep
#[derive(Debug)]
pub struct PipelineStepValidationSpec {
    pub pipeline_step_ids: Vec<String>,
    pub pipeline_graph: Vec<(String, Vec<String>)>,
}

impl PipelineStepValidationSpec {
    pub fn new() -> Self {
        Self {
            pipeline_step_ids: vec![],
            pipeline_graph: vec![],
        }
    }

    pub fn with_pipeline(
        mut self,
        step_ids: Vec<String>,
        graph: Vec<(String, Vec<String>)>,
    ) -> Self {
        self.pipeline_step_ids = step_ids;
        self.pipeline_graph = graph;
        self
    }
}

impl Default for PipelineStepValidationSpec {
    fn default() -> Self {
        Self::new()
    }
}

impl Specification<PipelineStep> for PipelineStepValidationSpec {
    fn is_satisfied_by(&self, candidate: &PipelineStep) -> bool {
        let name_spec = StepNameNotEmptySpec::new();
        let timeout_spec = ValidTimeoutSpec::new();
        let no_self_spec = NoSelfDependencySpec::new();
        let dependency_spec = DependencyExistsSpec::new(self.pipeline_step_ids.clone());
        let no_cycle_spec = NoCircularDependencySpec::new(self.pipeline_graph.clone());

        name_spec.is_satisfied_by(candidate)
            && timeout_spec.is_satisfied_by(candidate)
            && no_self_spec.is_satisfied_by(candidate)
            && dependency_spec.is_satisfied_by(candidate)
            && no_cycle_spec.is_satisfied_by(candidate)
    }
}

impl fmt::Display for PipelineStepValidationSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PipelineStep must be valid")
    }
}

/// Build validation specification with detailed error reporting
pub fn build_step_validation_spec(
    pipeline_steps: &[PipelineStep],
) -> impl Specification<PipelineStep> + fmt::Display {
    let step_ids: Vec<String> = pipeline_steps
        .iter()
        .map(|s| s.id.as_uuid().to_string())
        .collect();

    let graph: Vec<(String, Vec<String>)> = pipeline_steps
        .iter()
        .map(|s| {
            (
                s.id.as_uuid().to_string(),
                s.depends_on
                    .iter()
                    .map(|d| d.as_uuid().to_string())
                    .collect(),
            )
        })
        .collect();

    PipelineStepValidationSpec {
        pipeline_step_ids: step_ids,
        pipeline_graph: graph,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_definitions::JobSpec;
    use crate::pipeline::{PipelineStep, PipelineStepId};
    use uuid::Uuid;

    fn create_test_step(name: &str, timeout_ms: u64) -> PipelineStep {
        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: crate::job_definitions::ResourceQuota::default(),
            timeout_ms,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: vec![],
        };
        PipelineStep::new(name.to_string(), job_spec, timeout_ms)
    }

    #[test]
    fn test_step_name_not_empty() {
        let spec = StepNameNotEmptySpec::new();

        let step = create_test_step("valid-step", 300000);
        assert!(spec.is_satisfied_by(&step));

        let empty_step = create_test_step("", 300000);
        assert!(!spec.is_satisfied_by(&empty_step));
    }

    #[test]
    fn test_valid_timeout() {
        let spec = ValidTimeoutSpec::new();

        let valid_step = create_test_step("step", 300000);
        assert!(spec.is_satisfied_by(&valid_step));

        let invalid_step = create_test_step("step", 0);
        assert!(!spec.is_satisfied_by(&invalid_step));
    }

    #[test]
    fn test_no_self_dependency() {
        let spec = NoSelfDependencySpec::new();

        let step_id = PipelineStepId::new();
        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: crate::job_definitions::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: vec![],
        };

        let mut step = PipelineStep {
            id: step_id.clone(),
            name: "test-step".to_string(),
            job_spec,
            depends_on: vec![],
            timeout_ms: 300000,
        };

        assert!(spec.is_satisfied_by(&step));

        step.depends_on.push(step_id);
        assert!(!spec.is_satisfied_by(&step));
    }

    #[test]
    fn test_dependency_exists() {
        let spec = DependencyExistsSpec::new(vec![
            "11111111-1111-1111-1111-111111111111".to_string(),
            "22222222-2222-2222-2222-222222222222".to_string(),
        ]);

        let step_id1 = PipelineStepId::from_uuid(
            Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        );
        let step_id2 = PipelineStepId::from_uuid(
            Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
        );

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: crate::job_definitions::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: vec![],
        };

        let step = PipelineStep {
            id: PipelineStepId::new(),
            name: "test-step".to_string(),
            job_spec: job_spec.clone(),
            depends_on: vec![step_id1.clone()],
            timeout_ms: 300000,
        };

        assert!(spec.is_satisfied_by(&step));

        let step_with_invalid_dep = PipelineStep {
            id: PipelineStepId::new(),
            name: "test-step".to_string(),
            job_spec: job_spec.clone(),
            depends_on: vec![step_id2.clone()],
            timeout_ms: 300000,
        };

        assert!(!spec.is_satisfied_by(&step_with_invalid_dep));
    }

    #[test]
    fn test_no_circular_dependency() {
        let spec = NoCircularDependencySpec::new(vec![
            (
                "11111111-1111-1111-1111-111111111111".to_string(),
                vec!["22222222-2222-2222-2222-222222222222".to_string()],
            ),
            ("22222222-2222-2222-2222-222222222222".to_string(), vec![]),
        ]);

        let step_id1 = PipelineStepId::from_uuid(
            Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        );
        let step_id2 = PipelineStepId::from_uuid(
            Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
        );

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: crate::job_definitions::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: vec![],
        };

        let step = PipelineStep {
            id: step_id1.clone(),
            name: "test-step".to_string(),
            job_spec,
            depends_on: vec![step_id2.clone()],
            timeout_ms: 300000,
        };

        assert!(spec.is_satisfied_by(&step));

        let circular_spec = NoCircularDependencySpec::new(vec![
            (
                "11111111-1111-1111-1111-111111111111".to_string(),
                vec!["22222222-2222-2222-2222-222222222222".to_string()],
            ),
            (
                "22222222-2222-2222-2222-222222222222".to_string(),
                vec!["11111111-1111-1111-1111-111111111111".to_string()],
            ),
        ]);

        assert!(!circular_spec.is_satisfied_by(&step));
    }

    #[test]
    fn test_complete_validation_spec() {
        let step_ids = vec![
            "11111111-1111-1111-1111-111111111111".to_string(),
            "22222222-2222-2222-2222-222222222222".to_string(),
        ];

        let graph = vec![
            ("11111111-1111-1111-1111-111111111111".to_string(), vec![]),
            (
                "22222222-2222-2222-2222-222222222222".to_string(),
                vec!["11111111-1111-1111-1111-111111111111".to_string()],
            ),
        ];

        let spec = PipelineStepValidationSpec::new().with_pipeline(step_ids, graph);

        let step_id = PipelineStepId::from_uuid(
            Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
        );
        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: crate::job_definitions::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: vec![],
        };

        let step = PipelineStep {
            id: step_id,
            name: "valid-step".to_string(),
            job_spec,
            depends_on: vec![],
            timeout_ms: 300000,
        };

        assert!(spec.is_satisfied_by(&step));
    }

    #[test]
    fn test_build_validation_spec() {
        let step1_id = PipelineStepId::new();
        let step2_id = PipelineStepId::new();

        let job_spec = JobSpec {
            name: "test-job".to_string(),
            image: "test:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            resources: crate::job_definitions::ResourceQuota::default(),
            timeout_ms: 300000,
            retries: 0,
            env: std::collections::HashMap::new(),
            secret_refs: vec![],
        };

        let steps = vec![
            PipelineStep {
                id: step1_id.clone(),
                name: "step1".to_string(),
                job_spec: job_spec.clone(),
                depends_on: vec![],
                timeout_ms: 300000,
            },
            PipelineStep {
                id: step2_id.clone(),
                name: "step2".to_string(),
                job_spec,
                depends_on: vec![step1_id.clone()],
                timeout_ms: 300000,
            },
        ];

        let spec = build_step_validation_spec(&steps);

        // Create a test step with ID matching one of the pipeline steps
        let test_step = PipelineStep {
            id: step1_id,
            name: "test-step".to_string(),
            job_spec: JobSpec {
                name: "test-job".to_string(),
                image: "test:latest".to_string(),
                command: vec!["echo".to_string(), "hello".to_string()],
                resources: crate::job_definitions::ResourceQuota::default(),
                timeout_ms: 300000,
                retries: 0,
                env: std::collections::HashMap::new(),
                secret_refs: vec![],
            },
            depends_on: vec![],
            timeout_ms: 300000,
        };

        assert!(spec.is_satisfied_by(&test_step));
    }
}
