//! Provider Services

use super::entities::Provider;

/// Provider Registry Service
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn get_all_providers(&self, providers: &[Provider]) -> Vec<Provider> {
        providers.to_vec()
    }

    pub fn get_healthy_providers(&self, providers: &[Provider]) -> Vec<Provider> {
        providers
            .iter()
            .filter(|p| p.status == super::entities::ProviderStatus::Active)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_kernel::{ProviderCapabilities, ProviderId, ProviderType};

    #[test]
    fn test_get_all_providers() {
        let registry = ProviderRegistry::new();

        let provider1 = Provider::new(
            ProviderId::new("provider-1".to_string()),
            "Docker Provider".to_string(),
            ProviderType::Docker,
            ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["docker".to_string()],
                memory_limit_mb: Some(4096),
                cpu_limit: Some(2.0),
            },
            crate::provider_management::value_objects::ProviderConfig::new(
                "http://localhost:2375".to_string(),
            ),
        );

        let provider2 = Provider::new(
            ProviderId::new("provider-2".to_string()),
            "K8s Provider".to_string(),
            ProviderType::Kubernetes,
            ProviderCapabilities {
                max_concurrent_jobs: 50,
                supported_job_types: vec!["k8s".to_string()],
                memory_limit_mb: Some(8192),
                cpu_limit: Some(4.0),
            },
            crate::provider_management::value_objects::ProviderConfig::new(
                "http://k8s:8080".to_string(),
            ),
        );

        let providers = vec![provider1.clone(), provider2.clone()];
        let result = registry.get_all_providers(&providers);

        assert_eq!(result.len(), 2);
        assert!(result.contains(&provider1));
        assert!(result.contains(&provider2));
    }

    #[test]
    fn test_get_healthy_providers() {
        let registry = ProviderRegistry::new();

        let mut provider1 = Provider::new(
            ProviderId::new("provider-1".to_string()),
            "Active Provider".to_string(),
            ProviderType::Docker,
            ProviderCapabilities {
                max_concurrent_jobs: 10,
                supported_job_types: vec!["docker".to_string()],
                memory_limit_mb: Some(4096),
                cpu_limit: Some(2.0),
            },
            crate::provider_management::value_objects::ProviderConfig::new(
                "http://localhost:2375".to_string(),
            ),
        );

        let mut provider2 = Provider::new(
            ProviderId::new("provider-2".to_string()),
            "Inactive Provider".to_string(),
            ProviderType::Kubernetes,
            ProviderCapabilities {
                max_concurrent_jobs: 50,
                supported_job_types: vec!["k8s".to_string()],
                memory_limit_mb: Some(8192),
                cpu_limit: Some(4.0),
            },
            crate::provider_management::value_objects::ProviderConfig::new(
                "http://k8s:8080".to_string(),
            ),
        );

        let mut provider3 = Provider::new(
            ProviderId::new("provider-3".to_string()),
            "Error Provider".to_string(),
            ProviderType::Lambda,
            ProviderCapabilities {
                max_concurrent_jobs: 100,
                supported_job_types: vec!["lambda".to_string()],
                memory_limit_mb: Some(1024),
                cpu_limit: Some(1.0),
            },
            crate::provider_management::value_objects::ProviderConfig::new(
                "http://lambda:8080".to_string(),
            ),
        );

        provider1.activate(); // Active
        provider2.deactivate(); // Inactive
        provider3.mark_error(); // Error

        let providers = vec![provider1.clone(), provider2.clone(), provider3.clone()];
        let result = registry.get_healthy_providers(&providers);

        assert_eq!(result.len(), 1);
        assert!(result.contains(&provider1));
        assert!(!result.contains(&provider2));
        assert!(!result.contains(&provider3));
    }

    #[test]
    fn test_get_healthy_providers_empty_list() {
        let registry = ProviderRegistry::new();
        let providers = Vec::<Provider>::new();

        let result = registry.get_healthy_providers(&providers);

        assert!(result.is_empty());
    }
}
