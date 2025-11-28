# Hodei Pipelines - Despliegue con Helm

Este directorio contiene la configuración completa para desplegar Hodei Pipelines en Kubernetes usando Helm.

## 📁 Estructura

```
deployment/helm/
├── Chart.yaml                          # Chart principal (umbrella)
├── values.yaml                         # Valores por defecto
├── values-dev.yaml                     # Configuración desarrollo
├── values-prod.yaml                    # Configuración producción
├── templates/                          # Templates globales
│   ├── _helpers.tpl
│   └── namespace.yaml
└── charts/                             # Subcharts
    ├── postgresql/                     # PostgreSQL (Bitnami)
    ├── nats/                          # NATS JetStream (NATS)
    ├── hodei-server/                   # Servidor principal
    ├── hwp-agent/                      # Workers
    ├── prometheus/                     # Prometheus (kube-prometheus-stack)
    └── grafana/                        # Grafana
```

## 🚀 Inicio Rápido

### Prerequisitos

- Kubernetes 1.20+
- Helm 3.10+
- kubectl configurado

### Desarrollo

```bash
# Instalar en modo desarrollo
helm install hodei deployment/helm \
  --namespace hodei-dev \
  --create-namespace \
  -f deployment/helm/values-dev.yaml \
  --set hodei-server.image.tag=dev \
  --set hwp-agent.image.tag=dev

# Verificar estado
helm status hodei -n hodei-dev

# Ver logs
kubectl logs -n hodei-dev deployment/hodei-server

# Ver pods
kubectl get pods -n hodei-dev
```

### Producción

```bash
# Instalar en modo producción
helm install hodei deployment/helm \
  --namespace hodei-jobs \
  --create-namespace \
  -f deployment/helm/values-prod.yaml \
  --set hodei-server.image.tag=v0.1.0 \
  --set hwp-agent.image.tag=v0.1.0

# Verificar estado
helm status hodei -n hodei-jobs

# Ver servicios
kubectl get svc -n hodei-jobs

# Ver ingress
kubectl get ingress -n hodei-jobs
```

## 🔧 Configuración

### Variables Principales

#### Base de Datos
```yaml
postgresql:
  auth:
    postgresPassword: "change_me"  # ⚠️ CAMBIAR EN PRODUCCIÓN
  primary:
    persistence:
      size: 100Gi
      storageClass: "fast-ssd"
```

#### NATS JetStream
```yaml
nats:
  nats:
    jetstream:
      enabled: true
      fileStorage:
        size: 50Gi
```

#### Hodei Server
```yaml
hodei-server:
  autoscaling:
    minReplicas: 3
    maxReplicas: 20
  persistence:
    size: 50Gi
```

### Variables de Entorno Críticas

El servidor soporta las siguientes variables de entorno:

#### Base de Datos
- `DATABASE_URL`: URL de conexión PostgreSQL
- `DATABASE_TYPE`: Tipo de base de datos ("postgres" o "inmemory")
- `DATABASE_MAX_CONNECTIONS`: Máximo de conexiones (default: 20)

#### NATS
- `HODEI_NATS_URL`: URL de NATS
- `HODEI_NATS_SUBJECT_PREFIX`: Prefijo para subjects (default: "hodei")
- `HODEI_EVENT_BUS_TYPE`: Tipo de bus ("nats" o "inmemory")

#### Cache (Redb)
- `HODEI_CACHE_PATH`: Ruta del archivo de cache
- `HODEI_CACHE_TTL_SECONDS`: TTL del cache (default: 300)
- `HODEI_CACHE_MAX_ENTRIES`: Máximo de entradas (default: 10000)

#### Kubernetes
- `HODEI_K8S_INSECURE_SKIP_VERIFY`: Saltar verificación TLS
- `HODEI_K8S_CA_PATH`: Ruta del CA certificate

#### Agente
- `HODEI_AGENT_IMAGE`: Imagen del agente (default: "hwp-agent:latest")
- `HODEI_AGENT_PULL_POLICY`: Política de pull de imagen

#### Seguridad
- `HODEI_JWT_SECRET`: Secret para JWT (auto-generado)

#### Observabilidad
- `RUST_LOG`: Nivel de logging (trace, debug, info, warn, error)
- `HODEI_METRICS_ENABLED`: Habilitar métricas
- `HODEI_METRICS_PORT`: Puerto de métricas (default: 9091)

## 🔒 Seguridad

### En Producción

1. **Cambiar todos los secrets**:
   - `postgresql.auth.postgresPassword`
   - `hodei-server.secrets.jwtSecret`
   - `grafana.adminPassword`

2. **Habilitar TLS**:
   - Configurar cert-manager
   - Usar valores-prod.yaml con TLS habilitado

3. **Network Policies**:
   - Habilitadas por defecto en producción
   - Configurar reglas específicas según necesidades

4. **RBAC**:
   - Service accounts con permisos mínimos
   - Revisar ClusterRole y ClusterRoleBinding

### Certificados TLS

Para habilitar TLS en producción:

```yaml
# values-prod.yaml
security:
  tls:
    enabled: true
    certSecretName: "hodei-server-tls"

ingress:
  enabled: true
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
```

## 📊 Monitoreo

### Prometheus

El chart incluye kube-prometheus-stack para monitoreo completo:

```bash
# Verificar ServiceMonitors
kubectl get servicemonitor -n hodei-jobs

# Ver métricas
kubectl port-forward svc/hodei-server 9091:9091 -n hodei-jobs
curl http://localhost:9091/metrics
```

### Grafana

```bash
# Acceder a Grafana
kubectl port-forward svc/grafana 3000:80 -n monitoring

# Credenciales por defecto
# Usuario: admin
# Password: (ver grafana.adminPassword en values)
```

Dashboards incluidos:
- Hodei Server dashboard
- NATS dashboard
- HWP Agent dashboard

## 🔄 Operaciones

### Actualización

```bash
# Actualizar valores
helm upgrade hodei deployment/helm \
  --namespace hodei-jobs \
  -f deployment/helm/values-prod.yaml \
  --set hodei-server.image.tag=v0.1.1

# Ver historial
helm history hodei -n hodei-jobs

# Rollback
helm rollback hodei 1 -n hodei-jobs
```

### Backup y Restore

#### PostgreSQL
```bash
# Backup
kubectl exec -n hodei-jobs deployment/hodei-postgres -- \
  pg_dump -U hodei hodei_jobs > backup.sql

# Restore
kubectl exec -i -n hodei-jobs deployment/hodei-postgres -- \
  psql -U hodei hodei_jobs < backup.sql
```

#### NATS JetStream
```bash
# Backup streams
nats stream add HODEI_EVENTS --subjects 'hodei.>' --storage=file

# Verificar streams
nats stream ls
```

### Scaling

```bash
# Escalar Hodei Server
kubectl scale deployment/hodei-server --replicas=5 -n hodei-jobs

# Escalar HWP Agent
kubectl scale deployment/hwp-agent --replicas=10 -n hodei-jobs

# Ver HPA
kubectl get hpa -n hodei-jobs
```

### Troubleshooting

#### Ver logs
```bash
# Hodei Server
kubectl logs -f deployment/hodei-server -n hodei-jobs

# HWP Agent
kubectl logs -f deployment/hwp-agent -n hodei-jobs

# PostgreSQL
kubectl logs -f statefulset/hodei-postgres -n hodei-jobs

# NATS
kubectl logs -f deployment/hodei-nats -n hodei-jobs
```

#### Verificar estado
```bash
# Health check
kubectl exec -n hodei-jobs deployment/hodei-server -- \
  curl -f http://localhost:8080/health

# Database connection
kubectl exec -n hodei-jobs deployment/hodei-server -- \
  psql $DATABASE_URL -c "SELECT version();"
```

#### Eventos
```bash
kubectl get events -n hodei-jobs --sort-by='.lastTimestamp'

# Ver eventos específicos
kubectl describe deployment/hodei-server -n hodei-jobs
```

## 🧪 Testing

### Verificar instalación

```bash
# Verificar todos los pods
kubectl get pods -n hodei-jobs

# Verificar servicios
kubectl get svc -n hodei-jobs

# Verificar volúmenes
kubectl get pvc -n hodei-jobs

# Verificar ingress
kubectl get ingress -n hodei-jobs
```

### Tests de integración

```bash
# Crear job de prueba
kubectl run test-job --image=alpine --restart=Never -- \
  /bin/sh -c "echo 'Hello from test job'"

# Ver logs del job
kubectl logs test-job

# Limpiar
kubectl delete pod test-job
```

## 📋 Checklist de Producción

- [ ] Cambiar todas las contraseñas por defecto
- [ ] Configurar storage class SSD
- [ ] Habilitar TLS/HTTPS
- [ ] Configurar backup automático de PostgreSQL
- [ ] Configurar retención de Prometheus (90 días)
- [ ] Configurar alertas de AlertManager
- [ ] Configurar Network Policies
- [ ] Configurar PodDisruptionBudget
- [ ] Configurar HPA con métricas personalizadas
- [ ] Configurar Ingress con cert-manager
- [ ] Configurar logging centralizado
- [ ] Configurar tracing con Jaeger
- [ ] Verificar RBAC
- [ ] Configurar límites de recursos
- [ ] Configurar probes de liveness y readiness
- [ ] Configurar anti-affinity rules
- [ ] Configurar tolerations para taints

## 🆘 Soporte

Para issues y soporte:
- 📧 Email: support@hodei-pipelines.dev
- 📖 Documentación: https://docs.hodei-pipelines.dev
- 🐛 Issues: https://github.com/Rubentxu/hodei-pipelines/issues
