use async_trait::async_trait;
use hodei_core::security::{JwtClaims, Permission, Role, SecurityContext};
use hodei_ports::security::{Result, SecurityError, TokenService};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_seconds: u64,
}

pub struct JwtTokenService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtTokenService {
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }
}

#[async_trait]
impl TokenService for JwtTokenService {
    fn generate_token(
        &self,
        subject: &str,
        roles: Vec<Role>,
        permissions: Vec<Permission>,
        tenant_id: Option<String>,
    ) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SecurityError::Other(e.to_string()))?
            .as_secs() as usize;

        let claims = JwtClaims {
            sub: subject.to_string(),
            exp: now + self.config.expiration_seconds as usize,
            iat: now,
            roles,
            permissions,
            tenant_id,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| SecurityError::Jwt(e.to_string()))
    }

    fn verify_token(&self, token: &str) -> Result<JwtClaims> {
        let validation = Validation::default();
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| SecurityError::Jwt(e.to_string()))?;

        Ok(token_data.claims)
    }

    fn get_context(&self, token: &str) -> Result<SecurityContext> {
        let claims = self.verify_token(token)?;
        Ok(SecurityContext::new(
            claims.sub,
            claims.roles,
            claims.permissions,
            claims.tenant_id,
        ))
    }
}
