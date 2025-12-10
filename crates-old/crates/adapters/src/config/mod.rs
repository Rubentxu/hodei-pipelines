//! Configuration module
//!
//! Provides unified application configuration following DDD architecture.

pub mod app_config;
pub mod tests;

pub use app_config::{
    AgentConfig, AppConfig, CacheConfig, ConfigError, DatabaseConfig, EventBusConfig,
    K8sGlobalConfig, LoggingConfig, NatsConfig, Result as ConfigResult, ScalingConfig,
    ServerConfig, TlsConfig, WorkerConfig,
};
