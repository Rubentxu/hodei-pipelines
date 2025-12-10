//! Provider Aggregate Root
//!
//! Represents a provider in the system with its current status and capabilities

use crate::shared_kernel::{ProviderCapabilities, ProviderId, ProviderType};

/// Provider aggregate root
#[derive(Debug, Clone, PartialEq)]
pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    pub provider_type: ProviderType,
    pub status: ProviderStatus,
    pub capabilities: ProviderCapabilities,
    pub config: super::value_objects::ProviderConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderStatus {
    Active,
    Inactive,
    Error,
}

impl Provider {
    pub fn new(
        id: ProviderId,
        name: String,
        provider_type: ProviderType,
        capabilities: ProviderCapabilities,
        config: super::value_objects::ProviderConfig,
    ) -> Self {
        Self {
            id,
            name,
            provider_type,
            status: ProviderStatus::Active,
            capabilities,
            config,
        }
    }

    pub fn activate(&mut self) {
        self.status = ProviderStatus::Active;
    }

    pub fn deactivate(&mut self) {
        self.status = ProviderStatus::Inactive;
    }

    pub fn mark_error(&mut self) {
        self.status = ProviderStatus::Error;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_kernel::{ProviderCapabilities, ProviderId, ProviderType};

    #[test]
    fn test_create_provider() {
        let provider_id = ProviderId::new("provider-1".to_string());
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["docker".to_string()],
            memory_limit_mb: Some(4096),
            cpu_limit: Some(2.0),
        };
        let config = crate::provider_management::value_objects::ProviderConfig::new(
            "http://localhost:2375".to_string(),
        );

        let provider = Provider::new(
            provider_id.clone(),
            "Docker Provider".to_string(),
            ProviderType::Docker,
            capabilities.clone(),
            config.clone(),
        );

        assert_eq!(provider.id, provider_id);
        assert_eq!(provider.name, "Docker Provider");
        assert_eq!(provider.provider_type, ProviderType::Docker);
        assert_eq!(provider.status, ProviderStatus::Active);
        assert_eq!(provider.capabilities, capabilities);
        assert_eq!(provider.config, config);
    }

    #[test]
    fn test_activate_provider() {
        let provider_id = ProviderId::new("provider-1".to_string());
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["docker".to_string()],
            memory_limit_mb: Some(4096),
            cpu_limit: Some(2.0),
        };
        let config = crate::provider_management::value_objects::ProviderConfig::new(
            "http://localhost:2375".to_string(),
        );

        let mut provider = Provider::new(
            provider_id,
            "Docker Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            config,
        );

        // Initially active
        assert_eq!(provider.status, ProviderStatus::Active);

        // Deactivate
        provider.deactivate();
        assert_eq!(provider.status, ProviderStatus::Inactive);

        // Reactivate
        provider.activate();
        assert_eq!(provider.status, ProviderStatus::Active);
    }

    #[test]
    fn test_mark_error() {
        let provider_id = ProviderId::new("provider-1".to_string());
        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["docker".to_string()],
            memory_limit_mb: Some(4096),
            cpu_limit: Some(2.0),
        };
        let config = crate::provider_management::value_objects::ProviderConfig::new(
            "http://localhost:2375".to_string(),
        );

        let mut provider = Provider::new(
            provider_id,
            "Docker Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            config,
        );

        provider.mark_error();

        assert_eq!(provider.status, ProviderStatus::Error);
    }
}
