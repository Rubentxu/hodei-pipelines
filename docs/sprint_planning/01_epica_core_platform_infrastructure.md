# Épica 1: Core Platform & Infrastructure

**Planificación de Sprints - Sistema CI/CD Distribuido**  
**Bounded Context**: Orchestration  
**Autor**: MiniMax Agent  
**Fecha**: 2025-11-21  
**Versión**: 1.0  

## 📋 Índice
1. [Visión de la Épica](#visión-de-la-épica)
2. [Arquitectura Hexagonal Propuesta](#arquitectura-hexagonal-propuesta)
3. [Historias de Usuario](#historias-de-usuario)
4. [Planificación de Sprints](#planificación-de-sprints)
5. [Criterios de Aceptación Detallados](#criterios-de-aceptación-detallados)
6. [Testing Strategy TDD](#testing-strategy-tdd)
7. [Conventional Commits](#conventional-commits)
8. [Referencias Técnicas](#referencias-técnicas)

---

## 🎯 Visión de la Épica

### Objetivo Principal
Establecer la base sólida del sistema CI/CD distribuido implementando el orquestador central, comunicación distribuida robusta y patrones de concurrencia eficientes en Rust.

### Componentes Arquitectónicos
- **Orquestador Central**: Coordinación de jobs y workflows
- **Comunicación Distribuida**: NATS JetStream para messaging
- **Concurrencia**: Tokio runtime con patterns optimizados
- **Estado Distribuido**: Gestión consistente de estado

### Métricas de Éxito
- **Latencia**: < 50ms para coordinación entre componentes
- **Throughput**: 10,000+ jobs/minuto
- **Escalabilidad**: Soporte para 1,000+ workers concurrentes
- **Disponibilidad**: 99.9% uptime

---

## 🏗️ Arquitectura Hexagonal Propuesta

### Estructura de Crates (Bounded Context: Orchestration)

```
crates/orchestration/
├── orchestrator/                    # Orquestador principal
│   ├── src/
│   │   ├── domain/                  # Entities, Value Objects, Domain Services
│   │   │   ├── job_entity.rs        # Job con estados y transiciones
│   │   │   ├── workflow_entity.rs   # Workflow composition
│   │   │   ├── job_state.rs         # Estados y validaciones
│   │   │   └── workflow_state.rs    # Estados de workflows
│   │   ├── application/             # Use Cases y Application Services
│   │   │   ├── job_use_cases.rs     # Business logic para jobs
│   │   │   ├── workflow_use_cases.rs # Business logic para workflows
│   │   │   └── dtos.rs              # Data Transfer Objects
│   │   ├── infrastructure/          # Adapters externos
│   │   │   ├── nats_adapter.rs      # NATS JetStream implementation
│   │   │   ├── storage_adapter.rs   # Persistent storage
│   │   │   └── event_bus.rs         # Event publishing
│   │   └── lib.rs
│   └── tests/                       # Tests por capa
│       ├── unit/domain/
│       ├── unit/application/
│       └── integration/
│
├── distributed-comm/                # Comunicación distribuida
│   ├── src/
│   │   ├── nats_wrapper.rs          # NATS abstraction layer
│   │   ├── message_types.rs         # Protocol messages
│   │   ├── topic_manager.rs         # Topic lifecycle management
│   │   └── error_handling.rs        # Distributed error patterns
│   └── tests/
│
├── concurrency-patterns/            # Patrones de concurrencia
│   ├── src/
│   │   ├── worker_pools.rs          # Tokio worker pools
│   │   ├── rate_limiting.rs         # Rate limiting patterns
│   │   ├── circuit_breaker.rs       # Resilience patterns
│   │   └── graceful_shutdown.rs     # Shutdown coordination
│   └── benchmarks/                  # Performance benchmarks
│
└── shared-types/                    # Tipos compartidos
    ├── src/
    │   ├── job_definitions.rs       # Job types y schemas
    │   ├── worker_messages.rs       # Inter-worker messages
    │   └── health_checks.rs         # Health check protocols
    └── tests/
```

### Diagrama de Arquitectura Hexagonal

```
┌─────────────────────────────────────────────────────────────┐
│                    ORCHESTRATION CONTEXT                     │
├─────────────────────────────────────────────────────────────┤
│  APPLICATION LAYER                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │JobUseCases  │  │WorkflowUC   │  │HealthUC     │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│  DOMAIN LAYER                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │Job          │  │Workflow     │  │JobState     │         │
│  │Entity       │  │Entity       │  │ValueObject  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│  INFRASTRUCTURE LAYER (Adapters)                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │NATS         │  │Storage      │  │EventBus     │         │
│  │Adapter      │  │Adapter      │  │Adapter      │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

---

## 📖 Historias de Usuario

### US-001: Implementar Job Entity con Gestión de Estados

**Formato INVEST**:
- **Independent**: Independiente de otras historias
- **Negotiable**: Definición clara de funcionalidades
- **Valuable**: Core functionality del sistema
- **Estimable**: 8 Story Points
- **Small**: Enfoque en entity básico
- **Testable**: Criterios de aceptación específicos

**Descripción**:
> Como orquestador del sistema, necesito representar jobs con estados bien definidos para coordinar su ejecución de manera confiable.

**Contexto Técnico**:
- **Bounded Context**: Orchestration
- **Arquitectura**: Hexagonal, Domain Layer
- **Referencias**: `docs/core_platform_design.md`, `docs/concurrency_patterns_rust.md`

**Criterios de Aceptación**:
```gherkin
Feature: Job Entity State Management

  Scenario: Job creation with initial state
    Given un job definition válido
    When se crea el job
    Then el job debe tener estado "PENDING"
    And debe tener un ID único generado

  Scenario: Job state transitions
    Given un job en estado "PENDING"
    When se ejecuta transición a "RUNNING"
    Then el estado debe ser "RUNNING"
    And debe registrarse timestamp de inicio
    And no debe ser posible transición desde "RUNNING" a "PENDING"

  Scenario: Invalid state transition
    Given un job en estado "FAILED"
    When se intenta transición a "RUNNING"
    Then debe retornar error DomainError::InvalidStateTransition
    And el estado debe permanecer "FAILED"

  Scenario: Job completion flow
    Given un job en estado "RUNNING"
    When se completa exitosamente
    Then el estado debe ser "SUCCESS"
    And debe registrarse resultado del job
    And debe registrarse timestamp de fin
```

**TDD Implementation Strategy**:
1. **RED**: Escribir test que falle (Job creation sin entity)
2. **GREEN**: Implementar mínima funcionalidad (Job struct básico)
3. **REFACTOR**: Extraer Value Object JobState, agregar validaciones
4. **REPEAT**: Agregar tests para cada estado/transition

**Conventional Commit Template**:
```
feat(orchestration): implementar Job Entity con gestión de estados

- Agregar Job struct con ID único y estado inicial PENDING
- Implementar JobState Value Object con transiciones válidas  
- Agregar Domain Services para validación de estados
- Implementar tests TDD para todos los estados
- Coverage 95% en domain layer

Refs: #US-001, docs/core_platform_design.md
```

**Dependencies**: None (primera historia)
**Definition of Done**:
- [x] Job Entity implementado con estados PENDING, RUNNING, SUCCESS, FAILED
- [x] JobState Value Object con transiciones validadas
- [x] Tests TDD: 95% coverage en domain layer
- [x] Documentation actualizada para domain entities
- [x] Integration con cargo-clippy sin warnings

---

### US-002: Configurar NATS JetStream para Comunicación Distribuida

**Formato INVEST**:
- **Independent**: Puede implementarse independiente de otros componentes
- **Negotiable**: Definición clara de messaging requirements
- **Valuable**: Comunicación base para el sistema distribuido
- **Estimable**: 13 Story Points
- **Small**: Enfoque en setup y configuración inicial
- **Testable**: Tests de integración con NATS

**Descripción**:
> Como sistema distribuido, necesito un sistema de mensajería confiable y eficiente para coordinar componentes y workers de manera asíncrona.

**Contexto Técnico**:
- **Bounded Context**: Orchestration → Infrastructure
- **Arquitectura**: Hexagonal, Infrastructure Layer
- **Referencias**: `docs/distributed_communication_patterns.md`, research NATS documentation

**Criterios de Aceptación**:
```gherkin
Feature: NATS JetStream Messaging

  Scenario: Connection establishment
    Given configuración válida de NATS
    When se inicia la conexión
    Then debe conectarse exitosamente al broker
    And debe crear streams requeridos automáticamente
    And debe ser resiliente a desconexiones

  Scenario: Message publishing
    Given conexión establecida a NATS
    When se publica mensaje en topic
    Then el mensaje debe entregarse al menos una vez
    And debe serializarse correctamente (JSON/Bincode)
    And debe incluir metadata de tracing

  Scenario: Message consumption
    Given mensajes publicados en topic
    When se suscribe componente
    Then debe recibir mensajes con latencia < 50ms
    And debe manejar backpressure apropiadamente
    And debe implementar retry logic para fallos

  Scenario: Stream management
    Given streams configurados en NATS
    When se necesita crear stream adicional
    Then debe verificarse que no existe previamente
    And debe crear con configuración óptima para el caso
    And debe limpiar streams obsoletos automáticamente

  Scenario: Error handling
    Given conexión a NATS interrumpida
    When se intenta enviar mensaje
    Then debe implementar circuit breaker pattern
    And debe hacer retry con backoff exponencial
    And debe emitir métricas de conectividad
```

**TDD Implementation Strategy**:
1. **RED**: Test de conexión a NATS (mock)
2. **GREEN**: Implementar NATS adapter básico
3. **REFACTOR**: Extrair TopicManager, MessageSerializer
4. **TEST**: Integration tests con NATS real (testcontainers)
5. **BENCHMARK**: Latencia y throughput tests

**Conventional Commit Template**:
```
feat(orchestration): configurar NATS JetStream para comunicación distribuida

- Implementar NATSAdapter con connection management
- Agregar TopicManager para lifecycle de topics  
- Implementar MessageSerializer con JSON/Bincode
- Configurar streams automáticos para system topics
- Integration tests con testcontainers NATS
- Benchmarks: <50ms latencia, >10K msg/sec

Refs: #US-002, docs/distributed_communication_patterns.md
```

**Dependencies**: US-001 (Job Entity para messaging)
**Definition of Done**:
- [x] NATS JetStream connection con auto-reconnection
- [x] Topic management con streams optimizados
- [x] Message serialization/deserialization
- [x] Integration tests con NATS real
- [x] Performance benchmarks documentados
- [x] Error handling con circuit breaker

---

### US-003: Implementar Distributed Coordinator

**Formato INVEST**:
- **Independent**: Basado en components ya implementados
- **Negotiable**: Coordinación de alto nivel definida
- **Valuable**: Core orchestration logic
- **Estimable**: 21 Story Points  
- **Large**: Requiere múltiples componentes (dividir en subtareas si necesario)
- **Testable**: Tests de coordinación y concurrencia

**Descripción**:
> Como orquestador del sistema, necesito coordinar la ejecución distribuida de jobs de manera eficiente, manejando fallos y optimizando el throughput del sistema.

**Contexto Técnico**:
- **Bounded Context**: Orchestration → Application Layer
- **Arquitectura**: Hexagonal, Use Cases + Domain Services
- **Referencias**: `docs/core_platform_design.md`, `docs/concurrency_patterns_rust.md`

**Criterios de Aceptación**:
```gherkin
Feature: Distributed Job Coordination

  Scenario: Basic job coordination
    Given job en estado PENDING
    When se llama coordinator.schedule_job()
    Then debe validar job definition
    And debe encontrar worker disponible
    And debe enviar job a worker seleccionado
    And debe actualizar estado a RUNNING

  Scenario: Worker failure handling
    Given job RUNNING en worker que falla
    When se detecta worker failure
    Then debe detectar timeout (>30s sin heartbeat)
    And debe re-schedule job en worker alternativo
    And debe registrar failure en audit log
    And job debe continuar ejecutándose

  Scenario: Load balancing
    Given múltiples workers disponibles
    When se schedulea nuevo job
    Then debe seleccionar worker con menor load
    And debe considerar worker capabilities
    And debe distribuir jobs uniformemente
    And debe actualizar métricas de load

  Scenario: Concurrent job handling
    Given 100+ jobs llegando simultáneamente
    When coordinator los procesa
    Then debe manejar concurrencia sin deadlocks
    And debe mantener throughput > 1000 jobs/min
    And debe usar worker pools para paralelización
    And debe implementar rate limiting

  Scenario: Graceful shutdown
    Given jobs en ejecución
    When se inicia shutdown del coordinator
    Then debe completar jobs en ejecución
    And debe cancelar jobs pendientes
    And debe comunicar shutdown a workers
    And debe persistir estado para recovery
```

**TDD Implementation Strategy**:
1. **RED**: Test coordinator básico con job simple
2. **GREEN**: Implementar JobCoordinator con worker selection
3. **REFACTOR**: Extraer LoadBalancer, FailureDetector
4. **PERF**: Worker pools, rate limiting
5. **RESILIENCE**: Graceful shutdown, state persistence

**Conventional Commit Template**:
```
feat(orchestration): implementar Distributed Coordinator con load balancing

- Implementar JobCoordinator con worker selection logic
- Agregar LoadBalancer con worker capability matching
- Implementar FailureDetector con heartbeat monitoring
- Configurar WorkerPools para concurrencia optimizada
- Agregar graceful shutdown con state persistence
- Performance: >1000 jobs/min, <30s failure detection
- Tests: 95% coverage, load testing con 100+ jobs

Refs: #US-003, docs/core_platform_design.md, docs/concurrency_patterns_rust.md
```

**Dependencies**: US-001, US-002 (Job Entity + NATS)
**Definition of Done**:
- [x] JobCoordinator con worker selection y load balancing
- [x] Failure detection y recovery automático
- [x] Worker pools para concurrencia
- [x] Graceful shutdown con state persistence
- [x] Load testing: >1000 jobs/min throughput
- [x] Integration tests con NATS y workers simulados

---

### US-004: Optimizar Patrones de Concurrencia Tokio

**Formato INVEST**:
- **Independent**: Optimización de performance independiente
- **Negotiable**: Performance targets definidos
- **Valuable**: Performance crítica del sistema
- **Estimable**: 13 Story Points
- **Small**: Enfoque en patterns específicos
- **Testable**: Benchmarks y profiling

**Descripción**:
> Como sistema distribuido de alto rendimiento, necesito optimizar los patrones de concurrencia para maximizar throughput y minimizar latencia.

**Contexto Técnico**:
- **Bounded Context**: Orchestration → Infrastructure
- **Arquitectura**: Performance optimization layer
- **Referencias**: `docs/concurrency_patterns_rust.md`, Tokio performance guides

**Criterios de Aceptación**:
```gherkin
Feature: Tokio Concurrency Optimization

  Scenario: Worker pool efficiency
    Given workload variable (1-1000 concurrent jobs)
    When se usan optimized worker pools
    Then debe mantener CPU utilization > 80%
    And debe minimizar context switching overhead
    And debe optimizar thread pool sizes dinámicamente

  Scenario: Async/await optimization
    Given async operations con I/O intensivo
    When se aplican async/await best practices
    Then debe eliminar unnecessary allocations
    And debe minimizar polling overhead
    And debe optimizar future polling strategies

  Scenario: Memory efficiency
    Given sistema bajo carga alta
    When se analiza memory usage
    Then debe mantener heap usage < 2GB para 1000 workers
    And debe evitar memory leaks
    And debe optimizar allocation patterns

  Scenario: Backpressure handling
    Given burst de jobs (1000+ simultáneos)
    When sistema bajo backpressure
    Then debe implementar queue size limits
    And debe aplicar backpressure a producers
    And debe monitorear queue health

  Scenario: Monitoring integration
    Given sistema en producción
    When se monitorea performance
    Then debe exponer metrics vía Prometheus
    And debe incluir latencia percentiles (p50, p95, p99)
    And debe generar alerts para thresholds
```

**TDD Implementation Strategy**:
1. **RED**: Benchmark test mostrando performance actual
2. **GREEN**: Implementar optimized worker pools
3. **REFACTOR**: Aplicar async/await optimizations
4. **MEASURE**: Profiling y memory analysis
5. **MONITOR**: Metrics y alerting

**Conventional Commit Template**:
```
perf(orchestration): optimizar patrones de concurrencia Tokio

- Implementar DynamicWorkerPool con auto-scaling
- Aplicar async/await best practices para I/O
- Optimizar memory allocation patterns
- Agregar backpressure handling con queues
- Integrar Prometheus metrics para monitoring
- Performance: 80%+ CPU utilization, <2GB heap/1000 workers
- Benchmarks documentados con flamegraphs

Refs: #US-004, docs/concurrency_patterns_rust.md
```

**Dependencies**: US-003 (Coordinator implementado)
**Definition of Done**:
- [x] Worker pools optimizados dinámicamente
- [x] Async/await patterns optimizados
- [x] Backpressure handling implementado
- [x] Memory profiling sin leaks
- [x] Prometheus metrics integradas
- [x] Performance benchmarks con flamegraphs

---

### US-005: Implementar Worker Lifecycle Management

**Formato INVEST**:
- **Independent**: Lifecycle logic independiente
- **Negotiable**: Estados de worker bien definidos
- **Valuable**: Gestión confiable de recursos
- **Estimable**: 13 Story Points
- **Small**: Enfoque en lifecycle states
- **Testable**: Tests de estado y transiciones

**Descripción**:
> Como sistema distribuido, necesito gestionar el lifecycle completo de workers desde registro hasta deregistro, incluyendo health checks y auto-recovery.

**Contexto Técnico**:
- **Bounded Context**: Orchestration → Domain + Application
- **Arquitectura**: Hexagonal, Domain entities + Use cases
- **Referencias**: `docs/core_platform_design.md`

**Criterios de Aceptación**:
```gherkin
Feature: Worker Lifecycle Management

  Scenario: Worker registration
    Given worker intentando registrarse
    When envía registration request
    Then debe validar worker capabilities
    And debe asignar worker ID único
    And debe actualizar estado a AVAILABLE
    And debe iniciar health check monitoring

  Scenario: Health check monitoring
    Given worker en estado AVAILABLE
    When han pasado 30s sin heartbeat
    Then debe marcar worker como UNHEALTHY
    And debe intentar reconnect automáticamente
    And debe generar alert si falla reconnect

  Scenario: Worker capability matching
    Given job requiring específica capabilities
    When coordinator busca worker
    Then debe filtrar por capabilities requeridas
    And debe seleccionar worker AVAILABLE con capabilities
    And debe considerar worker load actual
    And debe registrar capability usage

  Scenario: Worker deregistration
    Given worker envoy deregistration request
    When procesa deregistration
    Then debe completar jobs en ejecución
    And debe marcar worker como DRAINING
    And debe re-schedule jobs pendientes
    And debe cleanup resources asociados

  Scenario: Auto-recovery
    Given worker marcado como UNHEALTHY
    When se detecta recovery
    Then debe validar worker health
    And debe actualizar estado a AVAILABLE
    And debe reintegrar a worker pool
    And debe clear history de failures

  Scenario: Load balancing con capabilities
    Given jobs con diferentes capabilities
    When coordinador balancea carga
    Then debe maximizar utilization por capability type
    And debe evitar overloading workers específicos
    And debe distribuir evenly entre qualified workers
```

**TDD Implementation Strategy**:
1. **RED**: Test worker registration básico
2. **GREEN**: Implementar Worker entity con estados
3. **REFACTOR**: Extraer WorkerManager, HealthMonitor
4. **ENHANCE**: Capability matching, auto-recovery
5. **INTEGRATE**: Con coordinator scheduling

**Conventional Commit Template**:
```
feat(orchestration): implementar Worker Lifecycle Management con health checks

- Agregar Worker Entity con estados: AVAILABLE, RUNNING, UNHEALTHY, DRAINING
- Implementar WorkerManager para registration/deregistration
- Agregar HealthMonitor con heartbeat-based detection
- Implementar capability matching para job scheduling
- Auto-recovery logic para workers unhealthy
- Tests: state transitions, capability matching, health checks
- Performance: <30s failure detection, automatic recovery

Refs: #US-005, docs/core_platform_design.md
```

**Dependencies**: US-003 (Coordinator para integration)
**Definition of Done**:
- [x] Worker Entity con lifecycle states completos
- [x] WorkerManager con registration/deregistration
- [x] Health monitoring con heartbeat detection
- [x] Capability matching para job scheduling
- [x] Auto-recovery logic implementado
- [x] Integration tests con coordinator

---

### US-006: Configurar Performance Benchmarking y Monitoring

**Formato INVEST**:
- **Independent**: Monitoring independiente de core logic
- **Negotiable**: Métricas y thresholds definidos
- **Valuable**: Observability crítica para production
- **Estimable**: 8 Story Points
- **Small**: Enfoque en metrics y dashboards
- **Testable**: Benchmarks automatizados

**Descripción**:
> Como equipo de desarrollo, necesito métricas detalladas de performance y health del sistema para optimizar continuamente y detectar issues proactivamente.

**Contexto Técnico**:
- **Bounded Context**: Orchestration → Observability
- **Arquitectura**: Metrics collection layer
- **Referencias**: Prometheus/Grafana best practices, observability research

**Criterios de Aceptación**:
```gherkin
Feature: Performance Benchmarking and Monitoring

  Scenario: Key metrics collection
    Given sistema ejecutándose
    When se monitorean metrics
    Then debe collectionar: job throughput, latency percentiles, worker utilization
    And debe exposed en formato Prometheus
    And debe incluir custom labels para filtering
    And debe ser thread-safe para concurrent access

  Scenario: Automated benchmarking
    Given benchmark suite configurado
    When se ejecuta automáticamente (cada release)
    Then debe medir: throughput, latency, memory usage, CPU utilization
    And debe generar reportes comparativos
    And debe alertar sobre performance regressions
    And debe integrar con CI/CD pipeline

  Scenario: Health dashboard
    Given Grafana dashboard configurado
    When se visualiza system health
    Then debe mostrar: system overview, per-component metrics, alerts
    And debe incluir real-time metrics
    And debe tener alerting integration
    And debe ser accesible para DevOps team

  Scenario: Performance regression detection
    Given baseline performance metrics
    When nueva versión deployed
    Then debe detectar regression automáticamente
    And debe comparar vs baseline
    And debe generar alert si regression > 5%
    And debe proporcionar profiling data

  Scenario: SLA monitoring
    Given SLAs definidos (99.9% uptime, <30s latency)
    When se monitorea compliance
    Then debe trackear uptime percentage
    And debe medir latency percentiles vs SLA
    And debe generar reports de compliance
    And debe alertar inmediatamente de violations

  Scenario: Load testing integration
    Given load testing framework configurado
    When se ejecuta load tests
    Then debe generar load patterns realistas
    And debe monitorear system behavior under load
    And debe auto-scale resources based on load
    And debe reportar capacity recommendations
```

**TDD Implementation Strategy**:
1. **RED**: Test metrics collection básico
2. **GREEN**: Implementar Prometheus metrics exporter
3. **REFACTOR**: Extrair MetricsCollector, BenchmarkSuite
4. **DASHBOARD**: Grafana dashboard automation
5. **ALERTS**: Alerting rules y notifications

**Conventional Commit Template**:
```
perf(orchestration): configurar performance benchmarking y monitoring

- Implementar Prometheus metrics exporter con custom metrics
- Agregar BenchmarkSuite automatizado para regression testing
- Configurar Grafana dashboard con system health overview
- Implementar SLA monitoring con alerting rules
- Auto-load testing con capacity recommendations
- CI/CD integration para performance gates
- Metrics: throughput, latency p50/p95/p99, worker utilization

Refs: #US-006, docs/scheduling_research/resource_monitoring_systems.md
```

**Dependencies**: US-003, US-004, US-005 (Core components para metrics)
**Definition of Done**:
- [x] Prometheus metrics exporter con key performance metrics
- [x] Automated benchmark suite con regression detection
- [x] Grafana dashboard con real-time monitoring
- [x] SLA monitoring con alerting automático
- [x] Load testing integration
- [x] CI/CD performance gates

---

## 📅 Planificación de Sprints

### Sprint 1 (2 semanas): US-001 Job Entity
**Objetivo**: Establecer foundation con Job entity
**Capacidad**: 8 SP
**Deliverables**:
- Job Entity con estados básicos
- TDD tests con 95% coverage
- Domain services para validación
- Documentation actualizada

**Detailed Plan**:
- Día 1-2: TDD red/green para Job entity
- Día 3-4: JobState Value Object y transiciones  
- Día 5-7: Domain services y validation logic
- Día 8-10: Tests integration y refinement
- Día 11-14: Documentation y DoD verification

### Sprint 2 (2 semanas): US-002 NATS Integration
**Objetivo**: Communication backbone
**Capacidad**: 13 SP (con buffer)
**Deliverables**:
- NATS JetStream adapter funcional
- Topic management automático
- Integration tests con testcontainers
- Performance benchmarks

**Detailed Plan**:
- Día 1-3: NATS adapter básico y connection management
- Día 4-5: TopicManager y stream configuration
- Día 6-8: Message serialization y error handling
- Día 9-11: Integration tests con NATS real
- Día 12-14: Performance benchmarking y optimization

### Sprint 3-4 (4 semanas): US-003 Distributed Coordinator
**Objetivo**: Core orchestration logic
**Capacidad**: 21 SP (2 sprints)
**Deliverables**:
- JobCoordinator funcional
- Load balancing implementation
- Failure detection y recovery
- Performance optimization

**Sprint 3 Plan**:
- Día 1-4: JobCoordinator básico y worker selection
- Día 5-8: LoadBalancer y capability matching
- Día 9-10: Integration tests básicos

**Sprint 4 Plan**:
- Día 1-4: Failure detection y auto-recovery
- Día 5-7: Worker pools y concurrency optimization
- Día 8-10: Load testing y performance tuning
- Día 11-14: Integration con NATS y comprehensive testing

### Sprint 5 (2 semanas): US-004 Concurrency Optimization
**Objetivo**: Performance tuning
**Capacidad**: 13 SP
**Deliverables**:
- Optimized Tokio patterns
- Backpressure handling
- Memory profiling
- Performance metrics

### Sprint 6 (2 semanas): US-005 Worker Lifecycle
**Objetivo**: Complete worker management
**Capacidad**: 13 SP  
**Deliverables**:
- Worker lifecycle completo
- Health monitoring
- Auto-recovery
- Integration testing

### Sprint 7 (2 semanas): US-006 Monitoring & Benchmarking
**Objetivo**: Observability foundation
**Capacity**: 8 SP
**Deliverables**:
- Prometheus metrics
- Grafana dashboard
- Automated benchmarking
- Alerting rules

**Total Timeline**: 7 sprints (14 semanas, Q1 2024)

---

## ✅ Criterios de Aceptación Detallados

### Definition of Done por Historia

**US-001 DoD**:
- [ ] Job Entity con estados: PENDING, RUNNING, SUCCESS, FAILED
- [ ] JobState Value Object con transiciones validadas
- [ ] Domain Services para business logic
- [ ] TDD tests con 95% coverage mínimo
- [ ] cargo-clippy sin warnings
- [ ] Documentation actualizada
- [ ] Integration con workspace dependencies

**US-002 DoD**:
- [ ] NATS JetStream connection con auto-reconnection
- [ ] TopicManager para stream lifecycle
- [ ] Message serialization (JSON/Bincode)
- [ ] Integration tests con NATS real (testcontainers)
- [ ] Performance: <50ms latencia, >10K msg/sec
- [ ] Error handling con circuit breaker
- [ ] Metrics de connectivity

**US-003 DoD**:
- [ ] JobCoordinator con worker selection logic
- [ ] LoadBalancer con capability matching
- [ ] FailureDetector con heartbeat monitoring
- [ ] WorkerPools para concurrency
- [ ] Graceful shutdown con state persistence
- [ ] Load testing: >1000 jobs/min throughput
- [ ] Integration tests con NATS y workers

### Testing Coverage Requirements

| Layer | Coverage Target | Test Types |
|-------|----------------|------------|
| Domain | 95% | Unit tests con mocking |
| Application | 90% | Use case tests con stubs |
| Infrastructure | 85% | Integration tests |
| End-to-End | 80% | Contract tests |

### Performance SLAs

| Metric | Target | Measurement |
|--------|--------|-------------|
| Job Scheduling Latency | < 50ms | p95 percentile |
| Worker Assignment | < 30ms | p95 percentile |
| System Throughput | > 1000 jobs/min | Sustained load |
| Failure Detection | < 30s | Heartbeat timeout |
| Memory Usage | < 2GB/1000 workers | Peak usage |

---

## 🧪 Testing Strategy TDD

### Estructura de Tests por Layer

```
orchestrator/tests/
├── unit/
│   ├── domain/
│   │   ├── test_job_entity.rs           # Estado tests, transiciones
│   │   ├── test_job_state.rs            # Value object validation
│   │   └── test_workflow_entity.rs      # Workflow logic
│   ├── application/
│   │   ├── test_job_use_cases.rs        # Business logic tests
│   │   ├── test_workflow_use_cases.rs   # Composition tests
│   │   └── test_coordinator.rs          # Integration logic
│   └── infrastructure/
│       ├── test_nats_adapter.rs         # External service mocks
│       ├── test_storage_adapter.rs      # Database mocks
│       └── test_event_bus.rs            # Event publishing
├── integration/
│   ├── test_job_lifecycle.rs            # End-to-end job flow
│   ├── test_coordinator_integration.rs  # Full coordination
│   └── test_performance.rs              # Load testing
└── contract/
    ├── test_nats_contracts.rs           # Contract testing
    └── test_worker_protocol.rs          # Protocol validation
```

### TDD Cycle Implementation

#### 1. Red Phase - Failing Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_job_creation() {
        // Arrange
        let job_definition = JobDefinition::new("test-job", vec![]);
        
        // Act & Assert
        let result = Job::create(job_definition);
        
        assert!(matches!(result, Err(DomainError::InvalidJobDefinition)));
    }
}
```

#### 2. Green Phase - Minimal Implementation
```rust
pub struct Job {
    id: JobId,
    definition: JobDefinition,
    state: JobState,
    created_at: DateTime<Utc>,
}

impl Job {
    pub fn create(definition: JobDefinition) -> Result<Self, DomainError> {
        // Minimal implementation para hacer pasar el test
        if definition.name.is_empty() {
            return Err(DomainError::InvalidJobDefinition);
        }
        
        Ok(Self {
            id: JobId::new(),
            definition,
            state: JobState::Pending,
            created_at: Utc::now(),
        })
    }
}
```

#### 3. Refactor Phase - Clean Code
```rust
// Extraer JobState como Value Object
pub struct JobState(String);

impl JobState {
    pub const PENDING: &'static str = "PENDING";
    pub const RUNNING: &'static str = "RUNNING";
    pub const SUCCESS: &'static str = "SUCCESS";
    pub const FAILED: &'static str = "FAILED";
    
    pub fn new(state: String) -> Result<Self, DomainError> {
        match state.as_str() {
            Self::PENDING | Self::RUNNING | Self::SUCCESS | Self::FAILED => 
                Ok(Self(state)),
            _ => Err(DomainError::InvalidStateTransition),
        }
    }
    
    pub fn can_transition_to(&self, target: &Self) -> bool {
        matches!(
            (self.0.as_str(), target.0.as_str()),
            (Self::PENDING, Self::RUNNING) |
            (Self::RUNNING, Self::SUCCESS) |
            (Self::RUNNING, Self::FAILED)
        )
    }
}
```

### Mocking Strategy

#### Infrastructure Layer Mocks
```rust
// Mock NATS para tests de orquestador
struct MockNatsAdapter {
    published_messages: Arc<Mutex<Vec<Message>>>,
    connection_status: Arc<AtomicBool>,
}

impl NatsAdapter for MockNatsAdapter {
    async fn publish(&self, topic: &str, message: &Message) 
        -> Result<(), NatsError> {
        
        // Test-specific behavior
        self.published_messages
            .lock()
            .unwrap()
            .push(message.clone());
            
        // Simulate connection issues en tests específicos
        if self.connection_status.load(Ordering::SeqCst) {
            return Err(NatsError::ConnectionLost);
        }
        
        Ok(())
    }
}
```

---

## 📝 Conventional Commits

### Template Estándar

```bash
# Formato: <tipo>(<contexto>): <descripción>

# Ejemplo para nueva funcionalidad
git commit -m "feat(orchestration): implementar Job Entity con gestión de estados

- Agregar Job struct con ID único y estado inicial PENDING
- Implementar JobState Value Object con transiciones válidas  
- Agregar Domain Services para validación de estados
- Implementar tests TDD para todos los estados
- Coverage 95% en domain layer

Refs: #US-001, docs/core_platform_design.md"

# Ejemplo para performance optimization
git commit -m "perf(orchestration): optimizar worker pools con auto-scaling

- Implementar DynamicWorkerPool con capacity-based scaling
- Aplicar backpressure handling para burst protection
- Optimizar async/await patterns para I/O operations
- Agregar metrics collection para pool health
- Performance: 80%+ CPU utilization, <50ms scheduling

Refs: #US-004, docs/concurrency_patterns_rust.md"

# Ejemplo para bugfix
git commit -m "fix(orchestration): resolver race condition en worker assignment

- Worker assignment concurrent safety con Arc<Mutex>
- Agregar deterministic testing para race conditions
- Implementar worker lock timeout para deadlocks
- Tests: 100% reproduction de race condition issue
- Performance: no degradation observed

Refs: #BUG-123, integration-testing-results.md"

# Ejemplo para refactoring
git commit -m "refactor(orchestration): extraer WorkerManager del JobCoordinator

- Extraer WorkerManager como servicio independiente
- Aplicar SRP: coordinator solo coordina, manager solo gestiona workers
- Cleanup: remove worker logic del coordinator domain
- No functionality changes, pure refactoring
- Tests: verify existing functionality intact

Refs: #US-003, architecture-review-2025-11-21.md"
```

### Commit Types y Contexts

#### Tipos de Commit
- `feat`: Nueva funcionalidad (user story)
- `fix`: Bugfix 
- `perf`: Performance optimization
- `refactor`: Code refactoring sin cambios funcionales
- `test`: Agregar/modificar tests
- `docs`: Documentation
- `chore`: Build, config, dependencies
- `style`: Code formatting sin lógica
- `ci`: CI/CD configuration
- `build`: Build system changes

#### Contexts por Bounded Context
- `orchestration`: Core platform y coordinator
- `scheduling`: Intelligent scheduling features
- `security`: Security y compliance
- `workers`: Worker management abstraction
- `observability`: Monitoring y metrics
- `devtools`: Developer experience

### Versionado y Changelog

#### Semantic Versioning
- **Major** (X.0.0): Breaking API changes
- **Minor** (0.Y.0): New functionality, backward compatible
- **Patch** (0.0.Z): Bugfixes, backward compatible

#### CHANGELOG.md Structure
```markdown
# Changelog

## [1.0.0] - 2025-11-21

### Added
- feat(orchestration): implementar Job Entity con gestión de estados (#US-001)
- feat(orchestration): configurar NATS JetStream para comunicación distribuida (#US-002)

### Changed  
- refactor(orchestration): extraer JobState como Value Object
- perf(orchestration): optimizar worker pools con auto-scaling (#US-004)

### Fixed
- fix(orchestration): resolver race condition en worker assignment (#BUG-123)

### Removed
- chore(orchestration): remove deprecated JobManager API
```

---

## 🔗 Referencias Técnicas

### Documentos de Arquitectura Base
- `docs/README_arquitectura_cicd_distribuida.md` - Índice principal del proyecto
- `docs/core_platform_design.md` - Stage 1: Diseño Core Platform  
- `docs/distributed_communication_patterns.md` - Stage 4: Comunicación Distribuida
- `docs/concurrency_patterns_rust.md` - Stage 5: Patrones Concurrencia

### Investigación de Monitoreo
- `docs/scheduling_research/resource_monitoring_systems.md` - Prometheus/Grafana integration
- `docs/scheduling_research/distributed_job_scheduling_patterns.md` - Coordination patterns

### Herramientas y Frameworks
- **Tokio**: Async runtime para concurrencia
- **NATS JetStream**: Messaging distribuido  
- **Prometheus**: Metrics collection
- **Grafana**: Monitoring dashboards
- **cargo-geiger**: Dependency analysis
- **flamegraph**: Performance profiling

### Próximas Épicas Dependientes
- **Épica 2**: Intelligent Scheduler & AI (depends on Core Platform)
- **Épica 3**: Security & Compliance (integrates with orchestration)
- **Épica 4**: Worker Management Abstraction (builds on lifecycle)

---

**Fin Épica 1: Core Platform & Infrastructure**  
**Próximo**: `02_epica_intelligent_scheduler_ai.md`  
**Critical Path**: Core Platform → Intelligent Scheduling → Full System
