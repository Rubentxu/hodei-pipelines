//! Provider Configuration Value Object

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provider configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub endpoint: String,
    pub credentials: HashMap<String, String>,
    pub region: Option<String>,
    pub timeout_seconds: Option<u64>,
}

impl ProviderConfig {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            credentials: HashMap::new(),
            region: None,
            timeout_seconds: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_provider_config_creation() {
        let config = ProviderConfig::new("http://localhost:2375".to_string());

        assert_eq!(config.endpoint, "http://localhost:2375");
        assert!(config.credentials.is_empty());
        assert!(config.region.is_none());
        assert!(config.timeout_seconds.is_none());
    }

    #[test]
    fn test_provider_config_with_credentials() {
        let mut config = ProviderConfig::new("http://localhost:2375".to_string());
        config
            .credentials
            .insert("username".to_string(), "admin".to_string());
        config
            .credentials
            .insert("password".to_string(), "secret".to_string());

        assert_eq!(config.endpoint, "http://localhost:2375");
        assert_eq!(config.credentials.len(), 2);
        assert_eq!(
            config.credentials.get("username"),
            Some(&"admin".to_string())
        );
        assert_eq!(
            config.credentials.get("password"),
            Some(&"secret".to_string())
        );
    }

    #[test]
    fn test_provider_config_with_region_and_timeout() {
        let mut config = ProviderConfig::new("http://localhost:2375".to_string());
        config.region = Some("us-east-1".to_string());
        config.timeout_seconds = Some(60);

        assert_eq!(config.region, Some("us-east-1".to_string()));
        assert_eq!(config.timeout_seconds, Some(60));
    }
}
