# Guía de Automatización y Postman para la API de Hodei

Esta guía detalla cómo utilizar la colección de Postman mejorada para pruebas manuales y automatización en CI/CD.

## 1. Resumen de la Colección

La colección `hodei_api_postman_collection.json` está organizada en tres secciones principales:

1.  **Worker Pools (Pools de Workers)**: Operaciones CRUD para gestionar los pools de recursos.
2.  **E2E Pipeline Scenario (Escenario Pipeline E2E)**: Un flujo de trabajo secuencial (Crear -> Ejecutar -> Verificar -> Borrar) diseñado para probar el ciclo de vida completo de un pipeline.
3.  **Negative Tests (Pruebas Negativas)**: Peticiones diseñadas para fallar (ej. 404 Not Found, 400 Bad Request) para verificar el manejo de errores.

## 2. Inicio Rápido (Manual)

1.  **Importar**: Arrastra `docs/testing/hodei_api_postman_collection.json` a Postman.
2.  **Configurar**: Establece la variable de colección `baseUrl` (por defecto: `http://localhost:8080`).
3.  **Ejecutar**:
    *   Abre la carpeta de la colección.
    *   Haz clic en **Run** (Ejecutar).
    *   Selecciona "Run manually" (Ejecutar manualmente) y haz clic en **Run Hodei API (Enhanced)**.

## 3. Automatización con Newman (CLI)

[Newman](https://www.npmjs.com/package/newman) es el ejecutor de colecciones de línea de comandos para Postman. Te permite ejecutar estas pruebas en un pipeline de CI/CD.

### Prerrequisitos
*   Node.js (>= v10)
*   npm

### Instalación
```bash
npm install -g newman
```

### Ejecutar Pruebas
Para ejecutar toda la colección desde la línea de comandos:

```bash
newman run docs/testing/hodei_api_postman_collection.json \
  --env-var "baseUrl=http://localhost:8080" \
  --reporters cli,json \
  --reporter-json-export docs/testing/report.json
```

**Explicación de Opciones:**
*   `--env-var "baseUrl=..."`: Sobrescribe la URL base para el entorno de destino.
*   `--reporters cli,json`: Muestra los resultados en la consola y guarda un informe JSON.
*   `--reporter-json-export`: Especifica dónde guardar el informe JSON.

### Ejemplo de Integración CI/CD (GitHub Actions)

Añade este paso a tu `.github/workflows/test.yml`:

```yaml
- name: Run API Tests
  run: |
    npm install -g newman
    newman run docs/testing/hodei_api_postman_collection.json --env-var "baseUrl=http://localhost:8080"
```

## 4. Detalles de las Pruebas

### Validación de Esquemas
Utilizamos `tv4` (Tiny Validator 4) dentro de los scripts de Postman para validar que las respuestas JSON coincidan con el esquema esperado.
*   *Ejemplo:* Verificar que una respuesta de "Crear Pool" contenga una cadena `id` y un objeto `config`.

### Variables Dinámicas
La colección tiene estado. Captura IDs de las respuestas y los guarda en **Variables de Colección**.
*   `poolId`: ID del pool creado en la carpeta "Worker Pools".
*   `e2ePipelineId`: ID del pipeline creado en el "E2E Scenario".
*   `e2eExecutionId`: ID de la ejecución iniciada en el "E2E Scenario".

**Nota:** Si ejecutas las peticiones fuera de orden manualmente, estas variables podrían faltar. Se recomienda ejecutar las carpetas secuencialmente.
