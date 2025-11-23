# Diagrama de Arquitectura Hexagonal - Hodei Jobs

## Arquitectura General

```mermaid
graph TB
    subgraph "Cliente"
        CLI[CLI Tool]
        WEB[Web Dashboard]
        API[Third-party APIs]
    end

    subgraph "🚀 Hodei Jobs Monolito (Un Solo Binario)"
        subgraph "Application Layer (Use Cases)"
            ORCH[Orchestrator Module]
            SCHED[Scheduler Module]
            WM[Worker Manager Module]
        end

        subgraph "Domain Layer (Core Business Logic)"
            CORE[Core Types]
            JOB[Job Entity]
            PIPELINE[Pipeline Entity]
            WORKER[Worker Entity]
        end

        subgraph "Ports (Interfaces)"
            REPO_PORT[JobRepository Trait]
            BUS_PORT[EventBus Trait]
            WORKER_PORT[WorkerClient Trait]
        end

        subgraph "Adapters (Infrastructure)"
            subgraph "Storage Adapters"
                REDB[(Redb<br/>Embedded)]
                PG[(PostgreSQL<br/>Production)]
            end

            subgraph "Bus Adapters"
                MEM_BUS[InMemoryBus<br/>Zero-Copy]
            end

            subgraph "RPC Adapters"
                GRPC[GRPC Server<br/>Tonic]
            end
        end

        subgraph "Infrastructure Layer"
            HTTP[HTTP Server<br/>Axum]
            SSE[SSE Endpoints<br/>Log Streaming]
        end
    end

    subgraph "🏃 Agentes Descentralizados"
        subgraph "Worker Agents (gRPC Clients)"
            AGENT1[Agent Docker]
            AGENT2[Agent K8s]
            AGENT3[Agent VM]
        end

        subgraph "Agent Features"
            PTY[PTY Support]
            BUFFER[Log Buffering]
            MASK[Secret Masking]
            METRICS[Resource Metrics]
        end
    end

    subgraph "External Systems"
        DOCKER[Docker Engine]
        K8S[Kubernetes API]
        VAULT[HashiCorp Vault]
        S3[Object Storage]
    end

    %% Client connections
    CLI --> HTTP
    WEB --> HTTP
    API --> HTTP
    WEB --> SSE

    %% Application layer
    HTTP --> ORCH
    HTTP --> SCHED
    HTTP --> WM
    SSE --> WM

    %% Core connections
    ORCH --> CORE
    SCHED --> CORE
    WM --> CORE

    %% Port connections
    ORCH --> REPO_PORT
    ORCH --> BUS_PORT
    SCHED --> REPO_PORT
    SCHED --> BUS_PORT
    SCHED --> WORKER_PORT
    WM --> REPO_PORT
    WM --> BUS_PORT
    WM --> GRPC

    %% Adapter connections
    REPO_PORT --> REDB
    REPO_PORT --> PG
    BUS_PORT --> MEM_BUS
    GRPC --> AGENT1
    GRPC --> AGENT2
    GRPC --> AGENT3

    %% Agent to infrastructure
    AGENT1 --> DOCKER
    AGENT2 --> K8S
    AGENT3 --> VAULT

    %% Agent features
    AGENT1 --> PTY
    AGENT2 --> BUFFER
    AGENT3 --> MASK
    AGENT1 --> METRICS

    classDef client fill:#e1f5fe
    classDef app fill:#f3e5f5
    classDef core fill:#e8f5e9
    classDef port fill:#fff3e0
    classDef adapter fill:#fce4ec
    classDef infra fill:#f1f8e9
    classDef agent fill:#e0f2f1
    classDef external fill:#f5f5f5

    class CLI,WEB,API client
    class ORCH,SCHED,WM app
    class CORE,JOB,PIPELINE,WORKER core
    class REPO_PORT,BUS_PORT,WORKER_PORT port
    class REDB,PG,MEM_BUS,GRPC adapter
    class HTTP,SSE infra
    class AGENT1,AGENT2,AGENT3,PTY,BUFFER,MASK,METRICS agent
    class DOCKER,K8S,VAULT,S3 external
```

## Flujo de Ejecución de Job

```mermaid
sequenceDiagram
    participant U as User (CLI)
    participant M as Monolith API
    participant O as Orchestrator
    participant B as EventBus
    participant S as Scheduler
    participant W as Worker Manager
    participant G as gRPC Server
    participant A as Agent
    participant D as Docker/K8s

    U->>M: POST /jobs (Pipeline spec)
    M->>O: create_job()
    O->>O: Validate pipeline
    O->>O: Create Job entity
    O->>REPO: save_job(job) [Redb/Postgres]
    O->>B: publish(JobCreated)
    O-->>M: Job created
    M-->>U: 201 Created

    Note over B: Zero-Copy Event
    B->>S: JobCreated event
    S->>S: Load ClusterState
    S->>S: Filter eligible workers
    S->>S: Score workers (bin-packing)
    S->>S: Select best worker
    S->>S: Reserve worker (atomic)
    S->>REPO: update_job_status
    S->>B: publish(JobScheduled)

    B->>W: JobScheduled event
    W->>G: assign_job(job, worker_id)
    G->>A: Connect (gRPC stream)

    Note over A: Agent Connection
    A->>A: Read HODEI_SERVER_URL
    A->>A: Read HODEI_TOKEN
    A->>G: Connect(stream)
    G->>A: Authenticate token
    A->>G: Send Register
    G->>A: Send AssignJob

    A->>A: Create temp dir
    A->>A: Launch PTY
    A->>D: Start container (if needed)
    A->>A: Execute command
    A->>G: Stream logs (LogChunk)

    loop Real-time logs
        A->>G: LogChunk{stdout}
        G->>M: Update RingBuffer
        M->>SSE: Notify subscribers
        SSE-->>U: SSE: data: {...}
    end

    A->>G: Send JobResult{exit_code}
    G->>W: job_completed()
    W->>REPO: save_result()
    W->>B: publish(JobCompleted)
    B->>O: JobCompleted
    O->>REPO: finalize_job()

    U->>M: GET /jobs/{id}
    M->>REPO: get_job()
    M-->>U: 200 OK (status: completed)
```

## Protocolo de Agente (HWP)

```mermaid
stateDiagram-v2
    [*] --> Starting: Container starts

    Starting --> Authenticating: Read env vars
    Authenticating --> Connected: TLS handshake
    Connected --> Registering: Send token

    Registering --> Waiting: Registration ACK
    Waiting --> Executing: Receive AssignJob
    Waiting --> Terminating: Receive Shutdown

    Executing --> StreamingLogs: Command running
    StreamingLogs --> Completed: Process exit
    StreamingLogs --> Cancelled: Receive CancelJob

    Completed --> Waiting: Ready for next job
    Cancelled --> Waiting: Ready for next job

    Waiting --> Terminating: Connection lost
    StreamingLogs --> Terminating: Fatal error

    Terminating --> [*]: Cleanup
```

## Persistencia Dual

```mermaid
graph LR
    subgraph "Application Decision"
        CONFIG{Configuration}
        CONFIG -->|use_embedded=true| EDGE[Edge Mode]
        CONFIG -->|use_embedded=false| PROD[Production Mode]
    end

    subgraph "Edge Mode (High Performance)"
        EDGE --> REDB[(Redb<br/>Embedded)]
        REDB --> MMAP[Memory Mapped Files]
        MMAP --> ZERO[Zero-Network Latency]
    end

    subgraph "Production Mode (Durable)"
        PROD --> PG[(PostgreSQL)]
        PG --> POOL[Connection Pool]
        POOL --> TX[ACID Transactions]
    end

    subgraph "Common Interface"
        REPO_IF[JobRepository Trait]
        REDB --> REPO_IF
        PG --> REPO_IF
        REPO_IF --> MODULES[All Modules]
    end

    classDef decision fill:#ffecb3
    classDef edge fill:#c8e6c9
    classDef prod fill:#bbdefb
    classDef interface fill:#f8bbd0

    class CONFIG decision
    class EDGE,REDB,MMAP,ZERO edge
    class PROD,PG,POOL,TX prod
    class REPO_IF,MODULES interface
```

## Bus de Eventos Zero-Copy

```mermaid
graph TB
    subgraph "Monolith Process"
        subgraph "Event Producers"
            PROD1[Orchestrator]
            PROD2[Scheduler]
            PROD3[Worker Manager]
        end

        subgraph "InMemoryBus"
            TX[broadcast::Sender]
            RX1[broadcast::Receiver 1]
            RX2[broadcast::Receiver 2]
            RX3[broadcast::Receiver 3]
        end

        subgraph "Event Consumers"
            CONS1[Scheduler Loop]
            CONS2[Log Aggregator]
            CONS3[Metrics Collector]
        end
    end

    PROD1 -->|Arc<Job>| TX
    PROD2 -->|JobScheduled| TX
    PROD3 -->|LogChunk| TX

    TX --> RX1
    TX --> RX2
    TX --> RX3

    RX1 --> CONS1
    RX2 --> CONS2
    RX3 --> CONS3

    Note over TX,RX1: Zero-copy: Only Arc pointer copied (8 bytes)
    Note over PROD1,CONS1: Latency: ~10-50μs (vs ~1-5ms with NATS)
```

## Scheduler Inteligente

```mermaid
graph TD
    JOB[New Job Received] --> FILTER{Filter Phase}

    FILTER -->|has label: gpu=true| ELIGIBLE[Eligible Workers]
    FILTER -->|mem_required=8GB| ELIGIBLE
    FILTER -->|cpu_cores=4| ELIGIBLE

    ELIGIBLE --> SCORE{Score Phase}

    SCORE -->|BinPacking| RANKED[Ranked Workers]
    SCORE -->|Load Aware| RANKED
    SCORE -->|Affinity| RANKED

    RANKED --> RESERVE[Atomic Reservation]

    RESERVE --> ASSIGN[Assign to Worker]

    ASSIGN --> gRPC[Send via gRPC]

    gRPC --> AGENT[Agent receives]

    AGENT --> ACK[Send ACK]

    RESERVE -.->|Failed| SCORE

    subgraph "ClusterState (In-Memory)"
        WORKER1[Worker1: CPU 80%, RAM 60%]
        WORKER2[Worker2: CPU 20%, RAM 30%]
        WORKER3[Worker3: CPU 50%, RAM 70%]
    end

    WORKER1 --> SCORE
    WORKER2 --> SCORE
    WORKER3 --> SCORE
```

## Log Streaming Pipeline

```mermaid
graph LR
    subgraph "Agent (Container)"
        PROCESS[User Process]
        PTY[PTY Terminal]
        BUFFER[4KB Buffer]
        MASK[Secret Masker]
        GRPC[gRPC Client]
    end

    subgraph "Monolith"
        GRPC_S[GRPC Server]
        RING[RingBuffer<br/>Shared Memory]
        ARCHIVE[Background Archiver]
        SSE[SSE Endpoint]
    end

    subgraph "Client"
        BROWSER[Web Browser]
        DASHBOARD[Dashboard]
    end

    PROCESS --> PTY
    PTY --> BUFFER
    BUFFER --> MASK
    MASK --> GRPC
    GRPC --> GRPC_S

    GRPC_S --> RING
    RING --> SSE
    SSE --> BROWSER

    RING --> ARCHIVE
    ARCHIVE --> S3[(S3/Disk)]

    BROWSER --> DASHBOARD

    Note over PTY,RING: Zero-copy within process
    Note over GRPC_S,SSE: <10ms latency
```

## Dependencias y Tecnologías

```mermaid
graph TB
    subgraph "Rust Crates"
        TOKIO[ Tokio<br/>Async Runtime]
        TONIC[ Tonic<br/>gRPC]
        SQLX[ SQLx<br/>PostgreSQL]
        REDB[ Redb<br/>Embedded DB]
        SERDE[ Serde<br/>Serialization]
        AXUM[ Axum<br/>HTTP Server]
        CROSSBEAM[ Crossbeam<br/>Channels]
    end

    subgraph "Hodei Jobs"
        AGENT[Agent Binary]
        SERVER[Monolith Server]
        MODULES[Modules]
    end

    AGENT --> TOKIO
    AGENT --> TONIC
    AGENT --> SERDE

    SERVER --> TOKIO
    SERVER --> TONIC
    SERVER --> SQLX
    SERVER --> REDB
    SERVER --> AXUM
    SERVER --> CROSSBEAM

    MODULES --> SERDE

    note right of AGENT: 5MB static binary
    note right of SERVER: Single binary
    note right of MODULES: Pure Rust
```
