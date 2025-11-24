# Sistema de Gestión de Workers - Análisis de Ingeniería Inversa

**Fecha:** 24 de noviembre de 2025
**Proyecto:** Hodei Jobs Platform
**Versión:** 1.0.0

---

## 📋 Resumen Ejecutivo

Este documento presenta un análisis de ingeniería inversa completo del sistema de gestión de workers de la plataforma Hodei Jobs. El sistema implementa una arquitectura distribuida con comunicación gRPC, scheduling inteligente basado en bin packing, y monitoring en tiempo real.

### Componentes Principales
- **Scheduler Module**: Orquestación y scheduling de jobs
- **HWP Agent**: Agente que corre en cada worker node
- **Worker Entity**: Dominio de worker con capacidades y estado
- **gRPC Protocol**: Protocolo de comunicación bidireccional
- **Heartbeat System**: Monitoring de salud y recursos

---

## 🏗️ Arquitectura General

```mermaid
graph TB
    %% Server Components
    subgraph "Hodei Server"
        SCH[Scheduler Module]
        SR[Worker Repository]
        E[Event Bus]
        J[Job Repository]
    end

    %% Worker Nodes
    subgraph "Worker Node 1"
        A1[HWP Agent]
        PM1[Process Manager]
        HS1[Heartbeat Sender]
        RM1[Resource Monitor]
    end

    subgraph "Worker Node 2"
        A2[HWP Agent]
        PM2[Process Manager]
        HS2[Heartbeat Sender]
        RM2[Resource Monitor]
    end

    subgraph "Worker Node N"
        AN[HWP Agent]
        PMN[Process Manager]
        HSN[Heartbeat Sender]
        RMN[Resource Monitor]
    end

    %% Connections
    SCH <-.->|gRPC Streaming| A1
    SCH <-.->|gRPC Streaming| A2
    SCH <-.->|gRPC Streaming| AN

    A1 <-.->|Heartbeat 5s| HS1
    A2 <-.->|Heartbeat 5s| HS2
    AN <-.->|Heartbeat 5s| HSN

    PM1 <-.->|Execute Jobs| RM1
    PM2 <-.->|Execute Jobs| RM2
    PMN <-.->|Execute Jobs| RMN

    SCH <-.-> SR
    SCH <-.-> E
    SCH <-.-> J

    %% Styling
    classDef server fill:#e1f5fe,stroke:#01579b,stroke-width:2px
    classDef worker fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef agent fill:#fff3e0,stroke:#e65100,stroke-width:2px

    class SCH,SR,E,J server
    class A1,A2,AN agent
    class PM1,PM2,PMN,HS1,HS2,HSN,RM1,RM2,RMN worker
```

---

## 🔄 Ciclo de Vida de un Worker

```mermaid
stateDiagram-v2
    [*] --> Creating: Worker Node Starts

    Creating --> Registering: HWP Agent starts
    Registering --> Available: Registration successful

    Available --> Running: Job assigned
    Running --> Available: Job completed/cancelled

    Available --> Unhealthy: Heartbeat timeout (>30s)
    Unhealthy --> Available: Reconnected

    Available --> Draining: Graceful shutdown
    Draining --> Terminated: All jobs finished

    Running --> Unhealthy: Connection lost
    Unhealthy --> Terminated: Max failures reached

    Unhealthy --> [*]: Error

    note right of Available
        - Accepts new jobs
        - Sends heartbeat every 5s
        - Reports resource usage
    end note
```

---

## 📡 Protocolo de Comunicación gRPC

### Servicios gRPC

```mermaid
graph LR
    subgraph "gRPC Services"
        RS[RegisterWorker]
        AJ[AssignJob]
        GWS[GetWorkerStatus]
        HB[Heartbeat]
        SL[StreamLogs]
        CJ[CancelJob]
        JS[JobStream]
    end

    subgraph "Message Flow"
        subgraph "Server → Worker"
            DIR1[AssignJobRequest]
            DIR2[CancelJobRequest]
        end

        subgraph "Worker → Server"
            DOR1[JobAccepted]
            DOR2[LogEntry]
            DOR3[JobResult]
        end
    end

    RS -.-> DOR1
    AJ -.-> DOR1
    JS <--> DIR1
    JS <--> DOR1
    JS <--> DOR2
    JS <--> DOR3
    HB -.-> SL
```

### Bidirectional Streaming (JobStream)

```mermaid
sequenceDiagram
    participant S as Hodei Server
    participant A as HWP Agent

    Note over S,A: Connection Established

    loop Every 5 seconds
        A->>S: Heartbeat(worker_id, timestamp, cpu, mem)
    end

    S->>A: AssignJobRequest(job_id, job_spec)

    alt Accept Job
        A->>S: JobAccepted(job_id)
        A->>A: Execute job
        loop Job logs
            A->>S: LogEntry(job_id, data)
        end
        A->>S: JobResult(job_id, exit_code, stdout, stderr)
    end

    S->>A: CancelJobRequest(job_id)
    A->>S: JobResult(job_id, exit_code, stdout, stderr)
```

---

## 🧠 Algoritmo de Scheduling

### Worker Selection Flow

```mermaid
flowchart TD
    A[Job Arrives] --> B[Get Available Workers]
    B --> C{Workers exist?}
    C -->|No| D[Return Error: NoEligibleWorkers]
    C -->|Yes| E[Calculate Worker Score]

    E --> F[Resource Utilization Score<br/>40% weight]
    E --> G[Load Balancing Score<br/>30% weight]
    E --> H[Health Score<br/>20% weight]
    E --> I[Capability Match Score<br/>10% weight]

    F --> J[Bin Packing Algorithm<br/>Optimal: 60-85% utilization]
    G --> K[Fewer jobs = Higher score]
    H --> L[Heartbeat recency]
    I --> M[Label/requirement match]

    J --> N[Combine Scores]
    K --> N
    L --> N
    M --> N

    N --> O[Select Max Score]
    O --> P[Return Best Worker]

    classDef process fill:#e8f5e9,stroke:#2e7d32
    classDef decision fill:#fff3e0,stroke:#e65100
    classDef output fill:#e3f2fd,stroke:#1565c0

    class A,B,E,F,G,H,I,J,K,L,M,N,O,P process
    class C decision
    class D output
```

### Bin Packing Algorithm

```mermaid
graph TD
    A[Job Request: 2 CPU, 4GB RAM] --> B[Available Workers]

    B --> W1[Worker 1: 8 CPU, 16GB<br/>Current: 2 CPU, 8GB]
    B --> W2[Worker 2: 4 CPU, 8GB<br/>Current: 3 CPU, 6GB]
    B --> W3[Worker 3: 2 CPU, 4GB<br/>Current: 0 CPU, 0GB]

    W1 --> F1[Utilization: (2+2)/8 = 50%<br/>Score: 0.8]
    W2 --> F2[Utilization: (2+3)/4 = 125%<br/>Score: 0.1 (Over-provisioned)]
    W3 --> F3[Utilization: (2+0)/2 = 100%<br/>Score: 0.3 (Tight fit)]

    F1 --> P[Select Worker 1<br/>Best bin packing fit]

    note right of W1
        Optimal range:
        60-85%
    end note
```

---

## 💓 Sistema de Heartbeat

```mermaid
graph TB
    subgraph "Heartbeat Flow"
        C[Scheduler Config<br/>Timeout: 30s]

        subgraph "Worker Node"
            T1[Timer: 5s]
            RM[Resource Monitor<br/>CPU, Memory, PIDs]
            HS[Heartbeat Sender]
        end

        subgraph "Server Side"
            DB[(Worker Repository)]
            C1[Check Heartbeat]
            U1[Update Worker Status]
        end

        T1 -->|Every 5s| RM
        RM --> HS
        HS -->|gRPC: Heartbeat| C1
        C1 --> U1
        U1 --> DB

        DB -.->|is_healthy()?| C1
    end

    subgraph "Health States"
        H1[Healthy:< 30s old]
        H2[Unhealthy:> 30s old]
        H3[Marked for rescheduling]
    end

    C1 -->|Heartbeat recent| H1
    C1 -->|Heartbeat stale| H2
    H2 -->|No recovery| H3

    classDef config fill:#f5f5f5,stroke:#757575
    classDef process fill:#e8f5e9,stroke:#2e7d32
    classDef storage fill:#e1f5fe,stroke:#01579b
    classDef state fill:#fff3e0,stroke:#e65100

    class C config
    class T1,RM,HS,C1,U1 process
    class DB storage
    class H1,H2,H3 state
```

---

## 🏭 Worker Entity (Dominio)

```mermaid
classDiagram
    class Worker {
        +WorkerId id
        +String name
        +WorkerStatus status
        +DateTime created_at
        +DateTime updated_at
        +Option~String~ tenant_id
        +WorkerCapabilities capabilities
        +HashMap~String,String~ metadata
        +Vec~Uuid~ current_jobs
        +DateTime last_heartbeat
        --
        +new(id, name, capabilities) Worker
        +with_tenant_id(tenant_id) Worker
        +with_metadata(key, value) Worker
        +update_status(status) void
        +is_available() bool
        +register_job(job_id) Result
        +unregister_job(job_id) void
        +is_healthy() bool
    }

    class WorkerCapabilities {
        +u32 cpu_cores
        +u64 memory_gb
        +Option~u8~ gpu
        +Vec~String~ features
        +HashMap~String,String~ labels
        +u32 max_concurrent_jobs
        --
        +new(cpu, memory) WorkerCapabilities
    }

    class WorkerStatus {
        +WorkerId worker_id
        +String status
        +Vec~JobId~ current_jobs
        +SystemTime last_heartbeat
        --
        +IDLE "IDLE"
        +BUSY "BUSY"
        +OFFLINE "OFFLINE"
        +DRAINING "DRAINING"
        +new(worker_id, status) WorkerStatus
        +is_available() bool
    }

    class WorkerState {
        <<enumeration>>
        +Creating
        +Available
        +Running
        +Unhealthy
        +Draining
        +Terminated
        +Failed(reason)
    }

    Worker --> WorkerCapabilities: has
    Worker --> WorkerStatus: has
    WorkerStatus --> WorkerState: status as
```

---

## 🔧 Job Execution Flow

```mermaid
sequenceDiagram
    participant S as Scheduler
    participant R as Worker Repository
    participant W as Selected Worker
    participant A as HWP Agent
    participant PM as Process Manager

    S->>R: Find available workers
    R->>S: List of workers

    S->>S: Calculate best worker

    S->>W: register_job(job_id)
    W->>S: Ok (job registered)

    S->>A: JobStream: AssignJobRequest

    A->>S: JobAccepted(job_id)

    par Execute Job
        A->>PM: Execute job spec
        PM->>PM: Spawn process
        PM->>PM: Monitor execution
    and Stream Logs
        loop Every log line
            PM->>A: Log data
            A->>S: LogEntry
        end
    end

    alt Job Success
        PM->>A: exit_code: 0
        A->>S: JobResult(success)
        A->>W: unregister_job(job_id)
    or Job Failure
        PM->>A: exit_code: != 0
        A->>S: JobResult(failure)
        A->>W: unregister_job(job_id)
    or Cancel Request
        S->>A: CancelJobRequest
        PM->>A: Job terminated
        A->>S: JobResult(cancelled)
        A->>W: unregister_job(job_id)
    end

    W->>S: Worker now available
```

---

## 🗄️ Persistencia y Estado

```mermaid
graph TD
    subgraph "Data Storage"
        subgraph "Worker Repository"
            WDB[(Workers Table)]
        end

        subgraph "Job Repository"
            JDB[(Jobs Table)]
        end

        subgraph "In-Memory"
            CACHE[Worker Cache]
            QUEUE[Priority Queue]
        end
    end

    subgraph "Worker Data Model"
        W1[Worker {
            id: UUID,
            name: String,
            status: String,
            capabilities: JSON,
            metadata: JSON,
            current_jobs: UUID[],
            last_heartbeat: Timestamp
        }]
    end

    WDB -.-> W1
    CACHE -.-> W1

    JDB --> QUEUE

    note right of CACHE
        - DashMap for lock-free access
        - Worker status in real-time
        - Health checks every 30s
    end note

    note right of QUEUE
        - LockFreePriorityQueue
        - 3 priority levels (high/normal/low)
        - O(1) enqueue/dequeue
    end
```

---

## 🎯 Capacidades y Compatibilidad

```mermaid
graph TB
    subgraph "Worker Capabilities"
        C1[CPU Cores: 4]
        C2[Memory: 8GB]
        C3[GPU: None]
        C4[Features: [docker, gpu]]
        C5[Labels: {env: prod}]
        C6[Max Concurrent Jobs: 4]
    end

    subgraph "Job Requirements"
        J1[CPU: 2000m (2 cores)]
        J2[Memory: 4096MB (4GB)]
        J3[GPU: Not required]
        J4[Labels: {env: prod}]
    end

    C1 -->|2000m ≤ 4000m| M1[✓ CPU Compatible]
    C2 -->|4GB ≤ 8GB| M2[✓ Memory Compatible]
    C3 -->|None OK| M3[✓ GPU Compatible]
    C4 -->|labels match| M4[✓ Labels Match]
    C5 -->|Max jobs: 4| M5[✓ Has Capacity]

    M1 --> F[Match Score: 100%]
    M2 --> F
    M3 --> F
    M4 --> F
    M5 --> F

    classDef worker fill:#e8f5e9,stroke:#2e7d32
    classDef job fill:#e3f2fd,stroke:#1565c0
    classDef match fill:#fff3e0,stroke:#e65100

    class C1,C2,C3,C4,C5,C6 worker
    class J1,J2,J3,J4 job
    class M1,M2,M3,M4,M5,F match
```

---

## 🚀 Performance Optimizations

### Lock-Free Data Structures

```mermaid
graph LR
    subgraph "LockFreePriorityQueue"
        Q1[SegQueue<br/>High Priority]
        Q2[SegQueue<br/>Normal Priority]
        Q3[SegQueue<br/>Low Priority]
        S[AtomicUsize<br/>Queue Size]
    end

    subgraph "Worker Cache"
        DM[DashMap<br/>WorkerId → Worker]
    end

    subgraph "Cluster State"
        QM[Queue<br/>Workers to schedule]
    end

    Q1 -->|O(1) Enqueue| S
    Q2 -->|O(1) Enqueue| S
    Q3 -->|O(1) Enqueue| S

    S -.->|Batching| QM

    DM -.->|Read/Write| QM

    classDef queue fill:#f3e5f5,stroke:#7b1fa2
    classDef cache fill:#e1f5fe,stroke:#0277bd
    classDef state fill:#fff3e0,stroke:#ef6c00

    class Q1,Q2,Q3,S queue
    class DM cache
    class QM state
```

### Optimization Techniques

1. **Lock-Free Queues** (O(1) operations)
   - `crossbeam::queue::SegQueue` for job queue
   - `dashmap::DashMap` for worker cache
   - `crossbeam::utils::CachePadded` for counters

2. **Batch Processing**
   - Dequeue up to batch_size jobs at once
   - Prioritize high → normal → low queue
   - Reduce contention

3. **Connection Pooling**
   - gRPC channel reuse
   - Keep-alive connections
   - Exponential backoff for reconnects

4. **Streaming Protocol**
   - Bidirectional gRPC stream
   - Real-time job assignments
   - Log streaming during execution

---

## 🔍 Health Monitoring

```mermaid
flowchart TD
    A[Worker Health Check] --> B{Heartbeat < 30s?}

    B -->|Yes| C[Mark as Healthy]
    B -->|No| D[Mark as Unhealthy]

    C --> E[Available for scheduling]
    D --> F[Remove from available pool]

    F --> G{Attempts to reconnect?}
    G -->|Success| C
    G -->|Failure| H[Max failures reached]

    H --> I[Mark for cleanup]
    I --> J[Reschedule active jobs]

    subgraph "Health Metrics"
        M1[CPU Usage %]
        M2[Memory Usage MB]
        M3[Active Jobs Count]
        M4[Heartbeat Timestamp]
    end

    C -.-> M1
    C -.-> M2
    C -.-> M3
    C -.-> M4

    classDef healthy fill:#c8e6c9,stroke:#2e7d32
    classDef unhealthy fill:#ffcdd2,stroke:#c62828
    classDef process fill:#e1f5fe,stroke:#01579b
    classDef metric fill:#fff3e0,stroke:#e65100

    class C,E healthy
    class D,F,H,I,J unhealthy
    class A,B,G process
    class M1,M2,M3,M4 metric
```

---

## 📊 Configuración del Sistema

### Scheduler Config
```toml
[scheduler]
max_queue_size = 1000           # Max jobs in queue
scheduling_interval_ms = 1000   # Scheduling loop interval
worker_heartbeat_timeout_ms = 30000  # Worker timeout
```

### Heartbeat Config
```toml
[heartbeat]
interval_ms = 5000              # Heartbeat frequency
max_failures = 3                # Max consecutive failures
failure_timeout_ms = 30000      # Failure threshold
```

### Worker Config
```toml
[worker]
cpu_cores = 4
memory_gb = 8
max_concurrent_jobs = 4
features = ["docker", "gpu"]
```

---

## 🎬 Escenarios de Uso

### Escenario 1: Worker Registration
```mermaid
sequenceDiagram
    participant HN as Hodei Node
    participant AG as HWP Agent
    participant S as Scheduler
    participant R as Repository

    HN->>AG: Start HWP Agent
    AG->>S: RegisterWorker(id, capabilities)
    S->>R: Save worker
    R-->>S: Worker saved
    S-->>AG: Status: Available
    AG->>AG: Start heartbeat timer
```

### Escenario 2: Job Assignment
```mermaid
sequenceDiagram
    participant J as Job Submitted
    participant S as Scheduler
    participant W as Worker
    participant A as HWP Agent
    participant P as Process

    J->>S: Submit job
    S->>S: Select best worker
    S->>W: register_job()
    W-->>S: Ok
    S->>A: JobStream: AssignJob
    A->>P: Execute job
    A-->>S: JobAccepted
    P->>A: Logs
    A->>S: LogEntry
    P->>A: Exit
    A->>S: JobResult
```

### Escenario 3: Worker Failure
```mermaid
sequenceDiagram
    participant S as Scheduler
    participant W as Worker
    participant R as Repository

    W->>S: Heartbeat (healthy)
    W->>W: Connection lost
    Note over W,S: 30 seconds pass...

    S->>S: Check health
    S->>W: is_healthy()? = false
    S->>R: Mark unhealthy
    S->>S: Reschedule jobs
    S->>S: Find backup worker
    S->>S: Assign orphaned jobs
```

---

## 🔐 Seguridad

### mTLS Configuration
```mermaid
graph TD
    subgraph "Client (HWP Agent)"
        C1[TLS Cert Path]
        C2[TLS Key Path]
        C3[TLS CA Path]
    end

    subgraph "Server (Hodei)"
        S1[CA Certificate]
        S2[Verify Client Cert]
    end

    C1 -->|Client Cert| S2
    C2 -->|Client Key| S2
    C3 -->|CA Cert| S2

    S2 -->|Verify| S1

    classDef cert fill:#f5f5f5,stroke:#757575
    classDef verify fill:#e8f5e9,stroke:#2e7d32

    class C1,C2,C3,S1 cert
    class S2 verify
```

### JWT Authentication
- Worker registration uses JWT tokens
- Token validation on every gRPC call
- Role-based access control:
  - Worker: Execute assigned jobs
  - Scheduler: Manage all workers
  - Admin: Full access

---

## 📈 Métricas y Observabilidad

### Prometheus Metrics
```mermaid
graph TB
    subgraph "Worker Metrics"
        M1[worker_heartbeats_total]
        M2[worker_jobs_running]
        M3[worker_cpu_usage_percent]
        M4[worker_memory_usage_mb]
    end

    subgraph "Scheduler Metrics"
        M5[scheduler_jobs_queued]
        M6[scheduler_assignment_latency]
        M7[scheduler_worker_selection_score]
    end

    subgraph "Job Metrics"
        M8[job_execution_duration]
        M9[job_success_rate]
        M10[job_failure_reason]
    end

    M1 --> P[Prometheus]
    M2 --> P
    M3 --> P
    M4 --> P
    M5 --> P
    M6 --> P
    M7 --> P
    M8 --> P
    M9 --> P
    M10 --> P
```

### Logs
- Structured logging with `tracing` crate
- Log levels: ERROR, WARN, INFO, DEBUG
- Correlation IDs for request tracing
- Job execution logs streamed in real-time

---

## 🔄 Recovery Mechanisms

### Automatic Recovery
```mermaid
flowchart TD
    A[Failure Detected] --> B{Failure Type}

    B -->|Network| C[Exponential backoff reconnect]
    B -->|Worker Unhealthy| D[Mark unhealthy, reschedule]
    B -->|Process Crash| E[Restart process, notify]
    B -->|Server Restart| F[Full reconnection, sync state]

    C --> G{Connected?}
    G -->|Yes| H[Resume normal operations]
    G -->|No| C

    D --> I[Update repository]
    I --> J[Find backup worker]
    J --> K[Reassign jobs]

    E --> L[Cleanup resources]
    L --> M[Notify scheduler]

    F --> N[Re-register worker]
    N --> O[Sync pending jobs]

    classDef failure fill:#ffebee,stroke:#c62828
    classDef recovery fill:#e8f5e9,stroke:#2e7d32
    classDef state fill:#fff3e0,stroke:#e65100

    class A failure
    class C,D,E,F recovery
    class G,H,I,J,K,L,M,N,O state
```

---

## 🎯 Mejoras y Optimizaciones Futuras

### Prioridad Alta
1. **Worker Autoscaling**
   - Dynamic worker provisioning
   - Cloud provider integration (AWS, GCP, Azure)
   - Auto-scaling based on queue depth

2. **GPU Scheduling**
   - GPU-aware bin packing
   - CUDA device monitoring
   - Multi-GPU worker support

3. **Multi-Region**
   - Geographic worker distribution
   - Data locality optimization
   - Cross-region job migration

### Prioridad Media
4. **Job Dependencies**
   - DAG-aware scheduling
   - Parallel step execution
   - Dependency resolution optimization

5. **Resource Prediction**
   - ML-based resource forecasting
   - Historical data analysis
   - Predictive autoscaling

### Prioridad Baja
6. **Fault Tolerance**
   - Multi-master scheduler
   - Worker replication
   - Automatic failover

---

## 📚 Referencias de Código

### Archivos Principales
- `/crates/core/src/worker.rs` - Worker entity y domain logic
- `/crates/shared-types/src/worker_messages.rs` - Message types
- `/crates/modules/src/scheduler/mod.rs` - Scheduler implementation
- `/crates/adapters/src/worker_client.rs` - gRPC client
- `/crates/hwp-agent/src/main.rs` - Agent entry point
- `/crates/hwp-agent/src/connection/grpc_client.rs` - gRPC connection
- `/crates/hwp-agent/src/monitor/heartbeat.rs` - Heartbeat system
- `/crates/hwp-proto/protos/hwp.proto` - Protocol definitions

### Puertos y Adaptadores
- `/crates/ports/src/worker_repository.rs` - Repository interface
- `/crates/ports/src/worker_client.rs` - Client interface

---

## 🏷️ Tags

`#worker-management` `#scheduling` `#grpc` `#distributed-systems` `#job-execution` `#bin-packing` `#heartbeat` `#worker-entity` `#hwp-agent` `#performance` `#monitoring`

---

**Documento generado:** 24 de noviembre de 2025
**Próxima revisión:** 1 de diciembre de 2025
**Versión:** 1.0.0
**Estado:** ✅ Completo
