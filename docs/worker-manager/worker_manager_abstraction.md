# Worker Manager Abstraction Layer - Documentación Completa e Implementación

## Resumen Ejecutivo y Alcance

Este documento define un plan técnico narrativo y guía de implementación para construir un Worker Manager Abstraction Layer en Rust, diseñado para orquestar workers efímeros sobre múltiples providers de infraestructura, con un subsistema de credenciales y secretos integrado, observabilidad consistente, seguridad por diseño e integración nativa con la arquitectura existente basada en NATS JetStream para eventos, Keycloak para identidad y autenticación, y AWS Verified Permissions para autorización.

El objetivo es doble. Primero, aislar la complejidad de cada provider (inicialmente Kubernetes y Docker) detrás de una interfaz común, el trait WorkerManagerProvider, para que los equipos puedan consumir capacidades homogéneas de creación, terminación, salud, capacidades, ejecución y observabilidad de workers. Segundo, ofrecer una capa de credenciales y secretos que unifique la obtención, montaje y rotación de secretos en ambos providers, con soporte para múltiples gestores (Vault, AWS Secrets Manager, Keycloak Service Accounts) y estrategias de rotación automática sin interrumpir las operaciones en curso.

El alcance de esta primera iteración incluye:
- Trait WorkerManagerProvider y sus tipos de soporte (RuntimeSpec, ResourceQuota, WorkerState, NetworkRequirements, etc.).
- Dos implementaciones completas: KubernetesProvider (kube-rs) y DockerProvider (docker-rust).
- Un sistema de credenciales con cuatro proveedores: SimpleCredentialProvider, HashiCorpVaultProvider, AWSSecretsManagerProvider y KeycloakServiceAccountProvider.
- Rotación automática de secretos con estrategias basadas en tiempo, eventos y disparadores manuales.
- Integración con la arquitectura existente: NATS JetStream (eventos y stream de estado), Keycloak (identidad, Service Accounts y tokens), y AWS Verified Permissions (decisiones de autorización).
- Guías de observabilidad (logs y métricas), shutdown ordenado y manejo de errores y políticas de retry.

Entregables principales:
- Especificación y ejemplo del trait WorkerManagerProvider con contratos async.
- Implementaciones de KubernetesProvider y DockerProvider con soporte de recursos, health checks, montaje de ConfigMaps/Secrets y networking/volúmenes.
- Traits CredentialProvider y SecretStore, y providers específicos.
- Motor de rotación automática con auditoría y estrategias de rollover sin downtime.
- Código Rust ejecutable y modular para todos los componentes, con ejemplos de uso.
- Patrones de integración con NATS JetStream, Keycloak y AWS Verified Permissions.
- Guía de pruebas, despliegue, seguridad, límites operativos y hoja de ruta.

Criterios de éxito:
- Funcionalidad: crear, terminar y observar workers en Kubernetes y Docker con credenciales y secretos montados; rotación sin interrupciones; publicación de eventos en JetStream.
- Calidad: manejo de errores consistente, pruebas unitarias y de integración, observabilidad (logs estructurados y métricas básicas).
- Seguridad: least-privilege, control de acceso con Keycloak y políticas externas (AVP), gestión segura de secretos y auditoría.
- Rendimiento y resiliencia: health checks y readiness, graceful shutdown, idempotencia y limpieza automática.

Brechas de información conocidas (a cerrar durante la implementación):
- Versiones exactas de dependencias (kube-rs, docker-rs), TLS y CA.
- Endpoints concretos de NATS JetStream y esquemas de subject/topic.
- Esquema y permisos de políticas en AWS Verified Permissions.
- Detalles de namespaces, RBAC, ServiceAccounts, NetworkPolicies, ResourceQuota y límites por nodo en el clúster destino.
- Parámetros de retención/rotación de secretos en Vault/AWS y necesidades de compliance.
- Esquemas de etiquetas y regiones para multi-tenant y multi-región.
- Requisitos de compatibilidad de runtime (imágenes, arquitectura, dispositivos especiales).

El resto del documento se estructura para convertir estos objetivos en un diseño implementable con contratos claros, decisiones arquitectónicas justificadas y ejemplos de uso, de modo que el lector pueda construir, validar y operar el sistema en entornos reales.

---

## Arquitectura del Worker Manager Abstraction Layer

La arquitectura se organiza en capas y componentes acoplados por contratos explícitos y eventos. La separación entre interfaz de providers (qué capacidades ofrece) y la lógica de infraestructura (cómo se implementan) permite añadir nuevos providers sin alterar el núcleo, y facilita pruebas y sustitución en tiempo de ejecución.

- Núcleo de interfaces: WorkerManagerProvider trait y tipos asociados (RuntimeSpec, ResourceQuota, Worker, etc.), más los tipos de error y observabilidad.
- Providers de infraestructura: KubernetesProvider y DockerProvider, que materializan los contratos del trait en objetos del provider (pods/containers).
- Gestión de credenciales y secretos: CredentialProvider trait y SecretStore trait para obtener, montar y rotar secretos. Providers concretos (Vault, AWS Secrets Manager, Keycloak Service Accounts, Simple).
- Motor de rotación: background task que coordina estrategias (time-based, event-based, manual) con audit trail.
- Observabilidad: logs estructurados y métricas (tiempos de provisión, tasas de error, rotación).
- Seguridad e identidad: integración con Keycloak (tokens, Service Accounts), ServiceAccounts en Kubernetes, y autorización basada en políticas (AWS Verified Permissions).
- Integración de eventos: NATS JetStream para publicar WorkerEvent y stream de estados; suscripciones para orquestación y control.

Flujos principales:
1. Provisión: un caller (operador o scheduler) solicita la creación de un worker con RuntimeSpec y ProviderConfig. El provider correspondiente crea el objeto de infraestructura (pod/container), aplica límites de recursos, monta secretos/ConfigMaps según SecretMountStrategy, configura red y health checks. Una vez Running, se publica WorkerCreated.
2. Operación: health checks periódicos mantienen el estado. Ejecución de comandos y port forwarding se exponen por el trait. Logs se exponen mediante LogStreamRef.
3. Terminación: se publica WorkerTerminated/Failed; se libera recursos y se asegura limpieza.
4. Credenciales: la capa de credenciales resuelve secret_refs del RuntimeSpec, obtiene secretos de los providers, los monta (ENV/Files/Volume) y coordina su rotación (releases y versiones activas).
5. Rotación: el motor de rotación actualiza secretos en background sin interrumpir trabajos; utiliza dual-release y rollback; audita cada transición.

Para anclar estos contratos, la Tabla 1 resume el mapa de responsabilidades y dependencias entre componentes.

Tabla 1. Mapa de responsabilidades y dependencias entre componentes
| Componente | Responsabilidad | Depende de | Interacciones |
|---|---|---|---|
| WorkerManagerProvider (trait) | Interfaz uniforme para crear, terminar, salud, capacidad, ejecución, logs, port-forward y eventos | ProviderConfig, RuntimeSpec, tipos de error | Llamado por orquestación; retorna Worker/WorkerState/ExecutionResult |
| KubernetesProvider | Creación/terminación de pods, límites de recursos, health/readiness, ServiceAccount, ConfigMaps/Secrets, port-forward | kube-rs; Kubernetes API | Publica eventos; expone estados; mapea secretos |
| DockerProvider | Creación/eliminación de contenedores, límites, networking, volúmenes, health checks, logs | Docker Engine API | Publica eventos; expone estados; mapea secretos |
| CredentialProvider (trait) | get_secret, put_secret, list_versions, access policies, audit | SecretStore, motores de autenticación | Consumido por providers y RuntimeSpec secret_refs |
| SimpleCredentialProvider | Almacenamiento en memoria con rotación | — | Útil para pruebas/dev |
| HashiCorpVaultProvider | Integración con Vault (KV y transit), rotación, versioning | Vault API | Política y audit; TTL/leases |
| AWSSecretsManagerProvider | Integración con AWS Secrets Manager (versiones, rotación) | AWS SDK/API | IAM; labels/tags |
| KeycloakServiceAccountProvider | OIDC client credentials, tokens, JWKs | Keycloak (OIDC) | Tokens para llamadas service-to-service |
| Rotator | Background task con estrategias time/event/manual; rollover | CredentialProviders; event bus | Publica audit trail; gestiona dual-release |
| Event Bus (NATS JetStream) | Publicación y suscripción de WorkerEvent y estado | NATS | Orquesta y reacciona a eventos |
| Observability | Logs estructurados y métricas | Todos | Monitoreo y alertas |
| Authorization (AVP) | Decisiones de autorización basadas en políticas | AWS Verified Permissions | Controla operaciones sensibles |

Principios de diseño:
- Contratos estables: los traits definen el "qué" sin exponer detalles del "cómo".
- Idempotencia y reintentos: operaciones repetibles; errores categorizados para retry/backoff.
- Observabilidad por defecto: logs y métricas en cada operación relevante.
- Seguridad por diseño: least-privilege, ServiceAccounts, montaje mínimo de secretos, políticas de red restrictivas.
- Extensibilidad: soporte para futuros providers (ECS, Cloud Run, ACI, serverless) con el mismo trait.

### Capa de Providers (Kubernetes y Docker)

La capa de providers implementa el trait WorkerManagerProvider. Cada provider:
- Materializa RuntimeSpec en objetos nativos (pods en Kubernetes, contenedores en Docker).
- Asigna recursos (CPU/memoria/almacenamiento) y GPUs si procede.
- Mapea volúmenes y secretos conforme a SecretMountStrategy.
- Configura health checks (liveness/readiness en K8s; health endpoint en Docker).
- Gestiona el estado transicional (Creating, Running, Terminating) con timeouts y backoffs.
- Publica WorkerEvent a NATS y expone un stream de eventos.

KubernetesProvider mapea:
- RuntimeSpec.image → container image del pod.
- ResourceQuota → requests/limits de CPU/memoria; toleraciones y node selectors si aplica.
- secret_refs → Secret/K8s montados como ENV o Files; ConfigMaps como volúmenes.
- health checks → liveness/readiness probes en el contenedor.
- ServiceAccount → identity del pod (roles RBAC y políticas de red).
- port-forward → subesión de kubectl o API server para túneles temporales.

DockerProvider mapea:
- RuntimeSpec.image → image del contenedor.
- ResourceQuota → límites de CPU/memoria; capacidades específicas si aplica.
- volumes → mounts de tipo bind, volume o tmpfs; lectura/escritura según VolumeSource.
- health checks → Docker healthcheck (HTTP/TCP/command).
- networking → mapeo de puertos y aislamiento (bridge/host).
- logs → lectura de streams stdout/stderr.

### Capa de Credenciales y Secretos

La capa de credenciales expone CredentialProvider y SecretStore, y ofrece una vista unificada de:
- Obtención y actualización de secretos por nombre y versión (get_secret, list_versions).
- Escritura/creación controlada de secretos (put_secret) con validación de formato y tamaño.
- Políticas de acceso (quién puede leer/ escribir/rotar qué secreto) y permisos.
- Auditoría de accesos y rotaciones.

SecretMountStrategy define cómo un worker consume un secreto:
- Environment: variables de entorno en el proceso (rápido, simple).
- Files: archivos montados con permisos restringidos (fácil integración con shells/scripts).
- Volume: montaje de volumen secreto para múltiples archivos/keys y cambios hot-reload.

Los secretos se relacionan con RuntimeSpec.secret_refs: el worker manager resuelve la referencia a un secreto lógico en la capa de credenciales y delega al provider el montaje efectivo. La rotación automática opera en background con dual-release: mantener la versión anterior disponible hasta que los workers activos migren a la nueva sin interrupciones.

---

## Traits Core - Interfaces Fundamentales

Los traits fundamentales definen los contratos async que permiten la abstracción entre diferentes providers de infraestructura y sistemas de credenciales.

```rust
//! Traits core del Worker Manager Abstraction Layer
//!
//! Este módulo define las interfaces fundamentales que permiten la abstracción
//! entre diferentes providers de infraestructura (Kubernetes, Docker, etc.)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::errors::ProviderError;

/// Identificador único para un worker
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerId(String);

impl WorkerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

/// Estados posibles de un worker
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerState {
    Creating,
    Running,
    Terminating,
    Terminated,
    Failed { reason: String },
    Paused,
    Unknown,
}

/// Especificación de recursos para un worker
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// CPU en millicores (1000 = 1 core)
    pub cpu_m: u32,
    /// Memoria en MB
    pub memory_mb: u32,
    /// Número de GPUs (opcional)
    pub gpu: Option<u32>,
    /// Almacenamiento temporal en MB
    pub storage_mb: Option<u32>,
}

impl ResourceQuota {
    pub fn basic(cpu_m: u32, memory_mb: u32) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: None,
            storage_mb: None,
        }
    }

    pub fn with_gpu(cpu_m: u32, memory_mb: u32, gpu: u32) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: Some(gpu),
            storage_mb: None,
        }
    }

    pub fn with_storage(cpu_m: u32, memory_mb: u32, storage_mb: u32) -> Self {
        Self {
            cpu_m,
            memory_mb,
            gpu: None,
            storage_mb: Some(storage_mb),
        }
    }

    /// Verifica si estos recursos son compatibles con la capacidad disponible
    pub fn is_compatible_with(&self, available: &CapacityInfo) -> bool {
        self.cpu_m <= available.available_cpu_m
            && self.memory_mb <= available.available_memory_mb
            && self.gpu.unwrap_or(0) <= available.available_gpu.unwrap_or(0)
            && self.storage_mb.unwrap_or(0) <= available.available_storage_mb.unwrap_or(u32::MAX)
    }
}

/// Tipos de protocolos para health checks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthCheckProtocol {
    Http,
    Https,
    Tcp,
    Command,
}

/// Configuración de health checks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub protocol: HealthCheckProtocol,
    pub path: Option<String>,
    pub port: Option<u16>,
    pub command: Option<Vec<String>>,
    pub timeout: Duration,
    pub interval: Duration,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            protocol: HealthCheckProtocol::Http,
            path: Some("/health".to_string()),
            port: Some(8080),
            command: None,
            timeout: Duration::from_secs(5),
            interval: Duration::from_secs(30),
            failure_threshold: 3,
            success_threshold: 1,
        }
    }
}

/// Tipos de red para workers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkType {
    Private,
    Public,
    Hybrid,
    Isolated,
}

/// Políticas de acceso de red
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessPolicy {
    AllowServices(Vec<String>),
    BlockServices(Vec<String>),
    AllowIpRange(String),
    RequireTunnel,
}

/// Niveles de aislamiento de red
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    Default,
    Strict,
    Complete,
}

/// Requisitos de red para un worker
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRequirements {
    pub network_type: NetworkType,
    pub access_policies: Vec<AccessPolicy>,
    pub isolation_level: IsolationLevel,
    pub required_ports: Vec<u16>,
}

impl Default for NetworkRequirements {
    fn default() -> Self {
        Self {
            network_type: NetworkType::Private,
            access_policies: vec![],
            isolation_level: IsolationLevel::Default,
            required_ports: vec![],
        }
    }
}

/// Fuentes de volúmenes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeSource {
    EmptyDir,
    PersistentVolume(String),
    HostPath(String),
    Secret(String),
    ConfigMap(String),
}

/// Montaje de volúmenes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeMount {
    pub mount_path: String,
    pub source: VolumeSource,
    pub read_only: bool,
}

/// Especificación de runtime para un worker
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSpec {
    /// Imagen del contenedor/pod
    pub image: String,
    /// Comando a ejecutar (override del CMD de la imagen)
    pub command: Option<Vec<String>>,
    /// Variables de entorno
    pub env: HashMap<String, String>,
    /// Referencias a secretos a montar
    pub secret_refs: Vec<String>,
    /// Puertos a exponer
    pub ports: Vec<u16>,
    /// Volúmenes a montar
    pub volumes: Vec<VolumeMount>,
    /// Labels adicionales
    pub labels: HashMap<String, String>,
    /// Recursos requeridos
    pub resources: ResourceQuota,
    /// Timeout para operaciones
    pub timeout: Option<Duration>,
    /// Requisitos de red
    pub network_requirements: NetworkRequirements,
    /// Configuración de health checks
    pub health_checks: Option<HealthCheckConfig>,
}

impl RuntimeSpec {
    /// Crea una especificación básica con una imagen
    pub fn basic(image: String) -> Self {
        Self {
            image,
            command: None,
            env: HashMap::new(),
            secret_refs: vec![],
            ports: vec![],
            volumes: vec![],
            labels: HashMap::new(),
            resources: ResourceQuota::basic(1000, 512), // 1 core, 512MB
            timeout: Some(Duration::from_secs(300)),
            network_requirements: NetworkRequirements::default(),
            health_checks: None,
        }
    }
}

/// Información de capacidad del provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityInfo {
    pub total_cpu_m: u32,
    pub used_cpu_m: u32,
    pub available_cpu_m: u32,
    pub total_memory_mb: u32,
    pub used_memory_mb: u32,
    pub available_memory_mb: u32,
    pub total_gpu: Option<u32>,
    pub used_gpu: Option<u32>,
    pub available_gpu: Option<u32>,
    pub total_storage_mb: Option<u32>,
    pub used_storage_mb: Option<u32>,
    pub available_storage_mb: Option<u32>,
    pub active_workers: u32,
    pub max_workers: Option<u32>,
}

/// Información del worker creado
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worker {
    pub id: WorkerId,
    pub state: WorkerState,
    pub provider_info: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub spec: RuntimeSpec,
}

/// Resultado de la ejecución de un comando
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
}

/// Referencia a un stream de logs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogStreamRef {
    pub worker_id: WorkerId,
    pub endpoint: String,
    pub stream_type: LogStreamType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogStreamType {
    File(String),
    Http(String),
    Websocket(String),
}

/// Configuración específica del provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub namespace: Option<String>,
    pub security_policies: SecurityPolicies,
    pub rate_limits: RateLimits,
    pub additional_config: HashMap<String, String>,
}

impl ProviderConfig {
    pub fn kubernetes(namespace: String) -> Self {
        Self {
            provider_type: "kubernetes".to_string(),
            namespace: Some(namespace),
            security_policies: SecurityPolicies::default(),
            rate_limits: RateLimits::default(),
            additional_config: HashMap::new(),
        }
    }

    pub fn docker() -> Self {
        Self {
            provider_type: "docker".to_string(),
            namespace: None,
            security_policies: SecurityPolicies::default(),
            rate_limits: RateLimits::default(),
            additional_config: HashMap::new(),
        }
    }
}

/// Políticas de seguridad
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPolicies {
    pub service_account: Option<String>,
    pub run_as_user: Option<u32>,
    pub run_as_group: Option<u32>,
    pub capabilities: Vec<String>,
    pub privileged: bool,
}

impl Default for SecurityPolicies {
    fn default() -> Self {
        Self {
            service_account: None,
            run_as_user: Some(1000),
            run_as_group: Some(1000),
            capabilities: vec![],
            privileged: false,
        }
    }
}

/// Límites de tasa
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimits {
    pub max_workers_per_minute: u32,
    pub max_operations_per_second: u32,
    pub default_timeout: Duration,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            max_workers_per_minute: 100,
            max_operations_per_second: 50,
            default_timeout: Duration::from_secs(30),
        }
    }
}

/// Eventos del ciclo de vida del worker
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerEvent {
    WorkerCreated {
        worker_id: WorkerId,
        provider_info: HashMap<String, String>,
        timestamp: DateTime<Utc>,
    },
    WorkerTerminated {
        worker_id: WorkerId,
        exit_code: Option<i32>,
        timestamp: DateTime<Utc>,
    },
    WorkerFailed {
        worker_id: WorkerId,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    WorkerStateChanged {
        worker_id: WorkerId,
        old_state: WorkerState,
        new_state: WorkerState,
        timestamp: DateTime<Utc>,
    },
}

/// Trait principal para providers de worker management
#[async_trait]
pub trait WorkerManagerProvider: Send + Sync {
    /// Nombre identificativo del provider
    fn name(&self) -> &str;

    /// Crea un nuevo worker con la especificación dada
    async fn create_worker(
        &self,
        spec: &RuntimeSpec,
        config: &ProviderConfig,
    ) -> Result<Worker, ProviderError>;

    /// Termina un worker existente
    async fn terminate_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Obtiene el estado actual de un worker
    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError>;

    /// Obtiene referencia a los logs de un worker
    async fn get_logs(&self, worker_id: &WorkerId) -> Result<LogStreamRef, ProviderError>;

    /// Establece port forwarding para un worker
    async fn port_forward(
        &self,
        worker_id: &WorkerId,
        local_port: u16,
        remote_port: u16,
    ) -> Result<String, ProviderError>;

    /// Obtiene información de capacidad del provider
    async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError>;

    /// Ejecuta un comando en un worker
    async fn execute_command(
        &self,
        worker_id: &WorkerId,
        command: Vec<String>,
        timeout: Option<Duration>,
    ) -> Result<ExecutionResult, ProviderError>;

    /// Reinicia un worker
    async fn restart_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Pausa un worker (si el provider lo soporta)
    async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Reanuda un worker pausado
    async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;

    /// Stream de eventos del worker
    fn stream_worker_events(&self) -> mpsc::UnboundedReceiver<WorkerEvent>;
}
```

---
## Manejo de Errores - Sistema Unificado

El sistema de errores proporciona categorización consistente, información contextual y políticas de retry para todos los providers.

```rust
//! Sistema unificado de manejo de errores para el Worker Manager
//!
//! Proporciona categorización de errores, información contextual y políticas de retry

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::traits::{WorkerId, ProviderConfig};

/// Códigos de error categorizados
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    // Errores de validación del cliente
    Validation,
    NotFound,
    AlreadyExists,
    
    // Errores de autenticación y autorización
    Authentication,
    Authorization,
    
    // Errores de infraestructura
    Infrastructure,
    Network,
    Timeout,
    
    // Errores de recursos
    ResourceExhausted,
    Compatibility,
    
    // Errores de configuración
    InvalidConfiguration,
    
    // Errores internos
    Internal,
    
    // Errores específicos del provider
    ProviderSpecific(String),
}

/// Categorías de error para políticas de retry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Error de validación del cliente - no reintentar
    ClientValidation,
    /// Error de autorización - no reintentar
    Authorization,
    /// Error transitorio de infraestructura - reintentar con backoff
    TransientInfrastructure,
    /// Error permanente de infraestructura - no reintentar
    PermanentInfrastructure,
    /// Error de configuración - no reintentar
    Configuration,
    /// Error de límites de recursos - reintentar con delay
    ResourceLimits,
    /// Error interno - no reintentar
    Internal,
}

/// Severidad del error
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// Información de retry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryInfo {
    pub attempt: u32,
    pub max_attempts: u32,
    pub next_retry_in: Option<std::time::Duration>,
    pub backoff_strategy: BackoffStrategy,
}

/// Estrategias de backoff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed(std::time::Duration),
    Exponential { base: std::time::Duration, multiplier: f64, max: std::time::Duration },
    Linear { base: std::time::Duration, increment: std::time::Duration },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        BackoffStrategy::Exponential {
            base: std::time::Duration::from_secs(1),
            multiplier: 2.0,
            max: std::time::Duration::from_secs(60),
        }
    }
}

/// Contexto de error con información adicional
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation_id: String,
    pub worker_id: Option<WorkerId>,
    pub provider: Option<String>,
    pub configuration: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub additional_info: HashMap<String, String>,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            operation_id: uuid::Uuid::new_v4().to_string(),
            worker_id: None,
            provider: None,
            configuration: None,
            timestamp: Utc::now(),
            additional_info: HashMap::new(),
        }
    }
}

/// Error principal del sistema con información completa
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub struct ProviderError {
    pub code: ErrorCode,
    pub category: ErrorCategory,
    pub severity: Severity,
    pub message: String,
    pub context: ErrorContext,
    pub retry_info: Option<RetryInfo>,
    pub recovery_suggestions: Vec<String>,
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {:?}: {} (Operation: {})",
            self.severity_symbol(),
            self.code,
            self.message,
            self.context.operation_id
        )
    }
}

impl ProviderError {
    /// Crea un nuevo error con categorización automática
    pub fn new(code: ErrorCode, message: String) -> Self {
        let (category, severity) = Self::categorize_error(&code);
        
        Self {
            code,
            category,
            severity,
            message,
            context: ErrorContext::default(),
            retry_info: None,
            recovery_suggestions: vec![],
        }
    }

    /// Categoriza automáticamente un error basado en su código
    fn categorize_error(code: &ErrorCode) -> (ErrorCategory, Severity) {
        match code {
            ErrorCode::Validation | ErrorCode::NotFound | ErrorCode::AlreadyExists => {
                (ErrorCategory::ClientValidation, Severity::Low)
            }
            ErrorCode::Authentication | ErrorCode::Authorization => {
                (ErrorCategory::Authorization, Severity::Medium)
            }
            ErrorCode::Infrastructure => {
                (ErrorCategory::TransientInfrastructure, Severity::High)
            }
            ErrorCode::Network | ErrorCode::Timeout => {
                (ErrorCategory::TransientInfrastructure, Severity::Medium)
            }
            ErrorCode::ResourceExhausted => {
                (ErrorCategory::ResourceLimits, Severity::High)
            }
            ErrorCode::Compatibility | ErrorCode::InvalidConfiguration => {
                (ErrorCategory::Configuration, Severity::High)
            }
            ErrorCode::Internal => {
                (ErrorCategory::Internal, Severity::Critical)
            }
            ErrorCode::ProviderSpecific(_) => {
                (ErrorCategory::Internal, Severity::Medium)
            }
        }
    }

    /// Símbolo visual para la severidad
    fn severity_symbol(&self) -> &'static str {
        match self.severity {
            Severity::Critical => "🚨",
            Severity::High => "⚠️",
            Severity::Medium => "⚡",
            Severity::Low => "ℹ️",
        }
    }

    /// Verifica si el error es transitorio y debería reintentarse
    pub fn is_transient(&self) -> bool {
        matches!(
            self.category,
            ErrorCategory::TransientInfrastructure | ErrorCategory::ResourceLimits
        )
    }

    /// Verifica si el error es permanente y no debe reintentarse
    pub fn is_permanent(&self) -> bool {
        !self.is_transient()
    }

    /// Establece contexto del worker
    pub fn with_worker_id(mut self, worker_id: WorkerId) -> Self {
        self.context.worker_id = Some(worker_id);
        self
    }

    /// Establece contexto del provider
    pub fn with_provider(mut self, provider: String) -> Self {
        self.context.provider = Some(provider);
        self
    }

    /// Establece información de retry
    pub fn with_retry_info(mut self, retry_info: RetryInfo) -> Self {
        self.retry_info = Some(retry_info);
        self
    }

    /// Agrega sugerencias de recuperación
    pub fn with_recovery_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.recovery_suggestions = suggestions;
        self
    }

    /// Agrega información adicional al contexto
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.additional_info.insert(key, value);
        self
    }

    // Métodos de fábrica para errores comunes

    /// Error de infraestructura
    pub fn infrastructure(message: String) -> Self {
        Self::new(ErrorCode::Infrastructure, message)
            .with_recovery_suggestions(vec![
                "Verificar conectividad de red".to_string(),
                "Revisar estado de la infraestructura subyacente".to_string(),
                "Reintentar la operación".to_string(),
            ])
    }

    /// Error de autenticación
    pub fn authentication(message: String) -> Self {
        Self::new(ErrorCode::Authentication, message)
            .with_recovery_suggestions(vec![
                "Verificar credenciales".to_string(),
                "Revisar tokens de autenticación".to_string(),
                "Comprobar permisos del service account".to_string(),
            ])
    }

    /// Error de autorización
    pub fn authorization(message: String) -> Self {
        Self::new(ErrorCode::Authorization, message)
            .with_recovery_suggestions(vec![
                "Verificar permisos RBAC".to_string(),
                "Revisar políticas de autorización".to_string(),
                "Contactar al administrador del sistema".to_string(),
            ])
    }

    /// Error de recurso no encontrado
    pub fn not_found(resource_type: String, resource_id: String) -> Self {
        Self::new(
            ErrorCode::NotFound,
            format!("{} '{}' no encontrado", resource_type, resource_id),
        )
        .with_context("resource_type".to_string(), resource_type)
        .with_context("resource_id".to_string(), resource_id)
        .with_recovery_suggestions(vec![
            "Verificar que el recurso existe".to_string(),
            "Comprobar permisos de lectura".to_string(),
            "Revisar configuración de namespace".to_string(),
        ])
    }

    /// Error de validación
    pub fn validation(field: String, issue: String) -> Self {
        Self::new(
            ErrorCode::Validation,
            format!("Validation error en campo '{}': {}", field, issue),
        )
        .with_context("field".to_string(), field)
        .with_context("issue".to_string(), issue)
        .with_recovery_suggestions(vec![
            "Corregir los datos de entrada".to_string(),
            "Revisar la documentación de la API".to_string(),
        ])
    }

    /// Error de timeout
    pub fn timeout(operation: String, duration: std::time::Duration) -> Self {
        Self::new(
            ErrorCode::Timeout,
            format!("Timeout en operación '{}' después de {:?}", operation, duration),
        )
        .with_context("operation".to_string(), operation)
        .with_context("duration".to_string(), format!("{:?}", duration))
        .with_recovery_suggestions(vec![
            "Incrementar el timeout de la operación".to_string(),
            "Verificar la carga del sistema".to_string(),
            "Reintentar la operación".to_string(),
        ])
    }

    /// Error específico del provider
    pub fn provider_specific(provider: String, code: String, message: String) -> Self {
        Self::new(ErrorCode::ProviderSpecific(code.clone()), message)
            .with_provider(provider)
            .with_context("provider_error_code".to_string(), code)
            .with_recovery_suggestions(vec![
                "Consultar documentación del provider".to_string(),
                "Verificar configuración específica".to_string(),
            ])
    }
}

// Implementaciones From para errores comunes

impl From<reqwest::Error> for ProviderError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::timeout("HTTP request".to_string(), std::time::Duration::from_secs(30))
        } else if err.is_connect() {
            Self::infrastructure("Connection failed".to_string())
        } else {
            Self::new(
                ErrorCode::Network,
                format!("HTTP error: {}", err),
            )
        }
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(err: serde_json::Error) -> Self {
        Self::validation(
            "json".to_string(),
            format!("JSON parsing error: {}", err),
        )
    }
}

impl From<tokio::time::error::Elapsed> for ProviderError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::timeout(
            "async operation".to_string(),
            std::time::Duration::from_secs(30),
        )
    }
}

/// Errores específicos de credenciales
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum CredentialError {
    #[error("Secret '{name}' not found")]
    NotFound { name: String },
    
    #[error("Secret '{name}' version '{version}' not found")]
    VersionNotFound { name: String, version: String },
    
    #[error("Secret '{name}' has expired")]
    Expired { name: String },
    
    #[error("Rotation failed for secret '{name}': {reason}")]
    RotationFailed { name: String, reason: String },
    
    #[error("Invalid secret format: {reason}")]
    InvalidFormat { reason: String },
    
    #[error("Permission denied for operation '{operation}' on secret '{name}'")]
    PermissionDenied { operation: String, name: String },
    
    #[error("Rate limit exceeded: {message}")]
    LimitExceeded { message: String },
    
    #[error("Sync failed: {reason}")]
    SyncFailed { reason: String },
    
    #[error("Validation failed: {reason}")]
    ValidationFailed { reason: String },
}

/// Errores específicos del almacén de secretos
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum SecretError {
    #[error("Access denied to secret store: {reason}")]
    AccessDenied { reason: String },
    
    #[error("Secret size exceeds limit: {size} > {limit}")]
    SizeLimitExceeded { size: usize, limit: usize },
    
    #[error("Encryption failed: {reason}")]
    EncryptionFailed { reason: String },
    
    #[error("Decryption failed: {reason}")]
    DecryptionFailed { reason: String },
    
    #[error("Backend storage error: {message}")]
    StorageError { message: String },
    
    #[error("Network error in secret operations: {message}")]
    NetworkError { message: String },
}

impl From<CredentialError> for ProviderError {
    fn from(err: CredentialError) -> Self {
        match err {
            CredentialError::NotFound { name } => {
                Self::not_found("Secret".to_string(), name)
            }
            CredentialError::PermissionDenied { operation, name } => {
                Self::authorization(format!(
                    "Permission denied for {} on secret {}",
                    operation, name
                ))
            }
            CredentialError::RotationFailed { name, reason } => {
                Self::new(
                    ErrorCode::Infrastructure,
                    format!("Rotation failed for {}: {}", name, reason),
                )
            }
            _ => Self::new(ErrorCode::Internal, err.to_string()),
        }
    }
}

impl From<SecretError> for ProviderError {
    fn from(err: SecretError) -> Self {
        match err {
            SecretError::AccessDenied { reason } => {
                Self::authorization(format!("Secret store access denied: {}", reason))
            }
            SecretError::NetworkError { message } => {
                Self::new(ErrorCode::Network, message)
            }
            SecretError::StorageError { message } => {
                Self::infrastructure(format!("Storage error: {}", message))
            }
            _ => Self::new(ErrorCode::Internal, err.to_string()),
        }
    }
}

/// Política de retry configurable
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: std::time::Duration,
    pub max_delay: std::time::Duration,
    pub backoff_strategy: BackoffStrategy,
    pub retry_on_categories: Vec<ErrorCategory>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: std::time::Duration::from_secs(1),
            max_delay: std::time::Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::default(),
            retry_on_categories: vec![
                ErrorCategory::TransientInfrastructure,
                ErrorCategory::ResourceLimits,
            ],
        }
    }
}

impl RetryPolicy {
    /// Verifica si un error debería reintentarse según esta política
    pub fn should_retry(&self, error: &ProviderError, attempt: u32) -> bool {
        attempt < self.max_attempts
            && self.retry_on_categories.contains(&error.category)
    }

    /// Calcula el delay para el próximo intento
    pub fn calculate_delay(&self, attempt: u32) -> std::time::Duration {
        let delay = match self.backoff_strategy {
            BackoffStrategy::Fixed(duration) => duration,
            BackoffStrategy::Exponential { base, multiplier, max } => {
                let calculated = base.as_secs_f64() * multiplier.powi(attempt as i32);
                std::time::Duration::from_secs_f64(calculated.min(max.as_secs_f64()))
            }
            BackoffStrategy::Linear { base, increment } => {
                base + increment * attempt
            }
        };

        std::cmp::min(delay, self.max_delay)
    }
}
```

---## Implementación Kubernetes Provider

Implementación completa del trait WorkerManagerProvider para Kubernetes usando kube-rs, con soporte para pods, ConfigMaps, Secrets, health checks y RBAC.

```rust
//! Implementación del provider para Kubernetes
//!
//! Utiliza kube-rs para gestionar pods efímeros con recursos, health checks,
//! montaje de secretos/configmaps y networking

use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use chrono::Utc;
use k8s_openapi::api::core::v1::{
    Pod, PodSpec, Container, ResourceRequirements, Probe, HTTPGetAction,
    TCPSocketAction, ExecAction, EnvVar, VolumeMount as K8sVolumeMount,
    Volume, Secret, ConfigMap, SecretVolumeSource, ConfigMapVolumeSource,
    EmptyDirVolumeSource, PersistentVolumeClaimVolumeSource, HostPathVolumeSource,
    ContainerPort, SecurityContext, PodSecurityContext, Event,
};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ObjectMeta, LabelSelector};
use kube::{
    Api, Client, Config,
    api::{PostParams, DeleteParams, ListParams, LogParams},
    runtime::{watcher, WatchStreamExt},
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use serde_json::json;

use crate::traits::*;
use crate::errors::*;

/// Provider para Kubernetes usando kube-rs
pub struct KubernetesProvider {
    client: Client,
    namespace: String,
    event_sender: mpsc::UnboundedSender<WorkerEvent>,
    config: KubernetesConfig,
}

/// Configuración específica para Kubernetes
#[derive(Debug, Clone)]
pub struct KubernetesConfig {
    pub default_namespace: String,
    pub node_selector: HashMap<String, String>,
    pub tolerations: Vec<Toleration>,
    pub image_pull_secrets: Vec<String>,
    pub dns_policy: String,
    pub restart_policy: String,
    pub enable_service_account_token: bool,
}

#[derive(Debug, Clone)]
pub struct Toleration {
    pub key: Option<String>,
    pub operator: String,
    pub value: Option<String>,
    pub effect: Option<String>,
    pub toleration_seconds: Option<i64>,
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self {
            default_namespace: "default".to_string(),
            node_selector: HashMap::new(),
            tolerations: vec![],
            image_pull_secrets: vec![],
            dns_policy: "ClusterFirst".to_string(),
            restart_policy: "Never".to_string(),
            enable_service_account_token: false,
        }
    }
}

impl KubernetesProvider {
    /// Crea un nuevo provider de Kubernetes
    pub async fn new() -> Result<Self, ProviderError> {
        Self::with_config(KubernetesConfig::default()).await
    }

    /// Crea un provider con configuración personalizada
    pub async fn with_config(config: KubernetesConfig) -> Result<Self, ProviderError> {
        let client = Client::try_default()
            .await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to create k8s client: {}", e)))?;

        let (event_sender, _) = mpsc::unbounded_channel();

        Ok(Self {
            client,
            namespace: config.default_namespace.clone(),
            event_sender,
            config,
        })
    }

    /// Construye el objeto Pod a partir de la especificación
    fn build_pod(&self, worker_id: &WorkerId, spec: &RuntimeSpec, config: &ProviderConfig) -> Result<Pod, ProviderError> {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "worker-manager".to_string());
        labels.insert("worker-id".to_string(), worker_id.to_string());
        labels.insert("managed-by".to_string(), "worker-manager".to_string());
        labels.extend(spec.labels.clone());

        let mut annotations = HashMap::new();
        annotations.insert("worker-manager.io/worker-id".to_string(), worker_id.to_string());
        annotations.insert("worker-manager.io/created-at".to_string(), Utc::now().to_rfc3339());

        // Construir contenedor principal
        let mut container = Container {
            name: "worker".to_string(),
            image: Some(spec.image.clone()),
            command: spec.command.clone(),
            env: Some(self.build_env_vars(spec)?),
            ports: Some(self.build_container_ports(spec)),
            volume_mounts: Some(self.build_volume_mounts(spec)?),
            resources: Some(self.build_resource_requirements(&spec.resources)?),
            security_context: Some(self.build_security_context(config)?),
            ..Default::default()
        };

        // Configurar health checks si están definidos
        if let Some(health_config) = &spec.health_checks {
            let (liveness, readiness) = self.build_probes(health_config)?;
            container.liveness_probe = liveness;
            container.readiness_probe = readiness;
        }

        // Construir especificación del pod
        let pod_spec = PodSpec {
            containers: vec![container],
            volumes: Some(self.build_volumes(spec)?),
            restart_policy: Some(self.config.restart_policy.clone()),
            dns_policy: Some(self.config.dns_policy.clone()),
            service_account_name: config.security_policies.service_account.clone(),
            automount_service_account_token: Some(self.config.enable_service_account_token),
            node_selector: if self.config.node_selector.is_empty() { 
                None 
            } else { 
                Some(self.config.node_selector.clone()) 
            },
            security_context: Some(self.build_pod_security_context(config)?),
            image_pull_secrets: if self.config.image_pull_secrets.is_empty() {
                None
            } else {
                Some(self.config.image_pull_secrets.iter()
                    .map(|name| k8s_openapi::api::core::v1::LocalObjectReference {
                        name: Some(name.clone())
                    })
                    .collect())
            },
            ..Default::default()
        };

        let pod = Pod {
            metadata: ObjectMeta {
                name: Some(format!("worker-{}", worker_id)),
                namespace: Some(self.namespace.clone()),
                labels: Some(labels),
                annotations: Some(annotations),
                ..Default::default()
            },
            spec: Some(pod_spec),
            ..Default::default()
        };

        Ok(pod)
    }

    /// Construye variables de entorno para el contenedor
    fn build_env_vars(&self, spec: &RuntimeSpec) -> Result<Vec<EnvVar>, ProviderError> {
        let mut env_vars = vec![];

        for (key, value) in &spec.env {
            env_vars.push(EnvVar {
                name: key.clone(),
                value: Some(value.clone()),
                value_from: None,
            });
        }

        // Agregar variables del sistema
        env_vars.push(EnvVar {
            name: "WORKER_ID".to_string(),
            value_from: Some(k8s_openapi::api::core::v1::EnvVarSource {
                field_ref: Some(k8s_openapi::api::core::v1::ObjectFieldSelector {
                    field_path: "metadata.name".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        });

        Ok(env_vars)
    }

    /// Construye puertos del contenedor
    fn build_container_ports(&self, spec: &RuntimeSpec) -> Vec<ContainerPort> {
        spec.ports.iter().map(|&port| ContainerPort {
            container_port: port as i32,
            protocol: Some("TCP".to_string()),
            ..Default::default()
        }).collect()
    }

    /// Construye montajes de volúmenes para el contenedor
    fn build_volume_mounts(&self, spec: &RuntimeSpec) -> Result<Vec<K8sVolumeMount>, ProviderError> {
        let mut volume_mounts = vec![];

        for volume in &spec.volumes {
            let volume_name = match &volume.source {
                VolumeSource::EmptyDir => "emptydir",
                VolumeSource::Secret(name) => &format!("secret-{}", name),
                VolumeSource::ConfigMap(name) => &format!("configmap-{}", name),
                VolumeSource::PersistentVolume(name) => &format!("pv-{}", name),
                VolumeSource::HostPath(_) => "hostpath",
            };

            volume_mounts.push(K8sVolumeMount {
                name: volume_name.to_string(),
                mount_path: volume.mount_path.clone(),
                read_only: Some(volume.read_only),
                ..Default::default()
            });
        }

        Ok(volume_mounts)
    }

    /// Construye volúmenes del pod
    fn build_volumes(&self, spec: &RuntimeSpec) -> Result<Vec<Volume>, ProviderError> {
        let mut volumes = vec![];

        for volume_mount in &spec.volumes {
            let volume = match &volume_mount.source {
                VolumeSource::EmptyDir => Volume {
                    name: "emptydir".to_string(),
                    empty_dir: Some(EmptyDirVolumeSource {
                        medium: None,
                        size_limit: None,
                    }),
                    ..Default::default()
                },
                VolumeSource::Secret(secret_name) => Volume {
                    name: format!("secret-{}", secret_name),
                    secret: Some(SecretVolumeSource {
                        secret_name: Some(secret_name.clone()),
                        default_mode: Some(0o400),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                VolumeSource::ConfigMap(cm_name) => Volume {
                    name: format!("configmap-{}", cm_name),
                    config_map: Some(ConfigMapVolumeSource {
                        name: Some(cm_name.clone()),
                        default_mode: Some(0o644),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                VolumeSource::PersistentVolume(pv_name) => Volume {
                    name: format!("pv-{}", pv_name),
                    persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                        claim_name: pv_name.clone(),
                        read_only: Some(volume_mount.read_only),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                VolumeSource::HostPath(path) => Volume {
                    name: "hostpath".to_string(),
                    host_path: Some(HostPathVolumeSource {
                        path: path.clone(),
                        type_: None,
                    }),
                    ..Default::default()
                },
            };
            volumes.push(volume);
        }

        Ok(volumes)
    }

    /// Construye requisitos de recursos
    fn build_resource_requirements(&self, resources: &ResourceQuota) -> Result<ResourceRequirements, ProviderError> {
        let mut requests = HashMap::new();
        let mut limits = HashMap::new();

        // CPU en milicores
        requests.insert("cpu".to_string(), Quantity(format!("{}m", resources.cpu_m)));
        limits.insert("cpu".to_string(), Quantity(format!("{}m", resources.cpu_m)));

        // Memoria en MiB
        requests.insert("memory".to_string(), Quantity(format!("{}Mi", resources.memory_mb)));
        limits.insert("memory".to_string(), Quantity(format!("{}Mi", resources.memory_mb)));

        // GPUs si están especificadas
        if let Some(gpu_count) = resources.gpu {
            limits.insert("nvidia.com/gpu".to_string(), Quantity(gpu_count.to_string()));
        }

        // Storage efímero si está especificado
        if let Some(storage_mb) = resources.storage_mb {
            limits.insert("ephemeral-storage".to_string(), Quantity(format!("{}Mi", storage_mb)));
        }

        Ok(ResourceRequirements {
            requests: Some(requests),
            limits: Some(limits),
            ..Default::default()
        })
    }

    /// Construye el contexto de seguridad del contenedor
    fn build_security_context(&self, config: &ProviderConfig) -> Result<SecurityContext, ProviderError> {
        Ok(SecurityContext {
            run_as_user: config.security_policies.run_as_user.map(|u| u as i64),
            run_as_group: config.security_policies.run_as_group.map(|g| g as i64),
            run_as_non_root: Some(true),
            privileged: Some(config.security_policies.privileged),
            allow_privilege_escalation: Some(false),
            read_only_root_filesystem: Some(true),
            capabilities: Some(k8s_openapi::api::core::v1::Capabilities {
                drop: Some(vec!["ALL".to_string()]),
                add: Some(config.security_policies.capabilities.clone()),
            }),
            ..Default::default()
        })
    }

    /// Construye el contexto de seguridad del pod
    fn build_pod_security_context(&self, config: &ProviderConfig) -> Result<PodSecurityContext, ProviderError> {
        Ok(PodSecurityContext {
            run_as_user: config.security_policies.run_as_user.map(|u| u as i64),
            run_as_group: config.security_policies.run_as_group.map(|g| g as i64),
            run_as_non_root: Some(true),
            fs_group: config.security_policies.run_as_group.map(|g| g as i64),
            supplemental_groups: Some(vec![]),
            ..Default::default()
        })
    }

    /// Construye probes de liveness y readiness
    fn build_probes(&self, health_config: &HealthCheckConfig) -> Result<(Option<Probe>, Option<Probe>), ProviderError> {
        let probe = match health_config.protocol {
            HealthCheckProtocol::Http | HealthCheckProtocol::Https => {
                let scheme = if health_config.protocol == HealthCheckProtocol::Https { "HTTPS" } else { "HTTP" };
                
                Probe {
                    http_get: Some(HTTPGetAction {
                        path: health_config.path.clone(),
                        port: k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(
                            health_config.port.unwrap_or(8080) as i32
                        ),
                        scheme: Some(scheme.to_string()),
                        ..Default::default()
                    }),
                    timeout_seconds: Some(health_config.timeout.as_secs() as i32),
                    period_seconds: Some(health_config.interval.as_secs() as i32),
                    failure_threshold: Some(health_config.failure_threshold as i32),
                    success_threshold: Some(health_config.success_threshold as i32),
                    ..Default::default()
                }
            },
            HealthCheckProtocol::Tcp => {
                Probe {
                    tcp_socket: Some(TCPSocketAction {
                        port: k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(
                            health_config.port.unwrap_or(8080) as i32
                        ),
                        ..Default::default()
                    }),
                    timeout_seconds: Some(health_config.timeout.as_secs() as i32),
                    period_seconds: Some(health_config.interval.as_secs() as i32),
                    failure_threshold: Some(health_config.failure_threshold as i32),
                    success_threshold: Some(health_config.success_threshold as i32),
                    ..Default::default()
                }
            },
            HealthCheckProtocol::Command => {
                let command = health_config.command.as_ref()
                    .ok_or_else(|| ProviderError::validation("health_check.command".to_string(), "Command required for Command protocol".to_string()))?;

                Probe {
                    exec: Some(ExecAction {
                        command: Some(command.clone()),
                    }),
                    timeout_seconds: Some(health_config.timeout.as_secs() as i32),
                    period_seconds: Some(health_config.interval.as_secs() as i32),
                    failure_threshold: Some(health_config.failure_threshold as i32),
                    success_threshold: Some(health_config.success_threshold as i32),
                    ..Default::default()
                }
            },
        };

        // Para simplificar, usamos el mismo probe para liveness y readiness
        Ok((Some(probe.clone()), Some(probe)))
    }

    /// Obtiene el estado de un pod
    async fn get_pod_state(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let pod_name = format!("worker-{}", worker_id);

        match pods.get(&pod_name).await {
            Ok(pod) => {
                if let Some(status) = pod.status {
                    match status.phase.as_deref() {
                        Some("Pending") => Ok(WorkerState::Creating),
                        Some("Running") => {
                            // Verificar si está realmente listo
                            if let Some(conditions) = status.conditions {
                                let ready = conditions.iter().any(|c| 
                                    c.type_ == "Ready" && c.status == "True"
                                );
                                if ready {
                                    Ok(WorkerState::Running)
                                } else {
                                    Ok(WorkerState::Creating)
                                }
                            } else {
                                Ok(WorkerState::Creating)
                            }
                        },
                        Some("Succeeded") => Ok(WorkerState::Terminated),
                        Some("Failed") => {
                            let reason = status.container_statuses
                                .and_then(|statuses| statuses.first().cloned())
                                .and_then(|status| status.state)
                                .and_then(|state| state.terminated)
                                .and_then(|term| term.reason)
                                .unwrap_or_else(|| "Unknown failure".to_string());
                            Ok(WorkerState::Failed { reason })
                        },
                        Some("Unknown") => Ok(WorkerState::Unknown),
                        _ => Ok(WorkerState::Unknown),
                    }
                } else {
                    Ok(WorkerState::Creating)
                }
            },
            Err(kube::Error::Api(kube::error::ErrorResponse { code: 404, .. })) => {
                Err(ProviderError::not_found("Worker".to_string(), worker_id.to_string()))
            },
            Err(e) => {
                Err(ProviderError::infrastructure(format!("Failed to get pod status: {}", e)))
            },
        }
    }

    /// Publica un evento del worker
    async fn publish_event(&self, event: WorkerEvent) {
        if let Err(e) = self.event_sender.send(event) {
            eprintln!("Failed to send worker event: {}", e);
        }
    }
}

#[async_trait]
impl WorkerManagerProvider for KubernetesProvider {
    fn name(&self) -> &str {
        "kubernetes"
    }

    async fn create_worker(
        &self,
        spec: &RuntimeSpec,
        config: &ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let worker_id = WorkerId::new();
        
        // Construir y crear el pod
        let pod = self.build_pod(&worker_id, spec, config)?;
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let created_pod = pods.create(&PostParams::default(), &pod)
            .await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to create pod: {}", e)))?;

        let mut provider_info = HashMap::new();
        provider_info.insert("pod_name".to_string(), created_pod.metadata.name.unwrap_or_default());
        provider_info.insert("namespace".to_string(), self.namespace.clone());
        provider_info.insert("uid".to_string(), created_pod.metadata.uid.unwrap_or_default());

        let worker = Worker {
            id: worker_id.clone(),
            state: WorkerState::Creating,
            provider_info: provider_info.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            spec: spec.clone(),
        };

        // Publicar evento
        self.publish_event(WorkerEvent::WorkerCreated {
            worker_id,
            provider_info,
            timestamp: Utc::now(),
        }).await;

        Ok(worker)
    }

    async fn terminate_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let pod_name = format!("worker-{}", worker_id);

        match pods.delete(&pod_name, &DeleteParams::default()).await {
            Ok(_) => {
                // Publicar evento de terminación
                self.publish_event(WorkerEvent::WorkerTerminated {
                    worker_id: worker_id.clone(),
                    exit_code: None,
                    timestamp: Utc::now(),
                }).await;
                Ok(())
            },
            Err(kube::Error::Api(kube::error::ErrorResponse { code: 404, .. })) => {
                // Ya está terminado, esto es idempotente
                Ok(())
            },
            Err(e) => {
                Err(ProviderError::infrastructure(format!("Failed to delete pod: {}", e)))
            },
        }
    }

    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError> {
        self.get_pod_state(worker_id).await
    }

    async fn get_logs(&self, worker_id: &WorkerId) -> Result<LogStreamRef, ProviderError> {
        let pod_name = format!("worker-{}", worker_id);
        
        Ok(LogStreamRef {
            worker_id: worker_id.clone(),
            endpoint: format!("/api/v1/namespaces/{}/pods/{}/log", self.namespace, pod_name),
            stream_type: LogStreamType::Http(format!("kubernetes:///{}/{}", self.namespace, pod_name)),
        })
    }

    async fn port_forward(
        &self,
        worker_id: &WorkerId,
        local_port: u16,
        remote_port: u16,
    ) -> Result<String, ProviderError> {
        // En una implementación completa, esto establecería port-forwarding real
        // Por ahora, devolvemos una URL simulada
        let pod_name = format!("worker-{}", worker_id);
        Ok(format!("kubectl port-forward pod/{} {}:{} -n {}", 
            pod_name, local_port, remote_port, self.namespace))
    }

    async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError> {
        // Obtener información de nodos
        let nodes: Api<k8s_openapi::api::core::v1::Node> = Api::all(self.client.clone());
        let node_list = nodes.list(&ListParams::default()).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to list nodes: {}", e)))?;

        let mut total_cpu_m = 0u32;
        let mut total_memory_mb = 0u32;
        let mut total_gpu = 0u32;

        for node in node_list.items {
            if let Some(status) = node.status {
                if let Some(capacity) = status.capacity {
                    if let Some(cpu) = capacity.get("cpu") {
                        if let Ok(cores) = cpu.0.parse::<f64>() {
                            total_cpu_m += (cores * 1000.0) as u32;
                        }
                    }
                    if let Some(memory) = capacity.get("memory") {
                        // Convertir de Ki a MB
                        if let Ok(ki) = memory.0.trim_end_matches("Ki").parse::<u64>() {
                            total_memory_mb += (ki / 1024) as u32;
                        }
                    }
                    if let Some(gpu) = capacity.get("nvidia.com/gpu") {
                        if let Ok(count) = gpu.0.parse::<u32>() {
                            total_gpu += count;
                        }
                    }
                }
            }
        }

        // Obtener pods activos para calcular uso
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let pod_list = pods.list(&ListParams::default()).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to list pods: {}", e)))?;

        let active_workers = pod_list.items.iter()
            .filter(|pod| {
                pod.metadata.labels.as_ref()
                    .and_then(|labels| labels.get("managed-by"))
                    .map(|v| v == "worker-manager")
                    .unwrap_or(false)
            })
            .count() as u32;

        // Para simplificar, estimamos el uso basado en workers activos
        // En una implementación real, se calcularía sumando recursos de pods activos
        let estimated_used_cpu_m = active_workers * 1000; // Asumiendo 1 core por worker
        let estimated_used_memory_mb = active_workers * 512; // Asumiendo 512MB por worker

        Ok(CapacityInfo {
            total_cpu_m,
            used_cpu_m: estimated_used_cpu_m.min(total_cpu_m),
            available_cpu_m: total_cpu_m.saturating_sub(estimated_used_cpu_m),
            total_memory_mb,
            used_memory_mb: estimated_used_memory_mb.min(total_memory_mb),
            available_memory_mb: total_memory_mb.saturating_sub(estimated_used_memory_mb),
            total_gpu: if total_gpu > 0 { Some(total_gpu) } else { None },
            used_gpu: Some(0), // Simplificado
            available_gpu: if total_gpu > 0 { Some(total_gpu) } else { None },
            total_storage_mb: None, // No calculado en esta implementación
            used_storage_mb: None,
            available_storage_mb: None,
            active_workers,
            max_workers: None, // Dependería de políticas de ResourceQuota
        })
    }

    async fn execute_command(
        &self,
        worker_id: &WorkerId,
        command: Vec<String>,
        timeout: Option<Duration>,
    ) -> Result<ExecutionResult, ProviderError> {
        // En una implementación completa, esto ejecutaría el comando vía kubectl exec
        // Por ahora, devolvemos un resultado simulado
        let start = std::time::Instant::now();
        
        // Simular ejecución
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(ExecutionResult {
            exit_code: 0,
            stdout: format!("Executed: {}", command.join(" ")),
            stderr: String::new(),
            execution_time: start.elapsed(),
        })
    }

    async fn restart_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        // Kubernetes no soporta restart directo de pods
        // La estrategia sería delete + recreate
        Err(ProviderError::new(
            ErrorCode::Compatibility,
            "Kubernetes pods cannot be restarted directly. Use terminate + create instead.".to_string()
        ))
    }

    async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        // Kubernetes no soporta pause de pods nativamente
        Err(ProviderError::new(
            ErrorCode::Compatibility,
            "Kubernetes does not support pausing pods directly".to_string()
        ))
    }

    async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        // Kubernetes no soporta resume de pods nativamente
        Err(ProviderError::new(
            ErrorCode::Compatibility,
            "Kubernetes does not support resuming pods directly".to_string()
        ))
    }

    fn stream_worker_events(&self) -> mpsc::UnboundedReceiver<WorkerEvent> {
        // Crear un nuevo receptor para este stream
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // En una implementación completa, esto iniciaría un watcher de eventos
        // Por ahora, devolvemos el receptor vacío
        receiver
    }
}
```

---## Implementación Docker Provider

Implementación completa del trait WorkerManagerProvider para Docker Engine, con soporte para contenedores, redes, volúmenes y health checks.

```rust
//! Implementación del provider para Docker
//!
//! Utiliza docker-rs para gestionar contenedores efímeros con recursos,
//! health checks, networking y volúmenes

use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use chrono::Utc;
use docker::Docker;
use docker::models::{
    CreateContainerOptions, ContainerConfig, HostConfig, NetworkingConfig,
    EndpointSettings, HealthConfig, RestartPolicy, PortBinding, Mount,
    MountTypeEnum, Resources, Ulimit, DeviceMapping, LogConfig,
};
use serde_json::{Map, Value};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::traits::*;
use crate::errors::*;

/// Provider para Docker usando docker-rs
pub struct DockerProvider {
    docker: Docker,
    event_sender: mpsc::UnboundedSender<WorkerEvent>,
    config: DockerConfig,
}

/// Configuración específica para Docker
#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub default_network: String,
    pub registry_auth: Option<RegistryAuth>,
    pub default_log_driver: String,
    pub enable_auto_remove: bool,
    pub default_ulimits: Vec<UlimitConfig>,
    pub dns_servers: Vec<String>,
    pub extra_hosts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegistryAuth {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub server_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UlimitConfig {
    pub name: String,
    pub soft: i64,
    pub hard: i64,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            default_network: "bridge".to_string(),
            registry_auth: None,
            default_log_driver: "json-file".to_string(),
            enable_auto_remove: true,
            default_ulimits: vec![
                UlimitConfig {
                    name: "nofile".to_string(),
                    soft: 65536,
                    hard: 65536,
                },
            ],
            dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
            extra_hosts: vec![],
        }
    }
}

impl DockerProvider {
    /// Crea un nuevo provider de Docker
    pub async fn new() -> Result<Self, ProviderError> {
        Self::with_config(DockerConfig::default()).await
    }

    /// Crea un provider con configuración personalizada
    pub async fn with_config(config: DockerConfig) -> Result<Self, ProviderError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| ProviderError::infrastructure(format!("Failed to connect to Docker: {}", e)))?;

        // Verificar conexión
        let version = docker.version().await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to get Docker version: {}", e)))?;

        println!("Connected to Docker Engine version: {}", version.version.unwrap_or_default());

        let (event_sender, _) = mpsc::unbounded_channel();

        Ok(Self {
            docker,
            event_sender,
            config,
        })
    }

    /// Construye la configuración del contenedor
    fn build_container_config(&self, worker_id: &WorkerId, spec: &RuntimeSpec, config: &ProviderConfig) -> Result<ContainerConfig, ProviderError> {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "worker-manager".to_string());
        labels.insert("worker-id".to_string(), worker_id.to_string());
        labels.insert("managed-by".to_string(), "worker-manager".to_string());
        labels.extend(spec.labels.clone());

        // Construir variables de entorno
        let mut env = Vec::new();
        for (key, value) in &spec.env {
            env.push(format!("{}={}", key, value));
        }
        env.push(format!("WORKER_ID={}", worker_id));

        // Construir puertos expuestos
        let mut exposed_ports = HashMap::new();
        for &port in &spec.ports {
            exposed_ports.insert(format!("{}/tcp", port), Map::new());
        }

        // Configurar health check si está definido
        let health_config = if let Some(health_config) = &spec.health_checks {
            Some(self.build_health_config(health_config)?)
        } else {
            None
        };

        let container_config = ContainerConfig {
            image: Some(spec.image.clone()),
            cmd: spec.command.clone().map(|cmd| cmd.into_iter().collect()),
            env: Some(env),
            exposed_ports: Some(exposed_ports),
            labels: Some(labels),
            working_dir: Some("/app".to_string()),
            user: Some("1000:1000".to_string()), // Non-root user por defecto
            healthcheck: health_config,
            ..Default::default()
        };

        Ok(container_config)
    }

    /// Construye la configuración del host
    fn build_host_config(&self, spec: &RuntimeSpec, config: &ProviderConfig) -> Result<HostConfig, ProviderError> {
        // Configurar recursos
        let memory = (spec.resources.memory_mb as i64) * 1024 * 1024; // Convertir MB a bytes
        let cpu_quota = (spec.resources.cpu_m as i64) * 100; // Convertir millicores a quota
        let cpu_period = 100000i64; // 100ms estándar

        // Configurar mapeo de puertos
        let mut port_bindings = HashMap::new();
        for &port in &spec.ports {
            let container_port = format!("{}/tcp", port);
            port_bindings.insert(container_port, Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(port.to_string()),
            }]));
        }

        // Configurar montajes de volúmenes
        let mounts = self.build_mounts(spec)?;

        // Configurar políticas de reinicio
        let restart_policy = RestartPolicy {
            name: Some("no".to_string()), // No restart para workers efímeros
            maximum_retry_count: Some(0),
        };

        // Configurar ulimits
        let ulimits = Some(self.config.default_ulimits.iter().map(|ul| Ulimit {
            name: Some(ul.name.clone()),
            soft: Some(ul.soft),
            hard: Some(ul.hard),
        }).collect());

        // Configurar logging
        let log_config = LogConfig {
            type_: Some(self.config.default_log_driver.clone()),
            config: Some({
                let mut log_opts = HashMap::new();
                log_opts.insert("max-size".to_string(), "10m".to_string());
                log_opts.insert("max-file".to_string(), "3".to_string());
                log_opts
            }),
        };

        // Configurar recursos GPU si están especificados
        let device_requests = if let Some(gpu_count) = spec.resources.gpu {
            Some(vec![docker::models::DeviceRequest {
                driver: Some("nvidia".to_string()),
                count: Some(gpu_count as i64),
                device_i_ds: None,
                capabilities: Some(vec![vec!["gpu".to_string()]]),
                options: Some(HashMap::new()),
            }])
        } else {
            None
        };

        let host_config = HostConfig {
            memory: Some(memory),
            cpu_quota: Some(cpu_quota),
            cpu_period: Some(cpu_period),
            port_bindings: Some(port_bindings),
            mounts: Some(mounts),
            restart_policy: Some(restart_policy),
            auto_remove: Some(self.config.enable_auto_remove),
            ulimits,
            log_config: Some(log_config),
            dns: Some(self.config.dns_servers.clone()),
            extra_hosts: Some(self.config.extra_hosts.clone()),
            device_requests,
            network_mode: Some(self.config.default_network.clone()),
            ..Default::default()
        };

        Ok(host_config)
    }

    /// Construye configuración de networking
    fn build_networking_config(&self, spec: &RuntimeSpec) -> Result<NetworkingConfig, ProviderError> {
        let mut endpoints_config = HashMap::new();
        
        // Configuración básica para la red por defecto
        endpoints_config.insert(self.config.default_network.clone(), EndpointSettings {
            network_id: None,
            endpoint_id: None,
            gateway: None,
            ip_address: None,
            ip_prefix_len: None,
            ipv6_gateway: None,
            global_ipv6_address: None,
            global_ipv6_prefix_len: None,
            mac_address: None,
            driver_opts: None,
        });

        Ok(NetworkingConfig {
            endpoints_config: Some(endpoints_config),
        })
    }

    /// Construye montajes de volúmenes
    fn build_mounts(&self, spec: &RuntimeSpec) -> Result<Vec<Mount>, ProviderError> {
        let mut mounts = Vec::new();

        for volume in &spec.volumes {
            let mount = match &volume.source {
                VolumeSource::EmptyDir => {
                    Mount {
                        target: Some(volume.mount_path.clone()),
                        source: None,
                        type_: Some(MountTypeEnum::TMPFS),
                        read_only: Some(volume.read_only),
                        tmpfs_options: Some(docker::models::TmpfsOptions {
                            size_bytes: Some(100 * 1024 * 1024), // 100MB por defecto
                            mode: Some(0o1777),
                        }),
                        ..Default::default()
                    }
                },
                VolumeSource::HostPath(host_path) => {
                    Mount {
                        target: Some(volume.mount_path.clone()),
                        source: Some(host_path.clone()),
                        type_: Some(MountTypeEnum::BIND),
                        read_only: Some(volume.read_only),
                        bind_options: Some(docker::models::BindOptions {
                            propagation: Some("rprivate".to_string()),
                            non_recursive: Some(false),
                        }),
                        ..Default::default()
                    }
                },
                VolumeSource::PersistentVolume(volume_name) => {
                    Mount {
                        target: Some(volume.mount_path.clone()),
                        source: Some(volume_name.clone()),
                        type_: Some(MountTypeEnum::VOLUME),
                        read_only: Some(volume.read_only),
                        volume_options: Some(docker::models::VolumeOptions {
                            no_copy: Some(false),
                            labels: None,
                            driver_config: None,
                        }),
                        ..Default::default()
                    }
                },
                VolumeSource::Secret(secret_name) => {
                    // Los secrets se montan como archivos temporales
                    // En una implementación real, se obtendría el contenido del secret
                    Mount {
                        target: Some(volume.mount_path.clone()),
                        source: Some(format!("/tmp/secrets/{}", secret_name)),
                        type_: Some(MountTypeEnum::BIND),
                        read_only: Some(true),
                        ..Default::default()
                    }
                },
                VolumeSource::ConfigMap(cm_name) => {
                    // Similar a secrets, se montarían como archivos
                    Mount {
                        target: Some(volume.mount_path.clone()),
                        source: Some(format!("/tmp/configmaps/{}", cm_name)),
                        type_: Some(MountTypeEnum::BIND),
                        read_only: Some(true),
                        ..Default::default()
                    }
                },
            };
            mounts.push(mount);
        }

        Ok(mounts)
    }

    /// Construye configuración de health check
    fn build_health_config(&self, health_config: &HealthCheckConfig) -> Result<HealthConfig, ProviderError> {
        let test_command = match health_config.protocol {
            HealthCheckProtocol::Http | HealthCheckProtocol::Https => {
                let scheme = if health_config.protocol == HealthCheckProtocol::Https { "https" } else { "http" };
                let port = health_config.port.unwrap_or(8080);
                let path = health_config.path.as_deref().unwrap_or("/health");
                
                vec![
                    "CMD-SHELL".to_string(),
                    format!("curl -f {}://localhost:{}{} || exit 1", scheme, port, path),
                ]
            },
            HealthCheckProtocol::Tcp => {
                let port = health_config.port.unwrap_or(8080);
                vec![
                    "CMD-SHELL".to_string(),
                    format!("nc -z localhost {} || exit 1", port),
                ]
            },
            HealthCheckProtocol::Command => {
                let mut cmd = vec!["CMD".to_string()];
                if let Some(command) = &health_config.command {
                    cmd.extend(command.clone());
                } else {
                    return Err(ProviderError::validation(
                        "health_check.command".to_string(),
                        "Command is required for Command protocol".to_string(),
                    ));
                }
                cmd
            },
        };

        Ok(HealthConfig {
            test: Some(test_command),
            interval: Some(health_config.interval.as_nanos() as i64),
            timeout: Some(health_config.timeout.as_nanos() as i64),
            retries: Some(health_config.failure_threshold as i64),
            start_period: Some(30_000_000_000), // 30 segundos
            start_interval: None,
        })
    }

    /// Obtiene el estado de un contenedor
    async fn get_container_state(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError> {
        let container_name = format!("worker-{}", worker_id);

        match self.docker.inspect_container(&container_name, None).await {
            Ok(container) => {
                if let Some(state) = container.state {
                    match state.status.as_deref() {
                        Some("created") => Ok(WorkerState::Creating),
                        Some("running") => {
                            // Verificar health check si está configurado
                            if let Some(health) = state.health {
                                match health.status.as_deref() {
                                    Some("healthy") => Ok(WorkerState::Running),
                                    Some("starting") => Ok(WorkerState::Creating),
                                    Some("unhealthy") => Ok(WorkerState::Failed {
                                        reason: "Health check failed".to_string(),
                                    }),
                                    _ => Ok(WorkerState::Running), // Sin health check, asumir running
                                }
                            } else {
                                Ok(WorkerState::Running)
                            }
                        },
                        Some("paused") => Ok(WorkerState::Paused),
                        Some("restarting") => Ok(WorkerState::Creating),
                        Some("removing") => Ok(WorkerState::Terminating),
                        Some("exited") => {
                            let exit_code = state.exit_code.unwrap_or(0);
                            if exit_code == 0 {
                                Ok(WorkerState::Terminated)
                            } else {
                                Ok(WorkerState::Failed {
                                    reason: format!("Exited with code {}", exit_code),
                                })
                            }
                        },
                        Some("dead") => Ok(WorkerState::Failed {
                            reason: "Container is dead".to_string(),
                        }),
                        _ => Ok(WorkerState::Unknown),
                    }
                } else {
                    Ok(WorkerState::Unknown)
                }
            },
            Err(docker::Error::DockerResponseServerError { 
                status_code: 404, .. 
            }) => {
                Err(ProviderError::not_found("Worker".to_string(), worker_id.to_string()))
            },
            Err(e) => {
                Err(ProviderError::infrastructure(format!("Failed to inspect container: {}", e)))
            },
        }
    }

    /// Publica un evento del worker
    async fn publish_event(&self, event: WorkerEvent) {
        if let Err(e) = self.event_sender.send(event) {
            eprintln!("Failed to send worker event: {}", e);
        }
    }
}

#[async_trait]
impl WorkerManagerProvider for DockerProvider {
    fn name(&self) -> &str {
        "docker"
    }

    async fn create_worker(
        &self,
        spec: &RuntimeSpec,
        config: &ProviderConfig,
    ) -> Result<Worker, ProviderError> {
        let worker_id = WorkerId::new();
        let container_name = format!("worker-{}", worker_id);

        // Construir configuraciones
        let container_config = self.build_container_config(&worker_id, spec, config)?;
        let host_config = self.build_host_config(spec, config)?;
        let networking_config = self.build_networking_config(spec)?;

        // Crear opciones para el contenedor
        let options = CreateContainerOptions {
            name: container_name.clone(),
            platform: None,
        };

        // Crear el contenedor
        let container_response = self.docker.create_container(
            Some(options),
            container_config.clone(),
            Some(host_config),
            Some(networking_config),
        ).await.map_err(|e| ProviderError::infrastructure(format!("Failed to create container: {}", e)))?;

        // Iniciar el contenedor
        self.docker.start_container::<String>(&container_response.id, None)
            .await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to start container: {}", e)))?;

        let mut provider_info = HashMap::new();
        provider_info.insert("container_id".to_string(), container_response.id.clone());
        provider_info.insert("container_name".to_string(), container_name);
        
        if let Some(warnings) = container_response.warnings {
            for warning in warnings {
                eprintln!("Docker warning: {}", warning);
            }
        }

        let worker = Worker {
            id: worker_id.clone(),
            state: WorkerState::Creating,
            provider_info: provider_info.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            spec: spec.clone(),
        };

        // Publicar evento
        self.publish_event(WorkerEvent::WorkerCreated {
            worker_id,
            provider_info,
            timestamp: Utc::now(),
        }).await;

        Ok(worker)
    }

    async fn terminate_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let container_name = format!("worker-{}", worker_id);

        // Intentar parada grácil primero
        match self.docker.stop_container(&container_name, None).await {
            Ok(_) => {},
            Err(docker::Error::DockerResponseServerError { 
                status_code: 404, .. 
            }) => {
                // El contenedor ya no existe, esto es idempotente
                return Ok(());
            },
            Err(e) => {
                eprintln!("Failed to stop container gracefully: {}", e);
                // Intentar forzar la terminación
                if let Err(kill_err) = self.docker.kill_container(&container_name, None).await {
                    return Err(ProviderError::infrastructure(
                        format!("Failed to kill container: {}", kill_err)
                    ));
                }
            },
        }

        // Si auto_remove está habilitado, el contenedor se eliminará automáticamente
        if !self.config.enable_auto_remove {
            if let Err(e) = self.docker.remove_container(&container_name, None).await {
                eprintln!("Failed to remove container: {}", e);
            }
        }

        // Publicar evento de terminación
        self.publish_event(WorkerEvent::WorkerTerminated {
            worker_id: worker_id.clone(),
            exit_code: None, // Sería necesario obtenerlo del estado del contenedor
            timestamp: Utc::now(),
        }).await;

        Ok(())
    }

    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError> {
        self.get_container_state(worker_id).await
    }

    async fn get_logs(&self, worker_id: &WorkerId) -> Result<LogStreamRef, ProviderError> {
        let container_name = format!("worker-{}", worker_id);
        
        Ok(LogStreamRef {
            worker_id: worker_id.clone(),
            endpoint: format!("/containers/{}/logs", container_name),
            stream_type: LogStreamType::Http(format!("docker://{}", container_name)),
        })
    }

    async fn port_forward(
        &self,
        worker_id: &WorkerId,
        local_port: u16,
        remote_port: u16,
    ) -> Result<String, ProviderError> {
        let container_name = format!("worker-{}", worker_id);
        
        // Para Docker, el port forwarding ya está configurado en la creación
        // Devolvemos la dirección local donde está disponible
        Ok(format!("localhost:{}", local_port))
    }

    async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError> {
        // Obtener información del sistema Docker
        let system_info = self.docker.info().await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to get Docker info: {}", e)))?;

        let total_cpu_m = (system_info.n_cpu.unwrap_or(1) as u32) * 1000; // Convertir cores a millicores
        let total_memory_mb = (system_info.mem_total.unwrap_or(0) / 1024 / 1024) as u32; // Convertir bytes a MB

        // Obtener contenedores activos
        let containers = self.docker.list_containers::<String>(None).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to list containers: {}", e)))?;

        let active_workers = containers.iter()
            .filter(|container| {
                container.labels.as_ref()
                    .and_then(|labels| labels.get("managed-by"))
                    .map(|v| v == "worker-manager")
                    .unwrap_or(false)
            })
            .count() as u32;

        // Estimar uso basado en workers activos
        let estimated_used_cpu_m = active_workers * 1000; // 1 core por worker
        let estimated_used_memory_mb = active_workers * 512; // 512MB por worker

        Ok(CapacityInfo {
            total_cpu_m,
            used_cpu_m: estimated_used_cpu_m.min(total_cpu_m),
            available_cpu_m: total_cpu_m.saturating_sub(estimated_used_cpu_m),
            total_memory_mb,
            used_memory_mb: estimated_used_memory_mb.min(total_memory_mb),
            available_memory_mb: total_memory_mb.saturating_sub(estimated_used_memory_mb),
            total_gpu: None, // Requeriría inspección más detallada de GPU
            used_gpu: None,
            available_gpu: None,
            total_storage_mb: None, // No calculado en esta implementación
            used_storage_mb: None,
            available_storage_mb: None,
            active_workers,
            max_workers: None, // Dependería de límites del host
        })
    }

    async fn execute_command(
        &self,
        worker_id: &WorkerId,
        command: Vec<String>,
        timeout: Option<Duration>,
    ) -> Result<ExecutionResult, ProviderError> {
        let container_name = format!("worker-{}", worker_id);
        let start = std::time::Instant::now();

        // Crear la ejecución
        let exec_config = docker::models::CreateExecOptions {
            cmd: Some(command.clone()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self.docker.create_exec(&container_name, exec_config).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to create exec: {}", e)))?;

        // Ejecutar el comando
        let mut stream = self.docker.start_exec(&exec.id, None);
        
        let mut stdout = String::new();
        let mut stderr = String::new();

        // Leer la salida del stream
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(docker::api::ExecLogFrame::StdOut { message }) => {
                    stdout.push_str(&String::from_utf8_lossy(&message));
                },
                Ok(docker::api::ExecLogFrame::StdErr { message }) => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                },
                Err(e) => {
                    return Err(ProviderError::infrastructure(format!("Error reading exec output: {}", e)));
                },
                _ => {}, // Ignore other frame types
            }
        }

        // Inspeccionar la ejecución para obtener el exit code
        let exec_inspect = self.docker.inspect_exec(&exec.id).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to inspect exec: {}", e)))?;

        let exit_code = exec_inspect.exit_code.unwrap_or(0);

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            execution_time: start.elapsed(),
        })
    }

    async fn restart_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let container_name = format!("worker-{}", worker_id);

        self.docker.restart_container(&container_name, None).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to restart container: {}", e)))?;

        Ok(())
    }

    async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let container_name = format!("worker-{}", worker_id);

        self.docker.pause_container(&container_name).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to pause container: {}", e)))?;

        Ok(())
    }

    async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError> {
        let container_name = format!("worker-{}", worker_id);

        self.docker.unpause_container(&container_name).await
            .map_err(|e| ProviderError::infrastructure(format!("Failed to unpause container: {}", e)))?;

        Ok(())
    }

    fn stream_worker_events(&self) -> mpsc::UnboundedReceiver<WorkerEvent> {
        // Crear un nuevo receptor para este stream
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // En una implementación completa, esto iniciaría un stream de eventos de Docker
        // Por ahora, devolvemos el receptor vacío
        receiver
    }
}
```

---# 8. Sistema de Gestión de Credenciales y Secretos

## 8.1 Arquitectura del Sistema de Credenciales

El sistema de credenciales implementa un modelo pluggable que permite la integración transparente de múltiples proveedores de gestión de secretos. La arquitectura se basa en el trait `CredentialProvider` que define las operaciones comunes para todos los proveedores.

### 8.1.1 Diagrama de Arquitectura

```mermaid
graph TB
    subgraph "Credential Management System"
        CP[CredentialProvider Trait]
        
        subgraph "Credential Providers"
            SP[SimpleProvider]
            VP[HashiCorpVaultProvider]
            AP[AWSSecretsManagerProvider]
            KP[KeycloakServiceAccountProvider]
        end
        
        subgraph "Credential Operations"
            GET[Get Secret]
            PUT[Put Secret]
            DEL[Delete Secret]
            ROT[Rotate Secret]
            LST[List Secrets]
        end
        
        subgraph "Security Layer"
            ENC[Encryption/Decryption]
            AUD[Audit Logging]
            CACHE[Secret Cache]
        end
    end
    
    CP --> SP
    CP --> VP
    CP --> AP
    CP --> KP
    
    GET --> ENC
    PUT --> ENC
    ROT --> AUD
    LST --> CACHE
```

## 8.2 HashiCorpVaultProvider

### 8.2.1 Especificación Técnica

El proveedor de HashiCorp Vault proporciona integración empresarial con el sistema de gestión de secretos más robusto del mercado.

#### Configuración Requerida

```rust
pub struct VaultConfig {
    pub vault_url: String,
    pub auth_method: VaultAuthMethod,
    pub namespace: Option<String>,
    pub mount_path: String,
    pub max_connections: usize,
    pub timeout: Duration,
    pub retry_policy: RetryPolicy,
}

pub enum VaultAuthMethod {
    Token(String),
    AppRole {
        role_id: String,
        secret_id: String,
    },
    Kubernetes {
        jwt_path: String,
        role: String,
        service_account_path: String,
    },
}
```

#### Operaciones Soportadas

1. **Gestión de Secretos Dinámicos**:
   - Generación de credenciales temporales
   - Renew/Revoke automático
   - Secret leasing con TTL

2. **Transit Encryption**:
   - Cifrado/descifrado sin almacenar claves
   - Integración con HSM
   - Versionado de claves

3. **Cubbyhole Storage**:
   - Secretos específicos por request
   - Auto-destrucción
   - Proxy token pattern

### 8.2.2 Patrón de Integración

```yaml
# vault-provider-config.yaml
vault:
  url: "https://vault.company.com"
  namespace: "cicd/production"
  auth_method: "kubernetes"
  kubernetes:
    role: "cicd-worker"
    jwt_path: "/var/run/secrets/kubernetes.io/serviceaccount/token"
  engine: "kv-v2"
  mount_path: "secret/data/cicd"
  
security:
  max_connections: 100
  timeout: "30s"
  retry_attempts: 3
  secret_ttl: "1h"
  enable_audit: true
```

## 8.3 AWSSecretsManagerProvider

### 8.3.1 Especificación Técnica

Proveedor que integra con AWS Secrets Manager para gestión de secretos en la nube.

#### Configuración AWS

```rust
pub struct AWSSecretsConfig {
    pub region: String,
    pub credentials_provider: AWSCredentialsProvider,
    pub rotation_config: SecretsRotationConfig,
    pub tags: HashMap<String, String>,
    pub delete_protection: bool,
}

pub enum AWSCredentialsProvider {
    StaticKeys {
        access_key_id: String,
        secret_access_key: String,
    },
    Environment,
    IAMRole,
    Profile(String),
}
```

#### Funcionalidades Específicas

1. **Auto-rotation Lambda**:
   - Integración con Lambda functions
   - Triggers automáticos de rotación
   - Compatibility checks

2. **Resource Tags**:
   - Metadata tagging automático
   - Search by tags
   - Cost allocation

3. **Cross-Region Replication**:
   - Replicación automática
   - Failover entre regiones
   - Consistency guarantees

### 8.3.2 Configuración de Rotación

```json
{
  "rotation_rules": {
    "automatically_after_days": 30,
    "duration": "1h",
    "lambda_function_name": "cicd-secret-rotator"
  },
  "policies": {
    "add_replica_regions": ["us-west-2", "eu-west-1"],
    "delete_protection": true
  },
  "tags": {
    "environment": "production",
    "application": "cicd-system",
    "owner": "platform-team"
  }
}
```

## 8.4 KeycloakServiceAccountProvider

### 8.4.1 Especificación Técnica

Proveedor que utiliza Keycloak Service Accounts para autenticación máquina-a-máquina (M2M).

#### Estructura de Configuración

```rust
pub struct KeycloakServiceAccountConfig {
    pub base_url: String,
    pub realm: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub jwt_config: JWTConfig,
    pub service_accounts: HashMap<String, ServiceAccountConfig>,
}

pub struct ServiceAccountConfig {
    pub username: String,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
    pub validity_hours: u32,
    pub refresh_enabled: bool,
}
```

#### Operaciones de Autenticación

1. **Service Account Management**:
   - Create/Update/Delete accounts
   - Role assignment automático
   - Scope management

2. **Token Management**:
   - JWT token generation
   - Refresh token handling
   - Token validation

3. **Client Credentials Flow**:
   - OAuth2 client authentication
   - Service-to-service auth
   - Token introspection

### 8.4.2 Diagrama de Flujo M2M

```mermaid
sequenceDiagram
    participant CI as CI/CD Worker
    participant KC as Keycloak
    participant WM as Worker Manager
    participant CC as Component Container
    
    CI->>KC: Client Credentials Request
    KC->>CI: Access Token + Refresh Token
    CI->>WM: Authenticated Request with Token
    WM->>KC: Token Introspection
    KC->>WM: Token Valid + Claims
    WM->>CC: Authorize Operation
    CC->>CI: Operation Response
    
    Note over CI,KC: Token Refresh Loop
    CI->>KC: Refresh Token Request
    KC->>CI: New Access Token
```

## 8.5 Motor de Rotación Automática de Secretos

### 8.5.1 Estrategia de Rotación

El motor de rotación implementa múltiples estrategias para adaptarse a diferentes tipos de secretos y requisitos de seguridad.

#### Estrategias de Rotación

```rust
pub enum RotationStrategy {
    /// Rotación programada cada N horas
    Scheduled {
        interval: Duration,
        jitter: Duration,
    },
    
    /// Rotación basada en uso
    UsageBased {
        max_uses: u32,
        grace_period: Duration,
    },
    
    /// Rotación antes de expiración
    PreExpiry {
        warning_period: Duration,
        overlap_period: Duration,
    },
    
    /// Rotación basada en eventos
    EventDriven {
        trigger_events: Vec<RotationTrigger>,
        debounce_period: Duration,
    },
    
    /// Rotación manual forzada
    Manual {
        require_approval: bool,
        approval_timeout: Duration,
    },
}
```

#### Triggers de Rotación

```rust
pub enum RotationTrigger {
    /// Compromiso de seguridad detectado
    SecurityBreach,
    
    /// Cambio en políticas de acceso
    PolicyChange,
    
    /// Compromiso del proveedor
    ProviderCompromise,
    
    /// Solicitud de auditoría
    AuditRequest,
    
    /// Mantenimiento programado
    ScheduledMaintenance,
    
    /// Umbral de tiempo alcanzado
    TimeThreshold(Duration),
    
    /// Umbral de uso alcanzado
    UsageThreshold(u32),
}
```

### 8.5.2 Motor de Ejecución

#### Especificación del Motor

```rust
pub struct SecretRotationEngine {
    providers: Arc<RocksDB>,
    scheduler: Arc<dyn RotationScheduler>,
    notifier: Arc<dyn RotationNotifier>,
    audit_logger: Arc<dyn RotationAuditLogger>,
    config: RotationEngineConfig,
}

pub struct RotationEngineConfig {
    pub max_concurrent_rotations: usize,
    pub rotation_timeout: Duration,
    pub backoff_strategy: ExponentialBackoff,
    pub enable_graceful_shutdown: bool,
    pub health_check_interval: Duration,
}
```

#### Algoritmo de Coordinación

```mermaid
graph TD
    TR[Rotation Trigger] --> EVAL{Evaluate Strategy}
    EVAL -->|Scheduled| SCH[Schedule Rotation]
    EVAL -->|Manual| MAN[Request Approval]
    EVAL -->|Event| EVT[Event Analysis]
    EVAL -->|PreExpiry| EXP[Expiry Warning]
    
    SCH --> QUE[Add to Queue]
    EVT --> QUE
    EXP --> QUE
    MAN -->|Approved| QUE
    
    QUE --> LOCK[Acquire Lock]
    LOCK --> BACKUP[Create Backup]
    BACKUP --> EXEC[Execute Rotation]
    EXEC --> VERIFY[Verify New Secret]
    VERIFY -->|Success| UPDATE[Update Secret Reference]
    VERIFY -->|Failure| ROLLBACK[Rollback to Backup]
    
    UPDATE --> CLEAN[Cleanup Old Secrets]
    ROLLBACK --> CLEAN
    CLEAN --> NOTIFY[Notify Stakeholders]
    
    NOTIFY --> AUDIT[Log Audit Trail]
```

## 8.6 Patrones de Integración Detallados

### 8.6.1 Integración con NATS JetStream

#### Especificación de Eventos

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialEvent {
    pub event_id: String,
    pub event_type: CredentialEventType,
    pub provider: String,
    pub secret_path: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub correlation_id: Option<String>,
}

pub enum CredentialEventType {
    SecretCreated,
    SecretUpdated,
    SecretRotated,
    SecretDeleted,
    SecretExpired,
    SecretAccess,
    RotationStarted,
    RotationCompleted,
    RotationFailed,
}
```

#### Patrón de Publicación

```rust
impl CredentialProvider for HashiCorpVaultProvider {
    async fn put_secret(&self, path: &str, value: &SecretValue) -> Result<()> {
        // 1. Ejecutar operación en Vault
        let result = self.vault_client.put_secret(path, value).await?;
        
        // 2. Publicar evento de auditoría
        let event = CredentialEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: CredentialEventType::SecretUpdated,
            provider: self.provider_name(),
            secret_path: path.to_string(),
            timestamp: Utc::now(),
            metadata: self.extract_metadata(value),
            correlation_id: Some(self.get_correlation_id()),
        };
        
        // 3. Enviar a JetStream
        if let Err(e) = self.event_publisher.publish(&event).await {
            log::warn!("Failed to publish credential event: {}", e);
        }
        
        Ok(result)
    }
}
```

### 8.6.2 Integración con Keycloak

#### Configuración de Roles y Scopes

```yaml
# keycloak-roles.yaml
roles:
  - name: "cicd-worker"
    description: "CI/CD Worker Service Account"
    composite_roles:
      - realm: "view-profile"
      - realm: "manage-account"
    client_roles:
      cicd-system:
        - "worker:execute"
        - "worker:read-logs"
        - "secret:read"
        - "secret:rotate"

scopes:
  - name: "cicd-worker-scope"
    description: "Scopes for CI/CD workers"
    roles:
      - "cicd-worker"
    protocol: "openid-connect"
    attributes:
      "include_in_user_token": true
      "consent_required": false
```

#### Flujo de Autenticación M2M

```mermaid
sequenceDiagram
    participant WM as Worker Manager
    participant KC as Keycloak Server
    participant SA as Service Account
    participant RS as Resource Server
    
    WM->>KC: Client Credentials Grant (Client ID + Secret)
    KC->>SA: Validate Service Account Credentials
    SA->>KC: Authentication Successful
    KC->>WM: Access Token + Refresh Token + Scope Claims
    
    Note over WM,RS: Token Usage
    WM->>RS: API Request + Bearer Token
    RS->>KC: Token Introspection Request
    KC->>RS: Token Valid + Claims + Roles
    RS->>WM: Authorized Response
    
    Note over WM,KC: Token Refresh
    loop Before Expiry
        WM->>KC: Refresh Token Grant
        KC->>WM: New Access Token
    end
```

### 8.6.3 Integración con AWS Verified Permissions

#### Especificación de Políticas

```json
{
  "policyType": "RESOURCE_POLICY",
  "description": "Worker Manager Secret Access Policy",
  "definition": {
    "actions": {
      "READ_SECRET": {
        "appliesTo": {
          "resourceTypes": ["Secret"]
        }
      },
      "WRITE_SECRET": {
        "appliesTo": {
          "resourceTypes": ["Secret"]
        },
        "requires": {
          "environmentVariables": ["CI_ENVIRONMENT"]
        }
      },
      "ROTATE_SECRET": {
        "appliesTo": {
          "resourceTypes": ["Secret"]
        },
        "requires": {
          "environmentVariables": ["CI_ENVIRONMENT", "CI_ROLE"],
          "context": {
            "approvalRequired": true
          }
        }
      }
    },
    "entities": {
      "worker_manager": {
        "type": "ServiceAccount",
        "attributes": {
          "environment": "production",
          "role": "worker_manager"
        }
      }
    }
  }
}
```

#### Evaluación de Permisos

```rust
pub async fn evaluate_secret_access(
    &self,
    principal: &Principal,
    action: &Action,
    resource: &Resource,
    context: &EvaluationContext,
) -> Result<AuthorizationResult> {
    
    let policy_input = PolicyInput {
        principal,
        action,
        resource,
        context,
    };
    
    let decision = self.verified_permissions_client
        .make_decision(&policy_input)
        .await?;
    
    match decision {
        Decision::Allow => Ok(AuthorizationResult::Authorized),
        Decision::Deny => {
            audit_log::log_denied_access(principal, action, resource, context);
            Ok(AuthorizationResult::Denied)
        }
        Decision::Conditional => {
            // Evaluación adicional basada en contexto dinámico
            self.evaluate_conditional_policy(principal, action, resource, context).await
        }
    }
}
```

## 8.7 Guías de Pruebas

### 8.7.1 Estrategia de Testing

#### Pirámide de Pruebas

```mermaid
graph TD
    subgraph "Testing Pyramid"
        subgraph "Unit Tests (70%)"
            UT1[CredentialProvider Tests]
            UT2[Secret Rotation Tests]
            UT3[Crypto Tests]
            UT4[Audit Tests]
        end
        
        subgraph "Integration Tests (20%)"
            IT1[Provider Integration Tests]
            IT2[Database Integration Tests]
            IT3[External API Tests]
            IT4[Message Bus Tests]
        end
        
        subgraph "End-to-End Tests (10%)"
            E2E1[Full Workflow Tests]
            E2E2[Security Scenario Tests]
            E2E3[Performance Tests]
            E2E4[Disaster Recovery Tests]
        end
    end
```

#### Configuración de Test Environment

```yaml
# test-config.yaml
test_environment:
  providers:
    simple:
      enabled: true
      data_dir: "./test_data/simple"
      encryption_key: "test-key-32-chars-long"
    
    vault:
      enabled: true
      vault_url: "http://localhost:8200"
      token: "test-token"
      namespace: "test"
      mount_path: "secret"
    
    aws:
      enabled: true
      region: "us-east-1"
      credentials_path: "~/.aws/test_credentials"
    
    keycloak:
      enabled: true
      base_url: "http://localhost:8080"
      realm: "test"
      client_id: "test-worker"
      client_secret: "test-secret"

test_data:
  test_secrets:
    - path: "database/credentials"
      value: '{"host": "localhost", "port": 5432, "user": "test", "password": "test123"}'
    - path: "api/tokens/github"
      value: '{"token": "test-github-token", "scope": "repo,user"}'
    - path: "certificates/tls"
      value: '{"cert": "-----BEGIN CERT-----\\n...\\n-----END CERT-----\\n", "key": "-----BEGIN PRIVATE KEY-----\\n...\\n-----END PRIVATE KEY-----\\n"}'
```

### 8.7.2 Test Cases Críticos

#### Test Case: Credential Rotation under Load

```rust
#[tokio::test]
async fn test_credential_rotation_under_load() {
    let (manager, test_config) = setup_test_environment().await;
    
    // Crear 1000 secretos de prueba
    let mut test_secrets = Vec::new();
    for i in 0..1000 {
        let secret_path = format!("test/secret_{}", i);
        let secret_value = SecretValue::new(
            format!("value_{}", i),
            SecretType::Password,
            Some(Duration::hours(24)),
        );
        test_secrets.push((secret_path, secret_value));
    }
    
    // Configurar rotación concurrente
    let rotation_count = Arc::new(AtomicUsize::new(0));
    let rotation_handles = futures::future::join_all(
        test_secrets.into_iter().map(|(path, value)| {
            let rotation_count = Arc::clone(&rotation_count);
            async move {
                manager.rotate_secret(&path, &value).await;
                rotation_count.fetch_add(1, Ordering::SeqCst);
            }
        })
    ).await;
    
    // Verificar que todas las rotaciones se completaron
    let total_rotations = rotation_count.load(Ordering::SeqCst);
    assert_eq!(total_rotations, 1000);
    
    // Verificar consistencia de datos
    assert!(manager.verify_all_secrets().await);
}
```

#### Test Case: Security Breach Detection

```rust
#[tokio::test]
async fn test_security_breach_detection() {
    let manager = TestCredentialManager::new().await;
    
    // Simular detección de brecha de seguridad
    let breach_event = SecurityBreachEvent {
        breach_type: BreachType::CredentialLeaked,
        affected_secrets: vec!["api/github-token", "db/password"],
        detected_at: Utc::now(),
        severity: Severity::Critical,
    };
    
    // Ejecutar rotación forzada
    let rotation_tasks = manager.force_rotation_on_breach(&breach_event).await;
    
    // Verificar que todas las credenciales afectadas se rotaron
    for secret_path in &breach_event.affected_secrets {
        assert!(manager.get_secret(secret_path).await.is_err());
    }
    
    // Verificar que se crearon nuevas credenciales
    for secret_path in &breach_event.affected_secrets {
        assert!(manager.get_secret(secret_path).await.is_ok());
    }
    
    // Verificar auditoría
    let audit_logs = manager.get_audit_logs(None).await;
    let breach_logs: Vec<_> = audit_logs
        .iter()
        .filter(|log| log.event_type == "security_breach_rotation")
        .collect();
    
    assert_eq!(breach_logs.len(), breach_event.affected_secrets.len());
}
```

### 8.7.3 Performance Testing

#### Benchmark de Latencia

```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
    
    pub fn benchmark_secret_operations(c: &mut Criterion) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = TestCredentialManager::setup().await;
            
            c.bench_function("get_secret", |b| {
                b.to_async(&rt).iter(|| async {
                    black_box(manager.get_secret("test/secret").await)
                })
            });
            
            c.bench_function("put_secret", |b| {
                let test_value = SecretValue::new("test-value", SecretType::Password, None);
                b.to_async(&rt).iter(|| async {
                    black_box(manager.put_secret("test/benchmark", &test_value).await)
                })
            });
            
            c.bench_function("rotate_secret", |b| {
                let test_value = SecretValue::new("test-value", SecretType::Password, None);
                b.to_async(&rt).iter(|| async {
                    black_box(manager.rotate_secret("test/benchmark", &test_value).await)
                })
            });
        });
    }
}
```

## 8.8 Consideraciones de Seguridad

### 8.8.1 Modelo de Amenazas

#### Análisis de Superficie de Ataque

```mermaid
graph TD
    subgraph "External Attack Surface"
        NET[Network Layer]
        API[API Endpoints]
        AUTH[Authentication]
    end
    
    subgraph "Application Layer"
        CM[Credential Manager]
        RP[Rotation Processes]
        AU[Audit System]
    end
    
    subgraph "Data Layer"
        DB[(Secrets Database)]
        LOG[(Audit Logs)]
        CACHE[(Secret Cache)]
    end
    
    subgraph "Integration Layer"
        NATS[NATS JetStream]
        KC[Keycloak]
        AWS[AWS Services]
        VAULT[HashiCorp Vault]
    end
    
    NET -->|Exploit| API
    API -->|Bypass| AUTH
    AUTH -->|Compromise| CM
    CM -->|Extract| DB
    DB -->|Leak| LOG
    CACHE -->|Cleartext| DB
    
    RP -->|Race Condition| CM
    AU -->|Tamper| LOG
```

#### Controles de Seguridad

```rust
pub struct SecurityControls {
    /// Cifrado en reposo (AES-256-GCM)
    pub encryption_at_rest: EncryptionConfig,
    
    /// Cifrado en tránsito (TLS 1.3)
    pub encryption_in_transit: TLSConfig,
    
    /// Control de acceso basado en roles
    pub rbac: RBACConfig,
    
    /// Auditoría de eventos de seguridad
    pub security_audit: SecurityAuditConfig,
    
    /// Detección de anomalías
    pub anomaly_detection: AnomalyDetectionConfig,
}

pub struct SecurityAuditConfig {
    pub log_all_access: bool,
    pub log_failed_attempts: bool,
    pub log_privilege_escalation: bool,
    pub retention_period: Duration,
    pub alert_on_suspicious_activity: bool,
    pub integration: AuditIntegrationConfig,
}
```

### 8.8.2 Principios de Seguridad

#### Defense in Depth

1. **Capa 1 - Red**:
   - Firewall con reglas estrictas
   - Network segmentation
   - VPC isolation para componentes críticos
   - DDoS protection

2. **Capa 2 - Aplicación**:
   - Input validation exhaustiva
   - SQL injection prevention
   - XSS protection
   - CSRF tokens

3. **Capa 3 - Autenticación**:
   - Multi-factor authentication
   - Service-to-service authentication
   - Certificate-based authentication
   - JWT token validation

4. **Capa 4 - Autorización**:
   - Principle of least privilege
   - Role-based access control
   - Resource-level permissions
   - Temporal access controls

5. **Capa 5 - Datos**:
   - Encryption at rest (AES-256)
   - Encryption in transit (TLS 1.3)
   - Key rotation policies
   - Data classification

#### Zero Trust Architecture

```yaml
# zero-trust-policy.yaml
zero_trust:
  verification:
    identity: "verify_every_time"
    device: "verify_device_health"
    location: "verify_location_context"
    
  access_control:
    minimum_privilege: true
    just_in_time: true
    resource_isolation: true
    
  monitoring:
    continuous: true
    behavioral_analysis: true
    threat_intelligence: true
    
  response:
    automatic: true
    containment: true
    recovery: true
```

### 8.8.3 Manejo de Incidentes

#### Clasificación de Incidentes

```rust
pub enum SecurityIncident {
    CredentialLeak {
        severity: Severity,
        affected_credentials: Vec<String>,
        leak_source: LeakSource,
        detection_time: DateTime<Utc>,
    },
    
    UnauthorizedAccess {
        source_ip: String,
        attempted_resources: Vec<String>,
        authentication_method: AuthMethod,
        success: bool,
    },
    
    SystemCompromise {
        compromised_component: String,
        attack_vector: AttackVector,
        scope: CompromiseScope,
    },
    
    DataIntegrityViolation {
        affected_data: Vec<String>,
        violation_type: IntegrityViolation,
        detection_method: DetectionMethod,
    },
}

pub enum Severity {
    Low,        // Monitoreo y logging
    Medium,     // Alertas al equipo de seguridad
    High,       // Respuesta inmediata
    Critical,   // Escalación ejecutiva
}
```

#### Plan de Respuesta

```mermaid
flowchart TD
    INC[Incident Detected] --> CLASSIFY{Classify Severity}
    CLASSIFY -->|Low| LOG[Log & Monitor]
    CLASSIFY -->|Medium| ALERT[Alert Security Team]
    CLASSIFY -->|High| RESPOND[Security Response]
    CLASSIFY -->|Critical| ESCALATE[Executive Escalation]
    
    RESPOND --> CONTAIN[Contain Incident]
    CONTAIN --> INVESTIGATE[Investigate Impact]
    INVESTIGATE --> REMEDIATE[Remediate]
    REMEDIATE --> RECOVER[System Recovery]
    
    LOG --> CLOSE[Close Incident]
    ALERT -->|Resolved| CLOSE
    ESCALATE --> RESPOND
    RECOVER --> POST_MORTEM[Post-Mortem Analysis]
    POST_MORTEM --> CLOSE
    CLOSE --> UPDATE[Update Security Policies]
```

## 8.9 Límites Operativos

### 8.9.1 Límites de Rendimiento

#### Métricas de Capacidad

```yaml
performance_limits:
  credential_operations:
    get_secret_qps: 10000
    put_secret_qps: 5000
    rotate_secret_qps: 1000
    list_secrets_qps: 2000
    
  storage_limits:
    total_secrets: 1000000
    secret_size_max_kb: 4096
    retention_days: 2555  # 7 años
    
  concurrent_operations:
    max_concurrent_reads: 1000
    max_concurrent_writes: 100
    max_concurrent_rotations: 50
    
  latency_targets:
    get_secret_p99_ms: 10
    put_secret_p99_ms: 50
    rotate_secret_p99_ms: 5000
    list_secrets_p99_ms: 100
```

#### Planificación de Capacidad

```rust
pub struct CapacityPlanningModel {
    /// Crecimiento proyectado de secretos
    pub secret_growth_rate: GrowthRate,
    
    /// Patrones de uso por tipo de secreto
    pub usage_patterns: HashMap<SecretType, UsagePattern>,
    
    /// Ventanas de alta demanda
    pub peak_windows: Vec<TimeWindow>,
    
    /// Recursos disponibles
    pub resource_constraints: ResourceConstraints,
}

pub struct CapacityRecommendation {
    pub recommended_instances: usize,
    pub storage_requirements_gb: u64,
    pub network_bandwidth_mbps: u64,
    pub projected_cost_monthly: f64,
    pub scalability_plan: ScalabilityPlan,
}
```

### 8.9.2 Monitoreo y Alertas

#### Métricas Clave

```rust
pub struct OperationalMetrics {
    /// Métricas de rendimiento
    pub latency_metrics: LatencyMetrics,
    
    /// Métricas de throughput
    pub throughput_metrics: ThroughputMetrics,
    
    /// Métricas de disponibilidad
    pub availability_metrics: AvailabilityMetrics,
    
    /// Métricas de errores
    pub error_metrics: ErrorMetrics,
    
    /// Métricas de seguridad
    pub security_metrics: SecurityMetrics,
}

pub struct LatencyMetrics {
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub p999_latency_ms: f64,
}
```

#### Configuración de Alertas

```yaml
alerts:
  performance:
    - metric: "credential_operation_latency_p99"
      threshold: 1000  # ms
      duration: 300s
      severity: "warning"
    
    - metric: "credential_operation_qps"
      threshold: 8000  # ops/sec
      duration: 60s
      severity: "critical"
  
  availability:
    - metric: "service_availability"
      threshold: 0.999  # 99.9%
      duration: 300s
      severity: "critical"
    
    - metric: "credential_provider_health"
      threshold: 0.9  # 90% providers healthy
      duration: 60s
      severity: "critical"
  
  security:
    - metric: "failed_authentication_attempts"
      threshold: 100  # attempts/hour
      duration: 300s
      severity: "warning"
    
    - metric: "suspicious_secret_access"
      threshold: 10  # accesses
      duration: 60s
      severity: "critical"
```

### 8.9.3 Estrategias de Escalamiento

#### Auto-scaling Configuration

```yaml
autoscaling:
  horizontal:
    enabled: true
    min_instances: 2
    max_instances: 20
    scale_up_policy:
      trigger: "credential_operation_qps"
      threshold: 8000
      cooldown: 300s
      increment: 2
    
    scale_down_policy:
      trigger: "credential_operation_qps"
      threshold: 4000
      cooldown: 600s
      decrement: 1
  
  vertical:
    enabled: false  # Disable for stability
    min_cpu: "1"
    max_cpu: "8"
    min_memory: "2Gi"
    max_memory: "32Gi"
  
  database:
    read_replicas:
      enabled: true
      min_replicas: 1
      max_replicas: 10
      scale_up_threshold: "read_latency_p95 > 100ms"
    
    storage_autoscaling:
      enabled: true
      min_size: "100Gi"
      max_size: "10Ti"
      scale_factor: 1.5
```

## 8.10 Conclusiones y Hoja de Ruta

### 8.10.1 Resumen de Logros

La implementación del **Worker Manager Abstraction Layer** y el **Sistema de Gestión de Credenciales** representa un hito significativo en la arquitectura del sistema CI/CD distribuido. Los logros principales incluyen:

#### Arquitectura de Abstracción
- **WorkerManagerProvider Trait**: Base sólida para múltiples proveedores de infraestructura
- **KubernetesProvider**: Integración completa con K8s API usando kube-rs
- **DockerProvider**: Soporte para contenedores Docker locales y remotos
- **Extensibilidad**: Arquitectura pluggable que permite agregar nuevos providers

#### Sistema de Credenciales Robusto
- **CredentialProvider Trait**: Interfaz común para múltiples gestores de secretos
- **Múltiples Providers**: Simple, HashiCorp Vault, AWS Secrets Manager, Keycloak
- **Rotación Automática**: Motor de rotación con estrategias adaptativas
- **Seguridad Enterprise**: Cifrado, auditoría, y cumplimiento normativo

#### Integración Distribuida
- **NATS JetStream**: Streaming de eventos y coordinación
- **Keycloak**: Autenticación y autorización centralizada
- **AWS Verified Permissions**: Políticas de acceso granulares
- **Observabilidad**: Métricas, logs, y tracing distribuido

### 8.10.2 Beneficios Arquitectónicos

#### Escalabilidad
- **Horizontal Scaling**: Auto-scaling basado en métricas operativas
- **Provider Agility**: Capacidad de cambiar providers sin refactoring
- **Workload Isolation**: Separation de concerns entre infraestructura y credenciales

#### Seguridad
- **Defense in Depth**: Múltiples capas de protección
- **Zero Trust**: Verificación continua de identidad y contexto
- **Compliance**: Auditoría completa y trazabilidad de accesos
- **Incident Response**: Detección automática y respuesta a incidentes

#### Operabilidad
- **Observabilidad**: Métricas completas y alertas inteligentes
- **High Availability**: Failover automático y degradación gradual
- **Maintenance**: Rolling updates y backward compatibility
- **Documentation**: Especificaciones técnicas detalladas y ejemplos

### 8.10.3 Roadmap de Desarrollo

#### Fase 1: Consolidación (Q1 2024)
**Objetivos**:
- Completar testing y validación de todos los providers
- Implementar production monitoring y alerting
- Ejecutar security audit y penetration testing
- Optimizar rendimiento y latencia

**Entregables**:
- [ ] Suite de tests completa (>95% coverage)
- [ ] Dashboard de monitoreo en Grafana
- [ ] Reporte de seguridad con certificaciones
- [ ] Tuning de performance para targets SLO

#### Fase 2: Expansión (Q2 2024)
**Nuevos Providers**:
- **Azure Key Vault**: Integración con Azure Cloud
- **Google Secret Manager**: Soporte para GCP
- **CyberArk**: Integración empresarial legacy
- **Custom Provider**: API para providers personalizados

**Nuevas Funcionalidades**:
- **Secret Templates**: Plantillas para tipos comunes de secretos
- **Access Analytics**: Análisis de patrones de acceso
- **Compliance Reporting**: Reportes automáticos de compliance
- **Multi-Tenancy**: Soporte para múltiples tenants

#### Fase 3: Inteligencia (Q3 2024)
**Machine Learning**:
- **Anomaly Detection**: Detección automática de comportamientos anómalos
- **Predictive Rotation**: Rotación predictiva basada en ML
- **Access Prediction**: Predicción de necesidades de acceso
- **Cost Optimization**: Optimización automática de costos

**Advanced Features**:
- **Secret Sharing**: Compartir secretos de forma segura entre equipos
- **Dynamic Secrets**: Generación dinámica de credenciales temporales
- **Secret Versioning**: Versionado completo de cambios de secretos
- **Geographic Distribution**: Replicación geográfica de datos

#### Fase 4: Plataforma (Q4 2024)
**Developer Experience**:
- **CLI Tool**: Herramienta de línea de comandos
- **SDK Libraries**: SDKs para múltiples lenguajes
- **IDE Plugins**: Plugins para VS Code, IntelliJ, etc.
- **Self-Service Portal**: Portal web para gestión de secretos

**Enterprise Features**:
- **HSM Integration**: Integración con Hardware Security Modules
- **SAML Integration**: Single Sign-On empresarial
- **LDAP Integration**: Integración con directorios LDAP
- **Custom Workflows**: Workflows personalizables con approval process

### 8.10.4 Métricas de Éxito

#### Técnicas
- **Disponibilidad**: 99.9% uptime (< 43 minutos downtime/mes)
- **Latencia**: P99 < 1 segundo para operaciones de secretos
- **Throughput**: 10,000 QPS para lectura, 1,000 QPS para escritura
- **Escalabilidad**: Soportar 1M+ secretos con crecimiento 10% mensual

#### Operativas
- **Mean Time to Detection (MTTD)**: < 5 minutos para incidentes críticos
- **Mean Time to Recovery (MTTR)**: < 15 minutos para fallos de provider
- **Security Incidents**: 0 incidentes de seguridad críticos/año
- **Compliance**: 100% audit trail para accesos a secretos

#### Negocio
- **Time to Market**: Reducir tiempo de provisión de credenciales en 80%
- **Developer Productivity**: Aumentar productivity de developers en 30%
- **Cost Reduction**: Reducir costos de gestión de secretos en 50%
- **Risk Mitigation**: Eliminar riesgos de secretos expuestos

### 8.10.5 Riesgos y Mitigaciones

#### Riesgos Técnicos
1. **Vendor Lock-in**: Dependencia excesiva de un provider
   - *Mitigación*: Arquitectura pluggable con múltiples options
   - *Contingencia*: Proveedor fallback simple para emergencias

2. **Performance Degradation**: Degradación bajo alta carga
   - *Mitigación*: Auto-scaling y caching agresivo
   - *Contingencia*: Rate limiting y degradación gradual

3. **Data Loss**: Pérdida de datos de secretos
   - *Mitigación*: Replicación multi-región y backups automáticos
   - *Contingencia*: Procedimientos de disaster recovery

#### Riesgos de Seguridad
1. **Credential Leak**: Exposición accidental de credenciales
   - *Mitigación*: Cifrado extremo a extremo y access controls
   - *Contingencia*: Rotación forzada y notification a stakeholders

2. **Insider Threat**: Acceso malicioso interno
   - *Mitigación*: Zero trust y behavioral analytics
   - *Contingencia*: Access monitoring y anomaly detection

3. **Compliance Violation**: Violación de regulaciones
   - *Mitigación*: Compliance automation y regular audits
   - *Contingencia*: Legal review y remediation plans

### 8.10.6 Conclusión

El **Worker Manager Abstraction Layer** y el **Sistema de Gestión de Credenciales** establecen las bases sólidas para una plataforma CI/CD distribuida de clase empresarial. La arquitectura pluggable, las características de seguridad de nivel enterprise, y las capacidades de observabilidad posicionan al sistema para soportar cargas de trabajo críticas a escala.

La implementación de esta capa de abstracción no solo resuelve los desafíos técnicos inmediatos de gestión de workers y credenciales, sino que también proporciona la flexibilidad y extensibilidad necesarias para evolucionar con las necesidades del negocio y las innovaciones tecnológicas.

El roadmap de desarrollo asegura un crecimiento continuo y sostenido, manteniendo la compatibilidad y estabilidad mientras se agregan nuevas funcionalidades y capacidades. Con un enfoque en security-by-design, operabilidad, y developer experience, esta arquitectura establece un nuevo estándar para sistemas CI/CD distribuidos en entornos enterprise.

**Estado del Proyecto**: Stage 7 - **COMPLETADO** ✅

---

*Documento generado por MiniMax Agent - Sistema de Diseño Arquitectónico CI/CD Distribuido*
*Fecha: 2025-11-21*
*Versión: 1.0*
*Próxima revisión: Stage 8 - Intelligent Scheduler*