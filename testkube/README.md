# TestKube Integration - Hodei Pipelines

Este directorio contiene la configuración completa de TestKube para testing automatizado de Hodei Pipelines.

## 📁 Estructura

```
testkube/
├── tests/                      # Definiciones de tests individuales
│   ├── test-server-health.yaml
│   ├── test-server-metrics.yaml
│   ├── test-database-connectivity.yaml
│   ├── test-nats-connectivity.yaml
│   ├── test-api-endpoints.yaml
│   ├── test-worker-functionality.yaml
│   ├── test-performance.yaml
│   └── test-stress.yaml
├── test-suites/               # Suites de tests (agrupaciones)
│   ├── hodei-smoke-tests.yaml
│   ├── hodei-integration-tests.yaml
│   └── hodei-full-suite.yaml
├── executors/                 # Ejecutores personalizados
│   └── hodei-server-executor.yaml
├── scripts/                   # Scripts de utilidad
│   ├── setup-testkube.sh
│   ├── deploy-tests.sh
│   ├── run-tests.sh
│   └── cleanup-tests.sh
├── config/                    # Configuraciones
│   └── testkube-values.yaml
└── README.md                  # Esta documentación
```

## 🚀 Inicio Rápido

### 1. Instalar TestKube

```bash
# Usar el script de instalación
./testkube/scripts/setup-testkube.sh

# O manualmente
helm repo add testkube https://kubeshop.github.io/helm-charts
helm install testkube testkube/testkube --namespace testkube --create-namespace
```

### 2. Desplegar Tests

```bash
# Desplegar todos los tests
./testkube/scripts/deploy-tests.sh

# O manualmente
kubectl apply -f testkube/tests/ -n testkube
kubectl apply -f testkube/test-suites/ -n testkube
kubectl apply -f testkube/executors/ -n testkube
```

### 3. Ejecutar Tests

```bash
# Ejecutar suite de smoke tests
./testkube/scripts/run-tests.sh --suite hodei-smoke-tests

# Ejecutar test específico
./testkube/scripts/run-tests.sh --test server-health-check

# Ver todos los tests disponibles
./testkube/scripts/run-tests.sh --list
```

## 📊 Tipos de Tests

### Tests de Salud (Health Checks)

**test-server-health.yaml**
- Verifica que el endpoint `/health` responde correctamente
- Verifica disponibilidad del servidor
- Ejecutado en cada deployment

**test-server-metrics.yaml**
- Verifica que el endpoint `/metrics` está disponible
- Verifica métricas de Prometheus
- Ejecutado en tests de smoke

### Tests de Conectividad

**test-database-connectivity.yaml**
- Verifica conexión a PostgreSQL
- Usa `pg_isready` para verificar disponibilidad
- Ejecutado en tests de integración

**test-nats-connectivity.yaml**
- Verifica conexión a NATS
- Verifica JetStream si está habilitado
- Ejecutado en tests de integración

### Tests Funcionales

**test-api-endpoints.yaml**
- Verifica múltiples endpoints de la API
- Testa `/health`, `/metrics`, `/api/v1/jobs`
- Verifica códigos de respuesta HTTP
- Ejecutado en smoke tests

**test-worker-functionality.yaml**
- Verifica que pods de workers están ejecutándose
- Revisa logs en busca de errores
- Verifica readiness de workers
- Ejecutado en tests de integración

### Tests de Performance

**test-performance.yaml**
- Test de carga con k6
- Configurable (VUs, duración)
- Genera métricas de performance
- Ejecutado en tests completos

**test-stress.yaml**
- Test de estrés con requests concurrentes
- Verifica estabilidad bajo carga
- Ejecutado en tests completos

## 🔧 Test Suites

### hodei-smoke-tests

Suite básica que se ejecuta en cada deployment para verificar funcionalidad crítica.

```bash
kubectl testkube run testsuite hodei-smoke-tests -n testkube --watch
```

**Incluye:**
- test-server-health-check
- test-server-metrics-check
- test-api-endpoints-validation

### hodei-integration-tests

Suite de integración que verifica interacción entre componentes.

```bash
kubectl testkube run testsuite hodei-integration-tests -n testkube --watch
```

**Incluye:**
- test-database-connectivity
- test-nats-connectivity
- test-server-health-check
- test-worker-functionality-check
- test-api-endpoints-validation

### hodei-full-suite

Suite completa que incluye todos los tests.

```bash
kubectl testkube run testsuite hodei-full-suite -n testkube --watch
```

**Incluye:**
- Todos los tests de integración
- Tests de performance
- Tests de stress (opcional)

## 🔄 CI/CD Integration

### En el CI/CD Pipeline

Agregar al archivo `deployment/scripts/ci-cd.sh`:

```bash
# Ejecutar tests con TestKube
./deployment/scripts/testkube-integration.sh
```

### Variables de Entorno

```bash
# Nivel de tests (fast, full)
export CI_TEST_LEVEL=fast

# Namespace de TestKube
export NAMESPACE=testkube

# Saltar tests
export SKIP_TESTS=false
```

### GitHub Actions Example

```yaml
- name: Run TestKube Tests
  run: |
    ./testkube/scripts/deploy-tests.sh
    ./testkube/scripts/run-tests.sh --suite hodei-smoke-tests
  env:
    NAMESPACE: testkube
```

## 📈 Monitoreo y Observabilidad

### TestKube Dashboard

```bash
# Acceder al dashboard
kubectl port-forward svc/testkube-dashboard 8088:8088 -n testkube
# Abrir: http://localhost:8088
```

### Ver Resultados

```bash
# Ver todas las ejecuciones
kubectl testkube get testexecutions -n testkube

# Ver ejecución específica
kubectl testkube get testexecution <execution-id> -n testkube

# Ver últimos resultados
kubectl testkube get testsuiteexecution -n testkube --latest
```

### Exportar Resultados

```bash
# Exportar resultados en formato JUnit
kubectl testkube get testsuiteexecution <suite-id> -n testkube --junit

# Exportar métricas
kubectl testkube get testexecution <execution-id> -n testkube --metrics
```

## 🎛️ Configuración Avanzada

### Variables por Test

Cada test puede tener variables personalizadas:

```yaml
executionRequest:
  variables:
    SERVER_URL:
      name: SERVER_URL
      value: "http://hodei-server:8080"
      type: basic
    TIMEOUT:
      name: TIMEOUT
      value: "30"
      type: basic
```

### Configurar Retries

```yaml
executionRequest:
  retryCount: 3
  retryInterval: 5s
```

### Configurar Timeout

```yaml
executionRequest:
  timeout: 300s
```

### Configurar Artifact Collection

```yaml
executionRequest:
  artifactRequest:
    type: junit
    volumeMountPath: /share
```

## 🛠️ Crear Tests Personalizados

### 1. Crear Test Definition

```yaml
# test-custom.yaml
apiVersion: tests.testkube.io/v3
kind: Test
metadata:
  name: custom-test
  namespace: testkube
spec:
  type: curl
  content:
    type: inline
    data: |
      #!/bin/sh
      # Tu script de test aquí
      curl -f $ENDPOINT
      exit $?
  executionRequest:
    variables:
      ENDPOINT:
        name: ENDPOINT
        value: "http://hodei-server:8080/health"
        type: basic
```

### 2. Aplicar el Test

```bash
kubectl apply -f test-custom.yaml -n testkube
```

### 3. Ejecutar el Test

```bash
kubectl testkube run test custom-test -n testkube --watch
```

## 🔍 Troubleshooting

### Test Falla - Debug

```bash
# Ver logs del test
kubectl testkube get testexecution <execution-id> -n testkube --logs

# Ver logs del pod
kubectl logs -n testkube <pod-name>

# Ver eventos
kubectl get events -n testkube --sort-by='.lastTimestamp'
```

### Test Stuck

```bash
# Cancelar ejecución
kubectl testkube abort testexecution <execution-id> -n testkube

# Eliminar test execution
kubectl delete testexecution <execution-id> -n testkube
```

### Problemas de Permisos

```bash
# Verificar RBAC
kubectl auth can-i create testexecutions -n testkube

# Verificar ServiceAccount
kubectl get sa testkube-api-server -n testkube -o yaml
```

### Problemas de Red

```bash
# Verificar conectividad
kubectl exec -n testkube <test-pod> -- curl -f http://hodei-server:8080/health

# Verificar DNS
kubectl exec -n testkube <test-pod> -- nslookup hodei-server.hodei-jobs.svc.cluster.local
```

## 📚 Recursos Adicionales

- [Documentación TestKube](https://docs.testkube.io/)
- [Tipos de Tests Soportados](https://docs.testkube.io/category/test-types)
- [Executors](https://docs.testkube.io/category/executors)
- [Test Suites](https://docs.testkube.io/articles/test-suites)
- [CI/CD Integration](https://docs.testkube.io/articles/cicd-overview)

## 🆘 Soporte

Para issues y soporte:
- 📖 Documentación: https://docs.testkube.io/
- 🐛 Issues: https://github.com/kubeshop/testkube/issues
- 💬 Slack: TestKube Community

## ✅ Checklist de Implementación

- [ ] TestKube instalado en el cluster
- [ ] Tests desplegados al namespace de testkube
- [ ] Tests ejecutándose correctamente
- [ ] CI/CD integrado con TestKube
- [ ] Dashboards configurados
- [ ] Alertas configuradas para fallan tests
- [ ] Documentación actualizada
- [ ] Runbooks creados
- [ ] Permisos verificados
- [ ] Variables de entorno configuradas
