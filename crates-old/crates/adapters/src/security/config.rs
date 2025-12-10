use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub jwt: super::jwt::JwtConfig,
    pub mtls: super::mtls::MtlsConfig,
    pub masking: super::masking::MaskingConfig,
    pub audit: super::audit::AuditConfig,
}
