//! Event Bus Configuration
//!
//! Configuration and factory for selecting between different event bus implementations
//!
//! # Breaking Change: NATS JetStream is now the default event bus
//!
//! Previously, the default event bus was InMemory. This has been changed to NATS JetStream
//! for production readiness. To continue using InMemory (for testing or development), explicitly
//! set `bus_type: EventBusType::InMemory` in your configuration.
//!
//! Example:
//! ```rust
//! use hodei_adapters::bus::config::{EventBusConfig, EventBusType, NatsConfig};
//!
//! // For production (NATS JetStream - default)
//! let config = EventBusConfig::default(); // Uses Nats
//!
//! // For testing
//! let test_config = EventBusConfig {
//!     bus_type: EventBusType::InMemory,
//!     inmemory_capacity: 1000,
//!     nats_config: NatsConfig::default(),
//! };
//! ```

use serde::{Deserialize, Serialize};

/// Event bus type enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EventBusType {
    /// In-memory event bus (deprecated - for testing only)
    InMemory,
    /// NATS JetStream event bus (default for production)
    #[default]
    Nats,
}

impl std::str::FromStr for EventBusType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "inmemory" | "in-memory" | "memory" | "in_memory" => Ok(EventBusType::InMemory),
            "nats" | "jetstream" => Ok(EventBusType::Nats),
            _ => Err(format!(
                "Invalid event bus type: {}. Valid values are: inmemory, nats",
                s
            )),
        }
    }
}

impl std::fmt::Display for EventBusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventBusType::InMemory => write!(f, "inmemory"),
            EventBusType::Nats => write!(f, "nats"),
        }
    }
}

/// Event bus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusConfig {
    /// Type of event bus to use
    pub bus_type: EventBusType,
    /// Capacity for InMemory bus
    pub inmemory_capacity: usize,
    /// NATS configuration (used when bus_type is Nats)
    pub nats_config: NatsConfig,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            bus_type: EventBusType::Nats, // NATS JetStream is now the default
            inmemory_capacity: 10_000,
            nats_config: NatsConfig::default(),
        }
    }
}

/// NATS specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URL
    pub url: String,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            connection_timeout_ms: 5000,
        }
    }
}

/// Convert NatsConfig to NatsBusConfig from nats module
impl From<NatsConfig> for super::nats::NatsBusConfig {
    fn from(config: NatsConfig) -> Self {
        super::nats::NatsBusConfig {
            url: config.url,
            stream_name: "HODEI_EVENTS".to_string(),
            subject_prefix: "hodei.events".to_string(),
            max_messages: 100_000,
            connection_timeout_ms: config.connection_timeout_ms,
        }
    }
}
