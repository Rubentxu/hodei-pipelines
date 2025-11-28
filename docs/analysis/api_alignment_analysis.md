# Análisis de Alineamiento Frontend-Backend (Contract First)

**Fecha:** 28 Noviembre 2025
**Estado:** ⚠️ CRITICAL MISALIGNMENT DETECTED

Este documento audita la integridad del sistema Hodei Pipelines basándose en el principio **Contract First**, triangulando la definición OpenAPI, la implementación del Backend (Rust/Axum) y el consumo del Frontend (React/TypeScript).

## 1. Resumen Ejecutivo

El análisis revela una **desconexión crítica** entre el Frontend y el Backend. Aunque existen implementaciones de controladores en el Backend (`server/src`), **no están montados en el punto de entrada principal (`main.rs`)**, lo que significa que el servidor solo expone endpoints de salud básicos. Además, hay discrepancias significativas en las rutas (URLs) esperadas por el Frontend y las definidas en el Backend.

*   **Cobertura de Implementación:** ~10% (Solo `/api/health` y `/api/server/status` están activos).
*   **Cobertura de Consumo:** El Frontend intenta consumir una API rica (`/api/workers`, `/api/observability`, etc.) que no está expuesta.
*   **Estado del Contrato:** Parcialmente definido con `utoipa` en algunos módulos (`observability_api.rs`), pero ausente en otros (`resource_pool_crud.rs`).

## 2. Matriz de Alineamiento

| Endpoint (Path) | Method | Defined in Contract? (Y/N) | Backend Impl Status | Frontend Consumed Status | Data/Type Drift | Status | Actions Needed |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `/api/health` | `GET` | ✅ (`api_docs.rs`) | ✅ Active (`main.rs`) | ❓ Not found in services | N/A | **Aligned** | Verify usage |
| `/api/server/status` | `GET` | ✅ (`api_docs.rs`) | ✅ Active (`main.rs`) | ❓ Not found in services | N/A | **Aligned** | Verify usage |
| `/api/worker-pools` | `GET` | ❌ | ✅ Implemented (`resource_pool_crud.rs`) but **NOT MOUNTED** as `/resource-pools` | ✅ `workersApi.ts` calls `/api/worker-pools` | **Path Mismatch** | **BROKEN** | Mount router & Fix Path |
| `/api/worker-pools/{id}` | `GET` | ❌ | ✅ Implemented but **NOT MOUNTED** | ✅ `workersApi.ts` | **Path Mismatch** | **BROKEN** | Mount router & Fix Path |
| `/api/worker-pools/{id}/scale` | `POST` | ❌ | ❌ No matching handler found in `resource_pool_crud.rs` | ✅ `workersApi.ts` | N/A | **Front Gap** | Implement in Backend |
| `/api/worker-pools/{id}` | `PATCH` | ❌ | ✅ Implemented as `PUT` | ✅ `workersApi.ts` uses `PATCH` | **Method Mismatch** | **BROKEN** | Align Method (PUT vs PATCH) |
| `/api/workers` | `GET` | ✅ (`RegisterWorkerRequest` schema exists) | ❓ Likely in `worker_management.rs` (Not Mounted) | ✅ `workersApi.ts` | TBD | **BROKEN** | Locate & Mount Controller |
| `/api/observability/topology` | `GET` | ❌ | ❌ Not found in `observability_api.rs` | ✅ `clusterApi.ts` | N/A | **Front Gap** | Implement Topology API |
| `/api/v1/observability/health` | `GET` | ✅ (`observability_api.rs`) | ✅ Implemented but **NOT MOUNTED** | ❌ No direct consumer | **Path Prefix** | **Gap** | Mount router |
| `/api/v1/observability/metrics` | `GET` | ✅ (`observability_api.rs`) | ✅ Implemented but **NOT MOUNTED** | ❌ No direct consumer | **Path Prefix** | **Gap** | Mount router |

## 3. Reporte de "API Drift"

### 3.1. Rutas Desconectadas (The "Unmounted" Problem)
El archivo `server/src/main.rs` solo monta:
```rust
let app = Router::new()
    .route("/api/health", get(health_check))
    .route("/api/server/status", get(server_status));
```
Sin embargo, existen módulos completos como `resource_pool_crud.rs` y `observability_api.rs` que definen sus propios `Router` pero no son invocados.

### 3.2. Discrepancia de Nombres (Naming Drift)
*   **Concepto:** Grupos de Workers.
    *   **Backend:** Usa el término `ResourcePool` y la ruta `/resource-pools`.
    *   **Frontend:** Usa el término `WorkerPool` y la ruta `/api/worker-pools`.
    *   **Acción:** Estandarizar a `WorkerPool` (más semántico para el usuario) o `ResourcePool` (más genérico).

### 3.3. Discrepancia de Métodos
*   **Update Pool:** Frontend envía `PATCH` (actualización parcial), Backend espera `PUT` (reemplazo o actualización, aunque la implementación parece soportar parciales, el verbo HTTP no coincide).

## 4. Propuesta de Consolidación

1.  **Centralizar Rutas:** Crear un módulo `server/src/routes.rs` que agregue todos los sub-routers (`resource_pool_crud`, `observability`, etc.) y montarlo en `main.rs`.
2.  **Estandarizar API Prefix:** Decidir si usar `/api/v1/` o `/api/`. El contrato de Observability usa `/api/v1/`, pero `main.rs` usa `/api/`.
3.  **Implementar OpenAPI Faltante:** Añadir macros `#[utoipa::path]` a `resource_pool_crud.rs` y otros controladores para generar el contrato real.
4.  **Sincronizar Frontend:** Actualizar `workersApi.ts` para coincidir con las rutas del backend una vez corregidas (ej: `/api/resource-pools`).

## 5. Próximos Pasos Inmediatos
1.  Modificar `server/src/main.rs` para integrar los routers existentes.
2.  Renombrar rutas en Backend o Frontend para que coincidan (`worker-pools` vs `resource-pools`).
3.  Implementar el endpoint faltante `/api/observability/topology`.
