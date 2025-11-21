//! Core traits y tipos para el Worker Manager abstraction layer
//!
//! Este módulo define las interfaces abstractas que permiten a diferentes
//! providers de infraestructura proporcionar servicios de gestión de workers
//! de forma uniforme para el sistema CI/CD distribuido.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Identificador único para un worker
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerId(pub Uuid);

impl WorkerId {
    /// Crea un nuevo WorkerId aleatorio
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WorkerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Especificación de recursos requeridos para un worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// Cantidad de CPU en mili-cores
    pub cpu_m: u64,
    /// Cantidad de memoria en megabytes
    pub memory_mb: u64,
    /// Número de GPUs (si es aplicable)
    pub gpu: Option<u8>,
    /// Almacenamiento en megabytes
    pub storage_mb: Option<u64>,
}

impl ResourceQuota {
    /// Crea una especificación básica de recursos
    pub fn basic(cpu_m: u64, memory_mb: u64) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: None,
            storage_mb: None,
        }
    }

    /// Crea una especificación para workers con GPU
    pub fn with_gpu(cpu_m: u64, memory_mb: u64, gpu: u8) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: Some(gpu),
            storage_mb: None,
        }
    }

    /// Verifica si los recursos son compatibles
    pub fn is_compatible_with(&self, available: &ResourceQuota) -> bool {
        self.cpu_m <= available.cpu_m &&
        self.memory_mb <= available.memory_mb &&
        self.gpu.map(|g| g <= available.gpu.unwrap_or(0)).unwrap_or(true) &&
        self.storage_mb.map(|s| s <= available.storage_mb.unwrap_or(0)).unwrap_or(true)
    }
}

/// Estados posibles de un worker
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerState {
    /// Worker está siendo creado
    Creating,
    /// Worker está ejecutándose
    Running,
    /// Worker está siendo terminado
    Terminating,
    /// Worker ha terminado exitosamente
    Terminated,
    /// Worker ha fallado
    Failed { reason: String },
    /// Worker está en pausa/suspendido
    Paused,
    /// Estado desconocido
    Unknown,
}

/// Especificación de configuración de runtime para un worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSpec {
    /// Imagen del contenedor o artefacto a ejecutar
    pub image: String,
    /// Comando a ejecutar (overrides default)
    pub command: Option<Vec<String>>,
    /// Variables de entorno
    pub env: HashMap<String, String>,
    /// Referencias a secretos
    pub secret_refs: Vec<String>,
    /// Puertos expuestos
    pub ports: Vec<u16>,
    /// Volúmenes montados
    pub volumes: Vec<VolumeMount>,
    /// Labels para metadatos
    pub labels: HashMap<String, String>,
    /// Recursos solicitados
    pub resources: ResourceQuota,
    /// Timeout de ejecución
    pub timeout: Option<Duration>,
    /// Dependencias de red
    pub network_requirements: NetworkRequirements,
}

impl RuntimeSpec {
    /// Crea una especificación básica
    pub fn basic(image: String) -> Self {
        Self {
            image,
            command: None,
            env: HashMap::new(),
            secret_refs: Vec::new(),
            ports: Vec::new(),
            volumes: Vec::new(),
            labels: HashMap::new(),
            resources: ResourceQuota::basic(1000, 1024),
            timeout: Some(Duration::from_secs(3600)),
            network_requirements: NetworkRequirements::default(),
        }
    }
}

/// Configuración de red para un worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Tipo de red requerida
    pub network_type: NetworkType,
    /// Políticas de acceso de red
    pub access_policies: Vec<NetworkPolicy>,
    /// Requisitos de aislamiento
    pub isolation_level: NetworkIsolation,
}

impl Default for NetworkRequirements {
    fn default() -> Self {
        Self {
            network_type: NetworkType::Private,
            access_policies: Vec::new(),
            isolation_level: NetworkIsolation::Default,
        }
    }
}

/// Tipos de red soportados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    /// Red privada interna
    Private,
    /// Red pública accesible
    Public,
    /// Red híbrida
    Hybrid,
    /// Aislamiento completo
    Isolated,
}

/// Políticas de red
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkPolicy {
    /// Permitir acceso a servicios específicos
    AllowServices(Vec<String>),
    /// Bloquear servicios específicos
    BlockServices(Vec<String>),
    /// Rango de IPs permitidas
    AllowIpRange(String),
    /// Requiere tunneling/VPN
    RequireTunnel,
}

/// Niveles de aislamiento de red
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkIsolation {
    /// Aislamiento por defecto del provider
    Default,
    /// Aislamiento fuerte (firewall adicional)
    Strict,
    /// Aislamiento total (sin conectividad externa)
    Complete,
}

/// Montaje de volumen para un worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Ruta dentro del contenedor
    pub mount_path: String,
    /// Fuente del volumen
    pub source: VolumeSource,
    /// Permisos de acceso
    pub read_only: bool,
}

/// Fuentes de volumen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolumeSource {
    /// Volumen temporal en memoria
    EmptyDir,
    /// Volumen persistente
    PersistentVolume(String),
    /// Directorio del host
    HostPath(String),
    /// Secrets como volumen
    Secret(String),
    /// ConfigMap como volumen
    ConfigMap(String),
}

/// Configuración específica de provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Nombre del provider (k8s, docker, ecs, etc.)
    pub provider_name: String,
    /// Configuración específica del provider
    pub specific_config: serde_json::Value,
    /// Regiones o zonas de disponibilidad
    pub regions: Vec<String>,
    /// Políticas de seguridad
    pub security_policies: SecurityPolicies,
    /// Límites de rate limiting
    pub rate_limits: RateLimits,
}

impl ProviderConfig {
    /// Crea configuración para Kubernetes
    pub fn kubernetes(namespace: String) -> Self {
        let mut config = HashMap::new();
        config.insert("namespace".to_string(), serde_json::Value::String(namespace));
        
        Self {
            provider_name: "kubernetes".to_string(),
            specific_config: serde_json::Value::Object(config.into()),
            regions: Vec::new(),
            security_policies: SecurityPolicies::default(),
            rate_limits: RateLimits::default(),
        }
    }

    /// Crea configuración para Docker
    pub fn docker() -> Self {
        Self {
            provider_name: "docker".to_string(),
            specific_config: serde_json::Value::Null,
            regions: Vec::new(),
            security_policies: SecurityPolicies::default(),
            rate_limits: RateLimits::default(),
        }
    }
}

/// Políticas de seguridad para el provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicies {
    /// Permisos RBAC requeridos
    pub rbac_permissions: Vec<String>,
    /// Service account requerida
    pub service_account: Option<String>,
    /// Políticas de seguridad del contenedor
    pub container_security: ContainerSecurity,
    /// Políticas de red
    pub network_policies: Vec<NetworkPolicy>,
}

impl Default for SecurityPolicies {
    fn default() -> Self {
        Self {
            rbac_permissions: Vec::new(),
            service_account: None,
            container_security: ContainerSecurity::default(),
            network_policies: Vec::new(),
        }
    }
}

/// Configuración de seguridad del contenedor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSecurity {
    /// Usuario para ejecutar el contenedor
    pub user: Option<u32>,
    /// Capacidades del contenedor
    pub capabilities: Vec<String>,
    /// Políticas de acceso a archivos
    pub security_context: SecurityContext,
    /// Configuración de secrets
    pub secrets_config: SecretsConfig,
}

impl Default for ContainerSecurity {
    fn default() -> Self {
        Self {
            user: None,
            capabilities: Vec::new(),
            security_context: SecurityContext::default(),
            secrets_config: SecretsConfig::default(),
        }
    }
}

/// Contexto de seguridad
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Modo de red del contenedor
    pub network_mode: String,
    /// Configuración de SELinux (si aplica)
    pub se_linux_options: Option<HashMap<String, String>>,
    /// Configuración de AppArmor (si aplica)
    pub app_armor_profile: Option<String>,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            network_mode: "default".to_string(),
            se_linux_options: None,
            app_armor_profile: None,
        }
    }
}

/// Configuración de secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Estrategia para montar secrets
    pub mount_strategy: SecretMountStrategy,
    /// Tiempo de vida de tokens
    pub token_ttl: Option<Duration>,
    /// Rotación automática
    pub auto_rotation: bool,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            mount_strategy: SecretMountStrategy::Environment,
            token_ttl: Some(Duration::from_secs(3600)),
            auto_rotation: true,
        }
    }
}

/// Estrategias para montar secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretMountStrategy {
    /// Como variables de entorno
    Environment,
    /// Como archivos montados
    Files,
    /// Como volumen
    Volume,
}

/// Límites de rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Máximo de workers por minuto
    pub workers_per_minute: Option<u32>,
    /// Máximo de operaciones concurrentes
    pub max_concurrent_operations: Option<u32>,
    /// Timeout por defecto
    pub default_timeout: Option<Duration>,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            workers_per_minute: Some(100),
            max_concurrent_operations: Some(50),
            default_timeout: Some(Duration::from_secs(30)),
        }
    }
}

/// Representación de un worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    /// Identificador único del worker
    pub id: WorkerId,
    /// Estado actual del worker
    pub state: WorkerState,
    /// Especificación de runtime utilizada
    pub runtime_spec: RuntimeSpec,
    /// Configuración del provider
    pub provider_config: ProviderConfig,
    /// Metadatos adicionales
    pub metadata: HashMap<String, String>,
    /// Fecha de creación
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Última actualización de estado
    pub last_update: chrono::DateTime<chrono::Utc>,
}

impl Worker {
    /// Crea un nuevo worker en estado Creating
    pub fn new(
        id: WorkerId,
        runtime_spec: RuntimeSpec,
        provider_config: ProviderConfig,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            state: WorkerState::Creating,
            runtime_spec,
            provider_config,
            metadata: HashMap::new(),
            created_at: now,
            last_update: now,
        }
    }

    /// Actualiza el estado del worker
    pub fn update_state(&mut self, new_state: WorkerState) {
        self.state = new_state;
        self.last_update = chrono::Utc::now();
    }

    /// Agrega metadatos al worker
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.last_update = chrono::Utc::now();
    }
}

/// Referencia a logs de un worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStreamRef {
    /// Identificador del stream de logs
    pub stream_id: Uuid,
    /// Worker asociado
    pub worker_id: WorkerId,
    /// Formato de los logs
    pub format: LogFormat,
    /// URLs o endpoints para acceder a los logs
    pub endpoints: Vec<String>,
}

/// Formatos de log soportados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    /// Texto plano
    Plain,
    /// JSON estructurado
    Json,
    /// Formato específico del servicio
    ServiceSpecific(String),
}

/// Evento de ciclo de vida de worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerEvent {
    /// Worker creado
    WorkerCreated {
        worker_id: WorkerId,
        provider_info: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Worker terminado
    WorkerTerminated {
        worker_id: WorkerId,
        exit_code: i32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Worker ha fallado
    WorkerFailed {
        worker_id: WorkerId,
        reason: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Estado del worker ha cambiado
    WorkerStateChanged {
        worker_id: WorkerId,
        old_state: WorkerState,
        new_state: WorkerState,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// Información de capacidad de recursos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityInfo {
    /// Recursos totales disponibles
    pub total_resources: ResourceQuota,
    /// Recursos utilizados actualmente
    pub used_resources: ResourceQuota,
    /// Recursos disponibles para nuevos workers
    pub available_resources: ResourceQuota,
    /// Número de workers activos
    pub active_workers: u32,
    /// Fecha de última actualización
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Interface unificada para todos los providers de infraestructura
#[async_trait]
pub trait WorkerManagerProvider: Send + Sync {
    /// Nombre del provider
    fn name(&self) -> &str;

    /// Crear un nuevo worker
    async fn create_worker(
        &self,
        spec: &RuntimeSpec,
        config: &ProviderConfig,
    ) -> Result<Worker, ProviderError>;

    /// Terminar un worker existente
    async fn terminate_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Obtener el estado actual de un worker
    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError>;

    /// Obtener los logs de un worker
    async fn get_logs(&self, worker_id: &WorkerId) -> Result<LogStreamRef, ProviderError>;

    /// Ejecutar port forwarding para un worker
    async fn port_forward(
        &self,
        worker_id: &WorkerId,
        local_port: u16,
        remote_port: u16,
    ) -> Result<String, ProviderError>;

    /// Obtener información de capacidad del provider
    async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError>;

    /// Ejecutar una función en un worker existente
    async fn execute_command(
        &self,
        worker_id: &WorkerId,
        command: Vec<String>,
        timeout: Option<Duration>,
    ) -> Result<ExecutionResult, ProviderError>;

    /// Reiniciar un worker
    async fn restart_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Pausar un worker
    async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Reanudar un worker pausado
    async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Suscribirse a eventos de ciclo de vida de workers
    fn stream_worker_events(&self) -> tokio_stream::wrappers::IntervalStream;
}

/// Resultado de ejecución de comando
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Código de salida
    pub exit_code: i32,
    /// Salida estándar
    pub stdout: String,
    /// Salida de error
    pub stderr: String,
    /// Tiempo de ejecución
    pub duration: Duration,
    /// Timestamp de inicio
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp de finalización
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

impl ExecutionResult {
    /// Crea un resultado de ejecución exitoso
    pub fn success(stdout: String, duration: Duration) -> Self {
        let now = chrono::Utc::now();
        let started_at = now - duration;
        Self {
            exit_code: 0,
            stdout,
            stderr: String::new(),
            duration,
            started_at,
            finished_at: now,
        }
    }

    /// Crea un resultado de ejecución con error
    pub fn error(exit_code: i32, stderr: String, duration: Duration) -> Self {
        let now = chrono::Utc::now();
        let started_at = now - duration;
        Self {
            exit_code,
            stdout: String::new(),
            stderr,
            duration,
            started_at,
            finished_at: now,
        }
    }

    /// Verifica si la ejecución fue exitosa
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}