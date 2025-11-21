//! Credential Rotation System (US-021)
//!
//! This module provides a comprehensive credential rotation system that supports
//! multiple credential providers with zero-downtime rotation capabilities.
//!
//! Features:
//! - Multiple credential providers (Simple, Vault, AWS Secrets Manager, Keycloak)
//! - Automatic rotation scheduling
//! - Zero-downtime rotation with rolling updates
//! - Health monitoring and automatic rollback
//! - Cryptographic operations (AES-256 encryption)
//! - Rotation event tracking and auditing

pub mod crypto;
pub mod engine;
pub mod models;
pub mod providers;

pub use engine::RotationEngine;
pub use models::RotationConfig;
pub use models::*;
pub use providers::CredentialProvider;

/// Error types for credential rotation system
#[derive(thiserror::Error, Debug)]
pub enum CredentialRotationError {
    #[error("credential provider error: {0}")]
    ProviderError(String),

    #[error("rotation execution failed: {0}")]
    RotationFailed(String),

    #[error("crypto operation failed: {0}")]
    CryptoError(String),

    #[error("provider not supported: {0}")]
    ProviderNotSupported(String),

    #[error("rotation timeout: {0}")]
    Timeout(String),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::time::Duration;

    #[tokio::test]
    async fn test_simple_credential_provider() {
        // RED: Test SimpleCredentialProvider operations
        let provider = providers::SimpleCredentialProvider::new();

        // GREEN: Create a test credential
        let credential = Credential {
            id: "test-cred-1".to_string(),
            name: "Test Credential".to_string(),
            provider_type: ProviderType::Simple,
            value: "secret-value".to_string(),
            version: "v1".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            metadata: HashMap::new(),
        };

        // Store and retrieve
        provider.store_credential(&credential).await.unwrap();
        let retrieved = provider.get_credential("test-cred-1").await.unwrap();
        assert_eq!(retrieved.value, "secret-value");

        // Rotate
        let event = provider.rotate_credential("test-cred-1").await.unwrap();
        assert_eq!(event.status, RotationStatus::Completed);
    }

    #[tokio::test]
    async fn test_rotation_engine() {
        // RED: Test RotationEngine functionality
        let mut engine = RotationEngine::new();

        // Add a simple provider
        use std::sync::Arc;
        let provider = Arc::new(providers::SimpleCredentialProvider::new());
        engine.add_provider("simple".to_string(), provider.clone());

        // Configure rotation
        let config = RotationConfig {
            policies: HashMap::new(),
            crypto_key_rotation_interval: None,
            event_retention_days: 30,
        };
        engine.configure(config).await;

        // Test credential operations
        let credential = Credential {
            id: "engine-test-cred".to_string(),
            name: "Engine Test Credential".to_string(),
            provider_type: ProviderType::Simple,
            value: "engine-secret".to_string(),
            version: "v1".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            metadata: HashMap::new(),
        };

        provider.store_credential(&credential).await.unwrap();
        let event = engine
            .rotate_credential("simple", "engine-test-cred")
            .await
            .unwrap();
        assert_eq!(event.status, RotationStatus::Completed);

        // Health check
        let health = engine.health_check().await.unwrap();
        assert!(health);
    }

    #[tokio::test]
    async fn test_crypto_operations() {
        // RED: Test cryptographic operations
        let key = crypto::generate_random_key(32).unwrap();
        let data = b"secret data to encrypt";

        // Encrypt
        let encrypted = crypto::simple_encrypt(data, &key).unwrap();
        assert_ne!(encrypted, data);

        // Decrypt
        let decrypted = crypto::simple_decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);

        // Test key derivation
        let password = b"my-password";
        let salt = b"random-salt";
        let derived_key = crypto::derive_key_from_password(password, salt, 100_000, 32).unwrap();
        assert_eq!(derived_key.len(), 32);
    }

    #[tokio::test]
    async fn test_rotation_status_tracking() {
        // RED: Test rotation status tracking
        let mut engine = RotationEngine::new();
        use std::sync::Arc;
        let provider = Arc::new(providers::SimpleCredentialProvider::new());
        engine.add_provider("simple".to_string(), provider.clone());

        let credential = Credential {
            id: "status-test-cred".to_string(),
            name: "Status Test Credential".to_string(),
            provider_type: ProviderType::Simple,
            value: "status-secret".to_string(),
            version: "v1".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            metadata: HashMap::new(),
        };

        provider.store_credential(&credential).await.unwrap();
        engine
            .rotate_credential("simple", "status-test-cred")
            .await
            .unwrap();

        // Get rotation history
        let history = engine
            .get_rotation_status("status-test-cred")
            .await
            .unwrap();
        assert!(!history.is_empty());
        assert_eq!(history[0].status, RotationStatus::Completed);
    }

    #[tokio::test]
    async fn test_provider_not_supported_error() {
        // RED: Test provider not supported error
        let engine = RotationEngine::new();

        let result = engine.rotate_credential("nonexistent", "some-cred");
        assert!(matches!(
            result.await,
            Err(CredentialRotationError::ProviderNotSupported(_))
        ));
    }

    #[tokio::test]
    async fn test_rotation_policy_configuration() {
        // RED: Test rotation policy configuration
        let engine = RotationEngine::new();

        let policy = RotationPolicy {
            enabled: true,
            rotation_interval: Duration::from_secs(3600),
            rotation_window: Some(Duration::from_secs(300)),
            max_rotation_time: Duration::from_secs(60),
            retry_attempts: 3,
            rollback_on_failure: true,
        };

        let mut policies = HashMap::new();
        policies.insert("test-policy".to_string(), policy);

        let config = RotationConfig {
            policies,
            crypto_key_rotation_interval: Some(Duration::from_secs(86400)),
            event_retention_days: 90,
        };

        engine.configure(config).await;

        // Verify configuration
        let config_reader = engine.config.read().await;
        assert!(config_reader.policies.contains_key("test-policy"));
        assert_eq!(config_reader.event_retention_days, 90);
    }
}
