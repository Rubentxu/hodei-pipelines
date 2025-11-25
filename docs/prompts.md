# Prompts de analisis del proyecto

## Analisis táctico DDD (Catalogación)

Aquí tienes el CODE_MANIFEST.txt en la raiz de mi repositorio. Este es el mapa de la verdad. No alucines archivos que no están aquí. Úsalo como índice para todas las búsquedas que te pida a continuación.

Actúa como un experto en DDD. Quiero que analices los archivos de código que estan referenciados en CODE_MANIFEST.txt uno por uno (ve fichero a fichero progresivamente en lotes, no te dejes ninguna). y vayas creando progresivamente un documento de análisis táctico en formato Markdown en el directorio `docs/analysis`.

**Tu tarea:** Rellena la siguiente tabla de 'Análisis Táctico'.
**Reglas de Oro:**
1. **Calidad sobre eficiencia:** No intentes resumir. Lee cada archivo línea por línea.
2. **Justificación:** Si marcas algo como 'Value Object', asegúrate de que es inmutable y tiene igualdad por valor. Si no, márcalo como 'Sospechoso' en la descripción.
3. **Extracción de Reglas:** En la columna 'Business Rules', extrae solo lo que veas explícitamente en el código (ifs, guards, throws), no supongas nada por el nombre de la clase.
4. **Comments:** Realiza comentarios sobre indicios de mejoras en el código, o errores de diseño, smelly code, y sugerencias de refactorización. No siempre tienes que hacerlo, solo cuando encuentres algo que pueda mejorar.

| No. | File Path | Layer | DDD Type | Description | Business Rules Implemented | Invariants / Validations | Comments |


---

### Propuesta de Mejora sobre el análisis táctico.

1. Segun el análisis táctico quiero realices unas propuestas de mejora en el código, para mejorar la calidad y eficiencia del mismo y lo persistas en un documento markdown en docs/analysis. Ten en cuenta que puedes investigar y sugerir mejoras en el código, apoyate en la documentación de DDD y en los principios SOLID. MCP como perplexity o cargo docs para buscar mejores soluciones. No te dejes ninguna linea que tenga errores de diseño.

2. Implementa las propuestas de mejora definidas en el documento, asegurate que los tests a los que pueda afectar se actualicen y pasen correctamente junto con los cambios en el código que se realicen.

---

## Análisis de Testing y Cobertura

Aquí tienes el CODE_MANIFEST.txt en la raiz de mi repositorio. Este es el mapa de la verdad. No alucines archivos que no están aquí. Úsalo como índice para todas las búsquedas que te pida a continuación.

Actúa como un experto en testing y QA. Basándome en el los ficheros mencionados en  CODE_MANIFEST.txt (ve fichero a fichero progresivamente en lotes, no te dejes ninguna), analiza la estructura de testing del proyecto y vayas creando progresivamente un documento de análisis de testing en formato Markdown en el directorio `docs/analysis` con el formato:

**Tu tarea:** Rellena la siguiente tabla de 'Análisis de Testing'.

**Reglas de Oro:**
1. **Cobertura Real:** Analiza la cobertura real de tests por componente DDD
2. **Pirámide de Tests:** Clasifica los tests según la pirámide (Unit, Integration, E2E)
3. **Calidad de Tests:** Evalúa la calidad, mantenibilidad y efectividad de los tests
4. **Gaps de Testing:** Identifica áreas sin cobertura o con cobertura insuficiente
5. **Mejoras:** Propone mejoras en la estrategia de testing

**Tabla de Análisis de Testing:**
| No. | Component Path | Test Type (Unit/Integration/E2E) | Test Coverage % | Test Quality Score | Missing Scenarios | Testing Strategy Issues | Improvement Recommendations |
|-----|----------------|----------------------------------|-----------------|--------------------|-------------------|-------------------------|------------------------------|
| 1   |                |                                  |                 |                    |                   |                         |                              |

**Además, genera:**
1. **Resumen de Pirámide de Tests:** Distribución actual vs. ideal
2. **Mapa de Cobertura:** Visualización de áreas cubiertas y gaps
3. **Plan de Mejora:** Roadmap para mejorar la estrategia de testing
4. **Métricas Clave:** KPIs de calidad y cobertura

---
### Propuesta de Mejora sobre el análisis de Tests.

Segun el análisis de tests quiero realices unas propuestas de mejora en el código de testing, para mejorar la calidad y eficiencia del mismo. Ten en cuenta que puedes investigar y sugerir mejoras en el código, apoyate en la documentación de BDD y en las mejores practicas de Testing. Utiliza MCP como perplexity o cargo docs para investigar mejores soluciones si lo necesitas.
IMPORTANTE: todos los cambios seran extrictamente en codigo de testing, no se toca codigo productivo, a menos que sea para corrigir un error detectado en la logica de negocio. 


---


## Análisis de Rendimiento y Optimización

Aquí tienes el CODE_MANIFEST.txt en la raiz de mi repositorio. Este es el mapa de la verdad. No alucines archivos que no están aquí. Úsalo como índice para todas las búsquedas que te pida a continuación.
Actúa como un experto en Performance Engineering. Basándome en el CODE_MANIFEST.txt (ve fichero a fichero progresivamente en lotes, no te dejes ninguna), analiza el código para identificar bottlenecks y oportunidades de optimización y vayas creando progresivamente un documento de análisis de Rendimiento en formato Markdown en el directorio `docs/analysis` con el formato:

**Tu tarea:** Rellena la siguiente tabla de 'Análisis de Rendimiento'.
**Reglas de Oro:**
1. **Algorithmic Complexity:** Analiza la complejidad temporal y espacial
2. **Database Queries:** Identifica queries problemáticas (N+1, missing indexes)
3. **Memory Leaks:** Detecta potenciales memory leaks
4. **I/O Operations:** Evalúa operaciones de entrada/salida
5. **Concurrency Issues:** Identifica problemas de concurrencia
**Tabla de Análisis de Rendimiento:**
| No. | Component Path | Complexity O() | Memory Usage | DB Query Issues | I/O Bottleneck | Concurrency Risk | Performance Score | Optimization Potential |
|-----|----------------|----------------|--------------|-----------------|----------------|------------------|-------------------|------------------------|
| 1   |                |                |              |                 |                |                  |                   |                        |
**Además, genera:**
1. **Performance Heatmap:** Visualización de puntos calientes
2. **Optimization Roadmap:** Plan de optimizaciones priorizadas
3. **Benchmarking Strategy:** Estrategia de medición de performance
4. **Scaling Recommendations:** Recomendaciones de escalabilidad

---
### Propuesta de Mejora sobre el análisis de Rendimiento.
Revisa este analisis y realia las mejoras necesarias para el codigo de nuestra aplicacion y que no tenga los posibles problemas detectados en el analisis. Ten en cuenta que puedes investigar y sugerir mejoras en el código, apoyate en la documentación de internet y en las mejores practicas de Rendimiento en rust. Usa perplexity para generar investigar si te hace falta.

---

Para aplicar el **"Movimiento de Pinza"** (Pincer Movement) que explica Fernando, la generación del **Bounded Context Canvas** no debe hacerse "en frío" solo leyendo el código. Se debe hacer **utilizando la información táctica que extrajiste previamente** (el Manifiesto de Código y la Tabla de Análisis Táctico de clases/ficheros).

Si intentas generar el Canvas directamente desde el código sin el paso intermedio, el LLM alucinará o será demasiado genérico ("Este es un sistema de gestión de usuarios").

Aquí tienes el Prompt Maestro y la Template exacta basada en las diapositivas del video.

---


### El Prompt para el Agente IA Bounded Context Canvas (Markdown)

> **Rol:** Actúa como un Arquitecto de Software experto en Domain-Driven Design (DDD) Estratégico.
>
> **Objetivo:** Vamos a realizar un "Movimiento de Pinza" (Pincer Movement). Usando la evidencia de bajo nivel (el código y el análisis táctico que tienes en contexto), vas a construir la definición estratégica de alto nivel: el **Bounded Context Canvas**.
>
> **Instrucciones CRÍTICAS (Anti-Alucinación):**
> 1.  **Evidencia sobre Creatividad:** No inventes un propósito de negocio que suene bien. Deduce el propósito basándote estrictamente en lo que el código *hace*, no en lo que *debería hacer*.
> 2.  **Lenguaje Ubicuo Real:** En la sección de Lenguaje Ubicuo, solo puedes listar términos que aparezcan explícitamente en el código (nombres de Clases, Enums o Métodos públicos). Si en el código se llama `Client` y no `Customer`, usa `Client`.
> 3.  **Detección de Fugas:** En la sección "Strategic Classification", si ves que el código tiene demasiadas dependencias de frameworks o librerías de utilidad, clasifícalo como "Generic" o "Supporting", no como "Core", y explica por qué.
> 4.  **Crítica Socrática:** La sección "Open Questions" es la más importante. Quiero que critiques la arquitectura. Si ves lógica de negocio en un Controlador, o Entidades anémicas (sin métodos de lógica), denúncialo ahí.
>
> **Formato de Salida:**
> Rellena estrictamente la siguiente plantilla Markdown. Si no tienes información para una celda, pon "NO EVIDENCIA EN CÓDIGO", no te lo inventes.
>
```markdown
# Bounded Context Canvas: [Nombre Sugerido del Contexto]

## 1. Strategic Overview
| Section | Description |
|---------|-------------|
| **Purpose** | *¿Para qué existe este contexto? ¿Qué dolor de negocio resuelve? (No técnico)* |
| **Strategic Classification** | *[Core Domain / Supporting Subdomain / Generic Subdomain] - Justifica tu elección.* |
| **Key Stakeholders** | *¿Quién usa esto? (Ej: Administradores, Clientes finales, Sistemas externos)* |

## 2. Ubiquitous Language (Evidence-Based)
*Extrae los términos clave que se repiten en Nombres de Clases, Métodos y Variables.*

| Term | Definition (Based on Code Behavior) | Primary Implementation Files |
|------|-------------------------------------|------------------------------|
| `[Term]` | *[Definición]* | `[Ruta del archivo]` |
| `[Term]` | *[Definición]* | `[Ruta del archivo]` |

## 3. Business Decisions & Policies (The "Rules")
*Qué reglas de negocio complejas o políticas se aplican aquí.*

| Policy Name | Description | Enforced By (Pattern/Class) |
|-------------|-------------|-----------------------------|
| *Ej: Política de Cancelación* | *No se puede cancelar si faltan < 24h* | *Guard Clause en `Booking.java`* |

## 4. Interactions (Flow of Control)
*Cómo se comunica este contexto con el mundo.*

| Direction | Type | Description | Handled By |
|-----------|------|-------------|------------|
| **Inbound** | *[API / Event / Cron]* | *[Qué entra]* | *[Controller/Listener]* |
| **Outbound** | *[Event / DB / API Call]* | *[Qué sale o cambia]* | *[Repository/Publisher]* |

## 5. Assumptions & Verification Metrics
*Cosas que el código asume que son ciertas.*

* **Assumption:** [Ej: El usuario siempre tiene un email válido antes de llegar aquí]
* **Metric:** [Ej: Latencia esperada o volumen de datos]

## 6. Open Questions & Tech Debt (Socratic Critique)
*Aquí es donde aplicas el juicio crítico. ¿Qué huele mal?*

* [ ] *Ej: El término "Manager" es ambiguo en `UserManager.java`. Parece un God Object.*
* [ ] *Ej: Reglas de negocio filtradas en la capa de Infraestructura en lugar del Dominio.*
```

---

### 3. Cómo completar y refinar el Canvas (El toque humano)

Fernando explica que el LLM te dará un borrador "decente", pero tú debes aplicar el **Diálogo Socrático** para pulirlo. Una vez que el agente genere el Canvas, usa estos "Contra-Prompts" para mejorarlo:

**Para validar la Clasificación Estratégica:**
> "Has clasificado este contexto como 'Core Domain'. Sin embargo, veo que el 80% del código son operaciones CRUD simples sin reglas complejas en la tabla de Business Policies. ¿Estás seguro de que no es un 'Supporting Subdomain'? Reevalúa tu respuesta basándote en la complejidad ciclomática de las entidades."

**Para pulir el Lenguaje Ubicuo:**
> "En la tabla de Lenguaje Ubicuo has puesto 'Pedido'. Pero mirando el Manifiesto de Código, veo un fichero `Order.java` y otro `PurchaseRequest.java`. ¿Cuál de los dos es el término real del dominio y cuál es un DTO? Corrige la tabla."

**Para las Interacciones:**
> "En 'Inbound Interactions' solo has puesto la API REST. Revisa el código buscando anotaciones de `@KafkaListener` o `RabbitListener` o implementaciones de `Consumer`. ¿Hay eventos de dominio entrando que te has perdido?"

### Resumen del Flujo
1.  **Input:** Manifiesto + Código.
2.  **Paso 1:** Generar Tabla Táctica (Fichero a fichero).
3.  **Paso 2:** Usar el Prompt de arriba + Template para generar el Canvas Estratégico.
4.  **Paso 3:** Discutir con la IA (Socrático) para corregir las asunciones erróneas.

Este proceso transforma un montón de código ilegible en un mapa estratégico que puedes usar para explicar el sistema a un Product Owner o a un nuevo desarrollador.