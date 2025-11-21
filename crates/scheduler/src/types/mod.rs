//! Core Types and Data Structures for Scheduler
//!
//! This module defines all the core data types used by the scheduler framework,
//! including jobs, workers, priorities, affinity rules, taints, etc.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Job priority levels (higher value = higher priority)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum JobPriority {
    Batch = 1,    // Low-priority batch processing
    Low = 2,      // Background jobs
    Medium = 3,   // Regular CI/CD jobs
    High = 4,     // Production deployments
    Critical = 5, // System critical jobs
}

impl JobPriority {
    /// Check if this priority can preempt another
    pub fn can_preempt(&self, other: &JobPriority) -> bool {
        self > other
    }

    /// Get numeric value
    pub fn value(&self) -> u32 {
        match self {
            JobPriority::Critical => 5,
            JobPriority::High => 4,
            JobPriority::Medium => 3,
            JobPriority::Low => 2,
            JobPriority::Batch => 1,
        }
    }
}

impl std::fmt::Display for JobPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobPriority::Critical => write!(f, "Critical"),
            JobPriority::High => write!(f, "High"),
            JobPriority::Medium => write!(f, "Medium"),
            JobPriority::Low => write!(f, "Low"),
            JobPriority::Batch => write!(f, "Batch"),
        }
    }
}

/// Backend types supported by the scheduler
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    Kubernetes,
    Docker,
    CloudVm,
    BareMetal,
    Serverless,
    Hpc,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Kubernetes => write!(f, "Kubernetes"),
            BackendType::Docker => write!(f, "Docker"),
            BackendType::CloudVm => write!(f, "Cloud VM"),
            BackendType::BareMetal => write!(f, "Bare Metal"),
            BackendType::Serverless => write!(f, "Serverless"),
            BackendType::Hpc => write!(f, "HPC"),
        }
    }
}

/// Worker node status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Pending,  // Node is starting up
    Ready,    // Node is ready to accept jobs
    Running,  // Node is running jobs
    Draining, // Node is being drained (no new jobs)
    Offline,  // Node is offline
    Failed,   // Node has failed
}

impl WorkerStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, WorkerStatus::Ready | WorkerStatus::Running)
    }
}

/// Job definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub metadata: JobMetadata,
    pub spec: JobSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    pub id: super::JobId,
    pub name: String,
    pub namespace: String,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub resource_requirements: Option<ResourceRequirements>,
    pub priority: JobPriority,
    pub node_selector: Option<NodeSelector>,
    pub affinity: Option<NodeAffinity>,
    pub tolerations: Vec<Toleration>,
    pub max_retries: u32,
}

/// Resource requirements for a job
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub gpu_count: Option<u32>,
    pub ephemeral_storage: Option<u64>,
}

impl ResourceRequirements {
    /// Check if requirements are valid
    pub fn is_valid(&self) -> bool {
        if let Some(cpu) = self.cpu_cores {
            if cpu <= 0.0 {
                return false;
            }
        }

        if let Some(memory) = self.memory_bytes {
            if memory == 0 {
                return false;
            }
        }

        if let Some(gpu) = self.gpu_count {
            if gpu == 0 {
                return false;
            }
        }

        true
    }
}

/// Node selector for job placement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSelector {
    pub labels: HashMap<String, String>,
}

/// Node affinity rules
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeAffinity {
    pub required_during_scheduling: Vec<LabelSelector>,
    pub preferred_during_scheduling: Vec<WeightedLabelSelector>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelSelector {
    pub key: String,
    pub operator: LabelSelectorOperator,
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedLabelSelector {
    pub selector: LabelSelector,
    pub weight: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelSelectorOperator {
    In,
    NotIn,
    Exists,
    DoesNotExist,
}

/// Taints for node dedication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taint {
    pub key: String,
    pub operator: TaintOperator,
    pub value: String,
    pub effect: TaintEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaintOperator {
    Equal,
    NotEqual,
    Exists,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaintEffect {
    NoSchedule,
    PreferNoSchedule,
    NoExecute,
}

/// Tolerations to override taints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toleration {
    pub key: String,
    pub operator: TaintOperator,
    pub value: String,
    pub effect: TaintEffect,
    pub toleration_seconds: Option<i64>,
}

/// Backend-specific information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendSpecific {
    Kubernetes(KubernetesNodeSpecific),
    Docker(DockerNodeSpecific),
    CloudVm(CloudVmNodeSpecific),
    BareMetal(BareMetalNodeSpecific),
    Serverless(ServerlessNodeSpecific),
    Hpc(HpcNodeSpecific),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesNodeSpecific {
    pub node_name: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerNodeSpecific {
    pub host: String,
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudVmNodeSpecific {
    pub instance_id: String,
    pub region: String,
    pub zone: String,
    pub instance_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BareMetalNodeSpecific {
    pub hostname: String,
    pub ip_address: String,
    pub rack_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessNodeSpecific {
    pub provider: String,
    pub runtime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpcNodeSpecific {
    pub cluster_name: String,
    pub partition: String,
}

/// Node location information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeLocation {
    pub region: Option<String>,
    pub zone: Option<String>,
    pub datacenter: Option<String>,
    pub rack: Option<String>,
}

/// Scoring weights for worker selection
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub resource_fit: f64,
    pub location: f64,
    pub backend: f64,
    pub affinity: f64,
    pub taints: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            resource_fit: 0.40, // 40%
            location: 0.25,     // 25%
            backend: 0.15,      // 15%
            affinity: 0.15,     // 15%
            taints: 0.05,       // 5%
        }
    }
}

impl ScoringWeights {
    pub fn new(resource_fit: f64, location: f64, backend: f64, affinity: f64, taints: f64) -> Self {
        Self {
            resource_fit,
            location,
            backend,
            affinity,
            taints,
        }
    }
}

/// Scored worker with score
#[derive(Debug, Clone)]
pub struct ScoredWorker {
    pub node: WorkerNode,
    pub score: f64,
    pub reasons: Vec<String>,
}

impl ScoredWorker {
    pub fn new(node: WorkerNode, score: f64) -> Self {
        Self {
            node,
            score,
            reasons: vec![],
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reasons.push(reason);
        self
    }
}

/// Worker selection strategies
#[derive(Debug, Clone)]
pub enum WorkerSelectionStrategy {
    LeastLoaded,
    ResourceBalance,
    BinPacking,
    RoundRobin,
    Custom(String), // Plugin-based strategy
}

/// Worker node representation
#[derive(Debug, Clone)]
pub struct WorkerNode {
    pub id: super::WorkerId,
    pub backend_type: BackendType,
    pub status: WorkerStatus,
    pub resources: super::backend::ComputeResource,
    pub labels: HashMap<String, String>,
    pub taints: Vec<Taint>,
    pub backend_specific: BackendSpecific,
    pub location: NodeLocation,
}

impl WorkerNode {
    /// Check if node matches job requirements
    pub fn matches_requirements(&self, job: &Job) -> bool {
        // Check node selector
        if let Some(selector) = &job.spec.node_selector {
            if !self.matches_node_selector(selector) {
                return false;
            }
        }

        // Check taints and tolerations
        if !self.has_matching_tolerations(&job.spec.tolerations) {
            return false;
        }

        // Check affinity rules (simplified)
        if let Some(affinity) = &job.spec.affinity {
            if !self.matches_affinity(affinity) {
                return false;
            }
        }

        // Check resources
        if let Some(req) = &job.spec.resource_requirements {
            if !self.resources.has_resources(req) {
                return false;
            }
        }

        true
    }

    /// Check if node matches node selector
    fn matches_node_selector(&self, selector: &NodeSelector) -> bool {
        for (key, value) in &selector.labels {
            match self.labels.get(key) {
                Some(node_value) => {
                    if node_value != value {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }

    /// Check if node has matching tolerations
    fn has_matching_tolerations(&self, tolerations: &[Toleration]) -> bool {
        for taint in &self.taints {
            let has_matching_toleration = tolerations.iter().any(|tol| {
                tol.key == taint.key && tol.value == taint.value && tol.effect == taint.effect
            });

            // If there's no matching toleration and effect is NoSchedule, reject
            if !has_matching_toleration && taint.effect == TaintEffect::NoSchedule {
                return false;
            }
        }

        true
    }

    /// Check if node matches affinity rules (simplified)
    fn matches_affinity(&self, _affinity: &NodeAffinity) -> bool {
        // Simplified: assume all nodes match for now
        // In production: implement full affinity matching logic
        true
    }

    /// Calculate score for this node based on job
    pub fn calculate_score(&self, job: &Job, weights: &ScoringWeights) -> f64 {
        let mut score = 0.0;

        // Resource fit score (0-100)
        if let Some(req) = &job.spec.resource_requirements {
            let utilization = self.resources.utilization(req);
            let resource_score = 100.0 * (1.0 - utilization.abs()); // Lower utilization = higher score
            score += resource_score * weights.resource_fit;
        }

        // Location score
        let location_score = 100.0; // Simplified: all locations equal
        score += location_score * weights.location;

        // Backend score
        let backend_score = if self.backend_type == BackendType::Kubernetes {
            100.0
        } else {
            80.0
        };
        score += backend_score * weights.backend;

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_priority_comparison() {
        assert!(JobPriority::Critical > JobPriority::High);
        assert!(JobPriority::High > JobPriority::Medium);
        assert!(JobPriority::Medium > JobPriority::Low);
        assert!(JobPriority::Low > JobPriority::Batch);
    }

    #[test]
    fn test_job_priority_preemption() {
        let critical = JobPriority::Critical;
        let medium = JobPriority::Medium;

        assert!(critical.can_preempt(&medium));
        assert!(!medium.can_preempt(&critical));
    }

    #[test]
    fn test_job_priority_value() {
        assert_eq!(JobPriority::Critical.value(), 5);
        assert_eq!(JobPriority::High.value(), 4);
        assert_eq!(JobPriority::Medium.value(), 3);
        assert_eq!(JobPriority::Low.value(), 2);
        assert_eq!(JobPriority::Batch.value(), 1);
    }

    #[test]
    fn test_worker_status_is_available() {
        assert!(WorkerStatus::Ready.is_available());
        assert!(WorkerStatus::Running.is_available());
        assert!(!WorkerStatus::Pending.is_available());
        assert!(!WorkerStatus::Draining.is_available());
        assert!(!WorkerStatus::Offline.is_available());
        assert!(!WorkerStatus::Failed.is_available());
    }

    #[test]
    fn test_resource_requirements_is_valid() {
        let valid = ResourceRequirements {
            cpu_cores: Some(2.0),
            memory_bytes: Some(4_000_000_000),
            gpu_count: Some(1),
            ephemeral_storage: None,
        };

        assert!(valid.is_valid());

        let invalid = ResourceRequirements {
            cpu_cores: Some(0.0),
            memory_bytes: Some(4_000_000_000),
            gpu_count: Some(1),
            ephemeral_storage: None,
        };

        assert!(!invalid.is_valid());
    }
}
