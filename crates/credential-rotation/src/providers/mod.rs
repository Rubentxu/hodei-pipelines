//! Credential Providers Module
//!
//! This module provides trait and implementations for different credential providers.

use crate::CredentialRotationError;
use crate::models::{Credential, ProviderType, RotationEvent};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;

/// Credential provider trait
#[async_trait]
pub trait CredentialProvider: Send + Sync {
    /// Get provider type
    fn get_provider_type(&self) -> ProviderType;

    /// Store a credential
    async fn store_credential(
        &self,
        credential: &Credential,
    ) -> Result<(), CredentialRotationError>;

    /// Retrieve a credential
    async fn get_credential(
        &self,
        credential_id: &str,
    ) -> Result<Credential, CredentialRotationError>;

    /// Update a credential
    async fn update_credential(
        &self,
        credential_id: &str,
        new_value: &str,
    ) -> Result<Credential, CredentialRotationError>;

    /// Delete a credential
    async fn delete_credential(&self, credential_id: &str) -> Result<(), CredentialRotationError>;

    /// List all credentials
    async fn list_credentials(&self) -> Result<Vec<Credential>, CredentialRotationError>;

    /// Rotate a credential
    async fn rotate_credential(
        &self,
        credential_id: &str,
    ) -> Result<RotationEvent, CredentialRotationError>;

    /// Get credential health
    async fn health_check(&self) -> Result<bool, CredentialRotationError>;
}

/// Simple credential provider for testing
pub struct SimpleCredentialProvider {
    credentials: std::sync::Arc<std::sync::Mutex<HashMap<String, Credential>>>,
}

impl SimpleCredentialProvider {
    pub fn new() -> Self {
        Self {
            credentials: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CredentialProvider for SimpleCredentialProvider {
    fn get_provider_type(&self) -> ProviderType {
        ProviderType::Simple
    }

    async fn store_credential(
        &self,
        credential: &Credential,
    ) -> Result<(), CredentialRotationError> {
        let mut creds = self.credentials.lock().unwrap();
        creds.insert(credential.id.clone(), credential.clone());
        Ok(())
    }

    async fn get_credential(
        &self,
        credential_id: &str,
    ) -> Result<Credential, CredentialRotationError> {
        let creds = self.credentials.lock().unwrap();
        creds.get(credential_id).cloned().ok_or_else(|| {
            CredentialRotationError::ProviderError(format!(
                "Credential {} not found",
                credential_id
            ))
        })
    }

    async fn update_credential(
        &self,
        credential_id: &str,
        new_value: &str,
    ) -> Result<Credential, CredentialRotationError> {
        let mut creds = self.credentials.lock().unwrap();
        if let Some(cred) = creds.get_mut(credential_id) {
            cred.value = new_value.to_string();
            Ok(cred.clone())
        } else {
            Err(CredentialRotationError::ProviderError(format!(
                "Credential {} not found",
                credential_id
            )))
        }
    }

    async fn delete_credential(&self, credential_id: &str) -> Result<(), CredentialRotationError> {
        let mut creds = self.credentials.lock().unwrap();
        creds.remove(credential_id);
        Ok(())
    }

    async fn list_credentials(&self) -> Result<Vec<Credential>, CredentialRotationError> {
        let creds = self.credentials.lock().unwrap();
        Ok(creds.values().cloned().collect())
    }

    async fn rotate_credential(
        &self,
        credential_id: &str,
    ) -> Result<RotationEvent, CredentialRotationError> {
        // Simulate credential rotation
        let timestamp = Utc::now();
        let new_value = format!("rotated-{}", timestamp.timestamp());
        self.update_credential(credential_id, &new_value).await?;

        Ok(RotationEvent {
            credential_id: credential_id.to_string(),
            status: crate::models::RotationStatus::Completed,
            timestamp,
            message: "Credential rotated successfully".to_string(),
            old_version: Some("v1".to_string()),
            new_version: Some("v2".to_string()),
        })
    }

    async fn health_check(&self) -> Result<bool, CredentialRotationError> {
        Ok(true)
    }
}
