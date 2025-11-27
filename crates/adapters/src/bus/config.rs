//! Event Bus Configuration
//!
//! Configuration and factory for selecting between different event bus implementations

use serde::{Deserialize, Serialize};

/// Event bus type enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventBusType {
    /// In-memory event bus (default)
    InMemory,
    /// NATS JetStream event bus
    Nats,
}

impl Default for EventBusType {
    fn default() -> Self {
        EventBusType::InMemory
    }
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
            bus_type: EventBusType::InMemory,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_type_default() {
        let bus_type = EventBusType::default();
        assert_eq!(bus_type, EventBusType::InMemory);
    }

    #[test]
    fn test_event_bus_type_display() {
        assert_eq!(EventBusType::InMemory.to_string(), "inmemory");
        assert_eq!(EventBusType::Nats.to_string(), "nats");
    }

    #[test]
    fn test_event_bus_config_default() {
        let config = EventBusConfig::default();
        assert_eq!(config.bus_type, EventBusType::InMemory);
        assert_eq!(config.inmemory_capacity, 10_000);
        assert_eq!(config.nats_config.url, "nats://localhost:4222");
    }

    #[test]
    fn test_event_bus_config_serialization() {
        let config = EventBusConfig {
            bus_type: EventBusType::Nats,
            inmemory_capacity: 5000,
            nats_config: NatsConfig {
                url: "nats://test:4222".to_string(),
                connection_timeout_ms: 10000,
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EventBusConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.bus_type, EventBusType::Nats);
        assert_eq!(deserialized.inmemory_capacity, 5000);
        assert_eq!(deserialized.nats_config.url, "nats://test:4222");
    }

    #[test]
    fn test_nats_config_default() {
        let config = NatsConfig::default();
        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.connection_timeout_ms, 5000);
    }

    #[test]
    fn test_event_bus_type_from_string() {
        assert_eq!(EventBusType::InMemory.to_string(), "inmemory");
        assert_eq!(EventBusType::Nats.to_string(), "nats");
    }
}
