# Informe de Gaps: Resource Governance & Pipelines

Este documento detalla los gaps identificados entre la propuesta final (`docs/analysis/propuesta_final_resource_governance_pipelines.md`) y la implementación actual del sistema.

## 1. Persistencia de Datos (Crítico)
**Estado Actual:**
- `ResourcePoolCrudService` (`crates/server/src/resource_pool_crud.rs`) utiliza `Arc<RwLock<HashMap<String, ResourcePoolConfig>>>` para almacenar la configuración de los pools.
- `GlobalResourceController` (`crates/modules/src/global_resource_controller.rs`) también mantiene el estado en memoria.

**Gap:**
- No existe persistencia real (Base de Datos) para los Resource Pools. Si el servidor se reinicia, se pierden todas las configuraciones de pools y el estado de las asignaciones.
- **Requerimiento:** Migrar a almacenamiento persistente (Redb o PostgreSQL) implementando repositorios adecuados (`ResourcePoolRepository`).

## 2. Integración Scheduler - GRC (Crítico)
**Estado Actual:**
- `SchedulerModule` (`crates/modules/src/scheduler/mod.rs`) tiene un campo `global_resource_controller: Option<()>` marcado con `TODO`.
- La lógica de programación (`schedule_job`, `run_scheduling_cycle`) selecciona workers individuales basándose en `worker_repo` y `cluster_state`, ignorando por completo los Resource Pools y las cuotas.

**Gap:**
- El Scheduler no consulta al GRC para validar cuotas o seleccionar el pool adecuado antes de asignar un job.
- Falta la lógica de "Placement" basada en Pools.
- **Requerimiento:** Integrar `GlobalResourceController` en el ciclo de scheduling para que actúe como gatekeeper de recursos y director de placement.

## 3. Pipeline Execution & Resource Requests
**Estado Actual:**
- `PipelineExecution` (`crates/core/src/pipeline_execution.rs`) gestiona el flujo de pasos pero no parece generar `ResourceRequest` formales hacia el GRC.
- No se ve evidencia de la creación de `WorkerTemplate` basada en los requerimientos del pipeline para solicitar workers efímeros.

**Gap:**
- Desconexión entre la ejecución del pipeline y la solicitud de recursos.
- Falta la implementación de `WorkerTemplate` dinámica basada en los steps del pipeline.
- **Requerimiento:** Modificar el orquestador de pipelines para calcular los recursos totales necesarios y solicitar un worker (o grupo de workers) al GRC/Scheduler antes de iniciar la ejecución.

## 4. Quota Enforcement & Multi-tenancy
**Estado Actual:**
- `GlobalResourceController` tiene lógica básica para chequear presupuestos (`budget.would_exceed`), pero al no estar integrado con el Scheduler, esta lógica no se ejecuta en el camino crítico.
- No se encontró implementación robusta de `TenantQuota` que limite el consumo agregado por tenant a través de múltiples pools.

**Gap:**
- Las cuotas de tenant no se aplican efectivamente.
- No hay seguimiento de consumo histórico real para facturación o reportes (FinOps incipiente).
- **Requerimiento:** Implementar el `QuotaEnforcementEngine` y conectarlo al GRC.

## 5. Frontend (Web Console)
**Estado Actual:**
- Existe `resourcePoolApi.ts` y una estructura de páginas en `web-console`.
- `workers-page.tsx` parece estar orientado a listar workers individuales.

**Gap:**
- Falta confirmación de una UI dedicada a la **Gestión de Resource Pools** (CRUD, visualización de capacidad, asignación de cuotas).
- Falta visualización de métricas de GRC (uso de pools, saturación, costes).
- **Requerimiento:** Implementar/Completar páginas de administración de Pools y Dashboards de Gobernanza.

## Resumen de Prioridades
1.  **Persistencia**: Habilitar almacenamiento real para Pools.
2.  **Integración Scheduler-GRC**: Conectar el cerebro de recursos con el ejecutor.
3.  **Pipeline Resources**: Hacer que los pipelines pidan recursos formalmente.
4.  **UI**: Gestión de pools.
