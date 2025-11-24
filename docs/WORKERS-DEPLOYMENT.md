# Workers en Hodei Jobs - Opciones de Manejo y Despliegue

## 📋 Arquitectura de Workers

Los **workers** en Hodei Jobs son nodos que ejecutan los jobs distribuidos. Cada worker ejecuta el binario `hwp-agent` que se conecta al servidor central via gRPC.

### Componentes
- **hwp-agent**: Binario que ejecuta en cada worker
- **Registro**: Los workers se registran vía API REST o gRPC
- **Heartbeat**: Envío periódico de estado de salud
- **Job Execution**: Recepción y ejecución de jobs

## 🐳 Opción 1: Docker (Desarrollo y Testing)

### Características
- ✅ Ideal para desarrollo local
- ✅ Rápido setup
- ✅ Fácil de debuggear
- ✅ Multi-node en una sola máquina
- ❌ No apta para producción distribuida

### Comando Principal
```bash
# Levantar server + 3 workers
make up-workers
```

### Detalles
El `docker-compose.yml` incluye 3 workers preconfigurados:

| Worker | CPU Cores | RAM | Comando |
|--------|-----------|-----|---------|
| worker-01 | 4 | 8GB | `docker compose up -d worker-01` |
| worker-02 | 4 | 8GB | `docker compose up -d worker-02` |
| worker-03 | 2 | 4GB | `docker compose up -d worker-03` |

### Registro Automático
Los workers se auto-registran al iniciar (si está configurado) o pueden registrarse manualmente via API:

```bash
curl -X POST http://localhost:8080/api/v1/workers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-worker",
    "cpu_cores": 4,
    "memory_gb": 8
  }'
```

### Comandos Útiles
```bash
# Ver estado de workers
make workers

# Ver logs de un worker específico
docker compose logs -f worker-01

# Levantar solo workers (sin server)
docker compose up -d worker-01 worker-02 worker-03

# Escalar workers (agregar más)
docker compose up -d --scale worker-01=3
```

## ☸️ Opción 2: Kubernetes (Producción)

### Características
- ✅ Escalado automático horizontal (HPA)
- ✅ Auto-healing y recuperación
- ✅ Orquestación avanzada
- ✅ Integración con cloud providers
- ✅ Resource quotas y limits
- ❌ Complejidad de setup inicial

### Deployment YAML

```yaml
# worker-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: hwp-worker
  namespace: hodei-jobs
spec:
  replicas: 3
  selector:
    matchLabels:
      app: hwp-worker
  template:
    metadata:
      labels:
        app: hwp-worker
    spec:
      containers:
      - name: hwp-agent
        image: hodei-jobs/hwp-agent:latest
        ports:
        - containerPort: 50052
        env:
        - name: WORKER_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: SERVER_URL
          value: "http://hodei-server:8080"
        - name: GRPC_SERVER_URL
          value: "http://hodei-server:50051"
        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "4"
            memory: "8Gi"
        livenessProbe:
          httpGet:
            path: /health
            port: 50052
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 50052
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: hwp-worker-service
  namespace: hodei-jobs
spec:
  selector:
    app: hwp-worker
  ports:
  - port: 50052
    targetPort: 50052
  type: ClusterIP
```

### Desplegar en K8s
```bash
# Aplicar deployment
kubectl apply -f k8s/worker-deployment.yaml

# Ver pods
kubectl get pods -n hodei-jobs -l app=hwp-worker

# Escalar workers
kubectl scale deployment hwp-worker --replicas=5 -n hodei-jobs

# Ver logs
kubectl logs -f deployment/hwp-worker -n hodei-jobs
```

### Horizontal Pod Autoscaler (HPA)
```yaml
# worker-hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: hwp-worker-hpa
  namespace: hodei-jobs
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: hwp-worker
  minReplicas: 2
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

## 🐳 Swarm (Docker Swarm)

### Características
- ✅ Más simple que K8s
- ✅ Scaling nativo
- ✅ Load balancing
- ❌ Menos features que K8s

### Stack File
```yaml
# docker-stack.yml
version: '3.8'

services:
  worker:
    image: hodei-jobs/hwp-agent:latest
    deploy:
      replicas: 5
      restart_policy:
        condition: any
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
    environment:
      - SERVER_URL=http://hodei-server:8080
      - GRPC_SERVER_URL=http://hodei-server:50051

  worker-visualizer:
    image: dockersamples/visualizer:latest
    ports:
      - "8080:8080"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    deploy:
      placement:
        constraints: [node.role == manager]
```

### Desplegar
```bash
# Inicializar Swarm
docker swarm init

# Desplegar stack
docker stack deploy -c docker-stack.yml hodei-jobs

# Ver servicios
docker service ls

# Escalar workers
docker service scale hodei-jobs_worker=10
```

## ☁️ Opción 3: Cloud Managed (EKS, GKE, AKS)

### Características
- ✅ Zero infrastructure management
- ✅ Auto-scaling avanzado
- ✅ Integración con cloud services
- ✅ Alta disponibilidad
- ❌ Costos variables
- ❌ Vendor lock-in

### AWS EKS
```bash
# Instalar workers con eksctl
eksctl create nodegroup \
  --cluster hodei-cluster \
  --name worker-nodes \
  --node-type c5.xlarge \
  --nodes 3 \
  --nodes-min 2 \
  --nodes-max 10 \
  --managed
```

## 🏠 Opción 4: Bare Metal / VMs

### Para Producción On-Premise

#### systemd Service
```ini
# /etc/systemd/system/hwp-worker.service
[Unit]
Description=HWP Worker Agent
After=network.target

[Service]
Type=simple
User=hwp-agent
Group=hwp-agent
WorkingDirectory=/app
ExecStart=/usr/local/bin/hwp-agent \
  --server-url=http://hodei-server:8080 \
  --grpc-server-url=http://hodei-server:50051 \
  --worker-id=worker-01
Restart=always
RestartSec=5

# Security
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ReadWritePaths=/var/log/hwp-agent

[Install]
WantedBy=multi-user.target
```

#### Habilitar y Iniciar
```bash
# Habilitar servicio
sudo systemctl enable hwp-worker

# Iniciar
sudo systemctl start hwp-worker

# Ver estado
sudo systemctl status hwp-worker

# Ver logs
sudo journalctl -u hwp-worker -f
```

### Ansible Playbook
```yaml
# deploy-workers.yml
- hosts: workers
  become: yes
  tasks:
    - name: Install hwp-agent
      copy:
        src: ./hwp-agent
        dest: /usr/local/bin/hwp-agent
        mode: '0755'
    
    - name: Create user
      user:
        name: hwp-agent
        system: yes
        shell: /bin/false
    
    - name: Create log directory
      file:
        path: /var/log/hwp-agent
        state: directory
        owner: hwp-agent
        group: hwp-agent
    
    - name: Deploy systemd service
      template:
        src: hwp-worker.service.j2
        dest: /etc/systemd/system/hwp-worker.service
      notify: reload systemd
    
    - name: Start and enable service
      systemd:
        name: hwp-worker
        state: started
        enabled: yes
    
  handlers:
    - name: reload systemd
      systemd:
        daemon_reload: yes
```

## 📊 Comparación de Opciones

| Opción | Setup | Escalado | Alta Disponibilidad | Complejidad | Costo |
|--------|-------|----------|---------------------|-------------|-------|
| **Docker Compose** | ⭐⭐⭐⭐⭐ | ⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| **Kubernetes** | ⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐ |
| **Docker Swarm** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Cloud Managed** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ |
| **Bare Metal** | ⭐⭐ | ⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

## 🎯 Recomendaciones

### Desarrollo Local
```bash
# Usar Docker Compose
make up-workers
```

### Testing/QA
```bash
# Docker con más workers
docker compose up -d --scale worker-01=5
```

### Producción (On-Premise)
```bash
# Opción 1: Bare Metal + Ansible
ansible-playbook -i inventory deploy-workers.yml

# Opción 2: Docker Swarm (menos complejo que K8s)
docker stack deploy -c docker-stack.yml hodei-jobs
```

### Producción (Cloud)
```bash
# Kubernetes con autoscaling
kubectl apply -f k8s/
kubectl apply -f k8s/worker-hpa.yaml
```

## 🚀 Comando Principal Recomendado

### Para Desarrollo
```bash
# Compilar y levantar stack completo
make build-all
make up-workers-with-monitoring
```

### Para Producción
```bash
# Docker Swarm (recomendado para empezar)
docker stack deploy -c docker-stack.yml hodei-jobs

# O Kubernetes (para escalar mucho)
kubectl apply -f k8s/worker-deployment.yaml
kubectl apply -f k8s/worker-hpa.yaml
```

## 📝 Scripts Adicionales

### Auto-register workers
```bash
#!/bin/bash
# register-workers.sh

for i in {01..10}; do
  curl -X POST http://localhost:8080/api/v1/workers \
    -H "Content-Type: application/json" \
    -d "{
      \"name\": \"worker-$i\",
      \"cpu_cores\": 4,
      \"memory_gb\": 8
    }"
done
```

### Monitor workers
```bash
#!/bin/bash
# monitor-workers.sh

watch -n 2 '
echo "=== Workers Status ==="
docker compose ps worker-01 worker-02 worker-03

echo ""
echo "=== Jobs Running ==="
curl -s http://localhost:8080/api/v1/jobs | jq -r ".jobs[] | select(.state == \"RUNNING\") | .name"
'
```

## ✅ Resumen

**Para empezar rápidamente**: `make up-workers`

**Para producción on-premise**: Docker Swarm + Ansible

**Para producción cloud**: Kubernetes + HPA

**Para desarrollo**: Docker Compose
