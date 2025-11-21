//! Credential Rotation Engine
//!
//! This module implements the core rotation engine that orchestrates
//! credential rotations across providers with zero-downtime.

use crate::CredentialRotationError;
use crate::models::{RotationConfig, RotationEvent};
use crate::providers::CredentialProvider;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Credential rotation engine
pub struct RotationEngine {
    providers: HashMap<String, Arc<dyn CredentialProvider>>,
    config: Arc<RwLock<RotationConfig>>,
    rotation_queue: Arc<RwLock<Vec<RotationEvent>>>,
}

impl RotationEngine {
    /// Create a new rotation engine
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            config: Arc::new(RwLock::new(RotationConfig {
                policies: HashMap::new(),
                crypto_key_rotation_interval: None,
                event_retention_days: 30,
            })),
            rotation_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a credential provider
    pub fn add_provider(&mut self, name: String, provider: Arc<dyn CredentialProvider>) {
        self.providers.insert(name, provider);
    }

    /// Configure rotation settings
    pub async fn configure(&self, config: RotationConfig) {
        let mut config_writer = self.config.write().await;
        *config_writer = config;
    }

    /// Get a read-only reference to the configuration
    pub fn get_config(&self) -> Arc<RwLock<RotationConfig>> {
        Arc::clone(&self.config)
    }

    /// Trigger a credential rotation
    pub async fn rotate_credential(
        &self,
        provider_name: &str,
        credential_id: &str,
    ) -> Result<RotationEvent, CredentialRotationError> {
        let provider = self.providers.get(provider_name).ok_or_else(|| {
            CredentialRotationError::ProviderNotSupported(format!(
                "Provider {} not found",
                provider_name
            ))
        })?;

        let event = provider.rotate_credential(credential_id).await?;

        // Add to rotation queue
        let mut queue = self.rotation_queue.write().await;
        queue.push(event.clone());

        Ok(event)
    }

    /// Get rotation status for a credential
    pub async fn get_rotation_status(
        &self,
        credential_id: &str,
    ) -> Result<Vec<RotationEvent>, CredentialRotationError> {
        let queue = self.rotation_queue.read().await;
        Ok(queue
            .iter()
            .filter(|e| e.credential_id == credential_id)
            .cloned()
            .collect())
    }

    /// Health check all providers
    pub async fn health_check(&self) -> Result<bool, CredentialRotationError> {
        for (name, provider) in &self.providers {
            let health = provider.health_check().await.map_err(|e| {
                CredentialRotationError::ProviderError(format!(
                    "Provider {} health check failed: {}",
                    name, e
                ))
            })?;
            if !health {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl Default for RotationEngine {
    fn default() -> Self {
        Self::new()
    }
}
