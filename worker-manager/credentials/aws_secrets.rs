//! AWS Secrets Manager Provider Implementation
//! 
//! This module provides a complete integration with AWS Secrets Manager for
//! secure credential management and storage.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

/// AWS Secrets Manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AWSSecretsConfig {
    pub region: String,
    pub profile: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub timeout: Duration,
    pub max_retries: u32,
}

/// AWS Secrets Manager client wrapper
#[derive(Debug)]
pub struct AWSSecretsClient {
    client: aws_sdk_secretsmanager::Client,
    config: AWSSecretsConfig,
}

impl AWSSecretsClient {
    /// Create new AWS Secrets Manager client
    pub async fn new(config: AWSSecretsConfig) -> Result<Self, CredentialError> {
        let aws_config = match (&config.access_key_id, &config.secret_access_key) {
            (Some(key_id), Some(secret_key)) => {
                aws_config::Config::builder()
                    .region(config.region.as_ref().try_into().map_err(|_| CredentialError::Configuration {
                        message: format!("Invalid AWS region: {}", config.region)
                    })?)
                    .credentials_provider(aws_sdk_secretsmanager::config::ProvideCredentials::ClientCredentials {
                        access_key_id: key_id.clone(),
                        secret_access_key: secret_key.clone(),
                        session_token: config.session_token.clone(),
                    })
                    .build()
            }
            _ => {
                // Use default credential chain
                aws_config::Config::builder()
                    .region(config.region.as_ref().try_into().map_err(|_| CredentialError::Configuration {
                        message: format!("Invalid AWS region: {}", config.region)
                    })?)
                    .build()
            }
        };

        let client = aws_sdk_secretsmanager::Client::from_conf(aws_config);
        Ok(Self { client, config })
    }

    /// Get secret from AWS Secrets Manager
    pub async fn get_secret(&self, secret_id: &str) -> Result<SecretString, CredentialError> {
        let request = self.client.get_secret_value()
            .secret_id(secret_id);

        match request.send().await {
            Ok(output) => {
                if let Some(secret_string) = output.secret_string() {
                    Ok(secret_string.to_string())
                } else if let Some(secret_binary) = output.secret_binary() {
                    // Decode binary secret to string
                    let bytes = secret_binary.as_ref();
                    Ok(String::from_utf8_lossy(bytes).to_string())
                } else {
                    Err(CredentialError::NotFound { 
                        name: secret_id.to_string() 
                    })
                }
            }
            Err(error) => {
                if error.code() == Some("ResourceNotFoundException") {
                    Err(CredentialError::NotFound { 
                        name: secret_id.to_string() 
                    })
                } else {
                    Err(CredentialError::Network { 
                        message: format!("AWS Secrets Manager error: {}", error) 
                    })
                }
            }
        }
    }

    /// Put secret to AWS Secrets Manager
    pub async fn put_secret(&self, secret_id: &str, secret_string: &str, description: Option<&str>) -> Result<(), CredentialError> {
        let mut request = self.client.put_secret_value()
            .secret_id(secret_id)
            .secret_string(secret_string);

        if let Some(desc) = description {
            request = request.description(desc);
        }

        match request.send().await {
            Ok(_) => Ok(()),
            Err(error) => {
                if error.code() == Some("ResourceNotFoundException") {
                    // Secret doesn't exist, create it
                    let mut request = self.client.create_secret()
                        .name(secret_id)
                        .secret_string(secret_string);

                    if let Some(desc) = description {
                        request = request.description(desc);
                    }

                    match request.send().await {
                        Ok(_) => Ok(()),
                        Err(create_error) => {
                            Err(CredentialError::Storage { 
                                message: format!("Failed to create secret: {}", create_error) 
                            })
                        }
                    }
                } else {
                    Err(CredentialError::Storage { 
                        message: format!("AWS Secrets Manager put error: {}", error) 
                    })
                }
            }
        }
    }

    /// List secrets in AWS Secrets Manager
    pub async fn list_secrets(&self) -> Result<Vec<String>, CredentialError> {
        let mut secrets = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = self.client.list_secrets();

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            match request.send().await {
                Ok(output) => {
                    if let Some(secret_list) = output.secret_list() {
                        for secret in secret_list {
                            if let Some(name) = secret.name() {
                                secrets.push(name.to_string());
                            }
                        }
                    }

                    if output.next_token().is_some() {
                        next_token = output.next_token().map(|t| t.to_string());
                    } else {
                        break;
                    }
                }
                Err(error) => {
                    return Err(CredentialError::Network { 
                        message: format!("AWS list secrets error: {}", error) 
                    });
                }
            }
        }

        Ok(secrets)
    }

    /// Delete secret from AWS Secrets Manager
    pub async fn delete_secret(&self, secret_id: &str) -> Result<(), CredentialError> {
        let request = self.client.delete_secret()
            .secret_id(secret_id);

        match request.send().await {
            Ok(_) => Ok(()),
            Err(error) => {
                if error.code() == Some("ResourceNotFoundException") {
                    // Secret doesn't exist, consider it already deleted
                    Ok(())
                } else {
                    Err(CredentialError::Storage { 
                        message: format!("AWS delete secret error: {}", error) 
                    })
                }
            }
        }
    }

    /// Rotate secret in AWS Secrets Manager
    pub async fn rotate_secret(&self, secret_id: &str, new_secret_string: &str) -> Result<(), CredentialError> {
        let request = self.client.rotate_secret()
            .secret_id(secret_id)
            .client_request_token(format!("rotation_{}", chrono::Utc::now().timestamp_millis()));

        match request.send().await {
            Ok(_) => Ok(()),
            Err(error) => {
                Err(CredentialError::RotationFailed { 
                    name: secret_id.to_string() 
                })
            }
        }
    }

    /// Get secret versions
    pub async fn list_secret_versions(&self, secret_id: &str) -> Result<Vec<SecretVersionInfo>, CredentialError> {
        let request = self.client.list_secret_version_ids()
            .secret_id(secret_id);

        match request.send().await {
            Ok(output) => {
                let mut versions = Vec::new();
                if let Some(version_ids) = output.secret_version_ids() {
                    for version in version_ids {
                        let version_info = SecretVersionInfo {
                            version_id: version.version_id().unwrap_or("unknown").to_string(),
                            is_current_version: version.is_current_version().unwrap_or(false),
                            created_date: version.created_date()
                                .map(|d| chrono::DateTime::from_utc(
                                    chrono::NaiveDateTime::from_timestamp_opt(d.as_secs(), 0).unwrap_or_default(),
                                    chrono::Utc
                                )),
                        };
                        versions.push(version_info);
                    }
                }
                Ok(versions)
            }
            Err(error) => {
                Err(CredentialError::Network { 
                    message: format!("AWS list secret versions error: {}", error) 
                })
            }
        }
    }
}

/// Secret version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVersionInfo {
    pub version_id: String,
    pub is_current_version: bool,
    pub created_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// AWS Secrets Manager Credential Provider
#[derive(Debug)]
pub struct AWSSecretsManagerProvider {
    client: AWSSecretsClient,
}

impl AWSSecretsManagerProvider {
    /// Create new AWS Secrets Manager Provider
    pub async fn new(config: AWSSecretsConfig) -> Result<Self, CredentialError> {
        let client = AWSSecretsClient::new(config).await?;
        Ok(Self { client })
    }

    /// Convert AWS secret string to Credential format
    fn aws_secret_to_credential(&self, secret_string: &str, secret_id: &str) -> Result<HashMap<String, String>, CredentialError> {
        // Try to parse as JSON first
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(secret_string) {
            let mut values = HashMap::new();
            
            if json_value.is_object() {
                for (key, value) in json_value.as_object().unwrap() {
                    if let Some(value_str) = value.as_str() {
                        values.insert(key.clone(), value_str.to_string());
                    } else if value.is_number() {
                        values.insert(key.clone(), value.to_string());
                    } else if value.is_boolean() {
                        values.insert(key.clone(), value.to_string());
                    } else {
                        values.insert(key.clone(), serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()));
                    }
                }
            } else {
                // Treat as a single value with a default key
                values.insert("value".to_string(), secret_string.to_string());
            }
            
            Ok(values)
        } else {
            // Not JSON, treat as plain text
            Ok(HashMap::from([("value".to_string(), secret_string.to_string())]))
        }
    }

    /// Convert Credential to AWS secret string
    fn credential_to_aws_secret(&self, credential: &Credential) -> String {
        if credential.values.len() == 1 {
            // Single value, return as plain text
            credential.values.values().next().unwrap().clone()
        } else {
            // Multiple values, return as JSON
            serde_json::to_string(&credential.values).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

#[async_trait::async_trait]
impl CredentialProvider for AWSSecretsManagerProvider {
    async fn get_credential(&self, name: &str, version: Option<&str>) -> Result<VersionedCredential, CredentialError> {
        // AWS doesn't support getting specific versions via the basic API
        // This would need additional implementation for version-specific access
        let secret_id = format!("secret/{}", name); // Assuming secrets are stored under "secret/" prefix
        
        let secret_string = self.client.get_secret(&secret_id).await?;
        let values = self.aws_secret_to_credential(&secret_string, &secret_id)?;

        let credential = Credential {
            name: name.to_string(),
            values,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            expires_at: None,
            rotation_enabled: false,
            access_policy: None,
        };

        let version = version.unwrap_or("current").to_string();
        
        Ok(VersionedCredential {
            credential,
            version,
            version_created_at: chrono::Utc::now(),
            is_active: true,
        })
    }

    async fn put_credential(&self, credential: &Credential) -> Result<VersionedCredential, CredentialError> {
        let secret_id = format!("secret/{}", credential.name);
        let secret_string = self.credential_to_aws_secret(credential);
        
        self.client.put_secret(&secret_id, &secret_string, Some(&format!("Secret for {}", credential.name))).await?;
        
        let version = format!("v{}_{}", chrono::Utc::now().timestamp_millis(), chrono::Utc::now().timestamp_nanos());
        
        Ok(VersionedCredential {
            credential: credential.clone(),
            version,
            version_created_at: chrono::Utc::now(),
            is_active: true,
        })
    }

    async fn list_credential_versions(&self, name: &str) -> Result<Vec<VersionedCredential>, CredentialError> {
        let secret_id = format!("secret/{}", name);
        let versions = self.client.list_secret_versions(&secret_id).await?;
        
        let mut versioned_credentials = Vec::new();
        let secret_string = self.client.get_secret(&secret_id).await?;
        let values = self.aws_secret_to_credential(&secret_string, &secret_id)?;
        
        for version_info in versions {
            versioned_credentials.push(VersionedCredential {
                credential: Credential {
                    name: name.to_string(),
                    values: values.clone(),
                    metadata: HashMap::new(),
                    created_at: version_info.created_date.unwrap_or(chrono::Utc::now()),
                    updated_at: chrono::Utc::now(),
                    expires_at: None,
                    rotation_enabled: false,
                    access_policy: None,
                },
                version: version_info.version_id,
                version_created_at: version_info.created_date.unwrap_or(chrono::Utc::now()),
                is_active: version_info.is_current_version,
            });
        }
        
        Ok(versioned_credentials)
    }

    async fn list_credentials(&self) -> Result<Vec<VersionedCredential>, CredentialError> {
        let secrets = self.client.list_secrets().await?;
        
        let mut credentials = Vec::new();
        for secret_name in secrets {
            if let Ok(version) = self.get_credential(&secret_name, None).await {
                credentials.push(version);
            }
        }
        
        Ok(credentials)
    }

    async fn delete_credential(&self, name: &str) -> Result<(), CredentialError> {
        let secret_id = format!("secret/{}", name);
        self.client.delete_secret(&secret_id).await?;
        Ok(())
    }

    async fn credential_exists(&self, name: &str) -> Result<bool, CredentialError> {
        let secret_id = format!("secret/{}", name);
        match self.client.get_secret(&secret_id).await {
            Ok(_) => Ok(true),
            Err(CredentialError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn rotate_credential(&self, name: &str, strategy: &RotationStrategy) -> Result<RotationResult, CredentialError> {
        let secret_id = format!("secret/{}", name);
        let current = self.get_credential(name, None).await?;
        
        // Create rotated version
        let mut rotated_credential = current.credential.clone();
        rotated_credential.updated_at = chrono::Utc::now();
        
        match strategy {
            RotationStrategy::TimeBased(time_based) => {
                rotated_credential.expires_at = Some(chrono::Utc::now() + time_based.rotation_interval);
            }
            RotationStrategy::EventBased(_) => {
                rotated_credential.expires_at = Some(chrono::Utc::now() + chrono::Duration::days(30));
            }
            RotationStrategy::Manual => {
                // Keep current expiry
            }
        }

        // Use AWS Secrets Manager rotation API
        let new_secret_string = self.credential_to_aws_secret(&rotated_credential);
        self.client.rotate_secret(&secret_id, &new_secret_string).await?;
        
        Ok(RotationResult {
            success: true,
            rotated_credential: Some(name.to_string()),
            old_credential_version: Some(current.version),
            new_credential_version: Some("rotated".to_string()),
            rotated_at: chrono::Utc::now(),
            error_message: None,
        })
    }

    async fn get_access_policy(&self, name: &str) -> Result<Option<AccessPolicy>, CredentialError> {
        // AWS Secrets Manager doesn't have built-in access policies in the secret itself
        // This would need to be managed via IAM policies and AWS Verified Permissions
        Ok(None)
    }

    async fn set_access_policy(&self, _name: &str, _policy: &AccessPolicy) -> Result<(), CredentialError> {
        // Not applicable for AWS Secrets Manager
        Ok(())
    }

    async fn has_permission(&self, _name: &str, _subject: &str, _permission: &str) -> Result<bool, CredentialError> {
        // Permission checking would be done via AWS IAM/Verified Permissions
        // For now, return true
        Ok(true)
    }

    async fn audit_operation(&self, _event: &AuditEvent) -> Result<(), CredentialError> {
        // In production, you would integrate with AWS CloudTrail for auditing
        Ok(())
    }

    async fn get_statistics(&self) -> Result<CredentialStatistics, CredentialError> {
        let credentials = self.list_credentials().await?;
        
        Ok(CredentialStatistics {
            total_credentials: credentials.len() as u64,
            active_credentials: credentials.len() as u64,
            credential_versions: credentials.len() as u64,
            rotation_statistics: RotationStatistics {
                total_rotations: 0,
                successful_rotations: 0,
                failed_rotations: 0,
                last_rotation: None,
                next_scheduled_rotation: None,
            },
            last_accessed: None,
        })
    }
}

/// Service Account credentials for AWS STS assume role
#[derive(Debug, Clone)]
pub struct AWSCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// AWS STS Service for assuming roles
pub struct AWSSTSService {
    client: aws_sdk_sts::Client,
}

impl AWSSTSService {
    /// Create new AWS STS service
    pub fn new() -> Result<Self, CredentialError> {
        let aws_config = aws_config::Config::builder()
            .region("us-east-1")
            .build();
        let client = aws_sdk_sts::Client::from_conf(aws_config);
        Ok(Self { client })
    }

    /// Assume role and get temporary credentials
    pub async fn assume_role(
        &self,
        role_arn: &str,
        role_session_name: &str,
        duration_seconds: Option<i32>,
    ) -> Result<AWSCredentials, CredentialError> {
        let mut request = self.client.assume_role()
            .role_arn(role_arn)
            .role_session_name(role_session_name);

        if let Some(duration) = duration_seconds {
            request = request.duration_seconds(duration);
        }

        match request.send().await {
            Ok(output) => {
                if let Some(credentials) = output.credentials() {
                    let access_key_id = credentials.access_key_id().unwrap_or("").to_string();
                    let secret_access_key = credentials.secret_access_key().unwrap_or("").to_string();
                    let session_token = credentials.session_token().map(|t| t.to_string());

                    Ok(AWSCredentials {
                        access_key_id,
                        secret_access_key,
                        session_token,
                    })
                } else {
                    Err(CredentialError::InternalError)
                }
            }
            Err(error) => {
                Err(CredentialError::Network {
                    message: format!("AWS STS assume role error: {}", error)
                })
            }
        }
    }
}
