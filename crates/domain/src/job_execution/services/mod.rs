//! Job Execution Services
//!
//! Domain services for job execution logic

use crate::shared_kernel::{ProviderCapabilities, ProviderId, ProviderType};

/// Information about an available provider
pub struct ProviderInfo {
    pub id: ProviderId,
    pub provider_type: ProviderType,
    pub capabilities: ProviderCapabilities,
    pub current_jobs: u32,
}

/// Job Scheduler Service
///
/// Selects the best provider for a job based on capabilities and availability
pub struct JobScheduler;

impl JobScheduler {
    pub fn new() -> Self {
        Self
    }

    /// Selects the best provider for a job
    pub fn select_provider(
        &self,
        _job: &super::entities::Job,
        available_providers: &[ProviderInfo],
    ) -> crate::shared_kernel::DomainResult<ProviderId> {
        if available_providers.is_empty() {
            return Err(crate::shared_kernel::DomainError::NotFound(
                "No providers available".to_string(),
            ));
        }

        // Simple selection: return first available provider
        Ok(available_providers[0].id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_execution::ExecutionContext;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_execution_context_creation() {
        let job_id = crate::shared_kernel::JobId::new("job-123".to_string());
        let provider_id = crate::shared_kernel::ProviderId::new("provider-1".to_string());
        let execution_id = "exec-456".to_string();

        let context =
            ExecutionContext::new(job_id.clone(), provider_id.clone(), execution_id.clone());

        assert_eq!(context.job_id, job_id);
        assert_eq!(context.provider_id, provider_id);
        assert_eq!(context.execution_id, execution_id);
        assert!(context.started_at <= chrono::Utc::now());
        assert!(context.result.is_none());
    }

    #[test]
    fn test_execution_context_with_result() {
        let job_id = crate::shared_kernel::JobId::new("job-123".to_string());
        let provider_id = crate::shared_kernel::ProviderId::new("provider-1".to_string());

        let mut context =
            ExecutionContext::new(job_id.clone(), provider_id.clone(), "exec-456".to_string());

        let result = crate::shared_kernel::JobResult {
            success: true,
            output: Some("Job completed successfully".to_string()),
            error: None,
            execution_time_ms: 1500,
        };

        context.result = Some(result.clone());

        assert_eq!(context.result, Some(result));
    }
}
