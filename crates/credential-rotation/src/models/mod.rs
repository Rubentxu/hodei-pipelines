//! Credential Rotation Models
//!
//! This module contains all data types for the credential rotation system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Types of credential providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    Simple,
    HashiCorpVault,
    AwsSecretsManager,
    KeycloakServiceAccount,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Simple => write!(f, "simple"),
            ProviderType::HashiCorpVault => write!(f, "hashicorp-vault"),
            ProviderType::AwsSecretsManager => write!(f, "aws-secrets-manager"),
            ProviderType::KeycloakServiceAccount => write!(f, "keycloak-service-account"),
        }
    }
}

/// Credential information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub value: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

/// Rotation policy configuration
#[derive(Debug, Clone)]
pub struct RotationPolicy {
    pub enabled: bool,
    pub rotation_interval: Duration,
    pub rotation_window: Option<Duration>,
    pub max_rotation_time: Duration,
    pub retry_attempts: u32,
    pub rollback_on_failure: bool,
}

/// Rotation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationStatus {
    Idle,
    Scheduled,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

/// Rotation event
#[derive(Debug, Clone)]
pub struct RotationEvent {
    pub credential_id: String,
    pub status: RotationStatus,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
}

/// Rotation configuration
#[derive(Debug, Clone)]
pub struct RotationConfig {
    pub policies: HashMap<String, RotationPolicy>,
    pub crypto_key_rotation_interval: Option<Duration>,
    pub event_retention_days: u32,
}
