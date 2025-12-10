//! Provider Management Use Cases

use super::{Provider, ProviderConfig, ProviderRepository};
use crate::shared_kernel::{DomainResult, ProviderCapabilities, ProviderId, ProviderType};

/// Register a new provider
pub struct RegisterProviderUseCase {
    provider_repo: Box<dyn ProviderRepository>,
}

impl RegisterProviderUseCase {
    pub fn new(provider_repo: Box<dyn ProviderRepository>) -> Self {
        Self { provider_repo }
    }

    pub async fn execute(
        &self,
        name: String,
        provider_type: ProviderType,
        capabilities: ProviderCapabilities,
        config: ProviderConfig,
    ) -> DomainResult<Provider> {
        let provider = Provider::new(
            ProviderId::new(uuid::Uuid::new_v4().to_string()),
            name,
            provider_type,
            capabilities,
            config,
        );

        self.provider_repo.save(&provider).await?;
        Ok(provider)
    }
}

/// List all providers
pub struct ListProvidersUseCase {
    provider_repo: Box<dyn ProviderRepository>,
}

impl ListProvidersUseCase {
    pub fn new(provider_repo: Box<dyn ProviderRepository>) -> Self {
        Self { provider_repo }
    }

    pub async fn execute(&self) -> DomainResult<Vec<Provider>> {
        self.provider_repo.list().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_management::value_objects::ProviderConfig;
    use crate::provider_management::{entities::ProviderStatus, Provider};
    use crate::shared_kernel::{ProviderCapabilities, ProviderId, ProviderType};

    struct MockProviderRepository {
        providers: std::sync::Mutex<std::collections::HashMap<String, Provider>>,
    }

    impl MockProviderRepository {
        fn new() -> Self {
            Self {
                providers: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl ProviderRepository for MockProviderRepository {
        async fn save(&self, provider: &Provider) -> crate::shared_kernel::DomainResult<()> {
            let mut providers = self.providers.lock().unwrap();
            providers.insert(provider.id.to_string(), provider.clone());
            Ok(())
        }

        async fn find_by_id(
            &self,
            id: &ProviderId,
        ) -> crate::shared_kernel::DomainResult<Option<Provider>> {
            let providers = self.providers.lock().unwrap();
            Ok(providers.get(&id.to_string()).cloned())
        }

        async fn list(&self) -> crate::shared_kernel::DomainResult<Vec<Provider>> {
            let providers = self.providers.lock().unwrap();
            Ok(providers.values().cloned().collect())
        }

        async fn delete(&self, id: &ProviderId) -> crate::shared_kernel::DomainResult<()> {
            let mut providers = self.providers.lock().unwrap();
            providers.remove(&id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_register_provider_use_case_success() {
        let repo = MockProviderRepository::new();
        let use_case = RegisterProviderUseCase::new(Box::new(repo));

        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["docker".to_string()],
            memory_limit_mb: Some(4096),
            cpu_limit: Some(2.0),
        };

        let config = crate::provider_management::value_objects::ProviderConfig::new(
            "http://localhost:2375".to_string(),
        );

        let result = use_case
            .execute(
                "Docker Provider".to_string(),
                ProviderType::Docker,
                capabilities.clone(),
                config.clone(),
            )
            .await
            .unwrap();

        assert_eq!(result.name, "Docker Provider");
        assert_eq!(result.provider_type, ProviderType::Docker);
        assert_eq!(result.capabilities, capabilities);
        assert_eq!(result.status, ProviderStatus::Active);
    }

    #[tokio::test]
    async fn test_list_providers_use_case_success() {
        // Pre-populate repository with providers
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

        let repo = MockProviderRepository::new();
        {
            let mut providers = repo.providers.lock().unwrap();
            providers.insert("provider-1".to_string(), provider1);
            providers.insert("provider-2".to_string(), provider2);
        }

        let use_case = ListProvidersUseCase::new(Box::new(repo));
        let result = use_case.execute().await.unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|p| p.name == "Docker Provider"));
        assert!(result.iter().any(|p| p.name == "K8s Provider"));
    }

    #[tokio::test]
    async fn test_list_providers_use_case_empty() {
        let repo = MockProviderRepository::new();
        let use_case = ListProvidersUseCase::new(Box::new(repo));

        let result = use_case.execute().await.unwrap();

        assert!(result.is_empty());
    }
}
