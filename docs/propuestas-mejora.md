Esta es una evolución arquitectónica muy interesante. Pasar a un **Monolito Modular con Arquitectura Hexagonal** en Rust te permitirá obtener un rendimiento "metal-level" (eliminando latencia de red interna y serialización) mientras mantienes el código desacoplado y testeable.

Respecto a tu pregunta sobre **Jenkins y su protocolo**: Sí, Jenkins usa *JNLP (Java Network Launch Protocol)* / *Remoting*, que permite que el Agente se conecte al Master (inversión de control) y establezca un canal bidireccional para logs, comandos y transferencia de archivos.
**Para Hodei Jobs, implementaremos una versión moderna de esto usando gRPC (con `tonic`)**. Esto abstraerá totalmente la infraestructura subyacente: al Worker Manager no le importará si el agente está en un contenedor Docker, un Pod de K8s o una Lambda; solo le importa que el agente se conecte y hable el "Protocolo Hodei".

Aquí tienes la propuesta de diseño detallada y optimizada.

---

### 1. Arquitectura General: Hexagonal y Monolítica

El sistema se compilará en **un único binario**, pero el código fuente estará estrictamente separado en crates (módulos).

#### Estructura de Directorios Propuesta
```text
crates/
  core/                 # Dominio puro (Entities, Value Objects, Errors)
  ports/                # Definición de Traits (Input/Output Ports)
  
  adapters/             # Implementaciones (Infrastructure)
    storage/
      postgres/         # Implementación SQLx
      embedded/         # Implementación Redb (High Perf Key-Value)
    bus/
      memory/           # Tokio Channels (Broadcast/MPSC)
    worker-proto/       # Definición gRPC (Protocolo Hodei)
  
  modules/              # Lógica de Aplicación (Use Cases)
    orchestrator/
    scheduler/
    worker-manager/
  
  server/               # Binario final (Wire up & Dependency Injection)
    src/main.rs
```

---

### 2. El Diseño de Puertos (Ports) y Traits

Para cumplir con la arquitectura hexagonal, definiremos las interacciones mediante Traits en `crates/ports`.

#### A. Repositorio (Storage Port)
Definiremos un repositorio genérico que permita cambiar entre persistencia robusta (Postgres) y ultrarrápida embebida (Embedded).

```rust
// crates/ports/src/repository.rs
use async_trait::async_trait;
use hodei_core::Job;

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn save(&self, job: &Job) -> Result<(), RepoError>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Job>, RepoError>;
    // Métodos transaccionales
    async fn execute_in_transaction<F>(&self, f: F) -> Result<(), RepoError>
    where F: FnOnce(&mut dyn TransactionContext) -> Future<...>;
}
```

#### B. Bus de Eventos (Event Port)
No usaremos NATS internamente. Usaremos Traits para publicar eventos de dominio.

```rust
// crates/ports/src/bus.rs
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publica un evento sin serialización (pasa la struct Rust)
    async fn publish_job_created(&self, event: JobCreatedEvent);
    async fn publish_worker_heartbeat(&self, event: HeartbeatEvent);
}

#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn subscribe_job_created(&self) -> Receiver<JobCreatedEvent>;
}
```

---

### 3. Adaptadores de Alto Rendimiento

#### A. Estrategia de Almacenamiento (Storage Adapters)

1.  **Opción A: PostgreSQL (Robustez / Producción)**
    *   Uso de `sqlx`.
    *   Ideal para entornos distribuidos o cuando se requiere auditoría histórica estricta.
2.  **Opción B: Embebida - Redb (Rendimiento Extremo / Edge)**
    *   Usaremos **Redb** (una alternativa a Sled, puramente en Rust, ACID y muy rápida).
    *   Funciona como un Key-Value store persistente en disco local.
    *   **Caso de uso:** Instancias "Single Node" de Hodei Jobs que deben procesar miles de jobs por segundo sin overhead de servidor SQL.

#### B. Estrategia del Bus Interno (Memory Adapter)
Implementaremos el trait `EventBus` usando canales de `tokio`.
*   **Zero-Copy:** Al pasar mensajes entre el Orchestrator y el Scheduler dentro del mismo binario, pasaremos `Arc<Job>` (punteros inteligentes). No hay copia de memoria ni serialización JSON. Esto es órdenes de magnitud más rápido que NATS local.

---

### 4. Protocolo de Comunicación de Workers (Hodei Worker Protocol - HWP)

Aquí es donde abstraemos la infraestructura (Docker, K8s, Cloud) emulando el modelo de Jenkins pero con tecnología moderna.

**El Problema:** Gestionar las APIs de Docker y K8s directamente desde el Worker Manager es complejo y difícil de escalar para logs en tiempo real.
**La Solución:** Un **Agente Ligero** y un protocolo estandarizado **gRPC bidireccional**.

#### ¿Cómo funciona el HWP?
1.  **El Agente:** Creamos un binario minúsculo en Rust (`hodei-agent`).
2.  **Inyección:** Cuando el `DockerProvider` o `K8sProvider` crea un contenedor, inyecta este binario (o usa una imagen base que lo tenga).
3.  **Call Home:** Al arrancar el contenedor, el agente **se conecta al Worker Manager** vía gRPC (TCP persistente).
4.  **Streaming:**
    *   El Worker Manager envía comandos al Agente.
    *   El Agente ejecuta el proceso y hace **streaming de stdout/stderr** en tiempo real por el mismo socket gRPC.

#### Definición del Protocolo (Protobuf)
```protobuf
service WorkerService {
  // Handshake inicial: El worker se identifica y reporta capacidades
  rpc Register(WorkerInfo) returns (RegisterResponse);

  // Canal bidireccional para control y logs
  rpc Connect(stream WorkerMessage) returns (stream ManagerMessage);
}

message WorkerMessage {
  oneof payload {
    Heartbeat heartbeat = 1;
    LogChunk log_chunk = 2;      // Streaming de logs eficiente
    JobResult job_result = 3;
  }
}

message ManagerMessage {
  oneof payload {
    ExecuteCommand command = 1;
    CancelJob cancel = 2;
  }
}
```

#### Ventajas de esta abstracción:
1.  **Universalidad:** Funciona igual en K8s, Docker, VM o incluso una Raspberry Pi física. El Worker Manager no necesita saber dónde corre, solo espera la conexión gRPC.
2.  **Firewall Friendly:** La conexión es saliente desde el Worker al Manager (como Jenkins). No necesitas abrir puertos en los nodos workers.
3.  **Rendimiento:** gRPC usa Protobuf (binario compacto) y HTTP/2 (multiplexación). Es mucho más eficiente que hacer polling o usar la API de Docker para logs.

---

### 5. Mejoras de Rendimiento "Monolito Modular"

Al tener todo en un proceso, aplicaremos optimizaciones agresivas:

#### A. Scheduler: Zero-Lock Queue
En lugar de una base de datos para la cola de prioridad (que es lenta), usaremos una estructura en memoria dentro del módulo Scheduler.
*   **Estructura:** `DashMap` o `Crossbeam Queue` para alta concurrencia sin bloqueos globales.
*   **Persistencia (WAL):** Para no perder datos si se va la luz, cada operación de encolado se escribe asíncronamente en un *Write-Ahead Log* (archivo secuencial en disco, muy rápido) usando el adaptador de almacenamiento embebido (`Redb`). Al reiniciar, se recarga la RAM desde el disco.

#### B. Log Streaming: Memoria Compartida
El flujo de logs será:
`Agente (gRPC)` -> `WorkerManager` -> `RingBuffer en RAM` -> `API (SSE)`.
*   Al estar en un monolito, el `WorkerManager` escribe en un `Arc<RwLock<VecDeque>>` y la capa API lee del mismo puntero.
*   **Mejora:** No hay serialización ni transporte interno de logs. Latencia de visualización < 1ms.

---

### Resumen de Cambios Propuestos

1.  **Arquitectura:** Reorganizar crates en `ports` (traits), `adapters` (impl) y `modules` (lógica).
2.  **Datos:** Crear adaptador `EmbeddedRepository` usando **Redb** para despliegues de alto rendimiento/bajo recurso.
3.  **Bus:** Implementar `InMemoryEventBus` usando `tokio::broadcast` para comunicación interna instantánea.
4.  **Workers:** Implementar **"Hodei Agent"** y protocolo **gRPC** para abstraer la infraestructura subyacente (Docker/K8s solo lanzan el agente, el agente hace el trabajo).
5.  **Logs:** Pipeline de logs sin copias internas usando memoria compartida.


---

Excelente elección. Profundizar en estos dos aspectos (Persistencia Dual y Protocolo de Worker) es lo que diferenciará a **Hodei Jobs** como una herramienta de alto rendimiento frente a soluciones tradicionales más pesadas.

Aquí tienes el diseño técnico detallado para implementar esta visión.

---

### 1. Estrategia de Persistencia Dual (Hexagonal Storage)

El objetivo es definir un **Puerto (Port)** agnóstico y dos **Adaptadores (Adapters)** radicalmente distintos pero intercambiables.

#### El Puerto (The Trait)
Definimos el comportamiento en `crates/ports`. Usaremos `async_trait` para permitir I/O asíncrono.

```rust
// crates/ports/src/repo.rs
use async_trait::async_trait;
use hodei_shared_types::{Job, JobId, JobStatus};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Not found")]
    NotFound,
}

#[async_trait]
pub trait JobRepository: Send + Sync {
    /// Guardar o actualizar un Job
    async fn save(&self, job: &Job) -> Result<(), StorageError>;
    
    /// Obtener Job por ID
    async fn get(&self, id: &JobId) -> Result<Option<Job>, StorageError>;
    
    /// Obtener jobs pendientes (optimizado para el Scheduler)
    async fn get_pending(&self, limit: usize) -> Result<Vec<Job>, StorageError>;
    
    /// Actualización atómica de estado (Critical for Scheduler)
    async fn compare_and_swap_status(
        &self, 
        id: &JobId, 
        expected: JobStatus, 
        new: JobStatus
    ) -> Result<bool, StorageError>;
}
```

#### Adaptador A: PostgreSQL (Producción / Cluster)
Utiliza `sqlx`. Aquí la clave del rendimiento es el **Connection Pooling**.

*   **Estrategia:** Mapeo Relacional.
*   **Optimización:** Usar `sqlx::query_as!` para verificación en tiempo de compilación.
*   **Ventaja:** Consultas complejas, backups estándar, escalabilidad horizontal de lectura.

#### Adaptador B: Redb (Embebido / Edge / High Perf)
Utiliza `redb` (una base de datos ACID embebida en Rust, similar a LMDB pero escrita en Rust puro y memory-safe).

*   **Estrategia:** Key-Value Store con serialización binaria (`bincode`).
*   **Estructura de Tablas (Tables):**
    *   `Table<Uuid, Vec<u8>>`: Para almacenar el objeto `Job` completo serializado.
    *   `Table<u64, Uuid>` (Índice Secundario): Para ordenar Jobs por `priority` + `created_at`. Esto hace que `get_pending` sea **O(1)** (acceso directo) en lugar de O(N).
*   **Rendimiento:**
    *   **Zero-Network:** No hay syscalls de red, todo ocurre en el proceso.
    *   **Zero-Copy (casi):** Redb usa *memory mapping* (mmap).
*   **Caso de Uso:** Despliegues en un solo nodo que procesan 10,000+ jobs/segundo.

---

### 2. Hodei Worker Protocol (HWP) sobre gRPC

Esta es la pieza clave para abstraer la infraestructura. En lugar de que el Worker Manager "controle" la infraestructura (que es complejo y específico de cada proveedor), invertimos el control.

#### Arquitectura "Agent-First"
1.  El Worker Manager le dice al proveedor (K8s/Docker): *"Arranca una imagen con el binario `hodei-agent`"*.
2.  El Provider arranca el contenedor. **Aquí termina la responsabilidad del Provider**.
3.  El `hodei-agent` arranca dentro del contenedor y **llama a casa (Dial-Out)** al Worker Manager mediante gRPC.

#### Definición del Protocolo (Protobuf)
Archivo `proto/worker.proto`:

```protobuf
syntax = "proto3";
package hodei.worker.v1;

service WorkerService {
  // El Worker se registra y mantiene el canal abierto
  rpc Connect(stream WorkerMessage) returns (stream ManagerMessage);
}

// Mensajes que envía el Agente al Manager
message WorkerMessage {
  oneof payload {
    Register register = 1;
    Heartbeat heartbeat = 2;
    LogChunk log_chunk = 3;      // Streaming de logs
    JobStatusUpdate status = 4;  // Cambio de estado del proceso
  }
}

// Mensajes que envía el Manager al Agente
message ManagerMessage {
  oneof payload {
    StartJob start_job = 1;
    CancelJob cancel_job = 2;
    Acknowledge ack = 3; // Para confirmar recepción de logs críticos
  }
}

message LogChunk {
  string job_id = 1;
  bytes data = 2;    // Datos crudos
  bool is_stderr = 3;
  uint64 sequence = 4; // Para ordenar si llegan desordenados (raro en TCP)
}

message StartJob {
  string job_id = 1;
  string command = 2;
  map<string, string> env_vars = 3;
  // Secretos inyectados directamente en RAM, no en disco
  map<string, string> secrets = 4; 
}
```

#### Implementación del Agente (`hodei-agent`)
Es un binario Rust pequeño (~5MB estático).
1.  **Tokio Runtime:** Inicia conexión gRPC persistente.
2.  **Command Executor:** Usa `tokio::process::Command` para lanzar el trabajo real (ej. `cargo build`).
3.  **Log Piper:** Lee `stdout` y `stderr` del proceso hijo y los envía al canal gRPC.
    *   **Optimización de Rendimiento:** Usa un buffer intermedio. Si el canal gRPC se satura (backpressure), el agente pausa la lectura del proceso hijo automáticamente (gracias a `tokio`), evitando que el contenedor explote por memoria.

#### Ventaja sobre Jenkins (JNLP)
Jenkins usa serialización Java, que es lenta y pesada. Nosotros usamos **Protobuf**, que es binario, compacto y permite streaming multiplexado sobre HTTP/2.

---

### 3. Bus de Eventos Interno: "Zero-Latency"

Para que el monolito sea modular pero rápido, los módulos no se llaman directamente para todo, sino que reaccionan a eventos.

#### Implementación en `crates/adapters/src/bus/memory.rs`

```rust
use tokio::sync::broadcast;
use std::sync::Arc;

// El evento es un Enum que contiene DATOS REALES (structs), no JSON.
#[derive(Clone, Debug)]
pub enum SystemEvent {
    JobCreated(Arc<Job>),      // Uso de Arc para evitar copiar el Job entero
    JobScheduled(Uuid, Uuid),  // JobId, WorkerId
    WorkerConnected(Uuid),     // WorkerId
    LogReceived(LogEntry),
}

pub struct InMemoryBus {
    sender: broadcast::Sender<SystemEvent>,
}

impl InMemoryBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: SystemEvent) {
        // Si nadie escucha, el mensaje se descarta (coste ~0)
        // Si hay oyentes, solo se incrementa el contador del Arc (coste nanosegundos)
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.sender.subscribe()
    }
}
```

---

### 4. Resumen del Flujo de Alto Rendimiento

Imaginemos el flujo completo de "Ver Logs en Vivo":

1.  **Proceso Hijo (Worker):** Escribe en `stdout`.
2.  **Hodei Agent:** Lee `stdout`, empaqueta en Protobuf `LogChunk`, envía por TCP (gRPC).
3.  **Worker Manager (Monolito):** Recibe gRPC. Desempaqueta.
    *   *Sin NATS interno.*
    *   *Sin escribir en BD SQL fila por fila.*
4.  **Worker Manager:** Empuja el log a un `RingBuffer` en memoria compartida (`Arc<RwLock<VecDeque>>`).
5.  **API Layer (Monolito):** El endpoint `/logs/stream` tiene un watcher sobre ese `RingBuffer`.
6.  **Cliente (Usuario):** Recibe el SSE.

**Resultado:** El log viaja desde el contenedor hasta el navegador del usuario sin tocar el disco del servidor principal y con copias de memoria mínimas.

### 5. Estructura de Inyección de Dependencias (`main.rs`)

El archivo `main.rs` será el único lugar donde el código se "acopla". Es el pegamento.

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Configuración
    let config = Config::load();

    // 2. Inicializar Adaptadores (Infraestructura)
    let bus = Arc::new(InMemoryBus::new(10000));
    
    let repo: Arc<dyn JobRepository> = if config.use_postgres {
        Arc::new(PostgresRepository::new(&config.db_url).await?)
    } else {
        Arc::new(EmbeddedRepository::new("hodei.db")?)
    };

    // 3. Inicializar Módulos (Core)
    // Inyectamos los puertos (traits) en los módulos
    let scheduler = SchedulerModule::new(repo.clone(), bus.clone());
    let worker_mgr = WorkerManagerModule::new(repo.clone(), bus.clone());
    let orchestrator = OrchestratorModule::new(repo.clone(), bus.clone());

    // 4. Arrancar Servidores (Puertos de Entrada)
    // API HTTP (Axum)
    let api_server = ApiServer::new(orchestrator, scheduler, worker_mgr);
    
    // Servidor gRPC para Workers
    let grpc_server = WorkerGrpcServer::new(worker_mgr);

    // 5. Run
    tokio::select! {
        _ = api_server.serve(8080) => {},
        _ = grpc_server.serve(50051) => {},
        _ = scheduler.start_loop() => {},
    }

    Ok(())
}
```

### Conclusión de las Mejoras

1.  **Modularidad Hexagonal:** El núcleo de la lógica (`modules/`) no sabe si usa Postgres o Redb, ni si los eventos van por memoria o por red. Es puramente lógico y testeable.
2.  **Eficiencia Extrema:**
    *   **Almacenamiento:** Redb elimina la latencia de red de la base de datos.
    *   **Bus:** Tokio Channels elimina la latencia de serialización.
    *   **Workers:** gRPC con Protobuf minimiza el ancho de banda y la CPU necesaria para logs.
3.  **Abstracción Real de Workers:** Gracias al "Hodei Agent", añadir soporte para AWS Lambda o Azure Container Instances es trivial: solo necesitas una forma de arrancar el binario del agente allí; el resto del sistema no cambia.

--- 

Esta es una de las partes más críticas del sistema. Si el Agente y el Protocolo están bien diseñados, el sistema será robusto, seguro y escalable. Si no, tendrás cuellos de botella de red, problemas de seguridad y dificultades para depurar errores.

Basándonos en las lecciones de **Jenkins (Remoting/JNLP)**, **GitHub Actions (Runner)** y **Tekton**, aquí tienes el diseño detallado del **Hodei Agent** y el **Protocolo HWP (Hodei Worker Protocol)**.

---

### 1. Filosofía de Diseño: "Reverse Connect" (Inversión de Control)

Al igual que Jenkins y GitHub Actions, utilizaremos un modelo **"Dial-Out" (Llamada Saliente)**.

*   **No:** El servidor (Monolito) NO se conecta al Worker. (Esto requeriría abrir puertos en firewalls, gestionar IPs dinámicas, etc.).
*   **Sí:** El Agente (dentro del contenedor/VM) se despierta y llama al Servidor.

**Ventaja:** Funciona transparentemente dentro de Kubernetes, detrás de NAT corporativos, en AWS Lambda o en tu portátil local sin configurar redes complejas.

---

### 2. Seguridad: Zero-Trust y mTLS

La seguridad en CI/CD es vital porque los workers ejecutan código arbitrario.

#### A. Identidad del Agente (Bootstrapping)
Cuando el *Worker Manager* solicita al proveedor (ej. Docker/K8s) crear un worker, inyecta dos variables de entorno seguras en el contenedor:
1.  `HODEI_SERVER_URL`: La dirección del servidor gRPC (`grpcs://hodei-server:50051`).
2.  `HODEI_WORKER_TOKEN`: Un JWT (Json Web Token) de un solo uso o corta duración, firmado por el servidor.

#### B. Encriptación del Canal (mTLS)
No usaremos TLS simple, sino **mTLS (Mutual TLS)** si es posible, o TLS estándar + Token de Aplicación.
*   **Recomendación:** Para simplificar la gestión de certificados en un inicio, usa **TLS estándar** para encriptar el canal (el servidor tiene cert) y el **Token JWT** en los metadatos de autenticación de gRPC (Bearer Token) para autorizar al agente.

---

### 3. El Protocolo HWP (Hodei Worker Protocol)

Definido en Protobuf (gRPC). Es un protocolo **bidireccional (Bi-di Streaming)** sobre HTTP/2.

**Archivo:** `proto/hodei_worker.proto`

```protobuf
syntax = "proto3";
package hodei.worker.v1;

service WorkerService {
  // Un único método streaming maneja todo el ciclo de vida.
  // Esto reduce el overhead de handshakes TCP/TLS constantes.
  rpc Connect(stream AgentMessage) returns (stream ServerMessage);
}

// --- Mensajes del Agente hacia el Servidor ---
message AgentMessage {
  // Identificador único de la sesión/request
  string request_id = 1;
  
  oneof payload {
    // 1. Handshake inicial
    RegisterRequest register = 2;
    
    // 2. Streaming de Logs (Optimizado)
    LogChunk log_chunk = 3;
    
    // 3. Telemetría de recursos (CPU, RAM)
    ResourceUsage resource_usage = 4;
    
    // 4. Cambios de estado (Empezó paso X, Terminó paso Y)
    StepStatusUpdate step_update = 5;
    
    // 5. Resultado final
    JobResult job_result = 6;
  }
}

// --- Mensajes del Servidor hacia el Agente ---
message ServerMessage {
  oneof payload {
    // 1. Asignación de trabajo
    AssignJob assign_job = 1;
    
    // 2. Cancelación (Signal Kill)
    CancelJob cancel_job = 2;
    
    // 3. KeepAlive / Ping (para evitar timeouts de load balancers)
    Heartbeat heartbeat = 3;
  }
}

// Detalle: Optimización de Logs
message LogChunk {
  string job_id = 1;
  bytes data = 2;       // Usamos bytes, no string, para soportar binarios/encodings raros
  StreamType stream = 3; // STDOUT o STDERR
  uint64 sequence = 4;   // Para reordenar si fuera necesario (TCP lo garantiza, pero bueno para depurar)
  int64 timestamp = 5;   // Nanosegundos desde epoch
}

enum StreamType {
  STDOUT = 0;
  STDERR = 1;
}

// Detalle: Telemetría (Inspirado en Docker stats)
message ResourceUsage {
  double cpu_percent = 1;    // % de uso de CPU
  uint64 memory_bytes = 2;   // RAM usada en bytes
  uint64 rx_bytes = 3;       // Red recibida
  uint64 tx_bytes = 4;       // Red enviada
  uint64 disk_io_bytes = 5;  // IO de disco (útil para detectar cuellos de botella)
}

message StepStatusUpdate {
  string step_id = 1;
  string step_name = 2;
  enum State { PENDING = 0; RUNNING = 1; SUCCESS = 2; FAILED = 3; }
  State state = 3;
  int32 exit_code = 4;
}
```

---

### 4. Diseño Interno del Agente (Rust)

El agente debe ser extremadamente ligero y resiliente. No puede fallar (panic) si el script del usuario falla.

#### Componentes del Agente

1.  **Supervisor (Tokio Task):**
    *   Inicia la conexión gRPC.
    *   Gestiona la reconexión automática (Exponential Backoff) si se cae la red.
    *   Mantiene el "Heartbeat" con el servidor.

2.  **Executor (Process Manager):**
    *   **Pseudo-Terminal (PTY):** Al igual que GitLab Runner o CI modernos, el agente no debe lanzar el proceso simplemente con `Stdio::piped()`. Debe usar una **PTY** (usando crates como `portable-pty` en Rust).
    *   *¿Por qué PTY?* Para preservar los colores ANSI y el formato de las herramientas de CLI. Si no usas PTY, los logs de herramientas como `npm install` o `cargo build` se ven feos y sin barras de progreso.

3.  **Log Aggregator (Buffer Inteligente):**
    *   **Problema:** Enviar un paquete gRPC por cada línea de log es ineficiente (demasiada syscall y overhead de red).
    *   **Solución (Buffering):** El agente lee del PTY y acumula bytes en un buffer. Envía el paquete (`LogChunk`) cuando:
        *   El buffer alcanza 4KB (tamaño de página típico).
        *   O han pasado 100ms desde el último envío (para evitar lag visual).
    *   Esto reduce drásticamente la carga de CPU y Red.

4.  **Metrics Sampler (Resource Monitor):**
    *   Una tarea secundaria (`tokio::interval`) se ejecuta cada 1 o 5 segundos.
    *   Lee `/proc/{pid}/stat` (en Linux) o usa el crate `sysinfo` para obtener el uso real del árbol de procesos generado por el Job.
    *   Envía el mensaje `ResourceUsage`. Esto permite al usuario ver gráficos de consumo en el dashboard ("Tu build falló porque consumió 8GB de RAM").

---

### 5. Flujo de Ejecución Detallado

1.  **Arranque:**
    *   Provider (K8s) arranca Pod `hodei-agent:latest` con ENV `TOKEN=xyz`.
    *   Agente inicia, valida Token, conecta gRPC a `hodei-server`.
    *   Envía `RegisterRequest` (OS, Arch, Hostname).

2.  **Asignación:**
    *   Servidor valida capacidad y envía `AssignJob` (contiene script, variables de entorno, secretos desencriptados en memoria).

3.  **Ejecución:**
    *   Agente crea directorio temporal aislado.
    *   Agente lanza proceso hijo (shell) dentro de una PTY.
    *   **Loop Principal:**
        *   `select!` de Tokio escuchando:
            *   Salida del PTY -> Empaquetar en `LogChunk` -> Enviar gRPC.
            *   Mensajes del Servidor (`CancelJob`) -> Matar proceso hijo.
            *   Señal de terminación del proceso hijo -> Recoger `exit_code`.
            *   Timer de Métricas -> Enviar `ResourceUsage`.

4.  **Finalización:**
    *   Proceso termina. Agente envía `JobResult` (exit code, hora fin).
    *   Agente limpia archivos temporales (Artifacts se suben antes de esto).
    *   Agente espera nuevas órdenes o se apaga (si es efímero).

---

### 6. Buenas Prácticas Copiadas (Benchmarking)

#### De Jenkins (Remoting)
*   **Resiliencia a la Red:** Si la conexión TCP se corta brevemente, el agente debe intentar reconectar y *reanudar* el envío de logs sin matar el proceso hijo inmediatamente (dando un margen de tolerancia, ej. 30s).

#### De GitHub Actions
*   **Masking de Secretos:** El Agente debe tener una lista de secretos cargados. Antes de enviar cualquier `LogChunk`, debe escanear el texto y reemplazar cualquier ocurrencia de un secreto por `***`. **Esto es vital para la seguridad.**
    *   *Implementación Rust:* Usar el crate `aho-corasick` para reemplazo de múltiples patrones de texto a ultra-alta velocidad sobre el buffer de logs antes de enviarlo.

#### De Tekton
*   **Entrypoint Wrapper:** Tekton envuelve el comando para capturar el exit code de forma fiable. Nuestro Agente actúa como ese wrapper.

#### De CircleCI
*   **SSH Debugging:** Una característica "premium" sería permitir que, si el job falla, el agente mantenga la conexión viva y permita hacer un "proxy" de una sesión SSH desde el CLI del usuario, pasando por el servidor gRPC, hasta el contenedor del agente. (Esto es complejo, pero el diseño gRPC bidireccional lo permite en el futuro).

### Resumen de Componentes del Agente (Rust)

```rust
// Pseudocódigo de la estructura del Agente

struct Agent {
    client: WorkerServiceClient<Channel>,
    config: Config,
}

impl Agent {
    async fn run(&mut self) -> Result<()> {
        // 1. Conectar
        let (tx, rx) = mpsc::channel(100); // Canal de salida
        let stream = self.client.connect(ReceiverStream::new(rx)).await?;
        
        // 2. Handshake
        tx.send(RegisterRequest::new(...)).await?;

        // 3. Loop de Mensajes
        while let Some(msg) = stream.next().await {
            match msg {
                ServerMessage::AssignJob(job) => {
                    self.execute_job(job, tx.clone()).await?;
                }
                ServerMessage::CancelJob => {
                    self.kill_current_process().await;
                }
                // ...
            }
        }
        Ok(())
    }

    async fn execute_job(&self, job: Job, tx: Sender<AgentMessage>) -> Result<()> {
        // Configurar PTY
        let pair = portable_pty::native_pty_system().openpty(...) ?;
        
        // Lanzar Comando
        let mut cmd = Command::new("bash");
        // Inyectar ENVs y Secretos
        
        let mut child = pair.slave.spawn_command(cmd)?;
        
        // Tareas Concurrentes
        let log_task = tokio::spawn(stream_logs(pair.master, tx.clone()));
        let metrics_task = tokio::spawn(stream_metrics(child.pid(), tx.clone()));
        
        // Esperar fin
        let status = child.wait().await?;
        
        // Reportar resultado
        tx.send(JobResult { exit_code: status }).await?;
        
        Ok(())
    }
}
```

Este diseño convierte al Agente de Hodei en una pieza de ingeniería moderna: seguro (mTLS, Secret Masking), eficiente (gRPC, Buffering, PTY) y universal.


---

Aquí tienes el **Plan Maestro de Refactorización y Mejora para Hodei Jobs**. He consolidado todo nuestro análisis (Arquitectura Hexagonal, Monolito Modular, Rendimiento Extremo, Protocolo de Agente tipo Jenkins/gRPC) en una guía técnica detallada para transformar tu código actual en un sistema de clase mundial.

---

## 1. Reestructuración Arquitectónica: El Monolito Hexagonal

El objetivo es eliminar la latencia de red interna y la complejidad de despliegue, manteniendo la separación lógica estricta.

### A. Nueva Estructura de Crates (Workspace)
Debemos reorganizar el repositorio para reflejar la arquitectura de Puertos y Adaptadores.

```text
root/
├── crates/
│   ├── core/                 # DOMINIO PURO: Entities (Job, Pipeline), Value Objects, Domain Errors.
│   │                         # Dependencias: Cero (solo std, chrono, uuid, serde).
│   │
│   ├── ports/                # PUERTOS: Traits (Interfaces) que definen qué necesita el core.
│   │                         # Ej: JobRepository, EventBus, WorkerClient.
│   │
│   ├── modules/              # CASOS DE USO (Application Layer):
│   │   ├── orchestrator/     # Gestión de Pipelines y ciclo de vida del Job.
│   │   ├── scheduler/        # Lógica de planificación, colas en memoria.
│   │   └── worker-manager/   # Gestión de conexiones gRPC y ciclo de vida de agentes.
│   │
│   ├── adapters/             # INFRAESTRUCTURA (Implementaciones):
│   │   ├── storage/          # Postgres (sqlx) y Embedded (Redb).
│   │   ├── bus/              # InMemoryBus (Tokio channels).
│   │   └── rpc/              # Servidor gRPC (Tonic) para workers.
│   │
│   └── agent/                # EL AGENTE: Binario independiente que corre dentro del worker.
│
└── server/                   # BINARIO FINAL (Main):
    └── src/main.rs           # Inyección de dependencias (Wiring) y configuración.
```

---

## 2. El Sistema Nervioso: Bus de Eventos "Zero-Copy"

Eliminamos NATS para la comunicación interna. Al ser un solo proceso, usar TCP es un desperdicio.

### A. Implementación (`crates/adapters/bus`)
Implementaremos el puerto `EventBus` usando canales de alta concurrencia.

*   **Tecnología:** `tokio::sync::broadcast` para eventos (uno a muchos) y `tokio::sync::mpsc` para comandos.
*   **Mejora de Rendimiento:** Pasaremos `Arc<Event>` en lugar de datos serializados. Esto significa que mover un Job del Orchestrator al Scheduler tiene un coste de **nanosegundos** (copiar un puntero) en lugar de microsegundos (serializar/deserializar JSON).

```rust
// En crates/ports/src/events.rs
pub enum SystemEvent {
    JobCreated(Arc<Job>),
    WorkerConnected(WorkerId, WorkerCapabilities),
    LogChunkReceived(LogEntry), // Optimizado para no saturar
}

// En crates/adapters/src/bus/memory.rs
pub struct InMemoryBus {
    tx: broadcast::Sender<SystemEvent>,
}
// Implementación del trait EventPublisher...
```

---

## 3. Estrategia de Almacenamiento Dual (Persistencia)

El código actual usa `HashMaps` volátiles. Necesitamos persistencia real con opción de alto rendimiento.

### A. Definición del Puerto (`crates/ports/src/repo.rs`)
```rust
#[async_trait]
pub trait Repository: Send + Sync {
    async fn get_job(&self, id: &JobId) -> Result<Option<Job>>;
    async fn save_job(&self, job: &Job) -> Result<()>;
    // Crítico para el Scheduler:
    async fn get_pending_jobs_sorted(&self) -> Result<Vec<Job>>;
}
```

### B. Los Adaptadores
1.  **PostgreSQL (`sqlx`):** Para entornos de producción distribuidos o donde la durabilidad histórica es vital. Implementar pool de conexiones compartido.
2.  **Redb (Embedded):** Para despliegues "Single Node" de ultra-baja latencia.
    *   **Mejora:** Implementar un **Write-Ahead Log (WAL)** simple para la cola del scheduler. Si el servidor cae, al reiniciar, la cola en memoria se reconstruye desde Redb en milisegundos.

---

## 4. Hodei Worker Protocol (HWP) y el Agente Inteligente

Esta es la mejora más significativa respecto a tu código actual (`docker-provider` básico). Adoptamos el modelo **"Reverse Connect"** vía gRPC.

### A. Protocolo (Protobuf)
Definimos un servicio de streaming bidireccional.

```protobuf
service WorkerService {
  rpc Connect(stream AgentMessage) returns (stream ServerMessage);
}

message AgentMessage {
  oneof payload {
    Register register = 1;
    Heartbeat heartbeat = 2; // Incluye telemetry (CPU/RAM real)
    LogChunk log = 3;        // Bytes crudos + stream type (stdout/stderr)
  }
}
```

### B. El Agente (`crates/agent`)
Un binario Rust estático (~5MB) que se inyecta en los contenedores/VMs.
1.  **Inyección:** El `Provider` (Docker/K8s) arranca el contenedor montando el binario `hodei-agent` o usándolo como entrypoint.
2.  **Conexión:** El agente lee `HODEI_SERVER_URL` y `HODEI_TOKEN` y conecta al monolito.
3.  **Ejecución Eficiente:**
    *   Usa **PTY (Pseudo-Terminal)** para preservar colores y formato de logs.
    *   Implementa **Buffering Inteligente**: No envía cada línea de log individualmente. Agrupa logs en chunks de 4KB o por tiempo (100ms) para reducir syscalls y paquetes de red.
4.  **Seguridad:**
    *   Valida certificados TLS del servidor.
    *   **Secret Masking:** Antes de enviar logs al buffer, aplica una pasada rápida (algoritmo Aho-Corasick) para ocultar secretos en memoria.

---

## 5. Scheduler Inteligente y "Estado del Mundo"

Tu scheduler actual es una cola simple. Vamos a convertirlo en un planificador reactivo basado en carga real.

### A. Modelo de Datos en Memoria (`ClusterState`)
El scheduler mantendrá un `DashMap<WorkerId, NodeState>` en RAM.
*   **NodeState:** Contiene `capacidad_total`, `uso_reportado` (por el agente) y `uso_reservado` (jobs asignados pero no iniciados).

### B. Pipeline de Planificación (Loop)
En lugar de `fifo` o `random`, implementaremos fases estrictas:

1.  **Filtrado (Predicates):**
    *   ¿Tiene el nodo la etiqueta `gpu=true` si el job lo pide?
    *   ¿`memoria_libre_real` > `job_request_ram`? (Usando datos del Heartbeat gRPC).
2.  **Puntuación (Priorities):**
    *   **BinPacking:** Preferir nodos más llenos (ahorro en cloud).
    *   **LeastLoaded:** Preferir nodos vacíos (latencia en bare metal).
3.  **Binding (Asignación):**
    *   Reserva atómica en el `ClusterState` (RAM).
    *   Envío de comando `StartJob` vía gRPC al agente seleccionado.

---

## 6. Streaming de Logs de Alto Rendimiento

Eliminamos la escritura en base de datos para logs en vivo.

### El Flujo "Zero-Lag":
1.  **Agente:** Lee stdout -> Buffer -> gRPC Stream.
2.  **Worker Manager (Server):** Recibe gRPC -> Escribe en **RingBuffer** (Memoria Compartida `Arc<RwLock<VecDeque>>`).
    *   *Simultáneamente:* Un hilo background ("Log Archiver") vacía el RingBuffer a disco/S3 por lotes grandes para persistencia.
3.  **API HTTP:** El handler de Axum para `/logs/stream` se suscribe a notificaciones del RingBuffer y envía SSE (Server-Sent Events) al frontend.

**Resultado:** El log llega al usuario casi instantáneamente sin tocar el disco de la base de datos SQL, evitando el cuello de botella más común en sistemas CI/CD.

---

## 7. Plan de Implementación (Paso a Paso)

Para aplicar esto sobre tu código actual:

1.  **Fase 1: Refactorización de Módulos (Sin lógica nueva)**
    *   Mueve el código de `crates/orchestrator`, `scheduler` y `worker-manager` a `crates/modules/*`.
    *   Elimina los servidores HTTP internos (`axum`) de esos módulos. Conviértelos en `structs` con métodos públicos (Traits).
    *   Crea el `server/src/main.rs` que instancia todo.

2.  **Fase 2: Capa de Datos**
    *   Define el Trait `Repository`.
    *   Implementa `EmbeddedRepository` con Redb. Reemplaza los `HashMap` en memoria actuales por llamadas a este repo.

3.  **Fase 3: Comunicación Worker (gRPC)**
    *   Define el `.proto`.
    *   Crea el binario `hodei-agent`.
    *   Modifica `DockerProvider` en `worker-manager`: En lugar de usar la API de Docker para logs y exec, simplemente lanza el contenedor con el Agente y espera la conexión entrante gRPC.

4.  **Fase 4: Scheduler Avanzado**
    *   Conecta los datos de telemetría del gRPC (`Heartbeat`) al `ClusterState` del Scheduler.
    *   Implementa el algoritmo de Puntuación usando estos datos reales.

Esta arquitectura coloca a **Hodei Jobs** técnicamente por encima de Jenkins (por eficiencia/modernidad) y muy cerca de sistemas como Nomad o las versiones ligeras de K3s, pero especializado en Jobs efímeros.


---

Esta es la estrategia definitiva. Vamos a transformar tu código actual en un **Monolito Modular Hexagonal de Alto Rendimiento**, maximizando la reutilización de tu lógica de negocio (que es muy buena) y cambiando solo la "fontanería" (comunicación y persistencia) para usar **gRPC** y **Canales de Memoria**.

Aquí tienes la guía de implementación paso a paso, respetando tu código base actual.

---

### Fase 1: Reestructuración del Workspace (Sin borrar lógica)

No vamos a reescribir la lógica de negocio (`domain`, `application`), solo la moveremos de lugar.

**Estructura de Carpetas Nueva vs Actual:**

```text
.
├── crates/
│   ├── core/ (NUEVO: Mueve aquí shared-types)
│   │   └── src/lib.rs  <-- Tu código actual de shared-types
│   │
│   ├── ports/ (NUEVO: Definición de Traits/Interfaces)
│   │   ├── src/repo.rs
│   │   ├── src/bus.rs
│   │   └── src/proto.rs (Definiciones gRPC)
│   │
│   ├── modules/ (REFACTORIZACIÓN: Tus servicios actuales convertidos en librerías)
│   │   ├── orchestrator/  <-- Tu código actual de crates/orchestration/orchestrator/src/lib.rs + domain + application
│   │   ├── scheduler/     <-- Tu código actual de crates/scheduler/src/lib.rs + queue + pipeline + ...
│   │   └── worker-manager/ <-- Tu código actual de crates/worker-manager (Lógica de providers)
│   │
│   ├── adapters/ (NUEVO: Implementaciones técnicas)
│   │   ├── storage/ (Postgres + Redb)
│   │   ├── bus/ (Canales Tokio)
│   │   └── infrastructure/ (Docker/K8s providers adaptados)
│   │
│   └── agent/ (NUEVO: El binario que corre en el worker)
│
└── server/ (NUEVO: El binario principal)
    └── src/main.rs <-- Aquí unificamos todo
```

---

### Fase 2: Definición de Puertos (Ports)

En `crates/ports`, definimos las interfaces. Esto permite que tu código actual deje de depender de `HashMap` o `reqwest` concretos.

**`crates/ports/src/repo.rs`**:
```rust
use async_trait::async_trait;
use hodei_core::{Job, JobId, Pipeline}; // Tu código actual de shared-types

#[async_trait]
pub trait Repository: Send + Sync {
    // Jobs (Usando tus tipos actuales)
    async fn save_job(&self, job: &Job) -> Result<(), anyhow::Error>;
    async fn get_job(&self, id: &JobId) -> Result<Option<Job>, anyhow::Error>;
    async fn get_pending_jobs(&self) -> Result<Vec<Job>, anyhow::Error>;
    
    // Pipelines
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<(), anyhow::Error>;
}
```

**`crates/ports/src/bus.rs`**:
```rust
use async_trait::async_trait;
use hodei_core::events::SystemEvent; // Definir eventos basados en tus structs actuales

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: SystemEvent);
}

#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn subscribe(&self) -> tokio::sync::broadcast::Receiver<SystemEvent>;
}
```

---

### Fase 3: Adaptadores de Alto Rendimiento (Infrastructure)

Aquí implementamos la "magia" de eficiencia que pediste.

#### A. Bus en Memoria (Reemplaza a NATS interno)
En `crates/adapters/src/bus/memory.rs`:
```rust
use hodei_ports::bus::{EventPublisher, EventSubscriber};
use tokio::sync::broadcast;

// Ultrarrápido, sin serialización
pub struct InMemoryBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl InMemoryBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }
}
// Implementación de traits...
```

#### B. Repositorio Híbrido (Postgres / Embebido)
Usaremos tu código actual de lógica pero cambiando dónde guarda los datos.

En `crates/adapters/src/storage/postgres.rs`: Implementar `Repository` usando `sqlx`.
En `crates/adapters/src/storage/redb.rs`: Implementar `Repository` usando `redb` (base de datos embebida rápida).

---

### Fase 4: Refactorización de Módulos (Tu código existente)

Aquí es donde **preservamos tu trabajo**.

#### 1. Módulo Orchestrator
*   **Conservar:** `domain/`, `application/`.
*   **Eliminar:** `main.rs` y los handlers HTTP internos.
*   **Modificar:** `JobApplicationService` ahora recibe `Arc<dyn Repository>` y `Arc<dyn EventPublisher>` en su constructor, en lugar de usar `InMemoryJobRepository`.

```rust
// crates/modules/orchestrator/src/lib.rs
pub struct OrchestratorModule {
    repo: Arc<dyn Repository>,
    bus: Arc<dyn EventPublisher>,
}

impl OrchestratorModule {
    pub async fn create_job(&self, spec: JobSpec) -> Result<JobId> {
        let job = Job::new(spec); // Tu lógica actual de dominio
        self.repo.save_job(&job).await?; // Persistencia real
        self.bus.publish(SystemEvent::JobCreated(job.clone())).await; // Notificación Zero-copy
        Ok(job.id)
    }
}
```

#### 2. Módulo Scheduler
*   **Conservar:** `affinity/`, `pipeline/`, `queue/`, `selection/`. ¡Todo esto es oro puro!
*   **Modificar:** El `Scheduler` ya no expone una API HTTP. En su lugar, inicia un loop en background (`tokio::spawn`) que:
    1.  Escucha eventos del `EventBus` (`JobCreated`).
    2.  Usa tu lógica actual de `SchedulingPipeline` (Filter -> Score -> Bind).
    3.  Actualiza el estado en memoria (`ClusterState`) alimentado por los heartbeats de los agentes.

#### 3. Módulo Worker Manager
Este cambia más radicalmente para soportar el **Protocolo Hodei (gRPC)**.

*   **Conservar:** La lógica de `providers` (Docker/K8s) para crear contenedores.
*   **Cambiar:** En lugar de que `DockerProvider` ejecute comandos (`exec`), ahora solo debe **lanzar un contenedor con el Agente**.
*   **Nuevo:** Implementar el Servidor gRPC (`tonic`) que recibirá las conexiones de los agentes.

---

### Fase 5: El Agente y el Protocolo (HWP)

Esta es la pieza nueva que unifica Docker, K8s, etc.

#### 1. Definición (`proto/worker.proto`)
```protobuf
service WorkerService {
  rpc Connect(stream WorkerMsg) returns (stream ServerMsg);
}
message WorkerMsg {
  oneof payload {
    Register register = 1;
    Heartbeat heartbeat = 2; // CPU, RAM actuales
    LogChunk log = 3;
  }
}
```

#### 2. El Binario del Agente (`crates/agent`)
Es un pequeño ejecutable Rust que usarás en tus imágenes Docker base.
*   **Lógica:**
    1.  Conecta al `HODEI_SERVER_URL`.
    2.  Espera comando `StartJob`.
    3.  Ejecuta comando usando `tokio::process` (o `portable-pty` para mantener colores).
    4.  Hace streaming de stdout/stderr vía gRPC.

---

### Fase 6: El Monolito Modular (`server/src/main.rs`)

Aquí "cableamos" todo. Es el único punto de acoplamiento.

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Configuración
    let config = Config::from_env();

    // 2. Infraestructura (Adaptadores)
    let bus = Arc::new(InMemoryBus::new(10000));
    
    // Selección de persistencia (Tu requerimiento de dos alternativas)
    let repo: Arc<dyn Repository> = if config.use_embedded_db {
        Arc::new(RedbRepository::new("hodei.db")?)
    } else {
        Arc::new(PostgresRepository::new(&config.db_url).await?)
    };

    // 3. Inicializar Módulos (Inyección de Dependencias)
    let orchestrator = OrchestratorModule::new(repo.clone(), bus.clone());
    let scheduler = SchedulerModule::new(repo.clone(), bus.clone()); // Scheduler mantiene su estado en RAM + Repo
    let worker_manager = WorkerManagerModule::new(repo.clone(), bus.clone());

    // 4. API Gateway (Axum) - Expone la API REST al usuario
    let api_router = Router::new()
        .route("/jobs", post(|json| orchestrator.create_job(json)))
        .route("/logs", get(|ws| worker_manager.stream_logs(ws)));

    // 5. Servidor gRPC (Tonic) - Para los Agentes
    let grpc_service = WorkerGrpcService::new(worker_manager.clone());

    // 6. Ejecución Concurrente
    info!("Hodei Jobs Monolith running...");
    tokio::select! {
        _ = axum::serve(listener_http, api_router) => {},
        _ = Server::builder().add_service(grpc_service).serve(addr_grpc) => {},
        _ = scheduler.run_loop() => {}, // El loop infinito del scheduler
    }

    Ok(())
}
```

### Resumen de Mejoras Aplicadas

1.  **Modularidad Hexagonal:** Tu código actual de `orchestrator`, `scheduler` y `worker-manager` se preserva como lógica pura en `crates/modules`. Se eliminan sus servidores HTTP internos.
2.  **Rendimiento Interno:** Se reemplaza NATS por `tokio::channels`. Comunicación en nanosegundos.
3.  **Persistencia Dual:** Se introduce el trait `Repository` permitiendo usar `Redb` (embebido, ultrarrápido) o `Postgres` sin cambiar una línea de la lógica de negocio.
4.  **Abstracción de Workers (Jenkins-style):** Se introduce el Agente y gRPC.
    *   El `WorkerManager` ya no pelea con la API de Docker para logs.
    *   El Agente hace "push" de logs y métricas (CPU/RAM) en tiempo real.
5.  **Eficiencia del Scheduler:** El Scheduler recibe métricas reales del Agente (vía Bus interno) y actualiza su `ClusterState` en memoria para tomar decisiones de planificación inteligentes (Bin Packing / Least Loaded) al instante.

Este plan respeta el 90% de tu lógica de dominio actual pero cambia radicalmente la infraestructura para que sea **un solo binario fácil de desplegar y extremadamente rápido**.