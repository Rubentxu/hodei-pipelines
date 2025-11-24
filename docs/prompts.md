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

Segun el análisis táctico quiero realices unas propuestas de mejora en el código, para mejorar la calidad y eficiencia del mismo. Ten en cuenta que puedes investigar y sugerir mejoras en el código, apoyate en la documentación de DDD y en los principios SOLID. MCP como perplexity o cargo docs para buscar mejores soluciones.

---

## Análisis de Testing y Cobertura

Aquí tienes el CODE_MANIFEST.txt en la raiz de mi repositorio. Este es el mapa de la verdad. No alucines archivos que no están aquí. Úsalo como índice para todas las búsquedas que te pida a continuación.

Actúa como un experto en testing y QA. Basándome en el los ficheros mencionados en  CODE_MANIFEST.txt (ve fichero a fichero progresivamente en lotes, no te dejes ninguna), analiza la estructura de testing del proyecto y genera un informe completo sobre:

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


## Análisis de Rendimiento y Optimización

Aquí tienes el CODE_MANIFEST.txt en la raiz de mi repositorio. Este es el mapa de la verdad. No alucines archivos que no están aquí. Úsalo como índice para todas las búsquedas que te pida a continuación.
Actúa como un experto en Performance Engineering. Basándome en el CODE_MANIFEST.txt (ve fichero a fichero progresivamente en lotes, no te dejes ninguna), analiza el código para identificar bottlenecks y oportunidades de optimización:

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

