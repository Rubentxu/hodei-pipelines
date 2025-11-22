# Testing End-to-End (E2E) - Hodei CI/CD Platform

**Documento de Investigación y Diseño**  
**Autor**: Hodei Team  
**Fecha**: 2025-11-22  
**Versión**: 1.0  

---

## 📋 Tabla de Contenidos

1. [Visión General](#visión-general)
2. [Análisis de la Arquitectura Actual](#análisis-de-la-arquitectura-actual)
3. [Estrategia de Testing E2E](#estrategia-de-testing-ee)
4. [Infraestructura de Testing](#infraestructura-de-testing)
5. [Trazabilidad y Observabilidad](#trazabilidad-y-observabilidad)
6. [Validación de Requisitos](#validación-de-requisitos)
7. [Plan de Implementación](#plan-de-implementación)
8. [Herramientas y Tecnologías](#herramientas-y-tecnologías)
9. [Escenarios de Testing](#escenarios-de-testing)
10. [Troubleshooting y Debugging](#troubleshooting-y-debugging)

---

## 🎯 Visión General

### Objetivo

Implementar un sistema de testing End-to-End (E2E) completo y automatizado que permita:

1. **Validar la integración completa** de todos los componentes del sistema
2. **Probar flujos de trabajo reales** desde la creación de pipelines hasta su ejecución
3. **Verificar el cumplimiento de requisitos** funcionales y no funcionales
4. **Proporcionar trazabilidad clara** de todas las operaciones
5. **Detectar problemas de integración** antes de producción
6. **Servir como documentación viva** del comportamiento del sistema

### Alcance del Testing E2E

```
┌─────────────────────────────────────────────────────────────────┐
│                        E2E Test Scope                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────┐      ┌──────────────┐      ┌──────────────┐    │
│  │   SDK    │─────→│     API      │─────→│ Orchestrator │    │
│  │  Client  │      │   Gateway    │      │              │    │
│  └──────────┘      └──────────────┘      └──────┬───────┘    │
│                                                   │             │
│                                                   ↓             │
│                                          ┌────────────────┐    │
│                                          │   Scheduler    │    │
│                                          └────────┬───────┘    │
│                                                   │             │
│                                                   ↓             │
│                                          ┌────────────────┐    │
│                                          │ Worker Manager │    │
│                                          └────────┬───────┘    │
│                                                   │             │
│                                                   ↓             │
│                                          ┌────────────────┐    │
│                                          │  Docker Worker │    │
│                                          └────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Componentes a Probar

1. **SDK Rust** - Cliente de alto nivel
2. **API REST** - Endpoints HTTP/gRPC
3. **Orchestrator** - Coordinación de pipelines
4. **Scheduler** - Planificación de jobs
5. **Worker Manager** - Gestión de workers
6. **Docker Provider** - Provisión de workers
7. **Docker Workers** - Ejecución de jobs
8. **Distributed Communication** - NATS/messaging
9. **Metrics Collection** - Prometheus
10. **Credential Rotation** - Seguridad

---

## 🏗️ Análisis de la Arquitectura Actual

### Estructura del Sistema

```
hodei-jobs/
├── crates/
│   ├── orchestration/
│   │   ├── orchestrator/          # Orquestador principal
│   │   ├── coordinator/           # Coordinador de tareas
│   │   ├── distributed-comm/      # Comunicación distribuida
│   │   ├── monitoring/            # Monitorización
│   │   ├── worker-lifecycle/      # Ciclo de vida de workers
│   │   └── concurrency-patterns/  # Patrones de concurrencia
│   │
│   ├── scheduler/                 # Planificador de jobs
│   │   ├── pipeline/              # Scheduler de pipelines
│   │   ├── queue/                 # Gestión de colas
│   │   ├── selection/             # Selección de workers
│   │   └── affinity/              # Afinidad de workers
│   │
│   ├── provider-abstraction/      # Abstracción de providers
│   ├── docker-provider/           # Provider Docker
│   ├── kubernetes-provider/       # Provider Kubernetes
│   │
│   ├── worker-lifecycle-manager/  # Gestor de ciclo de vida
│   ├── credential-rotation/       # Rotación de credenciales
│   │
│   ├── observability/
│   │   └── metrics-collection/    # Recolección de métricas
│   │
│   └── developer-experience/
│       ├── sdk-core/              # Core del SDK
│       └── rust-sdk/              # SDK Rust
│
└── docs/                          # Documentación
```

### Flujo de Datos del Sistema

```
┌─────────────────────────────────────────────────────────────────┐
│                    Pipeline Execution Flow                      │
└─────────────────────────────────────────────────────────────────┘

1. Client Request (SDK/API)
   ↓
2. API Gateway / HTTP Server
   ↓
3. Orchestrator
   ├─→ Validate Pipeline Config
   ├─→ Create Job Entity
   └─→ Publish to NATS
       ↓
4. Scheduler
   ├─→ Receive Job from Queue
   ├─→ Select Worker (Affinity/Resources)
   └─→ Schedule Job Assignment
       ↓
5. Worker Manager
   ├─→ Provision Worker (if needed)
   ├─→ Assign Job to Worker
   └─→ Monitor Worker Health
       ↓
6. Docker Worker
   ├─→ Pull Image
   ├─→ Execute Commands
   ├─→ Stream Logs
   └─→ Report Status
       ↓
7. Orchestrator (Completion)
   ├─→ Collect Results
   ├─→ Update Job Status
   └─→ Emit Metrics
```

### Dependencias entre Componentes

```rust
// Dependency Graph
orchestrator
  ├─→ scheduler
  ├─→ worker-lifecycle-manager
  ├─→ distributed-comm (NATS)
  └─→ metrics-collection

scheduler
  ├─→ provider-abstraction
  └─→ shared-types

worker-lifecycle-manager
  ├─→ docker-provider
  ├─→ kubernetes-provider
  └─→ credential-rotation

docker-provider
  └─→ provider-abstraction

kubernetes-provider
  └─→ provider-abstraction
```

---

## 🧪 Estrategia de Testing E2E

### Pirámide de Testing

```
        ┌─────────────┐
        │     E2E     │  ← 10% (Flujos completos)
        │   Tests     │
        └─────────────┘
      ┌─────────────────┐
      │   Integration   │  ← 20% (Componentes integrados)
      │     Tests       │
      └─────────────────┘
    ┌───────────────────────┐
    │     Unit Tests        │  ← 70% (Componentes aislados)
    │                       │
    └───────────────────────┘
```

### Niveles de Testing E2E

#### 1. **API Level E2E** (Black Box)
Probar la API pública sin conocimiento interno.

```
Test Client → HTTP/gRPC → API → System → Response
```

**Ejemplo**:
```rust
#[tokio::test]
async fn test_create_and_execute_pipeline() {
    let client = setup_test_client().await;
    
    // 1. Create pipeline
    let pipeline = client.create_pipeline(config).await.unwrap();
    
    // 2. Execute pipeline
    let job = client.execute_pipeline(&pipeline.id).await.unwrap();
    
    // 3. Wait for completion
    let result = client.wait_for_completion(&job.id).await.unwrap();
    
    // 4. Verify result
    assert_eq!(result.status, JobStatus::Success);
}
```

#### 2. **SDK Level E2E** (Gray Box)
Probar a través del SDK con conocimiento de la arquitectura.

```
SDK Client → API → Orchestrator → Scheduler → Workers
```

**Ejemplo**:
```rust
#[tokio::test]
async fn test_multi_stage_pipeline_execution() {
    let sdk_client = CicdClient::new(url, token).unwrap();
    
    let pipeline = PipelineBuilder::new("multi-stage")
        .stage("build", "rust:1.70", vec!["cargo build"])
        .stage("test", "rust:1.70", vec!["cargo test"])
        .build();
    
    let created = sdk_client.create_pipeline(pipeline).await.unwrap();
    let job = sdk_client.execute_pipeline(&created.id).await.unwrap();
    
    // Verify execution through all stages
    verify_all_stages_executed(&job.id).await;
}
```

#### 3. **System Level E2E** (White Box)
Probar con acceso completo a componentes internos.

```
Test → All Components (with observability)
```

### Tipos de Tests E2E

#### A. **Happy Path Tests**
Flujos exitosos sin errores.

```rust
- Create pipeline → Execute → Complete successfully
- Scale workers → Assign jobs → Execute
- Rotate credentials → Continue operations
```

#### B. **Error Handling Tests**
Manejo de errores y recuperación.

```rust
- Invalid pipeline configuration → Error response
- Worker failure → Job retry → Success
- Network partition → Reconnection → Resume
```

#### C. **Performance Tests**
Validación de requisitos no funcionales.

```rust
- 100 concurrent pipeline executions
- Worker scaling under load
- Message throughput in NATS
```

#### D. **Chaos Engineering Tests**
Resiliencia ante fallos.

```rust
- Kill random worker → Job reassignment
- Network delay → Timeout handling
- Resource exhaustion → Graceful degradation
```

---

## 🐳 Infraestructura de Testing

### Opción 1: Testcontainers-rs (Recomendado)

**Ventajas**:
- ✅ Integración nativa con Rust
- ✅ Gestión automática del ciclo de vida de containers
- ✅ Cleanup automático
- ✅ Portabilidad entre entornos
- ✅ Soporte para Docker Compose

**Desventajas**:
- ❌ Requiere Docker daemon
- ❌ Overhead de inicio de containers
- ❌ Puede ser lento en CI/CD

**Implementación**:

```rust
use testcontainers::*;

#[tokio::test]
async fn test_with_testcontainers() {
    let docker = clients::Cli::default();
    
    // Start NATS
    let nats_container = docker.run(images::generic::GenericImage::new("nats", "latest"));
    let nats_port = nats_container.get_host_port_ipv4(4222);
    
    // Start Prometheus
    let prometheus_container = docker.run(
        images::generic::GenericImage::new("prom/prometheus", "latest")
    );
    
    // Start application components
    let orchestrator = start_orchestrator(nats_port).await;
    let scheduler = start_scheduler(nats_port).await;
    
    // Run tests
    run_pipeline_test(&orchestrator).await;
    
    // Cleanup is automatic when containers go out of scope
}
```

### Opción 2: Docker Compose para E2E

**Ventajas**:
- ✅ Configuración declarativa
- ✅ Fácil de versionar
- ✅ Reutilizable en dev y CI/CD
- ✅ Soporta redes y volúmenes

**Desventajas**:
- ❌ Menos integración con tests
- ❌ Cleanup manual necesario
- ❌ Difícil de parametrizar

**Implementación**:

```yaml
# docker-compose.e2e.yml
version: '3.8'

services:
  nats:
    image: nats:latest
    ports:
      - "4222:4222"
      - "8222:8222"
    networks:
      - hodei-test-network
  
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./config/prometheus.yml:/etc/prometheus/prometheus.yml
    networks:
      - hodei-test-network
  
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: hodei_test
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
    ports:
      - "5432:5432"
    networks:
      - hodei-test-network
  
  redis:
    image: redis:7
    ports:
      - "6379:6379"
    networks:
      - hodei-test-network
  
  orchestrator:
    build:
      context: .
      dockerfile: Dockerfile.orchestrator
    environment:
      NATS_URL: nats://nats:4222
      PROMETHEUS_URL: http://prometheus:9090
      LOG_LEVEL: debug
    depends_on:
      - nats
      - prometheus
      - postgres
    networks:
      - hodei-test-network
  
  scheduler:
    build:
      context: .
      dockerfile: Dockerfile.scheduler
    environment:
      NATS_URL: nats://nats:4222
    depends_on:
      - nats
    networks:
      - hodei-test-network
  
  worker-manager:
    build:
      context: .
      dockerfile: Dockerfile.worker-manager
    environment:
      DOCKER_HOST: unix:///var/run/docker.sock
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    depends_on:
      - nats
    networks:
      - hodei-test-network

networks:
  hodei-test-network:
    driver: bridge
```

### Opción 3: Hybrid Approach (Mejor Opción)

Combinar Testcontainers para infraestructura y Docker para workers.

```rust
pub struct E2ETestEnvironment {
    nats: Container<'static, GenericImage>,
    prometheus: Container<'static, GenericImage>,
    postgres: Container<'static, Postgres>,
    orchestrator: OrchestorHandle,
    scheduler: SchedulerHandle,
    worker_manager: WorkerManagerHandle,
}

impl E2ETestEnvironment {
    pub async fn new() -> Self {
        let docker = clients::Cli::default();
        
        // Start infrastructure containers
        let nats = docker.run(nats_image());
        let prometheus = docker.run(prometheus_image());
        let postgres = docker.run(postgres_image());
        
        // Wait for services to be ready
        wait_for_nats(&nats).await;
        wait_for_prometheus(&prometheus).await;
        wait_for_postgres(&postgres).await;
        
        // Start application components
        let config = create_test_config(&nats, &prometheus, &postgres);
        let orchestrator = Orchestrator::start(config.clone()).await;
        let scheduler = Scheduler::start(config.clone()).await;
        let worker_manager = WorkerManager::start(config).await;
        
        Self {
            nats,
            prometheus,
            postgres,
            orchestrator,
            scheduler,
            worker_manager,
        }
    }
    
    pub async fn create_pipeline(&self, config: PipelineConfig) -> Pipeline {
        self.orchestrator.create_pipeline(config).await
    }
    
    pub async fn execute_pipeline(&self, pipeline_id: &str) -> Job {
        self.orchestrator.execute_pipeline(pipeline_id).await
    }
    
    pub async fn get_metrics(&self) -> Metrics {
        query_prometheus(&self.prometheus).await
    }
}

impl Drop for E2ETestEnvironment {
    fn drop(&mut self) {
        // Cleanup application components
        self.orchestrator.shutdown();
        self.scheduler.shutdown();
        self.worker_manager.shutdown();
        
        // Containers cleanup automatically via testcontainers
    }
}
```

---

## 📊 Trazabilidad y Observabilidad

### Correlation IDs

Cada operación debe tener un ID único que se propague por todos los componentes.

```rust
#[derive(Debug, Clone)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// En cada componente
pub struct JobContext {
    pub job_id: String,
    pub pipeline_id: String,
    pub correlation_id: CorrelationId,
    pub span: Span,
}

// Logging con correlation ID
tracing::info!(
    correlation_id = %context.correlation_id.as_str(),
    job_id = %context.job_id,
    "Job execution started"
);
```

### Structured Logging

```rust
use tracing::{info, warn, error, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[instrument(
    name = "execute_pipeline",
    skip(self),
    fields(
        pipeline_id = %pipeline_id,
        correlation_id = tracing::field::Empty,
    )
)]
pub async fn execute_pipeline(&self, pipeline_id: &str) -> Result<Job> {
    let correlation_id = CorrelationId::new();
    tracing::Span::current().record("correlation_id", correlation_id.as_str());
    
    info!("Starting pipeline execution");
    
    // ... execution logic
    
    info!(
        stages_count = pipeline.stages.len(),
        "Pipeline execution completed"
    );
    
    Ok(job)
}
```

### OpenTelemetry Integration

```rust
use opentelemetry::{global, sdk::trace as sdktrace};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::layer::SubscriberExt;

pub fn init_telemetry() -> Result<()> {
    // Setup OpenTelemetry pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            sdktrace::config()
                .with_sampler(sdktrace::Sampler::AlwaysOn)
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "hodei-cicd"),
                ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    // Setup tracing subscriber
    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}
```

### Metrics Collection Strategy

```rust
use prometheus::{Counter, Histogram, Registry};

pub struct JobMetrics {
    pub jobs_total: Counter,
    pub jobs_failed: Counter,
    pub jobs_duration: Histogram,
    pub workers_active: Gauge,
}

impl JobMetrics {
    pub fn new(registry: &Registry) -> Self {
        let jobs_total = Counter::new(
            "hodei_jobs_total",
            "Total number of jobs executed"
        ).unwrap();
        registry.register(Box::new(jobs_total.clone())).unwrap();
        
        let jobs_failed = Counter::new(
            "hodei_jobs_failed",
            "Total number of failed jobs"
        ).unwrap();
        registry.register(Box::new(jobs_failed.clone())).unwrap();
        
        let jobs_duration = Histogram::with_opts(
            histogram_opts!(
                "hodei_job_duration_seconds",
                "Job execution duration in seconds",
                vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]
            )
        ).unwrap();
        registry.register(Box::new(jobs_duration.clone())).unwrap();
        
        Self {
            jobs_total,
            jobs_failed,
            jobs_duration,
            workers_active,
        }
    }
    
    pub fn record_job_completion(&self, duration: Duration, success: bool) {
        self.jobs_total.inc();
        if !success {
            self.jobs_failed.inc();
        }
        self.jobs_duration.observe(duration.as_secs_f64());
    }
}
```

### Event Stream for Testing

```rust
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum SystemEvent {
    PipelineCreated { pipeline_id: String, correlation_id: String },
    JobScheduled { job_id: String, worker_id: String },
    JobStarted { job_id: String, timestamp: DateTime<Utc> },
    JobCompleted { job_id: String, status: JobStatus, duration: Duration },
    WorkerProvisioned { worker_id: String, provider: String },
    WorkerTerminated { worker_id: String, reason: String },
}

pub struct EventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx }
    }
    
    pub fn publish(&self, event: SystemEvent) {
        let _ = self.tx.send(event);
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }
}

// En tests
#[tokio::test]
async fn test_pipeline_execution_events() {
    let env = E2ETestEnvironment::new().await;
    let mut events = env.event_bus.subscribe();
    
    // Execute pipeline
    let job = env.execute_pipeline("pipeline-123").await;
    
    // Collect events
    let mut collected_events = Vec::new();
    timeout(Duration::from_secs(30), async {
        while let Ok(event) = events.recv().await {
            collected_events.push(event.clone());
            if matches!(event, SystemEvent::JobCompleted { .. }) {
                break;
            }
        }
    }).await.unwrap();
    
    // Verify event sequence
    assert!(matches!(collected_events[0], SystemEvent::JobScheduled { .. }));
    assert!(matches!(collected_events[1], SystemEvent::JobStarted { .. }));
    assert!(matches!(collected_events[2], SystemEvent::JobCompleted { .. }));
}
```

---

## ✅ Validación de Requisitos

### Test Requirements Matrix

| Requisito | Test Type | Validation Strategy | Evidence |
|-----------|-----------|---------------------|----------|
| Pipeline debe ejecutarse en < 5min | Performance | Measure execution time | Metrics + Logs |
| 99.9% uptime | Reliability | Chaos testing | Downtime tracking |
| Scale to 1000 workers | Scalability | Load testing | Worker count metrics |
| Job retry on failure | Resilience | Failure injection | Event logs |
| Secure credential rotation | Security | Security testing | Audit logs |
| API response < 100ms | Performance | API benchmarks | Response time metrics |

### Evidence Collection

```rust
pub struct TestEvidence {
    pub test_name: String,
    pub correlation_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub logs: Vec<LogEntry>,
    pub metrics: HashMap<String, f64>,
    pub events: Vec<SystemEvent>,
    pub traces: Vec<Span>,
    pub screenshots: Vec<Screenshot>, // For UI tests
    pub artifacts: Vec<PathBuf>,
}

impl TestEvidence {
    pub fn collect(correlation_id: &str) -> Self {
        let logs = collect_logs_by_correlation_id(correlation_id);
        let metrics = query_metrics_for_test(correlation_id);
        let events = get_events_for_correlation_id(correlation_id);
        let traces = export_traces(correlation_id);
        
        Self {
            test_name: current_test_name(),
            correlation_id: correlation_id.to_string(),
            start_time: test_start_time(),
            end_time: Utc::now(),
            logs,
            metrics,
            events,
            traces,
            artifacts: vec![],
        }
    }
    
    pub fn export_html(&self, path: &Path) -> Result<()> {
        // Generate HTML report with all evidence
        let html = self.generate_html_report();
        std::fs::write(path, html)?;
        Ok(())
    }
    
    pub fn export_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// Usage in tests
#[tokio::test]
async fn test_with_evidence_collection() {
    let correlation_id = CorrelationId::new();
    let env = E2ETestEnvironment::new().await;
    
    // Run test
    let result = env.execute_pipeline_with_correlation(&correlation_id).await;
    
    // Collect evidence
    let evidence = TestEvidence::collect(correlation_id.as_str());
    evidence.export_html(Path::new("test-reports/pipeline-execution.html")).unwrap();
    evidence.export_json(Path::new("test-reports/pipeline-execution.json")).unwrap();
    
    // Assertions
    assert!(result.is_ok());
    assert_eq!(evidence.metrics.get("job_duration_seconds").unwrap(), &45.2);
}
```

---

## 📝 Plan de Implementación

### Fase 1: Infraestructura Base (Semana 1)

**Objetivos**:
- ✅ Setup de Testcontainers
- ✅ Docker Compose para desarrollo
- ✅ CI/CD pipeline básico

**Tareas**:
1. Crear `crates/e2e-tests` con estructura base
2. Implementar `E2ETestEnvironment`
3. Setup de NATS, Prometheus, Postgres con Testcontainers
4. Configurar logging estructurado
5. Implementar CorrelationId propagation

**Deliverables**:
- `e2e-tests/src/infrastructure/`
- `docker-compose.e2e.yml`
- `.github/workflows/e2e-tests.yml`

### Fase 2: Happy Path Tests (Semana 2)

**Objetivos**:
- ✅ Tests de flujos principales
- ✅ Validación básica de requisitos

**Tareas**:
1. Test: Create simple pipeline
2. Test: Execute pipeline with single stage
3. Test: Execute multi-stage pipeline
4. Test: List pipelines
5. Test: Worker provisioning

**Deliverables**:
- `e2e-tests/tests/happy_path/`
- Test reports en HTML/JSON

### Fase 3: Error Handling & Edge Cases (Semana 3)

**Objetivos**:
- ✅ Resilience testing
- ✅ Error recovery validation

**Tareas**:
1. Test: Invalid pipeline configuration
2. Test: Worker failure and retry
3. Test: Network partition recovery
4. Test: Resource exhaustion
5. Test: Concurrent pipeline execution

**Deliverables**:
- `e2e-tests/tests/error_handling/`
- `e2e-tests/tests/edge_cases/`

### Fase 4: Performance & Chaos (Semana 4)

**Objetivos**:
- ✅ Performance validation
- ✅ Chaos engineering

**Tareas**:
1. Test: 100 concurrent pipelines
2. Test: Worker scaling under load
3. Test: Message throughput
4. Chaos: Random worker kills
5. Chaos: Network delays

**Deliverables**:
- `e2e-tests/tests/performance/`
- `e2e-tests/tests/chaos/`
- Performance benchmarks

### Fase 5: Observability & Evidence (Semana 5)

**Objetivos**:
- ✅ Complete traceability
- ✅ Evidence collection

**Tareas**:
1. OpenTelemetry integration
2. Jaeger for distributed tracing
3. Evidence collector implementation
4. HTML report generator
5. Metrics dashboard

**Deliverables**:
- `e2e-tests/src/observability/`
- `e2e-tests/src/evidence/`
- Grafana dashboards

---

## 🛠️ Herramientas y Tecnologías

### Testing Framework

```toml
[dev-dependencies]
# Test runtime
tokio-test = "0.4"
serial_test = "3.0"

# Testcontainers
testcontainers = "0.25"
testcontainers-modules = { version = "0.13", features = ["postgres", "redis"] }

# HTTP testing
reqwest = { version = "0.12", features = ["json"] }
wiremock = "0.6"

# Assertions
assert_matches = "1.5"
pretty_assertions = "1.4"

# Test data generation
fake = "2.9"
quickcheck = "1.0"

# Benchmarking
criterion = { version = "0.5", features = ["async_tokio"] }
```

### Observability Stack

```toml
[dependencies]
# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tracing-opentelemetry = "0.22"

# Metrics
prometheus = "0.13"
prometheus-client = "0.22"

# Tracing
opentelemetry = "0.21"
opentelemetry-otlp = "0.14"
opentelemetry-jaeger = "0.20"

# Correlation
uuid = { version = "1.0", features = ["v4"] }
```

### Docker Images for Testing

```dockerfile
# Dockerfile.e2e-orchestrator
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin orchestrator

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/orchestrator /usr/local/bin/
ENV RUST_LOG=debug
CMD ["orchestrator"]
```

### CI/CD Integration

```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  e2e-tests:
    runs-on: ubuntu-latest
    
    services:
      docker:
        image: docker:dind
        options: --privileged
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Start infrastructure
        run: docker-compose -f docker-compose.e2e.yml up -d
      
      - name: Wait for services
        run: |
          timeout 60 bash -c 'until curl -f http://localhost:4222; do sleep 1; done'
          timeout 60 bash -c 'until curl -f http://localhost:9090/-/ready; do sleep 1; done'
      
      - name: Run E2E tests
        run: cargo test --package e2e-tests --verbose
        env:
          RUST_LOG: debug
          TEST_NATS_URL: nats://localhost:4222
          TEST_PROMETHEUS_URL: http://localhost:9090
      
      - name: Collect logs
        if: failure()
        run: |
          docker-compose -f docker-compose.e2e.yml logs > e2e-logs.txt
      
      - name: Upload test artifacts
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: e2e-test-results
          path: |
            test-reports/
            e2e-logs.txt
      
      - name: Generate test report
        if: always()
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Html --output-dir coverage
      
      - name: Cleanup
        if: always()
        run: docker-compose -f docker-compose.e2e.yml down -v
```

---

## 🎭 Escenarios de Testing

### Escenario 1: Simple Pipeline Execution

```rust
#[tokio::test]
async fn test_simple_pipeline_execution() {
    // Setup
    let env = E2ETestEnvironment::new().await;
    let correlation_id = CorrelationId::new();
    
    // Create pipeline
    let pipeline_config = PipelineConfig {
        name: "hello-world".to_string(),
        stages: vec![
            StageConfig {
                name: "echo".to_string(),
                image: "alpine:latest".to_string(),
                commands: vec!["echo Hello, World!".to_string()],
                dependencies: vec![],
                environment: HashMap::new(),
                resources: None,
            }
        ],
        environment: HashMap::new(),
        triggers: vec![],
    };
    
    // Execute
    let pipeline = env.create_pipeline(pipeline_config).await.unwrap();
    let job = env.execute_pipeline(&pipeline.id).await.unwrap();
    
    // Wait for completion
    let result = env.wait_for_job_completion(&job.id, Duration::from_secs(60)).await.unwrap();
    
    // Verify
    assert_eq!(result.status, JobStatus::Success);
    
    // Collect evidence
    let evidence = TestEvidence::collect(&correlation_id);
    assert!(evidence.logs.iter().any(|l| l.message.contains("Hello, World!")));
    
    // Verify metrics
    let metrics = env.get_job_metrics(&job.id).await.unwrap();
    assert!(metrics.duration_seconds < 10.0);
}
```

### Escenario 2: Multi-Stage Pipeline with Dependencies

```rust
#[tokio::test]
async fn test_multi_stage_pipeline_with_dependencies() {
    let env = E2ETestEnvironment::new().await;
    
    let pipeline = PipelineBuilder::new("rust-build-pipeline")
        .stage("checkout", "alpine/git:latest", vec![
            "git clone https://github.com/example/repo.git /workspace"
        ])
        .stage_with_deps(
            "build",
            "rust:1.75",
            vec!["cd /workspace", "cargo build --release"],
            vec!["checkout"]
        )
        .stage_with_deps(
            "test",
            "rust:1.75",
            vec!["cd /workspace", "cargo test"],
            vec!["build"]
        )
        .build();
    
    let created = env.create_pipeline(pipeline).await.unwrap();
    let job = env.execute_pipeline(&created.id).await.unwrap();
    
    // Monitor stage execution
    let mut stage_events = Vec::new();
    let mut events_rx = env.event_bus.subscribe();
    
    timeout(Duration::from_secs(300), async {
        while let Ok(event) = events_rx.recv().await {
            if let SystemEvent::StageCompleted { stage_name, .. } = &event {
                stage_events.push(stage_name.clone());
            }
            if stage_events.len() == 3 {
                break;
            }
        }
    }).await.unwrap();
    
    // Verify execution order
    assert_eq!(stage_events, vec!["checkout", "build", "test"]);
}
```

### Escenario 3: Worker Scaling Test

```rust
#[tokio::test]
async fn test_worker_auto_scaling() {
    let env = E2ETestEnvironment::new().await;
    
    // Configure auto-scaling
    env.worker_manager.configure_auto_scaling(AutoScalingConfig {
        min_workers: 1,
        max_workers: 10,
        target_cpu_utilization: 0.7,
        scale_up_threshold: 0.8,
        scale_down_threshold: 0.3,
    }).await.unwrap();
    
    // Create load (20 pipelines)
    let mut jobs = Vec::new();
    for i in 0..20 {
        let pipeline = PipelineBuilder::new(&format!("load-test-{}", i))
            .stage("work", "alpine:latest", vec!["sleep 30"])
            .build();
        
        let created = env.create_pipeline(pipeline).await.unwrap();
        let job = env.execute_pipeline(&created.id).await.unwrap();
        jobs.push(job);
    }
    
    // Wait for scaling
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Verify worker count increased
    let workers = env.list_workers().await.unwrap();
    assert!(workers.len() > 1, "Should have scaled up workers");
    assert!(workers.len() <= 10, "Should not exceed max workers");
    
    // Wait for jobs completion
    for job in jobs {
        env.wait_for_job_completion(&job.id, Duration::from_secs(120)).await.unwrap();
    }
    
    // Wait for scale down
    tokio::time::sleep(Duration::from_secs(60)).await;
    
    // Verify scale down
    let workers_after = env.list_workers().await.unwrap();
    assert!(workers_after.len() < workers.len(), "Should have scaled down");
}
```

### Escenario 4: Failure Recovery Test

```rust
#[tokio::test]
async fn test_job_retry_on_worker_failure() {
    let env = E2ETestEnvironment::new().await;
    
    // Create pipeline that will fail initially
    let pipeline = PipelineBuilder::new("flaky-pipeline")
        .stage("flaky-task", "alpine:latest", vec![
            "if [ ! -f /tmp/retry ]; then touch /tmp/retry && exit 1; fi",
            "echo Success"
        ])
        .build();
    
    let created = env.create_pipeline(pipeline).await.unwrap();
    let job = env.execute_pipeline(&created.id).await.unwrap();
    
    // Monitor retry events
    let mut retry_count = 0;
    let mut events_rx = env.event_bus.subscribe();
    
    timeout(Duration::from_secs(120), async {
        while let Ok(event) = events_rx.recv().await {
            if let SystemEvent::JobRetrying { job_id, attempt } = event {
                if job_id == job.id {
                    retry_count = attempt;
                }
            }
            if let SystemEvent::JobCompleted { job_id, status, .. } = event {
                if job_id == job.id && status == JobStatus::Success {
                    break;
                }
            }
        }
    }).await.unwrap();
    
    // Verify retry happened
    assert!(retry_count > 0, "Job should have been retried");
    
    // Verify final success
    let final_status = env.get_job_status(&job.id).await.unwrap();
    assert_eq!(final_status.status, JobStatus::Success);
}
```

### Escenario 5: Chaos Engineering - Network Partition

```rust
#[tokio::test]
#[ignore] // Run separately as chaos test
async fn chaos_test_network_partition() {
    let env = E2ETestEnvironment::new().await;
    
    // Start pipeline
    let pipeline = PipelineBuilder::new("resilience-test")
        .stage("long-running", "alpine:latest", vec!["sleep 60"])
        .build();
    
    let created = env.create_pipeline(pipeline).await.unwrap();
    let job = env.execute_pipeline(&created.id).await.unwrap();
    
    // Wait a bit
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Inject network partition
    env.chaos_controller.inject_network_partition(
        Duration::from_secs(20)
    ).await.unwrap();
    
    // System should recover
    let result = env.wait_for_job_completion(&job.id, Duration::from_secs(120)).await;
    
    // Verify recovery
    assert!(result.is_ok(), "Job should complete despite network partition");
    
    // Collect metrics
    let metrics = env.get_system_metrics().await.unwrap();
    assert!(metrics.get("network_reconnections").unwrap() > &0.0);
}
```

---

## 🔍 Troubleshooting y Debugging

### Debug Utilities

```rust
pub struct DebugUtils;

impl DebugUtils {
    /// Dump full system state
    pub async fn dump_system_state(env: &E2ETestEnvironment) -> SystemState {
        SystemState {
            pipelines: env.list_pipelines().await.unwrap(),
            jobs: env.list_jobs().await.unwrap(),
            workers: env.list_workers().await.unwrap(),
            metrics: env.get_all_metrics().await.unwrap(),
            logs: env.collect_recent_logs(100).await.unwrap(),
        }
    }
    
    /// Verify system health
    pub async fn health_check(env: &E2ETestEnvironment) -> HealthReport {
        HealthReport {
            orchestrator_healthy: env.orchestrator.health_check().await.is_ok(),
            scheduler_healthy: env.scheduler.health_check().await.is_ok(),
            worker_manager_healthy: env.worker_manager.health_check().await.is_ok(),
            nats_healthy: env.nats_health_check().await.is_ok(),
            prometheus_healthy: env.prometheus_health_check().await.is_ok(),
        }
    }
    
    /// Trace request flow
    pub async fn trace_request(
        env: &E2ETestEnvironment,
        correlation_id: &str
    ) -> RequestTrace {
        let logs = env.get_logs_by_correlation_id(correlation_id).await;
        let spans = env.get_spans_by_correlation_id(correlation_id).await;
        let events = env.get_events_by_correlation_id(correlation_id).await;
        
        RequestTrace {
            correlation_id: correlation_id.to_string(),
            logs,
            spans,
            events,
            timeline: Self::build_timeline(&logs, &spans, &events),
        }
    }
}
```

### Common Issues and Solutions

#### Issue 1: Testcontainers Timeout

**Symptom**: Tests hang waiting for containers to start

**Solutions**:
```rust
// Increase timeout
let config = testcontainers::core::WaitFor::message_on_stdout("ready")
    .with_timeout(Duration::from_secs(120));

// Add health check
async fn wait_for_service_ready(port: u16) -> Result<()> {
    let client = reqwest::Client::new();
    for _ in 0..60 {
        if client.get(&format!("http://localhost:{}/health", port))
            .send()
            .await
            .is_ok()
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow!("Service did not become ready"))
}
```

#### Issue 2: Port Conflicts

**Symptom**: "Address already in use"

**Solutions**:
```rust
// Use random ports
let nats = docker.run(
    GenericImage::new("nats", "latest")
        .with_exposed_port(4222) // Let Docker assign random host port
);
let nats_port = nats.get_host_port_ipv4(4222);

// Or use serial test execution
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_with_fixed_ports() {
    // Test will run serially
}
```

#### Issue 3: Flaky Tests

**Symptom**: Tests pass/fail randomly

**Solutions**:
```rust
// Add retry logic
async fn with_retry<F, Fut, T>(f: F, max_attempts: u32) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;
    for attempt in 1..=max_attempts {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!(attempt, "Operation failed, retrying");
                last_error = Some(e);
                tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
            }
        }
    }
    Err(last_error.unwrap())
}

// Use eventually assertions
async fn eventually<F, Fut>(check: F, timeout: Duration) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = bool>,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if check().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(anyhow!("Condition not met within timeout"))
}

#[tokio::test]
async fn test_with_eventual_consistency() {
    let env = E2ETestEnvironment::new().await;
    let job = env.execute_pipeline("pipeline-123").await.unwrap();
    
    // Wait for eventual consistency
    eventually(
        || async {
            let status = env.get_job_status(&job.id).await.ok();
            status.map(|s| s.status == JobStatus::Success).unwrap_or(false)
        },
        Duration::from_secs(30)
    ).await.unwrap();
}
```

---

## 📚 Referencias y Recursos

### Documentación Relacionada

- [Intelligent Scheduler Design](./intelligent_scheduler_design.md)
- [Domain Model Design](./domain_model_design.md)
- [K8s-Style Scheduler](./k8s-style_scheduler_design.md)
- [SDK Implementation Summary](./sdk-implementation-summary.md)

### External Resources

- [Testcontainers for Rust](https://github.com/testcontainers/testcontainers-rs)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [Tracing](https://github.com/tokio-rs/tracing)
- [Prometheus Client](https://github.com/prometheus/client_rust)
- [Docker SDK](https://docs.rs/bollard/latest/bollard/)

### Best Practices

1. **Isolation**: Each test should be independent
2. **Idempotency**: Tests should be repeatable
3. **Speed**: Optimize for fast feedback
4. **Clarity**: Clear test names and assertions
5. **Coverage**: Test critical paths thoroughly
6. **Evidence**: Collect and preserve test artifacts

---

## 📊 Métricas de Éxito

### Test Coverage Goals

- **Unit Tests**: 70% code coverage
- **Integration Tests**: 20% scenario coverage
- **E2E Tests**: 10% critical path coverage
- **Total**: >85% overall coverage

### Performance Targets

- **Test Execution Time**: < 10 minutes for full E2E suite
- **CI/CD Pipeline**: < 15 minutes total
- **Flakiness Rate**: < 1% failed tests due to flakiness
- **Resource Usage**: < 4GB RAM, < 50% CPU during tests

### Quality Metrics

- **Bug Detection Rate**: >90% of bugs caught before production
- **False Positive Rate**: < 5% of test failures are false positives
- **Test Maintenance**: < 2 hours/week for test maintenance
- **Documentation**: 100% of tests have clear documentation

---

## 🎯 Conclusiones y Próximos Pasos

### Conclusiones

Este documento proporciona una estrategia completa para implementar testing E2E en Hodei CI/CD platform, incluyendo:

1. ✅ **Infraestructura robusta** con Testcontainers y Docker Compose
2. ✅ **Estrategia de testing** multinivel (API, SDK, System)
3. ✅ **Trazabilidad completa** con correlation IDs y OpenTelemetry
4. ✅ **Validación de requisitos** con evidence collection
5. ✅ **Plan de implementación** por fases

### Próximos Pasos

1. **Semana 1**: Implementar infraestructura base
2. **Semana 2**: Desarrollar happy path tests
3. **Semana 3**: Añadir error handling tests
4. **Semana 4**: Performance y chaos tests
5. **Semana 5**: Observability y reporting

### Recomendaciones

- Comenzar con **hybrid approach** (Testcontainers + Docker)
- Implementar **correlation IDs** desde el principio
- Configurar **OpenTelemetry** para tracing distribuido
- Establecer **CI/CD pipeline** temprano
- Mantener **documentación actualizada** de tests

---

**Última Actualización**: 2025-11-22  
**Mantenido por**: Hodei Team  
**Versión del Documento**: 1.0
