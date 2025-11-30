use async_trait::async_trait;
use hodei_pipelines_core::security::{JwtClaims, Permission, Role, SecurityContext};
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

#[cfg(test)]
mod tests {
    use super::*;
    

    #[tokio::test]
    async fn test_token_service_trait_exists() {
        // This test verifies the trait exists and compiles
        // In a real implementation, this would use an actual TokenService
        let _service: Option<Box<dyn TokenService + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[tokio::test]
    async fn test_secret_masker_trait_exists() {
        // This test verifies the trait exists and compiles
        let _masker: Option<Box<dyn SecretMasker + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[tokio::test]
    async fn test_certificate_validator_trait_exists() {
        // This test verifies the trait exists and compiles
        let _validator: Option<Box<dyn CertificateValidator + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[tokio::test]
    async fn test_audit_logger_trait_exists() {
        // This test verifies the trait exists and compiles
        let _logger: Option<Box<dyn AuditLogger + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[test]
    fn test_security_error_display() {
        let error = SecurityError::Jwt("Invalid token".to_string());
        assert!(error.to_string().contains("JWT error"));
        assert!(error.to_string().contains("Invalid token"));
    }

    #[test]
    fn test_security_error_variants() {
        let cert_error = SecurityError::CertificateValidation("Invalid cert".to_string());
        let jwt_error = SecurityError::Jwt("Invalid token".to_string());
        let audit_error = SecurityError::Audit("Audit failed".to_string());
        let auth_error = SecurityError::Authorization("Not authorized".to_string());

        assert!(
            cert_error
                .to_string()
                .contains("Certificate validation failed")
        );
        assert!(jwt_error.to_string().contains("JWT error"));
        assert!(audit_error.to_string().contains("Audit error"));
        assert!(auth_error.to_string().contains("Authorization failed"));
    }

    #[test]
    fn test_security_error_result_type() {
        // Test that Result type alias works correctly
        let _result: Result<String> = Ok("test".to_string());
        let _error_result: Result<String> = Err(SecurityError::Jwt("test error".to_string()));
    }
}
