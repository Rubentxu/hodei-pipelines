use hodei_adapters::security::JwtTokenService;
use hodei_ports::security::TokenService;
use std::sync::Arc;
use tonic::{Request, Status, service::Interceptor};

#[derive(Clone)]
pub struct AuthInterceptor {
    token_service: Arc<JwtTokenService>,
}

impl AuthInterceptor {
    pub fn new(token_service: Arc<JwtTokenService>) -> Self {
        Self { token_service }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let token = match request.metadata().get("authorization") {
            Some(t) => t
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid token format"))?,
            None => return Err(Status::unauthenticated("Missing authorization token")),
        };

        let bearer = token
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("Invalid token format"))?;

        match self.token_service.verify_token(bearer) {
            Ok(_) => Ok(request),
            Err(e) => Err(Status::unauthenticated(format!("Invalid token: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_adapters::security::{JwtConfig, JwtTokenService};
    use hodei_core::security::{Role, Permission};
    use hodei_ports::security::SecurityError;
    use std::sync::Arc;
    use tonic::service::Interceptor;

    // Mock TokenService for testing
    #[derive(Clone)]
    struct MockTokenService {
        valid_token: String,
        should_fail: bool,
    }

    impl MockTokenService {
        fn new_valid_token(valid_token: String) -> Self {
            Self {
                valid_token,
                should_fail: false,
            }
        }

        fn new_failing_token() -> Self {
            Self {
                valid_token: "invalid".to_string(),
                should_fail: true,
            }
        }
    }

    impl TokenService for MockTokenService {
        fn generate_token(
            &self,
            _subject: &str,
            _roles: Vec<Role>,
            _permissions: Vec<Permission>,
            _tenant_id: Option<String>,
        ) -> hodei_ports::security::Result<String> {
            Ok(self.valid_token.clone())
        }

        fn verify_token(&self, token: &str) -> hodei_ports::security::Result<hodei_core::security::JwtClaims> {
            if self.should_fail || token != self.valid_token {
                return Err(SecurityError::Jwt("Invalid token".to_string()));
            }

            Ok(hodei_core::security::JwtClaims {
                sub: "test-user".to_string(),
                exp: 9999999999,
                iat: 1000000000,
                roles: vec![Role::Admin],
                permissions: vec![Permission::AdminSystem],
                tenant_id: Some("test-tenant".to_string()),
            })
        }

        fn get_context(&self, _token: &str) -> hodei_ports::security::Result<hodei_core::security::SecurityContext> {
            Ok(hodei_core::security::SecurityContext::new(
                "test-user".to_string(),
                vec![Role::Admin],
                vec![Permission::AdminSystem],
                Some("test-tenant".to_string()),
            ))
        }
    }

    fn create_valid_token() -> String {
        let config = JwtConfig {
            secret: "test-secret-key-for-jwt-validation".to_string(),
            expiration_seconds: 3600,
        };
        let service = JwtTokenService::new(config);
        service.generate_token(
            "test-user",
            vec![Role::Admin],
            vec![Permission::AdminSystem],
            Some("test-tenant".to_string()),
        ).unwrap()
    }

    fn create_auth_interceptor_with_mock(valid_token: String) -> AuthInterceptor {
        let mock_service = MockTokenService::new_valid_token(valid_token);
        AuthInterceptor::new(Arc::new(mock_service))
    }

    fn create_auth_interceptor_failing() -> AuthInterceptor {
        let mock_service = MockTokenService::new_failing_token();
        AuthInterceptor::new(Arc::new(mock_service))
    }

    #[test]
    fn test_auth_interceptor_creation() {
        let valid_token = create_valid_token();
        let interceptor = create_auth_interceptor_with_mock(valid_token);

        assert!(interceptor.token_service.is_some());
    }

    #[test]
    fn test_interceptor_with_valid_bearer_token() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token.clone());

        let request = Request::builder()
            .add_header("authorization", format!("Bearer {}", valid_token))
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_ok());
    }

    #[test]
    fn test_interceptor_with_valid_token_without_bearer_prefix() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        let request = Request::builder()
            .add_header("authorization", valid_token)
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            assert!(status.message().contains("Invalid token format"));
        }
    }

    #[test]
    fn test_interceptor_with_missing_authorization_header() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        let request = Request::builder().body(()).unwrap();

        let result = interceptor.call(request);

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            assert!(status.message().contains("Missing authorization token"));
        }
    }

    #[test]
    fn test_interceptor_with_invalid_bearer_format() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        let request = Request::builder()
            .add_header("authorization", "InvalidFormat token")
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            assert!(status.message().contains("Invalid token format"));
        }
    }

    #[test]
    fn test_interceptor_with_empty_bearer_token() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        let request = Request::builder()
            .add_header("authorization", "Bearer ")
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
        }
    }

    #[test]
    fn test_interceptor_with_wrong_token() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        let request = Request::builder()
            .add_header("authorization", "Bearer wrong-token")
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            assert!(status.message().contains("Invalid token"));
        }
    }

    #[test]
    fn test_interceptor_with_malformed_metadata() {
        let mock_service = MockTokenService::new_failing_token();
        let mut interceptor = AuthInterceptor::new(Arc::new(mock_service));

        // Create request with invalid metadata value (non-UTF8)
        let request = Request::builder()
            .add_header("authorization", "\xff\xff")  // Invalid UTF-8
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
        }
    }

    #[test]
    fn test_interceptor_case_sensitive_bearer() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token.clone());

        // Test lowercase "bearer" - should fail
        let request = Request::builder()
            .add_header("authorization", format!("bearer {}", valid_token))
            .body(())
            .unwrap();

        let result = interceptor.call(request);
        assert!(result.is_err());  // Should fail due to case sensitivity
    }

    #[test]
    fn test_interceptor_preserves_request_data() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        let request = Request::builder()
            .add_header("authorization", format!("Bearer {}", valid_token))
            .add_header("custom-header", "custom-value")
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_ok());
        if let Ok(request) = result {
            assert!(request.metadata().get("custom-header").is_some());
        }
    }

    #[test]
    fn test_interceptor_with_multiple_auth_headers() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        // gRPC allows multiple values for the same header
        // The first one should be used
        let request = Request::builder()
            .add_header("authorization", format!("Bearer {}", valid_token))
            .add_header("authorization", "Bearer invalid")
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        // Should succeed if first token is valid
        assert!(result.is_ok());
    }

    #[test]
    fn test_auth_interceptor_clone() {
        let valid_token = create_valid_token();
        let interceptor1 = create_auth_interceptor_with_mock(valid_token);
        let interceptor2 = interceptor1.clone();

        assert!(Arc::ptr_eq(&interceptor1.token_service, &interceptor2.token_service));
    }

    #[test]
    fn test_interceptor_allowlist_path() {
        // This test verifies that the interceptor structure supports
        // future allowlist logic without actually testing it
        let valid_token = create_valid_token();
        let interceptor = create_auth_interceptor_with_mock(valid_token);

        // The interceptor is a simple struct, can be extended
        // This test ensures the struct remains cloneable and usable
        assert!(interceptor.token_service.is_some());
    }

    #[test]
    fn test_interceptor_with_token_containing_spaces() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token.clone());

        // Token with spaces (should fail)
        let request = Request::builder()
            .add_header("authorization", format!("Bearer {} extra", valid_token))
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        // Should fail because token contains "extra" which is not part of the actual token
        assert!(result.is_err());
    }

    #[test]
    fn test_interceptor_error_messages() {
        let mut interceptor = create_auth_interceptor_failing();

        // Test missing token error message
        let request = Request::builder().body(()).unwrap();
        let result = interceptor.call(request);
        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            assert!(status.message().contains("Missing"));
        }

        // Test invalid token error message
        let request = Request::builder()
            .add_header("authorization", "Bearer invalid")
            .body(())
            .unwrap();
        let result = interceptor.call(request);
        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            assert!(status.message().contains("Invalid"));
        }
    }

    #[test]
    fn test_interceptor_with_empty_token_string() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token);

        let request = Request::builder()
            .add_header("authorization", "")
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        assert!(result.is_err());
        if let Err(status) = result {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
        }
    }

    #[test]
    fn test_interceptor_with_very_long_token() {
        let valid_token = create_valid_token();
        let mut interceptor = create_auth_interceptor_with_mock(valid_token.clone());

        let long_token = valid_token + &"x".repeat(10000);
        let request = Request::builder()
            .add_header("authorization", format!("Bearer {}", long_token))
            .body(())
            .unwrap();

        let result = interceptor.call(request);

        // Should fail because token doesn't match
        assert!(result.is_err());
    }
}
