# Log Streaming E2E Tests

Este directorio contiene tests End-to-End para validar la funcionalidad de **log streaming en tiempo real** con API inspirada en Docker/kubectl.

## 📋 Requisitos

Antes de ejecutar estos tests, necesitas:

1. **worker-lifecycle-manager ejecutándose** en el puerto 8082
2. **Puerto 8082 disponible** (sin otros servicios corriendo)

## 🚀 Cómo Ejecutar los Tests

### Opción 1: Script Automático (Recomendado)

```bash
# Hacer el script ejecutable
chmod +x run_log_streaming_tests.sh

# Ejecutar (el script verificará que el servicio esté corriendo)
./run_log_streaming_tests.sh
```

### Opción 2: Manual

```bash
# Terminal 1: Iniciar el servicio
cargo run -p hodei-worker-lifecycle-manager

# Terminal 2: Ejecutar los tests
cargo test -p e2e-tests --test log_streaming_test
```

### Opción 3: Tests Individuales

```bash
# Solo test de logs históricos
cargo test -p e2e-tests --test log_streaming_test test_sse_historical_logs

# Solo test de streaming en tiempo real
cargo test -p e2e-tests --test log_streaming_test test_realtime_log_streaming

# Solo test de timestamps
cargo test -p e2e-tests --test log_streaming_test test_log_streaming_with_timestamps

# Solo test de tail
cargo test -p e2e-tests --test log_streaming_test test_log_streaming_with_tail

# Solo test de múltiples suscriptores
cargo test -p e2e-tests --test log_streaming_test test_multiple_concurrent_subscribers
```

## 🧪 Tests Incluidos

### 1. `test_sse_historical_logs`
- ✅ Valida conexión SSE
- ✅ Obtiene logs históricos
- ✅ Verifica parsing de eventos
- ✅ Valida contenido de logs

### 2. `test_realtime_log_streaming`
- ✅ Streaming en tiempo real con `follow=true`
- ✅ Recibe eventos live durante ejecución
- ✅ Verifica formato de eventos SSE

### 3. `test_log_streaming_with_timestamps`
- ✅ Parámetro `timestamps=true`
- ✅ Formato RFC3339 válido
- ✅ Campos requeridos (timestamp, stream, line)

### 4. `test_log_streaming_with_tail`
- ✅ Parámetro `tail=N`
- ✅ Límite de líneas aplicado
- ✅ Obtiene las últimas líneas correctas

### 5. `test_multiple_concurrent_subscribers`
- ✅ Múltiples conexiones simultáneas
- ✅ Todos los suscriptores reciben eventos
- ✅ Broadcast channels funcionan

## 🔧 Troubleshooting

### Error: "SSE endpoint should be accessible"

**Causa**: El worker-lifecycle-manager no está corriendo.

**Solución**:
```bash
# Verificar que el puerto está libre
lsof -i :8082

# Si hay algo corriendo, terminarlo
kill -9 $(lsof -t -i:8082)

# Iniciar el servicio
cargo run -p hodei-worker-lifecycle-manager
```

### Error: "Connection refused"

**Causa**: El servicio está corriendo en un puerto diferente.

**Solución**: Verificar que el servicio usa el puerto 8082:
```bash
# El servicio debe mostrar algo como:
# Starting server on 0.0.0.0:8082
```

### Tests timeout

**Causa**: El servicio es muy lento o está sobrecargado.

**Solución**:
```bash
# Ejecutar con logs detallados
RUST_LOG=debug cargo test -p e2e-tests --test log_streaming_test test_sse_historical_logs
```

## 📊 APIs Probadas

Los tests validan estos endpoints:

### Ejecutar Job
```http
POST http://localhost:8082/api/v1/execute
Content-Type: application/json

{
  "command": "echo 'test'"
}
```

### Stream Logs (SSE)
```http
GET http://localhost:8082/api/v1/executions/{execution_id}/logs/stream?follow=true&tail=50&timestamps=true
```

### Parámetros Validados:
- ✅ `follow` (bool)
- ✅ `tail` (int)
- ✅ `since` (timestamp o duración)
- ✅ `until` (timestamp o duración)
- ✅ `timestamps` (bool)
- ✅ `stream` (stdout/stderr/all)
- ✅ `page`, `page_size` (paginación)

## 🎯 Ejemplo de Uso Manual

```bash
# 1. Iniciar servicio
cargo run -p hodei-worker-lifecycle-manager

# 2. En otra terminal, ejecutar job
curl -X POST http://localhost:8082/api/v1/execute \
  -H "Content-Type: application/json" \
  -d '{"command":"for i in {1..5}; do echo \"Line \$i\"; sleep 0.5; done"}'

# 3. Obtener execution_id de la respuesta y hacer stream
curl -N "http://localhost:8082/api/v1/executions/{execution_id}/logs/stream?follow=true"

# 4. Ver logs con filtros
curl "http://localhost:8082/api/v1/executions/{execution_id}/logs/stream?tail=10&stream=stderr&timestamps=true"
```

## 📝 Formato de Eventos SSE

Los logs se envían como eventos SSE:

```
data: {"timestamp":"2024-01-15T10:30:15Z","stream":"stdout","line":"Starting execution..."}

data: {"timestamp":"2024-01-15T10:30:16Z","stream":"stderr","line":"Error: Connection failed"}

data: {"timestamp":"2024-01-15T10:30:16Z","stream":"stdout","line":"Retrying..."}
```

## ✅ Criterios de Éxito

Todos los tests deben:
1. ✅ Conectar exitosamente al servicio
2. ✅ Ejecutar jobs sin errores
3. ✅ Recibir eventos SSE válidos
4. ✅ Parsear JSON correctamente
5. ✅ Validar campos requeridos
6. ✅ Verificar filtros y parámetros

## 🔗 Referencias

- [Docker Logs CLI](https://docs.docker.com/engine/reference/commandline/logs/)
- [kubectl Logs](https://kubernetes.io/docs/reference/generated/kubectl/kubectl-commands#logs)
- [Server-Sent Events (SSE)](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)
- [Axum SSE Documentation](https://docs.rs/axum/latest/axum/response/sse/)

---

## 📞 Soporte

Si encuentras problemas con los tests:

1. Verifica que el servicio esté corriendo: `curl http://localhost:8082/health`
2. Revisa los logs del servicio: `RUST_LOG=debug cargo run -p hodei-worker-lifecycle-manager`
3. Ejecuta tests individuales para debuggear: `cargo test test_sse_historical_logs`
4. Verifica el puerto: `lsof -i :8082`
