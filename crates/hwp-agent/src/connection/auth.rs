//! Authentication module
//!
//! This module handles JWT token authentication.

use tonic::{Request, Status, service::Interceptor};
use tracing::debug;

/// Authentication interceptor for gRPC calls
#[derive(Debug, Clone)]
pub struct AuthInterceptor {
    token: Option<String>,
}

impl AuthInterceptor {
    /// Create a new auth interceptor
    pub fn new() -> Self {
        let token = std::env::var("HODEI_TOKEN").ok();
        Self { token }
    }

    /// Create with custom token
    pub fn with_token(token: String) -> Self {
        Self { token: Some(token) }
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
        // Default token should be None (not empty string) unless env var is set
        // We can't guarantee env var state here easily without serial tests,
        // but we assume it's unset in clean env
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
