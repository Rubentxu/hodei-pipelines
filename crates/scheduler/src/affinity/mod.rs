//! Affinity Rules and Taints/Tolerations Module
//!
//! This module implements Kubernetes-style affinity rules and taints/tolerations
//! for fine-grained control over job placement decisions.

use crate::SchedulerError;
use crate::types::*;
use std::collections::HashMap;

/// Affinity matcher for evaluating affinity rules
pub struct AffinityMatcher;

impl AffinityMatcher {
    /// Create new affinity matcher
    pub fn new() -> Self {
        Self
    }

    /// Check if node satisfies required (hard) affinity constraints
    pub fn check_required_affinity(&self, node: &WorkerNode, affinity: &NodeAffinity) -> bool {
        // Check required during scheduling constraints
        for selector in &affinity.required_during_scheduling {
            if !self.matches_label_selector(node, selector) {
                return false;
            }
        }

        true
    }

    /// Check if node satisfies preferred (soft) affinity constraints
    pub fn check_preferred_affinity(&self, node: &WorkerNode, affinity: &NodeAffinity) -> f64 {
        let mut total_weight = 0;
        let mut matched_weight = 0;

        for weighted_selector in &affinity.preferred_during_scheduling {
            total_weight += weighted_selector.weight.abs() as u32;

            if self.matches_label_selector(node, &weighted_selector.selector) {
                matched_weight += weighted_selector.weight.abs() as u32;
            }
        }

        if total_weight == 0 {
            0.0
        } else {
            matched_weight as f64 / total_weight as f64
        }
    }

    /// Check if node matches a label selector
    fn matches_label_selector(&self, node: &WorkerNode, selector: &LabelSelector) -> bool {
        let node_value = node.labels.get(&selector.key);

        match selector.operator {
            LabelSelectorOperator::In => {
                if let Some(values) = &selector.values {
                    if let Some(node_val) = node_value {
                        values.contains(node_val)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            LabelSelectorOperator::NotIn => {
                if let Some(values) = &selector.values {
                    if let Some(node_val) = node_value {
                        !values.contains(node_val)
                    } else {
                        true // Node doesn't have the key, so it's NOT in the values
                    }
                } else {
                    false
                }
            }
            LabelSelectorOperator::Exists => node_value.is_some(),
            LabelSelectorOperator::DoesNotExist => node_value.is_none(),
        }
    }

    /// Check if node selector is satisfied
    pub fn check_node_selector(&self, node: &WorkerNode, selector: &NodeSelector) -> bool {
        for (key, value) in &selector.labels {
            if node.labels.get(key) != Some(value) {
                return false;
            }
        }

        true
    }

    /// Check if node has sufficient tolerations for its taints
    pub fn check_taints_tolerations(&self, node: &WorkerNode, tolerations: &[Toleration]) -> bool {
        for taint in &node.taints {
            let mut has_matching_toleration = false;

            for toleration in tolerations {
                if self.matches_taint_toleration(taint, toleration) {
                    has_matching_toleration = true;
                    break;
                }
            }

            // If no matching toleration and effect is NoSchedule, reject
            if !has_matching_toleration && matches!(taint.effect, TaintEffect::NoSchedule) {
                return false;
            }
        }

        true
    }

    /// Check if a taint matches a toleration
    fn matches_taint_toleration(&self, taint: &Taint, toleration: &Toleration) -> bool {
        // Check key
        if taint.key != toleration.key {
            return false;
        }

        // Check effect
        if taint.effect != toleration.effect {
            return false;
        }

        // Check operator and value
        match (&taint.operator, &toleration.operator) {
            (TaintOperator::Equal, TaintOperator::Equal) => taint.value == toleration.value,
            (TaintOperator::NotEqual, TaintOperator::NotEqual) => taint.value != toleration.value,
            (TaintOperator::Exists, TaintOperator::Exists) => {
                // Just checking if the key exists
                true
            }
            _ => false,
        }
    }

    /// Evaluate all affinity rules for a job
    pub fn evaluate_affinity(
        &self,
        job: &Job,
        node: &WorkerNode,
    ) -> Result<AffinityResult, SchedulerError> {
        let mut can_schedule = true;
        let mut affinity_score = 0.0;

        // Check node selector
        if let Some(selector) = &job.spec.node_selector {
            if !self.check_node_selector(node, selector) {
                can_schedule = false;
            }
        }

        // Check node affinity
        if let Some(affinity) = &job.spec.affinity {
            if !self.check_required_affinity(node, affinity) {
                can_schedule = false;
            }

            // Calculate preferred affinity score
            affinity_score = self.check_preferred_affinity(node, affinity);
        }

        // Check taints and tolerations
        if !self.check_taints_tolerations(node, &job.spec.tolerations) {
            can_schedule = false;
        }

        Ok(AffinityResult {
            can_schedule,
            affinity_score,
        })
    }
}

/// Result of affinity evaluation
#[derive(Debug, Clone)]
pub struct AffinityResult {
    pub can_schedule: bool,
    pub affinity_score: f64, // 0.0 to 1.0, where 1.0 is perfect match
}

/// Helper to check if job can run on a specific node
pub fn can_schedule_on_node(job: &Job, node: &WorkerNode) -> Result<bool, SchedulerError> {
    let matcher = AffinityMatcher::new();
    let result = matcher.evaluate_affinity(job, node)?;
    Ok(result.can_schedule)
}

/// Helper to calculate affinity score for a node
pub fn calculate_affinity_score(job: &Job, node: &WorkerNode) -> Result<f64, SchedulerError> {
    let matcher = AffinityMatcher::new();
    let result = matcher.evaluate_affinity(job, node)?;
    Ok(result.affinity_score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::ComputeResource;
    use crate::types::KubernetesNodeSpecific;

    fn create_test_node(labels: HashMap<String, String>) -> WorkerNode {
        WorkerNode {
            id: uuid::Uuid::new_v4(),
            backend_type: BackendType::Kubernetes,
            status: WorkerStatus::Running,
            resources: ComputeResource {
                cpu_cores: 8.0,
                memory_bytes: 8_000_000_000,
                gpu_count: 0,
            },
            labels,
            taints: vec![],
            backend_specific: BackendSpecific::Kubernetes(KubernetesNodeSpecific {
                node_name: "test-node".to_string(),
                namespace: "default".to_string(),
            }),
            location: NodeLocation::default(),
        }
    }

    fn create_test_job_with_selector(selector_labels: HashMap<String, String>) -> Job {
        Job {
            metadata: JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            spec: JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(2.0),
                    memory_bytes: Some(2_000_000_000),
                    gpu_count: None,
                    ephemeral_storage: None,
                }),
                priority: JobPriority::Medium,
                node_selector: Some(NodeSelector {
                    labels: selector_labels,
                }),
                affinity: None,
                tolerations: vec![],
                max_retries: 3,
            },
        }
    }

    #[tokio::test]
    async fn test_node_selector_match() {
        let matcher = AffinityMatcher::new();

        let mut node_labels = HashMap::new();
        node_labels.insert("zone".to_string(), "us-east-1".to_string());
        node_labels.insert("arch".to_string(), "amd64".to_string());

        let node = create_test_node(node_labels);

        let mut job_selector = HashMap::new();
        job_selector.insert("zone".to_string(), "us-east-1".to_string());

        let job = create_test_job_with_selector(job_selector);

        let can_schedule = can_schedule_on_node(&job, &node).unwrap();

        assert!(can_schedule);
    }

    #[tokio::test]
    async fn test_node_selector_no_match() {
        let matcher = AffinityMatcher::new();

        let mut node_labels = HashMap::new();
        node_labels.insert("zone".to_string(), "us-west-2".to_string());

        let node = create_test_node(node_labels);

        let mut job_selector = HashMap::new();
        job_selector.insert("zone".to_string(), "us-east-1".to_string());

        let job = create_test_job_with_selector(job_selector);

        let can_schedule = can_schedule_on_node(&job, &node).unwrap();

        assert!(!can_schedule);
    }

    #[tokio::test]
    async fn test_required_affinity() {
        let matcher = AffinityMatcher::new();

        let mut node_labels = HashMap::new();
        node_labels.insert("gpu".to_string(), "true".to_string());
        node_labels.insert("zone".to_string(), "us-east-1".to_string());

        let node = create_test_node(node_labels);

        let required_affinity = NodeAffinity {
            required_during_scheduling: vec![LabelSelector {
                key: "gpu".to_string(),
                operator: LabelSelectorOperator::Exists,
                values: None,
            }],
            preferred_during_scheduling: vec![],
        };

        let mut job_selector = HashMap::new();
        let job = Job {
            metadata: JobMetadata {
                id: uuid::Uuid::new_v4(),
                name: "test-job".to_string(),
                namespace: "default".to_string(),
                labels: HashMap::new(),
                created_at: chrono::Utc::now(),
            },
            spec: JobSpec {
                resource_requirements: Some(ResourceRequirements {
                    cpu_cores: Some(2.0),
                    memory_bytes: Some(2_000_000_000),
                    gpu_count: None,
                    ephemeral_storage: None,
                }),
                priority: JobPriority::Medium,
                node_selector: Some(NodeSelector {
                    labels: job_selector,
                }),
                affinity: Some(required_affinity),
                tolerations: vec![],
                max_retries: 3,
            },
        };

        let can_schedule = can_schedule_on_node(&job, &node).unwrap();

        assert!(can_schedule);
    }

    #[tokio::test]
    async fn test_taints_tolerations() {
        let matcher = AffinityMatcher::new();

        let mut node = create_test_node(HashMap::new());
        node.taints = vec![Taint {
            key: "dedicated".to_string(),
            operator: TaintOperator::Equal,
            value: "gpu".to_string(),
            effect: TaintEffect::NoSchedule,
        }];

        let tolerations = vec![Toleration {
            key: "dedicated".to_string(),
            operator: TaintOperator::Equal,
            value: "gpu".to_string(),
            effect: TaintEffect::NoSchedule,
            toleration_seconds: None,
        }];

        let can_schedule = matcher.check_taints_tolerations(&node, &tolerations);

        assert!(can_schedule);
    }

    #[tokio::test]
    async fn test_no_toleration_for_noschedule_taint() {
        let matcher = AffinityMatcher::new();

        let mut node = create_test_node(HashMap::new());
        node.taints = vec![Taint {
            key: "dedicated".to_string(),
            operator: TaintOperator::Equal,
            value: "gpu".to_string(),
            effect: TaintEffect::NoSchedule,
        }];

        // No tolerations provided
        let tolerations = vec![];

        let can_schedule = matcher.check_taints_tolerations(&node, &tolerations);

        assert!(!can_schedule);
    }

    #[tokio::test]
    async fn test_affinity_score_calculation() {
        let matcher = AffinityMatcher::new();

        let mut node_labels = HashMap::new();
        node_labels.insert("zone".to_string(), "us-east-1".to_string());
        node_labels.insert("type".to_string(), "compute".to_string());

        let node = create_test_node(node_labels);

        let affinity = NodeAffinity {
            required_during_scheduling: vec![],
            preferred_during_scheduling: vec![
                WeightedLabelSelector {
                    selector: LabelSelector {
                        key: "zone".to_string(),
                        operator: LabelSelectorOperator::In,
                        values: Some(vec!["us-east-1".to_string()]),
                    },
                    weight: 50,
                },
                WeightedLabelSelector {
                    selector: LabelSelector {
                        key: "type".to_string(),
                        operator: LabelSelectorOperator::In,
                        values: Some(vec!["compute".to_string()]),
                    },
                    weight: 50,
                },
            ],
        };

        let score = matcher.check_preferred_affinity(&node, &affinity);

        assert_eq!(score, 1.0); // Perfect match
    }
}
