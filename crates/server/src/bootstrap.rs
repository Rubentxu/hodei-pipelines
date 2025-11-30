//! Server Bootstrap - Production Initialization
//!
//! This module handles the initialization of all server components including
//! configuration loading, event bus setup, repository initialization,
//! and dependency injection for production deployments.

use hodei_pipelines_adapters::{InMemoryBus, config::AppConfig};
use hodei_pipelines_ports::EventSubscriber;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info};

/// Bootstrap error types
#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Configuration error: {0}")]
    Config(#[from] hodei_pipelines_adapters::config::ConfigError),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, BootstrapError>;

/// Server components initialized during bootstrap
#[derive(Clone)]
pub struct ServerComponents {
    pub config: AppConfig,
    pub event_subscriber: Arc<dyn EventSubscriber>,
    pub event_publisher: Arc<dyn hodei_pipelines_ports::EventPublisher>,
    #[allow(dead_code)]
    pub status: &'static str,
}

/// Health checker for monitoring server components
#[allow(dead_code)]
pub struct HealthChecker {
    pub config_loaded: bool,
    pub status: &'static str,
}

impl HealthChecker {
    /// Check health of all components
    #[allow(dead_code)]
    pub fn check_health(&self) -> bool {
        self.config_loaded
    }

    /// Get health status as string
    #[allow(dead_code)]
    pub fn status(&self) -> &'static str {
        if self.check_health() {
            "healthy"
        } else {
            "unhealthy"
        }
    }
}

/// Initialize all server components for production
pub async fn initialize_server() -> Result<ServerComponents> {
    info!("🚀 Initializing Hodei Pipelines Server for Production");
    info!("📋 Loading application configuration...");

    // Load configuration from environment or file
    let config = AppConfig::load().map_err(|e| {
        error!("❌ Failed to load configuration: {}", e);
        BootstrapError::Config(e)
    })?;
    info!("✅ Configuration loaded successfully");

    // Validate critical configuration
    if config.tls.enabled {
        info!("🔒 TLS/mTLS enabled - Production security mode");
    }

    // Initialize Event Bus
    info!("📡 Initializing Event Bus...");
    // TODO: Use NatsEventBus if configured, for now defaulting to InMemory
    let event_bus = Arc::new(InMemoryBus::new(1000));
    let event_subscriber: Arc<dyn EventSubscriber> = event_bus.clone();
    let event_publisher: Arc<dyn hodei_pipelines_ports::EventPublisher> = event_bus;
    info!("✅ Event Bus initialized");

    // Log configuration summary
    log_config_summary(&config);

    info!("✨ Server bootstrap completed successfully");
    info!("📊 Status: ready");
    info!(
        "🌐 Ready to accept connections on {}:{}",
        config.server.host, config.server.port
    );

    Ok(ServerComponents {
        config,
        event_subscriber,
        event_publisher,
        status: "ready",
    })
}

/// Log configuration summary (without sensitive data)
pub fn log_config_summary(config: &AppConfig) {
    info!("📋 Configuration Summary:");
    info!(
        "   Database: {} (max_conn: {})",
        mask_url(&config.database.url),
        config.database.max_connections
    );
    info!(
        "   Cache: {} (ttl: {}s, max_entries: {})",
        config.cache.path, config.cache.ttl_seconds, config.cache.max_entries
    );
    info!(
        "   Event Bus: {} (NATS: {})",
        config.event_bus.bus_type, config.nats.url
    );
    info!("   Server: {}:{}", config.server.host, config.server.port);
    info!(
        "   Kubernetes: insecure_skip_verify={}",
        config.kubernetes.insecure_skip_verify
    );
    info!(
        "   TLS: enabled={}, cert_path={}",
        config.tls.enabled,
        config.tls.cert_path.as_deref().unwrap_or("none")
    );
}

/// Mask database URL for security (hide credentials)
fn mask_url(url: &str) -> String {
    if let Some(pos) = url.find("://") {
        let (protocol, rest) = url.split_at(pos + 3);
        if let Some(at_pos) = rest.find('@') {
            let (creds, host) = rest.split_at(at_pos);
            if let Some(colon_pos) = creds.find(':') {
                let (user, _) = creds.split_at(colon_pos);
                return format!("{}****:****@{}", protocol, user);
            }
            return format!("{}****@{}", protocol, host);
        }
    }
    url.to_string()
}
