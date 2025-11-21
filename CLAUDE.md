## Documentacion en el directorio docs

- Toda la documentacion sobre casos de uso, arquitectura, eventos, contract first etc esta en el directorio docs
- Cada vez que inicies una historia de usuario busca en la documentacion toda la informacion relevante para completar la historia con exito.
- Deberas coger todo el contexto posible de la documentacion que acompaña al proyecto en cada desicion porque en estas esta especificado todas las deciciones y alternativas.

### 

Principios de Organización de Dependencias

1. **Versionado Centralizado**: Todas las versiones en workspace.dependencies
2. **Shared Kernel**: Tipos comunes en `domain-core` y `types-shared`
3. **Infrastructure Abstractions**: Traits en `infrastructure-common`
4. **Testing First**: Helpers en `testing-utils` para TDD

---

## 🏛️ Principios Arquitectónicos Fundamentales

### Arquitectura Hexagonal (Ports & Adapters)

Cada bounded context sigue arquitectura hexagonal:

```
┌─────────────────────────────────────┐
│           APPLICATION               │  ← USECASES
├─────────────────────────────────────┤
│             DOMAIN                  │  ← ENTITIES, VALUEOBJECTS
├─────────────────────────────────────┤
│              CORE                   │  ← DOMAIN SERVICES
├─────────────────────────────────────┤
│  PORTS (traits)  │  ADAPTERS (impls)│  ← INFRASTRUCTURE
└─────────────────────────────────────┘
```

### Principios SOLID Aplicados

#### 1. Single Responsibility Principle (SRP)

- Cada crate tiene una responsabilidad única
- Separación clara de concerns por bounded context
- **Ejemplo**: `worker-management/provider-abstraction` solo define interfaces

#### 2. Open/Closed Principle (OCP)

- Extensible sin modificar código existente
- **Ejemplo**: Nuevos providers de workers implementando trait `WorkerProvider`

#### 3. Liskov Substitution Principle (LSP)

- Implementaciones son intercambiables
- **Ejemplo**: Cualquier `CredentialProvider` puede reemplazar otro

#### 4. Interface Segregation Principle (ISP)

- Interfaces pequeñas y específicas
- **Ejemplo**: `WorkerLifecycleProvider` separado de `WorkerMetricsProvider`

#### 5. Dependency Inversion Principle (DIP)

- Depende de abstracciones, no concreciones
- **Ejemplo**: `Orchestrator` depende de `SchedulerTrait`, no implementación específica

---

#### Estrategia de Testing

1. **Unit Tests** (80% del coverage):

   - Domain entities y value objects
   - Application use cases
   - Pure business logic
2. **Integration Tests** (15% del coverage):

   - Database operations
   - External service calls (mocked)
   - Message queue interactions
3. **Contract Tests** (5% del coverage):

   - API compatibility
   - Event contract validation

### Conventional Commits

Estructura estándar para cada implementación de historia de usuario:

```
tipo(contexto): descripción

detalles

feat(orchestration): implementar orquestador de jobs distribuidos
- Implementar Entity Job con estados y transiciones
- Agregar UseCase ScheduleJob con validaciones
- Configurar NATS JetStream para comunicación
- Tests unitarios con 95% coverage
- Configurar CI/CD para tests automáticos

Refs: #US-001, docs/core_platform_design.md
```

#### Tipos de Commit

- `feat`: Nueva funcionalidad (historia de usuario)
- `fix`: Bugfix
- `refactor`: Refactoring sin cambiar funcionalidad
- `test`: Agregar/modificar tests
- `docs`: Documentación
- `chore`: Configuración, dependencies, etc.

#### Información Contextual

Cada commit debe incluir:

- Referencia a historia de usuario: `Refs: #US-XXX`
- Referencia a documentación: `Refs: docs/xxx_design.md`
- Bounded context afectado: `(contexto)`

---

## 🤖 Patrones Conascense y Análisis de Acoplamientos

#### Patrones de Acoplamiento Detectables

1. **Temporal Coupling**: Components que deben ejecutarse en orden específico
2. **Data Coupling**: Compartir estructuras de datos complejas
3. **Control Coupling**: Pass control information (flags, parameters)
4. **Content Coupling**: Un módulo modifica otro directamente


## ✅ Criterios de Calidad y Definition of Done

### Definition of Done (DoD) por Historia de Usuario

#### Criterios Técnicos Obligatorios

1. **TDD Implementation**:

   - [ ]  Test rojo escrito primero
   - [ ]  Código mínimo para pasar test
   - [ ]  Refactoring sin romper tests
   - [ ]  Coverage mínimo 90%
2. **Architecture Compliance**:

   - [ ]  Sigue arquitectura hexagonal
   - [ ]  Respeto a bounded contexts
   - [ ]  No dependencias circulares
   - [ ]  SOLID principles aplicados
3. **Code Quality**:

   - [ ]  Rust clippy sin warnings
   - [ ]  Documentation completa (pub items)
   - [ ]  Error handling robusto
   - [ ]  Logging estructurado
4. **Performance Criteria**:

   - [ ]  Benchmarks incluidos
   - [ ]  Memory leaks descartados
   - [ ]  Response time dentro de SLA
   - [ ]  Scalability tests pasados

#### Criterios de Negocio

1. **User Story Acceptance**:

   - [ ]  Criterios de aceptación cumplidos
   - [ ]  Demo funcional realizada
   - [ ]  Product Owner approval
   - [ ]  Documentation actualizada
2. **Integration & Deployment**:

   - [ ]  CI/CD pipeline pasa
   - [ ]  Integration tests pasan
   - [ ]  Deployment script actualizado
   - [ ]  Rollback plan disponible
