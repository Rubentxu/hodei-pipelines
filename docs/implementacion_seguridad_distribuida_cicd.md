# Seguridad Distribuida para CI/CD en Rust: Integración con Keycloak y AWS Verified Permissions

## 1. Resumen ejecutivo y alcance

Este blueprint define la arquitectura e implementación de una capa de seguridad distribuida para una plataforma de Integración y Despliegue Continuos (CI/CD) en Rust. El objetivo es proporcionar autenticación robusta con OpenID Connect (OIDC) y OAuth 2.0 mediante Keycloak, combinada con autorización policy‑based usando Amazon Verified Permissions (AVP) sobre un modelo de control de acceso basado en roles y atributos (RBAC/ABAC). La seguridad se refuerza con autenticación mutua TLS (mTLS) para las comunicaciones internas, aislamiento de workers efímeros, cifrado en tránsito y en reposo, y un audit trail distribuido que soporta cumplimiento y respuesta a incidentes.

El diseño se fundamenta en los bounded contexts ya establecidos: Orchestration, Scheduling, Worker Management, Execution, Telemetry y Console. Sobre ellos, se proyecta una arquitectura de seguridad que separa claramente responsabilidades, aplica el principio de mínimo privilegio y habilita trazabilidad de punta a punta, con contratos versionados y gobernanza formal de APIs y eventos. La guía se alinea con patrones de microservicios, especialmente “database per service”, idempotencia, Sagas, outbox y backpressure, para asegurar resiliencia y consistencia operacional[^2][^3].

Los principales entregables de este documento incluyen: (i) la especificación de la integración OIDC/OAuth2 con Keycloak, incluyendo service accounts y MFA; (ii) el modelo de autorización con AVP, su esquema Cedar y flujos de decisión; (iii) el diseño de mTLS y su operación (CA, emisión, rotación, pinning, CRL); (iv) el diseño de permisos granulares por componente y contexto; (v) los controles de aislamiento para workers efímeros; (vi) el plan de cifrado end‑to‑end; (vii) el audit trail y reporting de cumplimiento; (viii) la mitigación de ataques comunes en pipelines; (ix) una guía de implementación en Rust con middleware, clientes y patrones; y (x) un plan de despliegue, validación y respuesta a incidentes.

## 2. Visión general de arquitectura de seguridad distribuida

La arquitectura propuesta es event‑driven, con control‑plane y telemetría soportados por un backbone de mensajería (NATS JetStream por defecto, Kafka opcional), APIs REST versionadas y gRPC/HTTP2 para llamadas de alto rendimiento entre servicios. La seguridad se diseña en capas y por dominio, de modo que cada bounded context encapsula sus riesgos, sus decisiones de autorización y su telemetría, y se integra con los demás mediante contratos explícitos y trazables. Las decisiones clave se justifican por resiliencia, escalabilidad, trazabilidad y compatibilidad con la selección de mensajería y protocolos ya aplicada en la plataforma[^2][^4][^5].

Para situar la capa de seguridad sobre la arquitectura base, la Tabla 1 sintetiza el posicionamiento de cada componente frente a autenticación, autorización y mTLS.

Tabla 1. Mapa de componentes vs responsabilidades de seguridad

| Componente       | Autenticación (Keycloak)             | Autorización (AVP)                                | mTLS interno                         | Notas de seguridad |
|------------------|--------------------------------------|---------------------------------------------------|--------------------------------------|-------------------|
| Orchestrator     | Service account + JWT validation     | Permisos sobre jobs/pipelines (CRUD, execute)     | Client + Server                      | Sagas, idempotencia |
| Scheduler        | Service account + JWT validation     | Permisos para asignaciones (schedule, revoke)     | Client + Server                      | Backpressure, fairness |
| Worker Manager   | Service account + JWT validation     | Permisos para lifecycle workers (create, terminate, logs) | Client + Server              | Pools, RBAC por namespace |
| Workers (ephemeral) | OIDC Token en ejecución            | Permisos para steps (execute, upload logs)        | Server (exposición de endpoints)     | Aislamiento fuerte |
| Telemetry        | Service account + JWT validation     | Lecturas y publicación de métricas (emit, read)   | Client                               | Retención y semánticas |
| Console (BFF)    | OIDC usuarios (Authorization Code + PKCE) | Lecturas y acciones de UI (read, operate)    | Client                               | MFA por riesgo |

La seguridad de eventos y APIs se articula con semánticas de entrega y versionado explícito. NATS JetStream se emplea para control‑plane y telemetría con entrega configurable y baja latencia; Kafka se reserva para replays masivos y analítica histórica. La idempotencia y deduplicación por claves de negocio (JobId, StepId, AssignmentId) son obligatorias; el outbox pattern evita dobles publicaciones. Los contratos REST se publican con OpenAPI, y los eventos se gobiernan con schemas versionados y reglas de compatibilidad hacia atrás[^4][^14].

## 3. Integración con Keycloak (OIDC/OAuth2, JWT, Service Accounts, User Federation, MFA)

La autenticación se construye sobre OIDC/OAuth2 con Keycloak: usuarios humanos mediante Authorization Code con PKCE y service accounts para cargas machine‑to‑machine. Los servicios validan tokens en el gateway y en cada servicio, auditan decisiones y corrigen errores de forma explícita (token inválido, expirado o denegado). La federación de identidades y el brokering se activan cuando el contexto empresarial lo requiere (por ejemplo, integración con directorios corporativos). La MFA se aplica por riesgo del endpoint y sensibilidad de la operación, consistente con el checklist de seguridad CI/CD[^1][^5].

Tabla 2. Clientes Keycloak por componente y flujos permitidos

| Componente       | Tipo de cliente        | Flujo(s) permitido(s)                      | Scopes típicos                     | Notas |
|------------------|------------------------|--------------------------------------------|------------------------------------|-------|
| Console (BFF)    | Público/Confidencial   | Authorization Code + PKCE                  | openid, profile, email, offline_access | MFA por riesgo |
| Orchestrator     | Confidencial           | Client Credentials                         | service:orchestrator               | Service account |
| Scheduler        | Confidencial           | Client Credentials                         | service:scheduler                  | Service account |
| Worker Manager   | Confidencial           | Client Credentials                         | service:worker_mgr                 | Service account |
| Workers          | Público (con token)    | N/A (consumen tokens en ejecución)         | exec:run, artifact:upload          | Token efímero |
| Telemetry        | Confidencial           | Client Credentials                         | service:telemetry                  | Service account |

Tabla 3. Scopes por caso de uso (tenant, entorno, recurso)

| Caso de uso                   | Scope mínimo                | Contexto requerido                         |
|-------------------------------|-----------------------------|--------------------------------------------|
| Crear job                     | job:create                  | tenant_id válido                            |
| Asignar job                   | schedule:assign             | pool/zone autorizado                         |
| Crear worker                  | worker:create               | provider_id/namespace permitido             |
| Ejecutar step                 | exec:run                    | environment habilitado (prod/stage/dev)     |
| Subir artefacto               | artifact:upload             | repository permitido + checksum             |

La validación de JWT incluye verificación de firma y emisión, audiencia y expiración; el refresh se maneja de forma segura según el flujo (offline access para refresh tokens en escenarios humanos; renovación de credenciales en service accounts). La auditoría registra AuthGranted, AuthRevoked y AuthorizationDenied, y el middleware propaga el TraceContext para correlacionar decisiones con jobs/steps. La integración con providers externos se realiza mediante identity brokering y user federation; los flujos MFA (TOTP, WebAuthn) se habilitan por realm y políticas de autenticación.

### 3.1 Configuración de clientes Keycloak y scopes

La configuración por componente y entorno diferencia entre clientes públicos (PKCE) y confidenciales (service accounts). Los realms soportan MFA por políticas de autenticación y conditional auth; el mapeo de scopes por tenant/environment/resource permite decisiones de autorización contextual. Los ejemplos prácticos de flujos OAuth2/OIDC y configuración de Keycloak están ampliamente documentados y alineados con guías oficiales[^14][^5].

Tabla 4. Matriz realm vs cliente vs flujo vs MFA

| Realm  | Cliente              | Flujo                        | MFA requerida         | Observaciones |
|--------|----------------------|------------------------------|-----------------------|---------------|
| corp   | console-web          | Authorization Code + PKCE    | TOTP (alto riesgo)    | Brokering a IdP externo |
| corp   | orchestrator-svc     | Client Credentials           | No                    | Scope: service:orchestrator |
| prod   | worker-mgr-svc       | Client Credentials           | No                    | RBAC por namespace |
| prod   | console-web          | Authorization Code + PKCE    | WebAuthn (prod)       | Conditional auth por IP |
| stage  | scheduler-svc        | Client Credentials           | No                    | Scope: service:scheduler |

### 3.2 Validación y refresh de tokens en Rust

Los servicios validan tokens en cada petición, con cachés locales y listas de revocación cuando aplique. El middleware extrae el token del encabezado Authorization, verifica firma/audiencia/expiración y propaga el principal. El refresh se implementa de forma segura, evitando exposición de secretos y registrando auditoría en cada paso. La instrumentación con tracing incluye spans de “authorize” y eventos AuthGranted/Denied, integrados con OpenTelemetry para observabilidad end‑to‑end[^11].

## 4. AWS Verified Permissions: diseño de políticas y evaluación

La autorización policy‑based con AVP se apoya en un modelo Principal–Resource–Action–Context, políticas Cedar y un esquema de entidades que refleja los bounded contexts. AVP centraliza las políticas y proporciona decisiones auditable Permit/Deny, con obligations y reasons para acciones específicas. Las políticas pueden ser estáticas (para roles claros) o vinculadas a entidades (para atributos como tenant_id, tags, environment). La integración con Cognito facilita la federación de identidades y el mapeo de tokens a entidades de AVP[^7][^8][^12].

Tabla 5. Recursos y acciones AVP por bounded context

| Bounded Context    | Recurso    | Acciones (principales)                                 | Contexto clave                         |
|--------------------|------------|---------------------------------------------------------|----------------------------------------|
| Orchestration      | Job        | create, read, cancel, execute, retry                    | tenant_id, owner, tags                 |
| Orchestration      | Pipeline   | create, read, update, execute                           | tenant_id, environment, owner          |
| Scheduling         | Offer      | read, schedule                                          | zone, pool, capacity_snapshot          |
| Scheduling         | Assignment | create, revoke                                         | policy, tenant                         |
| Worker Management  | Worker     | create, read, terminate, logs                           | provider_id, namespace, tenant         |
| Execution          | Step       | execute, upload_logs                                    | environment, step_type                 |
| Telemetry          | Metric     | emit, read                                              | subject, tenant                        |
| Artifact           | Artifact   | upload, read, promote, delete                           | repository, environment, checksum      |

Tabla 6. Atributos para ABAC (tenant, entorno, etiquetas, política de scheduling)

| Tipo de atributo | Ejemplos                      | Uso en políticas                               |
|------------------|-------------------------------|------------------------------------------------|
| tenant           | tenant_id, owner              | Segmentación por cliente/entidad               |
| entorno          | env=prod|stage|dev          | Restricciones de operaciones sensibles        |
| etiquetas        | tags=[“gpu”, “spot”]         | Afinidad y selección de pools                  |
| scheduling       | policy=FIFO|packing|affinity | Reglas de asignación                           |
| recurso          | IsPrivate=true (artefactos)   | Controles adicionales sobre recursos privados  |

La creación del policy store, definición del schema Cedar y políticas estáticas de ejemplo se implementan con el SDK de Rust; las decisiones se integran en cada endpoint crítico, con auditoría de obligaciones y determinantes[^7].

### 4.1 Política Cedar de ejemplo para la plataforma CI/CD

Para ilustrar el modelado, el esquema define entidades User, Job, Pipeline, Worker y Artifact, y acciones Create/Read/Update/Delete/Execute/Schedule/Upload/Promote. Una política estática puede permitir a un principal con scope adecuado crear un Job si tenant_id coincide y los metadatos de JobSpec cumplen restricciones (por ejemplo, límites de recursos). Las políticas pueden incluir condiciones sobre environment (prod/stage/dev) y sobre tags (p. ej., no permitir ejecutar jobs con tags “privileged” en prod). El uso de Static vs Linked policies se decide según la frecuencia de cambio y el nivel de parametrización requerido.

Tabla 7. Políticas por rol y condiciones

| Rol/Servicio   | Caso                 | Condiciones (ejemplos)                           | Decisión esperada |
|----------------|----------------------|--------------------------------------------------|-------------------|
| Orchestrator   | create_job           | scope=job:create && tenant_id == principal.tenant | Permit            |
| Scheduler      | assign_job           | policy in {FIFO,Packing} && capacity >= required | Permit            |
| Worker Manager | create_worker        | provider authorized && namespace allowed         | Permit            |
| Execution      | run_step             | env allowed && step_type allowed                 | Permit            |
| Artifact       | upload               | repository allowed && checksum valid             | Permit            |

### 4.2 Caché de permisos y auditoría

Para optimizar latencia, los servicios cachean decisiones AVP ( Permit/Deny) con TTL por tipo de operación y invalidación por eventos (por ejemplo, cambio de políticas, revocación de scopes, rotación de credenciales). Los errores de autorización se tratan de forma explícita y se auditan; ante fallos o indisponibilidad de AVP, se aplica “fail‑safe deny” por defecto, salvo rutas críticas explícitamente marcadas como “fail‑open” con controles compensatorios.

Tabla 8. TTL de caché por operación

| Operación             | TTL sugerido | Invalidación                              |
|-----------------------|--------------|-------------------------------------------|
| create_job            | 60–120 s     | Cambios de roles/scopes/tenants           |
| schedule              | 30–60 s      | Rotación de políticas de scheduling       |
| create_worker         | 120–300 s    | RBAC del provider/namespace                |
| exec:run              | 30–60 s      | Cambios de environment o MFA               |
| artifact:upload       | 300–600 s    | Cambios de repository/policy               |

## 5. Autenticación mutua (mTLS): CA, certificados, pinning, rotación, CRL

La autenticación mutua (mTLS) garantiza la identidad de servicios y cifra el tráfico interno. La arquitectura incluye una CA interna con chain of trust y secretos gestionados por el provider (Kubernetes/Docker). La emisión y rotación de certificados es automatizada; el pinning por service/tenant reduce la superficie de ataque; las CRL se publican y consumen para revocación en caliente. La configuración TLS por componente usa rustls para un stack moderno y auditable[^9][^10].

Tabla 9. Política de rotación por tipo de certificado

| Tipo                   | Frecuencia       | Acciones en renovación                       |
|------------------------|------------------|----------------------------------------------|
| Certificados de servicio | 7–30 días        | Reemisión, validación de cadena, rolling restart |
| CA intermedia          | 6–12 meses       | Rotación planificada y cross‑signing         |
| CRL                    | Diario           | Publicación y propagación                     |

Tabla 10. Pinning por componente/tenant

| Componente       | Pinning (CN/SAN)                 | Observaciones                          |
|------------------|----------------------------------|----------------------------------------|
| Orchestrator     | CN=orchestrator.<tenant>.svc     | Un SAN por tenant; rotación coordinada |
| Scheduler        | CN=scheduler.<zone>.svc          | Afinidad por zona                      |
| Worker Manager   | CN=worker_mgr.<namespace>.svc    | Scope por namespace                    |
| Telemetry        | CN=telemetry.<subject>.svc       | Subject‑scoped                         |

### 5.1 Generación y distribución segura de certificados

La emisión se automatiza con perfiles por servicio; la distribución usa secretos del provider y mecanismos de sincronización. Se mantiene consistencia de cadena mediante cross‑signing durante rotaciones, y se instrumenta telemetría de expiración y fallos de validación. La práctica se inspira en ejemplos de mTLS en Rust con herramientas estándar de CA y librerías como rustls[^9][^10].

## 6. Autorización granular: por componente, operación y scope

La autorización se diseña por bounded context, recurso y operación, con scopes y atributos en contexto. La decisión dinámica se basa en AVP, con evaluación sobre principal (usuario/service account), acción (por ejemplo, execute, schedule), recurso (Job, Worker, Artifact) y contexto (tenant, environment, tags). El caching con invalidación por eventos permite equilibrar latencia y seguridad; los errores se auditan y propagan para trazabilidad.

Tabla 11. Matriz Componente × Operación × Scope × Contexto

| Componente       | Operación           | Scope mínimo            | Contexto requerido                 | Decisión |
|------------------|---------------------|-------------------------|------------------------------------|---------|
| Orchestrator     | create_job          | job:create              | tenant_id coincide                 | AVP     |
| Orchestrator     | cancel_job          | job:cancel              | job.owner == principal             | AVP     |
| Scheduler        | assign_job          | schedule:assign         | policy aplicable + capacidad       | AVP     |
| Scheduler        | revoke_assignment   | schedule:revoke         | policy permite y tenant válido     | AVP     |
| Worker Manager   | create_worker       | worker:create           | provider/namespace permitido       | AVP     |
| Worker Manager   | terminate_worker    | worker:terminate        | worker.owner == tenant             | AVP     |
| Execution        | run_step            | exec:run                | environment permitido              | AVP     |
| Artifact         | upload              | artifact:upload         | repository permitido               | AVP     |

### 6.1 Patrones de caching e invalidación

El caching se basa en claves compuestas (principal, acción, recurso, contexto) y TTL por operación; la invalidación se dispara por eventos de política (AVP policy changed), cambio de scopes, rotación de certificados y actualización de roles/tenants. Las métricas de latencia de autorización y tasa de denegaciones se monitorizan para ajustar TTL y alcance del caché. El diseño sigue patrones de microservicios para consistencia y resiliencia[^2].

Tabla 12. Eventos de invalidación y acción

| Evento                    | Acción de caché                     |
|---------------------------|-------------------------------------|
| PolicyUpdated             | Invalidateall por recurso afectado  |
| ScopeRevoked              | Invalidate por principal y scope    |
| CertRotated               | Invalidate rutas mTLS afectadas     |
| RoleChanged               | Invalidate por rol/tenant           |

## 7. Aislamiento de workers efímeros

Los workers efímeros se ejecutan en entornos aislados con restricciones de seguridad estrictas: capabilities mínimas, contexts seguros, quotas de CPU/memoria/GPU, network policies que limitan egress/ingress, y políticas de pod security. La segmentación por namespaces y tenants, y el uso de Service Accounts con RBAC reducen la superficie de ataque. El hardening de imágenes y el escaneo continuo de vulnerabilidades son obligatorios para evitar ejecución de código malicioso en pipelines[^5].

Tabla 13. Security context por workload y entornos

| Entorno   | Security context                      | Capabilities              | Privileged | Notas             |
|-----------|----------------------------------------|---------------------------|------------|-------------------|
| prod      | runAsNonRoot, disallow privilege escalation | NET_BIND_SERVICE (solo si necesario) | No         | Pod Security Standards “restricted” |
| stage     | runAsNonRoot, readOnlyRootFilesystem  | Minimal set               | No         | Baseline + endurecimiento |
| dev       | allow mild privileges según caso      | Limitado                  | No         | Aislamiento lógico         |

Tabla 14. Network policies por tipo de pipeline

| Tipo de pipeline | Ingress                        | Egress                                    |
|------------------|--------------------------------|-------------------------------------------|
| build            | Solo desde Orchestrator/Scheduler | Hacia registries y artefactos permitidos |
| test             | Desde build (resultados)       | Hacia servicios de test externos permitidos |
| deploy           | Desde Orchestrator             | Hacia entornos destino y secretos         |

### 7.1 Políticas de red y firewall por entorno

Las políticas de red se aplican por tipo de pipeline, con egress limitado hacia registries y dependencias autorizadas; ingress se restringe a orígenes verificados. Los controles se revisan periódicamente y se auditan cambios. La instrumentación con watchers facilita detección eficiente de cambios y reconciliación de estado[^12].

Tabla 15. Puertos y orígenes/destinos permitidos

| Puerto/Servicio | Origen              | Destino                   | Justificación                         |
|-----------------|---------------------|---------------------------|---------------------------------------|
| 443 (HTTPS)     | Workers             | Registries                | Descarga de imágenes/artefactos       |
| 22 (SSH)        | Worker Manager      | Nodes                     | Operaciones controladas               |
| 8080 (HTTP API) | Orchestrator/Scheduler | Worker Manager          | Control‑plane                          |

## 8. Cifrado de datos (in transit y at rest)

El cifrado en tránsito cubre comunicaciones internas (mTLS), APIs externas (TLS), mensajería (NATS/Kafka) y streaming (SSE/WS). El cifrado en reposo protege métricas y eventos, artefactos, secretos y bases de datos por servicio. La gestión de claves y el ciclo de vida (incluida rotación y HSM/KMS cuando aplique) se integran con los mecanismos del provider y librerías Rust auditadas como rustls para TLS y primitivas modernas para cifrado simétrico (AEAD)[^10][^13].

Tabla 16. Algoritmos y librerías recomendadas por caso

| Caso                         | Algoritmos/Librerías          | Observaciones                         |
|-----------------------------|-------------------------------|---------------------------------------|
| TLS (in transit)            | rustls (TLS 1.2/1.3)          | Auditoría y modernidad del stack[^10] |
| NATS/Kafka (in transit)     | TLS (rustls)                  | Autenticación de clientes             |
| AEAD (at rest)              | AES‑GCM, ChaCha20‑Poly1305    | Primitivas ampliamente usadas         |
| Hash y firma                | SHA‑256, Ed25519              | Integridad y autenticación            |

Tabla 17. Política de rotación de secretos y claves

| Tipo de secreto/clave | Frecuencia       | Acciones                                 |
|-----------------------|------------------|-------------------------------------------|
| Tokens OIDC           | 24 h (servicio)  | Revocación y renovación automática        |
| Certificados mTLS     | 7–30 días        | Reemisión y validación de cadena          |
| Claves AEAD           | 90 días          | Rotación con re‑cifrado selectivo         |

### 8.1 Secret management y rotation

Los secretos se almacenan en gestores de secretos del proveedor (Kubernetes Secrets, Vault, AWS Secrets Manager), con rotación automatizada, invalidación y planes de recuperación. La protección de tokens y claves en logs es obligatoria; el diseño cumple con prácticas de CI/CD seguro[^1].

Tabla 18. Inventario de secretos y rotación

| Secreto                     | Ubicación                 | Rotación      | Responsable        |
|----------------------------|---------------------------|---------------|--------------------|
| OIDC service account keys  | Secret Manager/Vault      | 24 h          | Plataforma         |
| mTLS certs/keys            | Kubernetes Secrets        | 7–30 días     | Seguridad          |
| AEAD keys                  | KMS/HSM                   | 90 días       | Seguridad/Plataforma |
| Repository credentials     | Vault/Secrets Manager     | 60 días       | DevOps             |

## 9. Audit trail distribuido

La auditoría se construye con logs estructurados y eventos de seguridad (AuthGranted, AuthRevoked, AuthorizationDenied), correlacionados con jobs/pipelines y trazabilidad distribuida. La retención cumple requisitos de SOC2 y GDPR; se habilita reporting y exportación a SIEM. La forénsica se apoya en snapshots verificados y replays controlados del bus, con idempotencia y deduplicación en la ingesta de auditoría[^4].

Tabla 19. Catálogo de eventos de auditoría por dominio

| Dominio          | Evento                 | Campos mínimos                                  | Retención | Export |
|------------------|------------------------|--------------------------------------------------|-----------|--------|
| Seguridad        | AuthGranted            | principal, scope, decision_id, ts               | ≥ 180 días | SIEM   |
| Seguridad        | AuthRevoked            | principal, scope, decision_id, ts               | ≥ 180 días | SIEM   |
| Seguridad        | AuthorizationDenied    | resource, action, reason, ts                    | ≥ 180 días | SIEM   |
| Orchestration    | JobRequested/Completed | job_id, correlation_id, tenant_id, ts           | ≥ 90 días  | Data lake |
| Execution        | StepStarted/Completed  | step_id, exec_id, duration, ts                  | ≥ 30 días  | Data lake |
| Artifact         | ArtifactUploaded       | artifact_ref, checksum, ts                       | ≥ 90 días  | Data lake |

Tabla 20. Retención y export por tipo

| Tipo de dato     | Retención         | Export                 |
|------------------|-------------------|------------------------|
| Seguridad        | ≥ 180 días        | SIEM                   |
| Jobs/Pipelines   | ≥ 90 días         | Data lake              |
| Steps            | ≥ 30 días         | Data lake              |
| Métricas         | Horas/días        | Observabilidad         |

### 9.1 Observabilidad y alertas de seguridad

Las alertas se definen por eventos críticos (cambios de política, picos de denegaciones, fallos de autenticación), con reglas y supresiones. La cobertura de trazas en decisiones de autorización se integra con OpenTelemetry; dashboards exponen decisiones Permit/Deny por tenant, endpoint y contexto[^13].

Tabla 21. Reglas de alertas de seguridad

| Condición                               | Severidad | Canal       | Supresión |
|-----------------------------------------|-----------|-------------|-----------|
| Tasa de AuthorizationDenied > 5%       | Critical  | PagerDuty   | 5 min     |
| Cambios de política AVP                 | Info      | Slack       | N/A       |
| Fallos de autenticación OIDC            | Warning   | Slack       | 10 min    |

## 10. Protección contra ataques

La mitigación de ataques en pipelines y servicios se articula con rate limiting/throttling, protección DDoS en edge y gateway, validación y sanitización de entradas, y prevención de inyección/SQL‑XSS/CSRF. Se añade la protección de cadena de suministro con firma de artefactos (Sigstore/in‑toto), SBOM y escaneo de vulnerabilidades en imágenes. El diseño se alinea con mejores prácticas OWASP CI/CD[^1].

Tabla 22. Controles por vector de ataque y componente

| Vector            | Control                         | Componentes                      |
|-------------------|---------------------------------|----------------------------------|
| DDoS              | Rate limiting, circuit breaker  | Console, Orchestrator, Scheduler |
| Auth abuse        | MFA, scopes, AVP caching        | Console, Orchestrator            |
| Injection (code)  | SAST/DAST, sandbox              | Workers, Execution               |
| Supply chain      | Sigstore, SBOM, image scanning  | Artifact, Workers                |

### 10.1 Políticas de rate limiting y throttling

Los límites por tenant/usuario/endpoint/evento se definen con cuotas y burst, y backpressure en consumidores. La priorización por SLA distingue rutas críticas (creación de jobs) y secundarias (consultas de telemetría).

Tabla 23. Cuotas y límites por tipo de request

| Tipo de request    | Cuota (p95)        | Burst         | Prioridad |
|--------------------|--------------------|---------------|----------|
| POST /jobs         | < 300 ms           | 10 req/s      | Alta     |
| POST /schedule     | < 500 ms           | 20 req/s      | Alta     |
| GET /ui/jobs       | < 400 ms           | 50 req/s      | Media    |
| MetricEmitted      | < 1 s              | 1000 msg/s    | Baja     |

## 11. Implementación en Rust: ejemplos de código

La implementación en Rust adopta middleware de autenticación OIDC con Keycloak, validación de JWT y refresh seguro, clientes AVP para autorización, clientes mTLS con rustls y configuración basada en certificados. La integración con NATS/Kafka preserva idempotencia y backpressure; el tracing y métricas de seguridad se instrumentan con OpenTelemetry y Prometheus. La elección del framework (Axum/Actix) se hace con criterios de rendimiento, ergonomía y compatibilidad con contratos OpenAPI[^11][^6][^14].

Tabla 24. Mapa crates/librerías por función

| Función                          | Librería/patrón                  | Justificación                         |
|----------------------------------|----------------------------------|---------------------------------------|
| OIDC Cliente                     | openid-client                    | Cliente OIDC runtime‑agnóstico[^6]    |
| JWT Validation                   | jsonwebtoken                     | Validación y manejo de claims[^11]    |
| mTLS Client/Server               | rustls                           | TLS moderno y auditable[^10]          |
| HTTP/API                         | Actix‑web/Axum                   | Alto rendimiento y ergonomía[^14]     |
| Mensajería                       | nats.crate + JetStream           | Baja latencia, subjects[^4]           |
| Observabilidad                   | tracing + OpenTelemetry          | Trazabilidad y métricas[^13]          |

### 11.1 Middleware de autenticación y autorización

El middleware extrae y valida tokens OIDC, propaga el principal y el contexto (tenant, scopes), evalúa AVP por operación y cachea decisiones con invalidación por eventos. La auditoría registra decisiones y errores en cada endpoint. El flujo típico: request → validate JWT → decide AVP → enforce → audit.

### 11.2 Cliente AVP en Rust

La construcción del cliente AVP, mapeo de entidades y evaluación de decisiones se integran en cada servicio. El manejo de errores distingue entre fallos transitorios y denegaciones; se aplica caching y “fail‑safe deny” cuando AVP no responde. Las obligations y reasons se registran en auditoría.

### 11.3 mTLS y seguridad de transporte

La configuración de clientes/servidores con rustls aplica client authentication, certificados por servicio, validación de hostname y cadena de confianza. La sesión segura se establece con policies modernas (TLS 1.2/1.3), y se instrumentan métricas de handshake y errores[^10].

## 12. Despliegue, validación y respuesta a incidentes

El despliegue seguro de la capa de seguridad incluye configuración de Keycloak y AVP, gestión de secretos, validación de contratos (OpenAPI y eventos), y pruebas de resiliencia. Las pruebas de seguridad abarcan autenticación, autorización, mTLS y auditoría; los SLOs definen latencias y disponibilidad. El plan de respuesta a incidentes cubre revocaciones, rotaciones, fallbacks, playbooks y runbooks. La compatibilidad hacia atrás se gobierna con versionado y contract testing[^12][^4].

Tabla 25. Checklist de seguridad por entorno (dev/stage/prod)

| Ítem                               | Dev | Stage | Prod |
|------------------------------------|-----|-------|------|
| OIDC + MFA habilitados             | ✓   | ✓     | ✓    |
| AVP integrado con caché            | ✓   | ✓     | ✓    |
| mTLS con CA y rotación             | ✓   | ✓     | ✓    |
| Network policies                   | Básico | Estricto | Estricto |
| Audit trail completo               | ✓   | ✓     | ✓    |
| Secret rotation y KMS              | ✓   | ✓     | ✓    |

Tabla 26. Runbooks de seguridad

| Incidente                     | Pasos principales                         | Responsable       | Tiempo objetivo |
|------------------------------|-------------------------------------------|-------------------|-----------------|
| Fallo de autenticación       | Revocar tokens, reiniciar servicios       | Seguridad         | 30 min          |
| Denegaciones elevadas AVP    | Revisar políticas, invalidar caché        | Plataforma        | 60 min          |
| Expiración de cert mTLS      | Rotación de certs, validación de cadena   | Seguridad         | 30 min          |
| Vulnerabilidad en imagen     | Quarantena, parcheo, re‑despliegue        | DevOps            | 120 min         |

## 13. Roadmap e hitos

El roadmap organiza entregables incrementales con métricas y validación continua. Los hitos aseguran adopción gradual de seguridad y resiliencia, alineados con SLOs de la plataforma. El orden propuesto mitiga riesgos y facilita operación estable del backbone de mensajería y de los servicios[^4].

Tabla 27. Hitos y métricas objetivo

| Hito                       | Entregables                                       | Métricas objetivo                  |
|----------------------------|---------------------------------------------------|------------------------------------|
| Seguridad base             | OIDC + JWT + Service Accounts + AVP básico        | p95 auth < 100 ms                  |
| mTLS interno               | CA, emisión, rotación, pinning                    | Éxito handshakes > 99.9%           |
| Aislamiento workers        | Security contexts, network policies, quotas       | Incidentes de aislamiento = 0      |
| Cifrado y secretos         | AEAD at rest, KMS/HSM, rotación                   | Cumplimiento de política = 100%    |
| Auditoría y alertas        | Eventos de seguridad, dashboards, alertas         | Cobertura auditoría = 100%         |

## 14. Riesgos, supuestos y mitigaciones

Los riesgos clave incluyen operación del backbone de mensajería (NATS/Kafka), consistencia eventual entre servicios, mezcla de runtimes asíncronos, saturación del scheduling y seguridad multi‑tenant. Las mitigaciones se apoyan en automatización, observabilidad del bus y servicios, idempotencia, Sagas y auditoría, con runbooks y pruebas de resiliencia. La semántica de entrega y el diseño de contratos impactan latencia y throughput; deben medirse y gobernarse con SLOs y contract testing[^4][^2].

Tabla 28. Matriz de riesgos y mitigaciones

| Riesgo                         | Impacto | Probabilidad | Mitigación                                   | Métrica de control            |
|-------------------------------|---------|--------------|----------------------------------------------|-------------------------------|
| Operación de NATS/Kafka       | Medio   | Media        | Automatización, observabilidad, runbooks     | p95 latencia eventos          |
| Consistencia eventual         | Medio   | Alta         | Idempotencia, Sagas, auditoría               | Tasa de duplicados            |
| Mezcla de runtimes async      | Alto    | Media        | Tokio único; pruebas                         | Fallos de integración         |
| Saturación scheduling         | Alto    | Media        | Backpressure, autoscaling, políticas         | Longitud de cola              |
| Seguridad multi‑tenant        | Alto    | Media        | RBAC, segmentación, secretos                 | Decisiones AVP por tenant     |
| Falla de provider             | Alto    | Baja‑Media   | Circuit Breaker, fallback, retry             | Error rate provider           |

## 15. Gaps de información

Se reconocen las siguientes áreas que requieren definición detallada en fases posteriores:

- Topología exacta y dominios de seguridad (multi‑tenant, dominios de fallo, segmentación de red) entre componentes; decisiones de pinning por servicio/tenant y políticas de CRL/OCSP.
- Esquema Cedar completo y final de AVP para todos los recursos/acciones de la plataforma, y las políticas específicas por rol/servicio.
- Proveedor(es) KMS/HSM, estrategia de rotación de claves y dominios de cifrado (datos vs metadatos).
- Política organizacional de MFA (TOTP/WebAuthn), requisitos regulatorios específicos y su mapeo a flujos por endpoint/rol.
- Parámetros operativos del sidecar/daemonset de mTLS, política de fallos (fail‑open vs fail‑closed) y objetivos de latencia/overhead.
- Plan de rotación y caducidad de certificados y secretos, ventanas de mantenimiento y criterios de reversión.
- Herramientas de escaneo de imágenes y políticas de admisión (SBOM, signature verification, policy gates) y su integración en el pipeline.
- SLIs/SLOs de seguridad específicos (autenticación, autorización, mTLS) y umbrales de alerta para auditoría y forénsica.
- Estrategia de rate limiting/throttling por endpoint/evento (cuotas por tenant/usuario, burst/budget, backpressure).
- Mecanismos de firma y verificación de artefactos (Sigstore/in‑toto) y su integración con registries y pipeline.

Estas lagunas no impiden el diseño base, pero condicionan decisiones operativas finas y métricas de seguridad que se resolverán con experimentación y medición.

## 16. Conclusiones

La seguridad distribuida propuesta habilita una plataforma CI/CD robusta y gobernada, con autenticación OIDC/OAuth2 confiable mediante Keycloak, autorización policy‑based con AVP, transporte seguro con mTLS, aislamiento de ejecución y cifrado end‑to‑end. La auditoría distribuida y la observabilidad aseguran trazabilidad y cumplimiento, mientras que la mitigación de ataques comunes y de cadena de suministro fortalece la postura defensiva. La implementación en Rust, apoyada por contratos OpenAPI y eventos versionados, permite resiliencia y escalabilidad con patrones consolidados de microservicios. El roadmap y los runbooks guían la adopción operativa y la respuesta a incidentes, cerrando brechas mediante medición y validación continua[^2][^4][^13].

---

## Referencias

[^1]: OWASP Cheat Sheet Series. “CI/CD Security Cheat Sheet.” https://cheatsheetseries.owasp.org/cheatsheets/CI_CD_Security_Cheat_Sheet.html  
[^2]: microservices.io. “Pattern: Microservice Architecture.” https://microservices.io/patterns/microservices.html  
[^3]: Microsoft Learn — Azure Architecture Center. “Domain analysis for microservices.” https://learn.microsoft.com/en-us/azure/architecture/microservices/model/domain-analysis  
[^4]: Synadia. “NATS and Kafka Compared.” https://www.synadia.com/blog/nats-and-kafka-compared  
[^5]: Shuttle.dev. “Using Kubernetes with Rust.” https://www.shuttle.dev/blog/2024/10/22/using-kubernetes-with-rust  
[^6]: openid-client crate (docs.rs). https://docs.rs/openid-client/0.2.7  
[^7]: AWS Documentation. “Implementing Amazon Verified Permissions in Rust with the AWS SDK.” https://docs.aws.amazon.com/verifiedpermissions/latest/userguide/code-samples-rust.html  
[^8]: AWS Documentation. “What is Amazon Verified Permissions?” https://docs.aws.amazon.com/verifiedpermissions/latest/userguide/what-is-avp.html  
[^9]: camelop/rust-mtls-example (GitHub). https://github.com/camelop/rust-mtls-example  
[^10]: Rustls (GitHub). https://github.com/ctz/rustls  
[^11]: LogRocket Blog. “JWT authentication in Rust.” https://blog.logrocket.com/jwt-authentication-in-rust/  
[^12]: Kubernetes Documentation. “Efficient detection of changes (watchers).” https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes  
[^13]: Microsoft Learn — Azure Architecture Center. “Microservices architecture style.” https://learn.microsoft.com/en-us/azure/architecture/guide/architecture-styles/microservices  
[^14]: OpenAPI Specification. “Latest.” https://spec.openapis.org/oas/latest.html