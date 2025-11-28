//! Tests for unified application configuration

#[cfg(test)]
mod tests {
    use crate::config::{
        AgentConfig, AppConfig, CacheConfig, DatabaseConfig, EventBusConfig, K8sGlobalConfig,
        LoggingConfig, NatsConfig, ServerConfig, TlsConfig,
    };
    use serial_test::serial;

    fn cleanup_env_vars() {
        unsafe {
            let vars = [
                "HODEI_DB_URL",
                "HODEI_DB_MAX_CONNECTIONS",
                "HODEI_DB_TIMEOUT_MS",
                "HODEI_CACHE_PATH",
                "HODEI_CACHE_TTL_SECONDS",
                "HODEI_CACHE_MAX_ENTRIES",
                "HODEI_K8S_INSECURE_SKIP_VERIFY",
                "HODEI_K8S_CA_PATH",
                "HODEI_K8S_TOKEN",
                "HODEI_NATS_URL",
                "HODEI_NATS_SUBJECT_PREFIX",
                "HODEI_NATS_TIMEOUT_MS",
                "HODEI_AGENT_IMAGE",
                "HODEI_AGENT_PULL_POLICY",
                "HODEI_PORT",
                "HODEI_HOST",
                "HODEI_LOG_LEVEL",
                "HODEI_LOG_FORMAT",
                "HODEI_TLS_ENABLED",
                "HODEI_TLS_CERT_PATH",
                "HODEI_TLS_KEY_PATH",
                "HODEI_CONFIG_PATH",
                "HODEI_CONFIG_YAML",
            ];
            for var in &vars {
                std::env::remove_var(var);
            }
        }
    }

    #[test]
    #[serial]
    fn test_database_config_from_env() {
        cleanup_env_vars();

        unsafe {
            std::env::set_var(
                "HODEI_DB_URL",
                "postgresql://test:test@localhost:5432/testdb",
            );
            std::env::set_var("HODEI_DB_MAX_CONNECTIONS", "50");
            std::env::set_var("HODEI_DB_TIMEOUT_MS", "10000");
        }

        let config = DatabaseConfig::from_env().unwrap();

        assert_eq!(config.url, "postgresql://test:test@localhost:5432/testdb");
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.connection_timeout_ms, 10000);

        cleanup_env_vars();
    }

    #[test]
    fn test_database_config_validation() {
        let invalid_config = DatabaseConfig {
            url: "postgresql://localhost/db".to_string(),
            max_connections: 0,
            connection_timeout_ms: 5000,
        };

        assert!(invalid_config.validate().is_err());
    }

    #[test]
    #[serial]
    fn test_cache_config_from_env() {
        cleanup_env_vars();

        unsafe {
            std::env::set_var("HODEI_CACHE_PATH", "/tmp/test_cache.redb");
            std::env::set_var("HODEI_CACHE_TTL_SECONDS", "7200");
            std::env::set_var("HODEI_CACHE_MAX_ENTRIES", "50000");
        }

        let config = CacheConfig::from_env().unwrap();

        assert_eq!(config.path, "/tmp/test_cache.redb");
        assert_eq!(config.ttl_seconds, 7200);
        assert_eq!(config.max_entries, 50000);

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_k8s_config_from_env() {
        cleanup_env_vars();

        unsafe {
            std::env::set_var("HODEI_K8S_INSECURE_SKIP_VERIFY", "true");
            std::env::set_var("HODEI_K8S_CA_PATH", "/path/to/ca.pem");
            std::env::set_var("HODEI_K8S_TOKEN", "token123");
        }

        let config = K8sGlobalConfig::from_env().unwrap();

        assert!(config.insecure_skip_verify);
        assert_eq!(config.ca_cert_path, Some("/path/to/ca.pem".to_string()));
        assert_eq!(config.token, Some("token123".to_string()));

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_nats_config_from_env() {
        cleanup_env_vars();

        unsafe {
            std::env::set_var("HODEI_NATS_URL", "nats://nats.example.com:4222");
            std::env::set_var("HODEI_NATS_SUBJECT_PREFIX", "custom.events");
            std::env::set_var("HODEI_NATS_TIMEOUT_MS", "10000");
        }

        let config = NatsConfig::from_env().unwrap();

        assert_eq!(config.url, "nats://nats.example.com:4222");
        assert_eq!(config.subject_prefix, "custom.events");
        assert_eq!(config.connection_timeout_ms, 10000);

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_agent_config_from_env() {
        cleanup_env_vars();

        unsafe {
            std::env::set_var("HODEI_AGENT_IMAGE", "custom/agent:v1.0.0");
            std::env::set_var("HODEI_AGENT_PULL_POLICY", "Always");
        }

        let config = AgentConfig::from_env().unwrap();

        assert_eq!(config.image, "custom/agent:v1.0.0");
        assert_eq!(config.pull_policy, "Always");

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_server_config_from_env() {
        cleanup_env_vars();

        unsafe {
            std::env::set_var("HODEI_PORT", "9090");
            std::env::set_var("HODEI_HOST", "127.0.0.1");
        }

        let config = ServerConfig::from_env().unwrap();

        assert_eq!(config.port, 9090);
        assert_eq!(config.host, "127.0.0.1");

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_tls_config_from_env() {
        cleanup_env_vars();

        unsafe {
            std::env::set_var("HODEI_TLS_ENABLED", "true");
            std::env::set_var("HODEI_TLS_CERT_PATH", "/path/to/cert.pem");
            std::env::set_var("HODEI_TLS_KEY_PATH", "/path/to/key.pem");
        }

        let config = TlsConfig::from_env().unwrap();

        assert!(config.enabled);
        assert_eq!(config.cert_path, Some("/path/to/cert.pem".to_string()));
        assert_eq!(config.key_path, Some("/path/to/key.pem".to_string()));

        cleanup_env_vars();
    }

    #[test]
    fn test_tls_config_validation() {
        let invalid_config = TlsConfig {
            enabled: true,
            cert_path: None,
            key_path: None,
        };

        assert!(invalid_config.validate().is_err());
    }

    #[test]
    #[serial]
    fn test_app_config_load_from_env() {
        cleanup_env_vars();

        // Set test values
        unsafe {
            std::env::set_var(
                "HODEI_DB_URL",
                "postgresql://test:test@localhost:5432/hodei",
            );
            std::env::set_var("HODEI_CACHE_PATH", "/tmp/hodei_cache.redb");
            std::env::set_var("HODEI_K8S_INSECURE_SKIP_VERIFY", "false");
            std::env::set_var("HODEI_NATS_URL", "nats://localhost:4222");
            std::env::set_var("HODEI_NATS_SUBJECT_PREFIX", "hodei.events");
            std::env::set_var("HODEI_AGENT_IMAGE", "hodei/hwp-agent:latest");
            std::env::set_var("HODEI_PORT", "8080");
            std::env::set_var("HODEI_HOST", "0.0.0.0");
            std::env::set_var("HODEI_LOG_LEVEL", "info");
            std::env::set_var("HODEI_LOG_FORMAT", "json");
            std::env::set_var("HODEI_TLS_ENABLED", "false");
        }

        let config = AppConfig::load().unwrap();

        assert_eq!(
            config.database.url,
            "postgresql://test:test@localhost:5432/hodei"
        );
        assert_eq!(config.cache.path, "/tmp/hodei_cache.redb");
        assert_eq!(config.nats.url, "nats://localhost:4222");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.tls.enabled, false);

        cleanup_env_vars();
    }

    #[test]
    fn test_app_config_validation() {
        let mut config = AppConfig {
            database: DatabaseConfig {
                url: "postgresql://localhost/db".to_string(),
                max_connections: 10,
                connection_timeout_ms: 5000,
            },
            cache: CacheConfig {
                path: "/tmp/cache.redb".to_string(),
                ttl_seconds: 3600,
                max_entries: 1000,
            },
            event_bus: EventBusConfig {
                bus_type: "nats".to_string(),
            },
            kubernetes: K8sGlobalConfig {
                insecure_skip_verify: false,
                ca_cert_path: None,
                token: None,
            },
            nats: NatsConfig {
                url: "nats://localhost:4222".to_string(),
                subject_prefix: "hodei.events".to_string(),
                connection_timeout_ms: 5000,
            },
            agent: AgentConfig {
                image: "hodei/hwp-agent:latest".to_string(),
                pull_policy: "IfNotPresent".to_string(),
            },
            server: ServerConfig {
                port: 8080,
                host: "0.0.0.0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
        };

        assert!(config.validate().is_ok());

        // Test invalid config
        config.database.max_connections = 0;
        assert!(config.validate().is_err());
    }
}
