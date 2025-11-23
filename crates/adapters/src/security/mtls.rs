use async_trait::async_trait;
use hodei_ports::security::{CertificateValidator, Result, SecurityError};
use serde::Deserialize;
use std::fs;
use x509_parser::prelude::*;

#[derive(Debug, Clone, Deserialize)]
pub struct MtlsConfig {
    pub ca_cert_path: Option<String>,
    pub require_client_cert: bool,
}

pub struct TlsCertificateValidator {
    config: MtlsConfig,
    ca_cert: Option<Vec<u8>>,
}

impl TlsCertificateValidator {
    pub async fn new(config: MtlsConfig) -> Result<Self> {
        let ca_cert = if let Some(path) = &config.ca_cert_path {
            Some(fs::read(path).map_err(|e| SecurityError::Io(e.to_string()))?)
        } else {
            None
        };

        Ok(Self { config, ca_cert })
    }
}

#[async_trait]
impl CertificateValidator for TlsCertificateValidator {
    async fn validate_cert(&self, cert_pem: &[u8]) -> Result<()> {
        if !self.config.require_client_cert {
            return Ok(());
        }

        // Basic parsing validation using x509-parser
        let (_, _pem) = x509_parser::pem::parse_x509_pem(cert_pem)
            .map_err(|e| SecurityError::CertificateValidation(format!("Invalid PEM: {}", e)))?;

        let _cert = _pem.parse_x509().map_err(|e| {
            SecurityError::CertificateValidation(format!("Invalid Certificate: {}", e))
        })?;

        // In a real implementation, we would verify the signature against self.ca_cert
        // For now, we at least check it's a valid certificate structure

        Ok(())
    }
}
