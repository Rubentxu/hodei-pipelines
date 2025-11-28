# EPIC-10: API Alignment & Contract Stabilization

**Status:** 📝 Proposed
**Priority:** High
**Owner:** Architecture Team

## 1. Descripción General
Esta épica aborda la **desconexión crítica** identificada entre el Frontend (Web Console) y el Backend (Hodei Server). El objetivo es establecer una "Single Source of Truth" (SSOT) mediante un enfoque **Contract First**, automatizar la generación de clientes para evitar "Type Drift" y asegurar que todos los endpoints necesarios estén implementados y expuestos correctamente.

## 2. Propuestas Estratégicas de Mejora

### 2.1. Generación Automática de Tipos (Type Safety)
Actualmente, los tipos en el Frontend (`web-console/src/types`) se definen manualmente, lo que lleva a errores silenciosos cuando el Backend cambia.

*   **Propuesta:** Implementar **`openapi-generator-cli`** o **`swagger-typescript-api`**.
*   **Flujo:**
    1.  Backend genera `openapi.json` (vía `utoipa`).
    2.  Script CI/CD ejecuta el generador.
    3.  Frontend consume interfaces generadas automáticamente (`src/api/generated`).
*   **Beneficio:** Eliminación total de errores por discrepancia de tipos (ej: `Date` vs `Timestamp`).

### 2.2. Mocking Strategy (Desbloqueo de Frontend)
Para los endpoints faltantes (ej: `/api/observability/topology`), el Frontend no debe esperar a la implementación del Backend.

*   **Propuesta:** Integrar **MSW (Mock Service Worker)** en el Frontend.
*   **Implementación:**
    *   Interceptar peticiones a nivel de red en desarrollo.
    *   Devolver respuestas basadas en los esquemas OpenAPI.
*   **Beneficio:** Desarrollo paralelo real. El Frontend puede avanzar "asumiendo" que el Backend cumple el contrato.

### 2.3. Capa de Validación (Runtime Validation)
TypeScript solo protege en tiempo de compilación. Las respuestas de la API pueden no coincidir en tiempo de ejecución.

*   **Propuesta:** Integrar **Zod** en el Frontend.
*   **Implementación:**
    *   Generar esquemas Zod a partir del OpenAPI (usando `ts-to-zod` o plugins de `openapi-generator`).
    *   Validar cada respuesta de `fetch` en los servicios (`services/*.ts`).
*   **Beneficio:** "Fail Fast". Si el Backend rompe el contrato, el Frontend lanza un error descriptivo inmediato en lugar de renderizar UIs rotas.

---

## 3. Historias de Usuario (Plan de Acción)

### US-10.1: Centralización y Montaje de Rutas Backend
**Como** Desarrollador Backend,
**Quiero** tener un archivo centralizado de rutas (`routes.rs`),
**Para** asegurar que todos los controladores (`resource_pool`, `observability`) estén expuestos y accesibles.

**Criterios de Aceptación:**
*   [ ] Crear módulo `server/src/routes.rs`.
*   [ ] Refactorizar `main.rs` para usar este router central.
*   [ ] Asegurar que `resource_pool_crud` y `observability_api` estén montados bajo `/api/v1`.
*   [ ] Verificar que `/api/health` sigue funcionando.

### US-10.2: Estandarización de Nombres de Recursos (API Path Standardization)
**Como** Arquitecto de API,
**Quiero** unificar la nomenclatura de los endpoints,
**Para** eliminar la confusión entre `WorkerPool` (Front) y `ResourcePool` (Back).

**Criterios de Aceptación:**
*   [ ] Renombrar rutas en Backend de `/resource-pools` a `/worker-pools` (para ser más amigable al usuario).
*   [ ] Mantener el nombre interno de las estructuras Rust como `ResourcePool` si es necesario por dominio, pero exponerlo como `worker-pools`.
*   [ ] Actualizar `resource_pool_crud.rs` para reflejar estos cambios.

### US-10.3: Implementación de Topology API
**Como** Operador del Sistema,
**Quiero** visualizar la topología del clúster en el Frontend,
**Para** entender cómo están conectados los nodos y workers.

**Criterios de Aceptación:**
*   [ ] Implementar endpoint `GET /api/v1/observability/topology` en `observability_api.rs`.
*   [ ] Definir estructuras `ClusterTopology`, `ClusterNode`, `ClusterEdge` en Rust.
*   [ ] Documentar con macros `utoipa`.
*   [ ] Conectar con datos reales (o mock realista inicial) del estado del clúster.

### US-10.4: Corrección de Verbos HTTP (PUT vs PATCH)
**Como** Consumidor de API,
**Quiero** usar los verbos HTTP semánticamente correctos,
**Para** cumplir con los estándares REST.

**Criterios de Aceptación:**
*   [ ] Modificar `workersApi.ts` para usar `PUT` en actualizaciones completas o asegurar que el Backend soporte `PATCH` para actualizaciones parciales.
*   [ ] Dado que `resource_pool_crud.rs` parece hacer actualizaciones parciales manuales, implementar oficialmente el verbo `PATCH` en el router de Backend.

### US-10.5: Generación de OpenAPI Spec
**Como** Desarrollador Fullstack,
**Quiero** poder descargar el archivo `openapi.json` actualizado,
**Para** configurar herramientas de generación de código.

**Criterios de Aceptación:**
*   [ ] Configurar `utoipa` en `main.rs` para servir `/api/docs/openapi.json`.
*   [ ] Asegurar que todos los DTOs de `resource_pool_crud.rs` tengan `#[derive(ToSchema)]`.
*   [ ] Verificar que Swagger UI (`/api/docs`) carga correctamente todos los endpoints.

## 4. Refactorización Técnica (Detalle)

### Backend: `server/src/routes.rs` (Propuesta)
```rust
pub fn api_routes(state: AppState) -> Router {
    Router::new()
        .nest("/api/v1", Router::new()
            .merge(observability_api::routes())
            .merge(resource_pool_crud::routes())
            // ... otros routers
        )
        // Legacy/Simple routes
        .route("/api/health", get(health_check))
}
```

### Frontend: `services/api.ts` (Propuesta con Zod)
```typescript
import { z } from 'zod';

const WorkerSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  // ...
});

export async function getWorker(id: string) {
  const res = await fetch(\`/api/v1/workers/\${id}\`);
  const data = await res.json();
  return WorkerSchema.parse(data); // Lanza error si el contrato se rompe
}
```
