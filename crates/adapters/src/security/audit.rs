use async_trait::async_trait;
use hodei_core::security::SecurityContext;
use hodei_ports::security::{AuditLogger, Result};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
}

pub struct AuditLoggerAdapter {
    config: AuditConfig,
}

impl AuditLoggerAdapter {
    pub async fn new(config: AuditConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl AuditLogger for AuditLoggerAdapter {
    async fn log_event(
        &self,
        event_type: &str,
        details: &str,
        context: Option<&SecurityContext>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let user = context.map(|c| c.subject.as_str()).unwrap_or("anonymous");
        let tenant = context
            .and_then(|c| c.tenant_id.as_deref())
            .unwrap_or("system");

        // For now, log to tracing/stdout. In production, this would go to a secure audit log (DB/File/Service)
        info!(
            target: "audit",
            event_type = event_type,
            user = user,
            tenant = tenant,
            details = details,
            "AUDIT EVENT"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::security::{Permission, Role};
    use tracing_test::traced_test;

    fn create_enabled_config() -> AuditConfig {
        AuditConfig { enabled: true }
    }

    fn create_disabled_config() -> AuditConfig {
        AuditConfig { enabled: false }
    }

    fn create_security_context() -> SecurityContext {
        SecurityContext {
            subject: "test-user".to_string(),
            roles: vec![Role::Admin],
            permissions: vec![Permission::AdminSystem],
            tenant_id: Some("test-tenant".to_string()),
        }
    }

    fn create_anonymous_context() -> SecurityContext {
        SecurityContext {
            subject: "anonymous".to_string(),
            roles: vec![],
            permissions: vec![],
            tenant_id: None,
        }
    }

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await;

        assert!(logger.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logger_with_disabled_config() {
        let config = create_disabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let result = logger
            .log_event(
                "test_event",
                "test details",
                Some(&create_security_context()),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_log_event_when_enabled() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let result = logger
            .log_event(
                "USER_LOGIN",
                "User successfully logged in",
                Some(&create_security_context()),
            )
            .await;

        assert!(result.is_ok());

        // Check if audit event was logged
        // In a real test with mock, we'd verify the log output
    }

    #[tokio::test]
    #[traced_test]
    async fn test_log_event_with_context() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let context = create_security_context();

        let result = logger
            .log_event(
                "JOB_CREATED",
                "Job 'test-job' created successfully",
                Some(&context),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_log_event_without_context() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let result = logger
            .log_event("SYSTEM_EVENT", "System maintenance event", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_log_event_multiple_types() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let event_types = vec![
            "USER_LOGIN",
            "USER_LOGOUT",
            "JOB_CREATED",
            "JOB_DELETED",
            "WORKER_REGISTERED",
            "CONFIG_CHANGED",
        ];

        for event_type in event_types {
            let result = logger
                .log_event(event_type, "Test event", Some(&create_security_context()))
                .await;

            assert!(result.is_ok(), "Failed for event type: {}", event_type);
        }
    }

    #[tokio::test]
    async fn test_audit_logging_with_different_tenants() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let tenants = vec!["tenant-1", "tenant-2", "tenant-3"];

        for tenant_id in tenants {
            let context = SecurityContext {
                subject: "user".to_string(),
                roles: vec![Role::Worker],
                permissions: vec![Permission::ReadJobs],
                tenant_id: Some(tenant_id.to_string()),
            };

            let result = logger
                .log_event("ACCESS_REQUEST", "User accessed resource", Some(&context))
                .await;

            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_audit_logging_with_long_details() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let long_details = "A".repeat(10000);

        let result = logger
            .log_event(
                "BULK_OPERATION",
                &long_details,
                Some(&create_security_context()),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logging_with_special_characters() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let special_details = "User <admin@test.com> performed action on 'resource/123' with parameters: {\"key\": \"value\"}";

        let result = logger
            .log_event(
                "SPECIAL_OPERATION",
                special_details,
                Some(&create_security_context()),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logging_multiple_events_sequentially() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let num_events = 100;

        for i in 0..num_events {
            let result = logger
                .log_event(
                    &format!("EVENT_{}", i),
                    &format!("Event number {}", i),
                    Some(&create_security_context()),
                )
    .await;

            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_audit_logging_with_empty_details() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let result = logger
            .log_event("EMPTY_EVENT", "", Some(&create_security_context()))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logging_anonymous_user() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let result = logger
            .log_event(
                "ANONYMOUS_ACCESS",
                "Access without authentication",
                Some(&create_anonymous_context()),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_logging_no_context() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let result = logger
            .log_event("SYSTEM_EVENT", "Event without context", None)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_audit_config_disabled_still_creates_logger() {
        let config = create_disabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        // Even when disabled, logger should be created successfully
        assert!(logger.config.enabled == false);
    }

    #[tokio::test]
    async fn test_audit_logging_with_unicode_characters() {
        let config = create_enabled_config();
        let logger = AuditLoggerAdapter::new(config).await.unwrap();

        let unicode_details = "用户张三执行了操作 🚀 Привет мир مرحبا";

        let result = logger
            .log_event(
                "UNICODE_EVENT",
                unicode_details,
                Some(&create_security_context()),
            )
            .await;

        assert!(result.is_ok());
    }

    #[test]
    fn test_audit_config_structure() {
        let enabled_config = create_enabled_config();
        let disabled_config = create_disabled_config();

        assert!(enabled_config.enabled);
        assert!(!disabled_config.enabled);
    }

    #[test]
    fn test_security_context_creation() {
        let context = create_security_context();

        assert_eq!(context.subject, "test-user");
        assert_eq!(context.tenant_id, Some("test-tenant".to_string()));
        assert!(!context.roles.is_empty());
        assert!(!context.permissions.is_empty());
    }

    #[test]
    fn test_anonymous_security_context() {
        let context = create_anonymous_context();

        assert_eq!(context.subject, "anonymous");
        assert_eq!(context.tenant_id, None);
        assert!(context.roles.is_empty());
        assert!(context.permissions.is_empty());
    }
}
