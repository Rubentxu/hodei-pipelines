# Catálogo de Servicios y Sistemas - Hodei Jobs

**Fecha de análisis:** 2025-12-27  
**Basado en:** Análisis directo del código fuente

---

## Índice de Servicios

1. [Domain Layer Services](#1-domain-layer-services)
2. [Application Layer Services](#2-application-layer-services)
3. [Infrastructure Layer Services](#3-infrastructure-layer-services)
4. [API Layer Components](#4-api-layer-components)
5. [Diagrama de Componentes](#5-diagrama-de-componentes)

---

## 1. Domain Layer Services

### 1.1 JobScheduler

**Ubicación:** `crates/domain/src/job_execution/services/mod.rs`

**Tipo:** Domain Service

**Responsabilidad:** Seleccionar el proveedor óptimo para ejecutar un job basándose en disponibilidad y capacidades.

**Interfaz:**
```rust
pub struct JobScheduler;

impl JobScheduler {
    pub fn new() -> Self;
    pub fn select_provider(
        &self,
        job: &Job,
        available_providers: &[ProviderInfo],
    ) -> DomainResult<ProviderId>;
}
```

**Dependencias:**
- `Job` entity
- `ProviderInfo` struct
- `ProviderId` value object
- `DomainError` / `DomainResult`

**Lógica de negocio:**
- Retorna error si no hay proveedores disponibles
- Actualmente selecciona el primer proveedor disponible (estrategia simplificada para MVP)

**Acoplamiento:** Bajo  
**Cohesión:** Alta (una única responsabilidad)  
**Connascence:** Nombre y Tipo

---

### 1.2 ProviderRegistry

**Ubicación:** `crates/domain/src/provider_management/services/mod.rs`

**Tipo:** Domain Service

**Responsabilidad:** Filtrar y consultar proveedores basándose en su estado.

**Interfaz:**
```rust
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn new() -> Self;
    pub fn get_all_providers(&self, providers: &[Provider]) -> Vec<Provider>;
    pub fn get_healthy_providers(&self, providers: &[Provider]) -> Vec<Provider>;
}
```

**Dependencias:**
- `Provider` entity
- `ProviderStatus` enum

**Lógica de negocio:**
- Filtra proveedores por estado `Active` para obtener "healthy"
- No tiene estado propio (stateless)

**Acoplamiento:** Bajo  
**Cohesión:** Alta  
**Connascence:** Nombre y Tipo

---

### 1.3 ExecutionCoordinator

**Ubicación:** `crates/domain/src/execution_coordination/services/mod.rs`

**Tipo:** Domain Service

**Responsabilidad:** Coordinar la ejecución de jobs entre el contexto de jobs y el de providers.

**Interfaz:**
```rust
pub struct ExecutionCoordinator;

impl ExecutionCoordinator {
    pub fn new() -> Self;
    pub async fn coordinate_execution(
        &self,
        job: &Job,
        provider: &Provider,
    ) -> DomainResult<String>;
}
```

**Dependencias:**
- `Job` entity (de `job_execution`)
- `Provider` entity (de `provider_management`)

**Lógica de negocio:**
- Actualmente retorna un mensaje de confirmación (placeholder para MVP)
- Punto de extensión para lógica de coordinación compleja

**Acoplamiento:** Medio (cruza bounded contexts)  
**Cohesión:** Alta  
**Connascence:** Nombre y Tipo

---

## 2. Application Layer Services

### 2.1 JobService

**Ubicación:** `crates/application/src/job_service/mod.rs`

**Tipo:** Application Service (Use Case Orchestrator)

**Responsabilidad:** Orquestar operaciones del ciclo de vida de jobs, coordinando entre repositorios y entidades de dominio.

**Interfaz:**
```rust
pub struct JobService {
    job_repo: Box<dyn JobRepository>,
    provider_repo: Option<Box<dyn ProviderRepository>>,
}

impl JobService {
    pub fn new(job_repo: Box<dyn JobRepository>) -> Self;
    pub fn with_providers(
        job_repo: Box<dyn JobRepository>,
        provider_repo: Box<dyn ProviderRepository>,
    ) -> Self;
    
    pub async fn create_job(&self, spec: JobSpec) -> DomainResult<Job>;
    pub async fn execute_job(&self, job_id: &JobId) -> DomainResult<Job>;
    pub async fn get_job(&self, job_id: &JobId) -> DomainResult<Job>;
    pub async fn get_job_result(&self, job_id: &JobId) -> DomainResult<Option<JobResult>>;
    pub async fn cancel_job(&self, job_id: &JobId) -> DomainResult<()>;
    pub async fn list_jobs(&self) -> DomainResult<Vec<Job>>;
}
```

**Dependencias:**
| Dependencia | Tipo | Origen |
|-------------|------|--------|
| `JobRepository` | Trait (Port) | domain::job_execution |
| `ProviderRepository` | Trait (Port) | domain::provider_management |
| `Job` | Entity | domain::job_execution |
| `JobSpec` | Value Object | domain::job_execution |
| `JobId`, `JobResult`, `JobState` | Types | domain::shared_kernel |

**Operaciones:**

| Método | Descripción | Flujo |
|--------|-------------|-------|
| `create_job` | Crea nuevo job | Valida → Crea Job → Persiste |
| `execute_job` | Inicia ejecución | Busca → Actualiza estado → Persiste |
| `get_job` | Obtiene job por ID | Busca → Retorna o NotFound |
| `cancel_job` | Cancela job | Busca → Actualiza estado → Persiste |
| `list_jobs` | Lista todos | Delega a repositorio |

**Acoplamiento:** Medio (depende de abstracciones)  
**Cohesión:** Media (múltiples operaciones relacionadas)  
**Connascence:** Nombre, Tipo, Temporal

---

### 2.2 ProviderService

**Ubicación:** `crates/application/src/provider_service/mod.rs`

**Tipo:** Application Service

**Responsabilidad:** Orquestar operaciones de gestión de proveedores.

**Interfaz:**
```rust
pub struct ProviderService {
    provider_repo: Box<dyn ProviderRepository>,
}

impl ProviderService {
    pub fn new(provider_repo: Box<dyn ProviderRepository>) -> Self;
    
    pub async fn register_provider(
        &self,
        id: ProviderId,
        name: String,
        provider_type: ProviderType,
        capabilities: ProviderCapabilities,
        config: ProviderConfig,
    ) -> DomainResult<Provider>;
    
    pub async fn list_providers(&self) -> DomainResult<Vec<Provider>>;
    pub async fn get_provider(&self, id: &ProviderId) -> DomainResult<Provider>;
    pub async fn activate_provider(&self, id: &ProviderId) -> DomainResult<()>;
    pub async fn deactivate_provider(&self, id: &ProviderId) -> DomainResult<()>;
    pub async fn delete_provider(&self, id: &ProviderId) -> DomainResult<()>;
    pub async fn get_provider_status(&self, id: &ProviderId) -> DomainResult<ProviderStatus>;
}
```

**Dependencias:**
| Dependencia | Tipo | Origen |
|-------------|------|--------|
| `ProviderRepository` | Trait (Port) | domain::provider_management |
| `Provider` | Entity | domain::provider_management |
| `ProviderConfig` | Value Object | domain::provider_management |
| `ProviderId`, `ProviderType`, `ProviderCapabilities` | Types | domain::shared_kernel |

**Operaciones:**

| Método | Descripción |
|--------|-------------|
| `register_provider` | Crea y persiste nuevo proveedor |
| `activate_provider` | Activa un proveedor inactivo |
| `deactivate_provider` | Desactiva un proveedor |
| `delete_provider` | Elimina proveedor del sistema |

**Acoplamiento:** Bajo  
**Cohesión:** Alta  
**Connascence:** Nombre, Tipo

---

### 2.3 EventOrchestrator

**Ubicación:** `crates/application/src/event_orchestrator.rs`

**Tipo:** Application Service (Event Publisher Facade)

**Responsabilidad:** Publicar eventos de dominio de forma centralizada desde la capa de aplicación.

**Interfaz:**
```rust
pub struct EventOrchestrator {
    event_publisher: Box<dyn EventPublisher>,
}

impl EventOrchestrator {
    pub fn new(event_publisher: Box<dyn EventPublisher>) -> Self;
    
    // Job events
    pub async fn publish_job_created(&self, job_id: JobId, job_name: String, provider_id: Option<ProviderId>) -> EventResult<()>;
    pub async fn publish_job_scheduled(&self, job_id: JobId, provider_id: ProviderId) -> EventResult<()>;
    pub async fn publish_job_started(&self, job_id: JobId, provider_id: ProviderId, execution_id: String) -> EventResult<()>;
    pub async fn publish_job_completed(&self, job_id: JobId, provider_id: ProviderId, execution_id: String, success: bool, output: Option<String>, error: Option<String>, execution_time_ms: u64) -> EventResult<()>;
    pub async fn publish_job_failed(&self, job_id: JobId, provider_id: ProviderId, execution_id: String, error_message: String) -> EventResult<()>;
    pub async fn publish_job_cancelled(&self, job_id: JobId, provider_id: ProviderId) -> EventResult<()>;
    
    // Worker events
    pub async fn publish_worker_connected(&self, provider_id: ProviderId, capabilities: ProviderCapabilities) -> EventResult<()>;
    pub async fn publish_worker_disconnected(&self, provider_id: ProviderId) -> EventResult<()>;
    pub async fn publish_worker_heartbeat(&self, provider_id: ProviderId, active_jobs: u32, resource_usage: ResourceRequirements) -> EventResult<()>;
    
    // Batch
    pub async fn publish_batch(&self, events: Vec<DomainEvent>) -> EventResult<()>;
}
```

**Dependencias:**
| Dependencia | Tipo | Origen |
|-------------|------|--------|
| `EventPublisher` | Trait (Port) | domain::shared_kernel::events |
| `DomainEvent` | Enum | domain::shared_kernel::events |
| `JobId`, `ProviderId`, etc. | Types | domain::shared_kernel::types |

**Eventos publicados:** 9 tipos de eventos de dominio

**Acoplamiento:** Bajo (solo depende de abstracción)  
**Cohesión:** Alta  
**Connascence:** Nombre, Tipo

---

## 3. Infrastructure Layer Services

### 3.1 InMemoryJobRepository

**Ubicación:** `crates/infrastructure/src/repositories/mod.rs`

**Tipo:** Repository Implementation (Adapter)

**Responsabilidad:** Implementación in-memory del repositorio de jobs para desarrollo y testing.

**Implementa:** `domain::job_execution::JobRepository`

**Estructura interna:**
```rust
pub struct InMemoryJobRepository {
    jobs: Mutex<HashMap<String, Job>>,
}
```

**Thread-safety:** Sí (usa `Mutex`)  
**Persistencia:** No (solo en memoria)  
**Uso:** Tests, desarrollo local

---

### 3.2 InMemoryProviderRepository

**Ubicación:** `crates/infrastructure/src/repositories/mod.rs`

**Tipo:** Repository Implementation

**Responsabilidad:** Implementación in-memory del repositorio de providers.

**Implementa:** `domain::provider_management::ProviderRepository`

---

### 3.3 PostgresJobRepository

**Ubicación:** `crates/infrastructure/src/database/postgres.rs`

**Tipo:** Repository Implementation (Adapter)

**Responsabilidad:** Persistencia de jobs en PostgreSQL.

**Implementa:** `domain::job_execution::JobRepository`

**Estructura interna:**
```rust
pub struct PostgresJobRepository {
    pool: Pool<Postgres>,
}
```

**Características:**
- Usa SQLx con compile-time query verification
- Soporta operaciones CRUD + find_by_state
- Serializa JobSpec como JSON

**Métodos adicionales:**
- `find_by_state(state: JobState)` - Filtra jobs por estado

---

### 3.4 PostgresProviderRepository

**Ubicación:** `crates/infrastructure/src/database/postgres.rs`

**Tipo:** Repository Implementation

**Implementa:** `domain::provider_management::ProviderRepository`

---

### 3.5 DockerProviderAdapter

**Ubicación:** `crates/infrastructure/src/adapters/mod.rs`

**Tipo:** Port Adapter (ProviderWorker implementation)

**Responsabilidad:** Ejecutar jobs en contenedores Docker.

**Implementa:** `domain::shared_kernel::ProviderWorker`

**Interfaz:**
```rust
pub struct DockerProviderAdapter {
    provider_id: ProviderId,
    docker_socket: Option<String>,
}

// Implementa todos los métodos de ProviderWorker
async fn submit_job(&self, job_id: &JobId, spec: &JobSpec) -> DomainResult<String>;
async fn get_execution_status(&self, execution_id: &str) -> DomainResult<ExecutionStatus>;
async fn get_job_result(&self, execution_id: &str) -> DomainResult<Option<JobResult>>;
async fn cancel_job(&self, execution_id: &str) -> DomainResult<()>;
async fn get_capabilities(&self) -> DomainResult<ProviderCapabilities>;
```

**Estado actual:** Mock implementation (TODO markers para implementación real)

---

### 3.6 NatsEventPublisher

**Ubicación:** `crates/infrastructure/src/event_bus/mod.rs`

**Tipo:** Port Adapter

**Responsabilidad:** Publicar eventos de dominio a NATS.

**Implementa:** `domain::shared_kernel::EventPublisher`

**Características:**
- Mapea eventos a subjects de NATS por tipo
- Serializa eventos a JSON
- Actualmente usa MockNatsConnection para testing

**Mapeo de subjects:**
| Evento | Subject |
|--------|---------|
| `JobCreated` | `hodei.job.created` |
| `JobScheduled` | `hodei.job.scheduled` |
| `JobStarted` | `hodei.job.started` |
| `JobCompleted` | `hodei.job.completed` |
| `JobFailed` | `hodei.job.failed` |
| `JobCancelled` | `hodei.job.cancelled` |
| `WorkerConnected` | `hodei.worker.connected` |
| `WorkerDisconnected` | `hodei.worker.disconnected` |
| `WorkerHeartbeat` | `hodei.worker.heartbeat` |

---

### 3.7 InMemoryEventStore

**Ubicación:** `crates/infrastructure/src/event_bus/mod.rs`

**Tipo:** Port Adapter

**Responsabilidad:** Almacenar eventos en memoria para desarrollo/testing.

**Implementa:** `domain::shared_kernel::EventStore`

---

### 3.8 DatabasePool

**Ubicación:** `crates/infrastructure/src/database/postgres.rs`

**Tipo:** Infrastructure Service

**Responsabilidad:** Gestionar pool de conexiones PostgreSQL.

**Interfaz:**
```rust
pub struct DatabasePool {
    pool: Pool<Postgres>,
}

impl DatabasePool {
    pub async fn new(config: DatabaseConfig) -> DomainResult<Self>;
    pub fn get_pool(&self) -> &Pool<Postgres>;
    pub async fn health_check(&self) -> DomainResult<()>;
}
```

---

### 3.9 PgBouncerPool

**Ubicación:** `crates/infrastructure/src/pooling/mod.rs`

**Tipo:** Infrastructure Service

**Responsabilidad:** Pool de conexiones optimizado via PgBouncer.

**Configuración:**
```rust
pub struct PgBouncerConfig {
    pub host: String,
    pub port: u16,                    // Default: 6432
    pub database: String,
    pub username: String,
    pub password: String,
    pub pool_min: u32,
    pub pool_max: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub validate_on_checkout: bool,
}
```

---

### 3.10 HealthChecker

**Ubicación:** `crates/infrastructure/src/health/mod.rs`

**Tipo:** Infrastructure Service

**Responsabilidad:** Ejecutar health checks en componentes de infraestructura.

**Health Checks disponibles:**
- `DatabaseHealthCheck` - Verifica conexión a PostgreSQL
- `NatsHealthCheck` - Verifica conexión a NATS
- `PoolHealthCheck` - Verifica estado del pool de conexiones

**Output:**
```rust
pub struct HealthReport {
    pub overall_status: HealthStatus,  // Healthy, Unhealthy, Degraded
    pub checks: Vec<HealthCheckResult>,
    pub uptime_seconds: u64,
    pub timestamp: DateTime<Utc>,
}
```

---

## 4. API Layer Components

### 4.1 HTTP Handlers

**Ubicación:** `crates/api/src/handlers/mod.rs`

**Tipo:** Presentation Layer

**Responsabilidad:** Manejar requests HTTP, transformar DTOs, invocar servicios de aplicación.

**Handlers de Jobs:**

| Handler | Endpoint | Método |
|---------|----------|--------|
| `create_job_handler` | `/api/v1/jobs` | POST |
| `list_jobs_handler` | `/api/v1/jobs` | GET |
| `get_job_handler` | `/api/v1/jobs/:job_id` | GET |
| `execute_job_handler` | `/api/v1/jobs/:job_id/execute` | POST |
| `cancel_job_handler` | `/api/v1/jobs/:job_id/cancel` | DELETE |

**Handlers de Providers:**

| Handler | Endpoint | Método |
|---------|----------|--------|
| `register_provider_handler` | `/api/v1/providers` | POST |
| `list_providers_handler` | `/api/v1/providers` | GET |
| `get_provider_handler` | `/api/v1/providers/:provider_id` | GET |
| `activate_provider_handler` | `/api/v1/providers/:provider_id/activate` | POST |
| `deactivate_provider_handler` | `/api/v1/providers/:provider_id/deactivate` | POST |
| `delete_provider_handler` | `/api/v1/providers/:provider_id` | DELETE |

**Handler de Health:**

| Handler | Endpoint | Método |
|---------|----------|--------|
| `health_check_handler` | `/health` | GET |

### 4.2 AppState

**Ubicación:** `crates/api/src/handlers/mod.rs`

**Tipo:** Dependency Container

**Responsabilidad:** Mantener referencias a servicios de aplicación para inyección en handlers.

```rust
#[derive(Clone)]
pub struct AppState {
    pub job_service: Arc<Mutex<JobService>>,
    pub provider_service: Arc<Mutex<ProviderService>>,
    pub event_orchestrator: Arc<Mutex<EventOrchestrator>>,
}
```

### 4.3 Middleware

**Ubicación:** `crates/api/src/middleware/mod.rs`

**Componentes:**
- `cors_layer()` - Configuración CORS
- `trace_requests` - Logging de requests
- `add_request_id` - Header X-Request-ID
- `rate_limit_headers` - Headers de rate limiting

---

## 5. Diagrama de Componentes

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                   API Layer                                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌────────────┐ │
│  │   Job Handlers  │  │Provider Handlers│  │ Health Handler  │  │ Middleware │ │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  └────────────┘ │
└───────────┼─────────────────────┼───────────────────┼────────────────────────────┘
            │                     │                   │
            ▼                     ▼                   │
┌───────────────────────────────────────────────────────────────────────────────────┐
│                              Application Layer                                     │
│  ┌─────────────────┐  ┌───────────────────┐  ┌─────────────────────┐             │
│  │   JobService    │  │  ProviderService  │  │  EventOrchestrator  │             │
│  │                 │  │                   │  │                     │             │
│  │ - create_job    │  │ - register        │  │ - publish_job_*     │             │
│  │ - execute_job   │  │ - list/get        │  │ - publish_worker_*  │             │
│  │ - cancel_job    │  │ - activate/deact  │  │                     │             │
│  │ - list_jobs     │  │ - delete          │  │                     │             │
│  └───────┬─────────┘  └────────┬──────────┘  └──────────┬──────────┘             │
└──────────┼─────────────────────┼────────────────────────┼────────────────────────┘
           │                     │                        │
           │ uses                │ uses                   │ uses
           ▼                     ▼                        ▼
┌───────────────────────────────────────────────────────────────────────────────────┐
│                                Domain Layer                                        │
│  ┌───────────────────────────────────────────────────────────────────────────┐   │
│  │                            shared_kernel                                   │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐ ┌─────────────────────────┐  │   │
│  │  │  types   │ │  error   │ │    events    │ │        traits           │  │   │
│  │  │ - JobId  │ │ - Domain │ │ - DomainEvent│ │ - ProviderWorker        │  │   │
│  │  │ - etc.   │ │   Error  │ │ - Publisher  │ │ - ProviderWorkerBuilder │  │   │
│  │  └──────────┘ └──────────┘ └──────────────┘ └─────────────────────────┘  │   │
│  └───────────────────────────────────────────────────────────────────────────┘   │
│  ┌────────────────────┐ ┌────────────────────┐ ┌────────────────────────────┐   │
│  │   job_execution    │ │ provider_management│ │   execution_coordination   │   │
│  │                    │ │                    │ │                            │   │
│  │  Entities:         │ │  Entities:         │ │  Services:                 │   │
│  │  - Job (Aggregate) │ │  - Provider        │ │  - ExecutionCoordinator    │   │
│  │                    │ │                    │ │                            │   │
│  │  Repositories:     │ │  Repositories:     │ │                            │   │
│  │  - JobRepository   │ │  - ProviderRepo    │ │                            │   │
│  │                    │ │                    │ │                            │   │
│  │  Services:         │ │  Services:         │ │                            │   │
│  │  - JobScheduler    │ │  - ProviderRegistry│ │                            │   │
│  │                    │ │                    │ │                            │   │
│  │  Value Objects:    │ │  Value Objects:    │ │                            │   │
│  │  - JobSpec         │ │  - ProviderConfig  │ │                            │   │
│  │  - ExecutionCtx    │ │                    │ │                            │   │
│  └────────────────────┘ └────────────────────┘ └────────────────────────────┘   │
└─────────────────────────────────────────────────▲────────────────────────────────┘
                                                  │
                                                  │ implements
                                                  │
┌───────────────────────────────────────────────────────────────────────────────────┐
│                             Infrastructure Layer                                   │
│                                                                                    │
│  ┌─────────────────────────┐  ┌─────────────────────────┐  ┌──────────────────┐  │
│  │      Repositories       │  │       Event Bus         │  │     Adapters     │  │
│  │                         │  │                         │  │                  │  │
│  │  - InMemoryJobRepo     │  │  - NatsEventPublisher   │  │  - DockerAdapter │  │
│  │  - InMemoryProviderRepo│  │  - InMemoryEventStore   │  │                  │  │
│  │  - PostgresJobRepo     │  │                         │  │                  │  │
│  │  - PostgresProviderRepo│  │                         │  │                  │  │
│  └─────────────────────────┘  └─────────────────────────┘  └──────────────────┘  │
│                                                                                    │
│  ┌─────────────────────────┐  ┌─────────────────────────┐  ┌──────────────────┐  │
│  │       Database          │  │        Pooling          │  │      Health      │  │
│  │                         │  │                         │  │                  │  │
│  │  - DatabaseConfig      │  │  - PgBouncerConfig      │  │  - HealthChecker │  │
│  │  - DatabasePool        │  │  - PgBouncerPool        │  │  - HealthReport  │  │
│  └─────────────────────────┘  └─────────────────────────┘  └──────────────────┘  │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## Resumen de Componentes

| Capa | Componentes | Cantidad |
|------|-------------|----------|
| Domain | Entities | 2 |
| Domain | Value Objects | 7+ |
| Domain | Domain Services | 3 |
| Domain | Repository Ports | 2 |
| Domain | Event Types | 9 |
| Application | Application Services | 3 |
| Infrastructure | Repository Implementations | 4 |
| Infrastructure | Adapters | 1 |
| Infrastructure | Event Publishers | 2 |
| Infrastructure | Pools/Health | 3 |
| API | Handlers | 12 |
| API | Middleware | 4 |

**Total:** ~50+ componentes identificados

---

*Catálogo generado a partir del análisis del código fuente en `/home/rubentxu/Proyectos/rust/hodei-jobs`*

