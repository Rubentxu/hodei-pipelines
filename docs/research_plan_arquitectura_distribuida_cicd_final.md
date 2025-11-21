# Blueprint de arquitectura distribuida para una plataforma CI/CD en Rust: Orquestador, Planificador, Worker Manager y Workers efímeros

## 1. Resumen ejecutivo y objetivos

Este documento define un blueprint de arquitectura distribuida para una plataforma de Integración y Despliegue Continuos (CI/CD) escrita en Rust. El objetivo es articular un diseño que permita a equipos de plataforma construir un sistema de ejecución de pipelines altamente disponible, observable y extensible, con límites claros de responsabilidad, acoplamiento débil y una capa de abstracción para operar sobre distintos proveedores de infraestructura.

El alcance abarca seis dominios funcionales principales: Orquestador, Planificador, Worker Manager, Workers efímeros, Telemetría/Trazabilidad y Consola (Backends for Frontends, BFF). La propuesta se rige por los principios de Domain-Driven Design (DDD) y patrones de microservicios ampliamente validados, para asegurar que cada componente evolucione de forma autónoma sin comprometer la cohesión del sistema[^1][^2][^3].

Los criterios de éxito incluyen: una plataforma capaz de ejecutar pipelines en paralelo con elasticidad, con resiliencia ante fallos y capacidades de observabilidad y trazabilidad completas. La consola y el BFF ofrecen una interfaz de usuario coherente y segura, y el diseño deja abierta la incorporación de proveedores de infraestructura futuros a través de una interfaz uniforme.

Finalmente, reconocemos varias lagunas de información a la fecha que deben resolverse en fases posteriores: detalles de integración con la API de Docker; benchmarks comparativos entre NATS y Kafka en escenarios CI/CD específicos; autenticación, autorización y seguridad multi-tenant; estrategias de serialización binaria y zero-copy; definición y ciclo de vida de CRDs específicas para CI/CD; SLOs detallados por componente; y políticas avanzadas de scheduling más allá de FIFO y packing[^4].

## 2. Principios y objetivos de arquitectura

La arquitectura propuesta se sustenta en los principios del Diseño Orientado al Dominio (DDD) y de microservicios. En DDD, el modelado estratégico organiza el sistema por capacidades de negocio y establece límites lingüísticos y organizativos (bounded contexts), mientras que el modelado táctico define las entidades, agregados y servicios de dominio dentro de esos límites[^1]. En la perspectiva de microservicios, cada servicio debe ser autónomo, altamente cohesivo, y acoplado débilmente, con comunicación síncrona y asíncrona que preserve estas propiedades[^2][^3].

En este marco, la arquitectura busca:

- Escalabilidad y resiliencia: los fallos se manejan con aislamiento (bulkhead), cortes automáticos (circuit breaker), reintentos con backoff y timeouts, y se evitan propagaciones sistémicas[^2].
- Observabilidad y trazabilidad integradas: telemetría unificada, logs estructurados, métricas y trazas distribuidas.
- Seguridad y multi-tenancy: secretos gestionados por el proveedor (Kubernetes/Docker), permisos mínimos necesarios (RBAC), segmentación por namespaces/tenants[^12].
- Portabilidad cloud-native: ejecución nativa en Kubernetes, con soporte a Docker y previsión para nuevos proveedores.

En conjunto, estos principios gobiernan decisiones sobre el diseño de interfaces, la selección de tecnologías y los contratos entre componentes.

## 3. Componentes principales del sistema

El sistema se estructura en seis componentes con responsabilidades acotadas y contratos explícitos. Para visualizar su interacción, introducimos el mapa de responsabilidades y contratos, y el diagrama lógico de eventos.

Tabla 1. Mapa de responsabilidades y contratos por componente

| Componente | Responsabilidades | Tecnologías/patrones | Contratos | Notas |
|---|---|---|---|---|
| Orquestador | Ciclo de vida de jobs/pipelines; reconciliación; Sagas | Actix/Tokio; NATS JetStream | REST /jobs, /pipelines; Eventos JobRequested, JobStarted, JobCompleted, JobFailed | Idempotencia y reintentos; auditoría |
| Planificador | Matching job → worker; políticas de asignación; capacidad en tiempo real | Actors con Tokio; lock-free métricas | REST /schedule, /nodes; Eventos OfferCreated, AssignmentCreated, CapacityUpdated | FIFO, packing por recursos, afinidades |
| Worker Manager | Abstracción multi-provider; CRUD de ejecutores; observers | kube-rs; API Docker; traits | Operaciones create/terminate/logs; Eventos WorkerCreated, WorkerTerminated, WorkerFailed | Integración con K8s/Docker[^5][^12][^9][^10] |
| Workers efímeros | Ejecución aislada; reporting de logs/métricas; cleanup | Containers/Pods; HTTP callbacks | Endpoints /exec, /logs; Eventos StepStarted/Completed/Failed | Aislamiento por namespace/pod; límites de recursos |
| Telemetría/Trazabilidad | Métricas, eventos, trazas; streaming a dashboards | Tokio/Actix; NATS; OpenTelemetry | Streams de métricas; REST /metrics, /traces | Retención configurable |
| Consola (BFF) | UI; APIs agregadas; auth; streaming en tiempo real | Axum/Actix; WebSockets; OpenAPI | REST /ui/*; SSE/WS /events | RBAC; caché selectiva[^14][^13] |

Para complementar la visión estática de la Tabla 1, el siguiente diagrama lógico ilustra el flujo de eventos entre componentes y los canales de comunicación predominantes.

![Diagrama lógico de componentes y flujo de eventos (alta nivel).](docs/img/componentes_logica.png)

El diagrama destaca la separación entre control-plane (NATS) y streaming de métricas, así como el rol del Worker Manager como extensión de la plataforma hacia Kubernetes/Docker. La relevancia práctica de esta separación es doble: reduce el acoplamiento temporal y mejora la tolerancia a fallos, ya que los componentes pueden evolucionar y escalar de manera independiente según su carga y criticidad[^2][^5].

A continuación, describimos los estados y transiciones clave del ciclo de vida de un job.

Tabla 2. Estados y transiciones de un job

| Estado | Transiciones salientes | Disparador | Acción del Orquestador/Planificador |
|---|---|---|---|
| PENDING | → SCHEDULED | JobRequested | Planificador crea OfferCreated; Orquestador programa |
| SCHEDULED | → RUNNING | WorkerCreated/AssignmentAccepted | Orquestador inicia ejecución; publica JobStarted |
| RUNNING | → SUCCEEDED | JobCompleted | Orquestador persiste resultado y notifica |
| RUNNING | → FAILED | JobFailed | Orquestador ejecuta compensación (Saga) |
| RUNNING | → PENDING (retry) | StepFailed con retry | Reintento con backoff e idempotencia |
| SCHEDULED/RUNNING | → CANCELLED | JobCancelled | Cancela assignments; limpia workers |
| Any | → TERMINATED | WorkerTerminated | Reintento o compensación; limpieza |

La importancia de definir explícitamente estos estados y transiciones reside en que proveen un contrato operacional entre equipos de plataforma, orquestación y ejecución. Un contrato claro simplifica la instrumentación de telemetría y trazabilidad, y facilita diagnósticos cuando ocurren fallos.

### 3.1 Orquestador

El Orquestador es el núcleo de coordinación. Administra pipelines y jobs, asegura reconciliación de estados y aplica Sagas para consistencia distribuida cuando una operación requiere múltiples transacciones locales. Emite eventos sobre el bus NATS (JobRequested, JobStarted, JobCompleted, JobFailed) y expone APIs REST para consultar y administrar el ciclo de vida. La idempotencia de comandos y los reintentos con backoff son prácticas obligatorias para robustez[^2].

### 3.2 Planificador

El Planificador recibe las solicitudes de ejecución y, con visibilidad de la capacidad disponible, publica ofertas y asignaciones (OfferCreated, AssignmentCreated). Implementa políticas como FIFO y packing por recursos, además de afinidades o restricciones de ubicación. Se instrumenta con métricas lock-free (contadores e histogramas) para monitorear colas y saturación. La comunicación bidireccional con el Worker Manager permite ajustar la asignación ante cambios de estado o capacidad[^2].

### 3.3 Worker Manager

El Worker Manager presenta la interfaz WorkerManagerProvider, que unifica operaciones create/terminate/logs/port-forward para diferentes proveedores. En Kubernetes, se apoya en kube-rs para manejar Pods, aplicar CRDs, observar cambios con watchers y habilitar port-forward. Publica eventos WorkerCreated, WorkerTerminated y WorkerFailed para mantener sincronía con el Orquestador y el Planificador[^5][^12][^8][^9].

![Secuencia de creación y terminación de workers efímeros.](docs/img/secuencia_workers.png)

El diagrama de secuencia ilustra cómo el Orquestador solicita al Worker Manager la creación de un worker, cómo se publica el evento correspondiente y cómo la observación de cambios (watchers) realimenta el estado global. La relevancia operativa es que este circuito permite reaccionar ante fallos de infraestructura sin intervención manual, manteniendo la continuidad del pipeline.

### 3.4 Workers efímeros

Los workers son pods o contenedores de corta vida que ejecutan pasos de pipeline en aislamiento. Reportan logs y métricas mediante callbacks o streaming; el Worker Manager puede realizar port-forward para inspección segura. La seguridad se sustenta en segmentación por namespace/pod, secretos gestionados por el proveedor y límites de recursos, reduciendo el riesgo de fuga o consumo descontrolado[^5][^12].

### 3.5 Telemetría y trazabilidad

La telemetría unificada integra métricas de sistema y de jobs, eventos de negocio y trazas distribuidas. Un canal de streaming (NATS) transporta métricas a la Consola y sistemas de observabilidad. La correlación de trazas entre componentes facilita diagnósticos y auditorías. La política de retención se parametriza según necesidades de cumplimiento y operación[^4].

![Pipeline de telemetría y flujos hacia la consola/dashboards.](docs/img/telemetria_flujo.png)

La figura anterior muestra los puntos de instrumentación y los flujos hacia la Consola. Lo esencial es que la telemetría no es un “añadido” tardío, sino parte integral del diseño, de modo que cada evento de negocio tenga trazabilidad suficiente para reconstruir estados y responsabilidades.

### 3.6 Consola (dashboard y APIs)

La Consola funciona como BFF: expone endpoints específicos de UI, agrega información de múltiples servicios y publica streaming de estado (SSE/WebSockets). Integra RBAC y autenticación, y define una caché de metadatos con invalidación basada en eventos. Las APIs se documentan con OpenAPI, facilitando coherencia y evolución controlada[^14][^13].

## 4. Bounded Contexts (DDD) y modelo de dominio

El modelado DDD estratégico delimita contextos con un lenguaje y reglas propios, evitando la “anemia” de un modelo único para múltiples propósitos. La propuesta identifica cinco contextos: Pipeline Orchestration, Scheduling & Capacity, Provider Integration, Telemetry & Tracing y Console & Access. Cada uno define agregados, entidades y objetos de valor con eventos de dominio asociados. La comunicación entre contextos usa APIs publicadas y eventos de negocio, preservando límites y autonomía[^1][^6][^7].

Tabla 3. Bounded Contexts y modelos asociados

| Bounded Context | Propósito | Agregados/Entidades/Objetos de valor | Eventos clave | Integraciones |
|---|---|---|---|---|
| Pipeline Orchestration | Coordinar ciclos de vida de pipelines y jobs | Pipeline, Job; Step, Assignment; JobSpec, PipelineSpec | JobRequested, JobStarted, JobCompleted, JobFailed | NATS, Consola |
| Scheduling & Capacity | Asignación óptima y visibilidad | Offer, Node; Constraint; ResourceQuota, AffinityPolicy | OfferCreated, AssignmentCreated, CapacityUpdated | NATS, Worker Manager |
| Provider Integration | Abstraer infraestructura | Worker, ProviderEndpoint; ProviderConfig, RuntimeSpec | WorkerCreated, WorkerTerminated, WorkerFailed | Kubernetes API, Docker API |
| Telemetry & Tracing | Métricas y trazas | Metric, Trace; TagSet, TraceContext | MetricEmitted, TraceSpans | NATS, OpenTelemetry |
| Console & Access | UI y APIs | Dashboard, Session; RBACPolicy, ViewConfig | UIEvent, AuthGranted, AuthRevoked | BFF REST/GraphQL, SSE/WS |

![Mapa de bounded contexts y sus interacciones.](docs/img/ddd_context_map.png)

El mapa de contextos enfatiza que “Pipeline Orchestration” y “Scheduling & Capacity” son cores de negocio, mientras que “Provider Integration” es un contexto de soporte. La clave es que cada modelo represente las entidades con atributos relevantes al contexto, evitando que un modelo único se “contamine” con preocupaciones que pertenecen a otros dominios[^6]. Esta separación reduce complejidad accidental y acelera la evolución de cada servicio.

## 5. Patrones de comunicación y eventos distribuidos

La plataforma adopta NATS como backbone de control-plane y telemetría por su baja latencia, arquitectura ligera y soporte de persistencia con JetStream. NATS facilita patrones request-reply, publish-subscribe y grupos de cola, y su modelo de subjects jerárquicos permite un direccionamiento expresivo y flexible. Kafka se reserva opcionalmente para escenarios de streaming histórico, replay masivo y alto throughput sostenido[^4].

Tabla 4. Comparativa NATS vs Kafka

| Criterio | NATS (Core/JetStream) | Kafka |
|---|---|---|
| Arquitectura | Cliente-servidor; malla; subjects | Topics/particiones; clúster de brokers |
| Latencia | Tiempo real optimizada | Mayor por batch/compresión |
| Throughput | Millones de msg/s | Millones de registros/s |
| Garantías | Core at-most-once; JetStream configurable | At-least-once/exactly-once |
| Patrones | Pub-sub, request-reply, colas | Pub-sub; grupos de consumidores |
| Complejidad | Baja-media | Media-alta |

Tabla 5. Semánticas de entrega y configuración recomendada

| Semántica | NATS | Kafka | Uso CI/CD |
|---|---|---|---|
| At-most-once | Core (sin persistencia) | — | Señales transitorias, métricas no críticas |
| At-least-once | JetStream (acks/retry) | Grupos de consumidores | Estados de jobs, asignaciones, métricas |
| Exactly-on-once | JetStream (transaccional) | Transacciones | Auditoría, reconciliación regulatoria |

![Comparativa visual NATS vs Kafka para el dominio CI/CD.](docs/img/nats_vs_kafka.png)

La figura comparativa refuerza que NATS se alinea con las necesidades del control-plane y telemetría de una plataforma CI/CD, mientras que Kafka se especializa en pipelines de datos históricos. Esta segmentación reduce costos operativos y complejidad innecesaria, sin sacrificar garantías de entrega cuando son necesarias[^4].

## 6. Diseño de APIs y contratos internos

Cada servicio expone APIs REST documentadas con OpenAPI. Los eventos se publican en subjects jerárquicos en NATS, con correlación por identificadores de job/pipeline y metadatos de trazabilidad. Para workers se definen endpoints de ejecución y streaming de logs con semánticas claras de idempotencia, autenticación y autorización.

Tabla 6. Catálogo de endpoints internos

| Servicio | Método | Ruta | Semántica | Idempotencia |
|---|---|---|---|---|
| Orquestador | POST | /jobs | Crear job | Sí (JobId) |
| Orquestador | GET | /jobs/{id} | Consultar estado | — |
| Planificador | POST | /schedule | Solicitar asignación | Sí (JobId) |
| Planificador | GET | /nodes | Capacidades | — |
| Worker Manager | POST | /workers | Crear worker | Sí (WorkerId) |
| Worker Manager | DELETE | /workers/{id} | Terminar worker | Sí (WorkerId) |
| Worker Manager | GET | /workers/{id}/logs | Obtener logs | — |
| Workers | POST | /exec | Ejecutar step | Sí (StepId) |
| Telemetría | GET | /metrics | Consultar métricas | — |
| Telemetría | GET | /traces | Consultar trazas | — |
| Consola | GET | /ui/jobs | Agregación | — |

![Especificación OpenAPI como contrato Published Language.](docs/img/openapi_contrato.png)

La estandarización mediante OpenAPI opera como “Published Language”, alineando expectativas entre equipos y reduciendo fricción por cambios no coordinados[^14][^1].

## 7. Consideraciones técnicas en Rust

El diseño adopta el modelo de actores con Tokio/Actix para concurrencia, supervisión y manejo de backpressure. Async/await estructura las operaciones de I/O, y estructuras lock-free (contadores atómicos, colas no bloqueantes) reducen contención en rutas calientes. El diseño considera zero-copy y buffers reutilizables para minimizar copias en canales de datos.

Tabla 7. Mapa de requisitos y librerías Rust

| Requisito | Librería/patrón | Justificación |
|---|---|---|
| Concurrencia por actores | Tokio + Actix | Madurez, rendimiento y ecosistema productivo[^10][^11] |
| HTTP/API | Actix-web/Axum | Alto rendimiento, ergonomía, OpenAPI[^11] |
| Mensajería | NATS + JetStream | Baja latencia, garantías de entrega[^4] |
| Observabilidad | tracing, OpenTelemetry | Trazabilidad distribuida |
| Kubernetes | kube-rs, k8s-openapi | CRUD pods, CRDs, port-forward, watchers[^5] |
| Serialización | serde (JSON/binario) | Balance ergonomía/eficiencia |
| Métricas | atomic, crossbeam | Contención mínima en hot paths |

La coherencia en el runtime (Tokio) evita problemas de interoperabilidad entre crates asíncronos, como se recomienda en evaluaciones comparativas de frameworks web en Rust[^11].

## 8. Integración con infraestructura (Kubernetes/Docker y futuros providers)

La capa WorkerManagerProvider abstrae la infraestructura y unifica operaciones de ciclo de vida de ejecutores. En Kubernetes, se integra con la API para crear/eliminar Pods, aplicar CRDs, observar cambios y habilitar port-forward. En Docker, se usan operaciones equivalentes para contenedores y logs. El diseño contempla seguridad con RBAC y secretos gestionados por el proveedor, y deja abierta la incorporación de nuevos proveedores.

Tabla 8. Capacidades por proveedor y nivel de soporte

| Capacidad | Kubernetes | Docker | Futuros providers |
|---|---|---|---|
| Crear ejecutores | Pods/Jobs/CRDs | Contenedores | Interfaz uniforme |
| Terminar | Delete Pod/Job | Stop/rm contenedor | terminate |
| Logs | API de logs | API de logs | provider.logs() |
| Port-forward | API de port-forward | CLI/API | provider.port_forward() |
| Watchers/Events | Watchers de recursos | Estadísticas/eventos | provider.stream_events() |
| Secretos/RBAC | Nativos | Secrets básicos | provider.configure_security() |

![Flujo de integración Worker Manager ↔ Kubernetes/Docker.](docs/img/provider_integration_flow.png)

El flujo evidencia que la integración con Kubernetes es la vía preferente para una plataforma cloud-native, mientras que Docker actúa como fallback o entorno de desarrollo. El uso de watchers y CRDs en Kubernetes habilita un modelo reactivo y extensible, crucial para escalar y automatizar operaciones de CI/CD[^5][^12][^8][^9][^10].

## 9. Resiliencia, escalabilidad y gestión de estado

La resiliencia se logra combinando patrones de protección y aislamiento (Circuit Breaker, Bulkhead), manejo de fallos (Retry/Timeout) y consistencia distribuida con Saga. El escalado horizontal se aplica por servicio, usando el bus de eventos y, cuando corresponda, particiones o grupos de cola para paralelismo. Cada servicio persiste su estado (database per service), y se instrumenta auditoría para trazabilidad y recuperabilidad.

Tabla 9. Patrones de resiliencia y puntos de aplicación

| Patrón | Aplicación | Servicios |
|---|---|---|
| Circuit Breaker | Llamadas a proveedores y APIs externas | Worker Manager, Consola |
| Bulkhead | Aislamiento de pools y colas | Orquestador, Planificador, Telemetría |
| Retry/Timeout | Publicación/consumo de eventos | Todos |
| Saga | Flujos multi-etapa | Orquestador |
| Idempotencia | Comandos de creación/terminación | Orquestador, Worker Manager |
| Backpressure | Ingestión de eventos y scheduling | Planificador, Telemetría |

La adopción disciplinada de estos patrones reduce la probabilidad y el impacto de fallos, y acelera la recuperación cuando ocurren, sin comprometer la autonomía de cada servicio[^2].

## 10. Roadmap de implementación y criterios de aceptación

El roadmap propone cinco iteraciones incrementales:

1) Orquestador MVP: APIs de jobs/pipelines y eventos base.
2) Integración Kubernetes: Worker Manager con creación/terminación de pods, logs y watchers.
3) Planificador: visibilidad de recursos y políticas de asignación básicas.
4) Telemetría y trazabilidad: métricas, streaming de logs y trazas distribuidas.
5) Consola/BFF: dashboards, streaming en tiempo real y seguridad.

Tabla 10. Hitos e indicadores objetivo

| Hito | Entregables | Métricas objetivo |
|---|---|---|
| MVP Orquestador | REST /jobs, /pipelines; eventos | p95 creación job < 300 ms |
| K8s Integrado | Worker Manager + pods/logs | Éxito ciclo de vida > 99.5% |
| Planificador | Capacidades/offers; FIFO/packing | p95 asignación < 500 ms |
| Telemetría | Métricas y trazas; streaming | Cobertura de eventos clave 100% |
| Consola | Dashboards SSE/WS; RBAC | p95 consulta UI < 400 ms; uptime 99.9% |

Este plan equilibra riesgo y valor, priorizando primero el núcleo de coordinación y la integración con Kubernetes, antes de sofisticar scheduling y observabilidad[^12][^4].

## 11. Riesgos, supuestos y mitigaciones

- Operación de NATS/Kafka: complejidad de configuración y tuning. Mitigación: automatización, observabilidad del bus y runbooks.
- Consistencia eventual: riesgo de divergencia temporal entre servicios. Mitigación: idempotencia, Sagas y auditoría.
- Mezcla de runtimes async: incompatibilidades y errores sutiles. Mitigación: selección única (Tokio) y pruebas de integración.
- Saturación de scheduling: colas y backlogs. Mitigación: backpressure, políticas de packing y autoscaling del Planificador.
- Seguridad multi-tenant: exposición por permisos excesivos. Mitigación: RBAC, segmentación y secretos gestionados.

Tabla 11. Matriz de riesgos

| Riesgo | Impacto | Probabilidad | Mitigación |
|---|---|---|---|
| Brokers de eventos | Medio | Media | Automatización y observabilidad |
| Consistencia eventual | Medio | Alta | Idempotencia y Sagas |
| Runtimes async mezclados | Alto | Media | Tokio unificado |
| Saturación | Alto | Media | Backpressure y autoscaling |
| Seguridad | Alto | Media | RBAC y segmentación |

Estas mitigaciones se apoyan en las propiedades de NATS/Kafka y en buenas prácticas del ecosistema Rust para reducir riesgos operativos y de integración[^4][^11].

## 12. Anexos

### 12.1 Glosario

Tabla 12. Glosario y definiciones

| Término | Definición |
|---|---|
| Bounded Context | Límite lingüístico y organizacional donde un modelo de dominio es válido[^6][^7] |
| Agregado | Unidad transaccional de entidades coherentes en DDD[^1] |
| Saga | Patrón de consistencia distribuida con transacciones locales y compensaciones[^2] |
| Backpressure | Mecanismo para regular flujo ante congestión |
| JetStream | Persistencia y durabilidad en NATS[^4] |
| RBAC | Control de acceso basado en roles en Kubernetes[^12] |
| CRD | Recurso personalizado para extender la API de Kubernetes[^12] |
| Watchers | Mecanismo eficiente de detección de cambios en K8s[^12] |

### 12.2 Modelo de datos por contexto (resumen)

Tabla 13. Entidades y atributos principales por contexto

| Contexto | Entidades/Agregados | Atributos clave |
|---|---|---|
| Pipeline Orchestration | Pipeline, Job, Step, Assignment | PipelineSpec, JobSpec, estado, timestamps |
| Scheduling & Capacity | Offer, Node, Constraint | ResourceQuota, AffinityPolicy, capacidad |
| Provider Integration | Worker, ProviderEndpoint, RuntimeSpec | ProviderConfig, runtime, credenciales |
| Telemetry & Tracing | Metric, Trace, TagSet | TraceContext, etiquetas, valores |
| Console & Access | Dashboard, Session, RBACPolicy | ViewConfig, permisos, tokens |

Este modelo resume las estructuras necesarias para sostener los contratos y eventos descritos en el documento.

### 12.3 Convenciones de nombres de eventos y subjects de NATS

Los eventos se publican bajo subjects jerárquicos que reflejan el dominio y el tipo de evento. Por ejemplo:

- domain.pipeline.job.requested
- domain.pipeline.job.started
- domain.pipeline.job.completed
- domain.scheduler.offer.created
- domain.provider.worker.created
- domain.telemetry.metric.emitted

Esta convención mejora la expresividad y la observabilidad del flujo de eventos en el bus.

### 12.4 Política de versionado de APIs

Se adopta versionado mayor en la ruta (por ejemplo, /v1/jobs), con políticas de compatibilidad hacia atrás. Cambios no compatibles elevan la versión mayor y se comunican con antelación suficiente a consumidores internos.

### 12.5 Estructuras lock-free y contadores atómicos

En el Planificador y Telemetría, contadores atómicos (AtomicU64) y colas lock-free (crossbeam) se emplean para minimizar contención y permitir escalado en rutas calientes. Se definen métricas de latencia de colas y tasas de rechazo para ajustar políticas de backpressure.

### 12.6 Supuestos y lagunas de información

Reconocemos las siguientes lagunas que deberán resolverse en la siguiente fase:

- Integración con Docker: falta de guía formal de arquitectura de la API de Docker en Rust.
- Benchmarks NATS vs Kafka: no disponibles para escenarios específicos de CI/CD (construcción, paralelización, fan-out).
- Seguridad avanzada multi-tenant: autenticación/autorización, gestión de secretos, RBAC por proveedor.
- Serialización binaria y zero-copy: elección de codecs y formatos para transferencia de artefactos y logs.
- CRDs para CI/CD: definición detallada de recursos específicos y su ciclo de vida.
- SLOs del scheduler: parámetros numéricos, métricas y objetivos de desempeño detallados.
- Políticas de scheduling avanzadas: preemption, pools dedicados de GPU, afinidad por región/zone.

La resolución de estas lagunas permitirá afinar decisiones de rendimiento, seguridad y operación.

---

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