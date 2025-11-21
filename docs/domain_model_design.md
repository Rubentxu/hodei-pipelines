# Blueprint del Modelo de Dominio DDD y Diseño de Integración de Seguridad para una Plataforma CI/CD en Rust

## 1. Propósito, alcance y narrativa del modelo

Este documento especifica el modelo de dominio táctico y la integración de seguridad para una plataforma de Integración y Despliegue Continuos (CI/CD) multicontenido y cloud‑native, implementada en Rust. El objetivo es convertir la arquitectura distribuida ya definida en un diseño operativo y verificable: un modelo por contexto delimitado (bounded context) con agregados, entidades, objetos de valor y eventos de dominio; contratos API y eventos; flujos de orquestación y Sagas; y una capa de seguridad que combine autenticación OpenID Connect (OIDC) con OAuth2, autorización policy‑based y autenticación mutua Transport Layer Security (mTLS).

El alcance abarca siete contextos: Orchestration, Scheduling, Worker Management, Execution, Telemetry, Security y Artifact Management. La narrativa progresa desde el qué (modelo y límites), al cómo (contratos, flujos, seguridad), y al para qué (trazabilidad, decisiones, riesgos y roadmap). El enfoque se fundamenta en el análisis de dominios para microservicios y los principios del Diseño Orientado al Dominio (DDD), que recomiendan límites claros, contratos explícitos y autonomía por servicio[^1][^7].

El resultado esperado es un documento de arquitectura y especificación técnica que sirva como contrato de implementación para equipos de plataforma, seguridad, backend y SRE. Sirve de puente entre el diseño estratégico (bounded contexts y responsabilidades) y la ejecución (agregados, traits y eventos), a la vez que detalla la interacción con autenticación/autorización y los mecanismos de observabilidad y resiliencia.

## 2. Visión general de arquitectura y bounded contexts

La plataforma se compone de siete contextos con responsabilidades bien definidas y acoplamiento débil. El Orchestration Context gobierna el ciclo de vida de jobs y pipelines mediante agregados y Sagas; el Scheduling Context ofrece visibilidad de capacidad y aplica políticas de asignación; el Worker Management Context abstrae la infraestructura a través de un proveedor; el Execution Context ejecuta pasos en workers efímeros; el Telemetry Context centraliza métricas, trazas y eventos; el Security Context provee identidad, políticas y pruebas de autorización; y el Artifact Management Context gestiona artefactos y dependencias a lo largo del ciclo de vida.

La comunicación prioriza eventos en el bus de mensajes y APIs REST documentadas, siguiendo patrones consolidados de microservicios (database per service, Sagas, idempotencia y resiliencia)[^2]. La instrumentación y trazabilidad se integran desde el diseño para facilitar diagnósticos, auditoría y optimización.

Para clarificar responsabilidades y contratos, el siguiente mapa sintetiza los bounded contexts y su interacción.

Tabla 1. Mapa de bounded contexts, agregados y contratos

| Bounded Context     | Propósito                                    | Agregados/Entidades/VO                                                           | Eventos clave                                                        | Integraciones                             |
| ------------------- | -------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------- | ----------------------------------------- |
| Orchestration       | Coordinar ciclo de vida de pipelines y jobs  | Agregados: Pipeline, Job; Entidades: Step, Assignment; VO: JobSpec, PipelineSpec | JobRequested, JobStarted, JobCompleted, JobFailed, JobCancelled      | NATS (control-plane), REST interno        |
| Scheduling          | Asignación óptima y visibilidad de recursos  | Entidades: Offer, Node, Constraint; VO: ResourceQuota, AffinityPolicy            | OfferCreated, AssignmentCreated, AssignmentRejected, CapacityUpdated | NATS, Worker Manager                      |
| Worker Management   | Abstraer infraestructura y operar ejecutores | Entidades: Worker, ProviderEndpoint; VO: ProviderConfig, RuntimeSpec             | WorkerCreated, WorkerTerminated, WorkerFailed                        | Kubernetes/Docker API, eventos            |
| Execution           | Ejecutar pasos en aislamiento y reportar     | Entidades: Execution, StepRun; VO: ExecResult, LogStreamRef                      | StepStarted, StepCompleted, StepFailed                               | Orquestador, streaming de logs            |
| Telemetry           | Métricas, trazas y alertas                   | Entidades: Metric, Trace; VO: TagSet, TraceContext                               | MetricEmitted, TraceSpans, AlertRaised                               | NATS, OpenTelemetry                       |
| Security            | Identidad, autorización y auditoría          | Entidades: AuthorizationDecision, AuthSession; VO: Principal, Scope              | AuthGranted, AuthRevoked, AuthorizationDenied                        | OIDC (Keycloak), AWS Verified Permissions |
| Artifact Management | Gestión de artefactos y dependencias         | Entidades: Artifact, Dependency; VO: ArtifactRef, Checksum                       | ArtifactUploaded, ArtifactPromoted, ArtifactValidated                | Repositorios/registries, NATS             |

Este mapa guía el diseño táctico: cada contexto posee su modelo y sus invariantes, y se integra por eventos y APIs con semánticas de entrega configurables. La consistencia entre servicios se logra con idempotencia y Sagas donde existan transacciones distribuidas[^2].

## 3. Principios de modelado táctico en Rust

El modelado táctico se aplica con rigor para lograr claridad de dominio, encapsulamiento y verificabilidad. En Rust:

- Agregados raíz, entidades y objetos de valor. Los agregados encapsulan invariantes y límites de consistencia; las entidades poseen identidad y ciclo de vida; los objetos de valor son inmutables y se utilizan para IDs, especificaciones, políticas y mediciones.
- Optimización de rendimiento. Tipos Copy para objetos pequeños; `Arc<RwLock<>>` para estado compartido cuando sea estrictamente necesario; tipos atómicos para contadores; elección de allocators específicos cuando el perfil de memoria lo justifique.
- Concurrencia. Modelo de actores para aislar estado y comportamiento; `async/await` coherente con un único runtime (Tokio); trait objects para plugin‑like en providers; backpressure y límites de cola para evitar saturación.
- Serialización. Serde para JSON en APIs y OpenAPI; binario eficiente en canales de telemetría/logs cuando el perfil de latencia/throughput lo requiera.
- Errores. Tipos de error específicos por contexto, con categorías claras (validación, autorización, infraestructura, estado) y correlación a eventos y observabilidad.

La combinación de DDD y el patrón de actores en Tokio/Actix proporciona un marco robusto para construir sistemas reactivos y tolerantes a fallos[^10][^2].

## 4. Modelo de Dominio por Bounded Context

La siguiente tabla resume eventos de dominio por contexto, que luego se detallan en cada sección.

Tabla 2. Eventos de dominio por contexto

| Contexto | Evento | Descripción | Payload mínimo |
|---|---|---|---|
| Orchestration | JobRequested | Solicitud de ejecución de un job | JobId, Spec, CorrelationId, TenantId |
| Orchestration | JobStarted | Inicio de ejecución del job | JobId, WorkerId |
| Orchestration | JobCompleted | Finalización exitosa del job | JobId, ResultRef |
| Orchestration | JobFailed | Fallo del job con motivo | JobId, ErrorCode, Retryable |
| Orchestration | JobCancelled | Cancelación solicitada | JobId, Reason |
| Scheduling | OfferCreated | Oferta de capacidad disponible | OfferId, NodeId, ResourceQuota |
| Scheduling | AssignmentCreated | Asignación de job a nodo | AssignmentId, JobId, NodeId, Policy |
| Scheduling | AssignmentRejected | Rechazo de asignación | AssignmentId, Reason |
| Scheduling | CapacityUpdated | Cambio de capacidad en nodo | NodeId, ResourceQuota, Timestamp |
| Worker Management | WorkerCreated | Worker creado | WorkerId, ProviderId, RuntimeSpec |
| Worker Management | WorkerTerminated | Worker terminado | WorkerId, ExitCode |
| Worker Management | WorkerFailed | Fallo en worker | WorkerId, Reason |
| Execution | StepStarted | Inicio de step | StepId, JobId, ExecId |
| Execution | StepCompleted | Finalización de step | StepId, ResultRef, DurationMs |
| Execution | StepFailed | Fallo de step | StepId, ErrorCode, Retryable |
| Telemetry | MetricEmitted | Métrica emitida | MetricName, Value, Tags |
| Telemetry | TraceSpans | Spans de traza | TraceContext, Spans |
| Telemetry | AlertRaised | Alerta generada | AlertId, Severity, Message |
| Security | AuthGranted | Autenticación/autorización otorgada | Principal, Scope, DecisionId |
| Security | AuthRevoked | Revocación de acceso | Principal, Scope, DecisionId |
| Security | AuthorizationDenied | Acceso denegado | Resource, Action, Reason |
| Artifact | ArtifactUploaded | Artefacto subido | ArtifactRef, Checksum |
| Artifact | ArtifactPromoted | Artefacto promovido a entorno | ArtifactRef, Env |
| Artifact | ArtifactValidated | Validación de artefacto | ArtifactRef, Status |

### 4.1 Orchestration Context (Pipeline Orchestration)

El Orchestration Context es el núcleo de coordinación. Su propósito es orquestar el ciclo de vida de pipelines y jobs, aplicar Sagas para consistencia distribuida y gobernar reintentos e idempotencia.

Agregados y entidades clave:

- Agregado Pipeline. Raíz que agrupa uno o más jobs y sus invariantes: orden lógico, políticas de compensaciones, estados agregados.
- Agregado Job. Raíz que representa una unidad de trabajo; su estado evoluciona de PENDING a SCHEDULED, RUNNING y terminal (SUCCEEDED, FAILED, CANCELLED). Gestiona reintentos idempotentes por Step.
- Entidad Step. Unidad atómica de ejecución dentro de un Job; posee identidad propia (StepId), política de retry y tiempo máximo.
- Entidad Assignment. Asociación entre un Job y un nodo/worker; registra política aplicada y resultados de asignación.

Objetos de valor inmutables:

- JobSpec. Especificación de requisitos de ejecución (recursos, imagen, comandos, variables de entorno, secrets refs). Invariantes: no vacío, límites superiores de recursos, semántica de comandos.
- PipelineSpec. Estructura declarativa de pasos y dependencias; invariantes: DAG válido sin ciclos, pasos referenciados existentes.

Eventos de dominio:

- JobRequested, JobStarted, JobCompleted, JobFailed, JobCancelled. Publicados en NATS con `CorrelationId` y `TenantId`, habilitando trazabilidad end‑to‑end.
- StepStarted, StepCompleted, StepFailed. Emitidos por el Execution Context, consumidos por Orchestration para transición de estado y Sagas.

Sagas y flujo de orquestación:

- Saga de compensación para fallos. Si un Step falla irreversiblemente, el Orchestration evalúa compensaciones en pasos previos o en recursos externos, preservando idempotencia y publicando eventos de compensación.
- Reintentos idempotentes. El Job agrega política de backoff y límites; la idempotencia se garantiza por `StepId` y claves de dedup en el bus.

Estados del job y transiciones:

Tabla 3. Estados de Job y transiciones permitidas

| Estado | Transiciones salientes | Disparador | Acción |
|---|---|---|---|
| PENDING | → SCHEDULED | JobRequested | Planificador genera OfferCreated y AssignmentCreated |
| SCHEDULED | → RUNNING | WorkerCreated/AssignmentAccepted | Orquestador marca RUNNING; inicia step inicial |
| RUNNING | → SUCCEEDED | JobCompleted | Persiste resultado; publica JobCompleted |
| RUNNING | → FAILED | JobFailed | Inicia compensación (Saga) según política |
| RUNNING | → PENDING (retry) | StepFailed con retry | Programa reintento con backoff e idempotencia |
| SCHEDULED/RUNNING | → CANCELLED | JobCancelled | Cancela assignments; pide terminación a Worker Manager |
| Any | → TERMINATED | WorkerTerminated | Limpia recursos; decide reintento/compensación |

Catálogo de endpoints internos (Orchestration):

Tabla 4. Catálogo de endpoints internos del Orchestration Context

| Método | Ruta | Semántica | Idempotencia |
|---|---|---|---|
| POST | /jobs | Crear job (JobRequested) | Sí, por JobId |
| GET | /jobs/{id} | Consultar estado | N/A |
| GET | /pipelines/{id} | Consultar pipeline | N/A |
| POST | /jobs/{id}/cancel | Cancelar job (JobCancelled) | Sí, por JobId |

Contratos y trazabilidad. Los eventos usan subjects jerárquicos en NATS y payload con correlación y metadatos de traza; las APIs se publican con OpenAPI como contrato formal[^14][^4].

Especificación Rust (ilustrativa):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(u64);

#[derive(Debug, Clone)]
pub struct JobSpec {
    pub image: String,
    pub command: Vec<String>,
    pub resources: ResourceQuota,
    pub timeout_ms: u64,
    pub retries: u8,
    pub env: HashMap<String, String>,
    pub secret_refs: Vec<String>,
}

impl JobSpec {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.command.is_empty() { return Err(DomainError::Validation("command empty")); }
        if self.timeout_ms == 0 { return Err(DomainError::Validation("timeout invalid")); }
        Ok(())
    }
}

#[derive(Debug)]
pub enum JobState {
    Pending,
    Scheduled,
    Running,
    Succeeded,
    Failed { code: String, retryable: bool },
    Cancelled,
    Terminated,
}

pub struct Job {
    id: JobId,
    spec: JobSpec,
    state: JobState,
    attempts: u8,
    correlation_id: CorrelationId,
    tenant_id: TenantId,
}

impl Job {
    pub fn request(spec: JobSpec, correlation_id: CorrelationId, tenant_id: TenantId) -> Result<Self, DomainError> {
        spec.validate()?;
        Ok(Self { id: JobId::new(), spec, state: JobState::Pending, attempts: 0, correlation_id, tenant_id })
    }

    pub fn schedule(&mut self, node_id: NodeId) -> Result<Vec<Event>, DomainError> {
        self.state = JobState::Scheduled;
        let evt = Event::AssignmentCreated { job_id: self.id, node_id, policy: "default".into() };
        Ok(vec![evt])
    }

    pub fn start(&mut self, worker_id: WorkerId) -> Vec<Event> {
        self.state = JobState::Running;
        vec![Event::JobStarted { job_id: self.id, worker_id }]
    }

    pub fn complete(&mut self, result: ExecResultRef) -> Vec<Event> {
        self.state = JobState::Succeeded;
        vec![Event::JobCompleted { job_id: self.id, result }]
    }

    pub fn fail(&mut self, code: String, retryable: bool) -> Vec<Event> {
        self.state = JobState::Failed { code, retryable };
        vec![Event::JobFailed { job_id: self.id, code, retryable }]
    }

    pub fn cancel(&mut self) -> Vec<Event> {
        self.state = JobState::Cancelled;
        vec![Event::JobCancelled { job_id: self.id }]
    }
}

#[derive(Debug)]
pub enum Event {
    JobRequested { job_id: JobId, spec: JobSpec, correlation_id: CorrelationId, tenant_id: TenantId },
    JobStarted { job_id: JobId, worker_id: WorkerId },
    JobCompleted { job_id: JobId, result: ExecResultRef },
    JobFailed { job_id: JobId, code: String, retryable: bool },
    JobCancelled { job_id: JobId },
}
```

Objetos de valor y invariantes se aplican con validaciones explícitas; la idempotencia se conserva por claves de evento y comandos idempotentes (por `JobId` y `StepId`). La Saga de compensación se modela como secuencia de comandos y eventos de compensación, manteniendo auditoría y trazabilidad.

### 4.2 Scheduling Context (Scheduling & Capacity)

El Scheduling Context gestiona visibilidad de capacidad, ofertas de nodos y políticas de asignación. Sus entidades incluyen Offer, Node y Constraint; los objetos de valor ResourceQuota y AffinityPolicy encapsulan capacidades y reglas.

Eventos clave:

- OfferCreated. Publica disponibilidad por nodo o pool con un snapshot de ResourceQuota.
- AssignmentCreated. Registra la decisión de asignar un Job a un Node con la política aplicada (por ejemplo, FIFO, packing por recursos).
- AssignmentRejected. Señala que la oferta no puede satisfacerse, con motivo (recursos insuficientes, política incumplida).
- CapacityUpdated. Refleja cambios de capacidad en tiempo casi real.

Visibilidad de recursos y políticas:

- Las ofertas agregan un ResourceQuota (CPU, memoria, GPU, IO) con contadores atómicos y estructuras lock‑free para minimizar contención.
- Las políticas abarcan FIFO y packing por recursos; affinity por tipo de executor o zona; y límites de saturación con backpressure en ingestión de ofertas.

Especificación Rust (ilustrativa):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub cpu_m: u64,
    pub memory_mb: u64,
    pub gpu: u8,
}

#[derive(Debug, Clone)]
pub struct AffinityPolicy {
    pub by_executor_type: Option<String>,
    pub by_zone: Option<String>,
    pub by_pool: Option<String>,
}

pub struct Offer {
    pub id: OfferId,
    pub node_id: NodeId,
    pub quota: ResourceQuota,
    pub affinity: AffinityPolicy,
    pub ts: Timestamp,
}

pub struct Assignment {
    pub id: AssignmentId,
    pub job_id: JobId,
    pub node_id: NodeId,
    pub policy: String,
    pub status: AssignmentStatus,
}

pub enum AssignmentStatus {
    Pending,
    Created,
    Rejected { reason: String },
    Revoked,
}
```

Idempotencia y correlación. Las ofertas se deduplican por OfferId; las decisiones por AssignmentId y correlacionan con JobId para trazabilidad.

### 4.3 Worker Management Context (Provider Integration)

El Worker Management Context abstrae infraestructura mediante una capa de proveedor. Su propósito es crear, terminar y observar ejecutores, independientemente de si el backend es Kubernetes, Docker o futuros proveedores.

Entidades:

- Worker. Representa un ejecutor efímero con identidad WorkerId, estado y referencia a ProviderConfig y RuntimeSpec.
- ProviderEndpoint. Define endpoints y credenciales del provider.

Objetos de valor:

- ProviderConfig. Configuración del backend (contexto de Kubernetes,namespace, credenciales).
- RuntimeSpec. Especificación del runtime (imagen, límites de recursos, labels, toleraciones, secrets refs).

Eventos:

- WorkerCreated, WorkerTerminated, WorkerFailed. Publicados para que Orchestration y Scheduling ajusten estado y replanifiquen.

Abstracción Provider y extensibilidad:

- Un trait WorkerManagerProvider expone métodos `create`, `terminate`, `logs`, `port_forward` y `stream_events`. Permite incorporar proveedores futuros sin modificar el núcleo del dominio[^5].

Tabla 5. Operaciones por proveedor

| Capacidad | Kubernetes | Docker | Futuros providers |
|---|---|---|---|
| Crear ejecutores | Pods/Jobs/CRDs | Contenedores | provider.create_worker() |
| Terminar ejecutores | Delete de Pod/Job | Stop/rm contenedor | provider.terminate_worker() |
| Logs | API de logs; kubelet | API de logs | provider.logs() |
| Port‑forward | API de port‑forward | CLI/API | provider.port_forward() |
| Watchers/Events | Watchers de recursos | Estadísticas/eventos | provider.stream_events() |
| Secretos y RBAC | Nativos | Secrets básicos | provider.configure_security() |

Especificación Rust (ilustrativa):

```rust
pub trait WorkerManagerProvider: Send + Sync {
    async fn create_worker(&self, spec: &RuntimeSpec, cfg: &ProviderConfig) -> Result<Worker, ProviderError>;
    async fn terminate_worker(&self, id: WorkerId) -> Result<(), ProviderError>;
    async fn logs(&self, id: WorkerId) -> Result<LogStreamRef, ProviderError>;
    async fn port_forward(&self, id: WorkerId, port: u16) -> Result<String, ProviderError>;
    fn stream_events(&self) -> Pin<Box<dyn Stream<Item = ProviderEvent> + Send>>;
}

pub struct Worker {
    pub id: WorkerId,
    pub state: WorkerState,
    pub runtime_spec: RuntimeSpec,
}

pub enum WorkerState {
    Creating,
    Running,
    Terminating,
    Terminated,
    Failed { reason: String },
}
```

La integración con Kubernetes aprovecha watchers para detección eficiente de cambios y CRDs para extensión; Docker se utiliza como fallback o para escenarios ligeros[^5][^8][^9][^12].

### 4.4 Execution Context (Workers efímeros)

El Execution Context orquesta la ejecución de steps en workers efímeros y reporta logs/métricas.

Entidades:

- Execution. Contexto de ejecución asociado a un JobId/StepId; registra parámetros de runtime y estado.
- StepRun. Instancia de ejecución de un step; captura resultados, errores y tiempos.

Objetos de valor:

- ExecResult. Resultado de ejecución (código de salida, stdout/stderr refs, artefactos generados).
- LogStreamRef. Referencia al stream de logs para consultar/transportar.

Eventos:

- StepStarted, StepCompleted, StepFailed. Emitidos con metadatos de correlación para trazabilidad y toma de decisiones en Orchestration.

Endpoints estándar de ejecución:

Tabla 6. Endpoints de ejecución y semántica

| Método | Ruta | Semántica | Idempotencia |
|---|---|---|---|
| POST | /exec | Ejecutar step | Sí, por StepId |
| GET | /logs/{exec_id} | Streaming de logs | N/A |
| POST | /complete | Finalizar step | Sí, por ExecId |

Especificación Rust (ilustrativa):

```rust
pub struct Execution {
    pub exec_id: ExecId,
    pub step_id: StepId,
    pub job_id: JobId,
    pub params: RuntimeSpec,
}

pub struct StepRun {
    pub step_id: StepId,
    pub exec_id: ExecId,
    pub state: StepState,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub result: Option<ExecResult>,
}

pub enum StepState {
    Started,
    Completed,
    Failed { error: String, retryable: bool },
}
```

Los workers efímeros se aíslan por namespaces/pods; el streaming de logs se habilita mediante port‑forward o endpoints seguros; el sistema registra métricas de ejecución para observabilidad.

### 4.5 Telemetry Context (Métricas, Trazas y Eventos Distribuidos)

El Telemetry Context centraliza la emisión de métricas y trazas y habilita streaming hacia dashboards.

Entidades:

- Metric. Métrica con nombre, valor, tipo (counter/gauge/histogram), tags.
- Trace. Traza compuesta por spans correlacionados por TraceContext.

Objetos de valor:

- TagSet. Conjunto de etiquetas para enriquecer y filtrar métricas/trazas.
- TraceContext. Identificadores de correlación (TraceId, SpanId, ParentSpanId).

Eventos:

- MetricEmitted, TraceSpans, AlertRaised. Transportados por NATS JetStream para baja latencia y entrega configurable; se integran con OpenTelemetry para correlación.

Política de retención y semánticas de entrega:

Tabla 7. Política de retención por tipo y garantías de entrega

| Tipo | Retención | Garantía | Uso |
|---|---|---|---|
| Métricas operativas | Corto plazo (minutos/horas) | At‑least‑once | Observabilidad y alertas |
| Eventos de negocio | Medio plazo (días) | At‑least‑once / Exactly‑once crítico | Auditoría y reconciliación |
| Trazas | Corto‑medio plazo | At‑least‑once | Diagnóstico distribuido |

Especificación Rust (ilustrativa):

```rust
#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub metric_type: MetricType,
    pub tags: TagSet,
    pub ts: Timestamp,
}

#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
}

pub enum TelemetryEvent {
    MetricEmitted(Metric),
    TraceSpans(TraceContext, Vec<Span>),
    AlertRaised { id: AlertId, severity: Severity, message: String },
}
```

La instrumentación con tracing y OpenTelemetry garantiza visibilidad en orquestación, scheduling y provider; NATS JetStream se selecciona por su baja latencia y semánticas de entrega adecuadas para control‑plane y telemetría[^4].

### 4.6 Security Context (Keycloak + AWS Verified Permissions)

El Security Context consolida identidad y autorización para el conjunto de la plataforma. La autenticación se basa en OIDC/OAuth2 con Keycloak (incluyendo Service Accounts para cargas machine‑to‑machine), y la autorización se realiza policy‑based mediante AWS Verified Permissions (AVP), con gestión central de políticas, scopes y decisiones a nivel de recurso.

Modelo de autorización:

- Principal. Identidad autenticada (usuario o service account).
- Resource. Entidades del dominio (Job, Pipeline, Worker, Artifact).
- Action. Operaciones (create, read, update, delete, execute, schedule, upload, promote).
- Context. Atributos adicionales (tenant, tags, environment, scheduling policy).

Flujo de decisión:

1. El servicio solicita un token OIDC a Keycloak (para sesiones humanas o service accounts).
2. El servicio invoca AVP con el principal, la acción solicitada, el recurso y el contexto; recibe un Permit/Deny con motivos y obligaciones (por ejemplo, “requiere MFA” o “limitar scope a tenant X”)[^1].
3. El servicio aplica la decisión, audita el evento y propaga el contexto de autorización para trazabilidad.

Catálogo de recursos y acciones:

Tabla 8. Recursos y acciones para AVP

| Recurso | Acciones | Contexto |
|---|---|---|
| Job | create, read, cancel, execute, retry | tenant_id, tags, owner |
| Pipeline | create, read, update, execute | tenant_id, environment, owner |
| Worker | create, read, terminate, logs | provider_id, tenant_id, namespace |
| Node/Offer | read, schedule | zone, pool, capacity_snapshot |
| Artifact | upload, read, promote, delete | repository, environment, checksum |

Tabla 9. Flujo de autorización: endpoints y decisiones

| Servicio | Endpoint | Decisión AVP | Audit |
|---|---|---|---|
| Orchestration | POST /jobs | Permit/Deny (create job) | AuthGranted/AuthorizationDenied |
| Scheduling | POST /schedule | Permit/Deny (assign job) | AuthGranted/AuthorizationDenied |
| Worker Manager | POST /workers | Permit/Deny (create worker) | AuthGranted/AuthorizationDenied |
| Execution | POST /exec | Permit/Deny (execute step) | AuthGranted/AuthorizationDenied |
| Artifact | POST /artifacts | Permit/Deny (upload) | AuthGranted/AuthorizationDenied |

Especificación Rust (ilustrativa):

```rust
pub struct Principal {
    pub sub: String, // subject (Keycloak sub)
    pub scopes: Vec<Scope>,
    pub tenant_id: TenantId,
}

pub struct Resource {
    pub rtype: ResourceType,
    pub rid: String,
}

pub enum Action {
    Create, Read, Update, Delete, Execute, Schedule, Upload, Promote, Cancel, Retry,
}

pub struct AuthorizationContext {
    pub tenant_id: TenantId,
    pub tags: TagSet,
    pub environment: Option<String>,
}

pub enum AuthorizationDecision {
    Permit { decision_id: String, obligations: Vec<String> },
    Deny { reason: String },
}

#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    async fn decide(
        &self,
        principal: &Principal,
        action: &Action,
        resource: &Resource,
        ctx: &AuthorizationContext,
    ) -> Result<AuthorizationDecision, AuthError>;
}
```

### 4.7 Artifact Management Context (Gestión de artefactos y dependencias)

El Artifact Management Context administra artefactos de build y dependencias, con políticas de promoción y verificación (checksums). Persistencia y versionado se alinean con prácticas de registries y repositorios; el vínculo con pipelines se modela a partir de referencias a JobId/PipelineId y resultados de ejecución.

Entidades:

- Artifact. Representa un artefacto subido (binarios, imágenes, paquetes), con identidad ArtifactRef, metadatos y Checksum.
- Dependency. Relación entre artefactos y versiones, con políticas de compatibilidad.

Objetos de valor:

- ArtifactRef. Identificador canónico (incluye repository, nombre, versión/tag).
- Checksum. Valor de verificación (algoritmo y valor).

Eventos:

- ArtifactUploaded, ArtifactPromoted, ArtifactValidated. Publicados para integrar con pipelines y controles de calidad.

Especificación Rust (ilustrativa):

```rust
#[derive(Debug, Clone)]
pub struct ArtifactRef {
    pub repository: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct Checksum {
    pub algo: String, // e.g., "sha256"
    pub value: String,
}

pub struct Artifact {
    pub ref_: ArtifactRef,
    pub checksum: Checksum,
    pub size_bytes: u64,
    pub created_at: Timestamp,
}

pub struct Dependency {
    pub from: ArtifactRef,
    pub to: Vec<ArtifactRef>,
    pub constraint: String, // semver-like
}
```

## 5. Contratos de APIs y eventos (OpenAPI + NATS)

La plataforma utiliza APIs REST documentadas con OpenAPI y eventos publicados en NATS. El catálogo de endpoints consolida rutas por servicio; las políticas de idempotencia se aplican en creación/terminación y asignaciones.

Tabla 10. Catálogo consolidado de endpoints

| Servicio | Método | Ruta | Semántica | Idempotencia |
|---|---|---|---|---|
| Orquestador | POST | /jobs | Crear job | Sí, por JobId |
| Orquestador | GET | /jobs/{id} | Consultar estado | N/A |
| Planificador | POST | /schedule | Solicitar asignación | Sí, por JobId |
| Planificador | GET | /nodes | Capacidades y ofertas | N/A |
| Worker Manager | POST | /workers | Crear worker | Sí, por WorkerId |
| Worker Manager | DELETE | /workers/{id} | Terminar worker | Sí, por WorkerId |
| Worker Manager | GET | /workers/{id}/logs | Obtener logs | N/A |
| Workers | POST | /exec | Ejecutar step | Sí, por StepId |
| Telemetría | GET | /metrics | Consultar métricas | N/A |
| Telemetría | GET | /traces | Consultar trazas | N/A |
| Consola | GET | /ui/jobs | Agregación de datos | N/A |

Eventos por sujeto (NATS). Los subjects se organizan jerárquicamente por contexto y entidad, facilitando filtrado y consumo selectivo. La siguiente tabla ejemplifica sujetos y payloads mínimos.

Tabla 11. Sujetos y eventos NATS

| Sujeto | Evento | Payload mínimo |
|---|---|---|
| ci.job.requested | JobRequested | JobId, Spec, CorrelationId, TenantId |
| ci.job.started | JobStarted | JobId, WorkerId |
| ci.job.completed | JobCompleted | JobId, ResultRef |
| ci.job.failed | JobFailed | JobId, ErrorCode, Retryable |
| ci.job.cancelled | JobCancelled | JobId, Reason |
| ci.offer.created | OfferCreated | OfferId, NodeId, ResourceQuota |
| ci.assignment.created | AssignmentCreated | AssignmentId, JobId, NodeId, Policy |
| ci.assignment.rejected | AssignmentRejected | AssignmentId, Reason |
| ci.node.capacity.updated | CapacityUpdated | NodeId, ResourceQuota, Timestamp |
| ci.worker.created | WorkerCreated | WorkerId, ProviderId, RuntimeSpec |
| ci.worker.terminated | WorkerTerminated | WorkerId, ExitCode |
| ci.worker.failed | WorkerFailed | WorkerId, Reason |
| ci.step.started | StepStarted | StepId, JobId, ExecId |
| ci.step.completed | StepCompleted | StepId, ResultRef, DurationMs |
| ci.step.failed | StepFailed | StepId, ErrorCode, Retryable |
| ci.metric.emitted | MetricEmitted | MetricName, Value, Tags |
| ci.trace.spans | TraceSpans | TraceContext, Spans |
| ci.alert.raised | AlertRaised | AlertId, Severity, Message |
| ci.security.auth.granted | AuthGranted | Principal, Scope, DecisionId |
| ci.security.auth.revoked | AuthRevoked | Principal, Scope, DecisionId |
| ci.security.auth.denied | AuthorizationDenied | Resource, Action, Reason |
| ci.artifact.uploaded | ArtifactUploaded | ArtifactRef, Checksum |
| ci.artifact.promoted | ArtifactPromoted | ArtifactRef, Env |
| ci.artifact.validated | ArtifactValidated | ArtifactRef, Status |

La semántica de entrega se configura por caso de uso: at‑most‑once para señales transitorias, at‑least‑once para eventos de estado y exactly‑once en auditoría crítica donde la deduplicación idempotente y transacciones en el bus lo permitan[^4][^14].

## 6. Integración de Seguridad: Keycloak + AWS Verified Permissions + mTLS + Audit

Autenticación (Keycloak). La plataforma soporta OIDC/OAuth2: usuarios humanos con flujos Authorization Code y service accounts para cargas machine‑to‑machine. Los tokens se validan en elgateway y en los servicios, con scopes por tenant y recursos. La autenticación confiere identidad (Principal) y contexto de sesión.

Autorización (AWS Verified Permissions). Las políticas se expresan por recurso y acción; AVP evalúa en base a atributos de principal, recurso y contexto (tenant, tags, entorno). Las decisiones se auditan y se asocian a obligaciones (por ejemplo, “solo lectura” o “scope restringido a tenant X”)[^1].

mTLS. La autenticación mutua en comunicaciones internas garantiza identidad de servicios y canal seguro. La gestión de certificados se integra con secretos del proveedor (Kubernetes/Docker), rotaciones automáticas y control de revocación.

Audit Trail. Los eventos de seguridad (AuthGranted, AuthRevoked, AuthorizationDenied) se registran de forma distribuida, con correlación a job/pipeline y retención conforme a cumplimiento. La auditoría se integra con telemetría y OpenTelemetry para rastreo de decisiones de autorización en flujos end‑to‑end.

Matriz de políticas (resumen):

Tabla 12. Políticas AVP por rol/servicio y casos de uso

| Rol/Servicio | Caso de uso | Política |
|---|---|---|
| Orchestration | Crear job | Permit si principal.scope incluye “job:create” y tenant_id coincide |
| Scheduling | Asignar job | Permit si principal.scope incluye “schedule:assign” y recursos disponibles |
| Worker Manager | Crear worker | Permit si principal.scope incluye “worker:create” y provider autorizado |
| Execution | Ejecutar step | Permit si principal.scope incluye “exec:run” y entorno permitido |
| Artifact | Subir artefacto | Permit si principal.scope incluye “artifact:upload” y checksum válido |

Flujos de autorización por endpoint (resumen):

Tabla 13. Flujos y resultados

| Endpoint | Decisión | Obligaciones | Audit |
|---|---|---|---|
| POST /jobs | Permit/Deny | Scope requerido | AuthGranted/AuthorizationDenied |
| POST /schedule | Permit/Deny | Política de asignación | AuthGranted/AuthorizationDenied |
| POST /workers | Permit/Deny | RBAC provider | AuthGranted/AuthorizationDenied |
| POST /exec | Permit/Deny | Entorno permitido | AuthGranted/AuthorizationDenied |
| POST /artifacts | Permit/Deny | Checksum obligatorio | AuthGranted/AuthorizationDenied |

Especificación Rust (ilustrativa):

```rust
pub enum AuthEvent {
    AuthGranted { principal: Principal, scope: Scope, decision_id: String },
    AuthRevoked { principal: Principal, scope: Scope, decision_id: String },
    AuthorizationDenied { resource: Resource, action: Action, reason: String },
}
```

## 7. Patrones de comunicación y semánticas de entrega

La selección entre NATS y Kafka se realiza por caso de uso. NATS JetStream se privilegia en control‑plane y telemetría por baja latencia, sujetos jerárquicos y garantías configurables; Kafka se reserva para flujos de alto volumen histórico y replay masivo[^4].

Comparativa:

Tabla 14. NATS vs Kafka

| Criterio | NATS (JetStream) | Kafka |
|---|---|---|
| Latencia | Optimizada para tiempo real | Mayor latencia por batch |
| Throughput | Elevado, eficiente | Elevado, con MB/s altos |
| Garantías | At‑most/at‑least/exactly‑once | At‑least/exactly‑once |
| Complejidad | Baja‑media | Media‑alta |
| Casos de uso | Control‑plane, telemetría | Analítica, replay histórico |

Semánticas por tipo de evento:

Tabla 15. Semánticas y configuración

| Tipo | Semántica | Configuración | Impacto |
|---|---|---|---|
| Ofertas/Asignaciones | At‑least‑once | Acks y reintentos | Idempotencia del consumidor |
| Eventos de Job/Step | At‑least‑once | Repetición por offsets (Kafka) o JetStream | Deduplicación por IDs |
| Auditoría crítica | Exactly‑once | Transacciones en bus | Reconciliación segura |
| Métricas livianas | At‑most‑once | Sin persistencia | Latencia mínima |

El diseño considera idempotencia por claves de evento y correlación por identifiers de job/pipeline para evitar duplicidades y asegurar consistencia operacional.

## 8. Trazabilidad, telemetría y auditoría distribuida

La correlación de trazas y eventos usa un TraceContext propagateado en headers y payload de eventos; OpenTelemetry se integra para construir spans consistentes en Orchestration, Scheduling, Worker Manager y Execution. Las políticas de retención definen qué se conserva, dónde y por cuánto tiempo; los logs estructurados incluyen campos obligatorios (tenant, resource, action, decision, error).

Tabla 16. Puntos de instrumentación y trazas

| Servicio | Span | Evento | Atributos clave |
|---|---|---|---|
| Orchestration | create_job | JobRequested | tenant, spec hash, correlation_id |
| Orchestration | schedule_job | AssignmentCreated | job_id, node_id, policy |
| Worker Manager | create_worker | WorkerCreated | provider_id, runtime_spec |
| Execution | run_step | StepStarted/Completed | exec_id, step_id, duration |
| Telemetry | emit_metric | MetricEmitted | metric_name, value, tags |
| Security | authorize | AuthGranted/Denied | principal, resource, action |

La instrumentación consistente habilita diagnósticos end‑to‑end y análisis de performance en la plataforma, reforzando el patrón de observabilidad de microservicios[^13].

## 9. Resiliencia, escalabilidad y gestión de estado

La resiliencia se implementa con patrones específicos y puntos de aplicación claros:

- Circuit Breaker en llamadas a proveedores y servicios externos para evitar cascadas de fallos.
- Bulkhead para aislar pools de conexiones y colas por dominio.
- Retry/Timeout en publicación/consumo de eventos y llamadas HTTP, con límites y backoff.
- Saga en flujos multi‑etapa para consistencia distribuida e integridad de compensaciones.
- Idempotencia en comandos de creación/terminación y asignaciones.
- Backpressure en ingestión de eventos y scheduling para evitar saturación.

Tabla 17. Patrones por servicio

| Patrón | Aplicación | Servicios |
|---|---|---|
| Circuit Breaker | Providers (K8s/Docker), APIs externas | Worker Manager, Consola |
| Bulkhead | Pools y colas por dominio | Orquestador, Planificador, Telemetría |
| Retry/Timeout | Eventos y HTTP | Todos |
| Saga | Orquestación de pipelines | Orchestration |
| Idempotencia | Comandos críticos | Orchestration, Worker Manager |
| Backpressure | Ingestión y colas | Scheduling, Telemetry |

El escalado horizontal por servicio, junto con el bus de eventos y particiones/grupos de cola, permite paralelización y resiliencia operacional[^2].

## 10. Optimización de rendimiento en Rust

Las optimizaciones se aplican según el perfil de cada contexto:

- Tipos Copy para objetos pequeños (IDs, timestamps, ResourceQuota básica).
- `Arc<RwLock<>>` para estado compartido cuando sea indispensable, protegiendo acceso concurrente.
- Atómicos para contadores (métricas, capacity snapshots).
- Allocators personalizados en contextos críticos para mejorar latencia y memoria.
- Zero‑copy y formatos binarios en telemetría/locks‑free donde aplique; buffers reutilizables en I/O.
- Modelo de actores y `async/await` con Tokio para eficiencia en concurrencia y I/O[^10][^11].

Tabla 18. Estructuras de datos y optimizaciones

| Caso de uso | Tipo | Beneficio |
|---|---|---|
| IDs (JobId, StepId) | Copy types | Minimiza copias y heap |
| Métricas de colas | Atomics | Lock‑free, baja contención |
| Estado compartido de offers | Arc<RwLock<>> | Seguridad y claridad |
| Streaming de logs | Binario + buffers | Reducción de latencia |
| Actores | Tokio/Actix | Aislamiento y escalabilidad |

## 11. Roadmap de implementación y criterios de aceptación

El roadmap organiza la entrega en iteraciones que agregan valor y reducen riesgo, con SLOs alineados a la operación.

Tabla 19. Hitos y métricas objetivo

| Hito | Entregables | SLOs objetivo |
|---|---|---|
| MVP Orchestration | REST /jobs, /pipelines; eventos base | p95 creación job < 300 ms |
| K8s Integrado | Worker Manager con pods y logs | Éxito creación/terminación > 99.5% |
| Scheduling | Capacidades/offers; FIFO/packing | p95 asignación < 500 ms |
| Telemetry | Métricas, trazas; streaming | Cobertura eventos clave 100% |
| Console/BFF | Dashboards SSE/WS; RBAC | p95 consulta UI < 400 ms; uptime 99.9% |

Este plan facilita adopción incremental de seguridad, resiliencia y observabilidad en cada iteración, con instrumentación y pruebas contractuales de eventos[^12][^4].

## 12. Riesgos, supuestos y mitigaciones

La complejidad operativa de brokers de mensajes, la consistencia eventual y la mezcla de runtimes asíncronos constituyen riesgos relevantes. La estrategia de mitigación se basa en automatización, observabilidad del bus, runbooks, y selección estricta de librerías.

Tabla 20. Matriz de riesgos y mitigaciones

| Riesgo | Impacto | Probabilidad | Mitigación |
|---|---|---|---|
| Operación de NATS/Kafka | Medio | Media | Automatización, observabilidad del bus, runbooks |
| Consistencia eventual | Medio | Alta | Idempotencia, Sagas, auditoría de eventos |
| Mezcla de runtimes async | Alto | Media | Selección única (Tokio), pruebas de integración[^11] |
| Saturación de scheduling | Alto | Media | Backpressure, políticas de packing, autoscaling |
| Seguridad multi‑tenant | Alto | Media | RBAC, segmentación, secretos gestionados |
| Falla de provider | Alto | Baja‑Media | Circuit Breaker, fallback y colas de reintento |

Las decisiones de selección y operación de NATS/Kafka impactan directamente latencia, throughput y resiliencia; deben alinearse con los SLOs de cada caso de uso[^4][^11].

## 13. Lagunas de información (information gaps)

Se reconocen las siguientes áreas pendientes de especificación detallada en fases posteriores:

- Benchmarks de latencia y throughput comparativos de NATS vs Kafka en escenarios CI/CD específicos (construcción, paralelización, fan‑out) y su impacto en SLOs.
- Detalles operativos de autenticación/autorización multi‑tenant (gestión de secretos, mapeo de roles por proveedor) y su integración concreta con Keycloak y AWS Verified Permissions.
- Especificación de CRDs o recursos extendidos para jobs CI/CD en Kubernetes y su ciclo de vida (contratos detallados).
- Estrategias de serialización binaria y diseños zero‑copy para transferencia de artefactos y logs, con límites de tamaño y codecs.
- Definición de políticas de scheduling avanzadas (preemptible jobs, pool de GPU, afinidad por región/zone) con métricas y SLOs precisos.

Estas lagunas no impiden la puesta en marcha del diseño táctico, pero condicionan decisiones operativas finas y métricas detalladas que se resolverán con experimentación y medición en entornos controlados.

---

## Referencias

[^1]: Microsoft Learn — Azure Architecture Center. “Domain analysis for microservices.” https://learn.microsoft.com/en-us/azure/architecture/microservices/model/domain-analysis  
[^2]: microservices.io. “Pattern: Microservice Architecture.” https://microservices.io/patterns/microservices.html  
[^3]: Bits and Pieces. “Understanding the Bounded Context in Microservices.” https://blog.bitsrc.io/understanding-the-bounded-context-in-microservices-c70c0e189dd1  
[^4]: Synadia. “NATS and Kafka Compared.” https://www.synadia.com/blog/nats-and-kafka-compared  
[^5]: Shuttle.dev. “Using Kubernetes with Rust.” https://www.shuttle.dev/blog/2024/10/22/using-kubernetes-with-rust  
[^6]: Kubernetes Documentation. “Kubectl.” https://kubernetes.io/docs/tasks/tools/#kubectl  
[^7]: Martin Fowler. “Bounded Context.” https://www.martinfowler.com/bliki/BoundedContext.html  
[^8]: Kubernetes Documentation. “Efficient detection of changes (watchers).” https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes  
[^9]: Docker Docs. “Test your Rust deployment.” https://docs.docker.com/guides/rust/deploy/  
[^10]: Ryuichi. “Actors with Tokio.” https://ryhl.io/blog/actors-with-tokio/  
[^11]: Luca Palmieri. “Choosing a Rust web framework, 2020 edition.” https://www.lpalmieri.com/posts/2020-07-04-choosing-a-rust-web-framework-2020-edition/  
[^12]: Microsoft Learn — Azure Architecture Center. “Microservices architecture style.” https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices  
[^13]: OpenAPI Specification. “Latest.” https://spec.openapis.org/oas/latest.html