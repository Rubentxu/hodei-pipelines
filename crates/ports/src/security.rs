use async_trait::async_trait;
use hodei_core::security::{JwtClaims, Permission, Role, SecurityContext};
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

#[async_trait]
pub trait TokenService: Send + Sync {
    /// Generate a new token for a subject with roles and permissions
    fn generate_token(
        &self,
        subject: &str,
        roles: Vec<Role>,
        permissions: Vec<Permission>,
        tenant_id: Option<String>,
    ) -> Result<String>;

    /// Verify a token and return the claims
    fn verify_token(&self, token: &str) -> Result<JwtClaims>;

    /// Get the security context from a token
    fn get_context(&self, token: &str) -> Result<SecurityContext>;
}

#[async_trait]
pub trait SecretMasker: Send + Sync {
    /// Mask sensitive information in a text
    async fn mask_text(&self, source: &str, text: &str) -> String;
}

#[async_trait]
pub trait CertificateValidator: Send + Sync {
    /// Validate a client certificate
    async fn validate_cert(&self, cert_pem: &[u8]) -> Result<()>;
}

#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// Log a security event
    async fn log_event(
        &self,
        event_type: &str,
        details: &str,
        context: Option<&SecurityContext>,
    ) -> Result<()>;
}
