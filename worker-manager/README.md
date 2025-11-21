# Worker Manager Abstraction Layer

A comprehensive Rust-based worker management system providing abstractions for multiple infrastructure providers and credential management with automatic rotation.

## 🚀 Overview

The Worker Manager Abstraction Layer is a robust, production-ready system designed to simplify the orchestration of ephemeral workers across different infrastructure providers while providing a unified interface for credential management and automatic rotation.

### Key Features

- **Multi-Provider Support**: Kubernetes and Docker with extensible architecture for future providers
- **Credential Management**: Unified interface for multiple secret management systems
- **Automatic Rotation**: Time-based, event-based, and manual rotation strategies
- **Event-Driven Architecture**: Integration with NATS JetStream for real-time events
- **Security First**: Keycloak integration, Service Accounts, and robust access controls
- **Production Ready**: Comprehensive error handling, logging, metrics, and health checks
- **Graceful Operations**: Support for pause/resume, graceful shutdown, and zero-downtime rotations

## 📁 Project Structure

```
worker-manager/
├── traits/                           # Core interfaces and types
│   ├── worker_manager_provider.rs   # Main provider trait
│   ├── error_types.rs              # Comprehensive error handling
│   └── types.rs                    # Data types and configurations
├── providers/                       # Infrastructure provider implementations
│   ├── kubernetes/                 # Kubernetes integration (kube-rs)
│   ├── docker/                     # Docker integration (docker-rust)
│   └── plugin/                     # Plugin system for extensibility
├── credentials/                     # Credential management system
│   ├── mod.rs                      # Simple in-memory provider
│   ├── vault.rs                    # HashiCorp Vault integration
│   ├── aws_secrets.rs              # AWS Secrets Manager integration
│   ├── keycloak.rs                 # Keycloak Service Account provider
│   └── rotation.rs                 # Automatic rotation engine
├── ephemeral/                       # Ephemeral worker management
│   ├── auto_scaling.rs             # Auto-scaling strategies
│   ├── health_checks.rs            # Health monitoring
│   └── cost_optimization.rs        # Cost management
├── security/                        # Security implementations
│   ├── rbac.rs                     # Role-based access control
│   └── network_policies.rs         # Network isolation
├── examples/                        # Usage examples
│   ├── basic_usage.rs              # Basic usage patterns
│   └── advanced_usage.rs           # Advanced configurations
├── lib.rs                          # Main library interface
├── Cargo.toml                      # Dependencies and configuration
└── README.md                       # This file
```

## 🏗️ Architecture

The Worker Manager follows a layered architecture designed for extensibility and maintainability:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
├─────────────────────────────────────────────────────────────┤
│                   Worker Manager Core                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │ Provider    │ │ Credential  │ │ Event & Monitoring      │ │
│  │ Abstraction │ │ Management  │ │ System                  │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                Infrastructure Providers                     │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │ Kubernetes  │ │    Docker   │ │    Future Providers     │ │
│  │   (K8s)     │ │  (Docker)   │ │   (ECS, Cloud Run...)   │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    External Systems                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │ HashiCorp   │ │   AWS S3    │ │    NATS JetStream       │ │
│  │   Vault     │ │SecretsMgr   │ │      Event Bus          │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │  Keycloak   │ │  AWS IAM    │ │    Verified Permissions │ │
│  │     IDP     │ │             │ │        (AVP)            │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Core Components

### WorkerManagerProvider Trait

The foundation of the abstraction layer, defining the contract for all infrastructure providers:

```rust
#[async_trait]
pub trait WorkerManagerProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn create_worker(&self, spec: &RuntimeSpec, config: &ProviderConfig) -> Result<Worker, ProviderError>;
    async fn terminate_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;
    async fn get_worker_status(&self, worker_id: &WorkerId) -> Result<WorkerState, ProviderError>;
    async fn get_logs(&self, worker_id: &WorkerId) -> Result<LogStreamRef, ProviderError>;
    async fn port_forward(&self, worker_id: &WorkerId, local_port: u16, remote_port: u16) -> Result<String, ProviderError>;
    async fn get_capacity(&self) -> Result<CapacityInfo, ProviderError>;
    async fn execute_command(&self, worker_id: &WorkerId, command: Vec<String>, timeout: Option<Duration>) -> Result<ExecutionResult, ProviderError>;
    async fn restart_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;
    async fn pause_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;
    async fn resume_worker(&self, worker_id: &WorkerId) -> Result<(), ProviderError>;
    fn stream_worker_events(&self) -> tokio_stream::wrappers::IntervalStream;
}
```

### Provider Implementations

#### KubernetesProvider
- Full integration with Kubernetes API using kube-rs
- Support for pods, services, ConfigMaps, and Secrets
- Resource quotas, limits, and health checks
- ServiceAccount integration and RBAC
- Network policies and security contexts

#### DockerProvider
- Complete Docker Engine API integration
- Container lifecycle management
- Volume mounting and networking
- Health checks via Docker API
- Automatic cleanup and resource management

### Credential Management

Unified interface for credential storage and rotation across multiple systems:

#### Provider Types
- **SimpleCredentialProvider**: In-memory storage for development/testing
- **HashiCorpVaultProvider**: Full Vault integration with KV and Transit engines
- **AWSSecretsManagerProvider**: AWS Secrets Manager with version control
- **KeycloakServiceAccountProvider**: Service Account token management

#### Rotation Strategies
- **Time-Based**: Scheduled rotation with configurable intervals
- **Event-Based**: Reactive rotation based on security events
- **Manual**: On-demand rotation with approval workflows

## 🚀 Quick Start

### Basic Usage

```rust
use worker_manager::*;

// 1. Create credential provider
let credential_provider = Arc::new(credentials::SimpleCredentialProvider::new());

// 2. Create Worker Manager
let mut worker_manager = WorkerManager::new(credential_provider.clone());

// 3. Add Docker provider
let docker_provider = Arc::new(providers::docker::DockerProvider::from_provider_config(
    &ProviderConfig::docker(),
    credential_provider.clone()
)?);
worker_manager.register_provider("docker".to_string(), docker_provider);
worker_manager.set_default_provider("docker".to_string());

// 4. Create worker specification
let mut runtime_spec = RuntimeSpec::basic("alpine:3.18".to_string());
runtime_spec.command = Some(vec!["/bin/sh".to_string(), "-lc".to_string(), "echo 'Hello!'".to_string()]);

// 5. Create provider configuration
let provider_config = ProviderConfig::docker();

// 6. Create worker
let worker = worker_manager.create_worker(&runtime_spec, &provider_config).await?;
println!("Worker created: {}", worker.id);
```

### Advanced Configuration

```rust
use worker_manager::credentials::{HashiCorpVaultProvider, VaultConfig, VaultAuthMethod};

// Configure Vault provider
let vault_config = VaultConfig {
    vault_url: "https://vault.example.com".to_string(),
    auth_method: VaultAuthMethod::Token { token: "your-token".to_string() },
    timeout: Duration::from_secs(30),
    max_retries: 3,
    secrets_engine: "secret".to_string(),
    transit_engine: None,
    kv_version: KVVersion::V2,
};

let vault_provider = Arc::new(HashiCorpVaultProvider::new(vault_config).await?);

// Configure rotation engine
let rotation_config = RotationEngineConfig {
    max_concurrent_rotations: 10,
    default_retry_count: 3,
    rotation_timeout: Duration::from_secs(120),
    audit_retention_days: 90,
    enable_metrics: true,
    health_check_interval: Duration::from_secs(60),
};

worker_manager.start_rotation_engine(rotation_config).await?;

// Schedule credential rotation
let rotation_strategy = RotationStrategy::TimeBased(TimeBasedRotation {
    rotation_interval: Duration::from_secs(3600), // 1 hour
    rotation_time: None,
    grace_period: Duration::from_secs(60),
});

let rotation_task_id = worker_manager.schedule_rotation("database-credentials", rotation_strategy).await?;
```

## 🔐 Security Features

### Authentication & Authorization
- **Keycloak Integration**: Service Account authentication and token management
- **AWS Verified Permissions**: Policy-based authorization for operations
- **Role-Based Access Control**: Fine-grained permissions for credentials and operations
- **Multi-tenant Support**: Complete isolation between different tenants and environments

### Secret Management
- **Automatic Rotation**: Zero-downtime credential rotation with dual-release
- **Encryption at Rest**: All secrets encrypted using provider-native encryption
- **Audit Trail**: Comprehensive logging of all credential operations
- **Access Policies**: Granular control over who can read, write, or rotate secrets

### Network Security
- **Network Policies**: Kubernetes NetworkPolicies for traffic isolation
- **Service Mesh Integration**: Support for Istio, Linkerd, and other service meshes
- **Zero Trust**: Default deny policies with explicit allow rules
- **TLS Everywhere**: All communications encrypted with mTLS where possible

## 📊 Monitoring & Observability

### Metrics & Logging
- **Structured Logs**: JSON-formatted logs with correlation IDs
- **Health Checks**: Comprehensive health monitoring for all components
- **Performance Metrics**: Latency, throughput, and error rate monitoring
- **Custom Metrics**: Configurable metrics for business logic

### Event System
- **NATS JetStream Integration**: Real-time event streaming
- **Webhook Support**: Integration with external notification systems
- **Audit Events**: Complete audit trail for compliance and security
- **Alerting**: Configurable alerts for critical events

### Dashboards & Visualization
- **Worker Lifecycle**: Real-time worker status and health
- **Capacity Planning**: Resource utilization and forecasting
- **Security Dashboard**: Credential access patterns and security events
- **Cost Analysis**: Resource usage and cost optimization

## 🧪 Testing & Development

### Test Coverage
- **Unit Tests**: Comprehensive test coverage for all components
- **Integration Tests**: End-to-end testing with real infrastructure
- **Mock Infrastructure**: Mock providers for development and CI/CD
- **Performance Tests**: Benchmarking for performance regression detection

### Development Tools
- **Example Applications**: Complete examples for different use cases
- **Development Mode**: Easy development with local Docker and simple providers
- **Debugging Tools**: Enhanced logging and debug capabilities
- **Code Quality**: Clippy, rustfmt, and other code quality tools

## 🔌 Integration Points

### Existing Architecture
- **NATS JetStream**: Event streaming for real-time updates
- **Keycloak**: Identity and access management
- **AWS Services**: S3, Secrets Manager, EC2, and more
- **CI/CD Pipelines**: Integration with Jenkins, GitLab CI, GitHub Actions

### Extensibility
- **Plugin System**: Easy addition of new infrastructure providers
- **Provider Registry**: Dynamic provider discovery and loading
- **Event Handlers**: Custom event processing and handling
- **Custom Strategies**: Extensible rotation and auto-scaling strategies

## 📈 Performance & Scalability

### Performance Characteristics
- **Low Latency**: Sub-100ms operation times for common operations
- **High Throughput**: Support for thousands of concurrent workers
- **Efficient Resource Usage**: Minimal overhead per worker
- **Auto-scaling**: Dynamic scaling based on demand and capacity

### Scalability Features
- **Horizontal Scaling**: Support for multiple manager instances
- **Load Balancing**: Intelligent worker distribution across nodes
- **Resource Pooling**: Efficient reuse of resources and infrastructure
- **Queue Management**: Backpressure handling and queue management

## 🛠️ Installation & Configuration

### Prerequisites
- Rust 1.70 or later
- Docker (for Docker provider)
- Kubernetes cluster (for K8s provider)
- Access to external services (Vault, AWS, Keycloak, etc.)

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/worker-manager.git
cd worker-manager

# Build the project
cargo build --all-features

# Run basic example
cargo run --example basic-usage

# Run advanced example
cargo run --example advanced-usage
```

### Configuration

Configuration can be provided through:
- Environment variables
- Configuration files (YAML, JSON, TOML)
- Command-line arguments
- Service discovery (Consul, etcd)

## 📖 Documentation

### API Documentation
- **Generated API docs**: Available at `cargo doc --open`
- **Examples**: Comprehensive examples in the `examples/` directory
- **Architecture guides**: Detailed architecture documentation
- **Integration guides**: Step-by-step integration instructions

### Best Practices
- **Production Deployment**: Guide for production deployments
- **Security Hardening**: Security configuration and best practices
- **Performance Tuning**: Optimization guidelines for production
- **Troubleshooting**: Common issues and solutions

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run the test suite
6. Submit a pull request

### Code Standards
- Follow Rust formatting guidelines (`cargo fmt`)
- Pass clippy linting (`cargo clippy`)
- Write comprehensive tests
- Document new features
- Follow semantic versioning

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- The Rust community for excellent tooling and libraries
- Kubernetes and Docker communities for robust container orchestration
- HashiCorp for excellent infrastructure tooling
- AWS for cloud-native integrations
- All contributors who have helped make this project better

## 📞 Support

- **Documentation**: Check the docs and examples first
- **Issues**: Report bugs and feature requests on GitHub
- **Discussions**: Join the community discussions
- **Commercial Support**: Contact us for commercial support and consulting

---

**Built with ❤️ by the Worker Manager Team**
