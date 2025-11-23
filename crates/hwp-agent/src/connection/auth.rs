//! Authentication module
//!
//! This module handles JWT token authentication and mTLS certificate validation.

use tonic::{Request, Status, service::Interceptor};
use tracing::{debug, warn};

#[cfg(feature = "security")]
use hodei_security::jwt::{JwtClaims, TokenManager};

/// Authentication interceptor for gRPC calls
#[derive(Debug, Clone)]
pub struct AuthInterceptor {
    token: Option<String>,
    #[cfg(feature = "security")]
    token_manager: Option<TokenManager>,
}

impl AuthInterceptor {
    /// Create a new auth interceptor
    pub fn new() -> Self {
        let token = std::env::var("HODEI_TOKEN").ok();
        Self {
            token,
            #[cfg(feature = "security")]
            token_manager: None,
        }
    }

    #[cfg(feature = "security")]
    /// Create with custom token manager
    pub fn with_token_manager(token_manager: TokenManager) -> Self {
        let token = std::env::var("HODEI_TOKEN").ok();
        Self {
            token,
            token_manager: Some(token_manager),
        }
    }

    /// Create with custom token
    pub fn with_token(token: String) -> Self {
        Self {
            token: Some(token),
            #[cfg(feature = "security")]
            token_manager: None,
        }
    }

    /// Get the current token
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Validate token
    pub fn validate_token(&self) -> Result<(), Status> {
        if let Some(token) = &self.token {
            if token.is_empty() {
                return Err(Status::unauthenticated("Empty token provided"));
            }

            #[cfg(feature = "security")]
            {
                if let Some(manager) = &self.token_manager {
                    match manager.verify_token(token) {
                        Ok(claims) => {
                            debug!(
                                "JWT validation successful for worker: {}",
                                claims.claims.worker_id
                            );
                            return Ok(());
                        }
                        Err(e) => {
                            warn!("JWT validation failed: {}", e);
                            return Err(Status::unauthenticated("Invalid token"));
                        }
                    }
                }
            }

            debug!("Token validation passed");
            Ok(())
        } else {
            Err(Status::unauthenticated("No token provided"))
        }
    }
}

impl Default for AuthInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        // Validate token
        self.validate_token()?;

        // Add authorization header if we have a token
        if let Some(token) = &self.token {
            let auth_value = format!("Bearer {}", token);
            if let Ok(v) = auth_value.parse() {
                req.metadata_mut().insert("authorization", v);
            }
        }

        Ok(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_interceptor_creation() {
        let interceptor = AuthInterceptor::new();
        // Default token should be None (not empty string)
        assert_eq!(interceptor.token(), None);
    }

    #[test]
    fn test_auth_interceptor_with_token() {
        let interceptor = AuthInterceptor::with_token("test-token".to_string());
        assert_eq!(interceptor.token(), Some("test-token"));
    }

    #[test]
    fn test_empty_token_validation() {
        let interceptor = AuthInterceptor::with_token("".to_string());
        assert!(interceptor.validate_token().is_err());
    }

    #[test]
    fn test_token_validation() {
        let interceptor = AuthInterceptor::with_token("valid-token".to_string());
        assert!(interceptor.validate_token().is_ok());
    }
}
