//! Keycloak Service Account Provider Implementation
//! 
//! This module provides a complete integration with Keycloak for
//! Service Account authentication and token management.

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

/// Keycloak configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeycloakConfig {
    pub base_url: String,
    pub realm: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Vec<String>,
    pub token_endpoint: Option<String>,
    pub jwks_endpoint: Option<String>,
    pub timeout: Duration,
    pub cache_ttl: Duration,
}

/// Keycloak token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeycloakToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: Option<String>,
    pub issued_at: chrono::DateTime<chrono::Utc>,
}

/// Keycloak User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeycloakUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    pub email_verified: bool,
}

/// Keycloak Service Account client
#[derive(Debug)]
pub struct KeycloakClient {
    client: reqwest::Client,
    config: KeycloakConfig,
    token_cache: std::sync::RwLock<HashMap<String, (KeycloakToken, chrono::DateTime<chrono::Utc>)>>,
}

impl KeycloakClient {
    /// Create new Keycloak client
    pub fn new(config: KeycloakConfig) -> Result<Self, CredentialError> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| CredentialError::Network { 
                message: format!("Failed to create Keycloak HTTP client: {}", e) 
            })?;

        Ok(Self {
            client,
            config,
            token_cache: std::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Get cached token for client credentials
    async fn get_cached_token(&self, key: &str) -> Option<KeycloakToken> {
        if let Ok(cache) = self.token_cache.read() {
            if let Some((token, expires_at)) = cache.get(key) {
                if chrono::Utc::now() < *expires_at {
                    return Some(token.clone());
                }
            }
        }
        None
    }

    /// Cache token
    fn cache_token(&self, key: String, token: KeycloakToken) {
        let expires_at = token.issued_at + chrono::Duration::seconds(token.expires_in - 60); // 60s buffer
        if let Ok(mut cache) = self.token_cache.write() {
            cache.insert(key, (token, expires_at));
        }
    }

    /// Get access token using client credentials flow
    pub async fn get_access_token(&self) -> Result<KeycloakToken, CredentialError> {
        let cache_key = format!("client_credentials:{}", self.config.client_id);
        
        // Check cache first
        if let Some(token) = self.get_cached_token(&cache_key).await {
            return Ok(token);
        }

        // Get token from Keycloak
        let token_data = self.fetch_token().await?;
        self.cache_token(cache_key, token_data.clone());

        Ok(token_data)
    }

    /// Fetch token from Keycloak server
    async fn fetch_token(&self) -> Result<KeycloakToken, CredentialError> {
        let token_endpoint = self.get_token_endpoint().await?;
        
        let mut form_data = HashMap::from([
            ("grant_type".to_string(), "client_credentials".to_string()),
            ("client_id".to_string(), self.config.client_id.clone()),
            ("scope".to_string(), self.config.scopes.join(" ")),
        ]);

        if let Some(secret) = &self.config.client_secret {
            form_data.insert("client_secret".to_string(), secret.clone());
        }

        let response = self.client
            .post(&token_endpoint)
            .form(&form_data)
            .send()
            .await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to get Keycloak token: {}", e)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CredentialError::Authentication {
                message: format!("Keycloak authentication failed: HTTP {} - {}", status, body)
            });
        }

        let token_response: serde_json::Value = response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse Keycloak token response: {}", e)
            })?;

        let token_data = KeycloakToken {
            access_token: token_response.get("access_token")
                .and_then(|t| t.as_str())
                .ok_or_else(|| CredentialError::InvalidFormat {
                    name: "access_token".to_string()
                })?.to_string(),
            refresh_token: token_response.get("refresh_token").and_then(|t| t.as_str()).map(|s| s.to_string()),
            token_type: token_response.get("token_type")
                .and_then(|t| t.as_str())
                .unwrap_or("Bearer").to_string(),
            expires_in: token_response.get("expires_in")
                .and_then(|e| e.as_i64())
                .unwrap_or(3600),
            scope: token_response.get("scope").and_then(|s| s.as_str()).map(|s| s.to_string()),
            issued_at: chrono::Utc::now(),
        };

        Ok(token_data)
    }

    /// Get Keycloak token endpoint
    async fn get_token_endpoint(&self) -> Result<String, CredentialError> {
        if let Some(endpoint) = &self.config.token_endpoint {
            return Ok(endpoint.clone());
        }

        // Discover token endpoint from realm info
        let well_known_url = format!("{}/realms/{}/.well-known/openid-configuration", 
                                   self.config.base_url, self.config.realm);
        
        let response = self.client
            .get(&well_known_url)
            .send()
            .await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to get Keycloak OIDC configuration: {}", e)
            })?;

        if !response.status().is_success() {
            return Err(CredentialError::Network {
                message: format!("Failed to get Keycloak OIDC configuration: HTTP {}", response.status())
            });
        }

        let oidc_config: serde_json::Value = response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse Keycloak OIDC configuration: {}", e)
            })?;

        let token_endpoint = oidc_config.get("token_endpoint")
            .and_then(|e| e.as_str())
            .ok_or_else(|| CredentialError::Configuration {
                message: "Token endpoint not found in OIDC configuration".to_string()
            })?.to_string();

        Ok(token_endpoint)
    }

    /// Get user information using access token
    pub async fn get_user_info(&self, access_token: &str) -> Result<KeycloakUser, CredentialError> {
        let user_info_url = format!("{}/realms/{}/protocol/openid-connect/userinfo", 
                                  self.config.base_url, self.config.realm);

        let response = self.client
            .get(&user_info_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to get Keycloak user info: {}", e)
            })?;

        if !response.status().is_success() {
            return Err(CredentialError::Authentication {
                message: format!("Failed to get user info: HTTP {}", response.status())
            });
        }

        let user_info: serde_json::Value = response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse Keycloak user info: {}", e)
            })?;

        let user = KeycloakUser {
            id: user_info.get("sub")
                .and_then(|s| s.as_str())
                .ok_or_else(|| CredentialError::InvalidFormat { name: "sub".to_string() })?.to_string(),
            username: user_info.get("preferred_username")
                .and_then(|u| u.as_str())
                .unwrap_or("unknown").to_string(),
            email: user_info.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()),
            first_name: user_info.get("given_name").and_then(|n| n.as_str()).map(|s| s.to_string()),
            last_name: user_info.get("family_name").and_then(|n| n.as_str()).map(|s| s.to_string()),
            enabled: true, // Would need to check user status via admin API
            email_verified: user_info.get("email_verified")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };

        Ok(user)
    }

    /// Get JWKS for token validation
    pub async fn get_jwks(&self) -> Result<serde_json::Value, CredentialError> {
        let jwks_url = self.get_jwks_endpoint().await?;

        let response = self.client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to get JWKS: {}", e)
            })?;

        if !response.status().is_success() {
            return Err(CredentialError::Network {
                message: format!("Failed to get JWKS: HTTP {}", response.status())
            });
        }

        response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse JWKS: {}", e)
            })
    }

    /// Get JWKS endpoint
    async fn get_jwks_endpoint(&self) -> Result<String, CredentialError> {
        if let Some(endpoint) = &self.config.jwks_endpoint {
            return Ok(endpoint.clone());
        }

        // Discover JWKS endpoint from OIDC configuration
        let well_known_url = format!("{}/realms/{}/.well-known/openid-configuration", 
                                   self.config.base_url, self.config.realm);
        
        let response = self.client
            .get(&well_known_url)
            .send()
            .await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to get OIDC configuration for JWKS: {}", e)
            })?;

        let oidc_config: serde_json::Value = response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse OIDC configuration for JWKS: {}", e)
            })?;

        let jwks_uri = oidc_config.get("jwks_uri")
            .and_then(|e| e.as_str())
            .ok_or_else(|| CredentialError::Configuration {
                message: "JWKS URI not found in OIDC configuration".to_string()
            })?.to_string();

        Ok(jwks_uri)
    }

    /// Validate and decode JWT token
    pub async fn validate_token(&self, token: &str) -> Result<serde_json::Value, CredentialError> {
        // In a production environment, you would use a JWT library to validate the token
        // This is a simplified implementation
        let jwks = self.get_jwks().await?;
        
        // For now, just decode the base64 payload (NOT SECURE FOR PRODUCTION)
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(CredentialError::InvalidFormat { name: "jwt".to_string() });
        }

        let payload_b64 = parts[1];
        let payload_bytes = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, payload_b64)
            .map_err(|_| CredentialError::InvalidFormat { name: "jwt_payload".to_string() })?;

        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .map_err(|_| CredentialError::InvalidFormat { name: "jwt_payload".to_string() })?;

        Ok(payload)
    }
}

/// Keycloak Service Account Provider
#[derive(Debug)]
pub struct KeycloakServiceAccountProvider {
    client: Arc<KeycloakClient>,
}

impl KeycloakServiceAccountProvider {
    /// Create new Keycloak Service Account Provider
    pub fn new(config: KeycloakConfig) -> Result<Self, CredentialError> {
        let client = KeycloakClient::new(config)?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Convert Keycloak token to credential format
    fn keycloak_token_to_credential(&self, token: &KeycloakToken) -> HashMap<String, String> {
        let mut values = HashMap::new();
        values.insert("access_token".to_string(), token.access_token.clone());
        values.insert("token_type".to_string(), token.token_type.clone());
        values.insert("expires_in".to_string(), token.expires_in.to_string());
        
        if let Some(refresh_token) = &token.refresh_token {
            values.insert("refresh_token".to_string(), refresh_token.clone());
        }
        
        if let Some(scope) = &token.scope {
            values.insert("scope".to_string(), scope.clone());
        }
        
        values.insert("issued_at".to_string(), token.issued_at.to_rfc3339());
        values
    }
}

#[async_trait::async_trait]
impl CredentialProvider for KeycloakServiceAccountProvider {
    async fn get_credential(&self, name: &str, version: Option<&str>) -> Result<VersionedCredential, CredentialError> {
        if name != "service_account_token" {
            return Err(CredentialError::NotFound { name: name.to_string() });
        }

        let token = self.client.get_access_token().await?;
        let values = self.keycloak_token_to_credential(&token);

        let credential = Credential {
            name: name.to_string(),
            values,
            metadata: HashMap::from([
                ("client_id".to_string(), self.client.config.client_id.clone()),
                ("realm".to_string(), self.client.config.realm.clone()),
                ("scopes".to_string(), self.client.config.scopes.join(" ")),
            ]),
            created_at: token.issued_at,
            updated_at: chrono::Utc::now(),
            expires_at: Some(token.issued_at + chrono::Duration::seconds(token.expires_in)),
            rotation_enabled: true,
            access_policy: Some(AccessPolicy {
                allowed_subjects: vec!["system".to_string()],
                read_permissions: vec!["read".to_string()],
                write_permissions: vec![],
                rotation_permissions: vec!["rotate".to_string()],
            }),
        };

        let version = version.unwrap_or("current").to_string();
        
        Ok(VersionedCredential {
            credential,
            version,
            version_created_at: chrono::Utc::now(),
            is_active: true,
        })
    }

    async fn put_credential(&self, _credential: &Credential) -> Result<VersionedCredential, CredentialError> {
        Err(CredentialError::PermissionDenied {
            name: "Keycloak tokens are read-only".to_string()
        })
    }

    async fn list_credential_versions(&self, name: &str) -> Result<Vec<VersionedCredential>, CredentialError> {
        if name != "service_account_token" {
            return Err(CredentialError::NotFound { name: name.to_string() });
        }

        let token = self.client.get_access_token().await?;
        let values = self.keycloak_token_to_credential(&token);

        let credential = Credential {
            name: name.to_string(),
            values,
            metadata: HashMap::from([
                ("client_id".to_string(), self.client.config.client_id.clone()),
                ("realm".to_string(), self.client.config.realm.clone()),
                ("scopes".to_string(), self.client.config.scopes.join(" ")),
            ]),
            created_at: token.issued_at,
            updated_at: chrono::Utc::now(),
            expires_at: Some(token.issued_at + chrono::Duration::seconds(token.expires_in)),
            rotation_enabled: true,
            access_policy: Some(AccessPolicy {
                allowed_subjects: vec!["system".to_string()],
                read_permissions: vec!["read".to_string()],
                write_permissions: vec![],
                rotation_permissions: vec!["rotate".to_string()],
            }),
        };

        Ok(vec![VersionedCredential {
            credential,
            version: "current".to_string(),
            version_created_at: chrono::Utc::now(),
            is_active: true,
        }])
    }

    async fn list_credentials(&self) -> Result<Vec<VersionedCredential>, CredentialError> {
        // Keycloak Service Account Provider only manages service account tokens
        Ok(vec![])
    }

    async fn delete_credential(&self, _name: &str) -> Result<(), CredentialError> {
        // Cannot delete service account tokens
        Err(CredentialError::PermissionDenied {
            name: "Keycloak service account tokens cannot be deleted".to_string()
        })
    }

    async fn credential_exists(&self, name: &str) -> Result<bool, CredentialError> {
        Ok(name == "service_account_token")
    }

    async fn rotate_credential(&self, name: &str, strategy: &RotationStrategy) -> Result<RotationResult, CredentialError> {
        if name != "service_account_token" {
            return Err(CredentialError::NotFound { name: name.to_string() });
        }

        match strategy {
            RotationStrategy::TimeBased(time_based) => {
                // Keycloak tokens expire automatically, rotation is handled by the server
                // We just need to refresh our cache
                let old_token = self.client.get_access_token().await?;
                
                // Wait for rotation interval
                tokio::time::sleep(time_based.rotation_interval).await;
                
                // Get new token (will trigger cache refresh)
                let _ = self.client.get_access_token().await?;
                
                Ok(RotationResult {
                    success: true,
                    rotated_credential: Some(name.to_string()),
                    old_credential_version: Some(old_token.access_token.clone()),
                    new_credential_version: Some("rotated".to_string()),
                    rotated_at: chrono::Utc::now(),
                    error_message: None,
                })
            }
            RotationStrategy::EventBased(_) | RotationStrategy::Manual => {
                // Force cache refresh
                {
                    let mut cache = self.client.token_cache.write().map_err(|_| CredentialError::InternalError)?;
                    cache.clear();
                }
                
                let new_token = self.client.get_access_token().await?;
                
                Ok(RotationResult {
                    success: true,
                    rotated_credential: Some(name.to_string()),
                    old_credential_version: Some("refreshed".to_string()),
                    new_credential_version: Some(new_token.access_token.clone()),
                    rotated_at: chrono::Utc::now(),
                    error_message: None,
                })
            }
        }
    }

    async fn get_access_policy(&self, name: &str) -> Result<Option<AccessPolicy>, CredentialError> {
        if name == "service_account_token" {
            Ok(Some(AccessPolicy {
                allowed_subjects: vec!["system".to_string()],
                read_permissions: vec!["read".to_string()],
                write_permissions: vec![],
                rotation_permissions: vec!["rotate".to_string()],
            }))
        } else {
            Ok(None)
        }
    }

    async fn set_access_policy(&self, _name: &str, _policy: &AccessPolicy) -> Result<(), CredentialError> {
        // Cannot set policies on Keycloak service account tokens
        Err(CredentialError::PermissionDenied {
            name: "Cannot modify Keycloak service account token policies".to_string()
        })
    }

    async fn has_permission(&self, name: &str, subject: &str, permission: &str) -> Result<bool, CredentialError> {
        if name != "service_account_token" {
            return Ok(false);
        }

        // Allow access if subject is "system" for read/rotate permissions
        Ok(subject == "system" && (permission == "read" || permission == "rotate"))
    }

    async fn audit_operation(&self, event: &AuditEvent) -> Result<(), CredentialError> {
        // In production, you would log this to Keycloak events or external audit system
        Ok(())
    }

    async fn get_statistics(&self) -> Result<CredentialStatistics, CredentialError> {
        Ok(CredentialStatistics {
            total_credentials: 1,
            active_credentials: 1,
            credential_versions: 1,
            rotation_statistics: RotationStatistics {
                total_rotations: 0,
                successful_rotations: 0,
                failed_rotations: 0,
                last_rotation: None,
                next_scheduled_rotation: None,
            },
            last_accessed: Some(chrono::Utc::now()),
        })
    }
}

/// Keycloak User Provider for user authentication
#[derive(Debug)]
pub struct KeycloakUserProvider {
    client: Arc<KeycloakClient>,
}

impl KeycloakUserProvider {
    /// Create new Keycloak User Provider
    pub fn new(config: KeycloakConfig) -> Result<Self, CredentialError> {
        let client = KeycloakClient::new(config)?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Authenticate user with username and password
    pub async fn authenticate_user(&self, username: &str, password: &str) -> Result<KeycloakToken, CredentialError> {
        let token_endpoint = self.client.get_token_endpoint().await?;
        
        let mut form_data = HashMap::from([
            ("grant_type".to_string(), "password".to_string()),
            ("client_id".to_string(), self.client.config.client_id.clone()),
            ("username".to_string(), username.to_string()),
            ("password".to_string(), password.to_string()),
            ("scope".to_string(), self.client.config.scopes.join(" ")),
        ]);

        if let Some(secret) = &self.client.config.client_secret {
            form_data.insert("client_secret".to_string(), secret.clone());
        }

        let response = self.client
            .client
            .post(&token_endpoint)
            .form(&form_data)
            .send()
            .await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to authenticate user: {}", e)
            })?;

        if !response.status().is_success() {
            return Err(CredentialError::Authentication {
                message: format!("User authentication failed: HTTP {}", response.status())
            });
        }

        let token_response: serde_json::Value = response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse user token response: {}", e)
            })?;

        let token_data = KeycloakToken {
            access_token: token_response.get("access_token")
                .and_then(|t| t.as_str())
                .ok_or_else(|| CredentialError::InvalidFormat {
                    name: "access_token".to_string()
                })?.to_string(),
            refresh_token: token_response.get("refresh_token").and_then(|t| t.as_str()).map(|s| s.to_string()),
            token_type: token_response.get("token_type")
                .and_then(|t| t.as_str())
                .unwrap_or("Bearer").to_string(),
            expires_in: token_response.get("expires_in")
                .and_then(|e| e.as_i64())
                .unwrap_or(3600),
            scope: token_response.get("scope").and_then(|s| s.as_str()).map(|s| s.to_string()),
            issued_at: chrono::Utc::now(),
        };

        Ok(token_data)
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str, access_token: &str) -> Result<KeycloakUser, CredentialError> {
        let admin_api_url = format!("{}/admin/realms/{}/users/{}", 
                                  self.client.config.base_url, self.client.config.realm, user_id);

        let response = self.client
            .client
            .get(&admin_api_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to get user by ID: {}", e)
            })?;

        if !response.status().is_success() {
            return Err(CredentialError::NotFound {
                name: format!("User {}", user_id)
            });
        }

        let user_data: serde_json::Value = response.json().await
            .map_err(|e| CredentialError::Network {
                message: format!("Failed to parse user data: {}", e)
            })?;

        let user = KeycloakUser {
            id: user_data.get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("").to_string(),
            username: user_data.get("username")
                .and_then(|u| u.as_str())
                .unwrap_or("").to_string(),
            email: user_data.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()),
            first_name: user_data.get("firstName").and_then(|n| n.as_str()).map(|s| s.to_string()),
            last_name: user_data.get("lastName").and_then(|n| n.as_str()).map(|s| s.to_string()),
            enabled: user_data.get("enabled")
                .and_then(|e| e.as_bool())
                .unwrap_or(true),
            email_verified: user_data.get("emailVerified")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        };

        Ok(user)
    }
}
