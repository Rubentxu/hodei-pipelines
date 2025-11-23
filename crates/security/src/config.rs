use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Missing configuration: {0}")]
    Missing(String),

    #[error("File not found: {0}")]
    FileNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MtlsConfig {
    pub ca_cert_path: Option<String>,
    pub server_cert_path: String,
    pub server_key_path: String,
    pub verify_client_certs: bool,
    pub allow_self_signed: bool,
    pub pinned_cert_hashes: Vec<String>,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            ca_cert_path: None,
            server_cert_path: "certs/server.crt".to_string(),
            server_key_path: "certs/server.key".to_string(),
            verify_client_certs: true,
            allow_self_signed: false,
            pinned_cert_hashes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret_key: String,
    pub token_expiry_seconds: u64,
    pub refresh_token_expiry_seconds: u64,
    pub algorithm: String,
    pub issuer: String,
    pub audience: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret_key: "default-secret".to_string(),
            token_expiry_seconds: 900,
            refresh_token_expiry_seconds: 604800,
            algorithm: "HS256".to_string(),
            issuer: "hodei-jobs".to_string(),
            audience: "hodei-workers".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingConfig {
    pub enabled: bool,
    pub custom_patterns: Vec<String>,
    pub use_builtin_patterns: bool,
    pub replacement: String,
}

impl Default for MaskingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_patterns: Vec::new(),
            use_builtin_patterns: true,
            replacement: "****".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_directory: String,
    pub retention_days: u64,
    pub enable_hash_chain: bool,
    pub events_to_audit: Vec<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_directory: "logs/audit".to_string(),
            retention_days: 2555,
            enable_hash_chain: true,
            events_to_audit: vec!["agent_connected".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub session_timeout_seconds: u64,
    pub rate_limit_rpm: u64,
    pub rate_limit_burst: u64,
    pub whitelisted_ips: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            session_timeout_seconds: 3600,
            rate_limit_rpm: 1000,
            rate_limit_burst: 100,
            whitelisted_ips: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub mtls: MtlsConfig,
    pub jwt: JwtConfig,
    pub masking: MaskingConfig,
    pub audit: AuditConfig,
    pub auth: AuthConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            mtls: MtlsConfig::default(),
            jwt: JwtConfig::default(),
            masking: MaskingConfig::default(),
            audit: AuditConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}
