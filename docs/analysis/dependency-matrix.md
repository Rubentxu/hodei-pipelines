# Matriz de Dependencias y Relaciones - Hodei Jobs

**Fecha de análisis:** 2025-12-27  
**Basado en:** Análisis directo del código fuente

---

## 1. Dependencias entre Crates

### 1.1 Tabla de Dependencias (Cargo.toml)

| Crate | Depende de |
|-------|------------|
| `domain` | - (independiente) |
| `application` | `domain` |
| `infrastructure` | `domain` |
| `api` | `domain`, `application`, `infrastructure` |
| `integration-tests` | todos |

### 1.2 Matriz de Dependencias Directas

```
                    domain  application  infrastructure  api
domain                 -         ✗             ✗        ✗
application            ✓         -             ✗        ✗
infrastructure         ✓         ✗             -        ✗
api                    ✓         ✓             ✓        -
```

**Leyenda:**
- `✓` = depende de
- `✗` = no depende de
- `-` = self

### 1.3 Verificación de Reglas de Arquitectura

| Regla | Estado | Verificación |
|-------|--------|--------------|
| Domain no depende de nada | ✅ CUMPLE | `crates/domain/Cargo.toml` solo tiene deps técnicas |
| Application solo depende de Domain | ✅ CUMPLE | `domain = { path = "../domain" }` |
| Infrastructure solo depende de Domain | ✅ CUMPLE | `domain = { path = "../domain" }` |
| Infrastructure NO depende de Application | ✅ CUMPLE | No hay referencia a `application` |
| API puede depender de todos | ✅ CUMPLE | Es el punto de composición |

---

## 2. Dependencias de Tipos entre Bounded Contexts

### 2.1 Shared Kernel → Otros Contextos

```
shared_kernel provides:
├── types (JobId, ProviderId, ProviderType, JobState, etc.)
├── error (DomainError, DomainResult)
├── events (DomainEvent, EventPublisher, EventSubscriber, EventStore)
└── traits (ProviderWorker, ProviderWorkerBuilder)

Used by:
├── job_execution ─────────► types, error
├── provider_management ───► types, error
└── execution_coordination ► types, error, job_execution.Job, provider_management.Provider
```

### 2.2 Job Execution Context

```
job_execution exports:
├── Job (Aggregate Root)
├── JobRepository (Port)
├── JobScheduler (Service)
├── CreateJobUseCase, ExecuteJobUseCase, GetJobResultUseCase
├── JobSpec (Value Object)
└── ExecutionContext (Value Object)

Dependencies:
├── shared_kernel::types (JobId, ProviderId, JobState, ProviderType, ProviderCapabilities)
└── shared_kernel::error (DomainError, DomainResult)
```

### 2.3 Provider Management Context

```
provider_management exports:
├── Provider (Aggregate Root)
├── ProviderStatus (Enum)
├── ProviderRepository (Port)
├── ProviderRegistry (Service)
├── RegisterProviderUseCase, ListProvidersUseCase
└── ProviderConfig (Value Object)

Dependencies:
├── shared_kernel::types (ProviderId, ProviderType, ProviderCapabilities)
└── shared_kernel::error (DomainError, DomainResult)
```

### 2.4 Execution Coordination Context

```
execution_coordination exports:
└── ExecutionCoordinator (Service)

Dependencies:
├── shared_kernel::error (DomainResult)
├── job_execution::Job ─────────► CROSS-CONTEXT DEPENDENCY
└── provider_management::Provider ► CROSS-CONTEXT DEPENDENCY
```

**⚠️ Nota:** `execution_coordination` cruza bounded contexts, lo cual es aceptable para un servicio de coordinación pero debe monitorearse.

---

## 3. Relaciones de Implementación (Ports & Adapters)

### 3.1 Job Repository Port

```
┌─────────────────────────────────────────────────────────────┐
│ domain::job_execution::repositories::JobRepository (trait) │
└─────────────────────────────────────────────────────────────┘
                            ▲
                            │ implements
         ┌──────────────────┴──────────────────┐
         │                                     │
┌────────┴────────┐              ┌─────────────┴─────────────┐
│InMemoryJobRepo  │              │  PostgresJobRepository   │
│(infrastructure) │              │  (infrastructure)        │
└─────────────────┘              └───────────────────────────┘
```

**Métodos del Port:**
| Método | Parámetros | Retorno |
|--------|------------|---------|
| `save` | `&Job` | `DomainResult<()>` |
| `find_by_id` | `&JobId` | `DomainResult<Option<Job>>` |
| `list` | - | `DomainResult<Vec<Job>>` |
| `delete` | `&JobId` | `DomainResult<()>` |

### 3.2 Provider Repository Port

```
┌────────────────────────────────────────────────────────────────────┐
│ domain::provider_management::repositories::ProviderRepository     │
└────────────────────────────────────────────────────────────────────┘
                            ▲
                            │ implements
         ┌──────────────────┴──────────────────┐
         │                                     │
┌────────┴──────────┐          ┌───────────────┴───────────────┐
│InMemoryProviderRepo│          │ PostgresProviderRepository   │
│(infrastructure)    │          │ (infrastructure)             │
└────────────────────┘          └───────────────────────────────┘
```

### 3.3 Event Publisher Port

```
┌─────────────────────────────────────────────────────────────────┐
│ domain::shared_kernel::events::EventPublisher (trait)           │
└─────────────────────────────────────────────────────────────────┘
                            ▲
                            │ implements
         ┌──────────────────┴──────────────────┐
         │                                     │
┌────────┴────────────┐        ┌───────────────┴───────────────┐
│ MockEventPublisher  │        │   NatsEventPublisher         │
│ (tests)             │        │   (infrastructure)           │
└─────────────────────┘        └───────────────────────────────┘
```

### 3.4 Event Store Port

```
┌─────────────────────────────────────────────────────────────────┐
│ domain::shared_kernel::events::EventStore (trait)               │
└─────────────────────────────────────────────────────────────────┘
                            ▲
                            │ implements
                            │
               ┌────────────┴────────────┐
               │  InMemoryEventStore     │
               │  (infrastructure)       │
               └─────────────────────────┘
```

### 3.5 Provider Worker Port

```
┌─────────────────────────────────────────────────────────────────┐
│ domain::shared_kernel::traits::ProviderWorker (trait)           │
└─────────────────────────────────────────────────────────────────┘
                            ▲
                            │ implements
                            │
               ┌────────────┴────────────┐
               │  DockerProviderAdapter  │
               │  (infrastructure)       │
               └─────────────────────────┘

Future implementations:
├── KubernetesProviderAdapter
├── LambdaProviderAdapter
├── AzureVMProviderAdapter
└── GCPFunctionsProviderAdapter
```

---

## 4. Flujos de Datos

### 4.1 Crear Job

```
┌─────────┐   CreateJobRequest    ┌────────────────┐
│ Client  │ ───────────────────► │ create_job_    │
└─────────┘                       │ handler        │
                                  └───────┬────────┘
                                          │ CreateJobRequest → JobSpec
                                          ▼
                                  ┌───────────────┐
                                  │  JobService   │
                                  │  .create_job  │
                                  └───────┬───────┘
                                          │
                    ┌─────────────────────┼─────────────────────┐
                    │                     │                     │
                    ▼                     ▼                     ▼
           ┌──────────────┐      ┌──────────────┐      ┌──────────────┐
           │ Job::new()   │      │ job_repo     │      │ orchestrator │
           │ validation   │      │ .save()      │      │ .publish_    │
           └──────────────┘      └──────────────┘      │ job_created  │
                                                        └──────────────┘
```

### 4.2 Ejecutar Job

```
┌─────────┐   POST /jobs/:id/execute   ┌───────────────────┐
│ Client  │ ─────────────────────────► │ execute_job_      │
└─────────┘                             │ handler           │
                                        └─────────┬─────────┘
                                                  │
                                                  ▼
                                        ┌─────────────────┐
                                        │   JobService    │
                                        │  .execute_job   │
                                        └─────────┬───────┘
                                                  │
                    ┌─────────────────────────────┼───────────────┐
                    │                             │               │
                    ▼                             ▼               ▼
           ┌──────────────┐              ┌──────────────┐ ┌──────────────┐
           │ job_repo     │              │ job.submit_  │ │ job_repo     │
           │ .find_by_id  │              │ to_provider  │ │ .save()      │
           └──────────────┘              │ (state→Run)  │ └──────────────┘
                                         └──────────────┘
```

### 4.3 Registrar Provider

```
┌─────────┐  RegisterProviderRequest   ┌─────────────────────┐
│ Client  │ ─────────────────────────► │ register_provider_  │
└─────────┘                             │ handler             │
                                        └──────────┬──────────┘
                                                   │
                                                   ▼
                                        ┌──────────────────────┐
                                        │   ProviderService    │
                                        │ .register_provider   │
                                        └──────────┬───────────┘
                                                   │
                         ┌─────────────────────────┼─────────────────────┐
                         │                         │                     │
                         ▼                         ▼                     │
                ┌──────────────┐          ┌──────────────┐              │
                │Provider::new │          │ provider_repo│              │
                │ (Active)     │          │ .save()      │              │
                └──────────────┘          └──────────────┘              │
                                                                         │
                                                                         ▼
                                                                ┌──────────────┐
                                                                │ Return       │
                                                                │ Provider     │
                                                                └──────────────┘
```

---

## 5. Diagrama de Capas con Dependencias

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              API Layer (crates/api)                             │
│                                                                                 │
│   ┌────────────────┐     ┌────────────────┐     ┌──────────────────────────┐  │
│   │    handlers    │ ──► │ AppState       │ ──► │ Arc<Mutex<Services>>     │  │
│   │    routes      │     │ (composition)  │     │ - JobService             │  │
│   │    middleware  │     └────────────────┘     │ - ProviderService        │  │
│   └────────────────┘                            │ - EventOrchestrator      │  │
│                                                 └──────────────────────────┘  │
└─────────────────────────────────┬───────────────────────────────────────────────┘
                                  │ uses
                                  ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                       Application Layer (crates/application)                    │
│                                                                                 │
│   ┌────────────────────────────────────────────────────────────────────────┐   │
│   │                           Application Services                          │   │
│   │                                                                         │   │
│   │  ┌─────────────────┐  ┌───────────────────┐  ┌───────────────────────┐ │   │
│   │  │   JobService    │  │  ProviderService  │  │   EventOrchestrator   │ │   │
│   │  │                 │  │                   │  │                       │ │   │
│   │  │ Box<dyn JobRepo>│  │Box<dyn ProvRepo>  │  │Box<dyn EventPublisher>│ │   │
│   │  └────────┬────────┘  └─────────┬─────────┘  └───────────┬───────────┘ │   │
│   │           │                     │                        │             │   │
│   └───────────┼─────────────────────┼────────────────────────┼─────────────┘   │
│               │                     │                        │                 │
└───────────────┼─────────────────────┼────────────────────────┼─────────────────┘
                │                     │                        │
                │ uses traits         │ uses traits            │ uses traits
                ▼                     ▼                        ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Domain Layer (crates/domain)                          │
│                                                                                 │
│  ┌───────────────────────────────────────────────────────────────────────────┐ │
│  │                             shared_kernel                                  │ │
│  │   JobId, ProviderId, DomainError, DomainEvent, ProviderWorker, etc.      │ │
│  └───────────────────────────────────────────────────────────────────────────┘ │
│                                                                                 │
│  ┌─────────────────────┐ ┌───────────────────────┐ ┌─────────────────────────┐│
│  │   job_execution     │ │  provider_management  │ │ execution_coordination  ││
│  │                     │ │                       │ │                         ││
│  │ ┌─────────────────┐ │ │ ┌───────────────────┐ │ │ ┌─────────────────────┐ ││
│  │ │ JobRepository   │◄├─┼─┤ProviderRepository │◄├─┼─┤ExecutionCoordinator │ ││
│  │ │ (trait/port)    │ │ │ │ (trait/port)      │ │ │ │ (service)           │ ││
│  │ └─────────────────┘ │ │ └───────────────────┘ │ │ └─────────────────────┘ ││
│  │ ┌─────────────────┐ │ │ ┌───────────────────┐ │ │                         ││
│  │ │ Job (entity)    │ │ │ │ Provider (entity) │ │ │                         ││
│  │ └─────────────────┘ │ │ └───────────────────┘ │ │                         ││
│  │ ┌─────────────────┐ │ │ ┌───────────────────┐ │ │                         ││
│  │ │ JobSpec, etc.   │ │ │ │ ProviderConfig    │ │ │                         ││
│  │ └─────────────────┘ │ │ └───────────────────┘ │ │                         ││
│  └─────────────────────┘ └───────────────────────┘ └─────────────────────────┘│
│                                      ▲                                         │
└──────────────────────────────────────┼─────────────────────────────────────────┘
                                       │
                                       │ implements (Dependency Inversion)
                                       │
┌──────────────────────────────────────┼─────────────────────────────────────────┐
│                   Infrastructure Layer (crates/infrastructure)                 │
│                                                                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                          Repository Implementations                     │   │
│  │                                                                         │   │
│  │  ┌─────────────────────┐  ┌───────────────────────────────────────────┐│   │
│  │  │ InMemoryJobRepo     │  │ PostgresJobRepository                     ││   │
│  │  │ InMemoryProviderRepo│  │ PostgresProviderRepository                ││   │
│  │  └─────────────────────┘  └───────────────────────────────────────────┘│   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                              Event Bus                                  │   │
│  │  ┌─────────────────────┐  ┌──────────────────────────────────────────┐ │   │
│  │  │ NatsEventPublisher  │  │ InMemoryEventStore                       │ │   │
│  │  └─────────────────────┘  └──────────────────────────────────────────┘ │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐   │
│  │                              Adapters                                   │   │
│  │  ┌─────────────────────┐                                               │   │
│  │  │ DockerProviderAdapter│ ──► implements ProviderWorker                │   │
│  │  └─────────────────────┘                                               │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │   DatabasePool  │  │  PgBouncerPool  │  │     HealthChecker          │   │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Resumen de Acoplamiento

### 6.1 Niveles de Acoplamiento

| Relación | Tipo de Acoplamiento | Nivel | Notas |
|----------|---------------------|-------|-------|
| API → Application | Datos (DTOs) + Control | Medio | Necesario para composición |
| API → Domain | Datos (tipos) | Bajo | Solo usa tipos compartidos |
| API → Infrastructure | Creación | Bajo | Solo para crear implementaciones |
| Application → Domain | Datos + Control | Bajo | Via abstracciones (traits) |
| Infrastructure → Domain | Contrato (implementa) | Bajo | Inversión de dependencias |
| job_execution → shared_kernel | Datos | Bajo | Tipos compartidos |
| provider_mgmt → shared_kernel | Datos | Bajo | Tipos compartidos |
| exec_coord → job_execution | Datos | Medio | Usa entidad directamente |
| exec_coord → provider_mgmt | Datos | Medio | Usa entidad directamente |

### 6.2 Matriz de Impacto de Cambios

Si cambio... | Impacta a...
-------------|-------------
`JobId` (shared_kernel) | job_execution, provider_mgmt, exec_coord, application, infrastructure, api
`Job` entity | job_execution, exec_coord, application, api handlers
`JobRepository` trait | infrastructure (implementations)
`JobService` | api handlers
`PostgresJobRepository` | Ninguno (implementación aislada)

---

*Matriz de dependencias generada a partir del análisis del código fuente*

