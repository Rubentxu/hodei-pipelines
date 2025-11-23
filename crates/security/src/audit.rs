use crate::Result;
use crate::config::AuditConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub event_type: String,
    pub action: String,
}

pub struct AuditLogger {
    config: AuditConfig,
}

impl AuditLogger {
    pub async fn new(config: AuditConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub async fn log_event(&self, _event: AuditEvent) -> Result<()> {
        Ok(())
    }
}
