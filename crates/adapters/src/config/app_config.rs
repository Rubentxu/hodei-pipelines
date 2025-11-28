//! Unified Application Configuration
//!
//! This module provides a centralized configuration structure for the entire application,
//! following the Configuration Port pattern from DDD architecture.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unified application configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// Database configuration
    pub database: DatabaseConfig,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Event bus configuration
    pub event_bus: EventBusConfig,

    /// Kubernetes provider configuration
    pub kubernetes: K8sGlobalConfig,

    /// NATS configuration
    pub nats: NatsConfig,

    /// Worker configuration
    pub worker: WorkerConfig,

    /// Scaling configuration
    pub scaling: ScalingConfig,

    /// Worker agent configuration
    pub agent: AgentConfig,

    /// Server configuration
    pub server: ServerConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// TLS/mTLS configuration for production
    pub tls: TlsConfig,
}

impl AppConfig {
    /// Load configuration from environment and file
    pub fn load() -> Result<Self> {
        let config = match (
            std::env::var("HODEI_CONFIG_PATH").ok(),
            std::env::var("HODEI_CONFIG_YAML").ok(),
        ) {
            (Some(path), None) => {
                // Load from file path
                let path = PathBuf::from(path);
                if !path.exists() {
                    return Err(ConfigError::FileNotFound(path));
                }
                let content = std::fs::read_to_string(&path).map_err(ConfigError::FileRead)?;
                serde_yaml::from_str(&content).map_err(ConfigError::ParseYaml)?
            }
            (None, Some(yaml)) => {
                // Load from inline YAML
                serde_yaml::from_str(&yaml).map_err(ConfigError::ParseYaml)?
            }
            _ => {
                // Load from environment variables
                Self::from_env()?
            }
        };

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database: DatabaseConfig::from_env()?,
            cache: CacheConfig::from_env()?,
            event_bus: EventBusConfig::from_env()?,
            kubernetes: K8sGlobalConfig::from_env()?,
            nats: NatsConfig::from_env()?,
            worker: WorkerConfig::from_env()?,
            scaling: ScalingConfig::from_env()?,
            agent: AgentConfig::from_env()?,
            server: ServerConfig::from_env()?,
            logging: LoggingConfig::from_env()?,
            tls: TlsConfig::from_env()?,
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        self.database.validate()?;
        self.cache.validate()?;
        self.worker.validate()?;
        self.scaling.validate()?;
        self.kubernetes.validate()?;
        self.tls.validate()?;

        Ok(())
    }
}

/// Database configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,

    /// Maximum number of connections
    pub max_connections: u32,

    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self> {
        let url = std::env::var("HODEI_DB_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/hodei".to_string());

        let max_connections = std::env::var("HODEI_DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "20".to_string())
            .parse::<u32>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_DB_MAX_CONNECTIONS".to_string()))?;

        let connection_timeout_ms = std::env::var("HODEI_DB_TIMEOUT_MS")
            .unwrap_or_else(|_| "30000".to_string())
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_DB_TIMEOUT_MS".to_string()))?;

        Ok(Self {
            url,
            max_connections,
            connection_timeout_ms,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.max_connections == 0 {
            return Err(ConfigError::InvalidValue(
                "max_connections must be > 0".to_string(),
            ));
        }
        if !self.url.starts_with("postgresql://") {
            return Err(ConfigError::InvalidValue(
                "database URL must be PostgreSQL".to_string(),
            ));
        }
        Ok(())
    }
}

/// Cache configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// Redb cache file path
    pub path: String,

    /// Time-to-live for cache entries (seconds)
    pub ttl_seconds: u64,

    /// Maximum number of entries in cache
    pub max_entries: usize,
}

impl CacheConfig {
    pub fn from_env() -> Result<Self> {
        let path = std::env::var("HODEI_CACHE_PATH")
            .unwrap_or_else(|_| "/tmp/hodei_cache.redb".to_string());

        let ttl_seconds = std::env::var("HODEI_CACHE_TTL_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_CACHE_TTL_SECONDS".to_string()))?;

        let max_entries = std::env::var("HODEI_CACHE_MAX_ENTRIES")
            .unwrap_or_else(|_| "10000".to_string())
            .parse::<usize>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_CACHE_MAX_ENTRIES".to_string()))?;

        Ok(Self {
            path,
            ttl_seconds,
            max_entries,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.ttl_seconds == 0 {
            return Err(ConfigError::InvalidValue(
                "ttl_seconds must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Kubernetes global configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct K8sGlobalConfig {
    /// Whether to skip TLS verification (for development only)
    pub insecure_skip_verify: bool,

    /// Path to CA certificate
    pub ca_cert_path: Option<String>,

    /// Authentication token
    pub token: Option<String>,
}

impl K8sGlobalConfig {
    pub fn from_env() -> Result<Self> {
        let insecure_skip_verify = std::env::var("HODEI_K8S_INSECURE_SKIP_VERIFY")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_K8S_INSECURE_SKIP_VERIFY".to_string()))?;

        let ca_cert_path = std::env::var("HODEI_K8S_CA_PATH").ok();
        let token = std::env::var("HODEI_K8S_TOKEN").ok();

        Ok(Self {
            insecure_skip_verify,
            ca_cert_path,
            token,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.insecure_skip_verify {
            tracing::warn!("⚠️ Kubernetes TLS verification is disabled - FOR DEVELOPMENT ONLY");
        }
        Ok(())
    }
}

/// NATS configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsConfig {
    /// NATS server URL
    pub url: String,

    /// Subject prefix for topics
    pub subject_prefix: String,

    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
}

impl NatsConfig {
    pub fn from_env() -> Result<Self> {
        let url =
            std::env::var("HODEI_NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());

        let subject_prefix = std::env::var("HODEI_NATS_SUBJECT_PREFIX")
            .unwrap_or_else(|_| "hodei.events".to_string());

        let connection_timeout_ms = std::env::var("HODEI_NATS_TIMEOUT_MS")
            .unwrap_or_else(|_| "5000".to_string())
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_NATS_TIMEOUT_MS".to_string()))?;

        Ok(Self {
            url,
            subject_prefix,
            connection_timeout_ms,
        })
    }
}

/// Worker configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkerConfig {
    /// Worker timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum concurrent jobs per worker
    pub max_concurrent_jobs: u32,

    /// Worker heartbeat interval in seconds
    pub heartbeat_interval_seconds: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self> {
        let timeout_seconds = std::env::var("HODEI_WORKER_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_WORKER_TIMEOUT_SECONDS".to_string()))?;

        let max_concurrent_jobs = std::env::var("HODEI_WORKER_MAX_CONCURRENT_JOBS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .map_err(|_| {
                ConfigError::InvalidValue("HODEI_WORKER_MAX_CONCURRENT_JOBS".to_string())
            })?;

        let heartbeat_interval_seconds = std::env::var("HODEI_WORKER_HEARTBEAT_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|_| {
                ConfigError::InvalidValue("HODEI_WORKER_HEARTBEAT_INTERVAL_SECONDS".to_string())
            })?;

        Ok(Self {
            timeout_seconds,
            max_concurrent_jobs,
            heartbeat_interval_seconds,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.timeout_seconds == 0 {
            return Err(ConfigError::InvalidValue(
                "timeout_seconds must be > 0".to_string(),
            ));
        }
        if self.max_concurrent_jobs == 0 {
            return Err(ConfigError::InvalidValue(
                "max_concurrent_jobs must be > 0".to_string(),
            ));
        }
        if self.heartbeat_interval_seconds == 0 {
            return Err(ConfigError::InvalidValue(
                "heartbeat_interval_seconds must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Scaling configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScalingConfig {
    /// Scale cooldown period in seconds
    pub cooldown_period_seconds: u64,

    /// Minimum number of workers
    pub min_workers: u32,

    /// Maximum number of workers
    pub max_workers: u32,
}

impl ScalingConfig {
    pub fn from_env() -> Result<Self> {
        let cooldown_period_seconds = std::env::var("HODEI_SCALING_COOLDOWN_PERIOD_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .map_err(|_| {
                ConfigError::InvalidValue("HODEI_SCALING_COOLDOWN_PERIOD_SECONDS".to_string())
            })?;

        let min_workers = std::env::var("HODEI_SCALING_MIN_WORKERS")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u32>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_SCALING_MIN_WORKERS".to_string()))?;

        let max_workers = std::env::var("HODEI_SCALING_MAX_WORKERS")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u32>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_SCALING_MAX_WORKERS".to_string()))?;

        Ok(Self {
            cooldown_period_seconds,
            min_workers,
            max_workers,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.min_workers > self.max_workers {
            return Err(ConfigError::InvalidValue(
                "min_workers cannot be greater than max_workers".to_string(),
            ));
        }
        if self.cooldown_period_seconds == 0 {
            return Err(ConfigError::InvalidValue(
                "cooldown_period_seconds must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Worker agent configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Default agent image
    pub image: String,

    /// Image pull policy
    pub pull_policy: String,
}

impl AgentConfig {
    pub fn from_env() -> Result<Self> {
        let image = std::env::var("HODEI_AGENT_IMAGE")
            .unwrap_or_else(|_| "hodei/hwp-agent:latest".to_string());

        let pull_policy =
            std::env::var("HODEI_AGENT_PULL_POLICY").unwrap_or_else(|_| "IfNotPresent".to_string());

        Ok(Self { image, pull_policy })
    }
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Server port
    pub port: u16,

    /// Server host
    pub host: String,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        let port = std::env::var("HODEI_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_PORT".to_string()))?;

        let host = std::env::var("HODEI_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        Ok(Self { port, host })
    }
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,

    /// Log format
    pub format: String,
}

impl LoggingConfig {
    pub fn from_env() -> Result<Self> {
        let level = std::env::var("HODEI_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let format = std::env::var("HODEI_LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

        Ok(Self { level, format })
    }
}

/// TLS/mTLS configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,

    /// Path to TLS certificate
    pub cert_path: Option<String>,

    /// Path to TLS private key
    pub key_path: Option<String>,
}

impl TlsConfig {
    pub fn from_env() -> Result<Self> {
        let enabled = std::env::var("HODEI_TLS_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|_| ConfigError::InvalidValue("HODEI_TLS_ENABLED".to_string()))?;

        let cert_path = std::env::var("HODEI_TLS_CERT_PATH").ok();
        let key_path = std::env::var("HODEI_TLS_KEY_PATH").ok();

        Ok(Self {
            enabled,
            cert_path,
            key_path,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.enabled {
            if self.cert_path.is_none() || self.key_path.is_none() {
                return Err(ConfigError::InvalidValue(
                    "TLS enabled but cert_path or key_path not provided".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Event bus configuration (compatibility with existing EventBusConfig)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventBusConfig {
    /// Bus type (default: nats)
    pub bus_type: String,
}

impl EventBusConfig {
    pub fn from_env() -> Result<Self> {
        let bus_type = std::env::var("HODEI_EVENT_BUS_TYPE").unwrap_or_else(|_| "nats".to_string());

        Ok(Self { bus_type })
    }
}

/// Configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to read configuration file: {0}")]
    FileRead(std::io::Error),

    #[error("Failed to parse YAML configuration: {0}")]
    ParseYaml(serde_yaml::Error),

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),

    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;
