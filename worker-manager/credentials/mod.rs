//! Credential Management Module
//! 
//! This module provides a complete credential management system including
//! multiple credential providers and automatic rotation capabilities.

use crate::traits::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Credential Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub name: String,
    pub values: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rotation_enabled: bool,
    pub access_policy: Option<AccessPolicy>,
}

/// Access Policy for credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub allowed_subjects: Vec<String>,
    pub read_permissions: Vec<String>,
    pub write_permissions: Vec<String>,
    pub rotation_permissions: Vec<String>,
}

/// Secret Rotation Strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationStrategy {
    TimeBased(TimeBasedRotation),
    EventBased(EventBasedRotation),
    Manual,
}

/// Time-based rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedRotation {
    pub rotation_interval: Duration,
    pub rotation_time: Option<chrono::DateTime<chrono::Utc>>,
    pub grace_period: Duration,
}

/// Event-based rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBasedRotation {
    pub triggers: Vec<RotationTrigger>,
    pub max_concurrent_rotations: usize,
}

/// Rotation trigger types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RotationTrigger {
    CredentialAccessed,
    UnauthorizedAccessAttempt,
    KeyCompromised,
    CertificateExpiry(Duration),
    ComplianceRequirement,
}

/// Rotation result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    pub success: bool,
    pub rotated_credential: Option<String>,
    pub old_credential_version: Option<String>,
    pub new_credential_version: Option<String>,
    pub rotated_at: chrono::DateTime<chrono::Utc>,
    pub error_message: Option<String>,
}

/// Credential Audit Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_type: CredentialEventType,
    pub credential_name: String,
    pub subject: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub details: HashMap<String, String>,
}

/// Types of credential events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialEventType {
    CredentialCreated,
    CredentialRead,
    CredentialUpdated,
    CredentialDeleted,
    RotationStarted,
    RotationCompleted,
    RotationFailed,
    AccessDenied,
}

/// Versioned Credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedCredential {
    pub credential: Credential,
    pub version: String,
    pub version_created_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

/// Credential Provider Trait
#[async_trait::async_trait]
pub trait CredentialProvider: Send + Sync {
    /// Get credential by name
    async fn get_credential(&self, name: &str, version: Option<&str>) -> Result<VersionedCredential, CredentialError>;
    
    /// Put/update credential
    async fn put_credential(&self, credential: &Credential) -> Result<VersionedCredential, CredentialError>;
    
    /// List all versions of a credential
    async fn list_credential_versions(&self, name: &str) -> Result<Vec<VersionedCredential>, CredentialError>;
    
    /// List all credentials
    async fn list_credentials(&self) -> Result<Vec<VersionedCredential>, CredentialError>;
    
    /// Delete credential (soft delete)
    async fn delete_credential(&self, name: &str) -> Result<(), CredentialError>;
    
    /// Check if credential exists
    async fn credential_exists(&self, name: &str) -> Result<bool, CredentialError>;
    
    /// Rotate credential using specified strategy
    async fn rotate_credential(&self, name: &str, strategy: &RotationStrategy) -> Result<RotationResult, CredentialError>;
    
    /// Get access policy for credential
    async fn get_access_policy(&self, name: &str) -> Result<Option<AccessPolicy>, CredentialError>;
    
    /// Set access policy for credential
    async fn set_access_policy(&self, name: &str, policy: &AccessPolicy) -> Result<(), CredentialError>;
    
    /// Check if subject has permission for credential
    async fn has_permission(&self, name: &str, subject: &str, permission: &str) -> Result<bool, CredentialError>;
    
    /// Audit credential operation
    async fn audit_operation(&self, event: &AuditEvent) -> Result<(), CredentialError>;
    
    /// Get credential statistics
    async fn get_statistics(&self) -> Result<CredentialStatistics, CredentialError>;
}

/// Credential statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStatistics {
    pub total_credentials: u64,
    pub active_credentials: u64,
    pub credential_versions: u64,
    pub rotation_statistics: RotationStatistics,
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
}

/// Rotation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationStatistics {
    pub total_rotations: u64,
    pub successful_rotations: u64,
    pub failed_rotations: u64,
    pub last_rotation: Option<chrono::DateTime<chrono::Utc>>,
    pub next_scheduled_rotation: Option<chrono::DateTime<chrono::Utc>>,
}

/// Simple in-memory credential provider
#[derive(Debug)]
pub struct SimpleCredentialProvider {
    credentials: std::sync::RwLock<HashMap<String, Vec<VersionedCredential>>>,
    audit_log: std::sync::RwLock<Vec<AuditEvent>>,
    rotation_config: std::sync::RwLock<HashMap<String, RotationStrategy>>,
}

impl SimpleCredentialProvider {
    /// Create new simple credential provider
    pub fn new() -> Self {
        Self {
            credentials: std::sync::RwLock::new(HashMap::new()),
            audit_log: std::sync::RwLock::new(Vec::new()),
            rotation_config: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Generate a new version identifier
    fn generate_version() -> String {
        format!("v{}_{}", 
               chrono::Utc::now().timestamp_millis(),
               uuid::Uuid::new_v4().hyphenated().to_string()[0..8].to_string())
    }
}

#[async_trait::async_trait]
impl CredentialProvider for SimpleCredentialProvider {
    async fn get_credential(&self, name: &str, version: Option<&str>) -> Result<VersionedCredential, CredentialError> {
        let credentials = self.credentials.read().map_err(|_| CredentialError::InternalError)?;
        
        let versions = credentials.get(name)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        let target_version = match version {
            Some(v) => {
                versions.iter().find(|vc| vc.version == v)
                    .cloned()
                    .ok_or_else(|| CredentialError::VersionNotFound { name: name.to_string(), version: v.to_string() })?
            }
            None => {
                versions.iter().find(|vc| vc.is_active)
                    .cloned()
                    .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?
            }
        };

        // Audit the access
        let audit_event = AuditEvent {
            event_type: CredentialEventType::CredentialRead,
            credential_name: name.to_string(),
            subject: "system".to_string(),
            timestamp: chrono::Utc::now(),
            success: true,
            details: HashMap::new(),
        };
        let _ = self.audit_operation(&audit_event).await;

        Ok(target_version)
    }

    async fn put_credential(&self, credential: &Credential) -> Result<VersionedCredential, CredentialError> {
        let mut credentials = self.credentials.write().map_err(|_| CredentialError::InternalError)?;
        
        let version = Self::generate_version();
        let versioned_credential = VersionedCredential {
            credential: credential.clone(),
            version: version.clone(),
            version_created_at: chrono::Utc::now(),
            is_active: true,
        };

        // Deactivate previous versions
        if let Some(versions) = credentials.get_mut(credential.name.as_str()) {
            for v in versions.iter_mut() {
                v.is_active = false;
            }
        } else {
            credentials.insert(credential.name.clone(), Vec::new());
        }

        // Add new version
        let versions = credentials.get_mut(credential.name.as_str()).unwrap();
        versions.push(versioned_credential.clone());

        // Audit the creation
        let audit_event = AuditEvent {
            event_type: CredentialEventType::CredentialCreated,
            credential_name: credential.name.clone(),
            subject: "system".to_string(),
            timestamp: chrono::Utc::now(),
            success: true,
            details: HashMap::from([
                ("version".to_string(), version),
            ]),
        };
        let _ = self.audit_operation(&audit_event).await;

        Ok(versioned_credential)
    }

    async fn list_credential_versions(&self, name: &str) -> Result<Vec<VersionedCredential>, CredentialError> {
        let credentials = self.credentials.read().map_err(|_| CredentialError::InternalError)?;
        
        let versions = credentials.get(name)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        Ok(versions.clone())
    }

    async fn list_credentials(&self) -> Result<Vec<VersionedCredential>, CredentialError> {
        let credentials = self.credentials.read().map_err(|_| CredentialError::InternalError)?;
        
        let mut active_credentials = Vec::new();
        for versions in credentials.values() {
            if let Some(active) = versions.iter().find(|vc| vc.is_active) {
                active_credentials.push(active.clone());
            }
        }
        
        Ok(active_credentials)
    }

    async fn delete_credential(&self, name: &str) -> Result<(), CredentialError> {
        let mut credentials = self.credentials.write().map_err(|_| CredentialError::InternalError)?;
        
        if let Some(versions) = credentials.get_mut(name) {
            // Soft delete by deactivating all versions
            for version in versions.iter_mut() {
                version.is_active = false;
            }
            
            // Audit the deletion
            let audit_event = AuditEvent {
                event_type: CredentialEventType::CredentialDeleted,
                credential_name: name.to_string(),
                subject: "system".to_string(),
                timestamp: chrono::Utc::now(),
                success: true,
                details: HashMap::new(),
            };
            let _ = self.audit_operation(&audit_event).await;
            
            Ok(())
        } else {
            Err(CredentialError::NotFound { name: name.to_string() })
        }
    }

    async fn credential_exists(&self, name: &str) -> Result<bool, CredentialError> {
        let credentials = self.credentials.read().map_err(|_| CredentialError::InternalError)?;
        Ok(credentials.contains_key(name))
    }

    async fn rotate_credential(&self, name: &str, strategy: &RotationStrategy) -> Result<RotationResult, CredentialError> {
        let mut credentials = self.credentials.write().map_err(|_| CredentialError::InternalError)?;
        
        let versions = credentials.get_mut(name)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        // Get current active credential
        let current_active = versions.iter()
            .find(|vc| vc.is_active)
            .cloned()
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;

        // Audit rotation start
        let audit_event = AuditEvent {
            event_type: CredentialEventType::RotationStarted,
            credential_name: name.to_string(),
            subject: "system".to_string(),
            timestamp: chrono::Utc::now(),
            success: true,
            details: HashMap::new(),
        };
        let _ = self.audit_operation(&audit_event).await;

        // Deactivate current version
        for version in versions.iter_mut() {
            if version.is_active {
                version.is_active = false;
                break;
            }
        }

        // Create new version with updated expiry
        let mut new_credential = current_active.credential.clone();
        let new_expires_at = match strategy {
            RotationStrategy::TimeBased(time_based) => {
                Some(chrono::Utc::now() + time_based.rotation_interval)
            }
            RotationStrategy::EventBased(_) => {
                // For event-based, set a reasonable default expiry
                Some(chrono::Utc::now() + chrono::Duration::days(30))
            }
            RotationStrategy::Manual => {
                new_credential.expires_at
            }
        };

        new_credential.expires_at = new_expires_at;
        new_credential.updated_at = chrono::Utc::now();

        let new_version = Self::generate_version();
        let new_versioned_credential = VersionedCredential {
            credential: new_credential,
            version: new_version.clone(),
            version_created_at: chrono::Utc::now(),
            is_active: true,
        };

        versions.push(new_versioned_credential);

        // Audit rotation completion
        let audit_event = AuditEvent {
            event_type: CredentialEventType::RotationCompleted,
            credential_name: name.to_string(),
            subject: "system".to_string(),
            timestamp: chrono::Utc::now(),
            success: true,
            details: HashMap::from([
                ("old_version".to_string(), current_active.version),
                ("new_version".to_string(), new_version),
            ]),
        };
        let _ = self.audit_operation(&audit_event).await;

        Ok(RotationResult {
            success: true,
            rotated_credential: Some(name.to_string()),
            old_credential_version: Some(current_active.version),
            new_credential_version: Some(new_version),
            rotated_at: chrono::Utc::now(),
            error_message: None,
        })
    }

    async fn get_access_policy(&self, name: &str) -> Result<Option<AccessPolicy>, CredentialError> {
        let credentials = self.credentials.read().map_err(|_| CredentialError::InternalError)?;
        
        let versions = credentials.get(name)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        let active = versions.iter()
            .find(|vc| vc.is_active)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        Ok(active.credential.access_policy.clone())
    }

    async fn set_access_policy(&self, name: &str, policy: &AccessPolicy) -> Result<(), CredentialError> {
        let mut credentials = self.credentials.write().map_err(|_| CredentialError::InternalError)?;
        
        let versions = credentials.get_mut(name)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        // Update active version's access policy
        for version in versions.iter_mut() {
            if version.is_active {
                version.credential.access_policy = Some(policy.clone());
                version.credential.updated_at = chrono::Utc::now();
                break;
            }
        }

        Ok(())
    }

    async fn has_permission(&self, name: &str, subject: &str, permission: &str) -> Result<bool, CredentialError> {
        let credentials = self.credentials.read().map_err(|_| CredentialError::InternalError)?;
        
        let versions = credentials.get(name)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        let active = versions.iter()
            .find(|vc| vc.is_active)
            .ok_or_else(|| CredentialError::NotFound { name: name.to_string() })?;
        
        if let Some(policy) = &active.credential.access_policy {
            Ok(policy.allowed_subjects.contains(subject) &&
               match permission {
                   "read" => policy.read_permissions.is_empty() || policy.read_permissions.contains(permission),
                   "write" => policy.write_permissions.is_empty() || policy.write_permissions.contains(permission),
                   "rotate" => policy.rotation_permissions.is_empty() || policy.rotation_permissions.contains(permission),
                   _ => false,
               })
        } else {
            // Default: allow access to all subjects
            Ok(true)
        }
    }

    async fn audit_operation(&self, event: &AuditEvent) -> Result<(), CredentialError> {
        let mut audit_log = self.audit_log.write().map_err(|_| CredentialError::InternalError)?;
        audit_log.push(event.clone());
        
        // Limit audit log size to prevent memory issues
        if audit_log.len() > 10000 {
            audit_log.drain(0..5000);
        }
        
        Ok(())
    }

    async fn get_statistics(&self) -> Result<CredentialStatistics, CredentialError> {
        let credentials = self.credentials.read().map_err(|_| CredentialError::InternalError)?;
        
        let mut total_credentials = 0u64;
        let mut active_credentials = 0u64;
        let mut total_versions = 0u64;
        let mut last_accessed = None::<chrono::DateTime<chrono::Utc>>;
        
        for versions in credentials.values() {
            total_credentials += 1;
            total_versions += versions.len() as u64;
            
            for version in versions {
                if version.is_active {
                    active_credentials += 1;
                }
                if last_accessed.is_none() || version.version_created_at > last_accessed.unwrap() {
                    last_accessed = Some(version.version_created_at);
                }
            }
        }

        let audit_log = self.audit_log.read().map_err(|_| CredentialError::InternalError)?;
        let rotations = audit_log.iter()
            .filter(|event| matches!(event.event_type, CredentialEventType::RotationCompleted))
            .collect::<Vec<_>>();

        Ok(CredentialStatistics {
            total_credentials,
            active_credentials,
            credential_versions: total_versions,
            rotation_statistics: RotationStatistics {
                total_rotations: rotations.len() as u64,
                successful_rotations: rotations.iter().filter(|e| e.success).count() as u64,
                failed_rotations: rotations.iter().filter(|e| !e.success).count() as u64,
                last_rotation: rotations.last().map(|e| e.timestamp),
                next_scheduled_rotation: None, // Would be calculated based on strategies
            },
            last_accessed,
        })
    }
}

impl Default for SimpleCredentialProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Extended error types for credentials
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("Credential not found: {name}")]
    NotFound { name: String },
    
    #[error("Credential version not found: {name} version {version}")]
    VersionNotFound { name: String, version: String },
    
    #[error("Credential already exists: {name}")]
    AlreadyExists { name: String },
    
    #[error("Invalid credential format: {name}")]
    InvalidFormat { name: String },
    
    #[error("Permission denied for credential: {name}")]
    PermissionDenied { name: String },
    
    #[error("Credential limit exceeded: {name}")]
    LimitExceeded { name: String },
    
    #[error("Internal error")]
    InternalError,
    
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("Storage error: {message}")]
    Storage { message: String },
    
    #[error("Encryption error: {message}")]
    Encryption { message: String },
    
    #[error("Rotation failed: {name}")]
    RotationFailed { name: String },
}
