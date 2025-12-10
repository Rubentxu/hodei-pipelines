//! Application Service for Provider Management

use domain::provider_management::{Provider, ProviderRepository};
use domain::shared_kernel::{DomainError, DomainResult, ProviderId, ProviderType, ProviderCapabilities};

pub struct ProviderService {
    provider_repo: Box<dyn ProviderRepository>,
}

impl ProviderService {
    pub fn new(provider_repo: Box<dyn ProviderRepository>) -> Self {
        Self { provider_repo }
    }

    pub async fn register_provider(
        &self,
        id: ProviderId,
        name: String,
        provider_type: ProviderType,
        capabilities: ProviderCapabilities,
        config: domain::provider_management::value_objects::ProviderConfig,
    ) -> DomainResult<Provider> {
        let provider = Provider::new(id, name, provider_type, capabilities, config);
        self.provider_repo.save(&provider).await?;
        Ok(provider)
    }

    pub async fn list_providers(&self) -> DomainResult<Vec<Provider>> {
        self.provider_repo.list().await
    }

    pub async fn get_provider(&self, id: &ProviderId) -> DomainResult<Provider> {
        self.provider_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Provider not found".to_string()))
    }

    pub async fn activate_provider(&self, id: &ProviderId) -> DomainResult<()> {
        let mut provider = self.get_provider(id).await?;
        provider.activate();
        self.provider_repo.save(&provider).await?;
        Ok(())
    }

    pub async fn deactivate_provider(&self, id: &ProviderId) -> DomainResult<()> {
        let mut provider = self.get_provider(id).await?;
        provider.deactivate();
        self.provider_repo.save(&provider).await?;
        Ok(())
    }

    pub async fn delete_provider(&self, id: &ProviderId) -> DomainResult<()> {
        self.provider_repo.delete(id).await?;
        Ok(())
    }

    pub async fn get_provider_status(&self, id: &ProviderId) -> DomainResult<domain::provider_management::entities::ProviderStatus> {
        let provider = self.get_provider(id).await?;
        Ok(provider.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::provider_management::{Provider, ProviderRepository};
    use domain::shared_kernel::{ProviderId, ProviderType, ProviderCapabilities};
    use domain::provider_management::entities::ProviderStatus;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock implementations for testing
    struct MockProviderRepository {
        providers: Arc<Mutex<Vec<Provider>>>,
    }

    impl MockProviderRepository {
        fn new() -> Self {
            Self {
                providers: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl ProviderRepository for MockProviderRepository {
        async fn save(&self, provider: &Provider) -> domain::shared_kernel::DomainResult<()> {
            let mut providers = self.providers.lock().await;

            if let Some(index) = providers.iter().position(|p| p.id == provider.id) {
                providers[index] = provider.clone();
            } else {
                providers.push(provider.clone());
            }

            Ok(())
        }

        async fn find_by_id(&self, id: &ProviderId) -> domain::shared_kernel::DomainResult<Option<Provider>> {
            let providers = self.providers.lock().await;
            Ok(providers.iter().find(|p| p.id == *id).cloned())
        }

        async fn list(&self) -> domain::shared_kernel::DomainResult<Vec<Provider>> {
            let providers = self.providers.lock().await;
            Ok(providers.clone())
        }

        async fn delete(&self, id: &ProviderId) -> domain::shared_kernel::DomainResult<()> {
            let mut providers = self.providers.lock().await;
            providers.retain(|p| p.id != *id);
            Ok(())
        }
    }

    // ==================== TDD RED PHASE ====================
    // Writing tests BEFORE implementation

    #[tokio::test]
    async fn test_register_provider_success() {
        // Test: Should register a new provider successfully
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string(), "python".to_string()],
            memory_limit_mb: Some(2048),
            cpu_limit: Some(2.0),
        };

        let result = service.register_provider(
            ProviderId::new("docker-1".to_string()),
            "Docker Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
        ).await;

        assert!(result.is_ok(), "Failed to register provider: {:?}", result.err());

        let provider = result.unwrap();
        assert_eq!(provider.name, "Docker Provider");
        assert_eq!(provider.provider_type, ProviderType::Docker);
        assert!(matches!(provider.status, ProviderStatus::Active));
    }

    #[tokio::test]
    async fn test_list_providers() {
        // Test: Should list all registered providers
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        // Register multiple providers
        for i in 0..3 {
            let capabilities = ProviderCapabilities {
                max_concurrent_jobs: 5,
                supported_job_types: vec!["bash".to_string()],
                memory_limit_mb: Some(1024),
                cpu_limit: Some(1.0),
            };

            service.register_provider(
                ProviderId::new(format!("provider-{}", i)),
                format!("Provider {}", i),
                ProviderType::Docker,
                capabilities,
                domain::provider_management::value_objects::ProviderConfig::new(format!("http://localhost:{}", 2375 + i)),
            ).await.unwrap();
        }

        let providers = service.list_providers().await.unwrap();
        assert_eq!(providers.len(), 3, "Should have 3 providers");

        for (i, provider) in providers.iter().enumerate() {
            assert_eq!(provider.name, format!("Provider {}", i));
        }
    }

    #[tokio::test]
    async fn test_get_provider_success() {
        // Test: Should retrieve a specific provider
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };

        let provider_id = ProviderId::new("test-provider".to_string());
        service.register_provider(
            provider_id.clone(),
            "Test Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
        ).await.unwrap();

        let provider = service.get_provider(&provider_id).await.unwrap();
        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.id, provider_id);
    }

    #[tokio::test]
    async fn test_get_nonexistent_provider() {
        // Test: Should return error for non-existent provider
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        let fake_id = ProviderId::new("nonexistent".to_string());
        let result = service.get_provider(&fake_id).await;
        assert!(result.is_err(), "Should error for non-existent provider");
    }

    #[tokio::test]
    async fn test_activate_provider_success() {
        // Test: Should activate a provider
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };

        let provider_id = ProviderId::new("test-provider".to_string());
        service.register_provider(
            provider_id.clone(),
            "Test Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
        ).await.unwrap();

        // First deactivate it
        service.deactivate_provider(&provider_id).await.unwrap();
        let provider = service.get_provider(&provider_id).await.unwrap();
        assert!(matches!(provider.status, ProviderStatus::Inactive));

        // Then activate it
        service.activate_provider(&provider_id).await.unwrap();
        let activated_provider = service.get_provider(&provider_id).await.unwrap();
        assert!(matches!(activated_provider.status, ProviderStatus::Active));
    }

    #[tokio::test]
    async fn test_deactivate_provider_success() {
        // Test: Should deactivate a provider
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };

        let provider_id = ProviderId::new("test-provider".to_string());
        service.register_provider(
            provider_id.clone(),
            "Test Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
        ).await.unwrap();

        service.deactivate_provider(&provider_id).await.unwrap();
        let provider = service.get_provider(&provider_id).await.unwrap();
        assert!(matches!(provider.status, ProviderStatus::Inactive));
    }

    #[tokio::test]
    async fn test_delete_provider_success() {
        // Test: Should delete a provider
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };

        let provider_id = ProviderId::new("test-provider".to_string());
        service.register_provider(
            provider_id.clone(),
            "Test Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
        ).await.unwrap();

        // Verify provider exists
        let provider = service.get_provider(&provider_id).await.unwrap();
        assert_eq!(provider.name, "Test Provider");

        // Delete provider
        service.delete_provider(&provider_id).await.unwrap();

        // Verify provider no longer exists
        let result = service.get_provider(&provider_id).await;
        assert!(result.is_err(), "Provider should be deleted");
    }

    #[tokio::test]
    async fn test_get_provider_status() {
        // Test: Should get provider status
        let repo = MockProviderRepository::new();
        let service = ProviderService::new(Box::new(repo));

        let capabilities = ProviderCapabilities {
            max_concurrent_jobs: 10,
            supported_job_types: vec!["bash".to_string()],
            memory_limit_mb: Some(1024),
            cpu_limit: Some(1.0),
        };

        let provider_id = ProviderId::new("test-provider".to_string());
        service.register_provider(
            provider_id.clone(),
            "Test Provider".to_string(),
            ProviderType::Docker,
            capabilities,
            domain::provider_management::value_objects::ProviderConfig::new("http://localhost:2375".to_string()),
        ).await.unwrap();

        let status = service.get_provider_status(&provider_id).await.unwrap();
        assert!(matches!(status, ProviderStatus::Active));

        // Deactivate and check status again
        service.deactivate_provider(&provider_id).await.unwrap();
        let inactive_status = service.get_provider_status(&provider_id).await.unwrap();
        assert!(matches!(inactive_status, ProviderStatus::Inactive));
    }
}
