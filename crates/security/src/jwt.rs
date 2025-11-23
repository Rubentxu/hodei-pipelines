use crate::Result;
use crate::config::JwtConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct JwtToken {
    pub token: String,
    pub claims: JwtClaims,
}

impl JwtToken {
    pub fn as_str(&self) -> &str {
        &self.token
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub worker_id: String,
    pub team_id: Option<String>,
}

#[derive(Clone)]
pub struct TokenManager {
    config: JwtConfig,
}

impl TokenManager {
    pub fn new(config: JwtConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn create_access_token(&self, worker_id: String) -> Result<JwtToken> {
        Ok(JwtToken {
            token: format!("token-{}-{}", worker_id, self.config.token_expiry_seconds),
            claims: JwtClaims {
                worker_id,
                team_id: None,
            },
        })
    }

    pub fn verify_token(&self, token: &str) -> Result<JwtToken> {
        // Simplified validation - in production, validate JWT properly
        if token.is_empty() {
            return Err(crate::SecurityError::Jwt("Empty token".to_string()));
        }
        Ok(JwtToken {
            token: token.to_string(),
            claims: JwtClaims {
                worker_id: "worker".to_string(),
                team_id: None,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_manager_creation() {
        let config = JwtConfig::default();
        let manager = TokenManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_create_access_token() {
        let config = JwtConfig::default();
        let manager = TokenManager::new(config).unwrap();
        let token = manager.create_access_token("worker-123".to_string());
        assert!(token.is_ok());
        let token = token.unwrap();
        assert!(token.token.contains("worker-123"));
        assert_eq!(token.claims.worker_id, "worker-123");
    }

    #[test]
    fn test_verify_token() {
        let config = JwtConfig::default();
        let manager = TokenManager::new(config).unwrap();
        let token = manager.verify_token("test-token");
        assert!(token.is_ok());
        let token = token.unwrap();
        assert_eq!(token.token, "test-token");
    }

    #[test]
    fn test_verify_empty_token() {
        let config = JwtConfig::default();
        let manager = TokenManager::new(config).unwrap();
        let result = manager.verify_token("");
        assert!(result.is_err());
    }
}
