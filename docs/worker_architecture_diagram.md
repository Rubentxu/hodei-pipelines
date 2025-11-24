# Arquitectura del Sistema de Workers - Diagrama Visual

## Vista General del Sistema

```mermaid
graph TB
%% User/API Layer
    subgraph "API Layer"
        API[REST/gRPC API]
    end

%% Server Core
    subgraph "Hodei Server"
    %% Scheduler
        subgraph "Scheduler Module"
            S1[LockFreePriorityQueue]
            S2[Bin Packing Algorithm]
            S3[Worker Selection]
            S4[Health Monitor]
        end

    %% Repositories
        subgraph "Data Layer"
            WR[Worker Repository]
            JR[Job Repository]
            EB[Event Bus]
        end

    %% Server gRPC
        subgraph "gRPC Server"
            SRV[WorkerService]
            RS[RegisterWorker]
            AS[AssignJob]
            HS[Heartbeat Handler]
        end
    end

%% Worker Nodes
    subgraph "Worker Node Cluster"
        subgraph "Worker Node 1"
            A1[HWP Agent]
            C1[gRPC Client]
            PM1[Process Manager]
            HS1[Heartbeat Sender]
            RM1[Resource Monitor]
            P1[Docker/Container Runtime]
        end

        subgraph "Worker Node 2"
            A2[HWP Agent]
            C2[gRPC Client]
            PM2[Process Manager]
            HS2[Heartbeat Sender]
            RM2[Resource Monitor]
            P2[Docker/Container Runtime]
        end

        subgraph "Worker Node N..."
            AN[HWP Agent]
            CN[gRPC Client]
            PMN[Process Manager]
            HSN[Heartbeat Sender]
            RMN[Resource Monitor]
            PN[Docker/Container Runtime]
        end
    end

%% External Systems
    subgraph "External"
        DB[PostgreSQL]
        LOG[Log Aggregation]
        MET[Metrics Prometheus]
    end

%% Flow: API to Scheduler
    API --> S1
    S1 --> S2
    S2 --> S3
    S3 --> S4
    S4 --> WR

%% Flow: Scheduler to Workers
    S3 <-->|JobStream gRPC| C1
    S3 <-->|JobStream gRPC| C2
    S3 <-->|JobStream gRPC| CN

%% Worker Internal Flow
    A1 <--> C1
    C1 --> PM1
    PM1 --> P1
    RM1 --> HS1
    HS1 -->|Heartbeat| HS

    A2 <--> C2
    C2 --> PM2
    PM2 --> P2
    RM2 --> HS2
    HS2 -->|Heartbeat| HS

    AN <--> CN
    CN --> PMN
    PMN --> PN
    RMN --> HSN
    HSN -->|Heartbeat| HS

%% Data Flow
    WR --> DB
    JR --> DB
    EB --> LOG
    S4 --> MET

%% gRPC Services
    RS --> WR
    AS --> JR
    HS --> WR

%% Styling
    classDef server fill:#e1f5fe,stroke:#01579b,stroke-width:2px
    classDef scheduler fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    classDef worker fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef agent fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef storage fill:#fce4ec,stroke:#880e4f,stroke-width:2px
    classDef external fill:#f5f5f5,stroke:#757575,stroke-width:2px

    class API,SRV,RS,AS,HS server
    class S1,S2,S3,S4 scheduler
    class A1,A2,AN,C1,C2,CN agent
    class PM1,PM2,PMN,HS1,HS2,HSN,RM1,RM2,RMN,P1,P2,PN worker
    class WR,JR,EB storage
    class DB,LOG,MET,API external

```

---

## Flujo de Datos Detallado

```mermaid
sequenceDiagram
    participant U as User/API
    participant SCH as Scheduler
    participant R as Worker Repository
    participant W as Worker Node
    participant A as HWP Agent
    participant PM as Process Manager
    participant D as Docker
    participant L as Log System
    participant M as Metrics

    Note over U,M: Job Submission & Execution

    U->>SCH: Submit Job (CPU: 2, MEM: 4GB)

    SCH->>R: Get Available Workers
    R-->>SCH: Worker list

    SCH->>SCH: Bin Packing Algorithm
    Note right of SCH: Calculate best fit<br/>Optimal: 60-85% utilization

    SCH->>R: Register job to worker
    R-->>SCH: OK

    SCH->>W: gRPC JobStream: AssignJob
    W->>A: Receive job

    par Execution
        A->>PM: Execute job
        PM->>D: Docker run
        D->>PM: Container started
    and Monitoring
        loop Every 5 seconds
            RM->>A: Resource metrics
            A->>SCH: Heartbeat(cpu, mem, jobs)
            SCH->>M: Record metrics
        end
    and Logging
        loop Log lines
            D->>PM: Stdout/Stderr
            PM->>A: Log data
            A->>L: LogEntry
        end
    end

    D->>PM: Exit code
    PM->>A: Job complete
    A->>SCH: JobResult(exit_code, logs)
    A->>R: Unregister job

    SCH->>U: Job completed

    R->>R: Worker available for next job

    Note over U,M: Total flow: ~30 seconds typical
```

---

## Estados de Transición

```mermaid
stateDiagram-v2
    [*] --> Creating

    Creating --> Registering : Agent starts
    Registering --> Available : Registration success
    Registering --> Failed : Registration failed

    Available --> Running : Job assigned
    Running --> Available : Job complete
    Running --> Running : New job assigned
    Available --> Draining : Graceful shutdown

    Running --> Unhealthy : Connection lost
    Available --> Unhealthy : Heartbeat timeout
    Unhealthy --> Available : Reconnected
    Unhealthy --> Failed : Max failures reached

    Draining --> Terminated : All jobs done
    Draining --> Running : New urgent job

    Failed --> [*] : Fatal error
    Terminated --> [*] : Shutdown complete

    %% Notes
    note right of Available
        - Sends heartbeat every 5s
        - Accepts new jobs
        - Reports to scheduler
    end note

    note right of Unhealthy
        - No new jobs assigned
        - Retry connection
        - Marked for cleanup after timeout
    end note

    note left of Running
        - Executing 1-N jobs
        - Sending resource metrics
        - Streaming logs
    end note
```

---

## Componentes de Performance

```mermaid
graph LR
    subgraph "Lock-Free Structures"
        L1[SegQueue<br/>High Priority]
        L2[SegQueue<br/>Normal Priority]
        L3[SegQueue<br/>Low Priority]
    end

    subgraph "Caching Layer"
        C1[DashMap<br/>Worker Cache]
        C2[AtomicUsize<br/>Counter]
    end

    subgraph "Worker Selection"
        W1[CPU Fit Score]
        W2[Memory Fit Score]
        W3[Load Balance Score]
        W4[Health Score]
    end

    L1 -->|Batch 1/3| S[Scheduler Loop<br/>1 second]
    L2 -->|Batch 1/3| S
    L3 -->|Batch 1/3| S

    S --> C1
    C1 -->|Read| W1
    C1 -->|Read| W2
    C1 -->|Read| W3
    C1 -->|Read| W4

    W1 -->|Combine| B[Best Worker]
    W2 --> B
    W3 --> B
    W4 --> B

    B -->|Assign Job| L1

    classDef queue fill:#f3e5f5,stroke:#7b1fa2
    classDef cache fill:#e1f5fe,stroke:#0277bd
    classDef score fill:#fff3e0,stroke:#ef6c00
    classDef scheduler fill:#e8f5e9,stroke:#2e7d32

    class L1,L2,L3 queue
    class C1,C2 cache
    class W1,W2,W3,W4,B score
    class S scheduler
```

---

## Protocolo de Comunicación

```mermaid
graph TD
    subgraph "gRPC Services"
        S1[RegisterWorker]
        S2[AssignJob]
        S3[GetWorkerStatus]
        S4[Heartbeat]
        S5[JobStream]
    end

    subgraph "Messages Server -> Worker"
        M1[AssignJobRequest]
        M2[CancelJobRequest]
    end

    subgraph "Messages Worker -> Server"
        N1[JobAccepted]
        N2[LogEntry]
        N3[JobResult]
        N4[WorkerStatus]
        N5[Heartbeat]
    end

    S1 -.->|Response| N4
    S2 -.->|Response| N1
    S3 -.->|Response| N4
    S4 -.->|Response| Empty
    S5 <--> M1
    S5 <--> M2
    S5 <--> N1
    S5 <--> N2
    S5 <--> N3

    classDef service fill:#e1f5fe,stroke:#01579b
    classDef in fill:#e8f5e9,stroke:#2e7d32
    classDef out fill:#fff3e0,stroke:#e65100

    class S1,S2,S3,S4,S5 service
    class M1,M2 in
    class N1,N2,N3,N4,N5 out
```

---

## Resource Management

```mermaid
graph TB
    subgraph "Resource Quota"
        R1[CPU: 2000m<br/>2 Cores]
        R2[Memory: 4096MB<br/>4GB]
        R3[GPU: None]
    end

    subgraph "Worker Capabilities"
        C1[Total CPU: 8000m<br/>8 Cores]
        C2[Total Memory: 16384MB<br/>16GB]
        C3[Max Jobs: 4]
    end

    subgraph "Current Allocation"
        A1[Job 1: 2000m, 4GB]
        A2[Job 2: 2000m, 4GB]
        A3[Job 3: 2000m, 4GB]
        A4[Available: 2000m, 4GB]
    end

    subgraph "Utilization"
        U1[CPU: 6000m/8000m<br/>75%]
        U2[Memory: 12GB/16GB<br/>75%]
        U3[Jobs: 3/4<br/>75%]
    end

    R1 --> C1
    R2 --> C2
    R3 --> C3

    C1 --> A1
    C1 --> A2
    C1 --> A3
    C1 --> A4

    A1 --> U1
    A2 --> U1
    A3 --> U1
    A4 --> U1

    A1 --> U2
    A2 --> U2
    A3 --> U2
    A4 --> U2

    A1 --> U3
    A2 --> U3
    A3 --> U3
    A4 --> U3

    note right of U1
        Optimal range: 60-85%
        Current: 75% (Perfect!)
    end note

    classDef req fill:#e3f2fd,stroke:#1565c0
    classDef cap fill:#e8f5e9,stroke:#2e7d32
    classDef alloc fill:#fff3e0,stroke:#e65100
    classDef util fill:#fce4ec,stroke:#880e4f

    class R1,R2,R3 req
    class C1,C2,C3 cap
    class A1,A2,A3,A4 alloc
    class U1,U2,U3 util
```

---

## Error Handling & Recovery

```mermaid
flowchart TD
    subgraph "Failure Scenarios"
        F1[Network Partition]
        F2[Worker Crash]
        F3[Container Failure]
        F4[Server Restart]
    end

    subgraph "Detection"
        D1[Heartbeat Timeout]
        D2[Process Exit]
        D3[gRPC Error]
        D4[Health Check Fail]
    end

    subgraph "Recovery Actions"
        R1[Exponential Backoff<br/>Reconnect]
        R2[Mark Unhealthy<br/>Reschedule Jobs]
        R3[Restart Process<br/>Notify]
        R4[Full Reconnection<br/>State Sync]
    end

    subgraph "Success States"
        S1[Reconnected<br/>Resume Normal]
        S2[Jobs Rescheduled<br/>Worker Replaced]
        S3[Process Restarted<br/>Healthy]
        S4[State Synced<br/>Fully Recovered]
    end

    F1 --> D1
    F2 --> D2
    F3 --> D2
    F4 --> D3

    D1 --> R1
    D2 --> R2
    D3 --> R3
    D4 --> R4

    R1 --> S1
    R2 --> S2
    R3 --> S3
    R4 --> S4

    classDef failure fill:#ffebee,stroke:#c62828
    classDef detection fill:#fff3e0,stroke:#e65100
    classDef recovery fill:#e8f5e9,stroke:#2e7d32
    classDef success fill:#e1f5fe,stroke:#01579b

    class F1,F2,F3,F4 failure
    class D1,D2,D3,D4 detection
    class R1,R2,R3,R4 recovery
    class S1,S2,S3,S4 success
```

---

## Monitoring & Observability

```mermaid
graph TB
    subgraph "Metrics Collection"
        M1[Worker Heartbeats]
        M2[Job Execution Time]
        M3[Resource Usage]
        M4[Scheduler Performance]
        M5[Queue Size]
    end

    subgraph "Prometheus"
        P1[hodei_worker_heartbeats_total]
        P2[hodei_job_execution_duration]
        P3[hodei_worker_cpu_usage_percent]
        P4[hodei_scheduler_queue_size]
    end

    subgraph "Grafana Dashboards"
        G1[Worker Overview]
        G2[Job Performance]
        G3[Resource Utilization]
        G4[Scheduler Health]
    end

    subgraph "Logging"
        L1[Structured Logs]
        L2[Correlation IDs]
        L3[Request Tracing]
    end

    M1 --> P1
    M2 --> P2
    M3 --> P3
    M4 --> P4

    P1 --> G1
    P2 --> G2
    P3 --> G3
    P4 --> G4

    L1 -->|ELK Stack| G1
    L2 -->|Tracing| G2
    L3 -->|Distributed| G3

    classDef metrics fill:#f3e5f5,stroke:#7b1fa2
    classDef prom fill:#e1f5fe,stroke:#0277bd
    classDef graf fill:#e8f5e9,stroke:#2e7d32
    classDef logs fill:#fff3e0,stroke:#ef6c00

    class M1,M2,M3,M4,M5 metrics
    class P1,P2,P3,P4 prom
    class G1,G2,G3,G4 graf
    class L1,L2,L3 logs
```
