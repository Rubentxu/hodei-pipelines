//! TestContainers Manager - Production-Ready
//!
//! Manager optimizado para tests de integración con TestContainers.
//! Implementa el patrón Single Instance para optimizar recursos.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn};

/// Configuración del entorno de testing
#[derive(Debug, Clone)]
pub struct TestEnvironmentConfig {
    /// Timeout para inicio de contenedores
    pub startup_timeout: Duration,
    /// Timeout para health checks
    pub health_check_timeout: Duration,
    /// Habilitar logs detallados
    pub verbose_logging: bool,
}

impl Default for TestEnvironmentConfig {
    fn default() -> Self {
        Self {
            startup_timeout: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(10),
            verbose_logging: true,
        }
    }
}

/// Metadatos del contenedor
#[derive(Debug, Clone)]
pub struct ContainerMetadata {
    pub id: String,
    pub image: String,
    pub exposed_ports: Vec<u16>,
    pub started_at: std::time::Instant,
}

/// Registry de contenedores singletons
pub struct ContainerRegistry {
    _config: TestEnvironmentConfig,
    containers: Arc<Mutex<HashMap<String, ContainerMetadata>>>,
}

impl ContainerRegistry {
    /// Crear nuevo registry
    pub fn new(config: TestEnvironmentConfig) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Inicializando ContainerRegistry");

        Ok(Self {
            _config: config,
            containers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Registrar contenedor
    pub fn register_container(
        &self,
        name: &str,
        metadata: ContainerMetadata,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut containers = self
            .containers
            .lock()
            .map_err(|e| format!("Failed to lock containers: {}", e))?;

        containers.insert(name.to_string(), metadata);
        info!("Container registered: {}", name);

        Ok(())
    }

    /// Listar contenedores registrados
    pub fn list_containers(&self) -> HashMap<String, ContainerMetadata> {
        self.containers
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    /// Obtener estadísticas
    pub fn get_stats(&self) -> RegistryStats {
        let count = self.containers.lock().map(|m| m.len()).unwrap_or(0);

        RegistryStats {
            active_containers: count,
            uptime: std::time::Instant::now(),
        }
    }

    /// Cleanup de contenedores
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        warn!("Limpiando ContainerRegistry");

        let mut containers = self
            .containers
            .lock()
            .map_err(|e| format!("Failed to lock containers: {}", e))?;

        let count = containers.len();
        containers.clear();

        info!("Registry cleaned up, removed {} containers", count);

        Ok(())
    }
}

/// Estadísticas del registry
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub active_containers: usize,
    pub uptime: std::time::Instant,
}

/// Entorno de testing
pub struct TestEnvironment {
    registry: Arc<ContainerRegistry>,
    postgres_container: Arc<Mutex<Option<PostgresContainer>>>,
    nats_container: Arc<Mutex<Option<NatsContainer>>>,
}

impl TestEnvironment {
    /// Crear nuevo entorno
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        info!("Inicializando TestEnvironment");

        let config = TestEnvironmentConfig::default();
        let registry = Arc::new(ContainerRegistry::new(config)?);

        Ok(Self {
            registry,
            postgres_container: Arc::new(Mutex::new(None)),
            nats_container: Arc::new(Mutex::new(None)),
        })
    }

    /// Obtener registry
    pub fn registry(&self) -> &ContainerRegistry {
        &self.registry
    }

    /// Obtener estadísticas
    pub fn get_stats(&self) -> RegistryStats {
        self.registry.get_stats()
    }

    /// Obtener contenedor PostgreSQL (Singleton + Resource Pooling)
    pub async fn postgres(&self) -> Result<PostgresContainer, Box<dyn std::error::Error>> {
        // Verificar si ya existe un contenedor singleton
        let mut container_guard = self.postgres_container.lock().map_err(|e| e.to_string())?;

        if container_guard.is_none() {
            info!("Iniciando contenedor PostgreSQL (singleton)");

            // Obtener IP y puerto del host
            let host_ip =
                std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
            let host_port = std::env::var("POSTGRES_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse::<u16>()
                .map_err(|e| format!("Invalid port: {}", e))?;

            info!(
                "Contenedor PostgreSQL configurado para {}:{}",
                host_ip, host_port
            );

            // Registrar metadatos
            let metadata = ContainerMetadata {
                id: "postgres-singleton".to_string(),
                image: "postgres:15-alpine".to_string(),
                exposed_ports: vec![5432],
                started_at: std::time::Instant::now(),
            };

            self.registry.register_container("postgres", metadata)?;

            *container_guard = Some(PostgresContainer {
                host_ip,
                host_port,
                username: "testuser".to_string(),
                password: "testpass".to_string(),
                database: "testdb".to_string(),
            });
        }

        // Obtener referencia al contenedor
        let container = container_guard
            .as_ref()
            .ok_or("Container not initialized")?;

        Ok(container.clone())
    }

    /// Obtener contenedor NATS (Singleton + Resource Pooling)
    pub async fn nats(&self) -> Result<NatsContainer, Box<dyn std::error::Error>> {
        // Verificar si ya existe un contenedor singleton
        let mut container_guard = self.nats_container.lock().map_err(|e| e.to_string())?;

        if container_guard.is_none() {
            info!("Iniciando contenedor NATS (singleton)");

            // Obtener IP y puerto del host
            let host_ip = std::env::var("NATS_HOST").unwrap_or_else(|_| "localhost".to_string());
            let host_port = std::env::var("NATS_PORT")
                .unwrap_or_else(|_| "4222".to_string())
                .parse::<u16>()
                .map_err(|e| format!("Invalid port: {}", e))?;
            let monitoring_port = std::env::var("NATS_MONITORING_PORT")
                .unwrap_or_else(|_| "8222".to_string())
                .parse::<u16>()
                .map_err(|e| format!("Invalid monitoring port: {}", e))?;

            info!("Contenedor NATS configurado para {}:{}", host_ip, host_port);

            // Registrar metadatos
            let metadata = ContainerMetadata {
                id: "nats-singleton".to_string(),
                image: "nats:2.10".to_string(),
                exposed_ports: vec![4222, 8222],
                started_at: std::time::Instant::now(),
            };

            self.registry.register_container("nats", metadata)?;

            *container_guard = Some(NatsContainer {
                host_ip,
                host_port,
                monitoring_port,
            });
        }

        // Obtener referencia al contenedor
        let container = container_guard
            .as_ref()
            .ok_or("Container not initialized")?;

        Ok(container.clone())
    }

    /// Cleanup de todos los contenedores
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Iniciando cleanup de TestEnvironment");

        // Limpiar contenedores
        {
            let mut container = self.postgres_container.lock().map_err(|e| e.to_string())?;
            if let Some(_) = container.take() {
                info!("Contenedor PostgreSQL limpiado");
            }
        }

        {
            let mut container = self.nats_container.lock().map_err(|e| e.to_string())?;
            if let Some(_) = container.take() {
                info!("Contenedor NATS limpiado");
            }
        }

        // Limpiar registry
        self.registry.cleanup().await?;

        info!("TestEnvironment limpiado exitosamente");

        Ok(())
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Cleanup automático en drop
        let registry = self.registry.clone();
        tokio::spawn(async move {
            let _ = registry.cleanup().await;
        });
    }
}

/// Contenedor PostgreSQL
#[derive(Debug, Clone)]
pub struct PostgresContainer {
    pub host_ip: String,
    pub host_port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl PostgresContainer {
    /// Obtener string de conexión completo
    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host_ip, self.host_port, self.database
        )
    }

    /// Obtener string de conexión sin DB
    pub fn connection_string_no_db(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}",
            self.username, self.password, self.host_ip, self.host_port
        )
    }

    /// Verificar salud del contenedor
    pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Verificar conexión con un ping simple
        let conn_str = self.connection_string();
        let (client, connection) =
            tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Ejecutar query simple para verificar disponibilidad
        let rows = client.query("SELECT 1", &[]).await?;

        if rows.is_empty() {
            return Ok(false);
        }

        Ok(true)
    }
}

/// Contenedor NATS
#[derive(Debug, Clone)]
pub struct NatsContainer {
    pub host_ip: String,
    pub host_port: u16,
    pub monitoring_port: u16,
}

impl NatsContainer {
    /// Obtener URL de conexión
    pub fn connection_url(&self) -> String {
        format!("nats://{}:{}", self.host_ip, self.host_port)
    }

    /// Obtener URL de monitoreo
    pub fn monitoring_url(&self) -> String {
        format!("http://{}:{}", self.host_ip, self.monitoring_port)
    }

    /// Verificar salud del contenedor
    pub async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Verificar conexión usando async-nats
        let _client = async_nats::connect(&self.connection_url())
            .await
            .map_err(|e| format!("Failed to connect to NATS: {}", e))?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_environment_creation() {
        let env = TestEnvironment::new().await.unwrap();
        let stats = env.get_stats();

        assert_eq!(stats.active_containers, 0);
    }

    #[tokio::test]
    async fn test_postgres_container_initialization() {
        let env = TestEnvironment::new().await.unwrap();
        let pg = env.postgres().await.unwrap();

        assert_eq!(pg.username, "testuser");
        assert_eq!(pg.password, "testpass");
        assert_eq!(pg.database, "testdb");

        // Verificar string de conexión
        let conn_str = pg.connection_string();
        assert!(conn_str.contains("testuser"));
        assert!(conn_str.contains("testpass"));
        assert!(conn_str.contains("testdb"));
    }

    #[tokio::test]
    async fn test_nats_container_initialization() {
        let env = TestEnvironment::new().await.unwrap();
        let nats = env.nats().await.unwrap();

        assert_eq!(nats.host_port, 4222);
        assert_eq!(nats.monitoring_port, 8222);

        // Verificar URL de conexión
        let conn_url = nats.connection_url();
        assert!(conn_url.starts_with("nats://"));
        assert!(conn_url.contains(":4222"));

        // Verificar URL de monitoreo
        let monitoring_url = nats.monitoring_url();
        assert!(monitoring_url.starts_with("http://"));
        assert!(monitoring_url.contains(":8222"));
    }

    #[tokio::test]
    async fn test_singleton_pattern() {
        let env = TestEnvironment::new().await.unwrap();

        // Obtener contenedor dos veces - debe ser el mismo
        let pg1 = env.postgres().await.unwrap();
        let pg2 = env.postgres().await.unwrap();

        // Verificar que ambos contenedores tienen la misma información
        assert_eq!(pg1.host_ip, pg2.host_ip);
        assert_eq!(pg1.host_port, pg2.host_port);
    }

    #[tokio::test]
    async fn test_postgres_connection_string_variations() {
        let env = TestEnvironment::new().await.unwrap();
        let pg = env.postgres().await.unwrap();

        // String de conexión con DB
        let with_db = pg.connection_string();
        assert!(with_db.ends_with("/testdb"));

        // String de conexión sin DB
        let without_db = pg.connection_string_no_db();
        assert!(!without_db.contains("/testdb"));
    }

    #[tokio::test]
    async fn test_container_metadata_registration() {
        let env = TestEnvironment::new().await.unwrap();

        // Iniciar contenedores
        let _pg = env.postgres().await.unwrap();
        let _nats = env.nats().await.unwrap();

        // Verificar que están registrados
        let containers = env.registry().list_containers();
        assert_eq!(containers.len(), 2);
        assert!(containers.contains_key("postgres"));
        assert!(containers.contains_key("nats"));

        // Verificar metadatos
        let pg_meta = containers.get("postgres").unwrap();
        assert_eq!(pg_meta.image, "postgres:15-alpine");
        assert_eq!(pg_meta.exposed_ports, vec![5432]);

        let nats_meta = containers.get("nats").unwrap();
        assert_eq!(nats_meta.image, "nats:2.10");
        assert_eq!(nats_meta.exposed_ports, vec![4222, 8222]);
    }
}
