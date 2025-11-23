use crate::Result;
use crate::config::MtlsConfig;

pub struct TlsConfig {
    pub ca_cert_path: Option<String>,
}

pub struct CertificateValidator {
    config: MtlsConfig,
}

impl CertificateValidator {
    pub async fn new(config: MtlsConfig) -> Result<Self> {
        Ok(Self { config })
    }
}
