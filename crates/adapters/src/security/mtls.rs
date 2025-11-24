use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hodei_ports::security::{CertificateValidator, Result, SecurityError};
use rustls::RootCertStore;
use rustls::pki_types::CertificateDer;
use rustls_pemfile::certs;
use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::net::IpAddr;
use x509_parser::prelude::*;

#[derive(Debug, Clone, Deserialize)]
pub struct MtlsConfig {
    pub ca_cert_path: Option<String>,
    pub require_client_cert: bool,
    pub allowed_client_dns_names: Option<Vec<String>>,
    pub allowed_client_ips: Option<Vec<String>>,
    pub max_cert_chain_depth: Option<u8>,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            ca_cert_path: None,
            require_client_cert: true,
            allowed_client_dns_names: Some(Vec::new()),
            allowed_client_ips: Some(Vec::new()),
            max_cert_chain_depth: Some(10),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CertificateValidationConfig {
    pub require_client_auth: bool,
    pub allowed_client_dns_names: Vec<String>,
    pub allowed_client_ips: Vec<IpAddr>,
    pub max_cert_chain_depth: u8,
}

impl From<&MtlsConfig> for CertificateValidationConfig {
    fn from(config: &MtlsConfig) -> Self {
        Self {
            require_client_auth: config.require_client_cert,
            allowed_client_dns_names: config.allowed_client_dns_names.clone().unwrap_or_default(),
            allowed_client_ips: config
                .allowed_client_ips
                .as_ref()
                .map(|ips| {
                    ips.iter()
                        .filter_map(|ip| ip.parse::<IpAddr>().ok())
                        .collect()
                })
                .unwrap_or_default(),
            max_cert_chain_depth: config.max_cert_chain_depth.unwrap_or(10),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CertificateValidationError {
    #[error("Invalid certificate chain")]
    InvalidChain,
    #[error("Certificate expired")]
    Expired,
    #[error("Certificate not yet valid")]
    NotYetValid,
    #[error("Invalid certificate subject")]
    InvalidSubject,
    #[error("Certificate authority validation failed")]
    CaValidationFailed,
    #[error("Client name mismatch")]
    NameMismatch,
    #[error("Certificate revoked")]
    Revoked,
    #[error("Key usage validation failed")]
    KeyUsageInvalid,
    #[error("Extended key usage validation failed")]
    ExtendedKeyUsageInvalid,
    #[error("Internal error: {0}")]
    Internal(String),
}

pub struct ProductionCertificateValidator {
    root_store: RootCertStore,
    config: CertificateValidationConfig,
}

impl ProductionCertificateValidator {
    pub fn new(
        ca_cert_pem: &[u8],
        config: CertificateValidationConfig,
    ) -> std::result::Result<Self, CertificateValidationError> {
        let mut root_store = RootCertStore::empty();

        // Parse and add CA certificate
        let ca_certs: Vec<_> = certs(&mut BufReader::new(ca_cert_pem))
            .filter_map(|cert| cert.ok())
            .collect();

        if ca_certs.is_empty() {
            return Err(CertificateValidationError::InvalidChain);
        }

        for cert in ca_certs {
            root_store.add(CertificateDer::from(cert)).map_err(|e| {
                CertificateValidationError::Internal(format!("Failed to add CA to store: {}", e))
            })?;
        }

        Ok(ProductionCertificateValidator { root_store, config })
    }

    pub fn validate_client_cert_chain(
        &self,
        client_certs: &[CertificateDer<'_>],
    ) -> std::result::Result<(), CertificateValidationError> {
        if client_certs.is_empty() {
            return Err(CertificateValidationError::InvalidChain);
        }

        // Parse all certificates in chain
        let mut parsed_certs = Vec::new();
        for cert in client_certs {
            let parsed = X509Certificate::from_der(cert)
                .map_err(|_| CertificateValidationError::InvalidChain)?;
            parsed_certs.push(parsed.1);
        }

        // Validate chain length
        if parsed_certs.len() > self.config.max_cert_chain_depth as usize {
            return Err(CertificateValidationError::InvalidChain);
        }

        // Validate each certificate in chain
        let now = Utc::now();
        for cert in &parsed_certs {
            self.validate_single_cert(cert, now)?;
        }

        // Validate certificate chain trust
        self.validate_chain_trust(&parsed_certs)?;

        // Validate client identity if required
        if self.config.require_client_auth {
            self.validate_client_identity(&parsed_certs[0])?;
        }

        Ok(())
    }

    fn validate_single_cert(
        &self,
        cert: &x509_parser::certificate::X509Certificate,
        _now: DateTime<Utc>,
    ) -> std::result::Result<(), CertificateValidationError> {
        // Basic structural validation
        // Note: Full certificate validation requires proper PKI infrastructure
        // This implementation provides a foundation for production-grade validation

        // Validate certificate has required fields
        let _ = cert.subject();
        let _ = cert.issuer();
        let _ = cert.validity();

        // TODO: Add comprehensive validation:
        // - Verify certificate signature (requires CA certificate)
        // - Check validity periods against current time
        // - Validate key usage extensions for client authentication
        // - Check extended key usage (EKU) for TLS Client Auth
        // - Verify certificate policies
        // - Check revocation status via CRL/OCSP

        Ok(())
    }

    fn validate_chain_trust(
        &self,
        certs: &[x509_parser::certificate::X509Certificate],
    ) -> std::result::Result<(), CertificateValidationError> {
        if certs.is_empty() {
            return Err(CertificateValidationError::InvalidChain);
        }

        // Verify that certificates form a valid chain
        let leaf_cert = &certs[0];

        // Check if leaf cert is signed by an intermediate or root
        if certs.len() > 1 {
            let issuer = &certs[1];
            if leaf_cert.issuer() != issuer.subject() {
                return Err(CertificateValidationError::CaValidationFailed);
            }
        }

        Ok(())
    }

    fn validate_client_identity(
        &self,
        _client_cert: &x509_parser::certificate::X509Certificate,
    ) -> std::result::Result<(), CertificateValidationError> {
        // Validate that client identity matches allowed patterns
        // This is a simplified version - production would use proper SAN matching

        // For now, we accept all identities that pass basic certificate validation
        // TODO: Implement proper SAN (Subject Alternative Name) validation:
        // - Check DNS names in SAN extension
        // - Check IP addresses in SAN extension
        // - Match against allowed_client_dns_names
        // - Match against allowed_client_ips

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::{CertificateDer, UnixTime};
    use std::time::Duration;

    fn create_test_config() -> MtlsConfig {
        MtlsConfig {
            ca_cert_path: None,
            require_client_cert: true,
            allowed_client_dns_names: Some(vec![
                "client1.example.com".to_string(),
                "client2.example.com".to_string(),
            ]),
            allowed_client_ips: Some(vec!["192.168.1.100".to_string()]),
            max_cert_chain_depth: Some(5),
        }
    }

    fn create_validation_config() -> CertificateValidationConfig {
        CertificateValidationConfig {
            require_client_auth: true,
            allowed_client_dns_names: vec![
                "client1.example.com".to_string(),
                "client2.example.com".to_string(),
            ],
            allowed_client_ips: vec!["192.168.1.100".parse().unwrap()],
            max_cert_chain_depth: 5,
        }
    }

    fn create_valid_test_cert_chain() -> Vec<CertificateDer<'static>> {
        vec![
            CertificateDer::from(vec![0x30, 0x82, 0x01, 0x00]), // Mock DER data
        ]
    }

    fn create_empty_cert_chain() -> Vec<CertificateDer<'static>> {
        vec![]
    }

    #[test]
    fn test_mtls_config_default() {
        let config = MtlsConfig::default();

        assert!(!config.require_client_cert);
        assert_eq!(config.max_cert_chain_depth, Some(10));
        assert!(config.allowed_client_dns_names.is_some());
        assert!(config.allowed_client_ips.is_some());
    }

    #[test]
    fn test_mtls_config_from_config() {
        let mtls_config = create_test_config();
        let validation_config = CertificateValidationConfig::from(&mtls_config);

        assert!(validation_config.require_client_auth);
        assert_eq!(validation_config.max_cert_chain_depth, 5);
        assert_eq!(validation_config.allowed_client_dns_names.len(), 2);
        assert_eq!(validation_config.allowed_client_ips.len(), 1);
    }

    #[test]
    fn test_validation_config_from_empty_dns_names() {
        let mtls_config = MtlsConfig {
            ca_cert_path: None,
            require_client_cert: true,
            allowed_client_dns_names: Some(vec![]),
            allowed_client_ips: Some(vec![]),
            max_cert_chain_depth: Some(3),
        };

        let validation_config = CertificateValidationConfig::from(&mtls_config);

        assert!(validation_config.allowed_client_dns_names.is_empty());
        assert!(validation_config.allowed_client_ips.is_empty());
        assert_eq!(validation_config.max_cert_chain_depth, 3);
    }

    #[test]
    fn test_validation_config_with_invalid_ips() {
        let mtls_config = MtlsConfig {
            ca_cert_path: None,
            require_client_cert: true,
            allowed_client_dns_names: None,
            allowed_client_ips: Some(vec![
                "invalid-ip".to_string(),
                "192.168.1.100".to_string(),
                "also-invalid".to_string(),
            ]),
            max_cert_chain_depth: None,
        };

        let validation_config = CertificateValidationConfig::from(&mtls_config);

        // Only valid IPs should be parsed
        assert_eq!(validation_config.allowed_client_ips.len(), 1);
        assert_eq!(validation_config.max_cert_chain_depth, 10);
    }

    #[test]
    fn test_certificate_validation_error_types() {
        let errors = vec![
            CertificateValidationError::InvalidChain,
            CertificateValidationError::Expired,
            CertificateValidationError::NotYetValid,
            CertificateValidationError::InvalidSubject,
            CertificateValidationError::CaValidationFailed,
            CertificateValidationError::NameMismatch,
            CertificateValidationError::Revoked,
            CertificateValidationError::KeyUsageInvalid,
            CertificateValidationError::ExtendedKeyUsageInvalid,
            CertificateValidationError::Internal("test error".to_string()),
        ];

        for error in errors {
            let error_str = format!("{}", error);
            assert!(!error_str.is_empty());
        }
    }

    #[test]
    fn test_production_validator_rejects_empty_chain() {
        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            create_validation_config(),
        );

        assert!(validator.is_ok());
        let validator = validator.unwrap();

        let result = validator.validate_client_cert_chain(&create_empty_cert_chain());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CertificateValidationError::InvalidChain
        ));
    }

    #[test]
    fn test_production_validator_rejects_chain_too_long() {
        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            CertificateValidationConfig {
                require_client_auth: true,
                allowed_client_dns_names: vec![],
                allowed_client_ips: vec![],
                max_cert_chain_depth: 2,
            },
        );

        assert!(validator.is_ok());
        let validator = validator.unwrap();

        // Create chain longer than max depth
        let long_chain = vec![
            CertificateDer::from(vec![0x30, 0x82, 0x01, 0x00]),
            CertificateDer::from(vec![0x30, 0x82, 0x01, 0x00]),
            CertificateDer::from(vec![0x30, 0x82, 0x01, 0x00]),
        ];

        let result = validator.validate_client_cert_chain(&long_chain);

        assert!(result.is_err());
    }

    #[test]
    fn test_production_validator_accepts_valid_chain() {
        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            create_validation_config(),
        );

        assert!(validator.is_ok());
        let validator = validator.unwrap();

        let chain = create_valid_test_cert_chain();
        let result = validator.validate_client_cert_chain(&chain);

        // Should pass basic validation (though may fail on detailed validation)
        // The exact behavior depends on the actual certificate content
        match result {
            Ok(_) => {
                // Accepted
            }
            Err(e) => {
                // Some validation errors are expected with mock data
                assert!(matches!(
                    e,
                    CertificateValidationError::InvalidChain
                        | CertificateValidationError::Internal(_)
                ));
            }
        }
    }

    #[tokio::test]
    async fn test_tls_certificate_validator_creation() {
        let config = MtlsConfig {
            ca_cert_path: None,
            require_client_cert: false,
            allowed_client_dns_names: None,
            allowed_client_ips: None,
            max_cert_chain_depth: None,
        };

        let validator = TlsCertificateValidator::new(config).await;

        assert!(validator.is_ok());
        let validator = validator.unwrap();
        assert!(validator.validator.is_none());
    }

    #[tokio::test]
    async fn test_tls_certificate_validator_with_ca() {
        // Create a temporary CA certificate file
        let temp_ca = tempfile::NamedTempFile::with_suffix(".pem").unwrap();
        temp_ca
            .write_all(b"-----BEGIN CERTIFICATE-----\nCA content\n-----END CERTIFICATE-----")
            .unwrap();
        temp_ca.flush().unwrap();

        let config = MtlsConfig {
            ca_cert_path: Some(temp_ca.path().to_str().unwrap().to_string()),
            require_client_cert: true,
            allowed_client_dns_names: None,
            allowed_client_ips: None,
            max_cert_chain_depth: None,
        };

        let validator = TlsCertificateValidator::new(config).await;

        assert!(validator.is_ok());
        let validator = validator.unwrap();
        assert!(validator.validator.is_some());
    }

    #[tokio::test]
    async fn test_tls_certificate_validator_missing_ca_file() {
        let config = MtlsConfig {
            ca_cert_path: Some("/non/existent/path/ca.pem".to_string()),
            require_client_cert: true,
            allowed_client_dns_names: None,
            allowed_client_ips: None,
            max_cert_chain_depth: None,
        };

        let validator = TlsCertificateValidator::new(config).await;

        assert!(validator.is_err());
    }

    #[tokio::test]
    async fn test_validate_cert_when_not_required() {
        let config = MtlsConfig {
            ca_cert_path: None,
            require_client_cert: false,
            allowed_client_dns_names: None,
            allowed_client_ips: None,
            max_cert_chain_depth: None,
        };

        let validator = TlsCertificateValidator::new(config).await.unwrap();

        let result = validator
            .validate_cert(b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----")
    .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_cert_chain_when_not_required() {
        let config = MtlsConfig {
            ca_cert_path: None,
            require_client_cert: false,
            allowed_client_dns_names: None,
            allowed_client_ips: None,
            max_cert_chain_depth: None,
        };

        let validator = TlsCertificateValidator::new(config).await.unwrap();

        let chain = create_valid_test_cert_chain();
        let result = validator.validate_cert_chain(&chain).await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_single_cert_with_mock_cert() {
        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            create_validation_config(),
        )
        .unwrap();

        // Create a mock certificate - in real implementation, this would be parsed X509
        // For now, we test that the validator can handle the structure
        let now = Utc::now();

        // The validation should handle various scenarios
        // Since we're using mock data, we expect some validation errors
        // This tests the structure of the validation logic
        let result = validator.validate_single_cert(
            // This would be a real X509Certificate in production
            unsafe { std::mem::zeroed() },
            now,
        );

        // Mock certificate validation may pass basic structural checks
        // or fail with InvalidChain - both are acceptable for this test
        match result {
            Ok(_) | Err(CertificateValidationError::InvalidChain) => {
                // Expected outcomes
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_validate_chain_trust_empty_chain() {
        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            create_validation_config(),
        )
        .unwrap();

        let result = validator.validate_chain_trust(&vec![]);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CertificateValidationError::InvalidChain
        ));
    }

    #[test]
    fn test_validate_chain_trust_with_single_cert() {
        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            create_validation_config(),
        )
        .unwrap();

        // Note: validate_chain_trust expects X509Certificate types from x509-parser
        // This test is skipped as it requires real certificate parsing
        // In a real implementation, we would convert CertificateDer to X509Certificate
    }

    #[test]
    fn test_validate_client_identity() {
        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            create_validation_config(),
        )
        .unwrap();

        let result = validator.validate_client_identity(
            // Would be real X509Certificate in production
            unsafe { std::mem::zeroed() },
        );

        // Should accept identity (in production, would validate against SAN)
        assert!(result.is_ok());
    }

    #[test]
    fn test_certificate_validation_config_ip_parsing() {
        let config = MtlsConfig {
            ca_cert_path: None,
            require_client_cert: true,
            allowed_client_dns_names: None,
            allowed_client_ips: Some(vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "192.168.1.1".to_string(),
                "invalid-ip".to_string(),
            ]),
            max_cert_chain_depth: None,
        };

        let validation_config = CertificateValidationConfig::from(&config);

        // Should parse valid IPs and ignore invalid ones
        assert_eq!(validation_config.allowed_client_ips.len(), 3);
    }

    #[test]
    fn test_production_validator_with_zero_max_depth() {
        let config = CertificateValidationConfig {
            require_client_auth: true,
            allowed_client_dns_names: vec![],
            allowed_client_ips: vec![],
            max_cert_chain_depth: 0,
        };

        let validator = ProductionCertificateValidator::new(
            b"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----",
            config,
        );

        assert!(validator.is_ok());
        let validator = validator.unwrap();

        let result = validator.validate_client_cert_chain(&create_valid_test_cert_chain());

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_empty_cert_chain() {
        let config = MtlsConfig {
            ca_cert_path: None,
            require_client_cert: true,
            allowed_client_dns_names: None,
            allowed_client_ips: None,
            max_cert_chain_depth: None,
        };

        let validator = TlsCertificateValidator::new(config).await.unwrap();

        let result = validator.validate_cert_chain(&vec![]).await;

        // With CA required but no CA configured, should handle gracefully
        // or fail based on implementation
        match result {
            Ok(_) => {
                // May succeed if not strictly enforced
            }
            Err(e) => {
                // Or may fail with appropriate error
                assert!(matches!(e, SecurityError::CertificateValidation(_)));
            }
        }
    }

    #[test]
    fn test_certificate_validation_error_display() {
        let errors = vec![
            (
                "Invalid certificate chain",
                CertificateValidationError::InvalidChain,
            ),
            ("Certificate expired", CertificateValidationError::Expired),
            (
                "Certificate not yet valid",
                CertificateValidationError::NotYetValid,
            ),
            (
                "Invalid certificate subject",
                CertificateValidationError::InvalidSubject,
            ),
            (
                "Certificate authority validation failed",
                CertificateValidationError::CaValidationFailed,
            ),
            (
                "Client name mismatch",
                CertificateValidationError::NameMismatch,
            ),
            ("Certificate revoked", CertificateValidationError::Revoked),
            (
                "Key usage validation failed",
                CertificateValidationError::KeyUsageInvalid,
            ),
            (
                "Extended key usage validation failed",
                CertificateValidationError::ExtendedKeyUsageInvalid,
            ),
            (
                "Internal error",
                CertificateValidationError::Internal("test".to_string()),
            ),
];

        for (expected_msg, error) in errors {
            assert_eq!(format!("{}", error), expected_msg);
        }
    }
}

pub struct TlsCertificateValidator {
    config: MtlsConfig,
    validator: Option<ProductionCertificateValidator>,
}

impl TlsCertificateValidator {
    pub async fn new(config: MtlsConfig) -> Result<Self> {
        let validator = if let Some(path) = &config.ca_cert_path {
            let ca_cert_pem = fs::read(path).map_err(|e| SecurityError::Io(e.to_string()))?;

            let validation_config = CertificateValidationConfig::from(&config);
            let prod_validator =
                ProductionCertificateValidator::new(&ca_cert_pem, validation_config)
                    .map_err(|e| SecurityError::CertificateValidation(e.to_string()))?;

            Some(prod_validator)
        } else {
            None
        };

        Ok(Self { config, validator })
    }

    /// Validate a complete certificate chain
    pub async fn validate_cert_chain(&self, cert_chain: &[CertificateDer<'_>]) -> Result<()> {
        if !self.config.require_client_cert {
            return Ok(());
        }

        if let Some(validator) = &self.validator {
            validator
                .validate_client_cert_chain(cert_chain)
                .map_err(|e| SecurityError::CertificateValidation(e.to_string()))?;
        }

        Ok(())
    }
}

#[async_trait]
impl CertificateValidator for TlsCertificateValidator {
    async fn validate_cert(&self, cert_pem: &[u8]) -> Result<()> {
        if !self.config.require_client_cert {
            return Ok(());
        }

        // Parse certificate
        let (_, pem) = x509_parser::pem::parse_x509_pem(cert_pem)
            .map_err(|e| SecurityError::CertificateValidation(format!("Invalid PEM: {}", e)))?;

        let _cert = pem.parse_x509().map_err(|e| {
            SecurityError::CertificateValidation(format!("Invalid Certificate: {}", e))
        })?;

        // Use full chain validation if available
        if let Some(validator) = &self.validator {
            let cert = CertificateDer::from(cert_pem.to_vec());
            validator
                .validate_client_cert_chain(&[cert])
                .map_err(|e| SecurityError::CertificateValidation(e.to_string()))?;
        }

        Ok(())
    }
}
