# Plan de Implementación: Resource Governance & Pipelines

Este plan aborda los gaps identificados en `docs/analysis/gap_analysis_resource_governance.md` para llevar el sistema de Resource Governance a un estado "Production Ready".

## Fase 1: Persistencia y Modelo de Datos (Prioridad Alta)
**Objetivo:** Eliminar el almacenamiento en memoria y asegurar la durabilidad de la configuración de pools.

### Tareas:
1.  **Definir Puertos (Ports):**
    -   Crear `ResourcePoolRepository` trait en `crates/ports/src/resource_pool.rs`.
    -   Métodos: `save`, `get`, `list`, `delete`, `update_status`.
2.  **Implementar Adaptadores (Adapters):**
    -   Implementar `RedbResourcePoolRepository` en `crates/adapters/src/repositories/redb/resource_pool_repository.rs`.
    -   Asegurar serialización correcta de `ResourcePoolConfig`.
3.  **Refactorizar Servicios:**
    -   Modificar `ResourcePoolCrudService` en `crates/server` para usar `ResourcePoolRepository` en lugar de `HashMap`.
    -   Modificar `GlobalResourceController` para cargar pools desde el repositorio al inicio.

## Fase 2: Integración Scheduler - GRC (Prioridad Alta)
**Objetivo:** Que el Scheduler respete las cuotas y utilice los pools definidos.

### Tareas:
1.  **Conectar GRC al Scheduler:**
    -   En `SchedulerModule`, reemplazar `global_resource_controller: Option<()>` con `Arc<GlobalResourceController>`.
    -   Inicializar GRC correctamente en `main.rs`.
2.  **Lógica de Scheduling Aware:**
    -   En `SchedulerModule::schedule_job`, antes de buscar workers:
        -   Crear `ResourceRequest` basado en el Job.
        -   Llamar a `GRC::find_pools` para obtener pools candidatos.
        -   Llamar a `GRC::allocate_resources` para reservar capacidad.
    -   Si falla la asignación, poner el job en estado `QUEUED` (o `PENDING` con backoff).
3.  **Release de Recursos:**
    -   Asegurar que cuando un job termina (éxito o fallo), se llame a `GRC::release_resources`.

## Fase 3: Pipeline Execution & Worker Templates (Prioridad Media)
**Objetivo:** Que los pipelines soliciten recursos dinámicamente.

### Tareas:
1.  **Cálculo de Recursos:**
    -   Implementar `WorkerTemplate::from_pipeline(pipeline)` para sumarizar recursos de todos los steps.
2.  **Solicitud de Worker Efímero:**
    -   En `PipelineExecutionOrchestrator` (o servicio equivalente), antes de iniciar la ejecución:
        -   Crear `WorkerTemplate`.
        -   Solicitar provisionamiento al Scheduler/GRC.
        -   Esperar a que el worker esté listo (`AllocationStatus::Allocated`).

## Fase 4: Frontend - Gestión de Pools (Prioridad Media)
**Objetivo:** UI para administrar la gobernanza.

### Tareas:
1.  **Página de Listado de Pools:**
    -   Crear `web-console/src/pages/workers/ResourcePoolsPage.tsx`.
    -   Tabla con: Nombre, Tipo, Provider, Capacidad (Total/Usada), Estado.
2.  **Detalle y Edición:**
    -   Crear `web-console/src/pages/workers/ResourcePoolDetailsPage.tsx`.
    -   Formulario para editar `min_size`, `max_size`, `tags`, `quotas`.
3.  **Dashboard de Gobernanza (Opcional Fase 1):**
    -   Gráficos de uso de CPU/Memoria por Pool.

## Plan de Verificación

### Tests Automatizados
-   **Unit Tests:** Tests para `RedbResourcePoolRepository` y lógica de `GRC`.
-   **Integration Tests:**
    -   Crear un Pool vía API.
    -   Verificar que persiste en BD.
    -   Lanzar un Job que requiera recursos de ese Pool.
    -   Verificar que `GRC` registra la asignación.

### Verificación Manual
1.  Iniciar servidor.
2.  Crear un Pool "Docker-Test" vía API (o UI si está lista).
3.  Reiniciar servidor.
4.  Verificar que el Pool "Docker-Test" sigue existiendo.
5.  Ejecutar un Pipeline simple.
6.  Verificar logs para confirmar que pasó por GRC.
