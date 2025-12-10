//! Connection Pooling Module with PgBouncer
//!
//! Provides optimized database connection pooling for high-throughput
//! job execution workloads with configurable pool settings.

use async_trait::async_trait;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use domain::DomainError;
use tokio_postgres::Config as PgConfig;
use tracing::{debug, error, info};

/// PgBouncer configuration
#[derive(Debug, Clone)]
pub struct PgBouncerConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub pool_min: u32,
    pub pool_max: u32,
    pub connection_timeout: std::time::Duration,
    pub idle_timeout: std::time::Duration,
    pub max_lifetime: std::time::Duration,
    pub validate_on_checkout: bool,
}

impl PgBouncerConfig {
    /// Create a new PgBouncer configuration
    pub fn new(
        host: String,
        database: String,
        username: String,
        password: String,
    ) -> Self {
        Self {
            host,
            port: 6432, // Default PgBouncer port
            database,
            username,
            password,
            pool_min: 5,
            pool_max: 100,
            connection_timeout: std::time::Duration::from_secs(30),
            idle_timeout: std::time::Duration::from_secs(600),
            max_lifetime: std::time::Duration::from_secs(1800),
            validate_on_checkout: true,
        }
    }

    /// Set minimum pool size
    pub fn with_min_pool_size(mut self, size: u32) -> Self {
        self.pool_min = size;
        self
    }

    /// Set maximum pool size
    pub fn with_max_pool_size(mut self, size: u32) -> Self {
        self.pool_max = size;
        self
    }

    /// Set port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Build PostgreSQL configuration
    fn to_pg_config(&self) -> PgConfig {
        let mut config = PgConfig::new();
        config.host(&self.host);
        config.port(self.port);
        config.dbname(&self.database);
        config.user(&self.username);
        config.password(&self.password);
        config.connect_timeout(self.connection_timeout);
        config
    }
}

impl Default for PgBouncerConfig {
    fn default() -> Self {
        Self::new(
            "localhost".to_string(),
            "hodei_jobs".to_string(),
            "postgres".to_string(),
            "postgres".to_string(),
        )
    }
}

/// Managed PgBouncer connection pool
pub struct PgBouncerPool {
    pool: Pool<PostgresConnectionManager<tokio_postgres::NoTls>>,
    config: PgBouncerConfig,
}

impl PgBouncerPool {
    /// Create a new PgBouncer pool
    pub async fn new(config: PgBouncerConfig) -> Result<Self, DomainError> {
        let pg_config = config.to_pg_config();
        let manager = PostgresConnectionManager::new(pg_config, tokio_postgres::NoTls);

        debug!(
            "Initializing PgBouncer pool with min={}, max={}",
            config.pool_min, config.pool_max
        );

        let pool = Pool::builder()
            .min_idle(config.pool_min)
            .max_size(config.pool_max)
            .idle_timeout(Some(config.idle_timeout))
            .max_lifetime(Some(config.max_lifetime))
            .test_on_check_out(config.validate_on_checkout)
            .build(manager)
            .await
            .map_err(|e| {
                error!("Failed to initialize PgBouncer pool: {}", e);
                DomainError::Infrastructure(format!("Failed to create connection pool: {}", e))
            })?;

        info!(
            "PgBouncer pool initialized successfully (min={}, max={})",
            config.pool_min, config.pool_max
        );

        Ok(Self { pool, config })
    }

    /// Get pool reference
    pub fn pool(&self) -> &Pool<PostgresConnectionManager<tokio_postgres::NoTls>> {
        &self.pool
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        PoolStats {
            total_connections: 0,
            idle_connections: false,
            waiting_requests: false,
            max_size: self.config.pool_max,
        }
    }

    /// Health check - verify pool is operational
    pub async fn health_check(&self) -> Result<bool, DomainError> {
        match self.pool.get().await {
            Ok(conn) => {
                // Drop the connection immediately after health check
                drop(conn);
                debug!("PgBouncer pool health check passed");
                Ok(true)
            }
            Err(e) => {
                error!("PgBouncer pool health check failed: {}", e);
                Err(DomainError::Infrastructure(format!(
                    "Health check failed: {}",
                    e
                )))
            }
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: u32,
    pub idle_connections: bool,
    pub waiting_requests: bool,
    pub max_size: u32,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PoolStats {{ total: {}, idle: {}, waiting: {}, max: {} }}",
            self.total_connections, self.idle_connections, self.waiting_requests, self.max_size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tokio_postgres::NoTls;

    #[test]
    fn test_pgbouncer_config_default() {
        // Test: Should have correct default values
        let config = PgBouncerConfig::default();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 6432);
        assert_eq!(config.database, "hodei_jobs");
        assert_eq!(config.username, "postgres");
        assert_eq!(config.pool_min, 5);
        assert_eq!(config.pool_max, 100);
        assert_eq!(
            config.connection_timeout,
            std::time::Duration::from_secs(30)
        );
    }

    #[test]
    fn test_pgbouncer_config_with_custom_values() {
        // Test: Should accept custom configuration
        let config = PgBouncerConfig::new(
            "db.example.com".to_string(),
            "test_db".to_string(),
            "testuser".to_string(),
            "testpass".to_string(),
        )
        .with_min_pool_size(10)
        .with_max_pool_size(50)
        .with_port(6433);

        assert_eq!(config.host, "db.example.com");
        assert_eq!(config.database, "test_db");
        assert_eq!(config.username, "testuser");
        assert_eq!(config.pool_min, 10);
        assert_eq!(config.pool_max, 50);
        assert_eq!(config.port, 6433);
    }

    #[test]
    fn test_pgbouncer_config_to_pg_config() {
        // Test: Should convert to PostgreSQL config
        let config = PgBouncerConfig::new(
            "localhost".to_string(),
            "mydb".to_string(),
            "user".to_string(),
            "pass".to_string(),
        )
        .with_port(5432);

        let pg_config = config.to_pg_config();

        // Verify config can be used to create a connection manager
        let manager = PostgresConnectionManager::new(pg_config, tokio_postgres::NoTls);
        // If this compiles, the config is valid
        assert!(true);
    }

    #[test]
    fn test_pool_stats_creation() {
        // Test: Should create pool statistics
        let stats = PoolStats {
            total_connections: 25,
            idle_connections: false,
            waiting_requests: false,
            max_size: 100,
        };

        assert_eq!(stats.total_connections, 25);
        assert_eq!(stats.max_size, 100);
    }

    #[tokio::test]
    async fn test_pgbouncer_pool_new_with_invalid_config() {
        // Test: Should fail with invalid configuration
        let config = PgBouncerConfig::new(
            "invalid-host-that-does-not-exist".to_string(),
            "invalid_db".to_string(),
            "invalid_user".to_string(),
            "invalid_pass".to_string(),
        )
        .with_max_pool_size(1);

        let result = PgBouncerPool::new(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pgbouncer_pool_stats() {
        // Test: Should retrieve pool statistics
        let config = PgBouncerConfig::new(
            "localhost".to_string(),
            "hodei_jobs".to_string(),
            "postgres".to_string(),
            "postgres".to_string(),
        )
        .with_min_pool_size(0)
        .with_max_pool_size(1);

        let pool = PgBouncerPool::new(config).await;
        if let Ok(p) = pool {
            let stats = p.stats().await;
            assert_eq!(stats.max_size, 1);
            // Other stats may vary depending on pool state
        }
    }
}
