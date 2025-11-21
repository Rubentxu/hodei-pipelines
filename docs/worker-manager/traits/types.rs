//! Tipos adicionales y estructuras de datos para el Worker Manager
//!
//! Este módulo contiene tipos auxiliares, enums y estructuras que apoyan
//! la funcionalidad principal del Worker Manager abstraction layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Configuración avanzada de scheduling para workers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConfig {
    /// Prioridad del worker (1-100, donde 100 es más alta)
    pub priority: Option<u8>,
    /// Afinidad por etiquetas
    pub affinity_labels: Option<HashMap<String, String>>,
    /// Tolerancias para scheduling
    pub tolerations: Option<Vec<Toleration>>,
    /// Requisitos de nodos específicos
    pub node_selector: Option<HashMap<String, String>>,
    /// Afinidad por zona geográfica
    pub zone_affinity: Option<String>,
    /// Ventana de ejecución preferida
    pub execution_window: Option<ExecutionWindow>,
}

impl Default for SchedulingConfig {
    fn default() -> Self {
        Self {
            priority: None,
            affinity_labels: None,
            tolerations: None,
            node_selector: None,
            zone_affinity: None,
            execution_window: None,
        }
    }
}

/// Toleración para scheduling en Kubernetes y sistemas similares
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toleration {
    pub key: String,
    pub operator: TolerationOperator,
    pub value: Option<String>,
    pub effect: TolerationEffect,
    pub toleration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TolerationOperator {
    Exists,
    Equal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TolerationEffect {
    NoSchedule,
    PreferNoSchedule,
    NoExecute,
}

/// Ventana de ejecución preferida
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionWindow {
    pub start_time: String, // Formato "HH:MM"
    pub end_time: String,   // Formato "HH:MM"
    pub timezone: String,   // IANA timezone
    pub days: Vec<DayOfWeek>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Configuración de aislamiento y sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// Nivel de aislamiento de red
    pub network_isolation: NetworkIsolationLevel,
    /// Nivel de aislamiento de proceso
    pub process_isolation: ProcessIsolationLevel,
    /// Configuración de namespaces
    pub namespace_config: NamespaceConfig,
    /// Configuración de cgroups
    pub cgroup_config: CgroupConfig,
    /// Configuración de capabilities de Linux
    pub capabilities_config: CapabilitiesConfig,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            network_isolation: NetworkIsolationLevel::Default,
            process_isolation: ProcessIsolationLevel::Standard,
            namespace_config: NamespaceConfig::default(),
            cgroup_config: CgroupConfig::default(),
            capabilities_config: CapabilitiesConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkIsolationLevel {
    /// Aislamiento por defecto del provider
    Default,
    /// Aislamiento básico (firewall del host)
    Basic,
    /// Aislamiento estricto ( namespaces de red)
    Strict,
    /// Aislamiento completo ( sin conectividad externa)
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessIsolationLevel {
    /// Aislamiento estándar
    Standard,
    /// Aislamiento mejorado ( nuevos PID namespace )
    Enhanced,
    /// Aislamiento completo ( contenedores )
    Complete,
}

/// Configuración de namespaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceConfig {
    pub pid_namespace: bool,
    pub network_namespace: bool,
    pub mount_namespace: bool,
    pub uts_namespace: bool,
    pub ipc_namespace: bool,
    pub user_namespace: bool,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            pid_namespace: true,
            network_namespace: true,
            mount_namespace: true,
            uts_namespace: true,
            ipc_namespace: false,
            user_namespace: false,
        }
    }
}

/// Configuración de cgroups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CgroupConfig {
    pub cpu_shares: Option<u64>,
    pub memory_limit: Option<u64>,
    pub block_io_weight: Option<u64>,
    pub hugetlb_limit: Option<u64>,
    pub devices_allowed: Option<Vec<String>>,
}

impl Default for CgroupConfig {
    fn default() -> Self {
        Self {
            cpu_shares: None,
            memory_limit: None,
            block_io_weight: None,
            hugetlb_limit: None,
            devices_allowed: None,
        }
    }
}

/// Configuración de capabilities de Linux
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    pub allowed_capabilities: Option<Vec<String>>,
    pub denied_capabilities: Option<Vec<String>>,
    pub drop_capabilities: Option<Vec<String>>,
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            allowed_capabilities: None,
            denied_capabilities: None,
            drop_capabilities: Some(vec!["ALL".to_string()]),
        }
    }
}

/// Configuración de logs y monitoreo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Nivel de log del worker
    pub log_level: LogLevel,
    /// Formato de logs
    pub log_format: LogFormat,
    /// Configuración de streaming
    pub streaming_config: StreamingConfig,
    /// Configuración de rotación de logs
    pub rotation_config: LogRotationConfig,
    /// Configuración de métricas
    pub metrics_config: MetricsConfig,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            log_format: LogFormat::Json,
            streaming_config: StreamingConfig::default(),
            rotation_config: LogRotationConfig::default(),
            metrics_config: MetricsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Plain,
    Json,
    Syslog,
    Custom(String),
}

/// Configuración de streaming de logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub enabled: bool,
    pub buffer_size: Option<usize>,
    pub flush_interval: Option<Duration>,
    pub max_retries: Option<u32>,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: Some(8192),
            flush_interval: Some(Duration::from_secs(1)),
            max_retries: Some(3),
        }
    }
}

/// Configuración de rotación de logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    pub max_size_mb: Option<u64>,
    pub max_files: Option<u32>,
    pub compression: bool,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_size_mb: Some(100),
            max_files: Some(5),
            compression: true,
        }
    }
}

/// Configuración de métricas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub collection_interval: Option<Duration>,
    pub metrics_endpoint: Option<String>,
    pub custom_metrics: Option<Vec<CustomMetric>>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Some(Duration::from_secs(30)),
            metrics_endpoint: None,
            custom_metrics: None,
        }
    }
}

/// Métricas personalizadas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetric {
    pub name: String,
    pub metric_type: CustomMetricType,
    pub collection_command: String,
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustomMetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Configuración de dependencias entre workers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConfig {
    /// Workers que deben completarse antes de iniciar este
    pub depends_on: Option<Vec<WorkerId>>,
    /// Workers que no pueden ejecutarse simultáneamente
    pub excludes: Option<Vec<WorkerId>>,
    /// Trabajadores que prefieren ejecutarse juntos
    pub prefers: Option<Vec<WorkerId>>,
    /// Configuración de datos compartidos
    pub shared_data: Option<SharedDataConfig>,
}

impl Default for DependencyConfig {
    fn default() -> Self {
        Self {
            depends_on: None,
            excludes: None,
            prefers: None,
            shared_data: None,
        }
    }
}

/// Configuración de datos compartidos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDataConfig {
    pub shared_volumes: Option<Vec<SharedVolumeConfig>>,
    pub shared_secrets: Option<Vec<SharedSecretConfig>>,
    pub data_sync_interval: Option<Duration>,
}

/// Configuración de volumen compartido
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedVolumeConfig {
    pub volume_name: String,
    pub access_mode: SharedAccessMode,
    pub mount_path: String,
}

/// Modos de acceso a datos compartidos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SharedAccessMode {
    ReadOnly,
    ReadWrite,
    Exclusive,
}

/// Configuración de secreto compartido
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSecretConfig {
    pub secret_name: String,
    pub access_mode: SharedAccessMode,
}

/// Configuración de cost management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Tipo de instancia/máquina (para cálculo de costos)
    pub instance_type: Option<String>,
    /// Precio por hora estimado
    pub cost_per_hour: Option<f64>,
    /// Límite de costo total
    pub max_cost: Option<f64>,
    /// Optimización de costos
    pub optimization: CostOptimization,
    /// Spot/preemptible configuration
    pub spot_config: Option<SpotConfig>,
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            instance_type: None,
            cost_per_hour: None,
            max_cost: None,
            optimization: CostOptimization::Balanced,
            spot_config: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostOptimization {
    Performance,
    Cost,
    Balanced,
    Custom,
}

/// Configuración de instancias spot/preemptibles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotConfig {
    pub enabled: bool,
    pub max_price: Option<f64>,
    pub interruption_behavior: Option<InterruptionBehavior>,
    pub allocation_strategy: Option<SpotAllocationStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptionBehavior {
    Terminate,
    Hibernate,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpotAllocationStrategy {
    LowestPrice,
    Diversified,
    CapacityOptimized,
}

/// Configuración de high availability y fault tolerance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityConfig {
    /// Replicas del worker para alta disponibilidad
    pub replica_count: Option<u32>,
    /// Configuración de health checks
    pub health_checks: Option<Vec<HealthCheckConfig>>,
    /// Configuración de failure domains
    pub failure_domains: Option<Vec<String>>,
    /// Configuración de backup workers
    pub backup_config: Option<BackupConfig>,
    /// Configuración de recovery
    pub recovery_config: Option<RecoveryConfig>,
}

impl Default for AvailabilityConfig {
    fn default() -> Self {
        Self {
            replica_count: Some(1),
            health_checks: Some(vec![HealthCheckConfig::default()]),
            failure_domains: None,
            backup_config: None,
            recovery_config: None,
        }
    }
}

/// Configuración de health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub path: Option<String>,
    pub port: Option<u16>,
    pub protocol: HealthCheckProtocol,
    pub timeout: Option<Duration>,
    pub interval: Option<Duration>,
    pub failure_threshold: Option<u32>,
    pub success_threshold: Option<u32>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            path: Some("/health".to_string()),
            port: Some(8080),
            protocol: HealthCheckProtocol::Http,
            timeout: Some(Duration::from_secs(5)),
            interval: Some(Duration::from_secs(30)),
            failure_threshold: Some(3),
            success_threshold: Some(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckProtocol {
    Http,
    Https,
    Tcp,
    Command,
}

/// Configuración de backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub enabled: bool,
    pub interval: Option<Duration>,
    pub retention: Option<Duration>,
    pub backup_location: Option<String>,
}

/// Configuración de recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    pub auto_recovery: bool,
    pub recovery_timeout: Option<Duration>,
    pub max_recovery_attempts: Option<u32>,
    pub recovery_strategy: RecoveryStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Restart,
    Failover,
    Recreate,
}

/// Configuración de integración con sistemas de CI/CD externos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIntegrationConfig {
    /// URLs de webhooks para notificaciones
    pub webhook_urls: Option<Vec<String>>,
    /// Configuración de notification channels
    pub notification_channels: Option<Vec<NotificationChannel>>,
    /// Configuración de artefacto management
    pub artifact_config: Option<ArtifactConfig>,
    /// Configuración de pipeline integration
    pub pipeline_config: Option<PipelineConfig>,
}

/// Canales de notificación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub name: String,
    pub channel_type: NotificationType,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Slack,
    Email,
    Webhook,
    Teams,
    Discord,
}

/// Configuración de artefactos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactConfig {
    pub upload_enabled: bool,
    pub storage_backend: String,
    pub retention_days: Option<u32>,
}

/// Configuración de pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub pipeline_id: Option<String>,
    pub stage_id: Option<String>,
    pub step_id: Option<String>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
}

/// Configuración de metadata para tracking y auditoría
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    /// Labels para tracking
    pub custom_labels: Option<HashMap<String, String>>,
    /// Tags para categorización
    pub tags: Option<Vec<String>>,
    /// Información del usuario que solicitó el worker
    pub requester_info: Option<RequesterInfo>,
    /// Información del proyecto/entorno
    pub project_info: Option<ProjectInfo>,
    /// Metadata de auditoría
    pub audit_metadata: Option<AuditMetadata>,
}

/// Información del solicitante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequesterInfo {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
    pub api_key_id: Option<String>,
    pub request_source: String,
}

/// Información del proyecto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub project_name: String,
    pub environment: String,
    pub version: Option<String>,
    pub deployment_target: Option<String>,
}

/// Metadata de auditoría
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetadata {
    pub correlation_id: String,
    pub trace_id: Option<String>,
    pub session_id: Option<String>,
    pub compliance_tags: Option<Vec<String>>,
}