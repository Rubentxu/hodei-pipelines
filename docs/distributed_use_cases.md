# Casos de uso distribuidos por componente en una plataforma CI/CD en Rust

## 1. Propósito, alcance y supuestos (What y Why)

Este documento transforma el análisis de arquitectura distribuida y el modelo de dominio ya consolidados en una especificación operativa y verificable de casos de uso para cada componente: Orquestador, Planificador, Worker Manager, Telemetría y Seguridad. El alcance cubre contratos, eventos, flujos end‑to‑end, patrones de resiliencia, Sagas de compensación y trazabilidad. Se apoya en los bounded contexts definidos (Orchestration, Scheduling, Worker Management, Execution, Telemetry, Security y Artifact Management), y en principios de arquitectura de microservicios como database‑per‑service, idempotencia, backpressure y consistencia eventual gestionada con Sagas[^1][^2][^3].

La narrativa avanza desde los contratos y eventos que definen cada caso de uso, hacia los flujos de alto nivel que orquestan jobs y pipelines, y termina en el cómo técnico: decisiones de diseño, patrones de resiliencia y especificación Rust (traits async, error handling, CQRS y event sourcing). La métrica guía el éxito: latencia p95, throughput, éxito de jobs y cobertura de telemetría. Dado que existen dependencias operativas aún no fijadas (benchmarks de NATS vs Kafka; detalles multi‑tenant), se reconocen lagunas y se proponen decisiones condicionadas.

Supuestos principales:
- NATS JetStream se emplea para control‑plane y telemetría por baja latencia y entrega configurable; Kafka queda como opción para replay histórico y alto volumen. Las políticas de idempotencia aseguran deduplicación en todos los consumidores[^4].
- La autorización policy‑based opera sobre un modelo Principal–Resource–Action–Context y se instrumenta con decisiones auditables; su integración concreta multi‑tenant se refinará en fases posteriores[^1].
- La ejecución se realiza en workers efímeros con aislamiento por provider (Kubernetes/Docker) y limpieza automática; los workers publican Step* con correlación de TraceContext.

Para anclar la relación entre casos de uso y contextos, el siguiente mapa sintetiza qué servicios los materializan y qué eventos se emplean.

Tabla 1. Mapa de casos de uso por bounded context y eventos asociados

| Bounded Context | Caso de uso | Servicio materializador | Eventos clave |
|---|---|---|---|
| Orchestration | Coordinación de pipelines, DR, cancelaciones | Orquestador | JobRequested, JobStarted, JobCompleted, JobFailed, JobCancelled |
| Scheduling | Evaluación de recursos, distribución, rebalanceo, predicción, costos | Planificador | OfferCreated, AssignmentCreated, AssignmentRejected, CapacityUpdated |
| Worker Management | Provisión/terminación, pools, SA/credenciales | Worker Manager | WorkerCreated, WorkerTerminated, WorkerFailed |
| Execution | Streaming de logs, métricas por step, cleanup | Worker/Execution Context | StepStarted, StepCompleted, StepFailed |
| Telemetry | Métricas/trazas en tiempo real, alertas, compliance | Telemetría | MetricEmitted, TraceSpans, AlertRaised |
| Security | Autenticación, autorización policy‑based, mTLS, audit trail | Seguridad | AuthGranted, AuthRevoked, AuthorizationDenied |

Este mapa guía contratos, límites de idempotencia y políticas de resiliencia, y establece la semántica de entrega por caso, con base en patrones de microservicios[^2][^1][^3].


## 2. Metodología de especificación y criterios de calidad (How)

Cada caso de uso se especifica mediante:
- Precondiciones: identidad, scopes, contexto de tenancy y política de idempotencia (keys).
- Pasos: comandos y eventos, correlacionados con TraceContext y deduplicados por IDs (JobId, StepId, OfferId, AssignmentId).
- Postcondiciones: efectos sobre agregados, compensación (si aplica) y auditoría.
- Errores e idempotencia: clasificación por dominio (validación, autorización, infraestructura, estado) y estrategias de retry/backoff.
- Métricas: latencias p95, throughput, tasas de fallo, cobertura de eventos.

La trazabilidad se propaga mediante headers y eventos; cada evento incluye correlation_id, tenant_id y TraceContext. La calidad se prueba con contratos OpenAPI y tests de eventos (contract tests), asegurando compatibilidad hacia atrás y gobernanza de cambios[^14]. Los errores se representan con tipos específicos por contexto, evitando ambigüedades y simplificando auditoría.

Tabla 2. Catálogo de errores estándar por contexto

| Contexto | Código | Categoría | Acción recomendada |
|---|---|---|---|
| Orchestration | ORC‑VALID | Validación (JobSpec/PipelineSpec) | Rechazar comando; retornar errores de invariantes |
| Orchestration | ORC‑AUTH | Autorización | Denegar; auditar AuthorizationDenied |
| Orchestration | ORC‑INFRA | Infraestructura (bus/storage) | Retry con backoff; circuit breaker si persistente |
| Orchestration | ORC‑STATE | Estado inválido de Job/Pipeline | Reintentar transición o compensaciones (Saga) |
| Scheduling | SCH‑CAP | Capacidad insuficiente | Emitir OfferCreated con detalles; reintento |
| Scheduling | SCH‑POL | Política no aplicable | Rechazar AssignmentRejected; replanificar |
| Worker Mgmt | WKM‑PROV | Provider error | Retry limitado; fallback a otro provider |
| Worker Mgmt | WKM‑CREDS | Credenciales expiradas | Rotar secret; reintentar |
| Telemetry | TEL‑DROP | Overflow/presión | Activar backpressure; sampling controlado |
| Security | SEC‑TOK | Token inválido/expirado | Reautenticación; invalidar sesión |
| Security | SEC‑AVP | Decisión deny | Auditar; devolver reason/obligations |
| Artifact | ART‑CHK | Checksum inválido | Rechazar upload; registrar alerta |


## 3. Contratos y catálogo de eventos globales (What)

Las APIs se publican con OpenAPI, versionadas y compatibles hacia atrás; los eventos usan subjects jerárquicos en NATS, con payloads mínimos y claves de idempotencia. La semántica de entrega se configura por caso: at‑most‑once para señales transitorias, at‑least‑once para eventos operativos, exactly‑once para auditoría crítica[^14][^4].

Tabla 3. Catálogo de eventos NATS por subject

| Subject | Evento | Payload mínimo | Idempotency key |
|---|---|---|---|
| ci.job.requested | JobRequested | JobId, Spec, CorrelationId, TenantId | JobId |
| ci.job.started | JobStarted | JobId, WorkerId | JobId+WorkerId |
| ci.job.completed | JobCompleted | JobId, ResultRef | JobId |
| ci.job.failed | JobFailed | JobId, ErrorCode, Retryable | JobId |
| ci.job.cancelled | JobCancelled | JobId, Reason | JobId |
| ci.offer.created | OfferCreated | OfferId, NodeId, ResourceQuota | OfferId |
| ci.assignment.created | AssignmentCreated | AssignmentId, JobId, NodeId, Policy | AssignmentId |
| ci.assignment.rejected | AssignmentRejected | AssignmentId, Reason | AssignmentId |
| ci.node.capacity.updated | CapacityUpdated | NodeId, ResourceQuota, Timestamp | NodeId+Timestamp |
| ci.worker.created | WorkerCreated | WorkerId, ProviderId, RuntimeSpec | WorkerId |
| ci.worker.terminated | WorkerTerminated | WorkerId, ExitCode | WorkerId |
| ci.worker.failed | WorkerFailed | WorkerId, Reason | WorkerId |
| ci.step.started | StepStarted | StepId, JobId, ExecId | StepId+ExecId |
| ci.step.completed | StepCompleted | StepId, ResultRef, DurationMs | StepId+ResultRef |
| ci.step.failed | StepFailed | StepId, ErrorCode, Retryable | StepId |
| ci.metric.emitted | MetricEmitted | MetricName, Value, Tags | MetricName+Tags+ts |
| ci.trace.spans | TraceSpans | TraceContext, Spans | TraceContext.trace_id |
| ci.alert.raised | AlertRaised | AlertId, Severity, Message | AlertId |
| ci.security.auth.granted | AuthGranted | Principal, Scope, DecisionId | DecisionId |
| ci.security.auth.revoked | AuthRevoked | Principal, Scope, DecisionId | DecisionId |
| ci.security.auth.denied | AuthorizationDenied | Resource, Action, Reason | Resource+Action |
| ci.artifact.uploaded | ArtifactUploaded | ArtifactRef, Checksum | ArtifactRef |
| ci.artifact.promoted | ArtifactPromoted | ArtifactRef, Env | ArtifactRef+Env |
| ci.artifact.validated | ArtifactValidated | ArtifactRef, Status | ArtifactRef |

Tabla 4. Catálogo de endpoints internos

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

Estos contratos son el cimiento de la consistencia operativa y la trazabilidad end‑to‑end, y se gobiernan mediante OpenAPI[^14] con semánticas de entrega del bus[^4].


## 4. Casos de uso del Orquestador (Coordinación general)

El Orquestador materializa el bounded context de Orchestration y gobierna el ciclo de vida de jobs y pipelines, reconcilia estados y aplica Sagas de compensación. Sus casos de uso críticos: coordinar pipelines multi‑etapa, gestionar escalado automático, operar health checks, ejecutar failover/DR y manejar workflows complejos. Todos los comandos son idempotentes por JobId; los eventos usan CorrelationId y TenantId; las métricas capturan latencia de orquestación y duración por step.

### 4.1 Coordinación general de pipelines

El flujo se inicia con JobRequested; el Orquestador solicita asignación al Planificador, confirma la creación del worker y publica JobStarted. Al finalizar, emite JobCompleted o JobFailed, y activa Sagas cuando sea necesario.

Tabla 5. Secuencia de eventos y API para un job típico

| Paso | Acción | Evento/API | Idempotencia |
|---|---|---|---|
| 1 | Crear job | POST /jobs → JobRequested | JobId |
| 2 | Solicitar asignación | POST /schedule → OfferCreated/AssignmentCreated | JobId |
| 3 | Crear worker | WorkerManager.create → WorkerCreated | WorkerId |
| 4 | Iniciar ejecución | JobStarted | JobId+WorkerId |
| 5 | Ejecutar steps | StepStarted/StepCompleted/Failed | StepId+ExecId |
| 6 | Finalizar | JobCompleted/JobFailed | JobId |
| 7 | Limpiar | WorkerTerminated | WorkerId |

La Saga de compensación entra cuando un Step falla irreversiblemente: se emiten compensaciones hacia pasos previos y recursos externos, idempotentes y auditadas.

### 4.2 Gestión de escalado automático

El Orquestador ajusta la capacidad del clúster de workers interactuando con el Planificador y Worker Manager: amplía o reduce pools dedicados, respeta constraints de scheduling y garantiza fairness entre tenants.

Tabla 6. Triggers de escalado y métricas

| Trigger | Métrica/condición | Acción | Límites |
|---|---|---|---|
| Saturación de colas | p95 cola > umbral | Ofrecer capacidad; aumentar workers | Máx. por tenant/pool |
| SLAs por tenant | Tasa éxito < objetivo | Preemptive de jobs no críticos | Quotas por tenant |
| Backpressure | Longitud de cola > N | Drossel de JobRequested | Tasa máxima controlada |
| Coste | Coste por job > presupuesto | Packing por recursos; spot/preemptible | Fallback a on‑demand |

El escalado aplica Circuit Breaker hacia providers y retry con backoff para transitorios; los eventos CapacityUpdated alimentan decisiones.

### 4.3 Health checks y monitoring

El Orquestador expone readiness/liveness, watchea eventos del bus y reconcilia estados frente a fallos del worker o del planificador.

Tabla 7. Estados de salud y transiciones

| Health | Transición | Causa | Acción |
|---|---|---|---|
| Ready | → NotReady | Dependencia indisponible | Reintentar; aislar |
| Live | → NotLive | PANIC/timeout | Reinicio; escalado |
| Degraded | → Ready | Recuperación parcial | Reanudar tráfico |
| Unknown | → Ready | Inicialización | Warm‑up completo |

### 4.4 Failover y disaster recovery

Las réplicas activas‑activa del Orquestador eligen líder por sujeto NATS; al perder quorum, se aplica failover y reconciliación de jobs en vuelo (idempotencia por JobId/StepId).

Tabla 8. RPO/RTO por caso

| Caso | RPO (datos) | RTO (servicio) | Mecanismo |
|---|---|---|---|
| Caída de worker | ≤ eventos no persistidos | p95 < 2 min | Reintentos; replanificación |
| Pérdida de broker | ≤ ventana JetStream | p95 < 10 min | Reconexión; replay |
| Failover de nodo | ≤ última confirmación | p95 < 1 min | Elección de líder |

### 4.5 Gestión de workflows complejos

Se soportan DAGs con pasos paralelos; las compensaciones se aplican por pasos con dependencias.

Tabla 9. Estados y reintentos por step

| Step | Política de retry | Backoff | Idempotency key |
|---|---|---|---|
| build | 3 intentos | exponencial | StepId |
| test | 2 intentos | lineal | StepId |
| deploy | 1 intento (manual) | fijo | StepId |

Cada paso publica Step* correlacionado; la latencia por step y la tasa de reintento se instrumentan para telemetría y SLOs[^2].


## 5. Casos de uso del Planificador (Asignación inteligente)

El Planificador materializa el bounded context de Scheduling & Capacity: evalúa recursos en tiempo real, aplica políticas de asignación, rebalancea dinámicamente, realiza predicciones básicas de capacidad y optimiza costos. Publica OfferCreated, AssignmentCreated/Rejected y CapacityUpdated, con deduplicación por OfferId/AssignmentId y backpressure en ingestión de ofertas.

### 5.1 Evaluación de recursos en tiempo real

El Planificador consolida ResourceQuota de nodos (CPU/memoria/GPU) y constraints de afinidad, actualizando CapacityUpdated.

Tabla 10. Campos de ResourceQuota y precisión

| Campo | Unidad | Frecuencia de actualización |
|---|---|---|
| cpu_m | milicores | ~1–5 s por watcher |
| memory_mb | MB | ~1–5 s por watcher |
| gpu | unidades | ~5–10 s por watcher |

La precisión se garantiza con contadores atómicos y estructuras lock‑free en rutas calientes, según patrones de microservicios[^2].

### 5.2 Distribución inteligente de carga

Políticas: FIFO por llegada, packing por recursos para densificar, afinidad por executor/zona/pool. La equidad entre tenants se controla con cuotas; el backpressure reduce la ingestión de ofertas cuando la cola de scheduling supera umbrales.

Tabla 11. Políticas y escenarios

| Política | Escenario | Beneficio | Trade‑off |
|---|---|---|---|
| FIFO | Latencia sensible | Simplicidad | packing subóptimo |
| Packing por recursos | Coste crítico | Mejor utilización | riesgo hotspots |
| Affinity (zona/executor) | Requisitos hardware | Menos churn | potenciales skews |

### 5.3 Rebalanceo dinámico de jobs

Jobs en vuelo pueden moverse mediante AssignmentRevoked, respetando límites de seguridad (p. ej., steps que no toleran interrupciones).

Tabla 12. Reglas de rebalanceo

| Condición | Límite de seguridad | Acción |
|---|---|---|
| Saturación por nodo | Step no preemptible | No mover; activar más nodos |
| Drift de capacidad | Ventana < N | Revocar assignments pendientes |
| Coste > presupuesto | Pool no crítico | Migrar a spot/preemptible |

### 5.4 Predicción de capacidad

Heurísticas de corto plazo basadas en ventanas móviles de JobRequested y CapacityUpdated, activando autoscaling del Planificador.

Tabla 13. Métricas de predicción

| Ventana | Umbral de expansión | Umbral de contracción |
|---|---|---|
| 5–15 min | p95 colas > 70% | p95 colas < 30% |
| 15–60 min | tasa de llegada ↑ sostenida | tasa de llegada ↓ sostenida |

### 5.5 Optimización de costos

Selección de pool (on‑demand, spot, dedicado), packing y límites de presupuesto por tenant; el trade‑off coste/latencia se modela por caso de uso.

Tabla 14. Trade‑offs por pool

| Pool | Coste | Latencia | Riesgo |
|---|---|---|---|
| Spot | Bajo | Variable | Preemption |
| On‑demand | Medio | Estable | Coste alto |
| Dedicado | Alto | Predecible | Subutilización |


## 6. Casos de uso del Worker Manager (Provisión y gestión de ejecutores)

El Worker Manager implementa el bounded context de Provider Integration y expone la abstracción WorkerManagerProvider para Kubernetes y Docker. Gestiona pools de recursos, crea/termina workers efímeros, integra watchers de Kubernetes, opera Service Accounts y rotaciones de credenciales; publica WorkerCreated/WorkerTerminated/WorkerFailed.

### 6.1 Provisión/terminación de workers efímeros

Ciclo de vida: creación, readiness, terminación y limpieza; el streaming de logs y port‑forward se habilitan por provider.

Tabla 15. Operaciones por provider

| Operación | Kubernetes | Docker | Notas |
|---|---|---|---|
| create | Pod/Job/CRD | Contenedor | Idempotente por WorkerId |
| terminate | Delete Pod/Job | Stop/rm | Garantiza cleanup |
| logs | API de logs | API de logs | Streaming seguro |
| port_forward | API nativa | CLI/API | Uso controlado |
| watch/events | Watchers | Events/stats | Baja latencia[^5][^8] |

### 6.2 Gestión de pools de recursos

Se segmentan por tenant/namespace; se definen quotas y limites; el etiquetado facilita afinidad y filtrado.

Tabla 16. Pools y políticas

| Dimensión | Política | Uso |
|---|---|---|
| Tenant/namespace | Aislamiento | Seguridad/fairness |
| Quotas | Límite por tenant | Coste/equidad |
| Labels/tolerations | Afinidad | Performance |

### 6.3 Integración con Kubernetes/Docker

Se emplea kube‑rs, watchers y CRDs para un modelo reactivo; Docker se usa como fallback o ejecución ligera.

Tabla 17. Capacidades integradas

| Capacidad | Kubernetes | Docker |
|---|---|---|
| Watchers | Sí | Estadísticas |
| CRDs | Sí | No |
| Port‑forward | Sí | Limitado |
| Secretos/RBAC | Sí | Básicos |

### 6.4 Gestión de Service Accounts

Las SA proveen identidad al worker; se aplican RBAC por namespace/tenant; se minimizan privilegios.

Tabla 18. RBAC por namespace/tenant

| Rol | Permisos | Scope |
|---|---|---|
| worker‑exec | get/logs, create/pod | namespace |
| worker‑mgr | create/delete, watch | namespace |
| operator | port‑forward | tenant |

### 6.5 Rotación de credenciales

Se rotan secretos, se invalidan tokens y se reintentan llamadas afectadas; se publican eventos de auditoría.

Tabla 19. Plan de rotación

| Elemento | Frecuencia | Acción en fallo |
|---|---|---|
| Token SA | 24 h | Reemitir; reintentar |
| Cert mTLS | 7–30 días | Revocar; circuit breaker |
| Secrets app | 7–30 días | Regenerar; rollback |


## 7. Casos de uso de Telemetría (Observabilidad y compliance)

Telemetría integra métricas en tiempo real, tracing distribuido, alertas inteligentes, performance monitoring y auditoría. Publica MetricEmitted, TraceSpans y AlertRaised; aplica retención por tipo de dato y semánticas de entrega en NATS/Kafka según necesidades[^4].

### 7.1 Métricas en tiempo real

Tipos: counters, gauges, histograms; etiquetas obligatorias (tenant, job_id, step_id); streaming hacia dashboards.

Tabla 20. Métricas por componente

| Componente | Métrica | Tipo | Tags |
|---|---|---|---|
| Orquestador | orc_job_latency_ms | histogram | tenant, job_kind |
| Planificador | sch_assignment_ms | histogram | policy, pool |
| Worker Manager | wkm_create_worker_ms | histogram | provider |
| Execution | exec_step_duration_ms | histogram | step_type |
| Telemetría | tel_queue_depth | gauge | subject |

### 7.2 Tracing distribuido

Spans por servicio y correlación por TraceContext; cada evento de negocio carrying spans para diagnósticos end‑to‑end.

Tabla 21. Spans por servicio

| Servicio | Span | Atributos | Eventos relacionados |
|---|---|---|---|
| Orquestador | create_job | tenant, spec_hash, correlation_id | JobRequested |
| Orquestador | schedule_job | job_id, node_id, policy | AssignmentCreated |
| Worker Manager | create_worker | provider_id, runtime_spec | WorkerCreated |
| Execution | run_step | exec_id, step_id, duration | StepStarted/Completed/Failed |
| Telemetría | emit_metric | name, value, tags | MetricEmitted |
| Seguridad | authorize | principal, resource, action | AuthGranted/Denied |

### 7.3 Alertas inteligentes

Severidades y reglas de agregación temporal; deduplicación y supresión por tenant/pool para evitar alertas ruidosas.

Tabla 22. Reglas de alertas

| Condición | Severidad | Acción | Supresión |
|---|---|---|---|
| p95 latencia > SLO | warning | Autoescalado | 10 min |
| Tasa de fallos > 5% | critical | Circuit breaker | 5 min |
| Saturación colas | warning | Backpressure | 15 min |
| Errores de autorización | info | Auditoría | N/A |

### 7.4 Performance monitoring

SLIs por endpoint/evento; objetivos por componente se alinean con hitos y SLOs de plataforma[^3].

Tabla 23. SLIs/SLOs por componente

| Componente | SLI | SLO objetivo |
|---|---|---|
| Orquestador | p95 create_job | < 300 ms |
| Planificador | p95 assignment | < 500 ms |
| Worker Manager | éxito create/terminate | > 99.5% |
| Telemetría | cobertura eventos | 100% eventos clave |
| Consola | p95 consulta UI | < 400 ms |

### 7.5 Audit compliance

Eventos de seguridad y autorización se correlacionan con job/pipeline; retención y export controlado.

Tabla 24. Mapa de auditoría

| Evento | Fuente | Retención | Export |
|---|---|---|---|
| AuthGranted/Security | Seguridad | ≥ 180 días | SIEM |
| AuthorizationDenied | Seguridad | ≥ 180 días | SIEM |
| JobCompleted/Failed | Orquestador | ≥ 90 días | Data lake |


## 8. Casos de uso de Seguridad (Keycloak, AWS Verified Permissions, mTLS y auditoría)

Seguridad cubre autenticación OIDC/OAuth2 con Keycloak, autorización policy‑based con AWS Verified Permissions, mTLS para comunicaciones internas y auditoría integral. El modelo de decisión evalúa Principal–Resource–Action–Context con obligations y reasons auditables[^1].

Tabla 25. Recursos y acciones para AVP

| Recurso | Acciones | Contexto |
|---|---|---|
| Job | create, read, cancel, execute, retry | tenant_id, tags, owner |
| Pipeline | create, read, update, execute | tenant_id, environment, owner |
| Worker | create, read, terminate, logs | provider_id, tenant_id, namespace |
| Node/Offer | read, schedule | zone, pool, capacity_snapshot |
| Artifact | upload, read, promote, delete | repository, environment, checksum |

Tabla 26. Flujos de autorización por endpoint

| Servicio | Endpoint | Decisión AVP | Audit |
|---|---|---|---|
| Orchestration | POST /jobs | Permit/Deny | AuthGranted/AuthorizationDenied |
| Scheduling | POST /schedule | Permit/Deny | AuthGranted/AuthorizationDenied |
| Worker Manager | POST /workers | Permit/Deny | AuthGranted/AuthorizationDenied |
| Execution | POST /exec | Permit/Deny | AuthGranted/AuthorizationDenied |
| Artifact | POST /artifacts | Permit/Deny | AuthGranted/AuthorizationDenied |

### 8.1 Autenticación con Keycloak (OIDC/OAuth2)

Se soporta Authorization Code para usuarios y Client Credentials para service accounts; scopes por tenant; rotación y revocación.

Tabla 27. Scopes por caso de uso

| Caso | Scope | Notas |
|---|---|---|
| Crear job | job:create | bound to tenant |
| Asignar job | schedule:assign | bound to pool |
| Crear worker | worker:create | bound to provider |
| Ejecutar step | exec:run | bound to environment |
| Subir artefacto | artifact:upload | bound to repo |

### 8.2 Autorización con AWS Verified Permissions

Se construyen políticas basadas en atributos (principal, recurso, contexto) y se auditan decisiones con obligations; las denegaciones incluyen reasons.

Tabla 28. Atributos por contexto

| Contexto | Atributos | Ejemplos |
|---|---|---|
| tenant | tenant_id, owner, tags | T‑123, owner=alice |
| scheduling | zone, pool, policy | eu‑west‑1a, spot |
| environment | env, criticality | prod, non‑critical |

### 8.3 mTLS certificate management

Se emiten y rotan certificados internos; se gestionan secretos y CRL; se aplica revogación en fallos.

Tabla 29. Política de rotación mTLS

| Elemento | Frecuencia | Acciones |
|---|---|---|
| Cert de servicio | 7–30 días | Reemisión; validación |
| CA interna | 6–12 meses | Rotación planificada |
| CRL | Diario | Revocar; notificar |

### 8.4 Audit trail generation

Eventos correlacionados con job/pipeline; almacenamiento y consulta auditora con retención adecuada.

Tabla 30. Catálogo de eventos de auditoría

| Tipo | Origen | Retención | Consulta |
|---|---|---|---|
| AuthGranted | Seguridad | ≥ 180 días | Por principal/tenant |
| AuthorizationDenied | Seguridad | ≥ 180 días | Por recurso/acción |
| JobCompleted/Failed | Orquestador | ≥ 90 días | Por job/pipeline |

### 8.5 Compliance reporting

Informes por periodo/tenant/entorno; controles de acceso a reportes.

Tabla 31. Plantillas de reportes

| Plantilla | Filtros | Campos |
|---|---|---|
| Seguridad | tenant, periodo | decisiones, denegaciones |
| Jobs | entorno, tasa éxito | latencia, fallos |
| Telemetría | componente | cobertura, alertas |


## 9. Interacciones entre Componentes (Event‑driven y resiliencia)

La arquitectura es event‑driven: request‑reply para operaciones síncronas, pub/sub para cambios de estado y streaming para telemetría. Se aplican patrones de resiliencia: Circuit Breaker, Bulkhead, Retry/Timeout, Saga, idempotencia y backpressure[^2][^4].

### 9.1 Event‑driven architecture

Los subjects jerárquicos por contexto y entidad permiten filtrado fino; la correlación por job/pipeline y tenant asegura trazabilidad.

Tabla 32. Sujetos jerárquicos por contexto

| Context | Subject pattern | Uso |
|---|---|---|
| Orchestration | ci.job.* | Estados de jobs |
| Scheduling | ci.offer.* / ci.assignment.* | Capacidades y decisiones |
| Worker Mgmt | ci.worker.* | Ciclo de vida de workers |
| Execution | ci.step.* | Ejecución de steps |
| Telemetry | ci.metric.* / ci.trace.* | Observabilidad |
| Security | ci.security.* | Auditoría |
| Artifact | ci.artifact.* | Artefactos |

### 9.2 Message passing patterns

Request‑reply para create/assign; pub/sub para Job* y Step*; queue groups para consumo paralelo; los consumidores deben ser idempotentes por IDs.

Tabla 33. Patrones por caso de uso

| Caso | Patrón | Notas |
|---|---|---|
| Crear job | Request‑reply (REST) + pub/sub | Idempotente por JobId |
| Asignar job | Request‑reply + pub/sub | Deduplicar por AssignmentId |
| Crear worker | Request‑reply | Circuit breaker |
| Ejecutar step | Pub/sub | Streaming logs |
| Telemetría | Pub/sub + streaming | Backpressure |

### 9.3 Async communication protocols

APIs REST + NATS; SSE/WS hacia UI; timeouts y límites de cola por servicio; límites de respuesta para evitar saturación.

Tabla 34. SLAs de latencia por endpoint/evento

| Tipo | SLA p95 | Comentario |
|---|---|---|
| REST create job | < 300 ms | Orquestador |
| REST schedule | < 500 ms | Planificador |
| Evento JobStarted | < 1 s | Control‑plane |
| StepCompleted | < 2 s | Streaming |
| MetricEmitted | < 1 s | Telemetría |

### 9.4 Error handling y recovery

Los errores se clasifican por dominio; se activan reintentos con backoff, y compensaciones en Sagas para consistencia.

Tabla 35. Matriz de errores y acciones

| Categoría | Ejemplos | Acción |
|---|---|---|
| Validación | JobSpec inválido | Rechazar; auditar |
| Autorización | AVP Deny | Auditar; denegar |
| Infraestructura | Provider timeout | Retry/backoff; breaker |
| Estado | Transición inválida | Reintentar; reconciliar |

### 9.5 Circuit breaker patterns

Se configuran umbrales por dependencia; se definen acciones half‑open; se auditan estados del breaker.

Tabla 36. Umbrales y métricas del breaker

| Dependencia | Error rate | Timeout | Acción |
|---|---|---|---|
| Provider K8s | > 5% en 1 min | 3 s | Open → half‑open |
| Broker NATS | > 3% en 5 min | 2 s | Reintentos; failover |
| AVP | > 2% en 5 min | 2 s | Cachear denegaciones |


## 10. Especificación en Rust (Traits async, modelos y patrones)

La implementación en Rust se basa en Tokio/Actix y un modelo de actores, con traits async para orquestación, scheduling, worker management, telemetría y seguridad. Los errores se tipifican por contexto; los modelos request/response y eventos siguen CQRS; se aplican event sourcing para flujos de auditoría[^10][^11][^14].

### 10.1 Async trait definitions

Los traits definen contratos por caso de uso: Orchestration, Scheduling, WorkerManagement, Telemetry y Security.

```rust
#[async_trait]
pub trait OrchestrationService: Send + Sync {
    async fn create_job(&self, spec: JobSpec, cid: CorrelationId, tid: TenantId)
        -> Result<JobId, DomainError>;
    async fn cancel_job(&self, id: JobId) -> Result<(), DomainError>;
    async fn get_job(&self, id: JobId) -> Result<JobState, DomainError>;
}

#[async_trait]
pub trait SchedulingService: Send + Sync {
    async fn request_assignment(&self, job_id: JobId, spec: &JobSpec)
        -> Result<AssignmentId, SchedulingError>;
    async fn update_capacity(&self, node_id: NodeId, quota: ResourceQuota) -> Result<(), SchedulingError>;
}

#[async_trait]
pub trait WorkerManagement: Send + Sync {
    async fn create_worker(&self, spec: &RuntimeSpec, cfg: &ProviderConfig)
        -> Result<WorkerId, ProviderError>;
    async fn terminate_worker(&self, id: WorkerId) -> Result<(), ProviderError>;
    async fn logs(&self, id: WorkerId) -> Result<LogStreamRef, ProviderError>;
}

#[async_trait]
pub trait Telemetry: Send + Sync {
    async fn emit_metric(&self, metric: Metric) -> Result<(), TelemetryError>;
    async fn emit_trace_spans(&self, ctx: TraceContext, spans: Vec<Span>) -> Result<(), TelemetryError>;
    async fn raise_alert(&self, id: AlertId, severity: Severity, msg: String) -> Result<(), TelemetryError>;
}

#[async_trait]
pub trait Security: Send + Sync {
    async fn authorize(
        &self,
        principal: &Principal,
        action: &Action,
        resource: &Resource,
        ctx: &AuthorizationContext,
    ) -> Result<AuthorizationDecision, AuthError>;
}
```

El diseño favorece aislamiento por actores y compatibilidad con Actix/Axum para HTTP, reduciendo acoplamiento accidental[^10][^11].

### 10.2 Error handling patterns

Los errores por contexto se clasifican y se mapean a eventos de auditoría y métricas; cada dominio expone un tipo de error con variantes claras.

```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("authorization error")]
    Authorization,
    #[error("infrastructure error")]
    Infrastructure,
    #[error("invalid state transition")]
    InvalidState,
}
```

La política de retry/backoff se define por categoría (infraestructura/transitorio) y se instrumenta con métricas de errores por servicio.

### 10.3 Request/response models

Los DTOs de requests/responses encapsulan JobSpec, PipelineSpec, ResourceQuota, RuntimeSpec y métricas/trazas, versionados para compatibilidad.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub spec: JobSpec,
    pub correlation_id: CorrelationId,
    pub tenant_id: TenantId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateJobResponse {
    pub job_id: JobId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssignmentResponse {
    pub assignment_id: AssignmentId,
    pub node_id: NodeId,
    pub policy: String,
}
```

Los modelos se publican en OpenAPI para contratos formales[^14].

### 10.4 Command/query separation

Commands alteran estado y generan eventos; queries leen proyecciones para UI y diagnósticos; las proyecciones se mantienen consistentes mediante eventos.

Tabla 37. Comandos y eventos por servicio

| Servicio | Comandos | Eventos |
|---|---|---|
| Orchestration | CreateJob, CancelJob | JobRequested, JobCancelled |
| Scheduling | RequestAssignment, UpdateCapacity | AssignmentCreated, CapacityUpdated |
| Worker Mgmt | CreateWorker, TerminateWorker | WorkerCreated, WorkerTerminated |
| Telemetry | EmitMetric, EmitTraceSpans | MetricEmitted, TraceSpans |
| Security | Authorize | AuthGranted, AuthorizationDenied |

### 10.5 Event sourcing patterns

La bitácora de eventos se emplea para auditoría y reconstrucción de estado; se controla la retención por tipo.

Tabla 38. Tipos de eventos y retención

| Tipo | Retención | Uso |
|---|---|---|
| Job* | ≥ 90 días | Auditoría operativa |
| Step* | ≥ 30 días | Diagnóstico ejecución |
| Auth* | ≥ 180 días | Compliance |
| Metric/Trace | corto plazo | Observabilidad |


## 11. Criterios de aceptación y KPIs (So What)

Los criterios de aceptación se anclan en los hitos del roadmap y los SLOs objetivo: latencias p95 por endpoint/evento, throughput, éxito de jobs, cobertura de telemetría y uptime. Se definen pruebas por caso de uso y resiliencia, con medición y reporte continuo[^3].

Tabla 39. Hitos, métricas y evidencia

| Hito | Métricas objetivo | Evidencia requerida |
|---|---|---|
| MVP Orquestador | p95 create_job < 300 ms | Reportes de latencia; trazas |
| K8s Integrado | éxito create/terminate > 99.5% | Logs WorkerCreated/Terminated |
| Scheduling | p95 assignment < 500 ms | Ofertas/Asignaciones |
| Telemetría | cobertura eventos 100% | Streams MetricEmitted/TraceSpans |
| Consola/BFF | p95 consulta UI < 400 ms; uptime 99.9% | Dashboards; SLIs |

Los KPIs se monitorizan en tiempo real; las desviaciones activan alertas y planes de mitigación. La gobernanza de cambios en contratos se realiza mediante OpenAPI y pruebas contractuales, preservando compatibilidad hacia atrás[^14][^3].


## 12. Riesgos, dependencias y mitigaciones

Riesgos operativos y arquitectónicos se gestionan con patrones de resiliencia, instrumentación y automatización. La operación del bus (NATS/Kafka), la consistencia eventual y la selección de runtimes asíncronos son áreas de atención. Las mitigaciones incluyen backpressure, idempotencia, Sagas, auditoría y runbooks; la latencia y throughput del bus impactan directamente SLOs y deben medirse en entornos controlados[^4][^11].

Tabla 40. Riesgos y mitigaciones

| Riesgo | Impacto | Probabilidad | Mitigación | Métrica de control |
|---|---|---|---|---|
| Configuración de NATS/Kafka | Medio | Media | Automatización; observabilidad | p95 latencia eventos |
| Consistencia eventual | Medio | Alta | Idempotencia; Sagas; audit | Tasa de duplicados |
| Mezcla de runtimes async | Alto | Media | Tokio único; pruebas | Fallos de integración |
| Saturación de scheduling | Alto | Media | Backpressure; autoscaling | Longitud de cola |
| Seguridad multi‑tenant | Alto | Media | RBAC; segmentación; secretos | Decisiones AVP |
| Falla de provider | Alto | Baja‑Media | Circuit breaker; retry | Error rate provider |

Lagunas de información reconocidas:
- Benchmarks NATS vs Kafka específicos de CI/CD y su impacto en SLOs.
- Detalles operativos de autenticación/autorización multi‑tenant (Keycloak + AVP).
- Especificación de CRDs para jobs CI/CD y su ciclo de vida.
- Serialización binaria/zero‑copy para logs/artefactos, límites y codecs.
- Políticas de scheduling avanzadas (preemptible, GPU pools, afinidad por zona) y métricas/SLOs precisos.

Estas lagunas se abordarán con experimentación, medición y definición formal en fases posteriores.


---

## Referencias

[^1]: Microsoft Learn — Azure Architecture Center. “Domain analysis for microservices.” https://learn.microsoft.com/en-us/azure/architecture/microservices/model/domain-analysis  
[^2]: microservices.io. “Pattern: Microservice Architecture.” https://microservices.io/patterns/microservices.html  
[^3]: Microsoft Learn — Azure Architecture Center. “Microservices architecture style.” https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices  
[^4]: Synadia. “NATS and Kafka Compared.” https://www.synadia.com/blog/nats-and-kafka-compared  
[^5]: Shuttle.dev. “Using Kubernetes with Rust.” https://www.shuttle.dev/blog/2024/10/22/using-kubernetes-with-rust  
[^6]: Bits and Pieces. “Understanding the Bounded Context in Microservices.” https://blog.bitsrc.io/understanding-the-bounded-context-in-microservices-c70c0e189dd1  
[^7]: Martin Fowler. “Bounded Context.” https://www.martinfowler.com/bliki/BoundedContext.html  
[^8]: Kubernetes Documentation. “Efficient detection of changes (watchers).” https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes  
[^9]: Docker Docs. “Test your Rust deployment.” https://docs.docker.com/guides/rust/deploy/  
[^10]: Ryuichi. “Actors with Tokio.” https://ryhl.io/blog/actors-with-tokio/  
[^11]: Luca Palmieri. “Choosing a Rust web framework, 2020 edition.” https://www.lpalmieri.com/posts/2020-07-04-choosing-a-rust-web-framework-2020-edition/  
[^12]: Kubernetes Documentation. “Kubectl.” https://kubernetes.io/docs/tasks/tools/#kubectl  
[^13]: OpenAPI Specification. “Latest.” https://spec.openapis.org/oas/latest.html