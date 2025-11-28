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

/// Granular TLS/mTLS Configuration for Production Kubernetes Deployments
#[derive(Debug, Clone, Deserialize)]
pub struct MtlsConfig {
    /// Path to CA certificate for validating client certificates
    pub ca_cert_path: Option<String>,

    /// Require mutual TLS authentication (client certificates)
    pub require_client_cert: bool,

    /// Allowlist of client DNS names for certificate validation
    pub allowed_client_dns_names: Option<Vec<String>>,

    /// Allowlist of client IP addresses for certificate validation
    pub allowed_client_ips: Option<Vec<String>>,

    /// Maximum allowed certificate chain depth
    pub max_cert_chain_depth: Option<u8>,

    /// TLS protocol versions to allow (min and max)
    pub tls_version: Option<TlsVersionConfig>,

    /// Allowed cipher suites for TLS connections
    pub cipher_suites: Option<CipherSuiteConfig>,

    /// Certificate revocation checking configuration
    pub revocation_checking: Option<RevocationConfig>,

    /// Certificate policy OIDs that must be present
    pub required_policies: Option<Vec<String>>,

    /// Whether to verify certificate hostname/SAN
    pub verify_hostname: bool,

    /// Whether to verify certificate trust chain
    pub verify_trust_chain: bool,

    /// Whether to enable certificate transparency logging
    pub ct_logging: Option<CtLoggingConfig>,
}

/// TLS version configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TlsVersionConfig {
    /// Minimum TLS version (default: TLS 1.2)
    pub min_version: TlsVersion,
    /// Maximum TLS version (default: TLS 1.3)
    pub max_version: Option<TlsVersion>,
}

/// TLS version enum
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

/// Cipher suite configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CipherSuiteConfig {
    /// Allow modern cipher suites (TLS 1.3 compatible)
    pub modern_suites: bool,
    /// Allow TLS 1.2 cipher suites
    pub tls12_compatible: bool,
    /// Explicit allowlist of cipher suites (overrides presets)
    pub allowed_suites: Option<Vec<String>>,
    /// Disallow weak cipher suites
    pub disallow_weak: bool,
}

/// Certificate revocation checking configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RevocationConfig {
    /// Enable OCSP (Online Certificate Status Protocol) checking
    pub ocsp_enabled: bool,
    /// Enable CRL (Certificate Revocation List) checking
    pub crl_enabled: bool,
    /// OCSP responder URL
    pub ocsp_responder_url: Option<String>,
    /// Timeout for revocation checking (seconds)
    pub check_timeout_seconds: u64,
    /// Cache TTL for revocation status (seconds)
    pub cache_ttl_seconds: u64,
}

/// Certificate Transparency logging configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CtLoggingConfig {
    /// Enable CT log submission
    pub enabled: bool,
    /// CT log server URL
    pub log_server_url: Option<String>,
    /// Maximum number of retries for CT log submission
    pub max_retries: u8,
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self {
            ca_cert_path: None,
            require_client_cert: true,
            allowed_client_dns_names: Some(Vec::new()),
            allowed_client_ips: Some(Vec::new()),
            max_cert_chain_depth: Some(10),
            tls_version: Some(TlsVersionConfig {
                min_version: TlsVersion::Tls12,
                max_version: Some(TlsVersion::Tls13),
            }),
            cipher_suites: Some(CipherSuiteConfig {
                modern_suites: true,
                tls12_compatible: true,
                allowed_suites: None,
                disallow_weak: true,
            }),
            revocation_checking: Some(RevocationConfig {
                ocsp_enabled: true,
                crl_enabled: true,
                ocsp_responder_url: None,
                check_timeout_seconds: 5,
                cache_ttl_seconds: 3600,
            }),
            required_policies: Some(Vec::new()),
            verify_hostname: true,
            verify_trust_chain: true,
            ct_logging: Some(CtLoggingConfig {
                enabled: false,
                log_server_url: None,
                max_retries: 3,
            }),
        }
    }
}

/// Certificate validation configuration extracted from MtlsConfig
#[derive(Debug, Clone)]
pub struct CertificateValidationConfig {
    pub require_client_auth: bool,
    pub allowed_client_dns_names: Vec<String>,
    pub allowed_client_ips: Vec<IpAddr>,
    pub max_cert_chain_depth: u8,
    pub verify_hostname: bool,
    pub verify_trust_chain: bool,
    pub tls_version: Option<TlsVersionConfig>,
    pub cipher_suites: Option<CipherSuiteConfig>,
    pub revocation_checking: Option<RevocationConfig>,
    pub required_policies: Vec<String>,
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
            verify_hostname: config.verify_hostname,
            verify_trust_chain: config.verify_trust_chain,
            tls_version: config.tls_version.clone(),
            cipher_suites: config.cipher_suites.clone(),
            revocation_checking: config.revocation_checking.clone(),
            required_policies: config.required_policies.clone().unwrap_or_default(),
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
            root_store.add(cert).map_err(|e| {
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

    pub fn validate_single_cert(
        &self,
        cert: &x509_parser::certificate::X509Certificate,
        now: DateTime<Utc>,
    ) -> std::result::Result<(), CertificateValidationError> {
        // Validate certificate has required fields
        let _ = cert.subject();
        let _ = cert.issuer();
        let validity = cert.validity();

        // US-01.2: Validate certificate time validity periods
        // Check if certificate is currently valid based on not_before and not_after
        let not_before = validity.not_before;
        let not_after = validity.not_after;

        // Convert current time to Unix timestamp for comparison
        let current_timestamp = now.timestamp();

        // Get timestamps from certificate validity periods using ASN1Time::timestamp()
        let not_before_timestamp = not_before.timestamp();
        let not_after_timestamp = not_after.timestamp();

        // Check if certificate is not yet valid
        if current_timestamp < not_before_timestamp {
            return Err(CertificateValidationError::NotYetValid);
        }

        // Check if certificate is expired
        // not_after is an exclusive upper bound, so current_time >= not_after means expired
        if current_timestamp >= not_after_timestamp {
            return Err(CertificateValidationError::Expired);
        }

        // US-01.3: Validate Key Usage Extensions for client authentication
        self.validate_key_usage(cert)?;

        // US-01.4: Validate Extended Key Usage (EKU) for client authentication
        self.validate_extended_key_usage(cert)?;

        // US-01.5: Validate Subject Alternative Name (SAN) for client identity
        self.validate_subject_alternative_name(cert)?;

        // TODO: Add comprehensive validation:
        // - Verify certificate signature (requires CA certificate)
        // - Verify certificate policies
        // - Check revocation status via CRL/OCSP

        Ok(())
    }

    fn validate_key_usage(
        &self,
        cert: &x509_parser::certificate::X509Certificate,
    ) -> std::result::Result<(), CertificateValidationError> {
        // Extract Key Usage extension from certificate
        let key_usage = cert.key_usage().map_err(|e| {
            CertificateValidationError::Internal(format!("Failed to parse Key Usage: {}", e))
        })?;

        // If no Key Usage extension is present, fail validation
        // Client certificates MUST have Key Usage for security
        let key_usage_ext = key_usage.ok_or(CertificateValidationError::KeyUsageInvalid)?;

        let usage = key_usage_ext.value;

        // Validate that client certificate has required Key Usage bits for TLS Client Authentication
        // According to RFC 5280, client certificates should have:
        // - digitalSignature: to sign TLS handshake messages
        // - keyEncipherment: to encrypt keys (for RSA keys)

        if !usage.digital_signature() {
            return Err(CertificateValidationError::KeyUsageInvalid);
        }

        // Note: keyEncipherment is required for RSA keys, but may not be present for ECDSA keys
        // For now, we require it but this could be made configurable based on the key algorithm
        // TODO: Check public key algorithm and adjust requirements accordingly
        if !usage.key_encipherment() {
            return Err(CertificateValidationError::KeyUsageInvalid);
        }

        Ok(())
    }

    fn validate_extended_key_usage(
        &self,
        cert: &x509_parser::certificate::X509Certificate,
    ) -> std::result::Result<(), CertificateValidationError> {
        // Extract Extended Key Usage extension from certificate
        let eku = cert.extended_key_usage().map_err(|e| {
            CertificateValidationError::Internal(format!(
                "Failed to parse Extended Key Usage: {}",
                e
            ))
        })?;

        // Extended Key Usage is OPTIONAL for client certificates
        // If present, it SHOULD contain clientAuth for TLS Client Authentication
        // Reference: RFC 5280 Section 4.2.1.12

        if let Some(eku_ext) = eku {
            let usage = eku_ext.value;

            // Check if EKU includes clientAuth
            // This is the primary purpose for TLS Client Authentication
            if !usage.client_auth {
                return Err(CertificateValidationError::ExtendedKeyUsageInvalid);
            }
        }

        // EKU not present - this is acceptable, Key Usage is often sufficient
        // RFC allows certificates without EKU to be used for client authentication
        // if the Key Usage indicates the appropriate purposes

        Ok(())
    }

    fn validate_subject_alternative_name(
        &self,
        cert: &x509_parser::certificate::X509Certificate,
    ) -> std::result::Result<(), CertificateValidationError> {
        // Extract Subject Alternative Name extension from certificate
        let san = cert.subject_alternative_name().map_err(|e| {
            CertificateValidationError::Internal(format!("Failed to parse SAN: {}", e))
        })?;

        // SAN is OPTIONAL for client certificates
        // If present, it should contain DNS names or IP addresses for client identity
        // Reference: RFC 5280 Section 4.2.1.6

        if let Some(san_ext) = san {
            let general_names = &san_ext.value.general_names;

            // Check if SAN contains any DNS names
            let has_valid_dns = general_names.iter().any(|name| {
                if let x509_parser::extensions::GeneralName::DNSName(dns) = name {
                    // Check if this DNS is in the allowed list
                    self.config
                        .allowed_client_dns_names
                        .iter()
                        .any(|allowed| allowed == dns)
                } else {
                    false
                }
            });

            // Check if SAN contains any IP addresses
            let has_valid_ip = general_names.iter().any(|name| {
                if let x509_parser::extensions::GeneralName::IPAddress(ip_addr) = name {
                    // Parse the IP address
                    if let Ok(ip) = std::str::from_utf8(ip_addr) {
                        // Check if this IP is in the allowed list
                        self.config
                            .allowed_client_ips
                            .iter()
                            .any(|allowed_ip| allowed_ip.to_string() == ip)
                    } else {
                        false
                    }
                } else {
                    false
                }
            });

            // At least one SAN entry must match the allowed list
            if !has_valid_dns && !has_valid_ip {
                return Err(CertificateValidationError::NameMismatch);
            }
        }

        // SAN not present - this is acceptable
        // Fall back to Common Name (CN) validation if needed
        // RFC allows certificates without SAN to be validated using CN

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
