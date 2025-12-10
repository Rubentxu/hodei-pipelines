//! Conflict Resolution Port
//!
//! Defines the interface for resolving conflicts between bounded contexts
//! in the orchestrator bounded context.

use async_trait::async_trait;
use hodei_pipelines_domain::Result;
use std::collections::HashMap;

/// Type of resource conflict
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    /// Resource contention (multiple executions competing for same resources)
    ResourceContention,
    /// Dependency conflict (circular dependencies or missing dependencies)
    DependencyConflict,
    /// Scheduling conflict (workers not available)
    SchedulingConflict,
    /// Priority conflict (higher priority execution blocked)
    PriorityConflict,
    /// Quota exceeded (tenant exceeded resource quotas)
    QuotaExceeded,
    /// Timeout conflict (operation taking too long)
    TimeoutConflict,
}

/// Severity level of the conflict
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictSeverity {
    /// Low severity, can be resolved automatically
    Low,
    /// Medium severity, requires coordination
    Medium,
    /// High severity, requires intervention
    High,
    /// Critical severity, blocks all operations
    Critical,
}

/// Information about a conflict
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub conflict_id: String,
    pub conflict_type: ConflictType,
    pub severity: ConflictSeverity,
    pub description: String,
    pub affected_execution_ids: Vec<String>,
    pub resource_requirements: HashMap<String, u32>,
    pub estimated_resolution_time_secs: Option<u64>,
}

/// Resolution strategy for conflicts
#[derive(Debug, Clone)]
pub enum ResolutionStrategy {
    /// Automatically resolve using available resources
    AutoResolve,
    /// Queue execution until resources are available
    QueueExecution,
    /// Execute with reduced resources
    ReduceResources { max_resources: HashMap<String, u32> },
    /// Execute with lower priority
    LowerPriority { new_priority: u8 },
    /// Execute with timeout adjustment
    AdjustTimeout { new_timeout_secs: u64 },
    /// Fail the execution
    FailExecution { reason: String },
    /// Manual intervention required
    ManualIntervention { contact_info: String },
}

/// Result of conflict resolution
#[derive(Debug, Clone)]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub resolution_strategy: ResolutionStrategy,
    pub resolution_timestamp: std::time::SystemTime,
    pub success: bool,
    pub message: String,
    pub retry_suggested: bool,
    pub retry_after_secs: Option<u64>,
}

/// Conflict Resolution Port
/// Manages conflict detection and resolution in the orchestrator
#[async_trait]
pub trait ConflictResolutionPort: Send + Sync {
    /// Detect potential conflicts for a pipeline execution
    ///
    /// # Arguments
    ///
    /// * `pipeline_id` - The ID of the pipeline
    /// * `resource_requirements` - Required resources for execution
    /// * `priority` - Execution priority (higher number = higher priority)
    /// * `tenant_id` - The tenant requesting execution
    ///
    /// # Returns
    ///
    /// Returns a vector of detected conflicts (empty if no conflicts)
    ///
    /// # Errors
    ///
    /// Returns an error if conflict detection fails
    async fn detect_conflicts(
        &self,
        pipeline_id: &str,
        resource_requirements: HashMap<String, u32>,
        priority: u8,
        tenant_id: &str,
    ) -> Result<Vec<ConflictInfo>>;

    /// Resolve a detected conflict
    ///
    /// # Arguments
    ///
    /// * `conflict_id` - The ID of the conflict to resolve
    /// * `preferred_strategy` - Preferred resolution strategy
    ///
    /// # Returns
    ///
    /// Returns the resolution result
    ///
    /// # Errors
    ///
    /// Returns an error if resolution fails
    async fn resolve_conflict(
        &self,
        conflict_id: &str,
        preferred_strategy: Option<ResolutionStrategy>,
    ) -> Result<ConflictResolution>;

    /// Get conflicts for a specific execution
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution
    ///
    /// # Returns
    ///
    /// Returns all conflicts affecting the execution
    ///
    /// # Errors
    ///
    /// Returns an error if conflicts cannot be retrieved
    async fn get_execution_conflicts(&self, execution_id: &str) -> Result<Vec<ConflictInfo>>;

    /// Get all active conflicts
    ///
    /// # Arguments
    ///
    /// * `severity_filter` - Optional filter by severity level
    /// * `limit` - Maximum number of conflicts to return
    ///
    /// # Returns
    ///
    /// Returns all active conflicts matching the filter
    ///
    /// # Errors
    ///
    /// Returns an error if conflicts cannot be retrieved
    async fn get_active_conflicts(
        &self,
        severity_filter: Option<ConflictSeverity>,
        limit: Option<usize>,
    ) -> Result<Vec<ConflictInfo>>;

    /// Escalate a conflict for manual intervention
    ///
    /// # Arguments
    ///
    /// * `conflict_id` - The ID of the conflict to escalate
    /// * `escalation_reason` - Reason for escalation
    /// * `contact_info` - Contact information for the escalation
    ///
    /// # Errors
    ///
    /// Returns an error if escalation fails
    async fn escalate_conflict(
        &self,
        conflict_id: &str,
        escalation_reason: &str,
        contact_info: &str,
    ) -> Result<()>;

    /// Acknowledge and clear a resolved conflict
    ///
    /// # Arguments
    ///
    /// * `conflict_id` - The ID of the conflict to acknowledge
    ///
    /// # Errors
    ///
    /// Returns an error if acknowledgment fails
    async fn acknowledge_conflict(&self, conflict_id: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_conflict_resolution_port_exists() {
        // Test ConflictType variants
        let resource_conflict = ConflictType::ResourceContention;
        assert_eq!(resource_conflict, ConflictType::ResourceContention);

        let dependency_conflict = ConflictType::DependencyConflict;
        assert_eq!(dependency_conflict, ConflictType::DependencyConflict);

        let quota_conflict = ConflictType::QuotaExceeded;
        assert_eq!(quota_conflict, ConflictType::QuotaExceeded);

        // Test ConflictSeverity variants
        let low_severity = ConflictSeverity::Low;
        assert_eq!(low_severity, ConflictSeverity::Low);

        let high_severity = ConflictSeverity::High;
        assert_eq!(high_severity, ConflictSeverity::High);

        // Test ConflictInfo creation
        let conflict_info = ConflictInfo {
            conflict_id: "conflict-001".to_string(),
            conflict_type: ConflictType::ResourceContention,
            severity: ConflictSeverity::Medium,
            description: "Insufficient CPU resources".to_string(),
            affected_execution_ids: vec!["exec-1".to_string(), "exec-2".to_string()],
            resource_requirements: {
                let mut req = HashMap::new();
                req.insert("cpu".to_string(), 8);
                req.insert("memory".to_string(), 16384);
                req
            },
            estimated_resolution_time_secs: Some(300),
        };

        assert_eq!(conflict_info.conflict_id, "conflict-001");
        assert_eq!(conflict_info.severity, ConflictSeverity::Medium);
        assert_eq!(conflict_info.affected_execution_ids.len(), 2);

        // Test ResolutionStrategy variants
        let auto_strategy = ResolutionStrategy::AutoResolve;
        match auto_strategy {
            ResolutionStrategy::AutoResolve => {}
            _ => panic!("Expected AutoResolve strategy"),
        }

        let reduce_strategy = ResolutionStrategy::ReduceResources {
            max_resources: {
                let mut res = HashMap::new();
                res.insert("cpu".to_string(), 4);
                res.insert("memory".to_string(), 8192);
                res
            },
        };
        match reduce_strategy {
            ResolutionStrategy::ReduceResources { max_resources } => {
                assert_eq!(max_resources.get("cpu"), Some(&4));
            }
            _ => panic!("Expected ReduceResources strategy"),
        }

        // Test ConflictResolution creation
        let resolution = ConflictResolution {
            conflict_id: "conflict-001".to_string(),
            resolution_strategy: ResolutionStrategy::AutoResolve,
            resolution_timestamp: std::time::SystemTime::now(),
            success: true,
            message: "Conflict resolved automatically".to_string(),
            retry_suggested: false,
            retry_after_secs: None,
        };

        assert!(resolution.success);
        assert!(!resolution.retry_suggested);
    }
}
