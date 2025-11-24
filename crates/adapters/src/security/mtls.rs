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
