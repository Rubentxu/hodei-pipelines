//! Tests for server bootstrap

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_pipelines_adapters::config::{AppConfig, CacheConfig, DatabaseConfig, EventBusConfig};

    #[test]
    fn test_mask_url_hides_credentials() {
        let url = "postgresql://user:password@localhost:5432/db";
        let masked = mask_url(url);
        assert!(!masked.contains("password"));
        assert!(masked.contains("user"));
        assert!(masked.contains("localhost:5432/db"));
    }

    #[test]
    fn test_mask_url_with_no_credentials() {
        let url = "postgresql://localhost:5432/db";
        let masked = mask_url(url);
        assert_eq!(masked, url);
    }

    #[test]
    fn test_mask_url_with_user_only() {
        let url = "postgresql://user@localhost:5432/db";
        let masked = mask_url(url);
        assert!(!masked.contains("@"));
        assert!(masked.contains("user"));
        assert!(masked.contains("localhost:5432/db"));
    }

    #[test]
    fn test_health_check_all_healthy() {
        let checker = HealthChecker {
            config_loaded: true,
            database_connected: true,
            cache_initialized: true,
            event_bus_connected: true,
        };

        assert!(checker.check_health());
        assert_eq!(checker.status(), "healthy");
    }

    #[test]
    fn test_health_check_missing_components() {
        let checker = HealthChecker {
            config_loaded: true,
            database_connected: false,
            cache_initialized: true,
            event_bus_connected: true,
        };

        assert!(!checker.check_health());
        assert_eq!(checker.status(), "unhealthy");
    }

    #[test]
    fn test_log_config_summary_does_not_panic() {
        let config = AppConfig {
            database: DatabaseConfig {
                url: "postgresql://test:test@localhost:5432/hodei".to_string(),
                max_connections: 20,
                connection_timeout_ms: 30000,
            },
            cache: CacheConfig {
                path: "/tmp/test_cache.redb".to_string(),
                ttl_seconds: 3600,
                max_entries: 10000,
            },
            event_bus: EventBusConfig {
                bus_type: "nats".to_string(),
            },
            kubernetes: hodei_pipelines_adapters::config::K8sGlobalConfig {
                insecure_skip_verify: false,
                ca_cert_path: None,
                token: None,
            },
            nats: hodei_pipelines_adapters::config::NatsConfig {
                url: "nats://localhost:4222".to_string(),
                subject_prefix: "hodei.events".to_string(),
                connection_timeout_ms: 5000,
            },
            agent: hodei_pipelines_adapters::config::AgentConfig {
                image: "hodei/hwp-agent:latest".to_string(),
                pull_policy: "IfNotPresent".to_string(),
            },
            server: hodei_pipelines_adapters::config::ServerConfig {
                port: 8080,
                host: "0.0.0.0".to_string(),
            },
            logging: hodei_pipelines_adapters::config::LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            tls: hodei_pipelines_adapters::config::TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
        };

        // This should not panic
        log_config_summary(&config);
    }
}
