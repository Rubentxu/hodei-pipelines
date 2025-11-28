# Hodei Pipelines - Sequence Diagrams

This document illustrates the main use cases of Hodei Pipelines using sequence diagrams.

## 1. Worker Registration

This flow describes how a new HWP Agent registers itself with the Hodei Server.

```mermaid
sequenceDiagram
    autonumber
    participant Agent as HWP Agent
    participant CM as Connection Manager
    participant API as Server API
    participant WM as Worker Management
    participant DB as Database

    Agent->>CM: Initialize Connection
    CM->>API: Connect Request with Metadata
    API->>WM: Register Worker
    WM->>DB: Check if Worker Exists
    DB-->>WM: Worker Status
    
    alt New Worker
        WM->>DB: Create Worker Record
        DB-->>WM: Record Created
    else Existing Worker
        WM->>DB: Update Worker Status
        DB-->>WM: Status Updated
    end

    WM-->>API: Registration Successful
    API-->>CM: Connection Established
    CM-->>Agent: Ready to Process Jobs
```

## 2. Pipeline Creation

This flow shows how a user creates a new pipeline definition.

```mermaid
sequenceDiagram
    autonumber
    participant User as User Developer
    participant API as Server API
    participant PM as Pipeline Manager
    participant DB as Database

    User->>API: POST /api/v1/pipelines
    API->>PM: Create Pipeline Request
    PM->>PM: Validate Pipeline Definition
    
    alt Validation Failed
        PM-->>API: Validation Error
        API-->>User: 400 Bad Request
    else Validation Passed
        PM->>DB: Insert Pipeline Data
        DB-->>PM: Pipeline ID
        PM-->>API: Pipeline Created
        API-->>User: 201 Created with ID
    end
```

## 3. Job Submission and Scheduling

This flow illustrates how a job is submitted and scheduled to a worker.

```mermaid
sequenceDiagram
    autonumber
    participant User as User
    participant API as Server API
    participant Orch as Orchestrator
    participant Sched as Scheduler
    participant QM as Quota Manager
    participant DB as Database
    participant NATS as NATS Bus

    User->>API: POST /api/v1/jobs
    API->>Orch: Submit Job Request
    Orch->>QM: Check Tenant Quota
    
    alt Quota Exceeded
        QM-->>Orch: Deny Request
        Orch-->>API: Quota Error
        API-->>User: 429 Too Many Requests
    else Quota OK
        Orch->>DB: Save Job Pending
        Orch->>Sched: Schedule Job
        Sched->>DB: Fetch Available Workers
        DB-->>Sched: List of Workers
        Sched->>Sched: Select Best Worker
        Sched->>DB: Assign Job to Worker
        Sched->>NATS: Publish Job Assigned Event
        Sched-->>Orch: Job Scheduled
        Orch-->>API: Job Accepted
        API-->>User: 202 Accepted
    end
```

## 4. Job Execution and Status Update

This flow details the execution of a job by a worker and the feedback loop.

```mermaid
sequenceDiagram
    autonumber
    participant Agent as HWP Agent
    participant Exec as Job Executor
    participant API as Server API
    participant Orch as Orchestrator
    participant DB as Database

    Note over Agent, API: Agent listens to stream or polls
    API->>Agent: Send Job Payload
    Agent->>Exec: Execute Command
    Exec->>Exec: Run Process
    
    loop Execution Stream
        Exec-->>Agent: Stdout Stderr Log
        Agent->>API: Stream Log Chunk
        API->>DB: Append Log
    end

    alt Execution Success
        Exec-->>Agent: Exit Code 0
        Agent->>API: Report Success
        API->>Orch: Update Job Status Completed
        Orch->>DB: Update Job Status
    else Execution Failure
        Exec-->>Agent: Exit Code NonZero
        Agent->>API: Report Failure
        API->>Orch: Update Job Status Failed
        Orch->>DB: Update Job Status
    end
```
