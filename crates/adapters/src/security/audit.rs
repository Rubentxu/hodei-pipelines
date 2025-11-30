use async_trait::async_trait;
use hodei_pipelines_core::security::SecurityContext;
use hodei_pipelines_ports::security::{AuditLogger, Result};
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

