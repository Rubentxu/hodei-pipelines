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

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::security::{Permission, Role};
    use jsonwebtoken::errors::ErrorKind;

    fn create_test_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key-for-jwt-validation-256bits".to_string(),
            expiration_seconds: 3600,
        }
    }

    fn create_test_token_service() -> JwtTokenService {
        JwtTokenService::new(create_test_config())
    }

    #[tokio::test]
    async fn test_valid_token_generation() {
        let service = create_test_token_service();
        let roles = vec![Role::Worker, Role::Admin];
        let permissions = vec![Permission::WriteJobs, Permission::ReadJobs];

        let token = service.generate_token(
            "test-user",
            roles.clone(),
            permissions.clone(),
            Some("test-tenant".to_string()),
        );

        assert!(token.is_ok());
        let token_str = token.unwrap();
        assert!(!token_str.is_empty());
    }

    #[tokio::test]
    async fn test_token_with_empty_roles_and_permissions() {
        let service = create_test_token_service();

        let token = service.generate_token("test-user", vec![], vec![], None);

        assert!(token.is_ok());
    }

    #[tokio::test]
    async fn test_valid_token_verification() {
        let service = create_test_token_service();
        let roles = vec![Role::Admin];
        let permissions = vec![Permission::AdminSystem];

        let token = service
            .generate_token(
                "admin-user",
                roles.clone(),
                permissions.clone(),
                Some("admin-tenant".to_string()),
            )
            .unwrap();

        let verified_claims = service.verify_token(&token);

        assert!(verified_claims.is_ok());
        let claims = verified_claims.unwrap();
        assert_eq!(claims.sub, "admin-user");
        assert_eq!(claims.roles, roles);
        assert_eq!(claims.permissions, permissions);
        assert_eq!(claims.tenant_id, Some("admin-tenant".to_string()));
    }

    #[tokio::test]
    async fn test_invalid_token_rejection() {
        let service = create_test_token_service();

        let result = service.verify_token("invalid-token-string");

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, SecurityError::Jwt(_)));
        }
    }

    #[tokio::test]
    async fn test_malformed_token_rejection() {
        let service = create_test_token_service();

        let result = service.verify_token("malformed.token.format");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_expired_token_handling() {
        let mut config = create_test_config();
        config.expiration_seconds = 0;
        let service = JwtTokenService::new(config);

        let token = service
            .generate_token(
                "test-user",
                vec![Role::Worker],
                vec![Permission::ReadJobs],
                None,
            )
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));

        let result = service.verify_token(&token);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_context_from_valid_token() {
        let service = create_test_token_service();
        let roles = vec![Role::Admin];
        let permissions = vec![Permission::WriteJobs, Permission::WriteJobs];

        let token = service
            .generate_token(
                "scheduler-user",
                roles.clone(),
                permissions.clone(),
                Some("scheduler-tenant".to_string()),
            )
            .unwrap();

        let context = service.get_context(&token);

        assert!(context.is_ok());
        let ctx = context.unwrap();
        assert_eq!(ctx.subject(), "scheduler-user");
        assert_eq!(ctx.roles(), &roles);
        assert_eq!(ctx.permissions(), &permissions);
        assert_eq!(ctx.tenant_id(), Some("scheduler-tenant".to_string()));
    }

    #[tokio::test]
    async fn test_get_context_from_invalid_token() {
        let service = create_test_token_service();

        let result = service.get_context("invalid-token");

        assert!(result.is_err());
        assert!(matches!(result, Err(SecurityError::Jwt(_))));
    }

    #[test]
    fn test_token_service_with_different_secrets() {
        let config1 = JwtConfig {
            secret: "secret-1".to_string(),
            expiration_seconds: 3600,
        };
        let config2 = JwtConfig {
            secret: "secret-2".to_string(),
            expiration_seconds: 3600,
        };

        let service1 = JwtTokenService::new(config1);
        let service2 = JwtTokenService::new(config2);

        let token1 = service1
            .generate_token("user", vec![Role::Worker], vec![Permission::ReadJobs], None)
            .unwrap();

        let token2 = service2
            .generate_token("user", vec![Role::Worker], vec![Permission::ReadJobs], None)
            .unwrap();

        assert_ne!(token1, token2);

        let verified1 = service1.verify_token(&token1);
        let verified2 = service2.verify_token(&token2);

        assert!(verified1.is_ok());
        assert!(verified2.is_ok());

        let cross_verify_1 = service1.verify_token(&token2);
        let cross_verify_2 = service2.verify_token(&token1);

        assert!(cross_verify_1.is_err());
        assert!(cross_verify_2.is_err());
    }

    #[test]
    fn test_multiple_concurrent_token_generation() {
        let service = create_test_token_service();
        let iterations = 100;

        let tokens: Vec<Result<String, _>> = (0..iterations)
            .map(|i| {
                service.generate_token(
                    &format!("user-{}", i),
                    vec![Role::Worker],
                    vec![Permission::ReadJobs],
                    None,
                )
            })
            .collect();

        assert_eq!(tokens.len(), iterations);
        assert!(tokens.iter().all(|r| r.is_ok()));

        let unique_tokens: std::collections::HashSet<String> =
            tokens.into_iter().filter_map(Result::ok).collect();

        assert_eq!(unique_tokens.len(), iterations);
    }

    #[test]
    fn test_token_with_special_characters_in_subject() {
        let service = create_test_token_service();

        let special_subjects = vec![
            "user@example.com",
            "user-with-dashes",
            "user_with_underscores",
            "user.with.dots",
        ];

        for subject in special_subjects {
            let token = service.generate_token(
                subject,
                vec![Role::Worker],
                vec![Permission::ReadJobs],
                None,
            );

            assert!(token.is_ok(), "Failed for subject: {}", subject);

            if let Ok(tok) = token {
                let claims = service.verify_token(&tok);
                assert!(
                    claims.is_ok(),
                    "Verification failed for subject: {}",
                    subject
                );
                assert_eq!(claims.unwrap().sub, subject);
            }
        }
    }

    #[test]
    fn test_token_persistence_across_service_instances() {
        let config = create_test_config();

        let service1 = JwtTokenService::new(config.clone());
        let service2 = JwtTokenService::new(config);

        let token = service1
            .generate_token(
                "persistent-user",
                vec![Role::Admin],
                vec![Permission::AdminSystem],
                Some("persistent-tenant".to_string()),
            )
            .unwrap();

        let claims1 = service1.verify_token(&token);
        let claims2 = service2.verify_token(&token);

        assert!(claims1.is_ok());
        assert!(claims2.is_ok());

        let c1 = claims1.unwrap();
        let c2 = claims2.unwrap();

        assert_eq!(c1.sub, c2.sub);
        assert_eq!(c1.roles, c2.roles);
        assert_eq!(c1.permissions, c2.permissions);
        assert_eq!(c1.tenant_id, c2.tenant_id);
    }

    #[test]
    fn test_very_long_expiration_time() {
        let mut config = create_test_config();
        config.expiration_seconds = u64::MAX;

        let service = JwtTokenService::new(config);

        let token = service.generate_token(
            "long-lived-user",
            vec![Role::Worker],
            vec![Permission::ReadJobs],
            None,
        );

        assert!(token.is_ok());
    }

    #[test]
    fn test_token_with_all_permissions_and_roles() {
        let service = create_test_token_service();

        let all_roles = vec![Role::Admin, Role::Worker, Role::Admin, Role::Viewer];

        let all_permissions = vec![
            Permission::AdminSystem,
            Permission::WriteJobs,
            Permission::ReadJobs,
            Permission::WriteJobs,
            Permission::DeleteJobs,
            Permission::WriteJobs,
            Permission::WriteJobs,
            Permission::ManageWorkers,
            Permission::ManageWorkers,
            Permission::WriteJobs,
            Permission::WriteJobs,
            Permission::ReadJobs,
        ];

        let token = service.generate_token(
            "super-user",
            all_roles.clone(),
            all_permissions.clone(),
            Some("super-tenant".to_string()),
        );

        assert!(token.is_ok());

        if let Ok(tok) = token {
            let claims = service.verify_token(&tok);
            assert!(claims.is_ok());

            let verified = claims.unwrap();
            assert_eq!(verified.roles, all_roles);
            assert_eq!(verified.permissions, all_permissions);
        }
    }
}
