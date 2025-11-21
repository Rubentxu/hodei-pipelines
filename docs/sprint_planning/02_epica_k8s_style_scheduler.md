# Épica 2: Kubernetes-Style Scheduler & Resource Management

**Planificación de Sprints - Sistema CI/CD Distribuido**  
**Bounded Context**: Intelligent Scheduling  
**Autor**: MiniMax Agent  
**Fecha**: 2025-11-21  
**Versión**: 2.0  
**Cambio**: Reemplazado AI/ML-based scheduling por Kubernetes-inspired scheduler

---

## 📋 Índice
1. [Visión de la Épica](#visión-de-la-épica)
2. [Arquitectura de Scheduler](#arquitectura-de-scheduler)
3. [Patrones de Scheduling](#patrones-de-scheduling)
4. [Historias de Usuario](#historias-de-usuario)
5. [Planificación de Sprints](#planificación-de-sprints)
6. [Scheduling Framework](#scheduling-framework)
7. [Performance Expectations](#performance-expectations)
8. [Referencias Técnicas](#referencias-técnicas)

---

## 🎯 Visión de la Épica

### Objetivo Principal
Desarrollar un sistema de scheduling robusto y predecible inspirado en Kubernetes que proporcione asignación óptima de jobs a workers utilizando algoritmos probados, criterios de selección configurables y estrategias de scheduling determinísticas.

### Componentes del Scheduler
- **Framework de Scheduling**: Pipeline de 4 fases (Informer → Filter → Score → Bind)
- **Gestión de Prioridades**: Preemption y priority queues
- **Selección de Workers**: Múltiples algoritmos (Least Loaded, Resource Balance, Bin Packing)
- **Reglas de Afinidad**: Node affinity, inter-job affinity y anti-affinity
- **Taints & Tolerations**: Dedicación de workers a tipos específicos de jobs
- **Gestión de Colas**: FIFO, Priority, Fair Queuing
- **Múltiples Schedulers**: Soporte para schedulers especializados

### Métricas de Éxito Cuantificables
- **Scheduling Latency**: < 100ms para scheduling decisions
- **Queue Wait Time**: < 2s promedio para jobs de prioridad media
- **Preemption Success**: 100% éxito en preemption de low-priority jobs
- **Worker Utilization**: 85%+ utilization con balance óptimo
- **Scheduling Success Rate**: > 99.5% de jobs scheduled exitosamente

---

## 🏗️ Arquitectura de Scheduler

### Estructura de Crates (Bounded Context: Intelligent Scheduling)

```
crates/intelligent-scheduling/
├── scheduler-framework/              # Core Scheduler
│   ├── src/
│   │   ├── pipeline.rs               # Scheduler pipeline (Informer->Filter->Score->Bind)
│   │   ├── informer.rs               # Job queue watching
│   │   ├── filter/                   # Filtering plugins
│   │   │   ├── resource_filter.rs    # Resource availability
│   │   │   ├── affinity_filter.rs    # Node affinity rules
│   │   │   └── taint_filter.rs       # Taints & tolerations
│   │   ├── scoring/                  # Scoring plugins
│   │   │   ├── resource_scorer.rs    # Resource balance scoring
│   │   │   ├── load_scorer.rs        # Load balancing score
│   │   │   └── affinity_scorer.rs    # Affinity preference score
│   │   ├── binder.rs                 # Job binding to worker
│   │   ├── plugin.rs                 # Plugin framework
│   │   └── error.rs                  # Scheduler errors
│   └── tests/
│       ├── unit/filter_tests.rs
│       ├── unit/scoring_tests.rs
│       └── integration/scheduler_pipeline_tests.rs
│
├── scheduling-strategies/            # Scheduling Algorithms
│   ├── src/
│   │   ├── queue_manager.rs          # Job queue management
│   │   │   ├── priority_queue.rs     # Priority queue with preemption
│   │   │   ├── fifo_queue.rs         # Simple FIFO queue
│   │   │   └── fair_queue.rs         # Fair queuing by tenant
│   │   ├── worker_selection/         # Worker selection algorithms
│   │   │   ├── least_loaded.rs       # Select least loaded worker
│   │   │   ├── resource_balance.rs   # Balance resources across workers
│   │   │   ├── bin_packing.rs        # Bin packing algorithm
│   │   │   └── round_robin.rs        # Round-robin distribution
│   │   ├── preemption.rs             # Preemption logic
│   │   └── backoff.rs                # Scheduling backoff strategies
│   └── tests/
│       ├── queue_tests.rs
│       ├── worker_selection_tests.rs
│       └── preemption_tests.rs
│
├── scheduling-policies/              # Scheduling Policies
│   ├── src/
│   │   ├── priority.rs               # Priority definitions
│   │   ├── resource_quota.rs         # Resource quotas per tenant
│   │   ├── limit_ranges.rs           # Job resource limits
│   │   ├── affinity/                 # Affinity rules
│   │   │   ├── node_affinity.rs      # Node affinity policies
│   │   │   └── pod_affinity.rs       # Inter-job affinity
│   │   └── taints/                   # Taints & tolerations
│   │       ├── taint.rs              # Taint definitions
│   │       └── toleration.rs         # Toleration matching
│   └── tests/
│       ├── priority_tests.rs
│       └── affinity_tests.rs
│
└── scheduler-api/                    # Scheduler Interface
    ├── src/
    │   ├── scheduler.rs              # Scheduler trait/interface
    │   ├── job_scheduling_request.rs # Job scheduling request
    │   ├── worker_info.rs            # Worker information
    │   ├── scheduling_result.rs      # Scheduling outcome
    │   ├── config.rs                 # Scheduler configuration
    │   └── multiple_schedulers.rs    # Multiple scheduler support
    └── tests/
        ├── interface_tests.rs
        └── config_tests.rs
```

### Diagrama de Arquitectura del Scheduler

```
┌─────────────────────────────────────────────────────────────┐
│                   SCHEDULING PIPELINE                         │
├─────────────────────────────────────────────────────────────┤
│  1. INFORMER (Job Discovery)                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Watch Job Queue                                         │ │
│  │ Filter Pending Jobs                                     │ │
│  │ Extract Scheduling Requirements                         │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│  2. FILTER (Feasibility Check)                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ ✓ Resource Availability Check                          │ │
│  │ ✓ Node Affinity & Selector Match                       │ │
│  │ ✓ Taints & Tolerations Match                           │ │
│  │ ✓ Quota & Limit Verification                           │ │
│  └─────────────────────────────────────────────────────────┘ │
│                 ↓ Feasible Workers ↓                        │
├─────────────────────────────────────────────────────────────┤
│  3. SCORE (Ranking)                                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Score: Resource Balance (40%)                          │ │
│  │ Score: Load Distribution (30%)                         │ │
│  │ Score: Affinity Preferences (20%)                      │ │
│  │ Score: Network Proximity (10%)                         │ │
│  └─────────────────────────────────────────────────────────┘ │
│                      ↓ Best Worker ↓                        │
├─────────────────────────────────────────────────────────────┤
│  4. BIND (Assignment)                                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Reserve Worker for Job                                  │ │
│  │ Update Job State to Scheduled                           │ │
│  │ Notify Worker Manager                                   │ │
│  │ Update Scheduler Cache                                  │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## 🎯 Patrones de Scheduling

### Priority and Preemption

Los jobs tienen prioridades y pueden preemptar jobs de menor prioridad:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobPriority {
    Critical,  // System critical (emergency, security)
    High,      // Production deployments
    Medium,    // Regular CI/CD
    Low,       // Background jobs
    Batch,     // Batch processing
}

#[derive(Debug, Clone)]
pub struct PreemptionPolicy {
    pub enabled: bool,
    pub max_preemptions: u32,
    pub grace_period: Duration,
}
```

### Queue Strategies

Diferentes estrategias de cola para diferentes casos de uso:

```rust
#[derive(Debug, Clone)]
pub enum QueueStrategy {
    Fifo,  // First In, First Out
    Priority {
        with_preemption: bool,
        max_queue_time: Duration,
    },
    Fair {
        tenant_key: String,
        weights: HashMap<String, u32>,
        quantum: Duration,
    },
}
```

### Worker Selection Algorithms

```rust
#[derive(Debug, Clone)]
pub enum WorkerSelectionAlgorithm {
    LeastLoaded,         // Minimize current load
    MostFree,            // Maximize free resources
    ResourceBalance,     // Balance cluster-wide resources
    BinPacking,          // Pack efficiently (First Fit Decreasing)
    RoundRobin,          // Distribute evenly
    LocalityAware,       // Prefer local workers
    Custom(String),      // Plugin-based custom algorithm
}
```

### Affinity Rules

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAffinity {
    pub required: Vec<LabelSelector>,      // Hard constraints (must match)
    pub preferred: Vec<WeightedSelector>,  // Soft constraints (nice to have)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PodAffinityTerm {
    InSameRegion(Vec<LabelSelector>),           // Co-locate
    InDifferentRegions(Vec<LabelSelector>),     // Spread out
    InNRegions(Vec<LabelSelector>, u32),        // Spread across N regions
}
```

---

## 📖 Historias de Usuario

### ✅ US-007: Implementar Scheduler Framework Core (COMPLETADO)

**Formato INVEST**:
- **Independent**: Framework base independiente
- **Negotiable**: APIs bien definidas
- **Valuable**: Core functionality del scheduler
- **Estimable**: 13 Story Points
- **Large**: Múltiples componentes interrelacionados
- **Testable**: Unit tests y integration tests

**Descripción**:
> Como scheduler del sistema, necesito un framework de scheduling con pipeline de 4 fases (Informer, Filter, Score, Bind) para coordinar la asignación de jobs a workers de manera eficiente y predecible.

**✅ Definition of Done**:
- [x] Scheduler framework con 4-phase pipeline (Informer → Filter → Score → Bind)
- [x] Backend abstraction layer multi-backend (Kubernetes, Docker, Cloud VMs)
- [x] Priority queue con preemption support
- [x] Worker selection algorithms (scoring-based)
- [x] Affinity rules y taints/tolerations system
- [x] Error handling y metrics collection
- [x] 19/21 tests passing, arquitectura Kubernetes-proven

**Criterios de Aceptación**:
```gherkin
Feature: Scheduler Framework Pipeline

  Scenario: Job scheduling through complete pipeline
    Given un job en cola con requisitos de recursos
    When el scheduler procesa el job
    Then debe ejecutar las 4 fases: Informer → Filter → Score → Bind
    And debe asignar el job al worker más adecuado

  Scenario: Filter phase eliminates infeasible workers
    Given un job que requiere GPU
    And workers que no tienen GPU
    When el scheduler ejecuta la fase Filter
    Then debe eliminar workers sin GPU de la候选

  Scenario: Score phase ranks feasible workers
    Given 5 workers feasible para un job
    When el scheduler ejecuta la fase Score
    Then debe asignar scores basados en criterios configurables
    And debe seleccionar el worker con mayor score

  Scenario: Bind phase assigns job to worker
    Given un worker seleccionado
    When el scheduler ejecuta la fase Bind
    Then debe actualizar el estado del job a "SCHEDULED"
    And debe notificar al worker manager
```

---

### US-008: Implementar Priority Queues y Preemption

**Formato INVEST**:
- **Independent**: Queue management independiente
- **Negotiable**: Queue strategies configurables
- **Valuable**: Control de scheduling por prioridad
- **Estimable**: 8 Story Points
- **Small**: Enfoque específico en queues
- **Testable**: Queue behavior tests específicos

**Descripción**:
> Como scheduler, necesito un sistema de colas con prioridades que permita preemptar jobs de baja prioridad para jobs críticos, manteniendo fairness entre tenants.

**Criterios de Aceptación**:
```gherkin
Feature: Priority Queue with Preemption

  Scenario: High priority job preempts low priority
    Given un job High-priority en cola
    And un job Low-priority ejecutándose
    When el high-priority job está en scheduling
    Then debe preemptar el low-priority job
    And debe mover el preemptado a pending state

  Scenario: Priority queue ordering
    Given multiple jobs con diferentes prioridades: [Low, High, Medium, Critical]
    When el scheduler selecciona el siguiente job
    Then debe seleccionar en orden: Critical → High → Medium → Low

  Scenario: Fair queuing across tenants
    Given multiple tenants con jobs en cola
    When el fair queue está habilitado
    Then debe round-robin entre tenants balanceados
    And debe respetar los weights por tenant
```

---

### US-009: Implementar Worker Selection Algorithms

**Formato INVEST**:
- **Independent**: Algoritmos independientes
- **Negotiable**: Selección de algoritmos configurable
- **Valuable**: Optimización de resource utilization
- **Estimable**: 8 Story Points
- **Small**: Implementación de algoritmos específicos
- **Testable**: Algoritmo-specific tests

**Descripción**:
> Como scheduler, necesito múltiples algoritmos de selección de workers (Least Loaded, Resource Balance, Bin Packing) para optimizar la asignación según el contexto del cluster.

**Criterios de Aceptación**:
```gherkin
Feature: Worker Selection Algorithms

  Scenario: Least Loaded algorithm
    Given 3 workers con loads: [80%, 40%, 60%]
    When se selecciona worker para nuevo job
    Then debe seleccionar el worker con 40% load

  Scenario: Resource Balance algorithm
    Given cluster con workers having different CPU/Memory ratios
    When se aplica Resource Balance
    Then debe seleccionar worker que mejor balance el cluster

  Scenario: Bin Packing algorithm
    Given jobs con tamaños diferentes
    When se aplica Bin Packing (First Fit Decreasing)
    Then debe packear jobs eficientemente minimizando fragmentation
```

---

### US-010: Implementar Affinity Rules y Taints/Tolerations

**Formato INVEST**:
- **Independent**: Reglas de scheduling independientes
- **Negotiable**: Configuración flexible
- **Valuable**: Control granular de placement
- **Estimable**: 13 Story Points
- **Large**: Múltiples tipos de reglas
- **Testable**: Affinity matching tests

**Descripción**:
> Como scheduler, necesito soporte para affinity rules (node affinity, inter-job affinity) y taints/tolerations para controlar precisamente dónde se ejecutan los jobs.

**Criterios de Aceptación**:
```gherkin
Feature: Affinity Rules and Taints/Tolerations

  Scenario: Node affinity required constraint
    Given job con node affinity requerida: label "zone" = "us-east-1"
    And workers: 2 en us-east-1, 1 en us-west-2
    When el scheduler hace filter
    Then debe eliminar worker en us-west-2
    And solo considerar workers en us-east-1

  Scenario: Taints and tolerations matching
    Given worker con taint: key="dedicated", value="gpu", effect="NoSchedule"
    And job con toleration matching la taint
    When el scheduler ejecuta filter
    Then debe considerar el worker como feasible

  Scenario: Pod anti-affinity spreading
    Given 3 jobs con same label "app=nginx"
    And pod anti-affinity: "should not be in same region"
    When se schedulean los 3 jobs
    Then deben estar en regions diferentes si están disponibles
```

---

### US-011: Implementar Multiple Schedulers

**Formato INVEST**:
- **Independent**: Múltiples schedulers independientes
- **Negotiable**: Configuración por scheduler
- **Valuable**: Especialización de scheduling
- **Estimable**: 8 Story Points
- **Small**: Framework de múltiples schedulers
- **Testable**: Scheduler coordination tests

**Descripción**:
> Como sistema de scheduling, necesito soporte para múltiples schedulers simultáneos (ej: scheduler general, scheduler de GPU, scheduler de alta prioridad) para especialización y separación de concerns.

**Criterios de Aceptación**:
```gherkin
Feature: Multiple Schedulers Support

  Scenario: Multiple scheduler instances
    Given 2 schedulers configurados: "general" y "gpu"
    When jobs con different requirements llegan
    Then debe rutear GPU jobs al "gpu" scheduler
    And debe rutear jobs generales al "general" scheduler

  Scenario: Scheduler specialization
    Given scheduler "gpu" con algoritmos optimizados para GPU
    And job que requiere GPU
    When el job es scheduleado
    Then debe usar el "gpu" scheduler

  Scenario: Scheduler fallback
    Given scheduler "gpu" no disponible
    When job GPU llega
    Then debe fall back al scheduler "general"
    Or debe marcar como unschedulable
```

---

### US-012: Integrar Scheduler con Worker Lifecycle

**Formato INVEST**:
- **Independent**: Scheduler-Worker integration independiente
- **Negotiable**: Integration points claros
- **Valuable**: End-to-end scheduling workflow
- **Estimable**: 5 Story Points
- **Small**: Integration specific
- **Testable**: End-to-end integration tests

**Descripción**:
> Como scheduler, necesito integrar completamente con el Worker Lifecycle Management para una experiencia de scheduling end-to-end desde job pending hasta completion.

**Criterios de Aceptación**:
```gherkin
Feature: Scheduler-Worker Lifecycle Integration

  Scenario: Job assignment to available worker
    Given job scheduled y worker available
    When el scheduler asigna el job
    Then debe notificar al worker manager
    And debe updatear el job state correctamente

  Scenario: Worker failure during job execution
    Given job ejecutándose en worker
    And worker fails unexpectedly
    When el worker manager detecta failure
    Then debe notificar al scheduler
    And scheduler debe re-queue el job para rescheduling

  Scenario: Worker becomes available during scheduling
    Given scheduler evaluando workers
    And worker está en estado "Terminating"
    When el worker se vuelve "Available"
    Then debe ser incluido en scheduling decisions
```

---

## 📅 Planificación de Sprints

### Sprint 1 (3 semanas): US-007 Scheduler Framework Core
**Objetivo**: Implementar el pipeline básico de scheduling
- Informer para watching de jobs
- Filter framework con plugins básicos
- Score framework con plugins básicos
- Binder para job assignment
- Scheduler pipeline orchestration

### Sprint 2 (2 semanas): US-008 Priority Queues
**Objetivo**: Sistema de colas con prioridades y preemption
- Priority queue implementation
- Preemption logic
- FIFO queue support
- Fair queuing por tenant
- Queue metrics y monitoring

### Sprint 3 (2 semanas): US-009 Worker Selection
**Objetivo**: Algoritmos de selección de workers
- Least Loaded algorithm
- Resource Balance algorithm
- Bin Packing algorithm
- Round Robin algorithm
- Scoring framework integration

### Sprint 4 (3 semanas): US-010 Affinity & Taints
**Objetivo**: Reglas de affinity y taints/tolerations
- Node affinity (required y preferred)
- Pod affinity y anti-affinity
- Taints definition
- Tolerations matching
- Taint-based scheduling

### Sprint 5 (2 semanas): US-011 Multiple Schedulers
**Objetivo**: Soporte para múltiples schedulers
- Scheduler registry
- Job routing by scheduler
- Scheduler isolation
- Configuración por scheduler
- Fallback mechanisms

### Sprint 6 (1 semana): US-012 Worker Integration
**Objetivo**: Integración completa con worker lifecycle
- Job assignment integration
- Failure handling y rescheduling
- State synchronization
- End-to-end testing

---

## 🔧 Scheduling Framework

### Plugin System

El scheduler utiliza un sistema de plugins extensible:

```rust
#[async_trait]
pub trait FilterPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    
    async fn filter(
        &self,
        job: &Job,
        workers: Vec<Worker>,
    ) -> Result<Vec<Worker>, SchedulerError>;
}

#[async_trait]
pub trait ScorePlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn weight(&self) -> f64;
    
    async fn score(
        &self,
        job: &Job,
        worker: &Worker,
    ) -> Result<f64, SchedulerError>;
}
```

### Configuration

```rust
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub name: String,
    pub queue_strategy: QueueStrategy,
    pub worker_selection: WorkerSelectionAlgorithm,
    pub filter_plugins: Vec<String>,
    pub score_plugins: Vec<String>,
    pub preemption_policy: PreemptionPolicy,
    pub scheduling_timeout: Duration,
    pub parallel_scheduling: bool,
}
```

---

## 📊 Performance Expectations

### Latency Targets
- **Job Discovery (Informer)**: < 10ms
- **Filter Phase**: < 20ms (1000 workers)
- **Score Phase**: < 50ms (1000 workers)
- **Bind Phase**: < 10ms
- **Total Scheduling Time**: < 100ms

### Throughput Targets
- **Scheduling Rate**: 10,000+ jobs/minute
- **Concurrent Scheduling**: 100+ jobs simultaneously
- **Queue Throughput**: 50,000+ jobs/hour

### Resource Efficiency
- **Worker Utilization**: 85%+ average
- **Resource Fragmentation**: < 5%
- **Scheduling Success Rate**: > 99.5%

---

## 🔗 Referencias Técnicas

### Documentos de Arquitectura Base
- `docs/k8s-style_scheduler_design.md` - Scheduler design detallado
- `docs/scheduling_research/k8s_scheduler_analysis.md` - Análisis de Kubernetes
- `docs/scheduling_research/scheduling_algorithms.md` - Algoritmos de scheduling

### Investigación de Scheduling
- Kubernetes Scheduler Framework: https://kubernetes.io/docs/concepts/scheduling-eviction/scheduling-framework/
- Kube-Scheduler Source: https://github.com/kubernetes/kubernetes/tree/cmd/kube-scheduler
- Scheduling Profiles: https://kubernetes.io/docs/reference/scheduling/config/

### Herramientas y Frameworks
- Scheduler Simulation: https://github.com/kubernetes-sigs/scheduler-plugins
- Scheduling Benchmarks: https://github.com/kubernetes/perf-tests/tree/master/clusterloader2/scheduling

### Próximas Épicas Dependientes
- Épica 3: Distributed Orchestration & Workflows (depends on scheduler)
- Épica 4: Performance Optimization & Scaling (depends on scheduler metrics)
