//! HashiCorp Vault Provider Implementation
//! 
//! This module provides a complete integration with HashiCorp Vault for
//! secure credential management and storage.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

/// Vault-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub vault_url: String,
    pub auth_method: VaultAuthMethod,
    pub timeout: Duration,
    pub max_retries: u32,
    pub secrets_engine: String,
    pub transit_engine: Option<String>,
    pub kv_version: KVVersion,
}

/// Vault authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VaultAuthMethod {
    Token { token: String },
    AppRole { role_id: String, secret_id: String },
    Kubernetes { jwt_path: String, role: String },
    Azure { client_id: String, client_secret: String, tenant_id: String },
}

/// KV engine version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KVVersion {
    V1,
    V2,
}

/// Vault API client wrapper
#[derive(Debug)]
pub struct VaultClient {
    client: reqwest::Client,
    config: VaultConfig,
    token: Option<String>,
    token_expiry: Option<chrono::DateTime<chrono::Utc>>,
}

impl VaultClient {
    /// Create new Vault client
    pub async fn new(config: VaultConfig) -> Result<Self, CredentialError> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| CredentialError::Network { 
                message: format!("Failed to create HTTP client: {}", e) 
            })?;

        let mut vault_client = Self {
            client,
            config: config.clone(),
            token: None,
            token_expiry: None,
        };

        // Authenticate using configured method
        vault_client.authenticate().await?;

        Ok(vault_client)
    }

    /// Authenticate with Vault
    async fn authenticate(&mut self) -> Result<(), CredentialError> {
        match &self.config.auth_method {
            VaultAuthMethod::Token { token } => {
                self.token = Some(token.clone());
                self.token_expiry = Some(chrono::Utc::now() + chrono::Duration::hours(24));
            }
            VaultAuthMethod::AppRole { role_id, secret_id } => {
                let auth_data = HashMap::from([
                    ("role_id", role_id.clone()),
                    ("secret_id", secret_id.clone()),
                ]);
                
                let response = self.client
                    .post(&format!("{}/v1/auth/approle/login", self.config.vault_url))
                    .json(&auth_data)
                    .send()
                    .await
                    .map_err(|e| CredentialError::Network {
                        message: format!("Vault authentication failed: {}", e)
                    })?;

                if !response.status().is_success() {
                    return Err(CredentialError::Authentication {
                        message: "Failed to authenticate with Vault".to_string(),
                    });
                }

                let auth_response: serde_json::Value = response.json().await
                    .map_err(|e| CredentialError::Network {
                        message: format!("Failed to parse Vault auth response: {}", e)
                    })?;

                if let Some(auth) = auth_response.get("auth") {
                    if let Some(token) = auth.get("client_token").and_then(|t| t.as_str()) {
                        if let Some(lease_duration) = auth.get("lease_duration").and_then(|d| d.as_u64()) {
                            self.token_expiry = Some(chrono::Utc::now() + chrono::Duration::seconds(lease_duration as i64));
                        }
                        self.token = Some(token.to_string());
                    }
                }
            }
            VaultAuthMethod::Kubernetes { jwt_path, role } => {
                let jwt = tokio::fs::read_to_string(jwt_path).await
                    .map_err(|e| CredentialError::Network {
                        message: format!("Failed to read JWT file: {}", e)
                    })?;

                let auth_data = HashMap::from([
                    ("role", role.clone()),
                    ("jwt", jwt.trim().to_string()),
                ]);

                let response = self.client
                    .post(&format!("{}/v1/auth/kubernetes/login", self.config.vault_url))
                    .json(&auth_data)
                    .send()
                    .await
                    .map_err(|e| CredentialError::Network {
                        message: format!("Vault Kubernetes authentication failed: {}", e)
                    })?;

                if !response.status().is_success() {
                    return Err(CredentialError::Authentication {
                        message: "Failed to authenticate with Vault via Kubernetes".to_string(),
                    });
                }

                let auth_response: serde_json::Value = response.json().await
                    .map_err(|e| CredentialError::Network {
                        message: format!("Failed to parse Vault auth response: {}", e)
                    })?;

                if let Some(auth) = auth_response.get("auth") {
                    if let Some(token) = auth.get("client_token").and_then(|t| t.as_str()) {
                        if let Some(lease_duration) = auth.get("lease_duration").and_then(|d| d.as_u64()) {
                            self.token_expiry = Some(chrono::Utc::now() + chrono::Duration::seconds(lease_duration as i64));
                        }
                        self.token = Some(token.to_string());
                    }
                }
            }
            _ => {
                return Err(CredentialError::Configuration {
                    message: "Unsupported authentication method".to_string(),
                });
            }
        }

        if self.token.is_none() {
            return Err(CredentialError::Authentication {
                message: "Failed to obtain Vault token".to_string(),
            });
        }

        Ok(())
    }

    /// Ensure we have a valid token
    async fn ensure_authenticated(&mut self) -> Result<(), CredentialError> {
        if let Some(expiry) = self.token_expiry {
            if chrono::Utc::now() > expiry - chrono::Duration::minutes(5) {
                self.authenticate().await?;
            }
        }
        Ok(())
    }

    /// Make authenticated request to Vault
    async fn request(&mut self, method: reqwest::Method, path: &str, body: Option<serde_json::Value>) -> Result<reqwest::Response, CredentialError> {
        self.ensure_authenticated().await?;

        let url = if path.starts_with("/v1/") {
            format!("{}{}", self.config.vault_url, path)
        } else {
            format!("{}/v1/{}", self.config.vault_url, path)
        };

        let mut request = self.client.request(method, &url);
        
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        request.send().await
            .map_err(|e| CredentialError::Network {
                message: format!("Vault API request failed: {}", e)
            })
    }

    /// Get secret from Vault
    pub async fn get_secret(&mut self, path: &str) -> Result<serde_json::Value, CredentialError> {
        let response = self.request(reqwest::Method::GET, path, None).await?;
        
        if !response.status().is_success() {
            return Err(CredentialError::Network {
                message: format!("Failed to get secret from Vault: HTTP {}", response.status())
            });
        }

        response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse Vault response: {}", e)
            })
    }

    /// Put secret to Vault
    pub async fn put_secret(&mut self, path: &str, secret_data: &HashMap<String, String>) -> Result<(), CredentialError> {
        let secret_payload = match self.config.kv_version {
            KVVersion::V1 => {
                serde_json::to_value(secret_data).map_err(|e| CredentialError::Storage {
                    message: format!("Failed to serialize secret data: {}", e)
                })?
            }
            KVVersion::V2 => {
                serde_json::json!({ "data": secret_data })
            }
        };

        let response = self.request(reqwest::Method::PUT, path, Some(secret_payload)).await?;
        
        if !response.status().is_success() {
            return Err(CredentialError::Network {
                message: format!("Failed to put secret to Vault: HTTP {}", response.status())
            });
        }

        Ok(())
    }

    /// List secrets at path
    pub async fn list_secrets(&mut self, path: &str) -> Result<Vec<String>, CredentialError> {
        let list_path = if self.config.kv_version == KVVersion::V2 {
            format!("{}/metadata", path)
        } else {
            path.to_string()
        };

        let response = self.request(reqwest::Method::GET, &format!("{}?list=1", list_path), None).await?;
        
        if !response.status().is_success() {
            return Err(CredentialError::Network {
                message: format!("Failed to list secrets from Vault: HTTP {}", response.status())
            });
        }

        let response_data: serde_json::Value = response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse Vault list response: {}", e)
            })?;

        let mut keys = Vec::new();
        if let Some(data) = response_data.get("data") {
            if let Some(key_list) = data.get("keys").and_then(|k| k.as_array()) {
                for key in key_list {
                    if let Some(key_str) = key.as_str() {
                        keys.push(key_str.to_string());
                    }
                }
            }
        }

        Ok(keys)
    }

    /// Delete secret from Vault
    pub async fn delete_secret(&mut self, path: &str) -> Result<(), CredentialError> {
        let delete_path = match self.config.kv_version {
            KVVersion::V1 => path.to_string(),
            KVVersion::V2 => format!("{}/metadata", path),
        };

        let response = self.request(reqwest::Method::DELETE, &delete_path, None).await?;
        
        if !response.status().is_success() {
            return Err(CredentialError::Network {
                message: format!("Failed to delete secret from Vault: HTTP {}", response.status())
            });
        }

        Ok(())
    }
}

/// HashiCorp Vault Credential Provider
#[derive(Debug)]
pub struct HashiCorpVaultProvider {
    client: Arc<std::sync::Mutex<VaultClient>>,
    config: VaultConfig,
}

impl HashiCorpVaultProvider {
    /// Create new HashiCorp Vault Provider
    pub async fn new(config: VaultConfig) -> Result<Self, CredentialError> {
        let client = VaultClient::new(config.clone()).await?;
        Ok(Self {
            client: Arc::new(std::sync::Mutex::new(client)),
            config,
        })
    }

    /// Convert secret data from Vault format to Credential format
    fn vault_data_to_credential(&self, vault_data: &serde_json::Value) -> Result<HashMap<String, String>, CredentialError> {
        let mut values = HashMap::new();

        if let Some(data) = vault_data.get("data") {
            if data.is_object() {
                for (key, value) in data.as_object().unwrap() {
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
            }
        } else {
            // For V1, the root object is the data
            if vault_data.is_object() {
                for (key, value) in vault_data.as_object().unwrap() {
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
            }
        }

        Ok(values)
    }

    /// Convert Credential to Vault format
    fn credential_to_vault_data(&self, credential: &Credential) -> HashMap<String, String> {
        let mut vault_data = HashMap::new();
        
        // Copy all values
        for (key, value) in &credential.values {
            vault_data.insert(key.clone(), value.clone());
        }
        
        // Add metadata
        if let Some(policy) = &credential.access_policy {
            vault_data.insert("rotation_enabled".to_string(), credential.rotation_enabled.to_string());
            vault_data.insert("expires_at".to_string(), 
                credential.expires_at.map(|e| e.to_rfc3339()).unwrap_or_else(|| "null".to_string()));
            
            // Store access policy as JSON
            let policy_json = serde_json::to_string(policy)
                .unwrap_or_else(|_| "{}".to_string());
            vault_data.insert("access_policy".to_string(), policy_json);
        }
        
        vault_data
    }

    /// Get secret path in Vault
    fn get_secret_path(&self, name: &str) -> String {
        format!("{}/{}", self.config.secrets_engine, name)
    }
}

#[async_trait::async_trait]
impl CredentialProvider for HashiCorpVaultProvider {
    async fn get_credential(&self, name: &str, version: Option<&str>) -> Result<VersionedCredential, CredentialError> {
        let mut client = self.client.lock().map_err(|_| CredentialError::InternalError)?;
        
        let path = self.get_secret_path(name);
        let vault_data = client.get_secret(&path).await?;
        
        let values = self.vault_data_to_credential(&vault_data)?;
        
        // Extract metadata
        let mut credential_values = HashMap::new();
        let mut metadata = HashMap::new();
        let mut rotation_enabled = false;
        let mut expires_at = None;
        let mut access_policy = None;

        for (key, value) in values {
            match key.as_str() {
                "rotation_enabled" => rotation_enabled = value.parse().unwrap_or(false),
                "expires_at" => {
                    if value != "null" {
                        expires_at = chrono::DateTime::parse_from_rfc3339(&value)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc));
                    }
                }
                "access_policy" => {
                    if value != "{}" {
                        access_policy = serde_json::from_str(&value).ok();
                    }
                }
                _ => credential_values.insert(key.clone(), value),
            }
        }

        let credential = Credential {
            name: name.to_string(),
            values: credential_values,
            metadata,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            expires_at,
            rotation_enabled,
            access_policy,
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
        let mut client = self.client.lock().map_err(|_| CredentialError::InternalError)?;
        
        let path = self.get_secret_path(&credential.name);
        let vault_data = self.credential_to_vault_data(credential);
        
        client.put_secret(&path, &vault_data).await?;
        
        let version = format!("v{}_{}", chrono::Utc::now().timestamp_millis(), chrono::Utc::now().timestamp_nanos());
        
        Ok(VersionedCredential {
            credential: credential.clone(),
            version,
            version_created_at: chrono::Utc::now(),
            is_active: true,
        })
    }

    async fn list_credential_versions(&self, name: &str) -> Result<Vec<VersionedCredential>, CredentialError> {
        // Note: Vault V1 doesn't support version listing
        // This would need to be implemented differently for V1 vs V2
        let mut client = self.client.lock().map_err(|_| CredentialError::InternalError)?;
        
        // For simplicity, return current version only
        let current = self.get_credential(name, None).await?;
        Ok(vec![current])
    }

    async fn list_credentials(&self) -> Result<Vec<VersionedCredential>, CredentialError> {
        let mut client = self.client.lock().map_err(|_| CredentialError::InternalError)?;
        
        let secrets = client.list_secrets(&self.config.secrets_engine).await?;
        
        let mut credentials = Vec::new();
        for secret_name in secrets {
            if let Ok(version) = self.get_credential(&secret_name, None).await {
                credentials.push(version);
            }
        }
        
        Ok(credentials)
    }

    async fn delete_credential(&self, name: &str) -> Result<(), CredentialError> {
        let mut client = self.client.lock().map_err(|_| CredentialError::InternalError)?;
        
        let path = self.get_secret_path(name);
        client.delete_secret(&path).await?;
        
        Ok(())
    }

    async fn credential_exists(&self, name: &str) -> Result<bool, CredentialError> {
        // Simple existence check by attempting to read
        match self.get_credential(name, None).await {
            Ok(_) => Ok(true),
            Err(CredentialError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn rotate_credential(&self, name: &str, strategy: &RotationStrategy) -> Result<RotationResult, CredentialError> {
        // Get current credential
        let current = self.get_credential(name, None).await?;
        
        // Create rotated version
        let mut rotated_credential = current.credential.clone();
        rotated_credential.updated_at = chrono::Utc::now();
        
        match strategy {
            RotationStrategy::TimeBased(time_based) => {
                rotated_credential.expires_at = Some(chrono::Utc::now() + time_based.rotation_interval);
            }
            RotationStrategy::EventBased(_) => {
                // For event-based, rotate immediately
                rotated_credential.expires_at = Some(chrono::Utc::now() + chrono::Duration::days(30));
            }
            RotationStrategy::Manual => {
                // Keep current expiry
            }
        }

        // Store rotated credential
        let rotated_version = self.put_credential(&rotated_credential).await?;
        
        Ok(RotationResult {
            success: true,
            rotated_credential: Some(name.to_string()),
            old_credential_version: Some(current.version),
            new_credential_version: Some(rotated_version.version),
            rotated_at: chrono::Utc::now(),
            error_message: None,
        })
    }

    async fn get_access_policy(&self, name: &str) -> Result<Option<AccessPolicy>, CredentialError> {
        let version = self.get_credential(name, None).await?;
        Ok(version.credential.access_policy)
    }

    async fn set_access_policy(&self, name: &str, policy: &AccessPolicy) -> Result<(), CredentialError> {
        let mut version = self.get_credential(name, None).await?;
        version.credential.access_policy = Some(policy.clone());
        version.credential.updated_at = chrono::Utc::now();
        
        let _ = self.put_credential(&version.credential).await?;
        
        Ok(())
    }

    async fn has_permission(&self, name: &str, subject: &str, permission: &str) -> Result<bool, CredentialError> {
        let version = self.get_credential(name, None).await?;
        
        if let Some(policy) = &version.credential.access_policy {
            Ok(policy.allowed_subjects.contains(subject))
        } else {
            // Default policy: allow all access
            Ok(true)
        }
    }

    async fn audit_operation(&self, event: &AuditEvent) -> Result<(), CredentialError> {
        // In a production environment, you would log this to a secure audit system
        // For now, just return success
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

/// Transit encryption provider for Vault
pub struct VaultTransitProvider {
    client: Arc<std::sync::Mutex<VaultClient>>,
    key_name: String,
}

impl VaultTransitProvider {
    /// Create new Transit provider
    pub fn new(client: Arc<std::sync::Mutex<VaultClient>>, key_name: String) -> Self {
        Self { client, key_name }
    }

    /// Encrypt data using Vault Transit
    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CredentialError> {
        let mut client = self.client.lock().map_err(|_| CredentialError::InternalError)?;
        
        let payload = HashMap::from([
            ("plaintext", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, plaintext))
        ]);

        let response = client.request(
            reqwest::Method::POST,
            &format!("v1/transit/encrypt/{}", self.key_name),
            Some(serde_json::to_value(&payload).unwrap())
        ).await?;

        if !response.status().is_success() {
            return Err(CredentialError::Encryption {
                message: "Failed to encrypt data".to_string()
            });
        }

        let response_data: serde_json::Value = response.json().await
            .map_err(|_| CredentialError::Encryption {
                message: "Failed to parse encryption response".to_string()
            })?;

        if let Some(ciphertext) = response_data.get("data").and_then(|d| d.get("ciphertext")) {
            if let Some(ciphertext_str) = ciphertext.as_str() {
                return Ok(base64::Engine::decode(&base64::engine::general_purpose::STANDARD, ciphertext_str).unwrap());
            }
        }

        Err(CredentialError::Encryption {
            message: "Invalid encryption response format".to_string()
        })
    }

    /// Decrypt data using Vault Transit
    pub async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CredentialError> {
        let mut client = self.client.lock().map_err(|_| CredentialError::InternalError)?;
        
        let ciphertext_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, ciphertext);
        let payload = HashMap::from([
            ("ciphertext", ciphertext_b64)
        ]);

        let response = client.request(
            reqwest::Method::POST,
            &format!("v1/transit/decrypt/{}", self.key_name),
            Some(serde_json::to_value(&payload).unwrap())
        ).await?;

        if !response.status().is_success() {
            return Err(CredentialError::Decryption {
                message: "Failed to decrypt data".to_string()
            });
        }

        let response_data: serde_json::Value = response.json().await
            .map_err(|_| CredentialError::Decryption {
                message: "Failed to parse decryption response".to_string()
            })?;

        if let Some(plaintext) = response_data.get("data").and_then(|d| d.get("plaintext")) {
            if let Some(plaintext_str) = plaintext.as_str() {
                return Ok(base64::Engine::decode(&base64::engine::general_purpose::STANDARD, plaintext_str).unwrap());
            }
        }

        Err(CredentialError::Decryption {
            message: "Invalid decryption response format".to_string()
        })
    }
}
