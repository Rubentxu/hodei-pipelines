# Custom Worker Templates - SOLID Principle

## 🎯 Principio SOLID: Interface Segregation

La implementación sigue **ISP (Interface Segregation Principle)** - los clientes no dependen de interfaces que no usan.

### ✅ ProviderConfig - Configuración Flexible

```rust
// Sin Customización - Usa HWP Agent por defecto
let config = ProviderConfig::docker("docker-1".to_string());
let provider = factory.create_provider(config).await?;

// Con Custom Docker Image
let config = ProviderConfig::docker("docker-2".to_string())
    .with_image("my-hwp-agent:v1.0".to_string());
let provider = factory.create_provider(config).await?;

// Con Custom K8s Pod Template
let pod_template = r#"
{
  "apiVersion": "v1",
  "kind": "Pod",
  "spec": {
    "containers": [{
      "name": "worker",
      "image": "my-hwp-agent:v1.0"
    }]
  }
}
"#;

let config = ProviderConfig::kubernetes("k8s-1".to_string())
    .with_pod_template(pod_template.to_string());
let provider = factory.create_provider(config).await?;
```

## 🔧 Custom Templates por Provider

### Docker Provider

**Default:** `hwp-agent:latest`

**Custom Image:**
```rust
let config = ProviderConfig::docker("custom-docker".to_string())
    .with_image("registry.company.com/hwp-agent:v1.2.3".to_string());
```

**Por qué no necesita template completo:**
- Docker es más simple: imagen + env vars + volumes
- Se puede configurar directamente en el código sin template
- HWP Agent container ya tiene configuración estándar
- Custom image es suficiente para personalización

**Características:**
- ✅ Usa la imagen personalizada
- ✅ Mantiene las labels: `hodei.worker=true`
- ✅ Mantiene las variables de entorno estándar
- ✅ Sigue el contrato `WorkerProvider`

### Kubernetes Provider

**Default:** HWP Agent Pod con labels estándar

**Custom Pod Template (necesario para K8s):**

**¿Por qué K8s SÍ necesita template?**
- Pods tienen múltiples campos complejos: affinity, nodeSelector, initContainers, sidecars
- Resource management, tolerations, security contexts
- Multi-container Pods, volume mounts, configMaps, secrets
- Template JSON es más flexible que configurar cada campo en código
- Permite usar Helm charts, Kustomize, etc.

**Custom Pod Template:**
```rust
let template = serde_json::json!({
    "apiVersion": "v1",
    "kind": "Pod",
    "spec": {
        "containers": [{
            "name": "worker",
            "image": "registry.company.com/hwp-agent:v1.2.3",
            "resources": {
                "requests": {"cpu": "4", "memory": "8Gi"},
                "limits": {"cpu": "4", "memory": "8Gi"}
            },
            "volumeMounts": [{
                "name": "shared-data",
                "mountPath": "/data"
            }]
        }],
        "volumes": [{
            "name": "shared-data",
            "emptyDir": {}
        }]
    }
}).to_string();

let config = ProviderConfig::kubernetes("custom-k8s".to_string())
    .with_pod_template(template);
```

**Auto-inyección de valores:**
- ✅ `metadata.name` → `hodei-worker-{worker_id}`
- ✅ `metadata.namespace` → configuración del provider
- ✅ `metadata.labels["hodei.worker.id"]` → `{worker_id}`
- ✅ `metadata.labels["hodei.worker"]` → `"true"`
- ✅ Env vars: `WORKER_ID`, `HODEI_SERVER_GRPC_URL`

## 📊 SOLID Compliance

### Single Responsibility Principle (SRP)
- ✅ `ProviderConfig`: Solo configuración
- ✅ `WorkerProvider`: Solo operaciones de worker
- ✅ `DockerProvider`: Solo Docker
- ✅ `KubernetesProvider`: Solo Kubernetes

### Open/Closed Principle (OCP)
- ✅ Extensible sin modificar código existente
- ✅ Agrega nuevos providers sin cambiar interfaces

### Liskov Substitution Principle (LSP)
- ✅ Cualquier `WorkerProvider` puede reemplazar otro
- ✅ Todos implementan el mismo contrato

### Interface Segregation Principle (ISP)
- ✅ `WorkerProvider` es pequeño y específico
- ✅ Clientes no dependen de métodos que no usan
- ✅ `ProviderConfig` extensible pero no acoplada al trait

### Dependency Inversion Principle (DIP)
- ✅ `WorkerManagementService` depende de `WorkerProvider` trait, no de implementaciones
- ✅ `DefaultProviderFactory` crea providers dinámicamente

## 🔌 Factory Pattern - Creación Dinámica

```rust
// Mismo código para cualquier provider
let factory = DefaultProviderFactory::new();

// Docker
let docker_config = ProviderConfig::docker("docker".to_string());
let docker_provider = factory.create_provider(docker_config).await?;

// Kubernetes
let k8s_config = ProviderConfig::kubernetes("k8s".to_string());
let k8s_provider = factory.create_provider(k8s_config).await?;

// Ambos cumplen el mismo contrato
fn use_provider(provider: Box<dyn WorkerProvider>) {
    let worker = provider.create_worker(worker_id, config).await.unwrap();
    // ...
}
```

## 🎨 Ejemplos Completos

### Ejemplo 1: Docker con Imagen Custom

```rust
use hodei_adapters::DefaultProviderFactory;
use hodei_ports::{ProviderFactoryTrait, worker_provider::ProviderConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Crear provider con imagen personalizada
    let config = ProviderConfig::docker("prod-worker".to_string())
        .with_image("registry.company.com/hwp-agent:v2.0".to_string());
    
    let factory = DefaultProviderFactory::new();
    let provider = factory.create_provider(config).await?;
    
    // Provisionar worker
    let worker = provider.create_worker(
        WorkerId::new(),
        ProviderConfig::docker("prod-worker".to_string())
            .with_image("registry.company.com/hwp-agent:v2.0".to_string())
    ).await?;
    
    println!("Worker creado: {}", worker.id);
    Ok(())
}
```

### Ejemplo 2: Kubernetes con Pod Template Custom

```rust
let pod_template = serde_json::json!({
    "apiVersion": "v1",
    "kind": "Pod",
    "metadata": {
        "labels": {
            "app": "custom-worker"
        }
    },
    "spec": {
        "restartPolicy": "Never",
        "containers": [{
            "name": "worker",
            "image": "registry.company.com/hwp-agent:gpu-v1.0",
            "resources": {
                "requests": {
                    "cpu": "4",
                    "memory": "16Gi",
                    "nvidia.com/gpu": "1"
                },
                "limits": {
                    "cpu": "4",
                    "memory": "16Gi",
                    "nvidia.com/gpu": "1"
                }
            },
            "env": [
                {"name": "GPU_ENABLED", "value": "true"}
            ]
        }]
    }
}).to_string();

let config = ProviderConfig::kubernetes("gpu-worker".to_string())
    .namespace("gpu-workers".to_string())
    .with_pod_template(pod_template);
```

### Ejemplo 3: WorkerManagementService con Custom Config

```rust
use hodei_modules::{WorkerManagementService, create_default_worker_management_service};

async fn create_custom_service() -> Result<WorkerManagementService> {
    let config = ProviderConfig::docker("custom".to_string())
        .with_image("my-hwp-agent:v1.0".to_string());
    
    let factory = DefaultProviderFactory::new();
    let provider = factory.create_provider(config).await?;
    
    Ok(WorkerManagementService::new(provider))
}

// Uso
let service = create_custom_service().await?;
let worker = service.provision_worker(
    "my-custom-image:v1.0".to_string(),
    4,      // cpu_cores
    8192,   // memory_mb
).await?;
```

## ✅ Resumen

**SOLID Principles Respetados:**
1. **SRP**: Cada componente tiene una responsabilidad
2. **OCP**: Extensible sin modificar código
3. **LSP**: Providers intercambiables
4. **ISP**: Interfaces pequeñas y específicas ✅
5. **DIP**: Depende de abstracciones

**Flexibilidad:**
- ✅ Default: HWP Agent image
- ✅ Custom Docker image
- ✅ Custom K8s Pod template
- ✅ Factory pattern dinámico
- ✅ No breaking changes

**No se viola ISP:**
- ❌ No hay métodos extra en `WorkerProvider`
- ✅ Solo configuración en `ProviderConfig`
- ✅ Clients usan solo lo que necesitan
