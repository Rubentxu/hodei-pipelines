# Hodei Pipelines - Architecture

This document describes the architecture of Hodei Pipelines using the C4 model.

## 1. System Context Diagram

The System Context diagram shows the software system in the context of its users and external systems.

```mermaid
C4Context
    title System Context Diagram for Hodei Pipelines

    Person(user, "User", "Developer or DevOps Engineer using the platform")
    System(hodei, "Hodei Pipelines", "Distributed Job Orchestration Platform")

    System_Ext(k8s, "Kubernetes Cluster", "Container Orchestration Platform")
    System_Ext(registry, "Container Registry", "Stores Docker images")
    System_Ext(git, "Git Repository", "Source code and pipeline definitions")

    Rel(user, hodei, "Uses", "HTTP/gRPC")
    Rel(hodei, k8s, "Orchestrates Pods", "K8s API")
    Rel(hodei, registry, "Pulls Images", "HTTPS")
    Rel(hodei, git, "Fetches Code", "HTTPS/SSH")
```

## 2. Container Diagram

The Container diagram shows the high-level software containers (applications, databases, etc.) and how they interact.

```mermaid
C4Container
    title Container Diagram for Hodei Pipelines

    Person(user, "User", "Developer or DevOps Engineer")

    System_Boundary(hodei, "Hodei Pipelines") {
        Container(server, "Hodei Server", "Rust, Axum, Tonic", "Core API and Orchestration Engine")
        Container(agent, "HWP Agent", "Rust", "Worker node agent that executes jobs")
        ContainerDb(db, "PostgreSQL", "PostgreSQL 15", "Stores pipelines, jobs, and execution history")
        Container(nats, "NATS JetStream", "NATS", "Event bus for async messaging and streams")
        Container(ui, "Web Console", "React/Next.js", "Web interface for management")
    }

    System_Ext(k8s, "Kubernetes API", "Cluster Management")

    Rel(user, ui, "Uses", "HTTPS")
    Rel(user, server, "Uses CLI/API", "HTTP/gRPC")
    Rel(ui, server, "API Calls", "HTTP/JSON")

    Rel(server, db, "Reads/Writes", "SQL/TCP")
    Rel(server, nats, "Publishes/Subscribes", "NATS Protocol")
    Rel(server, k8s, "Manages Resources", "HTTPS")

    Rel(agent, server, "Connects/Streams", "gRPC/HTTP2")
    Rel(agent, nats, "Subscribes (optional)", "NATS Protocol")
```

## 3. Component Diagram - Hodei Server

The Component diagram shows the internal structure of the Hodei Server.

```mermaid
C4Component
    title Component Diagram for Hodei Server

    Container(agent, "HWP Agent", "Worker Node")
    ContainerDb(db, "PostgreSQL", "Database")
    Container(nats, "NATS JetStream", "Event Bus")

    Container_Boundary(server, "Hodei Server") {
        Component(api, "API Layer", "Axum/Tonic", "Handles HTTP and gRPC requests")
        
        Component(orchestrator, "Orchestrator Module", "Module", "Manages job lifecycle and execution flow")
        Component(scheduler, "Scheduler Module", "Module", "Schedules jobs to workers")
        Component(worker_mgr, "Worker Management", "Module", "Manages worker registration and health")
        Component(pipeline_mgr, "Pipeline CRUD", "Module", "Manages pipeline definitions")
        Component(quota_mgr, "Quota Manager", "Module", "Enforces multi-tenancy quotas")
        Component(rbac, "RBAC Service", "Module", "Handles authentication and authorization")
        Component(observability, "Observability API", "Module", "Exposes metrics and logs")

        Component(repo_adapter, "Repository Adapter", "Adapter", "Persistence implementation")
        Component(msg_adapter, "Messaging Adapter", "Adapter", "NATS implementation")
    }

    Rel(api, orchestrator, "Uses")
    Rel(api, pipeline_mgr, "Uses")
    Rel(api, worker_mgr, "Uses")
    Rel(api, observability, "Uses")

    Rel(orchestrator, scheduler, "Triggers")
    Rel(scheduler, worker_mgr, "Selects Workers")
    Rel(orchestrator, quota_mgr, "Checks Quota")
    
    Rel(worker_mgr, agent, "Communicates", "gRPC")
    
    Rel(repo_adapter, db, "Persists Data")
    Rel(msg_adapter, nats, "Publishes Events")

    Rel(orchestrator, repo_adapter, "Uses")
    Rel(pipeline_mgr, repo_adapter, "Uses")
    Rel(orchestrator, msg_adapter, "Uses")
```

## 4. Component Diagram - HWP Agent

The Component diagram shows the internal structure of the HWP Agent.

```mermaid
C4Component
    title Component Diagram for HWP Agent

    Container(server, "Hodei Server", "Control Plane")

    Container_Boundary(agent, "HWP Agent") {
        Component(conn_mgr, "Connection Manager", "Core", "Manages connection and retries")
        Component(stream_handler, "Stream Handler", "Core", "Processes incoming job streams")
        Component(executor, "Job Executor", "Core", "Executes jobs (Shell/Docker)")
        Component(monitor, "Resource Monitor", "Core", "Monitors CPU/Memory usage")
    }

    Rel(conn_mgr, server, "Connects", "gRPC")
    Rel(conn_mgr, stream_handler, "Passes Stream")
    Rel(stream_handler, executor, "Triggers Execution")
    Rel(monitor, server, "Reports Metrics")
```
