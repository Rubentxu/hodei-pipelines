![Hodei Pipelines](docs/assets/header.png)

<div align="center">

# Hodei Pipelines

**High-Performance Distributed Job Orchestration Platform**

[![Build Status](https://img.shields.io/github/actions/workflow/status/Rubentxu/hodei-jobs/ci.yml?branch=main)](https://github.com/Rubentxu/hodei-jobs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://www.docker.com/)

[Documentation](docs/) | [Architecture](docs/architecture.md) | [Contributing](CONTRIBUTING.md) | [Español](README.es.md)

</div>

---

## 🚀 Overview

**Hodei Pipelines** is a next-generation, distributed job orchestration platform built entirely in **Rust**. It is designed to deliver extreme performance, low latency, and rock-solid reliability for complex CI/CD pipelines, data processing workflows, and automated tasks.

Unlike traditional CI/CD systems that can be heavy and resource-intensive, Hodei Pipelines leverages the efficiency of Rust and the power of **NATS JetStream** to handle thousands of concurrent jobs with minimal overhead.

## ✨ Key Features

- **⚡ Blazing Fast Performance**: Built with Rust for near-zero runtime overhead and efficient resource usage.
- **🌐 Distributed Architecture**: Decoupled **Server** and **Agent** architecture allows you to scale workers horizontally across any infrastructure (Kubernetes, VMs, Bare Metal).
- **🔒 Enterprise-Grade Security**:
    - **mTLS** encryption for all Agent-Server communication.
    - **RBAC** (Role-Based Access Control) for granular permission management.
    - **Secret Masking** to protect sensitive data in logs.
- **📡 Real-Time Event Bus**: Powered by **NATS JetStream** for reliable, asynchronous message passing and stream processing.
- **📊 Deep Observability**: Native integration with **OpenTelemetry** and **Prometheus** for comprehensive metrics, traces, and logs.
- **🏢 Multi-Tenancy**: Built-in support for multiple tenants with strict quota enforcement and resource isolation.
- **🐳 Container Native**: First-class support for Docker and Kubernetes execution environments.

## 🏗️ Architecture

Hodei Pipelines follows a modern, modular architecture:

- **Hodei Server**: The control plane that manages API requests, orchestration logic, scheduling, and state persistence (PostgreSQL).
- **HWP Agent**: Lightweight worker agents that connect securely to the server and execute assigned jobs.
- **NATS JetStream**: The nervous system ensuring reliable communication between components.

👉 **[Explore the Architecture (C4 Model)](docs/architecture.md)** - Detailed Context, Container, and Component diagrams.
👉 **[View Sequence Diagrams (Use Cases)](docs/sequence_diagrams.md)** - Visual flows for Worker Registration, Job Submission, and more.

## 🛠️ Quick Start

### Prerequisites

- Rust 1.75+
- Docker & Docker Compose
- Kubernetes (optional, for full E2E testing)

### Local Development Setup

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/Rubentxu/hodei-jobs.git
    cd hodei-jobs
    ```

2.  **Start infrastructure (DB, NATS):**
    ```bash
    docker-compose up -d postgres nats
    ```

3.  **Run the Server:**
    ```bash
    cargo run --bin hodei-server
    ```

4.  **Run an Agent:**
    ```bash
    cargo run --bin hwp-agent
    ```

For detailed testing instructions, including E2E tests with Testkube, see **[TESTING.md](TESTING.md)**.

## 📦 Deployment

Hodei Pipelines is cloud-ready. We provide **Helm Charts** for easy deployment to Kubernetes.

```bash
make deploy
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on how to submit pull requests, report issues, and setup your development environment.

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

<div align="center">
  <sub>Built with ❤️ by the Hodei Team</sub>
</div>
