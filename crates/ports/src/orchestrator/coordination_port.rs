//! Pipeline Coordination Port
//!
//! Defines the interface for coordinating pipeline execution between bounded contexts.

use async_trait::async_trait;
use hodei_pipelines_domain::{
    Result, pipeline_execution::entities::execution::ExecutionId,
    pipeline_execution::entities::pipeline::PipelineId,
};
use std::collections::HashMap;

/// Context for pipeline execution coordination
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    pub tenant_id: String,
    pub variables: HashMap<String, String>,
    pub correlation_id: Option<String>,
    pub priority: Option<u8>,
}

/// Result of a pipeline coordination request
#[derive(Debug, Clone)]
pub struct CoordinationResult {
    pub execution_id: ExecutionId,
    pub estimated_duration_secs: Option<u64>,
    pub resource_requirements: HashMap<String, u32>,
}

/// Pipeline Coordination Port
/// Coordinates pipeline execution across bounded contexts
#[async_trait]
pub trait PipelineCoordinationPort: Send + Sync {
    /// Coordinate the execution of a pipeline
    ///
    /// This method is responsible for:
    /// 1. Validating pipeline and dependencies
    /// 2. Coordinating with scheduling for worker allocation
    /// 3. Coordinating with resource governance for resource allocation
    /// 4. Applying resilience policies
    /// 5. Returning the execution plan
    ///
    /// # Arguments
    ///
    /// * `pipeline_id` - The ID of the pipeline to execute
    /// * `context` - Execution context including tenant, variables, and policies
    ///
    /// # Returns
    ///
    /// Returns the coordination result including execution ID and resource requirements
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Pipeline does not exist
    /// - Insufficient resources available
    /// - Coordination with other bounded contexts fails
    /// - Resilience policy violations occur
    async fn coordinate_pipeline_execution(
        &self,
        pipeline_id: PipelineId,
        context: ExecutionContext,
    ) -> Result<CoordinationResult>;

    /// Cancel an ongoing pipeline execution
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to cancel
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Execution does not exist
    /// - Cancellation is not possible (execution already completed)
    async fn cancel_pipeline_execution(&self, execution_id: ExecutionId) -> Result<()>;

    /// Get the status of a pipeline coordination
    ///
    /// # Arguments
    ///
    /// * `execution_id` - The ID of the execution to query
    ///
    /// # Returns
    ///
    /// Returns the current coordination status
    ///
    /// # Errors
    ///
    /// Returns an error if the execution does not exist
    async fn get_coordination_status(
        &self,
        execution_id: ExecutionId,
    ) -> Result<CoordinationStatus>;
}

/// Status of a pipeline coordination
#[derive(Debug, Clone, PartialEq)]
pub enum CoordinationStatus {
    /// Coordination is in progress
    InProgress {
        current_step: Option<String>,
        completed_steps: Vec<String>,
        estimated_remaining_secs: Option<u64>,
    },
    /// Coordination completed successfully
    Completed {
        completed_at: std::time::SystemTime,
        execution_summary: HashMap<String, String>,
    },
    /// Coordination failed
    Failed {
        failed_at: std::time::SystemTime,
        error: String,
        retry_count: u8,
    },
    /// Coordination was cancelled
    Cancelled {
        cancelled_at: std::time::SystemTime,
        reason: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_coordination_port_exists() {
        // This test verifies that the PipelineCoordinationPort trait exists and can be used
        // In a real implementation, this would test actual coordination behavior

        let context = ExecutionContext {
            tenant_id: "test-tenant".to_string(),
            variables: HashMap::new(),
            correlation_id: None,
            priority: None,
        };

        // Test that we can create an ExecutionContext
        assert_eq!(context.tenant_id, "test-tenant");
        assert!(context.variables.is_empty());

        // Test that CoordinationResult can be created
        let result = CoordinationResult {
            execution_id: ExecutionId::new(),
            estimated_duration_secs: Some(3600),
            resource_requirements: HashMap::new(),
        };

        assert!(result.estimated_duration_secs.is_some());
        assert!(result.resource_requirements.is_empty());

        // Test CoordinationStatus variants
        let status = CoordinationStatus::InProgress {
            current_step: Some("step1".to_string()),
            completed_steps: vec![],
            estimated_remaining_secs: Some(1800),
        };

        match status {
            CoordinationStatus::InProgress { current_step, .. } => {
                assert_eq!(current_step, Some("step1".to_string()));
            }
            _ => panic!("Expected InProgress status"),
        }
    }
}
