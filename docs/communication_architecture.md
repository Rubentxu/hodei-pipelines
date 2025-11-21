# Arquitectura de Comunicación y Eventos para Plataforma CI/CD Distribuida (Rust)

## Resumen ejecutivo y objetivos

Este documento blueprint establece la arquitectura de comunicación y eventos para una plataforma de Integración y Despliegue Continuos (CI/CD) distribuida, multicontenido y cloud‑native, implementada en Rust. El sistema se compone de seis componentes principales: Orquestador, Planificador, Worker Manager, Workers efímeros, Telemetría/Trazabilidad y Consola (Backend‑for‑Frontend). La propuesta adopta un enfoque event‑driven con un backbone de mensajería para el control‑plane y la telemetría, complementado por APIs síncronas para operaciones CRUD y control, y streaming en tiempo real para la interfaz y la observabilidad operacional.

Las decisiones centrales son: (i) seleccionar NATS JetStream como bus por defecto para el control‑plane y la telemetría por su baja latencia y semánticas de entrega configurables; (ii) disponer de Apache Kafka opcional para escenarios con throughput histórico masivo y replays extendidos; (iii) emplear gRPC/HTTP2 para llamadas síncronas de alto rendimiento; (iv) exponer WebSockets o Server‑Sent Events (SSE) para la Consola; y (v) estandarizar Protocol Buffers (protobuf) como formato binario para eventos y DTOs, asegurando compatibilidad y versionado. Este conjunto habilita resiliencia, observabilidad de punta a punta y escalabilidad horizontal, con contratos formales en OpenAPI para APIs REST y una taxonomía de eventos coherente con los bounded contexts definidos[^13][^4].

Los principios rectores —acoplamiento débil, cohesión por capacidad, database‑per‑service, idempotencia, Sagas y backpressure— se aplican transversalmente para contener la complejidad y aislar fallos. La propuesta se alinea con patrones modernos de microservicios y DDD estratégico/táctico, y se instrumenta con trazabilidad distribuida y streaming de métricas hacia paneles operativos[^2][^13].

## Contexto arquitectónico y modelo de dominio (inputs base)

La arquitectura se fundamenta en seis componentes con responsabilidades claras y contratos explícitos:

- Orquestador: gobierna el ciclo de vida de pipelines y jobs, publica eventos de estado y aplica Sagas de compensación. Expone APIs REST para creación/consulta de jobs y endpoints de cancel.
- Planificador: realiza matching job→worker con visibilidad de recursos en tiempo casi real, publica ofertas y asignaciones y aplica políticas de scheduling (FIFO, packing por recursos, afinidades).
- Worker Manager: capa de abstracción multi‑provider (Kubernetes/Docker) para crear/terminar workers, obtener logs y observar eventos del provider; publica WorkerCreated/Terminated/Failed.
- Workers efímeros: ejecutan pasos en aislamiento y reportan StepStarted/StepCompleted/StepFailed; habilitan streaming de logs y métricas de ejecución.
- Telemetría/Trazabilidad: centraliza emisión de MetricEmitted y TraceSpans, integra OpenTelemetry para spans por servicio y define retención por tipo.
- Consola (BFF): expone endpoints para UI, agrega datos, integra SSE/WS y aplica RBAC; publica UIEvent para eventos de cliente.

Los bounded contexts (Orchestration, Scheduling, Worker Management, Execution, Telemetry, Security, Artifact Management) emiten y consumen eventos con sujetos jerárquicos en NATS. El modelo de dominio define agregados como Pipeline y Job; entidades como Step, Assignment, Worker, Offer, Node; y objetos de valor como JobSpec, PipelineSpec, ResourceQuota y RuntimeSpec. La semántica de entrega en el bus se alinea con idempotencia y deduplicación por claves (JobId, StepId, AssignmentId) para garantizar consistencia operacional[^1][^7][^14].

Para anclar la relación entre eventos y servicios, la siguiente tabla sintetiza los flujos esenciales.

Tabla 1. Bounded contexts → eventos → servicios productores/consumidores

| Bounded Context | Evento | Productor | Consumidores |
|---|---|---|---|
| Orchestration | JobRequested, JobStarted, JobCompleted, JobFailed, JobCancelled | Orquestador | Planificador, Worker Manager, Telemetría, Consola |
| Scheduling | OfferCreated, AssignmentCreated, AssignmentRejected, CapacityUpdated | Planificador | Orquestador, Worker Manager, Telemetría |
| Worker Management | WorkerCreated, WorkerTerminated, WorkerFailed | Worker Manager | Orquestador, Planificador, Telemetría |
| Execution | StepStarted, StepCompleted, StepFailed | Worker/Execution | Orquestador, Telemetría, Consola |
| Telemetry | MetricEmitted, TraceSpans, AlertRaised | Telemetría | Consola, SRE tooling |
| Security | AuthGranted, AuthRevoked, AuthorizationDenied | Seguridad | Todos (audit) |
| Artifact | ArtifactUploaded, ArtifactPromoted, ArtifactValidated | Artifact Management | Orchestration, Consola |

La gobernanza de contratos se apoya en especificaciones OpenAPI (REST) y catálogos de eventos versionados, garantizando compatibilidad hacia atrás y evolución controlada[^14][^13].

## Sistema de eventos distribuido (backbone, schemas, patterns)

La selección del backbone se realiza por caso de uso: NATS JetStream para control‑plane y telemetría de baja latencia y entrega configurable; Apache Kafka para streams con gran volumen histórico y necesidades de replay y procesamiento batch con offsets. Redis Streams se reserva para casos puntuales y no como bus principal del control‑plane, por sus garantías y complejidad operativa en este dominio.

Tabla 2. Comparativa NATS vs Kafka vs Redis Streams

| Criterio | NATS (JetStream) | Apache Kafka | Redis Streams |
|---|---|---|---|
| Latencia | Muy baja, optimizada | Media por batch | Baja (variable) |
| Throughput | Millones de msg/s | Millones de rec/s | Alto (dependiente) |
| Garantías | At‑most/at‑least/exactly | At‑least/exactly | At‑least (según config) |
| Modelo | Subjects jerárquicos | Topics/particiones | Streams + consumer groups |
| Replay histórico | Limitado (config) | Nativo, offsets | Nativo (trim/persist limitada) |
| Complejidad operativa | Baja‑media | Media‑alta | Media |
| Casos ideales | Control‑plane, telemetría | Analítica, audit histórico | Caching, colas ligeras |

Fuente de comparación técnica y posicionamiento relativo: Synadia, NATS y Kafka comparados[^4].

La semántica de entrega se configura por tipo de evento:

Tabla 3. Semánticas de entrega y configuración

| Tipo | Semántica | Configuración | Notas |
|---|---|---|---|
| JobRequested, AssignmentCreated | At‑least‑once | Acks + reintentos | Idempotencia por JobId/AssignmentId |
| WorkerCreated/Terminated/Failed | At‑least‑once | Durable + reintentos | Reconciliación de estado |
| StepStarted/Completed/Failed | At‑least‑once | Durable + dedup | Diagnóstico ejecución |
| MetricEmitted | At‑most/at‑least | JetStream (según criticidad) | Latencia vs confiabilidad |
| Audit crítico (Auth*, ArtifactValidated) | Exactly‑once | Transacciones producer/consumer | Deduplicación estricta |

El esquema de subjects jerárquicos se organiza por contexto y entidad:

Tabla 4. Subjects NATS por contexto y versión

| Subject pattern | Versión | Ejemplo de evento |
|---|---|---|
| ci.job.v1.requested | v1 | JobRequested |
| ci.job.v1.started | v1 | JobStarted |
| ci.job.v1.completed | v1 | JobCompleted |
| ci.offer.v1.created | v1 | OfferCreated |
| ci.assignment.v1.created | v1 | AssignmentCreated |
| ci.worker.v1.created | v1 | WorkerCreated |
| ci.step.v1.started | v1 | StepStarted |
| ci.metric.v1.emitted | v1 | MetricEmitted |

Los schemas de eventos se versionan con compatibilidad hacia atrás:

Tabla 5. Campos mínimos por evento y reglas de evolución

| Evento | Campos mínimos | Versión | Reglas de compatibilidad |
|---|---|---|---|
| JobRequested | job_id, spec_hash, correlation_id, tenant_id, timestamp | v1 | Añadir campos opcionales; no romper existentes |
| JobStarted | job_id, worker_id, timestamp | v1 | Permitir metadata adicional opc. |
| AssignmentCreated | assignment_id, job_id, node_id, policy, timestamp | v1 | Nuevos atributos deben ser opcionales |
| WorkerCreated | worker_id, provider_id, runtime_spec, timestamp | v1 | No modificar tipos base |

Event sourcing y proyecciones. Se aplicará event sourcing en Orchestration y Security para auditoría y reconstrucción de estado: los eventos se almacenan con retención por tipo (por ejemplo, Job* ≥ 90 días, Security ≥ 180 días), y se construyen proyecciones (read models) para consultas rápidas de la Consola y diagnósticos. La retención y el replay permiten reindexación y recuperación ante incidentes; el diseño requiere gobernanza de versiones y pruebas contractuales para evitar incompatibilidades[^2].

Outbox pattern. Para garantizar entrega y evitar dobles publicaciones en el bus, cada servicio implementa outbox transaccional: los eventos se写入 en un almacenamiento local transaccional y se publican de manera atómica mediante un dispatch confiable. En caso de fallo, se aplican reintentos y deduplicación por claves de idempotencia.

Event replay y time‑travel. Se habilitan mecanismos de replay controlado por ventana temporal y offset; la reconstrucción de estado se realiza desde snapshots + streams de eventos. Time‑travel se utiliza para auditoría con snapshots verificados, asegurando integridad temporal.

## Protocolos de comunicación (gRPC/HTTP2, WebSocket/SSE, REST, Message Queues, protobuf)

La plataforma combina protocolos según el patrón de interacción:

- gRPC/HTTP2 para llamadas síncronas de alto rendimiento entre servicios (por ejemplo, Orquestador ↔ Planificador ↔ Worker Manager), con timeouts, cancelación y streaming bidireccional en casos específicos.
- WebSockets/SSE para streaming hacia la Consola (estado de jobs, métricas en vivo, eventos de ejecución).
- REST (OpenAPI) para operaciones CRUD y endpoints de control con semánticas claras; versionado compatible hacia atrás.
- Message queues (NATS/Kafka) para comunicación asíncrona de eventos.
- Protocol Buffers para serialización binaria eficiente de eventos y DTOs, con esquemas centralizados.

Tabla 6. Matriz protocolo vs caso de uso

| Caso | Protocolo | Semántica | SLA orientativo |
|---|---|---|---|
| Crear job | REST (HTTP/1.1) + evento | Idempotente | p95 < 300 ms |
| Solicitar asignación | gRPC + evento | Idempotente | p95 < 500 ms |
| Publicar estado (Job*) | NATS | At‑least‑once | p95 < 1 s |
| Streaming logs a UI | WS/SSE | Tiempo real | p95 < 1 s inicio |
| Telemetría | NATS | At‑least/at‑most | p95 < 1 s |

Para gRPC, se define un catálogo de servicios y métodos:

Tabla 7. Catálogo de servicios/métodos gRPC

| Servicio | Método | Request/Response | Timeouts |
|---|---|---|---|
| SchedulerService | RequestAssignment | JobSpec → AssignmentResponse | 500 ms |
| WorkerService | CreateWorker | RuntimeSpec → WorkerId | 3 s |
| WorkerService | TerminateWorker | WorkerId → Status | 3 s |

Las APIs REST se publican con OpenAPI, reforzando contratos, compatibilidad y pruebas automatizadas[^14]. La elección de HTTP/1.1 para REST (vs HTTP/2) es pragmática para compatibilidad con ecosistemas estándar, mientras que gRPC/HTTP2 atiende rutas internas de alto rendimiento; ambas conviven en la arquitectura[^11][^14].

## Streaming de métricas y observabilidad (Prometheus, OpenTelemetry, ELK/Grafana, alertas)

La observabilidad integra métricas en tiempo real con Prometheus; trazabilidad distribuida con OpenTelemetry; logs estructurados en ELK; y alertas inteligentes con PagerDuty/Slack. Los puntos de instrumentación se definen en Orquestador, Planificador, Worker Manager y Execution, con etiquetas obligatorias para correlación (tenant, job_id, step_id).

Tabla 8. Métricas por componente

| Componente | Métrica | Tipo | Tags obligatorios |
|---|---|---|---|
| Orquestador | orc_job_latency_ms | histogram | tenant, job_kind |
| Planificador | sch_assignment_ms | histogram | policy, pool |
| Worker Manager | wkm_create_worker_ms | histogram | provider, namespace |
| Execution | exec_step_duration_ms | histogram | step_type |
| Telemetría | tel_queue_depth | gauge | subject |

La retención por tipo de dato y semántica de entrega se resume en la Tabla 9.

Tabla 9. Retención por tipo y semánticas

| Tipo | Retención | Semántica | Uso |
|---|---|---|---|
| Eventos de negocio (Job*, Step*) | ≥ 30–90 días | At‑least‑once | Auditoría operativa |
| Trazas (TraceSpans) | Corto‑medio plazo | At‑least‑once | Diagnóstico distribuido |
| Métricas operativas | Minutos/horas | At‑least/at‑most | Observabilidad/alertas |
| Seguridad (Auth*) | ≥ 180 días | Exactly‑once | Compliance |

El plan de alertas define condiciones, severidades y supresiones:

Tabla 10. Plan de alertas (PagerDuty/Slack)

| Condición | Severidad | Canal | Supresión |
|---|---|---|---|
| p95 latencia > SLO (create_job) | warning | Slack | 10 min |
| Tasa de fallos > 5% (jobs) | critical | PagerDuty | 5 min |
| Saturación de colas (scheduling) | warning | Slack | 15 min |
| Brechas de autorización | info | Slack | N/A |

Los dashboards de Grafana exponen latencias p95, throughput por servicio y profundidad de colas, con vistas por tenant y entorno. La correlación de trazas se realiza con TraceContext y se propaga en headers y eventos[^4][^13].

## APIs para Workers efímeros (lifecycle, jobs, health, artifact upload/download, log streaming)

Los workers efímeros exponen endpoints estandarizados y publish eventos de ciclo de vida. El Worker Manager orquestará creación/terminación y habilitará port‑forward o streaming de logs seguro. Artifact Management se integra para descargas/ subidas con validación de checksums.

Tabla 11. Endpoints de ejecución y semántica

| Método | Ruta | Semántica | Idempotencia | Autenticación |
|---|---|---|---|---|
| POST | /exec | Ejecutar step | Sí, por StepId | Token OIDC + RBAC |
| GET | /logs/{exec_id} | Streaming de logs | N/A | Token OIDC |
| POST | /complete | Finalizar step | Sí, por ExecId | Token OIDC |

El ciclo de vida del worker se describe en la Tabla 12.

Tabla 12. Ciclo de vida del worker y eventos

| Estado | Evento | Transición | Cleanup |
|---|---|---|---|
| Creating | WorkerCreated | → Running | N/A |
| Running | StepStarted/Completed/Failed | N/A | Limpieza parcial |
| Terminating | WorkerTerminated | → Terminated | Trash removal |
| Failed | WorkerFailed | → Terminated | Quarantine/logs |

El Worker Manager expone operaciones CRUD a providers Kubernetes/Docker; los contratos y endpoints se documentan con OpenAPI, alineando la interfaz de ejecución con el resto de la plataforma[^5][^14].

## Manejo de fallos (circuit breaker, bulkhead, retries/backoff, graceful degradation, DLQ)

Se aplican patrones de resiliencia en servicios y dependencias:

- Circuit Breaker en llamadas a providers (K8s/Docker) y brokers; evita cascadas de fallos y habilita recuperación con half‑open.
- Bulkhead isolation en pools de conexiones y colas por dominio; contiene impacto y permite degradación elegante.
- Retry/Timeout con backoff exponencial y límites por categoría (transitorio vs persistente).
- Graceful degradation: reducción de funcionalidad no crítica ante presión (por ejemplo, desactivar ciertas visualizaciones en la Consola).
- Dead Letter Queue (DLQ) para eventos que no pueden procesarse tras reintentos, con reproceso manual/automatizado y auditoría.

Tabla 13. Matriz de resiliencia por dependencia

| Dependencia | Patrones | Umbrales | Observabilidad |
|---|---|---|---|
| K8s/Docker | Circuit Breaker, Retry, Bulkhead | Error rate > 5% en 1 min | Tasa fallos, latencia create/terminate |
| NATS/Kafka | Retry, Backpressure, DLQ | p95 latencia evento > SLO | Profundidad de cola, acks |
| AVP/Security | Cache denegaciones, Timeout | Error rate > 2% en 5 min | Decisiones Permit/Deny |

Tabla 14. Catálogo de errores y reintentos

| Código | Categoría | Backoff | Límite |
|---|---|---|---|
| WKM‑PROV | Infraestructura (provider) | exponencial | 5 intentos |
| ORC‑INFRA | Infraestructura (bus/storage) | exponencial | 7 intentos |
| SCH‑CAP | Capacidad | fijo | 3 intentos |
| SEC‑TOK | Token inválido | no reintentar | N/A |

La aplicación de estos patrones sigue recomendaciones de diseño para microservicios tolerantes a fallos[^2][^13].

## Escalabilidad y rendimiento (horizontal scaling, load balancing, connection pooling, backpressure, memoria)

El escalado horizontal se aplica por servicio, con autonomía de storage (database‑per‑service) y desacoplamiento temporal mediante eventos. Se emplean load balancers a nivel de clúster/servicio, connection pooling en clientes gRPC/HTTP y límites de concurrencia por actor/cola. El backpressure se instrumenta en ingestión de eventos y colas de scheduling para evitar saturación; las proyecciones y snapshots optimizan memoria y latencia.

Tabla 15. KPIs y objetivos

| KPI | Objetivo | Comentario |
|---|---|---|
| Latencia p95 create_job | < 300 ms | Orquestador |
| Latencia p95 assignment | < 500 ms | Planificador |
| Throughput jobs/min | Definir por entorno | Escalado horizontal |
| Profundidad de cola | < umbral por servicio | Backpressure |
| Éxito create/terminate | > 99.5% | Worker Manager |

Tabla 16. Políticas de backpressure y límites

| Servicio | Límite de cola | Acción al exceder |
|---|---|---|
| Planificador | Longitud N | Drossel JobRequested |
| Telemetría | Depth > X | Sampling controlado |
| Worker Manager | Pending creates | Encolar + breaker |

La correlación de trazas y métricas se preserva bajo escalado, facilitando diagnósticos end‑to‑end[^2][^13].

## Implementación en Rust (Tokio/Actix, Crossbeam, protobuf, error handling, observabilidad)

El runtime asíncrono elegido es Tokio; el modelo de actores con Tokio/Actix encapsula estado y comportamiento. Crossbeam se emplea para concurrencia avanzada (colas lock‑free y estructuras de datos concurrentes). Protocol Buffers definen mensajes de eventos y DTOs; se centralizan esquemas y se gestionan con versionado. El error handling se tipifica por contexto, con categorización clara (validación, autorización, infraestructura, estado) y políticas de retry/backoff. La observabilidad se integra con tracing y OpenTelemetry.

Tabla 17. Mapa crates/librerías por función

| Función | Librería/patrón | Justificación |
|---|---|---|
| HTTP/API | Actix‑web/Axum | Alto rendimiento y ergonomía[^11] |
| Mensajería | nats.crate + JetStream | Baja latencia, subjects[^4] |
| Actors | Tokio + Actix | Madurez, supervisión[^10] |
| Kubernetes | kube‑rs, k8s‑openapi | Cliente, watchers, CRDs[^5] |
| Serialización | protobuf + Serde | Compatibilidad y eficiencia |
| Observabilidad | tracing + OpenTelemetry | Trazas y métricas |

La coherencia del stack y la elección de framework se alinean con buenas prácticas y comparativas recientes[^11][^10].

## Seguridad y multi‑tenancy (Keycloak OIDC/OAuth2, AWS Verified Permissions, mTLS, audit)

La autenticación se basa en OpenID Connect (OIDC) con Keycloak para usuarios humanos y service accounts machine‑to‑machine; la autorización policy‑based se implementa con AWS Verified Permissions (AVP), evaluando decisiones sobre Principal–Resource–Action–Context. mTLS garantiza autenticación mutua en comunicaciones internas; la auditoría distribuye eventos de seguridad (AuthGranted, AuthRevoked, AuthorizationDenied) y se correlaciona con jobs/pipelines.

Tabla 18. Matriz de políticas por rol/servicio

| Rol/Servicio | Caso | Política |
|---|---|---|
| Orchestration | create_job | Permit si scope “job:create” y tenant coincide |
| Scheduling | assign_job | Permit si scope “schedule:assign” y recursos disponibles |
| Worker Manager | create_worker | Permit si scope “worker:create” y provider autorizado |
| Execution | run_step | Permit si scope “exec:run” y entorno permitido |
| Artifact | upload | Permit si scope “artifact:upload” y checksum válido |

La retención de auditoría cumple con políticas corporativas; las decisiones se auditan y exportan a SIEM conforme a la tabla de retención (Security ≥ 180 días; Jobs ≥ 90 días).

Tabla 19. Retención de auditoría por evento

| Evento | Retención | Export |
|---|---|---|
| AuthGranted/Denied | ≥ 180 días | SIEM |
| JobCompleted/Failed | ≥ 90 días | Data lake |
| ArtifactValidated | ≥ 90 días | Data lake |

La integración de seguridad se alinea con el análisis de dominios y decisiones de arquitectura, manteniendo trazabilidad completa[^1][^13].

## Roadmap de implementación y criterios de aceptación

La hoja de ruta organiza entregables incrementales con métricas objetivo, reduciendo riesgo y agregando valor temprano.

Tabla 20. Hitos y métricas objetivo

| Hito | Entregables | SLOs |
|---|---|---|
| MVP Orquestador | REST /jobs, /pipelines; eventos base | p95 create_job < 300 ms |
| K8s Integrado | Worker Manager (pods/logs) | Éxito create/terminate > 99.5% |
| Scheduling | Capacidades/offers; FIFO/packing | p95 assignment < 500 ms |
| Telemetría | Métricas, trazas; streaming | Cobertura eventos 100% |
| Consola/BFF | Dashboards SSE/WS; RBAC | p95 consulta UI < 400 ms; uptime 99.9% |

Los criterios de aceptación incluyen latencias p95, throughput, éxito de jobs, cobertura de telemetría, cumplimiento de contratos OpenAPI y operación estable del bus (NATS/Kafka). La gobernanza se asegura con pruebas contractuales de eventos y revisión de cambios[^12][^4].

## Riesgos, supuestos y mitigaciones

Los principales riesgos se muestran en la Tabla 21.

Tabla 21. Riesgos y mitigaciones

| Riesgo | Impacto | Probabilidad | Mitigación |
|---|---|---|---|
| Operación del bus (NATS/Kafka) | Medio | Media | Automatización; observabilidad; runbooks |
| Consistencia eventual | Medio | Alta | Idempotencia; Sagas; auditoría |
| Mezcla de runtimes async | Alto | Media | Tokio único; pruebas integración[^11] |
| Saturación scheduling | Alto | Media | Backpressure; autoscaling; políticas |
| Seguridad multi‑tenant | Alto | Media | RBAC; segmentación; secretos |
| Falla de provider | Alto | Baja‑Media | Circuit Breaker; fallback; retry |

Supuestos operativos: disponibilidad de clusters Kubernetes, gestión centralizada de secretos y RBAC; selección única de runtime asíncrono; métricas de latencia y throughput del bus instrumentadas desde el inicio.

## Anexos (catálogos y especificaciones)

Este anexo consolida catálogos de endpoints, eventos y métricas para consulta operativa.

Tabla 22. Catálogo consolidado de endpoints internos

| Servicio | Método | Ruta | Semántica | Idempotencia |
|---|---|---|---|---|
| Orquestador | POST | /jobs | Crear job | Sí, por JobId |
| Orquestador | GET | /jobs/{id} | Consultar estado | N/A |
| Orquestador | POST | /jobs/{id}/cancel | Cancelar job | Sí, por JobId |
| Planificador | POST | /schedule | Solicitar asignación | Sí, por JobId |
| Planificador | GET | /nodes | Capacidades y ofertas | N/A |
| Worker Manager | POST | /workers | Crear worker | Sí, por WorkerId |
| Worker Manager | DELETE | /workers/{id} | Terminar worker | Sí, por WorkerId |
| Worker Manager | GET | /workers/{id}/logs | Obtener logs | N/A |
| Workers | POST | /exec | Ejecutar step | Sí, por StepId |
| Telemetría | GET | /metrics | Consultar métricas | N/A |
| Telemetría | GET | /traces | Consultar trazas | N/A |
| Consola | GET | /ui/jobs | Agregación de datos | N/A |

Tabla 23. Catálogo de eventos NATS (subject, payload, idempotency key)

| Subject | Evento | Payload mínimo | Idempotency key |
|---|---|---|---|
| ci.job.v1.requested | JobRequested | job_id, spec_hash, correlation_id, tenant_id, timestamp | job_id |
| ci.job.v1.started | JobStarted | job_id, worker_id, timestamp | job_id+worker_id |
| ci.job.v1.completed | JobCompleted | job_id, result_ref, timestamp | job_id |
| ci.job.v1.failed | JobFailed | job_id, error_code, retryable, timestamp | job_id |
| ci.offer.v1.created | OfferCreated | offer_id, node_id, resource_quota, timestamp | offer_id |
| ci.assignment.v1.created | AssignmentCreated | assignment_id, job_id, node_id, policy, timestamp | assignment_id |
| ci.worker.v1.created | WorkerCreated | worker_id, provider_id, runtime_spec, timestamp | worker_id |
| ci.worker.v1.terminated | WorkerTerminated | worker_id, exit_code, timestamp | worker_id |
| ci.step.v1.started | StepStarted | step_id, job_id, exec_id, timestamp | step_id+exec_id |
| ci.step.v1.completed | StepCompleted | step_id, result_ref, duration_ms, timestamp | step_id+result_ref |
| ci.step.v1.failed | StepFailed | step_id, error_code, retryable, timestamp | step_id |
| ci.metric.v1.emitted | MetricEmitted | metric_name, value, tags, timestamp | metric_name+tags+ts |
| ci.trace.v1.spans | TraceSpans | trace_context, spans, timestamp | trace_id |
| ci.alert.v1.raised | AlertRaised | alert_id, severity, message, timestamp | alert_id |
| ci.security.v1.auth.granted | AuthGranted | principal, scope, decision_id, timestamp | decision_id |
| ci.security.v1.auth.revoked | AuthRevoked | principal, scope, decision_id, timestamp | decision_id |
| ci.security.v1.auth.denied | AuthorizationDenied | resource, action, reason, timestamp | resource+action |
| ci.artifact.v1.uploaded | ArtifactUploaded | artifact_ref, checksum, timestamp | artifact_ref |
| ci.artifact.v1.promoted | ArtifactPromoted | artifact_ref, env, timestamp | artifact_ref+env |
| ci.artifact.v1.validated | ArtifactValidated | artifact_ref, status, timestamp | artifact_ref |

Tabla 24. Métricas por servicio y etiquetas

| Servicio | Métrica | Tipo | Tags |
|---|---|---|---|
| Orquestador | orc_job_latency_ms | histogram | tenant, job_kind |
| Planificador | sch_assignment_ms | histogram | policy, pool |
| Worker Manager | wkm_create_worker_ms | histogram | provider, namespace |
| Execution | exec_step_duration_ms | histogram | step_type |
| Telemetría | tel_queue_depth | gauge | subject |
| Consola | ui_latency_ms | histogram | endpoint, tenant |

Las especificaciones OpenAPI y protobuf versionadas son la base de la compatibilidad y la validación automatizada de contratos[^14].

## Lagunas de información (information gaps) y próximos pasos

Se reconocen las siguientes áreas pendientes que condicionan decisiones operativas finas:

- Benchmarks comparativos de latencia y throughput entre NATS JetStream y Apache Kafka para escenarios CI/CD específicos (construcción, paralelización, fan‑out) y su impacto en SLOs.
- Detalles operativos de autenticación/autorización multi‑tenant, gestión de secretos y mapeo de roles por proveedor, con la integración concreta Keycloak + AWS Verified Permissions.
- Especificación de CRDs o recursos extendidos para jobs CI/CD en Kubernetes y su ciclo de vida (contratos detallados).
- Estrategias de serialización binaria y diseños zero‑copy para transferencia de artefactos y logs, incluyendo límites de tamaño y codecs.
- Políticas de scheduling avanzadas (preemptible jobs, pools de GPU, afinidad por región/zone) con métricas y SLOs precisos.

Próximos pasos: ejecutar campañas de benchmarks en entornos controlados; definir CRDs y validar contratos con operadores; afinar políticas de scheduling y almacenamiento binario; completar integración y pruebas de seguridad multi‑tenant.

---

## Referencias

[^1]: Microsoft Learn — Azure Architecture Center. “Domain analysis for microservices.” https://learn.microsoft.com/en-us/azure/architecture/microservices/model/domain-analysis  
[^2]: microservices.io. “Pattern: Microservice Architecture.” https://microservices.io/patterns/microservices.html  
[^3]: Bits and Pieces. “Understanding the Bounded Context in Microservices.” https://blog.bitsrc.io/understanding-the-bounded-context-in-microservices-c70c0e189dd1  
[^4]: Synadia. “NATS and Kafka Compared.” https://www.synadia.com/blog/nats-and-kafka-compared  
[^5]: Shuttle.dev. “Using Kubernetes with Rust.” https://www.shuttle.dev/blog/2024/10/22/using-kubernetes-with-rust  
[^6]: Kubernetes Documentation. “Kubectl.” https://kubernetes.io/docs/tasks/tools/#kubectl  
[^7]: Martin Fowler. “Bounded Context.” https://www.martinfowler.com/bliki/BoundedContext.html  
[^8]: Docker Docs. “Test your Rust deployment.” https://docs.docker.com/guides/rust/deploy/  
[^9]: Kubernetes Documentation. “Efficient detection of changes (watchers).” https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes  
[^10]: Ryuichi. “Actors with Tokio.” https://ryhl.io/blog/actors-with-tokio/  
[^11]: Luca Palmieri. “Choosing a Rust web framework, 2020 edition.” https://www.lpalmieri.com/posts/2020-07-04-choosing-a-rust-web-framework-2020-edition/  
[^12]: Kubernetes Documentation. “Efficient detection of changes (watchers).” https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes  
[^13]: Microsoft Learn — Azure Architecture Center. “Microservices architecture style.” https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices  
[^14]: OpenAPI Specification. “Latest.” https://spec.openapis.org/oas/latest.html