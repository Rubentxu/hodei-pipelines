# Arquitectura distribuida CI/CD en Rust: Orquestador, Planificador, Worker Manager y Workers efímeros

## Resumen ejecutivo

Este documento define una arquitectura distribuida para un sistema de Integración y Despliegue Continuos (CI/CD) diseñado desde los principios de Diseño Orientado al Dominio (DDD) y patrones probados de microservicios. El sistema se compone de seis componentes fundamentales: Orquestador (núcleo de coordinación), Planificador (asignación óptima de cargas con visibilidad de recursos en tiempo real), Worker Manager (abstracción multi-provider para aprovisionar y terminar ejecutores), Workers efímeros (unidades aisladas de ejecución en Kubernetes o Docker), y los dominios de apoyo de Telemetría/Trazabilidad y Consola. La arquitectura privilegia el acoplamiento débil, la cohesión funcional por capacidad de negocio, la resiliencia y la escalabilidad horizontal, conforme a las recomendaciones contemporáneas de microservicios y DDD estratégico/táctico[^1][^2][^3].

Las decisiones clave incluyen: un bus de eventos con NATS JetStream para control-plane y telemetría por su baja latencia y garantías de entrega configurables, complementado opcionalmente con Kafka donde el throughput histórico y el replay masivo sean predominantes; el modelo de actores con Tokio/Actix para concurrencia, backpressure y tolerancia a fallos; APIs REST documentadas con OpenAPI para integración con infraestructura y clientes internos; y un diseño “database per service” con Sagas para consistencia distribuida en flujos de trabajos multi-etapa[^4][^5][^2].

Entre los beneficios esperados destacan: elasticidad y resiliencia ante fallos mediante workers efímeros y reintentos idempotentes; observabilidad de punta a punta con trazabilidad correlacionada entre eventos y jobs; y una plataforma preparada para evolución tecnológica (futuros providers de ejecución) a través de la abstracción WorkerManagerProvider. El resultado es una plataforma CI/CD moderna, cloud-native y multi-tenant, con límites de contexto claros y contratos explícitos que minimizan el acoplamiento accidental y maximizan la autonomía de equipos y servicios[^1][^2][^3].

## Principios y objetivos de arquitectura

La arquitectura se gobierna por un conjunto de principios operativos y técnicos:

- Enfoque DDD estratégico y táctico: modelado por capacidades de negocio, definición de bounded contexts, entidades, agregados y objetos de valor; evolución iterativa de límites y contratos[^1][^6][^7].
- Alta cohesión y acoplamiento débil: cada servicio encapsula conocimiento del dominio, expone interfaces explícitas y mantiene su propio almacenamiento (“database per service”). La comunicación asíncrona se prioriza para reducir dependencias temporales[^2][^3].
- Escalabilidad y resiliencia: aislamiento de fallos con patrones como Circuit Breaker y Bulkhead; idempotencia y reintentos con backoff; ejecución en workers efímeros para seguridad y aislamiento de recursos[^2].
- Observabilidad integrada: telemetría, métricas y logs estructurados; propagación de contexto para trazabilidad distribuida; interfaces de streaming en tiempo real para dashboards operativos.
- Seguridad y multi-tenancy: secrets gestionados por el proveedor (Kubernetes/Docker), políticas de acceso por roles y segmentación por namespaces/tenants; controles de red y permisos mínimos necesarios[^12].
- Portabilidad cloud-native: diseño centrado en Kubernetes, con soporte directo a Pods, CRDs y watchers, y una capa de abstracción que permite incorporar otros proveedores sin modificar el núcleo[^5][^12][^8][^9].

Estos principios guían las decisiones de diseño, los contratos entre contextos y la selección de tecnologías y patrones asociados.

## Componentes principales del sistema

La plataforma se descompone en componentes con responsabilidades bien definidas y contratos explícitos. El objetivo es asegurar que cada componente sea cohesivo, esté libremente acoplado y pueda evolucionar sin afectar al resto.

Para contextualizar el sistema, la Tabla 1 sintetiza las responsabilidades, tecnologías y contratos de cada componente.

Antes de detallar cada componente, la siguiente tabla ofrece una vista de conjunto de responsabilidades y contratos principales.

Tabla 1. Mapa de responsabilidades y contratos por componente

| Componente | Responsabilidades principales | Tecnologías/patrones Rust | Contratos (APIs/Eventos) | Notas |
|---|---|---|---|---|
| Orquestador | Coordinator global, ciclo de vida de jobs/pipelines; reconciliación; manejo de estados y Sagas | Actix/Tokio para actors y HTTP; NATS JetStream para eventos | REST: /jobs, /pipelines; Eventos: JobRequested, JobStarted, JobCompleted, JobFailed | Contiene agregados Job y Pipeline; idempotencia y reintentos |
| Planificador | Matching job → worker; visibilidad de recursos; políticas de scheduling; rebalanceo | Actors con Tokio; estructuras lock-free para métricas; backpressure | REST: /schedule, /nodes; Eventos: OfferCreated, AssignmentCreated, CapacityUpdated | Algoritmos FIFO, packing por recursos, affinity; integra con Worker Manager |
| Worker Manager | Abstracción multi-provider (K8s/Docker/futuros); CRUD de ejecutores; observers de estado | kube-rs para K8s; API Docker; traits WorkerManagerProvider | Operaciones: create, terminate, logs, port-forward; Eventos: WorkerCreated, WorkerTerminated, WorkerFailed | Gestión de secretos y RBAC; CRDs y watchers[^5][^12][^10][^8][^9] |
| Workers efímeros | Ejecución aislada de steps; reporting de logs y métricas; cleanup | Containers/Pods; HTTP callbacks; streaming de logs | Endpoints: /exec, /logs; Eventos: StepStarted, StepCompleted, StepFailed | Aislamiento por namespace/pod; límites de recursos; artefactual efímera[^5][^12] |
| Telemetría/Trazabilidad | Métricas de sistema y jobs; trazas distribuidas; streaming a dashboards | Tokio/Actix; NATS; estructuras lock-free para contadores | Streams de métricas; REST: /metrics, /traces | OpenTelemetry para correlación; política de retención configurable |
| Consola (BFF) | UI y APIs agregadas; auth y RBAC; streaming de estado en tiempo real | Axum/Actix; WebSockets; OpenAPI | REST: /ui/*; GraphQL/REST selectivo; SSEREST: /events | BFF por cliente; caché selectiva; caché de metadatos[^14][^13] |

La Tabla 2, por su parte, esquematiza los estados y transiciones del ciclo de vida de un job.

Tabla 2. Estados y transiciones de un job

| Estado | Transiciones salientes | Evento disparador | Acción del Orquestador/Planificador |
|---|---|---|---|
| PENDING | → SCHEDULED | JobRequested | Planificador selecciona nodo; Orquestador crea AssignmentCreated |
| SCHEDULED | → RUNNING | WorkerCreated/AssignmentAccepted | Orquestador marca job en ejecución; inicia step inicial |
| RUNNING | → SUCCEEDED | JobCompleted | Orquestador persiste resultado; publica JobCompleted |
| RUNNING | → FAILED | JobFailed | Orquestador invoca compensación (Saga) si aplica |
| RUNNING | → PENDING (retry) | StepFailed con retry | Orquestador programa reintento con backoff e idempotencia |
| SCHEDULED/RUNNING | → CANCELLED | JobCancelled | Orquestador cancela assignments; Worker Manager termina pods |
| Any | → TERMINATED | WorkerTerminated | Orquestador limpia recursos; reintento o compensación |

### Orquestador

El Orquestador ejerce de “coordinador central” con responsabilidades sobre el ciclo de vida de trabajos y pipelines, la reconciliación de estados y la aplicación de Sagas donde existan transacciones distribuidas. Expone APIs REST para crear y consultar jobs/pipelines, y publica eventos sobre el bus NATS para notificar cambios de estado (por ejemplo, JobRequested, JobStarted, JobCompleted, JobFailed). La idempotencia en comandos y la gestión de reintentos con backoff son fundamentales para la tolerancia a fallos. Agregados como Job y Pipeline residen en este contexto, y sus transiciones deben ser estrictamente auditadas para permitir trazabilidad y recuperabilidad ante errores del sistema[^2].

### Planificador

El Planificador posee visibilidad casi en tiempo real de la capacidad disponible (memoria, CPU, GPU, IO) y las ofertas de nodos (incluidos pools dedicados), y ejecuta políticas de asignación para maximizar throughput y equidad. Al recibir JobRequested, publica OfferCreated; al asignar, emite AssignmentCreated. Estructuras lock-free (por ejemplo, contadores y histográficos) se emplean para métricas de colas y uso, mientras que mecanismos de backpressure regulan la ingestión para evitar saturación del clúster de workers. Las políticas pueden abarcar desde FIFO y packing por recursos hasta afinidades por tipo de executor o disponibilidad de GPU[^2].

### Worker Manager

El Worker Manager presenta una capa de abstracción de infraestructura (WorkerManagerProvider) que encapsula la interacción con Kubernetes y Docker, a la vez que deja abierta la incorporación futura de otros proveedores (por ejemplo, gestores de compute/serverless). En Kubernetes, la integración aprovecha la API y patrones como watchers, CRDs y port-forward para observar estados, crear pods y obtener logs. El componente publica eventos WorkerCreated, WorkerTerminated y WorkerFailed para que el Orquestador y el Planificador ajusten el estado global y replanifiquen en consecuencia[^5][^12][^10][^8][^9].

### Workers efímeros

Los workers son ejecutores de corto生命周期, típicamente pods en Kubernetes, que descargan artefactos, ejecutan pasos de pipeline en aislamiento y reportan logs y métricas. Para facilitar la inspección en caliente, el Worker Manager puede habilitar port-forward o exponer endpoints para streaming de logs. Las políticas de seguridad incluyen restricciones de red, límites de recursos y rotación de secretos. Al finalizar, se aplica limpieza automática para evitar rezagos de recursos. La integración con la API de Kubernetes (logs, watchers) y el ciclo de vida de pods es central al diseño[^5][^12].

### Telemetría y trazabilidad

La telemetría unificada integra métricas operativas, eventos de negocio y trazas distribuidas. Un canal de streaming (NATS) transporta métricas en tiempo real hacia la Consola y sistemas de observabilidad, mientras que OpenTelemetry permite correlacionar spans entre componentes. Se definen puntos de instrumentación en el Orquestador, Planificador y Worker Manager para asegurar visibilidad sobre colas, asignaciones y tiempos de ejecución, facilitando diagnósticos y optimización continua[^4].

### Consola (dashboard y APIs)

La Consola actúa como Backends for Frontends (BFF), exponiendo endpoints específicos para UI, agregando datos de múltiples servicios y realizando streaming de estado (por ejemplo, mediante Server-Sent Events o WebSockets). La seguridad y RBAC protegen endpoints sensibles, y se define una caché selectiva para metadatos de jobs/pipelines con invalidadores por eventos. La especificación de APIs se publica con OpenAPI para facilitar consumo interno y externo coherente[^14][^13].

## Bounded contexts (DDD) y modelo de dominio

El modelado por dominios distingue subdominios funcionales con límites (bounded contexts) explícitos y contratos estables. Cada contexto encapsula su propio modelo (entidades, agregados y objetos de valor) y se relaciona mediante eventos o APIs bien definidas. La Tabla 3 resume los bounded contexts, sus agregados principales y eventos asociados[^1][^6][^7][^2].

Para proporcionar una visión compacta de los límites y modelos, la siguiente tabla sintetiza los contextos delimitados del dominio CI/CD.

Tabla 3. Bounded Contexts y modelos asociados

| Bounded Context | Propósito | Agregados/Entidades/Objetos de valor | Eventos de dominio clave | Integraciones |
|---|---|---|---|---|
| Pipeline Orchestration | Coordinar ciclos de vida de pipelines y jobs | Agregados: Pipeline, Job; Entidades: Step, Assignment; VO: PipelineSpec, JobSpec | JobRequested, JobStarted, JobCompleted, JobFailed, JobCancelled | Event bus (NATS), REST de Consola |
| Scheduling & Capacity | Asignación óptima y visibilidad de recursos | Entidades: Offer, Node, Constraint; VO: ResourceQuota, AffinityPolicy | OfferCreated, AssignmentCreated, AssignmentRejected, CapacityUpdated | Event bus, Worker Manager (provider) |
| Provider Integration (Worker Manager) | Abstraer infraestructura y operar ejecutores | Entidades: Worker, ProviderEndpoint; VO: ProviderConfig, RuntimeSpec | WorkerCreated, WorkerTerminated, WorkerFailed | Kubernetes API, Docker API; watchers/CRDs[^5][^12] |
| Telemetry & Tracing | Recolección de métricas y trazas | Entidades: Metric, Trace; VO: TagSet, TraceContext | MetricEmitted, TraceSpans, AlertRaised | Streaming (NATS), OpenTelemetry |
| Console & Access | APIs y UI; seguridad y sesiones | Entidades: Dashboard, Session; VO: RBACPolicy, ViewConfig | UIEvent, AuthGranted, AuthRevoked | BFF REST/GraphQL, SSE/WS[^14] |

Los límites se establecen con patrones de integración como Open Host Service y Published Language, utilizando especificaciones públicas (OpenAPI) para contratos de APIs, lo que reduce la fricción entre equipos y facilita la evolución independiente de servicios[^1][^14].

## Patrones de comunicación y eventos distribuidos

La arquitectura adopta un enfoque mixto para equilibrar baja latencia, desacoplamiento y garantías de entrega:

- NATS para control-plane y telemetría por su baja latencia, modelo de subjects jerárquicos y soporte de JetStream (durabilidad, reintentos, exactamente-una vez donde aplique). NATS facilita patrones request-reply, queue groups y streaming eficiente con requisitos operativos moderados[^4].
- Kafka opcional para flujos de alto volumen con requerimientos de replay masivo y persistencia de logs, donde su modelo de topics/particiones y ecosistema maduro (conectores, replays históricos) sean decisivos[^4].

La Tabla 4 compara criterios clave de selección.

Tabla 4. Comparativa NATS vs Kafka

| Criterio | NATS (Core/JetStream) | Kafka |
|---|---|---|
| Arquitectura | Cliente-servidor; malla de servidores; subjects jerárquicos | Logs basados en topics/particiones; clúster de brokers |
| Latencia | Optimizada para tiempo real | Mayor latencia por batch/compresión |
| Throughput | Millones de mensajes por segundo | Millones de registros/s con altos MB/s |
| Garantías de entrega | Core: at-most-once; JetStream: at-least-once/exactly-once configurable | At-least-once y exactly-once |
| Patrones | Publish-subscribe, request-reply, queue groups | Publish-subscribe; grupos de consumidores |
| Complejidad operativa | Baja-media (menos componentes) | Media-alta (brokers, replicación, retención) |
| Casos de uso típicos | Control-plane, telemetría, coordinación | Streaming histórico, analítica, replay masivo |

Para estandarizar semánticas de entrega, la Tabla 5 sintetiza garantías y configuración recomendada.

Tabla 5. Semánticas de entrega de mensajes y configuración

| Semántica | Descripción | NATS (JetStream) | Kafka | Aplicación en CI/CD |
|---|---|---|---|---|
| At-most-once | Sin reintentos; puede perder mensajes | Core NATS (sin persistencia) | N/A estándar | Señales transitorias y métricas de baja criticidad |
| At-least-once | Reintentos hasta éxito | Acks individuales; reintentos automáticos | Repetición por offsets | Eventos de estado de jobs, asignaciones, métricas |
| Exactly-once | Entrega única idempotente | Configuración estricta de JetStream | Transacciones en productores/consumidores | Auditoría crítica y reconciliación de pagos/compliance |

En todos los casos, la idempotencia del consumidor y el uso de claves y correlaciones por job/pipeline se vuelven esenciales para evitar duplicidades y asegurar consistencia operacional[^4].

## Diseño de APIs y contratos internos

El sistema utiliza APIs REST documentadas con OpenAPI para cada servicio, con versionado y políticas de compatibilidad hacia atrás. Los eventos de dominio se publican en subjects jerárquicos en NATS, con correlación por identifiers de job/pipeline y metadatos de trazabilidad. Para comunicación con workers efímeros se exponen endpoints estandarizados de ejecución y streaming de logs, accesibles vía port-forward o mecanismos seguros equivalentes.

La Tabla 6 cataloga los endpoints críticos y sus semánticas.

Tabla 6. Catálogo de endpoints internos

| Servicio       | Método | Ruta               | Semántica                          | Idempotencia     |
| -------------- | ------ | ------------------ | ---------------------------------- | ---------------- |
| Orquestador    | POST   | /jobs              | Crear job (JobRequested)           | Sí, por JobId    |
| Orquestador    | GET    | /jobs/{id}         | Consultar estado                   | N/A              |
| Planificador   | POST   | /schedule          | Solicitar asignación               | Sí, por JobId    |
| Planificador   | GET    | /nodes             | Capacidades y ofertas              | N/A              |
| Worker Manager | POST   | /workers           | Crear worker (WorkerCreated)       | Sí, por WorkerId |
| Worker Manager | DELETE | /workers/{id}      | Terminar worker (WorkerTerminated) | Sí, por WorkerId |
| Worker Manager | GET    | /workers/{id}/logs | Obtener logs                       | N/A              |
| Workers        | POST   | /exec              | Ejecutar step                      | Sí, por StepId   |
| Telemetría     | GET    | /metrics           | Consultar métricas                 | N/A              |
| Telemetría     | GET    | /traces            | Consultar trazas                   | N/A              |
| Consola        | GET    | /ui/jobs           | Agregación de datos                | N/A              |

Las especificaciones OpenAPI sirven como contrato formal y “Published Language” para clientes internos y externos, reforzando el patrón de límites explícitos y facilitando evolución controlada[^14][^1].

## Consideraciones técnicas en Rust

La implementación se apoya en un conjunto de prácticas y librerías consolidadas:

- Actor model con Tokio/Actix: los actores encapsulan estado y comportamiento, mientras que el runtime de Tokio proporciona asincronía y multitarea. Actix facilita la construcción de servidores HTTP y pattern de supervisión/aislamiento de fallos; la comunidad destaca su madurez y adopción en producción[^10][^11].
- Async/await para I/O: selección coherente del runtime (Tokio) y bibliotecas compatibles; evitar mezcla de runtimes para prevenir problemas de interoperabilidad, como se recomienda en comparativas de frameworks web[^11].
- Lock-free para métricas: contadores atómicos y colas de alta concurrencia minimizan contención y permiten escalado en rutas calientes.
- Zero-copy y serialización eficiente: uso de formatos binarios cuando sea viable (por ejemplo, para streaming de logs o telemetría), y buffers reutilizables para reducir copias en rutas de I/O.

La Tabla 7 vincula requisitos con librerías Rust recomendadas.

Tabla 7. Mapa de requisitos y librerías Rust

| Requisito | Librería/patrón | Justificación |
|---|---|---|
| Concurrencia por actores | Tokio + Actix | Runtime maduro, APIs estables, ecosistema productivo[^10][^11] |
| HTTP/API | Actix-web o Axum | Alto rendimiento y ergonomía; integración con OpenAPI[^11] |
| Mensajería | NATS (nats.crate) + JetStream | Baja latencia, subjects jerárquicos, garantías de entrega[^4] |
| Observabilidad | tracing, OpenTelemetry | Trazabilidad distribuida y métricas |
| Kubernetes | kube-rs, k8s-openapi | Cliente, watchers, CRDs, port-forward[^5] |
| Serialización | serde (JSON), binario cuando aplique | Balance entre ergonomía y eficiencia |
| Métricas lock-free | atomic, crossbeam/queues | Minimizar contención en contadores y colas |

## Integración con infraestructura (Kubernetes/Docker y futuros providers)

La arquitectura separa el “qué” (dominio y scheduling) del “dónde” (infraestructura), gracias a la abstracción WorkerManagerProvider. En Kubernetes, se aprovechan patrones como watchers (detección eficiente de cambios), CRDs (extensión de API) y port-forward (observación y acceso seguro). En Docker, la API permite ciclo de vida de contenedores, logs y estadísticas. La seguridad se fundamenta en RBAC, secretos gestionados por el proveedor y segmentación por namespaces/tenants. El diseño permite incorporar proveedores futuros (por ejemplo, compute serverless) sin modificar el núcleo[^5][^12][^8][^9].

La Tabla 8 compara capacidades por proveedor.

Tabla 8. Capacidades por proveedor y nivel de soporte

| Capacidad           | Kubernetes           | Docker                | Futuros providers                 |
| ------------------- | -------------------- | --------------------- | --------------------------------- |
| Crear ejecutores    | Pods/Jobs/CRDs       | Contenedores          | Abstracción uniforme vía Provider |
| Terminar ejecutores | Delete de Pod/Job    | Stop/rm de contenedor | delete/terminate                  |
| Logs                | API de logs; kubelet | API de logs           | provider.logs()                   |
| Port-forward        | API de port-forward  | CLI/API               | provider.port_forward()           |
| Watchers/Events     | Watchers de recursos | Estadísticas/eventos  | provider.stream_events()          |
| Secretos y RBAC     | Nativos              | Secrets básicos       | provider.configure_security()     |

### Abstracción WorkerManagerProvider

La interfaz define métodos para crear, listar, terminar y observar ejecutores; y expone eventos del ciclo de vida (creación, terminación, fallo). Los contratos se alinean con eventos de dominio, lo que desacopla la lógica de orquestación de la infraestructura concreta. Esto facilita la incorporación de nuevos providers con mínima fricción[^5].

### Integración con Kubernetes

El cliente kube-rs permite interactuar con la API de Kubernetes, aplicar CRDs y usar watchers para reaccionar a cambios en recursos. La gestión de Pods incluye creación, etiquetado, obtención de logs y port-forward, todos relevantes para observar y operar workers efímeros[^5][^8].

### Integración con Docker

La API de Docker ofrece operaciones para crear/eliminar contenedores, obtener logs y estadísticas. Se utiliza como fallback cuando Kubernetes no está disponible o para escenarios de ejecución ligera en entornos pre-Kubernetes[^5][^9].

### Preparación para futuros providers

La capa de provider define traits extendibles y versionados, con política de compatibilidad y pruebas contractuales (contract tests). Esto permite, por ejemplo, integrar proveedores de compute serverless con garantías similares de aislamiento y telemetría.

## Resiliencia, escalabilidad y gestión de estado

La resiliencia se asegura con patrones como Circuit Breaker (cortar llamadas a dependencias fallidas), Bulkhead (aislar recursos en pools), Retry/Timeout (reintentos con límites) y Saga (consistencia distribuida mediante compensaciones). El escalado horizontal se aplica por servicio, con el bus de eventos y la partición (en Kafka) o grupos de cola (en NATS) para paralelizar. El estado se persiste por servicio (database per service), y donde se requiera consistencia cross-service se aplican Sagas con eventos de compensación[^2].

La Tabla 9 mapea patrones de resiliencia y dónde aplicarlos.

Tabla 9. Patrones de resiliencia y puntos de aplicación

| Patrón | Aplicación | Servicios |
|---|---|---|
| Circuit Breaker | Llamadas a providers (K8s/Docker) y APIs externas | Worker Manager, Consola |
| Bulkhead | Aislamiento de pools de conexiones y colas por dominio | Orquestador, Planificador, Telemetría |
| Retry/Timeout | Publicación/consumo de eventos y llamadas HTTP | Todos, especialmente Worker Manager |
| Saga | Flujos multi-etapa de job/pipeline | Orquestador (Pipeline Orchestration) |
| Idempotencia | Comandos de creación/terminación | Orquestador, Worker Manager |
| Backpressure | Ingestión de eventos y scheduling | Planificador, Telemetría |

## Roadmap de implementación y criterios de aceptación

La entrega se organiza en iteraciones incrementales que mitigan riesgos y agregan valor temprano.

- Iteración 1: Orquestador mínimo viable (APIs de jobs/pipelines; eventos base; almacenamiento por servicio).
- Iteración 2: Integración con Kubernetes (kube-rs, creación/terminación de pods, logs, watchers; Worker Manager抽象).
- Iteración 3: Planificador con visibilidad de recursos y políticas básicas (FIFO, packing).
- Iteración 4: Telemetría y trazabilidad (métricas, streaming de logs, trazas distribuidas).
- Iteración 5: Consola/BFF (dashboards, streaming de estado en tiempo real, RBAC y seguridad).

Los criterios de aceptación se centran en latencia end-to-end, throughput, tasa de éxito de jobs, cobertura de telemetría, cumplimiento de contratos OpenAPI y operación estable de NATS/Kafka. La Tabla 10 resume hitos y métricas objetivo[^12][^4].

Tabla 10. Hitos e indicadores objetivo

| Hito | Entregables | Métricas objetivo |
|---|---|---|
| MVP Orquestador | REST /jobs, /pipelines; eventos base | p95 latencia creación job < 300 ms |
| K8s Integrado | Worker Manager con pods y logs | Éxito creación/terminación > 99.5% |
| Planificador | Capacidades y offers; FIFO/packing | Asignación p95 < 500 ms; saturación balanceada |
| Telemetría | Métricas y trazas; streaming | Cobertura eventos clave 100% |
| Consola | Dashboards SSE/WS; RBAC | p95 consulta UI < 400 ms; uptime 99.9% |

## Riesgos, supuestos y mitigaciones

Los riesgos principales se relacionan con la complejidad operativa de brokers de mensajes, la consistencia eventual y la mezcla de runtimes asíncronos. Las mitigaciones incluyen instrumentación robusta, pruebas contractuales de eventos y estricta selección de librerías. La Tabla 11 consolida los riesgos y sus mitigaciones[^4][^11].

Tabla 11. Matriz de riesgos y mitigaciones

| Riesgo | Impacto | Probabilidad | Mitigación |
|---|---|---|---|
| Configuración y operación de NATS/Kafka | Medio | Media | Automatización, observabilidad del bus, runbooks |
| Consistencia eventual entre servicios | Medio | Alta | Idempotencia, Sagas, auditoría de eventos |
| Mezcla de runtimes async | Alto | Media | Selección única (Tokio), pruebas de integración[^11] |
| Saturación de scheduling | Alto | Media | Backpressure, políticas de packing, autoscaling del Planificador |
| Seguridad multi-tenant | Alto | Media | RBAC, segmentación, secretos gestionados |
| Falla de provider (K8s/Docker) | Alto | Baja-Media | Circuit Breaker, fallback y colas de reintento |

## Referencias

[^1]: Microsoft Learn — Azure Architecture Center. “Domain analysis for microservices.” https://learn.microsoft.com/en-us/azure/architecture/microservices/model/domain-analysis

[^2]: Codefresh. “Top 10 Microservices Design Patterns and How to Choose.” https://codefresh.io/learn/microservices/top-10-microservices-design-patterns-and-how-to-choose/

[^3]: microservices.io. “Pattern: Microservice Architecture.” https://microservices.io/patterns/microservices.html

[^4]: Synadia. “NATS and Kafka Compared.” https://www.synadia.com/blog/nats-and-kafka-compared

[^5]: Shuttle.dev. “Using Kubernetes with Rust.” https://www.shuttle.dev/blog/2024/10/22/using-kubernetes-with-rust

[^6]: Bits and Pieces. “Understanding the Bounded Context in Microservices.” https://blog.bitsrc.io/understanding-the-bounded-context-in-microservices-c70c0e189dd1

[^7]: Martin Fowler. “Bounded Context.” https://www.martinfowler.com/bliki/BoundedContext.html

[^8]: Kubernetes Documentation. “Kubectl.” https://kubernetes.io/docs/tasks/tools/#kubectl

[^9]: Docker Docs. “Test your Rust deployment.” https://docs.docker.com/guides/rust/deploy/

[^10]: Ryuichi. “Actors with Tokio.” https://ryhl.io/blog/actors-with-tokio/

[^11]: Luca Palmieri. “Choosing a Rust web framework, 2020 edition.” https://www.lpalmieri.com/posts/2020-07-04-choosing-a-rust-web-framework-2020-edition/

[^12]: Kubernetes Documentation. “Efficient detection of changes (watchers).” https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes

[^13]: Microsoft Learn — Azure Architecture Center. “Microservices architecture style.” https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices

[^14]: OpenAPI Specification. “Latest.” https://spec.openapis.org/oas/latest.html

---

Información pendiente (gaps reconocidos para fases posteriores):

- Benchmarks de latencia y throughput para NATS vs Kafka en escenarios específicos de CI/CD (construcción, paralelización, fan-out) y su impacto en SLOs.
- Detalles de autenticación/autorización multi-tenant, gestión de secretos y RBAC por proveedor de infraestructura.
- Especificación de CRDs o recursos extendidos para jobs CI/CD en Kubernetes y su ciclo de vida (contratos detallados).
- Estrategias de serialización binaria y diseños zero-copy para transferencia de artefactos y logs, con límites de tamaño y codecs.
- Definición de políticas de scheduling avanzadas (preemptible jobs, pool de GPU, afinidad por región/zone) con métricas y SLOs precisos.