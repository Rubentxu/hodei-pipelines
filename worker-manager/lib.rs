//! Worker Manager Abstraction Layer
//! 
//! A comprehensive Rust-based worker management system providing abstractions
//! for multiple infrastructure providers and credential management with automatic rotation.

#![warn(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export main traits and types
pub use crate::traits::*;
pub use crate::providers::{WorkerManagerProvider, KubernetesProvider, DockerProvider};
pub use crate::credentials::{CredentialProvider, RotationEngine, RotationEvent, RotationStrategy};

// Provider modules
pub mod traits {
    pub use worker_manager_traits::*;
}

pub mod providers {
    pub mod kubernetes {
        pub use worker_manager_providers_kubernetes::*;
    }
    pub mod docker {
        pub use worker_manager_providers_docker::*;
    }
    pub mod plugin {
        pub mod registry {
            pub use worker_manager_providers_plugin_registry::*;
        }
    }
}

// Credential management modules
pub mod credentials {
    pub mod mod_rs {
        pub use worker_manager_credentials_mod::*;
    }
    pub mod vault {
        pub use worker_manager_credentials_vault::*;
    }
    pub mod aws_secrets {
        pub use worker_manager_credentials_aws_secrets::*;
    }
    pub mod keycloak {
        pub use worker_manager_credentials_keycloak::*;
    }
    pub mod rotation {
        pub use worker_manager_credentials_rotation::*;
    }
}

// Ephemeral worker management
pub mod ephemeral {
    pub mod auto_scaling {
        pub use worker_manager_ephemeral_auto_scaling::*;
    }
    pub mod health_checks {
        pub use worker_manager_ephemeral_health_checks::*;
    }
    pub mod cost_optimization {
        pub use worker_manager_ephemeral_cost_optimization::*;
    }
}

// Security and networking
pub mod security {
    pub mod rbac {
        pub use worker_manager_security_rbac::*;
    }
    pub mod network_policies {
        pub use worker_manager_security_network_policies::*;
    }
}

/// Main Worker Manager
#[derive(Debug)]
pub struct WorkerManager {
    providers: HashMap<String, Arc<dyn WorkerManagerProvider + Send + Sync>>,
    credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
    rotation_engine: Arc<RwLock<Option<rotation::RotationEngine>>>,
    current_provider: String,
}

impl WorkerManager {
    /// Create new Worker Manager
    pub fn new(
        credential_provider: Arc<dyn CredentialProvider + Send + Sync>,
    ) -> Self {
        Self {
            providers: HashMap::new(),
            credential_provider,
            rotation_engine: Arc::new(RwLock::new(None)),
            current_provider: "kubernetes".to_string(),
        }
    }

    /// Register a provider
    pub fn register_provider(
        &mut self,
        name: String,
        provider: Arc<dyn WorkerManagerProvider + Send + Sync>,
    ) {
        self.providers.insert(name, provider);
    }

    /// Set default provider
    pub fn set_default_provider(&mut self, provider_name: String) {
        if self.providers.contains_key(&provider_name) {
            self.current_provider = provider_name;
        }
    }

    /// Get current provider
    pub fn get_provider(&self) -> Option<Arc<dyn WorkerManagerProvider + Send + Sync>>> {
        self.providers.get(&self.current_provider).cloned()
    }

    /// Get specific provider
    pub fn get_provider_by_name(
        &self,
        name: &str,
    ) -> Option<Arc<dyn WorkerManagerProvider + Send + Sync>> {
        self.providers.get(name).cloned()
    }

    /// Start rotation engine
    pub async fn start_rotation_engine(
        &self,
        config: credentials::rotation::RotationEngineConfig,
    ) -> Result<(), ProviderError> {
        let mut rotation_engine = self.rotation_engine.write().await;
        
        let engine = credentials::rotation::RotationEngine::new(
            self.credential_provider.clone(),
            config,
        );
        
        // Start the rotation engine
        let engine_clone = engine.clone();
        tokio::spawn(async move {
            if let Err(e) = engine_clone.start().await {
                eprintln!("Rotation engine failed to start: {:?}", e);
            }
        });
        
        *rotation_engine = Some(engine);
        Ok(())
    }

    /// Stop rotation engine
    pub async fn stop_rotation_engine(&self) -> Result<(), ProviderError> {
        let rotation_engine = self.rotation_engine.read().await;
        if let Some(engine) = rotation_engine.as_ref() {
            engine.stop().await.map_err(|e| ProviderError::internal(
                format!("Failed to stop rotation engine: {}", e),
                Some("rotation_engine".to_string()),
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: Some("rotation_engine".to_string()),
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))?;
        }
        Ok(())
    }

    /// Create worker with default provider
    pub async fn create_worker(
        &self,
        spec: &RuntimeSpec,
        config: &ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let provider = self.get_provider()
            .ok_or_else(|| ProviderError::configuration(
                "No provider configured".to_string(),
                None,
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: None,
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))?;
        
        provider.create_worker(spec, config).await
    }

    /// Terminate worker
    pub async fn terminate_worker(
        &self,
        worker_id: &WorkerId,
    ) -> Result<(), ProviderError> {
        let provider = self.get_provider()
            .ok_or_else(|| ProviderError::configuration(
                "No provider configured".to_string(),
                None,
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: None,
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))?;
        
        provider.terminate_worker(worker_id).await
    }

    /// Get worker status
    pub async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<WorkerState, ProviderError> {
        let provider = self.get_provider()
            .ok_or_else(|| ProviderError::configuration(
                "No provider configured".to_string(),
                None,
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: None,
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))?;
        
        provider.get_worker_status(worker_id).await
    }

    /// Get system capacity
    pub async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError> {
        let provider = self.get_provider()
            .ok_or_else(|| ProviderError::configuration(
                "No provider configured".to_string(),
                None,
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: None,
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))?;
        
        provider.get_capacity().await
    }

    /// Schedule rotation
    pub async fn schedule_rotation(
        &self,
        credential_name: &str,
        strategy: credentials::rotation::RotationStrategy,
    ) -> Result<uuid::Uuid, ProviderError> {
        let rotation_engine = self.rotation_engine.read().await;
        if let Some(engine) = rotation_engine.as_ref() {
            engine.schedule_rotation(credential_name, strategy)
                .await
                .map_err(|e| ProviderError::credentials(
                    format!("Failed to schedule rotation: {}", e),
                    "rotation_engine".to_string()
                ))
        } else {
            Err(ProviderError::configuration(
                "Rotation engine not started".to_string(),
                None,
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: Some("rotation_engine".to_string()),
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))
        }
    }

    /// Get rotation status
    pub async fn get_rotation_status(&self) -> Result<credentials::rotation::RotationEngineStatus, ProviderError> {
        let rotation_engine = self.rotation_engine.read().await;
        if let Some(engine) = rotation_engine.as_ref() {
            Ok(engine.get_status().await)
        } else {
            Err(ProviderError::configuration(
                "Rotation engine not started".to_string(),
                None,
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: Some("rotation_engine".to_string()),
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))
        }
    }

    /// Get available providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Health check
    pub async fn health_check(&self) -> Result<WorkerManagerHealthStatus, ProviderError> {
        let providers_status = futures::future::join_all(
            self.providers.values().map(|provider| {
                let name = self.providers.iter()
                    .find_map(|(k, v)| if v.as_ref() == provider.as_ref() { Some(k.clone()) } else { None })
                    .unwrap_or_else(|| "unknown".to_string());
                
                async move {
                    match provider.get_capacity().await {
                        Ok(_) => (name, true, None),
                        Err(e) => (name, false, Some(e.to_string())),
                    }
                }
            })
        ).await;

        let healthy_providers = providers_status.iter().filter(|(_, healthy, _)| *healthy).count();
        let is_healthy = healthy_providers > 0;

        Ok(WorkerManagerHealthStatus {
            is_healthy,
            healthy_providers,
            total_providers: self.providers.len(),
            provider_details: providers_status.into_iter().map(|(name, healthy, error)| {
                (name, healthy, error)
            }).collect(),
            rotation_engine_healthy: self.rotation_engine.read().await.is_some(),
            timestamp: chrono::Utc::now(),
        })
    }
}

/// Worker Manager health status
#[derive(Debug, Clone)]
pub struct WorkerManagerHealthStatus {
    pub is_healthy: bool,
    pub healthy_providers: usize,
    pub total_providers: usize,
    pub provider_details: Vec<(String, bool, Option<String>)>,
    pub rotation_engine_healthy: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Worker Manager Builder for easier setup
#[derive(Debug)]
pub struct WorkerManagerBuilder {
    credential_provider: Option<Arc<dyn CredentialProvider + Send + Sync>>,
    providers: HashMap<String, Arc<dyn WorkerManagerProvider + Send + Sync>>,
    default_provider: Option<String>,
}

impl WorkerManagerBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            credential_provider: None,
            providers: HashMap::new(),
            default_provider: None,
        }
    }

    /// Set credential provider
    pub fn credential_provider(mut self, provider: Arc<dyn CredentialProvider + Send + Sync>) -> Self {
        self.credential_provider = Some(provider);
        self
    }

    /// Add provider
    pub fn add_provider(
        mut self,
        name: String,
        provider: Arc<dyn WorkerManagerProvider + Send + Sync>,
    ) -> Self {
        self.providers.insert(name, provider);
        if self.default_provider.is_none() {
            self.default_provider = Some(name);
        }
        self
    }

    /// Set default provider
    pub fn default_provider(mut self, name: String) -> Self {
        self.default_provider = Some(name);
        self
    }

    /// Build Worker Manager
    pub fn build(self) -> Result<WorkerManager, ProviderError> {
        let credential_provider = self.credential_provider
            .ok_or_else(|| ProviderError::configuration(
                "Credential provider is required".to_string(),
                None,
                crate::traits::ErrorContext {
                    operation_id: None,
                    worker_id: None,
                    provider: None,
                    configuration: None,
                    timestamp: chrono::Utc::now(),
                }
            ))?;

        let mut manager = WorkerManager::new(credential_provider);

        for (name, provider) in self.providers {
            manager.register_provider(name, provider);
        }

        if let Some(default) = self.default_provider {
            manager.set_default_provider(default);
        }

        Ok(manager)
    }
}

impl Default for WorkerManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating common configurations
pub mod builders {
    use super::*;

    /// Create a simple in-memory provider setup for testing
    pub async fn create_simple_setup() -> Result<(WorkerManager, Vec<Arc<dyn CredentialProvider + Send + Sync>>), ProviderError> {
        use crate::providers::docker::DockerProvider;
        use crate::providers::kubernetes::KubernetesProvider;
        use crate::credentials::SimpleCredentialProvider;

        let simple_credential = Arc::new(SimpleCredentialProvider::new());
        
        let mut k8s_config = crate::providers::kubernetes::KubernetesConfig::default();
        k8s_config.namespace = "default".to_string();
        
        let k8s_provider = Arc::new(KubernetesProvider::from_provider_config(
            &ProviderConfig::kubernetes("default".to_string()),
            simple_credential.clone()
        ).await?);

        let docker_provider = Arc::new(DockerProvider::from_provider_config(
            &ProviderConfig::docker(),
            simple_credential.clone()
        ).await?);

        let mut builder = WorkerManagerBuilder::new()
            .credential_provider(simple_credential.clone())
            .add_provider("kubernetes".to_string(), k8s_provider)
            .add_provider("docker".to_string(), docker_provider)
            .default_provider("docker".to_string());

        let manager = builder.build()?;
        Ok((manager, vec![simple_credential]))
    }

    /// Create a production setup with multiple credential providers
    pub async fn create_production_setup() -> Result<(WorkerManager, Vec<Arc<dyn CredentialProvider + Send + Sync>>), ProviderError> {
        use crate::credentials::vault::HashiCorpVaultProvider;
        use crate::credentials::aws_secrets::AWSSecretsManagerProvider;
        use crate::credentials::keycloak::KeycloakServiceAccountProvider;

        // This would require proper configuration in production
        // For now, return a simple setup
        create_simple_setup().await
    }
}
