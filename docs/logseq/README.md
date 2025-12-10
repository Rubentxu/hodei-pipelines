# Logseq DDD Notes - Hodei Jobs Platform

## 📚 Descripción

Este directorio contiene las notas de Logseq para documentar el diseño DDD (Domain-Driven Design) del proyecto **Hodei Jobs Platform**. Las notas están organizadas según las plantillas DDD estándar y cubren todos los elementos arquitectónicos principales.

## 📋 Índice de Notas

### 🎯 Project & Overview
- `01-project-dashboard.md` - Dashboard principal del proyecto
- `99-context-map-overview.md` - Mapa de contextos completo (¡LEE ESTO PRIMERO!)

### 🏛️ Bounded Contexts
- `02-bc-pipeline-execution.md` - Orquestación de pipelines y jobs
- `03-bc-scheduling.md` - Gestión y scheduling de workers
- `04-bc-resource-governance.md` - Control de recursos y cuotas
- `05-bc-identity-access.md` - Autenticación y RBAC
- `06-bc-observability.md` - Métricas y monitoreo
- `20-bc-job-context.md` - Especificaciones de jobs

### 🧩 Aggregates
- `07-agg-pipeline.md` - Aggregate Pipeline con DAG
- `08-agg-job.md` - Aggregate Job con state machine
- `09-agg-worker.md` - Aggregate Worker
- `21-agg-resource-pool.md` - Aggregate ResourcePool

### 📦 Value Objects
- `10-vo-pipeline-id.md` - PipelineId (UUID)
- `11-vo-job-state.md` - JobState (state machine)
- `12-vo-worker-id.md` - WorkerId (UUID)
- `22-vo-job-spec.md` - JobSpec (specification)
- `23-vo-resource-quota.md` - ResourceQuota
- `24-vo-worker-capabilities.md` - WorkerCapabilities

### ⚙️ Use Cases
- `13-usecase-pipeline-orchestration.md` - Orquestación de pipelines
- `14-usecase-job-scheduling.md` - Scheduling de jobs
- `15-usecase-resource-allocation.md` - Asignación de recursos
- `25-usecase-worker-management.md` - Gestión de workers
- `26-usecase-rbac-service.md` - Servicio RBAC
- `27-usecase-metrics-collection.md` - Recolección de métricas

### 🔌 Architecture Ports & Adapters
- `16-port-job-repository.md` - Port JobRepository
- `17-port-scheduler.md` - Port SchedulerPort
- `28-port-worker-repository.md` - Port WorkerRepository
- `29-port-pipeline-repository.md` - Port PipelineRepository
- `18-adapter-postgresql-job-repo.md` - Adapter PostgreSQL
- `19-adapter-docker-provider.md` - Adapter Docker Provider
- `30-adapter-nats-event-bus.md` - Adapter NATS Event Bus

### 🏗️ Domain Services
- `32-domain-service-pipeline-orchestrator.md` - PipelineOrchestrator
- `33-domain-service-resource-controller.md` - ResourceController

### 📄 Entities
- `34-entity-pipeline-step.md` - PipelineStep Entity
- `35-entity-pipeline-execution.md` - PipelineExecution Entity

### 🧠 Shared Kernel
- `31-shared-kernel.md` - Shared Kernel con tipos comunes

## 🚀 Cómo Usar las Notas

### 1. **Navegación con Queries**
Cada nota contiene queries de Logseq que te permiten navegar automáticamente:
- Agregados en un contexto
- Adaptadores que implementan un port
- Elementos del shared kernel

### 2. **Enlaces Bidireccionales**
Las notas están interconectadas con `[[...]]` links para navegación fácil:
- Sigue los links de `[[Pipeline]]` para ver el aggregate
- Click en `[[JobRepository]]` para ver el port
- Navega a `[[PostgreSQL]]` para ver la implementación

### 3. **Metadatos para Filtrado**
Cada nota tiene metadatos útiles:
- `ddd-type`: Tipo de elemento DDD (Aggregate, Entity, Value Object, etc.)
- `context`: Bounded context al que pertenece
- `layer`: Capa arquitectónica (Domain, Application, Infrastructure)

### 4. **Ejemplo de Query Personalizada**
```javascript
// En Logseq, puedes crear queries como:
{:title "Todos los Aggregates"
 :query [:find (pull ?b [*])
         :where [?b :block/properties ?p]
                [(get ?p :ddd-type) ?t]
                [(= ?t "[[Aggregate]]")]]}
```

## 🎯 Casos de Uso de las Notas

### Para Desarrolladores
- **Entender el dominio**: Lee los bounded contexts primero
- **Encontrar código**: Usa los links para navegar al código fuente
- **Entender responsabilidades**: Revisa los aggregates y sus invariantes

### Para Arquitectos
- **Analizar acoplamientos**: Revisa el context map
- **Planificar integraciones**: Estudia los ports y adapters
- **Identificar boundaries**: Analiza los bounded contexts

### Para Product Owners
- **Entender el negocio**: Lee los use cases
- **Validar flujos**: Revisa los domain services
- **Conocer limitaciones**: Revisa las invariantes

## 🔄 Actualización de las Notas

Cuando el código cambie:

1. **Actualizar la nota correspondiente** manteniendo la plantilla
2. **Verificar las queries** para asegurar que still funcionan
3. **Actualizar enlaces** si hay cambios en nombres
4. **Revisar el context map** si hay cambios arquitectónicos

## 📖 Plantillas Utilizadas

Las notas siguen estas plantillas DDD estándar:
- `Template: Project/Dashboard`
- `Template: DDD/Strategic/Bounded Context`
- `Template: DDD/Aggregate`
- `Template: DDD/Tactical/Entity`
- `Template: DDD/App/Use Case`
- `Template: Architecture/Port`
- `Template: Architecture/Adapter`

## 💡 Tips

1. **Empieza por el Context Map** (`99-context-map-overview.md`)
2. **Usa la búsqueda de Logseq** para encontrar elementos específicos
3. **Crea tus propias queries** para análisis personalizados
4. **Añade tags personalizados** para tu flujo de trabajo
5. **Exporta a PDF** para documentación externa

## 🤝 Contribución

Para añadir nuevas notas:
1. Usa las plantillas existentes
2. Mantén consistencia en metadatos
3. Añade queries útiles
4. Crea enlaces bidireccionales relevantes
5. Documenta en inglés

---

**Generado**: 2025-12-10
**Basado en**: Análisis de código fuente de Hodei Jobs Platform
**Total de notas**: 36
