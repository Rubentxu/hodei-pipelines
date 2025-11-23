//! Configuration management for HWP Agent

use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

/// Configuration error types
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required configuration: {0}")]
    Missing(String),

    #[error("Invalid configuration value: {0}")]
    Invalid(String),
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Worker identification
    pub worker_id: String,

    /// Server connection settings
    pub server_url: String,
    pub server_token: String,
    pub tls_enabled: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub tls_ca_path: Option<String>,

    /// Connection settings
    pub reconnect_initial_delay_ms: u64,
    pub reconnect_max_delay_ms: u64,
    pub connection_timeout_ms: u64,

    /// Execution settings
    pub job_timeout_ms: Option<u64>,
    pub max_log_buffer_size: usize,
    pub log_flush_interval_ms: u64,

    /// Monitoring settings
    pub resource_sampling_interval_ms: u64,

    /// Logging settings
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            worker_id: "worker-default".to_string(),
            server_url: "http://localhost:50051".to_string(),
            server_token: "".to_string(),
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            reconnect_initial_delay_ms: 1000,
            reconnect_max_delay_ms: 60000,
            connection_timeout_ms: 5000,
            job_timeout_ms: None,
            max_log_buffer_size: 4096,
            log_flush_interval_ms: 100,
            resource_sampling_interval_ms: 5000,
            log_level: "info".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Config::default();

        // Worker ID (optional, defaults to hostname)
        if let Ok(worker_id) = env::var("HODEI_WORKER_ID") {
            config.worker_id = worker_id;
        } else if let Ok(hostname) = hostname::get() {
            config.worker_id = hostname.to_string_lossy().to_string();
        }

        // Server URL (required)
        if let Ok(url) = env::var("HODEI_SERVER_URL") {
            config.server_url = url;
        }
        if let Ok(cert) = env::var("HODEI_TLS_CERT_PATH") {
            config.tls_cert_path = Some(cert);
        }
        if let Ok(key) = env::var("HODEI_TLS_KEY_PATH") {
            config.tls_key_path = Some(key);
        }
        if let Ok(ca) = env::var("HODEI_TLS_CA_PATH") {
            config.tls_ca_path = Some(ca);
        }

        // Server token (required)
        config.server_token =
            env::var("HODEI_TOKEN").map_err(|_| ConfigError::Missing("HODEI_TOKEN".to_string()))?;

        // TLS settings
        config.tls_enabled =
            env::var("HODEI_TLS_ENABLED").map_or(false, |v| v.to_lowercase() == "true");

        if config.tls_enabled {
            config.tls_cert_path = env::var("HODEI_TLS_CERT_PATH").ok();
        }

        // Connection settings
        if let Ok(delay) = env::var("HODEI_RECONNECT_INITIAL_DELAY_MS") {
            config.reconnect_initial_delay_ms = delay.parse().map_err(|_| {
                ConfigError::Invalid("HODEI_RECONNECT_INITIAL_DELAY_MS".to_string())
            })?;
        }

        // Job timeout
        if let Ok(timeout) = env::var("HODEI_JOB_TIMEOUT_MS") {
            config.job_timeout_ms = Some(
                timeout
                    .parse()
                    .map_err(|_| ConfigError::Invalid("HODEI_JOB_TIMEOUT_MS".to_string()))?,
            );
        }

        // Log buffer size
        if let Ok(size) = env::var("HODEI_LOG_BUFFER_SIZE") {
            config.max_log_buffer_size = size
                .parse()
                .map_err(|_| ConfigError::Invalid("HODEI_LOG_BUFFER_SIZE".to_string()))?;
        }

        // Resource sampling interval
        if let Ok(interval) = env::var("HODEI_RESOURCE_SAMPLING_INTERVAL_MS") {
            config.resource_sampling_interval_ms = interval.parse().map_err(|_| {
                ConfigError::Invalid("HODEI_RESOURCE_SAMPLING_INTERVAL_MS".to_string())
            })?;
        }

        // Log level
        if let Ok(level) = env::var("HODEI_LOG_LEVEL") {
            config.log_level = level;
        }

        // Validation
        if config.server_token.is_empty() {
            return Err(ConfigError::Missing("HODEI_TOKEN".to_string()));
        }

        if config.max_log_buffer_size == 0 {
            return Err(ConfigError::Invalid(
                "max_log_buffer_size cannot be 0".to_string(),
            ));
        }

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.server_url.is_empty() {
            return Err(ConfigError::Invalid(
                "server_url cannot be empty".to_string(),
            ));
        }

        if self.server_token.is_empty() {
            return Err(ConfigError::Invalid(
                "server_token cannot be empty".to_string(),
            ));
        }

        if self.max_log_buffer_size < 1024 {
            return Err(ConfigError::Invalid(
                "max_log_buffer_size should be at least 1024".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server_url, "http://localhost:50051");
        assert!(config.max_log_buffer_size > 0);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        // Default config may have empty token, so set it first
        config.server_token = "test-token".to_string();
        assert!(config.validate().is_ok());

        config.server_token = "".to_string();
        assert!(config.validate().is_err());

        config.server_token = "token".to_string();
        config.max_log_buffer_size = 0;
        assert!(config.validate().is_err());
    }
}
