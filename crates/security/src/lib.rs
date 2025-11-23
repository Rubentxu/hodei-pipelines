//! Hodei Security Module
//!
//! Enterprise-grade security implementation for Hodei Jobs

pub mod audit;
pub mod auth;
pub mod config;
pub mod jwt;
pub mod mtls;
pub mod secret_masking;

pub use audit::{AuditEvent, AuditLogger};
pub use auth::{AuthorizationService, Permission, Role, SecurityContext};
pub use config::{ConfigError, SecurityConfig};
pub use jwt::{JwtClaims, JwtToken, TokenManager};
pub use mtls::{CertificateValidator, TlsConfig};
pub use secret_masking::{MaskPattern, PatternCompiler, SecretMasker};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Certificate validation failed: {0}")]
    CertificateValidation(String),

    #[error("JWT error: {0}")]
    Jwt(String),

    #[error("Audit error: {0}")]
    Audit(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Other security error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SecurityError>;

pub struct SecurityModule {
    pub mtls: mtls::CertificateValidator,
    pub jwt: jwt::TokenManager,
    pub masker: secret_masking::SecretMasker,
    pub audit: audit::AuditLogger,
    pub auth: auth::AuthorizationService,
}

impl SecurityModule {
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        let mtls = mtls::CertificateValidator::new(config.mtls.clone()).await?;
        let jwt = jwt::TokenManager::new(config.jwt.clone())?;
        let masker = secret_masking::SecretMasker::new(config.masking.clone());
        let audit = audit::AuditLogger::new(config.audit.clone()).await?;
        let auth = auth::AuthorizationService::new(config.auth.clone())?;

        Ok(Self {
            mtls,
            jwt,
            masker,
            audit,
            auth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_module_creation() {
        let config = SecurityConfig::default();
        let result = SecurityModule::new(config).await;
        assert!(result.is_ok());
    }
}
