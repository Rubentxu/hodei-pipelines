# Épica 6: Developer Experience & Tools

**Planificación de Sprints - Sistema CI/CD Distribuido**  
**Bounded Context**: Developer Experience  
**Autor**: MiniMax Agent  
**Fecha**: 2025-11-21  
**Versión**: 1.0  

## 📋 Índice
1. [Visión de la Épica](#visión-de-la-épica)
2. [Arquitectura de Developer Tools](#arquitectura-de-developer-tools)
3. [Patrones Conascense para Developer Experience](#patrones-conascense-para-developer-experience)
4. [Historias de Usuario](#historias-de-usuario)
5. [Planificación de Sprints](#planificación-de-sprints)
6. [SDK Development Framework](#sdk-development-framework)
7. [Self-Service Developer Portal](#self-service-developer-portal)
8. [Referencias Técnicas](#referencias-técnicas)

---

## 🚀 Visión de la Épica

### Objetivo Principal
Crear un ecosistema completo de herramientas para desarrolladores que incluya SDKs multi-lenguaje, CLI avanzada, IDE plugins, self-service portal, code generation tools, y comprehensive documentation para acelerar el desarrollo y deployment en el sistema CI/CD distribuido.

### Developer Experience Stack
- **Multi-Language SDKs**: Rust, Python, JavaScript/TypeScript, Go
- **Advanced CLI Tools**: Interactive deployment, monitoring, debugging
- **IDE Integrations**: VS Code, IntelliJ IDEA, Visual Studio plugins
- **Self-Service Portal**: Developer dashboard, resource management
- **Code Generation**: Template engine, scaffolding tools
- **Documentation System**: Interactive docs, API explorer, tutorials

### Métricas de Developer Experience
- **SDK Coverage**: 95% feature parity across all languages
- **CLI Performance**: < 2s command execution, < 5s complex operations
- **IDE Integration**: Real-time sync, < 1s code completion
- **Portal Response**: < 3s page load, < 5s dashboard updates
- **Developer Onboarding**: < 30min to first deployment
- **Documentation**: 99% API coverage, < 1s search results

---

## 🏗️ Arquitectura de Developer Tools

### Estructura de Crates (Bounded Context: Developer Experience)

```
crates/developer-experience/
├── sdk-core/                        # Core SDK Framework
│   ├── src/
│   │   ├── common/                  # Shared SDK components
│   │   │   ├── client.rs            # HTTP client abstraction
│   │   │   ├── auth.rs              # Authentication handling
│   │   │   ├── config.rs            # Configuration management
│   │   │   ├── error.rs             # Error handling & types
│   │   │   └── types.rs             # Shared data types
│   │   ├── pipeline/                # Pipeline management
│   │   │   ├── pipeline_client.rs   # Pipeline API client
│   │   │   ├── job_operations.rs    # Job lifecycle operations
│   │   │   ├── deployment.rs        # Deployment operations
│   │   │   └── monitoring.rs        # Monitoring integration
│   │   ├── worker/                  # Worker management
│   │   │   ├── worker_client.rs     # Worker API client
│   │   │   ├── provider_ops.rs      # Provider operations
│   │   │   ├── scaling.rs           # Auto-scaling operations
│   │   │   └── health.rs            # Health check integration
│   │   └── lib.rs
│   ├── tests/
│   │   ├── client/client_tests.rs
│   │   ├── pipeline/pipeline_tests.rs
│   │   └── integration/sdk_integration_tests.rs
│   └── examples/
│       ├── basic_pipeline.rs
│       ├── worker_management.rs
│       └── monitoring_integration.rs
│
├── rust-sdk/                        # Rust SDK Implementation
│   ├── src/
│   │   ├── lib.rs                   # Main Rust SDK
│   │   ├── client.rs                # Rust-specific client
│   │   ├── async_runtime.rs         # Async runtime integration
│   │   ├── futures_integration.rs   # Futures/TStreams support
│   │   └── macros/                  # Rust macros
│   │       ├── pipeline_macro.rs    # Pipeline DSL macros
│   │       ├── worker_macro.rs      # Worker abstraction macros
│   │       └── monitoring_macro.rs  # Monitoring macros
│   ├── tests/
│   │   ├── unit/rust_client_tests.rs
│   │   ├── macro/macro_tests.rs
│   │   └── integration/async_integration_tests.rs
│   └── examples/
│       ├── async_pipeline.rs
│         ├── worker_scaling.rs
│         └── custom_provider.rs
│
├── python-sdk/                      # Python SDK Implementation
│   ├── src/
│   │   ├── __init__.py              # Main Python package
│   │   ├── client.py                # Python client wrapper
│   │   ├── pipeline.py              # Pipeline management
│   │   ├── worker.py                # Worker operations
│   │   ├── decorators.py            # Python decorators
│   │   └── typing.py                # Type hints
│   ├── tests/
│   │   ├── test_client.py
│   │   ├── test_pipeline.py
│   │   └── test_integration.py
│   └── examples/
│       ├── basic_usage.py
│       ├── advanced_pipeline.py
│       └── monitoring_integration.py
│
├── js-sdk/                          # JavaScript/TypeScript SDK
│   ├── src/
│   │   ├── index.ts                 # Main JS SDK
│   │   ├── client.ts                # JavaScript client
│   │   ├── types.ts                 # TypeScript definitions
│   │   ├── utils.ts                 # Utility functions
│   │   └── browser.ts               # Browser-specific utilities
│   ├── tests/
│   │   ├── client.test.ts
│   │   ├── pipeline.test.ts
│   │   └── integration.test.ts
│   └── examples/
│       ├── basic_usage.ts
│         ├── node_integration.ts
│         └── browser_usage.ts
│
├── cli-tools/                       # Advanced CLI Tools
│   ├── src/
│   │   ├── commands/                # CLI command implementations
│   │   │   ├── pipeline_cmd.rs      # Pipeline management commands
│   │   │   ├── worker_cmd.rs        # Worker management commands
│   │   │   ├── deploy_cmd.rs        # Deployment commands
│   │   │   ├── monitor_cmd.rs       # Monitoring commands
│   │   │   ├── debug_cmd.rs         # Debugging commands
│   │   │   ├── config_cmd.rs        # Configuration commands
│   │   │   └── auth_cmd.rs          # Authentication commands
│   │   ├── interactive/             # Interactive mode
│   │   │   ├── shell.rs             # Interactive shell
│   │   │   ├── completion.rs        # Command completion
│   │   │   ├── progress.rs          # Progress tracking
│   │   │   └── visualizer.rs        # Data visualization
│   │   ├── utils/                   # CLI utilities
│   │   │   ├── printer.rs           # Output formatting
│   │   │   ├── validator.rs         # Input validation
│   │   │   └── template.rs          # Template rendering
│   │   └── lib.rs
│   ├── tests/
│   │   ├── commands/command_tests.rs
│   │   ├── interactive/shell_tests.rs
│   │   └── integration/cli_integration_tests.rs
│   └── examples/
│       ├── interactive_demo.rs
│       ├── batch_operations.rs
│       └── custom_commands.rs
│
├── ide-plugins/                     # IDE Integration Plugins
│   ├── vscode/
│   │   ├── src/
│   │   │   ├── extension.ts         # VS Code extension
│   │   │   ├── client.ts            # Language client
│   │   │   ├── language-server.ts   # Language server protocol
│   │   │   ├── tree-view.ts         # Pipeline explorer
│   │   │   └── commands.ts          # VS Code commands
│   │   ├── tests/
│   │   │   ├── extension.test.ts
│   │   │   └── integration.test.ts
│   │   └── package.json
│   │
│   ├── intellij/
│   │   ├── src/
│   │   │   ├── CicdPlugin.kt        # IntelliJ plugin main
│   │   │   ├── CicdToolWindow.kt    # Tool window
│   │   │   ├── CicdService.kt       # Plugin service
│   │   │   └── CicdConfiguration.kt # Configuration
│   │   ├── tests/
│   │   │   ├── CicdPluginTest.kt
│   │   │   └── IntegrationTest.kt
│   │   └── build.gradle
│   │
│   └── visual-studio/
│       ├── src/
│       │   ├── CicdExtension.cs     # VS extension
│       │   ├── CicdToolWindow.cs    # Tool window
│       │   └── CicdService.cs       # Extension service
│       ├── tests/
│       │   └── CicdExtensionTest.cs
│       └── CicdExtension.vsix
│
├── developer-portal/                # Self-Service Developer Portal
│   ├── src/
│   │   ├── frontend/                # React frontend
│   │   │   ├── components/
│   │   │   │   ├── Dashboard.tsx    # Developer dashboard
│   │   │   │   ├── PipelineView.tsx # Pipeline management UI
│   │   │   │   ├── WorkerView.tsx   # Worker management UI
│   │   │   │   ├── MetricsView.tsx  # Metrics visualization
│   │   │   │   ├── SettingsView.tsx # User settings
│   │   │   │   └── Documentation.tsx # Interactive docs
│   │   │   ├── services/            # Frontend services
│   │   │   │   ├── ApiClient.ts     # API client
│   │   │   │   ├── AuthService.ts   # Authentication
│   │   │   │   └── WebSocketService.ts # Real-time updates
│   │   │   └── utils/               # Frontend utilities
│   │   ├── backend/                 # Portal backend
│   │   │   ├── api/                 # REST API endpoints
│   │   │   │   ├── pipelines.rs     # Pipeline management API
│   │   │   │   ├── workers.rs       # Worker management API
│   │   │   │   ├── metrics.rs       # Metrics API
│   │   │   │   └── auth.rs          # Authentication API
│   │   │   ├── websocket/           # WebSocket handlers
│   │   │   │   ├── handlers.rs      # WebSocket message handlers
│   │   │   │   └── broadcaster.rs   # Real-time broadcasting
│   │   │   └── middleware/          # API middleware
│   │   │       ├── auth.rs          # Authentication middleware
│   │   │       ├── rate_limit.rs    # Rate limiting
│   │   │       └── logging.rs       # Request logging
│   │   └── shared/                  # Shared types
│   │       ├── types.ts             # Shared TypeScript types
│   │       ├── validators.ts        # Input validators
│   │       └── constants.ts         # Shared constants
│   └── tests/
│       ├── frontend/component_tests.tsx
│       ├── backend/api_tests.rs
│       └── integration/portal_integration_tests.tsx
│
├── code-generation/                 # Code Generation Tools
│   ├── src/
│   │   ├── templates/               # Code generation templates
│   │   │   ├── pipeline_templates.rs # Pipeline templates
│   │   │   ├── worker_templates.rs   # Worker templates
│   │   │   ├── test_templates.rs     # Test templates
│   │   │   └── config_templates.rs   # Configuration templates
│   │   ├── generator/               # Code generator engine
│   │   │   ├── template_engine.rs    # Template processing
│   │   │   ├── code_renderer.rs      # Code generation
│   │   │   ├── dependency_resolver.rs # Dependency management
│   │   │   └── file_writer.rs       # File generation
│   │   ├── cli/                     # Generator CLI
│   │   │   ├── commands.rs          # Generator commands
│   │   │   ├── interactive.rs       # Interactive generation
│   │   │   └── config.rs            # Generation configuration
│   │   └── lib.rs
│   ├── tests/
│   │   ├── template/template_tests.rs
│   │   ├── generator/generation_tests.rs
│   │   └── integration/CLI_integration_tests.rs
│   └── examples/
│       ├── generate_pipeline.rs
│       ├── generate_worker.rs
│         └── custom_template.rs
│
├── documentation/                   # Documentation System
│   ├── src/
│   │   ├── generator/               # Doc generation
│   │   │   ├── api_docs.rs          # API documentation generator
│   │   │   ├── guide_generator.rs   # User guide generator
│   │   │   ├── example_generator.rs # Example documentation
│   │   │   └── reference_generator.rs # Reference documentation
│   │   ├── search/                  # Documentation search
│   │   │   ├── indexer.rs           # Search indexer
│   │   │   ├── search_engine.rs     # Full-text search
│   │   │   └── autocomplete.rs      # Search autocomplete
│   │   ├── interactive/             # Interactive documentation
│   │   │   ├── api_explorer.rs      # API explorer
│   │   │   ├── live_examples.rs     # Live code examples
│   │   │   └── tutorial_runner.rs   # Interactive tutorials
│   │   └── lib.rs
│   ├── tests/
│   │   ├── generator/doc_gen_tests.rs
│   │   ├── search/search_tests.rs
│   │   └── interactive/interactive_tests.rs
│   └── examples/
│       ├── generate_api_docs.rs
│       ├── interactive_tutorial.rs
│         └── search_demo.rs
│
└── testing-tools/                   # Testing Framework Integration
    ├── src/
    │   ├── framework/               # Testing framework
    │   │   ├── test_runner.rs       # Test execution engine
    │   │   ├── parallel_runner.rs   # Parallel test execution
    │   │   ├── coverage.rs          # Coverage tracking
    │   │   └── reporter.rs          # Test reporting
    │   ├── integration/             # Framework integrations
    │   │   ├── junit.rs             # JUnit integration
    │   │   ├── pytest.rs            # Pytest integration
    │   │   ├── jest.rs              # Jest integration
    │   │   └── cargo_test.rs        # Cargo test integration
    │   ├── fixtures/                # Test fixtures
    │   │   ├── pipeline_fixtures.rs # Pipeline test data
    │   │   ├── worker_fixtures.rs   # Worker test data
    │   │   └── config_fixtures.rs   # Configuration test data
    │   └── lib.rs
    ├── tests/
    │   ├── framework/framework_tests.rs
    │   ├── integration/integration_tests.rs
    │   └── fixtures/fixture_tests.rs
    └── examples/
        ├── basic_tests.rs
        ├── parallel_execution.rs
        └── coverage_example.rs
```

### Dependencias Centralizadas (Cargo.toml raíz)

```toml
[workspace.dependencies]
# SDK Core
reqwest = { version = "0.11", features = ["json", "stream"] }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"

# CLI Tools
clap = { version = "4.0", features = ["derive", "cargo"] }
crossterm = "0.27"
tui = "0.22"
indicatif = "0.17"

# Code Generation
templating = "0.15"
quote = "1.0"
syn = { version = "2.0", features = ["full"] }
proc-macro2 = "1.0"

# Documentation
markdown = "0.3"
search-index = "0.8"
fuse = "0.4"

# IDE Integration
language-server-protocol = "0.15"
serde_json = "1.0"

# Testing
tempfile = "3.0"
fake = "2.5"
proptest = "1.0"

# Portal
axum = "0.6"
tower = "0.4"
leptos = { version = "0.4", features = ["hydrate"] }
```

---

## 🔗 Patrones Conascense para Developer Experience

### CoC (Coupling of Coincidence) - Developer Tools

| Módulo | Dependencias Ocasionales | Patrón de Desacoplamiento |
|--------|--------------------------|---------------------------|
| SDKs ↔ CLI | Configuration sharing | **Shared configuration schema** |
| IDE Plugins ↔ SDKs | API compatibility | **API versioning abstraction** |
| Portal ↔ All | User context propagation | **JWT token propagation** |
| Code Generation ↔ SDKs | Template processing | **Template engine isolation** |

### CoP (Coupling of Position) - Developer Tools

| Módulo | Posición Crítica | Tiempo de Respuesta |
|--------|------------------|-------------------|
| SDK Client | API communication | < 1s |
| CLI Interactive | Command execution | < 2s |
| IDE Plugin | Code completion | < 1s |
| Developer Portal | Dashboard loading | < 3s |

### CoI (Coupling of Identity) - Developer Tools

| Módulo | Identidad Compartida | Estrategia de Identidad |
|--------|---------------------|------------------------|
| SDKs + CLI | Developer API token | **JWT-based identity propagation** |
| IDE + Portal | User session ID | **Session management abstraction** |
| Code Gen + SDKs | Project context | **Project-scoped configuration** |
| All modules | Service account | **Service-to-service authentication** |

### CoID (Coupling of Identity and Data) - Developer Tools

| Data Flow | Identidad | Datos | Estrategia de Desacoplamiento |
|-----------|-----------|--------|------------------------------|
| SDK → CLI | API Token | Configuration | **Configuration service layer** |
| IDE → Portal | User Session | Project Data | **Project data abstraction** |
| Code Gen → SDKs | Project ID | Generated Code | **Code generation pipeline** |
| Portal → SDKs | User Context | Runtime Data | **Context propagation middleware** |

---

## 📝 Historias de Usuario

### 🏗️ Sprint 1: Multi-Language SDK Development

#### Historia 1.1: Rust SDK Implementation
**Como** Rust Developer  
**Quiero** comprehensive Rust SDK  
**Para** integrate con el sistema CI/CD desde aplicaciones Rust  

**INVEST**:
- **Independent**: SDK implementation standalone
- **Negotiable**: Feature-complete Rust integration
- **Valuable**: Native Rust developer experience
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core SDK features
- **Testable**: SDK integration tests

**Acceptance Criteria**:
- [ ] Complete pipeline management API (create, execute, monitor)
- [ ] Worker management integration (provision, scale, monitor)
- [ ] Async/await support con tokio runtime
- [ ] Comprehensive error handling con Result types
- [ ] Type-safe API con serde_json serialization
- [ ] Documentation con examples y integration guides

**Rust SDK Architecture (Hexagonal)**:
```
Port (Interface): CicdClient trait
├── create_pipeline() -> Result<Pipeline, Error>
├── execute_pipeline() -> Result<Job, Error>
├── monitor_job() -> Result<JobStatus, Error>
├── create_worker() -> Result<Worker, Error>
└── scale_workers() -> Result<ScalingResult, Error>

Adapter (Implementation): RustCicdClient
├── reqwest HTTP client
├── tokio async runtime
├── serde JSON serialization
└── Error handling con custom Error types
```

**TDD Implementation**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use serde_json::json;

    #[tokio::test]
    async fn test_pipeline_creation() {
        let client = RustCicdClient::new("http://localhost:8080", "test-token");
        let pipeline_config = PipelineConfig {
            name: "test-pipeline".to_string(),
            stages: vec![
                Stage {
                    name: "build".to_string(),
                    image: "rust:1.70".to_string(),
                    commands: vec!["cargo build".to_string()],
                }
            ],
        };
        
        let pipeline = client.create_pipeline(pipeline_config).await.unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
        assert!(pipeline.id.len() > 0);
    }

    #[test]
    fn test_error_handling() {
        let client = RustCicdClient::new("http://invalid-url", "test-token");
        let result = client.create_pipeline(PipelineConfig::default()).await;
        
        assert!(matches!(result, Err(Error::ConnectionFailed)));
    }
}
```

**Conventional Commit**: `feat(rust-sdk): implement comprehensive Rust SDK with async/await support, type-safe API, and tokio integration`

---

#### Historia 1.2: Python SDK Implementation
**Como** Python Developer  
**Quiero** idiomatic Python SDK  
**Para** integrate con el sistema desde scripts y applications Python  

**INVEST**:
- **Independent**: Python SDK standalone implementation
- **Negotiable**: PEP-8 compliant Python integration
- **Valuable**: Python ecosystem compatibility
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core Python features
- **Testable**: Python package tests

**Acceptance Criteria**:
- [ ] Pythonic API con context managers y decorators
- [ ] Async support con asyncio y aiohttp
- [ ] Type hints integration (typing module)
- [ ] Comprehensive error handling con custom exceptions
- [ ] Package distribution via PyPI
- [ ] Documentation con sphinx y examples

**Python SDK Implementation**:
```python
from typing import Optional, Dict, Any, List
from contextlib import asynccontextmanager
import asyncio
import aiohttp

class CicdClient:
    """Python SDK client para CI/CD system"""
    
    def __init__(self, base_url: str, api_token: str):
        self.base_url = base_url
        self.api_token = api_token
        self._session: Optional[aiohttp.ClientSession] = None
    
    async def __aenter__(self) -> 'CicdClient':
        self._session = aiohttp.ClientSession()
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self._session:
            await self._session.close()
    
    async def create_pipeline(self, config: Dict[str, Any]) -> Dict[str, Any]:
        """Create a new pipeline"""
        headers = {
            'Authorization': f'Bearer {self.api_token}',
            'Content-Type': 'application/json'
        }
        
        async with self._session.post(
            f"{self.base_url}/api/v1/pipelines",
            json=config,
            headers=headers
        ) as response:
            if response.status == 201:
                return await response.json()
            else:
                raise CicdError(f"Failed to create pipeline: {response.status}")

# Usage con context manager
async def main():
    async with CicdClient("http://localhost:8080", "token") as client:
        pipeline = await client.create_pipeline({
            "name": "test-pipeline",
            "stages": [{"name": "build", "image": "python:3.11"}]
        })
        print(f"Created pipeline: {pipeline['id']}")

# Usage con decorator
@pipeline_decorator(name="data-processing", image="python:3.11")
async def process_data():
    # Pipeline logic here
    pass
```

**TDD Implementation**:
```python
import pytest
import asyncio
from unittest.mock import AsyncMock, patch

@pytest.mark.asyncio
async def test_pipeline_creation():
    client = CicdClient("http://localhost:8080", "test-token")
    
    with patch('aiohttp.ClientSession.post') as mock_post:
        mock_response = AsyncMock()
        mock_response.status = 201
        mock_response.json = AsyncMock(return_value={
            "id": "pipeline-123",
            "name": "test-pipeline"
        })
        mock_post.return_value.__aenter__.return_value = mock_response
        
        pipeline = await client.create_pipeline({
            "name": "test-pipeline",
            "stages": []
        })
        
        assert pipeline["id"] == "pipeline-123"
        assert pipeline["name"] == "test-pipeline"

@pytest.mark.asyncio
async def test_context_manager():
    async with CicdClient("http://localhost:8080", "token") as client:
        assert client._session is not None
    
    assert client._session is None
```

**Conventional Commit**: `feat(python-sdk): implement idiomatic Python SDK with async support, context managers, and PyPI distribution`

---

#### Historia 1.3: JavaScript/TypeScript SDK
**Como** JavaScript Developer  
**Quiero** TypeScript SDK para Node.js y browser  
**Para** integrate con el sistema desde aplicaciones web y Node.js  

**INVEST**:
- **Independent**: JS/TS SDK standalone implementation
- **Negotiable**: ES6+ compliant con TypeScript definitions
- **Valuable**: Modern JavaScript ecosystem integration
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core JS features
- **Testable**: Jest unit tests + integration tests

**Acceptance Criteria**:
- [ ] TypeScript definitions para full type safety
- [ ] Node.js y browser compatibility
- [ ] Promise-based API con async/await
- [ ] Fetch API integration
- [ ] NPM package distribution
- [ ] Tree-shaking support para bundle optimization

**JavaScript SDK Implementation**:
```typescript
// types.ts - TypeScript definitions
export interface PipelineConfig {
  name: string;
  stages: StageConfig[];
  environment?: Record<string, string>;
}

export interface StageConfig {
  name: string;
  image: string;
  commands: string[];
  dependencies?: string[];
}

export interface Pipeline {
  id: string;
  name: string;
  status: PipelineStatus;
  createdAt: Date;
}

export enum PipelineStatus {
  PENDING = 'PENDING',
  RUNNING = 'RUNNING',
  SUCCESS = 'SUCCESS',
  FAILED = 'FAILED'
}

// client.ts - Main client implementation
export class CicdClient {
  private baseUrl: string;
  private apiToken: string;

  constructor(baseUrl: string, apiToken: string) {
    this.baseUrl = baseUrl;
    this.apiToken = apiToken;
  }

  private async request<T>(
    endpoint: string, 
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const response = await fetch(url, {
      ...options,
      headers: {
        'Authorization': `Bearer ${this.apiToken}`,
        'Content-Type': 'application/json',
        ...options.headers,
      },
    });

    if (!response.ok) {
      throw new CicdError(`HTTP ${response.status}: ${response.statusText}`);
    }

    return response.json();
  }

  async createPipeline(config: PipelineConfig): Promise<Pipeline> {
    return this.request<Pipeline>('/api/v1/pipelines', {
      method: 'POST',
      body: JSON.stringify(config),
    });
  }

  async getPipeline(id: string): Promise<Pipeline> {
    return this.request<Pipeline>(`/api/v1/pipelines/${id}`);
  }

  async executePipeline(id: string): Promise<Pipeline> {
    return this.request<Pipeline>(`/api/v1/pipelines/${id}/execute`, {
      method: 'POST',
    });
  }
}

// Usage examples
const client = new CicdClient('http://localhost:8080', 'api-token');

// Async/await usage
const pipeline = await client.createPipeline({
  name: 'web-deployment',
  stages: [
    {
      name: 'build',
      image: 'node:18',
      commands: ['npm install', 'npm run build']
    }
  ]
});

// Promise chain usage
client.createPipeline(config)
  .then(pipeline => client.executePipeline(pipeline.id))
  .then(result => console.log('Pipeline executed:', result.id))
  .catch(error => console.error('Pipeline failed:', error));
```

**TDD Implementation**:
```typescript
// client.test.ts
import { CicdClient } from './client';
import { PipelineStatus } from './types';

describe('CicdClient', () => {
  const client = new CicdClient('http://localhost:8080', 'test-token');

  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  test('should create pipeline successfully', async () => {
    const mockPipeline = {
      id: 'pipeline-123',
      name: 'test-pipeline',
      status: PipelineStatus.PENDING,
      createdAt: new Date()
    };

    (fetch as jest.Mock).mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve(mockPipeline)
    });

    const pipeline = await client.createPipeline({
      name: 'test-pipeline',
      stages: []
    });

    expect(pipeline.id).toBe('pipeline-123');
    expect(pipeline.name).toBe('test-pipeline');
    expect(fetch).toHaveBeenCalledWith(
      'http://localhost:8080/api/v1/pipelines',
      expect.objectContaining({
        method: 'POST',
        headers: {
          'Authorization': 'Bearer test-token',
          'Content-Type': 'application/json'
        }
      })
    );
  });

  test('should throw error on failed request', async () => {
    (fetch as jest.Mock).mockResolvedValueOnce({
      ok: false,
      status: 400,
      statusText: 'Bad Request'
    });

    await expect(client.createPipeline({ name: 'test', stages: [] }))
      .rejects.toThrow('HTTP 400: Bad Request');
  });
});
```

**Conventional Commit**: `feat(js-sdk): implement TypeScript SDK with Node.js and browser support, full type definitions, and NPM distribution`

---

### 🛠️ Sprint 2: Advanced CLI Tools

#### Historia 2.1: Interactive CLI Shell
**Como** DevOps Engineer  
**Quiero** interactive CLI con real-time feedback  
**Para** manage pipelines y workers interactively  

**INVEST**:
- **Independent**: CLI shell implementation standalone
- **Negotiable**: Rich terminal interface
- **Valuable**: Enhanced developer productivity
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en interactive shell
- **Testable**: CLI shell functionality tests

**Acceptance Criteria**:
- [ ] Interactive shell con command history
- [ ] Real-time progress tracking con progress bars
- [ ] Tab completion para commands y parameters
- [ ] Syntax highlighting para configuration files
- [ ] Color-coded output para different status types
- [ ] Auto-suggestions based on context

**Interactive CLI Implementation**:
```rust
pub struct InteractiveShell {
    config: CliConfig,
    history: CommandHistory,
    completer: CommandCompleter,
    progress_tracker: ProgressTracker,
}

impl InteractiveShell {
    pub async fn start(&mut self) -> Result<(), Error> {
        println!("🚀 CI/CD Interactive Shell - Type 'help' for commands");
        println!("============================================================");
        
        let mut rl = Editor::<()>::new();
        rl.set_history(&mut self.history);
        
        loop {
            let readline = rl.readline("cicd> ");
            
            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    
                    rl.add_history_entry(line);
                    
                    let result = self.execute_command(line).await;
                    match result {
                        Ok(output) => println!("{}", output),
                        Err(e) => eprintln!("❌ Error: {}", e),
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("^D");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    async fn execute_command(&self, input: &str) -> Result<String, Error> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok("".to_string());
        }
        
        match parts[0] {
            "pipeline" => self.handle_pipeline_command(&parts[1..]).await,
            "worker" => self.handle_worker_command(&parts[1..]).await,
            "deploy" => self.handle_deploy_command(&parts[1..]).await,
            "monitor" => self.handle_monitor_command(&parts[1..]).await,
            "help" => Ok(self.generate_help()),
            "exit" | "quit" => std::process::exit(0),
            _ => Err(Error::UnknownCommand(parts[0].to_string())),
        }
    }
    
    async fn handle_pipeline_command(&self, args: &[&str]) -> Result<String, Error> {
        match args.get(0) {
            Some(&"list") => {
                let pipelines = self.client.list_pipelines().await?;
                self.format_pipeline_list(&pipelines)
            }
            Some(&"create") => {
                let name = args.get(1).ok_or_else(|| Error::MissingArgument("name".to_string()))?;
                let pipeline = self.create_pipeline_interactive(name).await?;
                Ok(format!("✅ Created pipeline: {}", pipeline.name))
            }
            Some(&"execute") => {
                let pipeline_id = args.get(1).ok_or_else(|| Error::MissingArgument("pipeline_id".to_string()))?;
                let job = self.execute_pipeline_with_progress(pipeline_id).await?;
                Ok(format!("🚀 Executing pipeline: {}", job.id))
            }
            _ => Err(Error::InvalidPipelineCommand),
        }
    }
    
    async fn execute_pipeline_with_progress(&self, pipeline_id: &str) -> Result<Job, Error> {
        let pb = ProgressBar::new(100);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap());
        
        let job = self.client.execute_pipeline(pipeline_id).await?;
        
        // Poll job status con progress updates
        loop {
            let status = self.client.get_job_status(&job.id).await?;
            
            match status {
                JobStatus::Running => {
                    let progress = self.calculate_progress(&status);
                    pb.set_position(progress);
                    pb.set_message(format!("Running {}", status.current_stage));
                }
                JobStatus::Success => {
                    pb.set_position(100);
                    pb.finish_with_message("✅ Pipeline completed successfully");
                    break Ok(job);
                }
                JobStatus::Failed => {
                    pb.abandon_with_message("❌ Pipeline failed");
                    break Err(Error::PipelineFailed(status.error_message));
                }
                _ => {}
            }
            
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}
```

**TDD Implementation**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_interactive_shell_startup() {
        let config = CliConfig::default();
        let shell = InteractiveShell::new(config);
        
        assert!(!shell.is_running());
    }

    #[test]
    fn test_command_completion() {
        let completer = CommandCompleter::new();
        let completions = completer.complete_command("pipe");
        
        assert!(completions.contains(&"pipeline".to_string()));
        assert!(completions.contains(&"pipelines".to_string()));
    }
}
```

**Conventional Commit**: `feat(cli): implement interactive shell with real-time progress tracking, tab completion, and syntax highlighting`

---

#### Historia 2.2: Visual Data Representation
**Como** Software Engineer  
**Quiero** CLI con visual charts y graphs  
**Para** visualize pipeline performance y metrics  

**INVEST**:
- **Independent**: Visualization system independiente
- **Negotiable**: Rich terminal visualization
- **Valuable**: Enhanced data understanding
- **Estimable**: 5 puntos de historia
- **Small**: Enfoque en core visualization
- **Testable**: Chart rendering tests

**Acceptance Criteria**:
- [ ] ASCII charts para pipeline metrics
- [ ] Real-time graphs para resource utilization
- [ ] Tree visualization para pipeline dependencies
- [ ] Status indicators con colors y symbols
- [ ] Performance timelines
- [ ] Interactive drill-down capabilities

**Visualization Implementation**:
```rust
pub struct Visualizer {
    terminal_size: (u16, u16),
    color_theme: ColorTheme,
}

impl Visualizer {
    pub fn render_pipeline_metrics(&self, metrics: &PipelineMetrics) -> String {
        let mut output = String::new();
        
        // Pipeline status bar
        output.push_str(&self.render_status_bar(metrics.status, metrics.duration));
        
        // Success rate chart
        output.push_str("\n📊 Success Rate Over Time:\n");
        output.push_str(&self.render_line_chart(&metrics.success_rates, "Success %"));
        
        // Resource utilization
        output.push_str("\n💾 Resource Utilization:\n");
        output.push_str(&self.render_resource_chart(&metrics.resource_usage));
        
        output
    }
    
    fn render_line_chart(&self, data: &[f64], label: &str) -> String {
        let mut chart = String::new();
        let max_value = data.iter().fold(0.0, f64::max);
        let chart_height = 20;
        let chart_width = data.len();
        
        // Render Y-axis labels
        for i in (0..chart_height).rev() {
            let value = (i as f64 / chart_height as f64) * max_value;
            chart.push_str(&format!("{:>5.1} |", value));
            
            // Render data points
            for &point in data {
                let normalized = (point / max_value * chart_height as f64).round() as usize;
                if normalized >= i {
                    chart.push('█');
                } else {
                    chart.push(' ');
                }
            }
            chart.push('\n');
        }
        
        // Render X-axis
        chart.push_str("      ");
        for i in 0..chart_width {
            chart.push(if i % 5 == 0 { '┼' } else { '┄' });
        }
        chart.push('\n');
        
        chart
    }
    
    fn render_resource_chart(&self, usage: &ResourceUsage) -> String {
        format!(
            "CPU:  [{}{}] {:.1}%\n\
             Memory: [{}{}] {:.1}%\n\
             Disk:  [{}{}] {:.1}%",
            "█".repeat((usage.cpu * 20.0) as usize),
            " ".repeat(20 - (usage.cpu * 20.0) as usize),
            usage.cpu * 100.0,
            "█".repeat((usage.memory * 20.0) as usize),
            " ".repeat(20 - (usage.memory * 20.0) as usize),
            usage.memory * 100.0,
            "█".repeat((usage.disk * 20.0) as usize),
            " ".repeat(20 - (usage.disk * 20.0) as usize),
            usage.disk * 100.0,
        )
    }
    
    fn render_tree(&self, pipeline: &Pipeline) -> String {
        let mut tree = String::new();
        tree.push_str(&format!("📋 {}\n", pipeline.name));
        
        for (i, stage) in pipeline.stages.iter().enumerate() {
            let is_last = i == pipeline.stages.len() - 1;
            let prefix = if is_last { "└── " } else { "├── " };
            
            let status_icon = match stage.status {
                StageStatus::Success => "✅",
                StageStatus::Running => "🔄",
                StageStatus::Failed => "❌",
                StageStatus::Pending => "⏳",
            };
            
            tree.push_str(&format!("{} {}{} ({})\n", 
                prefix, status_icon, stage.name, stage.duration));
        }
        
        tree
    }
}

#[derive(Debug, Clone)]
pub struct ColorTheme {
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,
    pub running: String,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            success: "\x1b[32m".to_string(), // Green
            warning: "\x1b[33m".to_string(), // Yellow
            error: "\x1b[31m".to_string(),   // Red
            info: "\x1b[36m".to_string(),    // Cyan
            running: "\x1b[35m".to_string(), // Magenta
        }
    }
}
```

**Conventional Commit**: `feat(cli): implement terminal visualization with ASCII charts, resource utilization graphs, and pipeline tree views`

---

#### Historia 2.3: Batch Operations Support
**Como** Operations Team  
**Quiero** CLI support para batch operations  
**Para** manage multiple pipelines y workers simultaneously  

**INVEST**:
- **Independent**: Batch operations standalone feature
- **Negotiable**: Parallel execution capabilities
- **Valuable**: Operational efficiency
- **Estimable**: 5 puntos de historia
- **Small**: Enfoque en batch logic
- **Testable**: Batch operation tests

**Acceptance Criteria**:
- [ ] Bulk pipeline execution con parallel processing
- [ ] Batch worker scaling operations
- [ ] Progress tracking para multiple operations
- [ ] Error aggregation y reporting
- [ ] Rollback capabilities para failed operations
- [ ] Configurable concurrency limits

**Batch Operations Implementation**:
```rust
pub struct BatchOperator {
    client: Arc<CicdClient>,
    concurrency_limit: usize,
    retry_policy: RetryPolicy,
}

impl BatchOperator {
    pub async fn execute_pipelines_parallel(
        &self,
        pipeline_ids: Vec<String>,
    ) -> Result<BatchExecutionResult, Error> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        let tasks = pipeline_ids.into_iter().map(|id| {
            let semaphore = Arc::clone(&semaphore);
            let client = Arc::clone(&self.client);
            
            async move {
                let _permit = semaphore.acquire().await;
                client.execute_pipeline(&id).await.map(|job| (id, job))
            }
        });
        
        let results = futures::future::join_all(tasks).await;
        
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        
        for result in results {
            match result {
                Ok((id, job)) => successful.push((id, job)),
                Err(e) => failed.push(e),
            }
        }
        
        Ok(BatchExecutionResult {
            successful,
            failed,
            total_count: successful.len() + failed.len(),
        })
    }
    
    pub async fn scale_workers_batch(
        &self,
        scaling_requests: Vec<ScalingRequest>,
    ) -> Result<Vec<ScalingResult>, Error> {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        let mut results = Vec::new();
        
        for request in scaling_requests {
            let semaphore = Arc::clone(&semaphore);
            let client = Arc::clone(&self.client);
            
            let result = tokio::spawn(async move {
                let _permit = semaphore.acquire().await;
                client.scale_workers(&request.worker_group, request.target_count).await
            }).await.map_err(|e| Error::TaskJoinError(e.to_string()))?;
            
            results.push(result);
        }
        
        Ok(results)
    }
    
    pub async fn monitor_batch_execution(
        &self,
        execution_id: &str,
    ) -> Result<BatchExecutionStatus, Error> {
        let mut active_jobs = self.get_active_batch_jobs(execution_id).await?;
        
        loop {
            let mut all_completed = true;
            let mut completed_jobs = Vec::new();
            
            for job in &mut active_jobs {
                match self.client.get_job_status(&job.id).await? {
                    JobStatus::Running => {
                        all_completed = false;
                    }
                    JobStatus::Success | JobStatus::Failed => {
                        completed_jobs.push(job.clone());
                    }
                    _ => {
                        all_completed = false;
                    }
                }
            }
            
            if all_completed {
                break Ok(BatchExecutionStatus::Completed {
                    completed: completed_jobs,
                    execution_id: execution_id.to_string(),
                });
            }
            
            // Progress reporting
            let progress = self.calculate_batch_progress(&active_jobs, &completed_jobs);
            println!("Batch execution progress: {:.1}%", progress * 100.0);
            
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

#[derive(Debug)]
pub struct BatchExecutionResult {
    pub successful: Vec<(String, Job)>,
    pub failed: Vec<Error>,
    pub total_count: usize,
}

impl BatchExecutionResult {
    pub fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.successful.len() as f64 / self.total_count as f64
    }
    
    pub fn generate_report(&self) -> String {
        format!(
            "Batch Execution Report\n\
             ====================\n\
             Total: {}\n\
             Successful: {}\n\
             Failed: {}\n\
             Success Rate: {:.1}%",
            self.total_count,
            self.successful.len(),
            self.failed.len(),
            self.success_rate() * 100.0
        )
    }
}
```

**Conventional Commit**: `feat(cli): implement batch operations with parallel execution, progress monitoring, and consolidated reporting`

---

### 🎨 Sprint 3: IDE Integration Plugins

#### Historia 3.1: VS Code Extension
**Como** VS Code User  
**Quiero** VS Code extension para CI/CD management  
**Para** manage pipelines directly desde el editor  

**INVEST**:
- **Independent**: VS Code extension standalone
- **Negotiable**: Rich VS Code integration
- **Valuable**: Integrated development workflow
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core extension features
- **Testable**: Extension functionality tests

**Acceptance Criteria**:
- [ ] Pipeline explorer en side panel
- [ ] Right-click context menus para pipeline operations
- [ ] Real-time pipeline status updates
- [ ] Inline pipeline logs display
- [ ] CodeLens integration para pipeline triggers
- [ ] Settings integration para API configuration

**VS Code Extension Implementation**:
```typescript
// extension.ts - Main extension entry point
import * as vscode from 'vscode';
import { CicdClient } from './client';
import { PipelineTreeProvider } from './tree-view';
import { PipelineCommands } from './commands';

export async function activate(context: vscode.ExtensionContext) {
    console.log('CI/CD Extension activated');

    // Initialize CicdClient
    const cicdClient = new CicdClient(
        vscode.workspace.getConfiguration('cicd').get('apiUrl') || 'http://localhost:8080',
        vscode.workspace.getConfiguration('cicd').get('apiToken') || ''
    );

    // Register tree view provider
    const pipelineTreeProvider = new PipelineTreeProvider(cicdClient);
    vscode.window.registerTreeDataProvider('cicdExplorer', pipelineTreeProvider);

    // Register commands
    const commands = new PipelineCommands(cicdClient, pipelineTreeProvider);
    
    context.subscriptions.push(
        vscode.commands.registerCommand('cicd.refreshExplorer', () => {
            pipelineTreeProvider.refresh();
        }),
        
        vscode.commands.registerCommand('cicd.createPipeline', async () => {
            await commands.createPipeline();
        }),
        
        vscode.commands.registerCommand('cicd.executePipeline', async (pipelineId: string) => {
            await commands.executePipeline(pipelineId);
        }),
        
        vscode.commands.registerCommand('cicd.viewLogs', async (pipelineId: string) => {
            await commands.viewPipelineLogs(pipelineId);
        }),
        
        vscode.commands.registerCommand('cicd.workerManagement', async () => {
            await commands.showWorkerManagement();
        })
    );

    // Register CodeLens provider
    const codeLensProvider = vscode.languages.registerCodeLensProvider(
        { pattern: '**/*.cicd.yml' },
        {
            provideCodeLenses(document: vscode.TextDocument) {
                return [
                    new vscode.CodeLens(
                        new vscode.Range(0, 0, 0, 0),
                        {
                            title: '🚀 Deploy Pipeline',
                            command: 'cicd.executePipelineFromFile',
                            arguments: [document.uri]
                        }
                    )
                ];
            }
        }
    );

    context.subscriptions.push(codeLensProvider);
}

export function deactivate() {
    console.log('CI/CD Extension deactivated');
}

// tree-view.ts - Pipeline Explorer
export class PipelineTreeProvider implements vscode.TreeDataProvider<CicdNode> {
    private _onDidChangeTreeData: vscode.EventEmitter<CicdNode | undefined | null | void> = new vscode.EventEmitter<CicdNode | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<CicdNode | undefined | null | void> = this._onDidChangeTreeData.event;

    constructor(private cicdClient: CicdClient) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: CicdNode): vscode.TreeItem {
        if (element instanceof PipelineNode) {
            const treeItem = new vscode.TreeItem(
                element.pipeline.name,
                vscode.TreeItemCollapsibleState.Collapsed
            );
            
            // Set icons based on pipeline status
            switch (element.pipeline.status) {
                case 'SUCCESS':
                    treeItem.iconPath = new vscode.ThemeIcon('check');
                    treeItem.contextValue = 'pipeline-success';
                    break;
                case 'FAILED':
                    treeItem.iconPath = new vscode.ThemeIcon('error');
                    treeItem.contextValue = 'pipeline-failed';
                    break;
                case 'RUNNING':
                    treeItem.iconPath = new vscode.ThemeIcon('sync~spin');
                    treeItem.contextValue = 'pipeline-running';
                    break;
                default:
                    treeItem.iconPath = new vscode.ThemeIcon('clock');
                    treeItem.contextValue = 'pipeline-pending';
            }
            
            treeItem.command = {
                command: 'cicd.viewPipelineDetails',
                title: 'View Pipeline Details',
                arguments: [element.pipeline.id]
            };
            
            return treeItem;
        }
        
        if (element instanceof JobNode) {
            const treeItem = new vscode.TreeItem(
                `Job: ${element.job.name}`,
                vscode.TreeItemCollapsibleState.None
            );
            
            treeItem.contextValue = 'job';
            treeItem.command = {
                command: 'cicd.viewJobLogs',
                title: 'View Job Logs',
                arguments: [element.job.id]
            };
            
            return treeItem;
        }
        
        throw new Error('Unknown tree node type');
    }

    async getChildren(element?: CicdNode): Promise<CicdNode[]> {
        if (!element) {
            // Root level - return pipelines
            try {
                const pipelines = await this.cicdClient.listPipelines();
                return pipelines.map(p => new PipelineNode(p));
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to load pipelines: ${error.message}`);
                return [];
            }
        }
        
        if (element instanceof PipelineNode) {
            // Pipeline level - return jobs
            try {
                const jobs = await this.cicdClient.listJobs(element.pipeline.id);
                return jobs.map(j => new JobNode(j));
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to load jobs: ${error.message}`);
                return []
            }
        }
        
        return [];
    }
}

// commands.ts - Command implementations
export class PipelineCommands {
    constructor(
        private cicdClient: CicdClient,
        private treeProvider: PipelineTreeProvider
    ) {}

    async createPipeline() {
        const name = await vscode.window.showInputBox({
            prompt: 'Enter pipeline name',
            placeHolder: 'my-pipeline'
        });
        
        if (!name) return;
        
        try {
            const pipeline = await this.cicdClient.createPipeline({
                name,
                stages: [
                    {
                        name: 'build',
                        image: 'node:18',
                        commands: ['npm install', 'npm run build']
                    }
                ]
            });
            
            vscode.window.showInformationMessage(`Created pipeline: ${pipeline.name}`);
            this.treeProvider.refresh();
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to create pipeline: ${error.message}`);
        }
    }
    
    async executePipeline(pipelineId: string) {
        try {
            const job = await this.cicdClient.executePipeline(pipelineId);
            
            // Show output channel para real-time logs
            const outputChannel = vscode.window.createOutputChannel('CI/CD Pipeline');
            outputChannel.show();
            outputChannel.appendLine(`🚀 Starting pipeline execution: ${job.id}`);
            
            // Monitor job status y update logs
            this.monitorJobAndUpdateLogs(job.id, outputChannel);
            
            vscode.window.showInformationMessage(`Pipeline execution started: ${job.id}`);
            this.treeProvider.refresh();
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to execute pipeline: ${error.message}`);
        }
    }
    
    private async monitorJobAndUpdateLogs(jobId: string, outputChannel: vscode.OutputChannel) {
        while (true) {
            try {
                const job = await this.cicdClient.getJobStatus(jobId);
                
                if (job.status === 'SUCCESS' || job.status === 'FAILED') {
                    outputChannel.appendLine(`\n✅ Pipeline ${job.status.toLowerCase()}`);
                    break;
                }
                
                // Append new logs
                const logs = await this.cicdClient.getJobLogs(jobId);
                outputChannel.clear();
                outputChannel.appendLine(logs.output);
                
                await new Promise(resolve => setTimeout(resolve, 2000));
            } catch (error) {
                outputChannel.appendLine(`Error monitoring job: ${error.message}`);
                break;
            }
        }
    }
}
```

**TDD Implementation**:
```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';
import { PipelineTreeProvider } from '../tree-view';
import { CicdClient } from '../client';

// Mock CicdClient
class MockCicdClient {
    async listPipelines() {
        return [
            { id: '1', name: 'test-pipeline', status: 'SUCCESS' },
            { id: '2', name: 'build-pipeline', status: 'RUNNING' }
        ];
    }
}

suite('PipelineTreeProvider', () => {
    let mockClient: MockCicdClient;
    let provider: PipelineTreeProvider;

    setup(() => {
        mockClient = new MockCicdClient();
        provider = new PipelineTreeProvider(mockClient as any);
    });

    test('should return pipeline nodes at root level', async () => {
        const children = await provider.getChildren();
        
        assert.strictEqual(children.length, 2);
        assert.strictEqual(children[0].label, 'test-pipeline');
        assert.strictEqual(children[1].label, 'build-pipeline');
    });

    test('should provide tree items with correct context values', async () => {
        const children = await provider.getChildren();
        const treeItem = provider.getTreeItem(children[0]);
        
        assert.strictEqual(treeItem.contextValue, 'pipeline-success');
    });
});
```

**Conventional Commit**: `feat(vscode-extension): implement VS Code extension with pipeline explorer, real-time monitoring, and CodeLens integration`

---

#### Historia 3.2: IntelliJ IDEA Plugin
**Como** IntelliJ User  
**Quiero** IntelliJ plugin para CI/CD integration  
**Para** manage pipelines desde el IDE  

**INVEST**:
- **Independent**: IntelliJ plugin standalone
- **Negotiable**: IntelliJ Platform integration
- **Valuable**: Java/Kotlin developer workflow
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core plugin features
- **Testable**: Plugin functionality tests

**Acceptance Criteria**:
- [ ] Tool window para pipeline management
- [ ] Run configuration integration
- [ ] Build tool integration (Maven, Gradle)
- [ ] Live templates para pipeline definition
- [ ] Code inspection para pipeline files
- [ ] Settings dialog para API configuration

**IntelliJ Plugin Implementation**:
```kotlin
// CicdPlugin.kt - Main plugin class
class CicdPlugin : ApplicationComponent {
    override fun initComponent() {
        // Initialize plugin components
        CicdToolWindowManager.getInstance().registerToolWindow()
    }
}

// CicdToolWindow.kt - Main tool window
class CicdToolWindow : ToolWindowFactory {
    override fun createToolWindowContent(project: Project) {
        val cicdToolWindow = CicdToolWindow(project)
        val contentManager = toolWindow.contentManager
        val content = contentManager.factory.createContent(cicdToolWindow.getContent(), null, false)
        contentManager.addContent(content)
    }
}

class CicdToolWindow(private val project: Project) {
    private val cicdService = CicdService.getInstance(project)
    private val cicdClient = CicdClient(
        CicdConfig.getApiUrl(),
        CicdConfig.getApiToken()
    )

    fun getContent(): JPanel {
        val mainPanel = JPanel(BorderLayout())
        
        // Pipeline list panel
        val pipelinePanel = JPanel(BorderLayout())
        val pipelineList = DefaultListModel<String>()
        val pipelineJList = JList(pipelineList)
        
        pipelineJList.addListSelectionListener { e ->
            if (!e.valueIsAdjusting) {
                val selectedPipeline = pipelineJList.selectedValue
                if (selectedPipeline != null) {
                    displayPipelineDetails(selectedPipeline)
                }
            }
        }
        
        // Pipeline actions panel
        val actionPanel = JPanel(FlowLayout())
        val createButton = JButton("Create Pipeline")
        val executeButton = JButton("Execute Pipeline")
        val logsButton = JButton("View Logs")
        
        createButton.addActionListener {
            createNewPipeline()
        }
        
        executeButton.addActionListener {
            val selectedPipeline = pipelineJList.selectedValue
            if (selectedPipeline != null) {
                executePipeline(selectedPipeline)
            }
        }
        
        actionPanel.add(createButton)
        actionPanel.add(executeButton)
        actionPanel.add(logsButton)
        
        pipelinePanel.add(JScrollPane(pipelineJList), BorderLayout.CENTER)
        pipelinePanel.add(actionPanel, BorderLayout.SOUTH)
        
        // Pipeline details panel
        val detailsPanel = JPanel(BorderLayout())
        detailsPanel.border = BorderFactory.createTitledBorder("Pipeline Details")
        
        mainPanel.add(pipelinePanel, BorderLayout.WEST)
        mainPanel.add(detailsPanel, BorderLayout.CENTER)
        
        // Load pipelines
        loadPipelines(pipelineList)
        
        return mainPanel
    }
    
    private fun loadPipelines(listModel: DefaultListModel<String>) {
        thread {
            try {
                val pipelines = cicdClient.listPipelines()
                
                SwingUtilities.invokeLater {
                    listModel.clear()
                    pipelines.forEach { pipeline ->
                        listModel.addElement("${pipeline.name} (${pipeline.status})")
                    }
                }
            } catch (e: Exception) {
                SwingUtilities.invokeLater {
                    JOptionPane.showMessageDialog(
                        null,
                        "Failed to load pipelines: ${e.message}",
                        "Error",
                        JOptionPane.ERROR_MESSAGE
                    )
                }
            }
        }
    }
    
    private fun createNewPipeline() {
        val dialog = CreatePipelineDialog(project, cicdClient)
        if (dialog.showAndGet()) {
            loadPipelines(dialog.listModel)
        }
    }
    
    private fun executePipeline(pipelineName: String) {
        thread {
            try {
                val pipeline = cicdClient.getPipelineByName(pipelineName)
                val job = cicdClient.executePipeline(pipeline.id)
                
                SwingUtilities.invokeLater {
                    JOptionPane.showMessageDialog(
                        null,
                        "Pipeline execution started: ${job.id}",
                        "Success",
                        JOptionPane.INFORMATION_MESSAGE
                    )
                }
            } catch (e: Exception) {
                SwingUtilities.invokeLater {
                    JOptionPane.showMessageDialog(
                        null,
                        "Failed to execute pipeline: ${e.message}",
                        "Error",
                        JOptionPane.ERROR_MESSAGE
                    )
                }
            }
        }
    }
}

// CicdService.kt - Plugin service
class CicdService : ProjectService {
    private val cicdClient = CicdClient(
        CicdConfig.getApiUrl(),
        CicdConfig.getApiToken()
    )
    
    companion object {
        fun getInstance(project: Project): CicdService = 
            project.getService(CicdService::class.java)
    }
    
    suspend fun createPipeline(config: PipelineConfig): Pipeline {
        return cicdClient.createPipeline(config)
    }
    
    suspend fun executePipeline(pipelineId: String): Job {
        return cicdClient.executePipeline(pipelineId)
    }
    
    suspend fun getPipelineLogs(pipelineId: String): String {
        return cicdClient.getPipelineLogs(pipelineId)
    }
}

// CicdRunConfiguration.kt - Run configuration integration
class CicdRunConfiguration(
    name: String,
    project: Project,
    configurationFactory: ConfigurationFactory
) : RunConfigurationBase<CicdRunConfigurationOptions>(project, configurationFactory, name) {
    
    private var pipelineId: String = ""
    private var environmentVariables: Map<String, String> = emptyMap()
    
    override fun getOptions(): CicdRunConfigurationOptions = 
        CicdRunConfigurationOptions()
    
    override fun checkConfiguration() {
        if (pipelineId.isEmpty()) {
            throw RuntimeConfigurationWarning("Pipeline ID is required")
        }
    }
    
    override fun getConfigurationEditor(): SettingsEditor<CicdRunConfigurationOptions> = 
        CicdRunConfigurationEditor(project)
    
    override fun run(environment: ExecutionEnvironment) {
        val executor = ExecutorRegistry.getInstance().getExecutorById(DefaultRunExecutor.EXECUTOR_ID)
        val runner = CicdRunProfileState(environment.project, pipelineId, environmentVariables)
        executor.execute(environment, runner)
    }
}

class CicdRunProfileState(
    private val project: Project,
    private val pipelineId: String,
    private val environment: Map<String, String>
) : RunProfileState {
    
    override fun execute(executor: Executor, env: ExecutionEnvironment): ExecutionResult {
        return thread {
            try {
                val cicdService = CicdService.getInstance(project)
                val job = cicdService.executePipeline(pipelineId)
                
                ExecutionResult()
                    .withProcessHandler(EmptyProcessHandler())
                    .withConsole(viewJobLogs(job.id))
            } catch (e: Exception) {
                throw ExecutionException("Pipeline execution failed: ${e.message}")
            }
        }
    }
    
    private fun viewJobLogs(jobId: String): ConsoleViewImpl {
        val console = ConsoleViewImpl()
        
        thread {
            try {
                val cicdService = CicdService.getInstance(project)
                val logs = cicdService.getPipelineLogs(jobId)
                
                SwingUtilities.invokeLater {
                    console.print(logs, ConsoleViewContentType.LOG_INFO_LEVEL)
                }
            } catch (e: Exception) {
                SwingUtilities.invokeLater {
                    console.print("Failed to fetch logs: ${e.message}", ConsoleViewContentType.ERROR_OUTPUT)
                }
            }
        }
        
        return console
    }
}
```

**TDD Implementation**:
```kotlin
@Test
fun `should create pipeline successfully`() {
    val project = mockProject()
    val cicdService = CicdService(project)
    
    runBlocking {
        val pipeline = cicdService.createPipeline(
            PipelineConfig(
                name = "test-pipeline",
                stages = listOf(
                    StageConfig(
                        name = "build",
                        image = "maven:3.8",
                        commands = listOf("mvn compile")
                    )
                )
            )
        )
        
        assertEquals("test-pipeline", pipeline.name)
        assertNotNull(pipeline.id)
    }
}

@Test
fun `should validate pipeline configuration`() {
    val config = CicdRunConfiguration("test", mockProject(), mockFactory())
    
    assertFailsWith<RuntimeConfigurationWarning> {
        config.checkConfiguration()
    }
}
```

**Conventional Commit**: `feat(intellij-plugin): implement IntelliJ plugin with tool window, run configuration integration, and build tool support`

---

### 🏪 Sprint 4: Self-Service Developer Portal

#### Historia 4.1: Developer Dashboard
**Como** Developer  
**Quiero** web dashboard para manage todos mis resources  
**Para** visualize pipelines, workers, y metrics en un solo lugar  

**INVEST**:
- **Independent**: Portal dashboard standalone
- **Negotiable**: React-based responsive dashboard
- **Valuable**: Centralized resource management
- **Estimable**: 13 puntos de historia
- **Small**: Enfoque en core dashboard features
- **Testable**: Dashboard component tests

**Acceptance Criteria**:
- [ ] Real-time pipeline status dashboard
- [ ] Worker utilization metrics visualization
- [ ] Resource usage charts y trends
- [ ] Quick actions para common operations
- [ ] Search y filter capabilities
- [ ] Mobile-responsive design

**React Dashboard Implementation**:
```tsx
// components/Dashboard.tsx
import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { 
  PipelineStatus, 
  WorkerMetrics, 
  ResourceUsage 
} from '@/types';
import { ApiClient } from '@/services/ApiClient';
import { WebSocketService } from '@/services/WebSocketService';
import { 
  Activity, 
  Cpu, 
  HardDrive, 
  MemoryStick, 
  PlayCircle, 
  RefreshCw,
  Search,
  Plus
} from 'lucide-react';

export const Dashboard: React.FC = () => {
  const [pipelines, setPipelines] = useState<PipelineStatus[]>([]);
  const [workers, setWorkers] = useState<WorkerMetrics[]>([]);
  const [resourceUsage, setResourceUsage] = useState<ResourceUsage | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedTab, setSelectedTab] = useState('overview');

  const apiClient = new ApiClient();
  const wsService = new WebSocketService();

  useEffect(() => {
    loadDashboardData();
    setupWebSocketConnection();
    
    return () => {
      wsService.disconnect();
    };
  }, []);

  const loadDashboardData = async () => {
    try {
      setIsLoading(true);
      const [pipelinesData, workersData, resourceData] = await Promise.all([
        apiClient.getPipelines(),
        apiClient.getWorkers(),
        apiClient.getResourceUsage()
      ]);
      
      setPipelines(pipelinesData);
      setWorkers(workersData);
      setResourceUsage(resourceData);
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const setupWebSocketConnection = () => {
    wsService.connect('/dashboard', {
      onPipelineUpdate: (updatedPipeline: PipelineStatus) => {
        setPipelines(prev => 
          prev.map(p => p.id === updatedPipeline.id ? updatedPipeline : p)
        );
      },
      onWorkerUpdate: (updatedWorker: WorkerMetrics) => {
        setWorkers(prev => 
          prev.map(w => w.id === updatedWorker.id ? updatedWorker : w)
        );
      },
      onResourceUpdate: (updatedResource: ResourceUsage) => {
        setResourceUsage(updatedResource);
      }
    });
  };

  const handleExecutePipeline = async (pipelineId: string) => {
    try {
      await apiClient.executePipeline(pipelineId);
      await loadDashboardData(); // Refresh data
    } catch (error) {
      console.error('Failed to execute pipeline:', error);
    }
  };

  const filteredPipelines = pipelines.filter(pipeline =>
    pipeline.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
    pipeline.status.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const getStatusColor = (status: string) => {
    switch (status.toLowerCase()) {
      case 'success': return 'bg-green-500';
      case 'failed': return 'bg-red-500';
      case 'running': return 'bg-blue-500';
      case 'pending': return 'bg-yellow-500';
      default: return 'bg-gray-500';
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-96">
        <RefreshCw className="w-8 h-8 animate-spin" />
        <span className="ml-2">Loading dashboard...</span>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold">Developer Dashboard</h1>
        <Button onClick={loadDashboardData} variant="outline">
          <RefreshCw className="w-4 h-4 mr-2" />
          Refresh
        </Button>
      </div>

      {/* Quick Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Pipelines</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {pipelines.filter(p => p.status === 'RUNNING').length}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Workers</CardTitle>
            <Cpu className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{workers.length}</div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">CPU Usage</CardTitle>
            <MemoryStick className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {resourceUsage ? `${Math.round(resourceUsage.cpu * 100)}%` : 'N/A'}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Memory Usage</CardTitle>
            <HardDrive className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {resourceUsage ? `${Math.round(resourceUsage.memory * 100)}%` : 'N/A'}
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Main Content */}
      <Tabs value={selectedTab} onValueChange={setSelectedTab}>
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="pipelines">Pipelines</TabsTrigger>
          <TabsTrigger value="workers">Workers</TabsTrigger>
          <TabsTrigger value="analytics">Analytics</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="space-y-6">
          {/* Pipeline Status Overview */}
          <Card>
            <CardHeader>
              <CardTitle>Recent Pipeline Activity</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {pipelines.slice(0, 5).map((pipeline) => (
                  <div key={pipeline.id} className="flex items-center justify-between p-4 border rounded-lg">
                    <div className="flex items-center space-x-3">
                      <div className={`w-3 h-3 rounded-full ${getStatusColor(pipeline.status)}`} />
                      <div>
                        <h3 className="font-medium">{pipeline.name}</h3>
                        <p className="text-sm text-muted-foreground">
                          Started: {new Date(pipeline.startTime).toLocaleString()}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <Badge variant={pipeline.status === 'SUCCESS' ? 'default' : 'secondary'}>
                        {pipeline.status}
                      </Badge>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => handleExecutePipeline(pipeline.id)}
                        disabled={pipeline.status === 'RUNNING'}
                      >
                        <PlayCircle className="w-4 h-4 mr-1" />
                        Execute
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="pipelines" className="space-y-6">
          {/*          {/* Search and Filter */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>Pipeline Management</CardTitle>
                <div className="flex items-center space-x-2">
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground w-4 h-4" />
                    <Input
                      placeholder="Search pipelines..."
                      value={searchTerm}
                      onChange={(e) => setSearchTerm(e.target.value)}
                      className="pl-10"
                    />
                  </div>
                  <Button>
                    <Plus className="w-4 h-4 mr-2" />
                    New Pipeline
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {filteredPipelines.map((pipeline) => (
                  <div key={pipeline.id} className="flex items-center justify-between p-4 border rounded-lg hover:bg-muted/50">
                    <div className="flex items-center space-x-4">
                      <div className={`w-3 h-3 rounded-full ${getStatusColor(pipeline.status)}`} />
                      <div>
                        <h3 className="font-medium">{pipeline.name}</h3>
                        <p className="text-sm text-muted-foreground">
                          Last execution: {pipeline.lastExecution ? 
                            new Date(pipeline.lastExecution).toLocaleString() : 'Never'}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <Badge variant="outline">{pipeline.stages.length} stages</Badge>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => handleExecutePipeline(pipeline.id)}
                        disabled={pipeline.status === 'RUNNING'}
                      >
                        <PlayCircle className="w-4 h-4 mr-1" />
                        Execute
                      </Button>
                      <Button size="sm" variant="ghost">
                        View Details
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="workers" className="space-y-6">
          {/* Worker Management */}
          <Card>
            <CardHeader>
              <CardTitle>Worker Status</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {workers.map((worker) => (
                  <div key={worker.id} className="p-4 border rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <h3 className="font-medium">{worker.name}</h3>
                      <Badge variant={worker.status === 'ACTIVE' ? 'default' : 'secondary'}>
                        {worker.status}
                      </Badge>
                    </div>
                    <div className="space-y-2 text-sm">
                      <div className="flex justify-between">
                        <span>CPU Usage:</span>
                        <span>{Math.round(worker.cpuUsage * 100)}%</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Memory Usage:</span>
                        <span>{Math.round(worker.memoryUsage * 100)}%</span>
                      </div>
                      <div className="flex justify-between">
                        <span>Jobs Processed:</span>
                        <span>{worker.jobsProcessed}</span>
                      </div>
                    </div>
                    <div className="mt-2">
                      <div className="w-full bg-gray-200 rounded-full h-2">
                        <div 
                          className="bg-blue-600 h-2 rounded-full" 
                          style={{ width: `${worker.cpuUsage * 100}%` }}
                        />
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="analytics" className="space-y-6">
          {/* Analytics Charts */}
          <Card>
            <CardHeader>
              <CardTitle>Performance Analytics</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="h-64 flex items-center justify-center text-muted-foreground">
                Analytics charts will be implemented here
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
};

export default Dashboard;#### Historia 4.2: Self-Service Resource Management
**Como** Developer  
**Quiero** self-service portal para manage recursos  
**Para** provision y scale workers autonomously  

**INVEST**:
- **Independent**: Resource management standalone
- **Negotiable**: User-friendly resource provisioning
- **Valuable**: Autonomous resource management
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core resource management
- **Testable**: Resource provisioning tests

**Acceptance Criteria**:
- [ ] Worker group creation y configuration
- [ ] Auto-scaling policy definition
- [ ] Resource quota management
- [ ] Cost tracking y optimization
- [ ] Resource usage analytics
- [ ] Bulk operations support

**Resource Management Implementation**:
```tsx
// components/ResourceManagement.tsx
import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { Slider } from '@/components/ui/slider';
import { WorkerGroup, ScalingPolicy, ResourceQuota } from '@/types';
import { ApiClient } from '@/services/ApiClient';
import { 
  Plus, 
  Settings, 
  Trash2, 
  TrendingUp, 
  TrendingDown,
  DollarSign,
  Cpu,
  HardDrive
} from 'lucide-react';

export const ResourceManagement: React.FC = () => {
  const [workerGroups, setWorkerGroups] = useState<WorkerGroup[]>([]);
  const [isCreating, setIsCreating] = useState(false);
  const [newGroup, setNewGroup] = useState({
    name: '',
    provider: '',
    instanceType: '',
    minWorkers: 1,
    maxWorkers: 10,
    scalingPolicy: 'CPU' as ScalingPolicy,
    autoScaling: false,
    resourceQuota: {
      maxCPU: 8,
      maxMemory: 16,
      maxStorage: 100
    }
  });

  const apiClient = new ApiClient();

  useEffect(() => {
    loadWorkerGroups();
  }, []);

  const loadWorkerGroups = async () => {
    try {
      const groups = await apiClient.getWorkerGroups();
      setWorkerGroups(groups);
    } catch (error) {
      console.error('Failed to load worker groups:', error);
    }
  };

  const handleCreateGroup = async () => {
    try {
      await apiClient.createWorkerGroup(newGroup);
      setIsCreating(false);
      setNewGroup({
        name: '',
        provider: '',
        instanceType: '',
        minWorkers: 1,
        maxWorkers: 10,
        scalingPolicy: 'CPU',
        autoScaling: false,
        resourceQuota: { maxCPU: 8, maxMemory: 16, maxStorage: 100 }
      });
      await loadWorkerGroups();
    } catch (error) {
      console.error('Failed to create worker group:', error);
    }
  };

  const handleScaleGroup = async (groupId: string, targetWorkers: number) => {
    try {
      await apiClient.scaleWorkerGroup(groupId, targetWorkers);
      await loadWorkerGroups();
    } catch (error) {
      console.error('Failed to scale worker group:', error);
    }
  };

  const getScalingIcon = (policy: ScalingPolicy) => {
    switch (policy) {
      case 'CPU': return <Cpu className="w-4 h-4" />;
      case 'MEMORY': return <HardDrive className="w-4 h-4" />;
      default: return <TrendingUp className="w-4 h-4" />;
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <h2 className="text-2xl font-bold">Resource Management</h2>
        <Button onClick={() => setIsCreating(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Create Worker Group
        </Button>
      </div>

      {/* Worker Groups Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {workerGroups.map((group) => (
          <Card key={group.id}>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-lg">{group.name}</CardTitle>
                <div className="flex items-center space-x-2">
                  {group.autoScaling && (
                    <Badge variant="default">
                      {getScalingIcon(group.scalingPolicy)}
                      <span className="ml-1">Auto-scaling</span>
                    </Badge>
                  )}
                  <Button size="sm" variant="ghost">
                    <Settings className="w-4 h-4" />
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Current Stats */}
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <p className="text-muted-foreground">Current Workers</p>
                  <p className="font-medium">{group.currentWorkers}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Provider</p>
                  <p className="font-medium capitalize">{group.provider}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Instance Type</p>
                  <p className="font-medium">{group.instanceType}</p>
                </div>
                <div>
                  <p className="text-muted-foreground">Jobs Processed</p>
                  <p className="font-medium">{group.jobsProcessed}</p>
                </div>
              </div>

              {/* Scaling Controls */}
              <div className="space-y-2">
                <Label>Target Workers: {group.currentWorkers}</Label>
                <Slider
                  value={[group.currentWorkers]}
                  onValueChange={([value]) => handleScaleGroup(group.id, value)}
                  max={group.maxWorkers}
                  min={group.minWorkers}
                  step={1}
                  className="w-full"
                />
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>Min: {group.minWorkers}</span>
                  <span>Max: {group.maxWorkers}</span>
                </div>
              </div>

              {/* Resource Usage */}
              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span>CPU Usage</span>
                  <span>{Math.round(group.cpuUsage * 100)}%</span>
                </div>
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div 
                    className="bg-blue-600 h-2 rounded-full" 
                    style={{ width: `${group.cpuUsage * 100}%` }}
                  />
                </div>
              </div>

              {/* Cost Information */}
              <div className="pt-2 border-t">
                <div className="flex items-center justify-between text-sm">
                  <span className="flex items-center">
                    <DollarSign className="w-4 h-4 mr-1" />
                    Estimated Cost
                  </span>
                  <span className="font-medium">${group.estimatedCost}/hour</span>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Create Worker Group Modal */}
      {isCreating && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <Card className="w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <CardHeader>
              <CardTitle>Create Worker Group</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="name">Group Name</Label>
                  <Input
                    id="name"
                    value={newGroup.name}
                    onChange={(e) => setNewGroup(prev => ({ ...prev, name: e.target.value }))}
                    placeholder="my-worker-group"
                  />
                </div>
                <div>
                  <Label htmlFor="provider">Provider</Label>
                  <Select 
                    value={newGroup.provider} 
                    onValueChange={(value) => setNewGroup(prev => ({ ...prev, provider: value }))}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select provider" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="aws">AWS</SelectItem>
                      <SelectItem value="gcp">GCP</SelectItem>
                      <SelectItem value="azure">Azure</SelectItem>
                      <SelectItem value="docker">Docker</SelectItem>
                      <SelectItem value="kubernetes">Kubernetes</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="instanceType">Instance Type</Label>
                  <Input
                    id="instanceType"
                    value={newGroup.instanceType}
                    onChange={(e) => setNewGroup(prev => ({ ...prev, instanceType: e.target.value }))}
                    placeholder="m5.large"
                  />
                </div>
                <div>
                  <Label htmlFor="scalingPolicy">Scaling Policy</Label>
                  <Select 
                    value={newGroup.scalingPolicy} 
                    onValueChange={(value) => setNewGroup(prev => ({ ...prev, scalingPolicy: value as ScalingPolicy }))}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="CPU">CPU Based</SelectItem>
                      <SelectItem value="MEMORY">Memory Based</SelectItem>
                      <SelectItem value="QUEUE_SIZE">Queue Size Based</SelectItem>
                      <SelectItem value="TIME_BASED">Time Based</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="minWorkers">Min Workers: {newGroup.minWorkers}</Label>
                  <Slider
                    value={[newGroup.minWorkers]}
                    onValueChange={([value]) => setNewGroup(prev => ({ ...prev, minWorkers: value }))}
                    max={newGroup.maxWorkers}
                    min={1}
                    step={1}
                  />
                </div>
                <div>
                  <Label htmlFor="maxWorkers">Max Workers: {newGroup.maxWorkers}</Label>
                  <Slider
                    value={[newGroup.maxWorkers]}
                    onValueChange={([value]) => setNewGroup(prev => ({ ...prev, maxWorkers: value }))}
                    max={50}
                    min={newGroup.minWorkers}
                    step={1}
                  />
                </div>
              </div>

              <div className="flex items-center space-x-2">
                <Switch
                  id="autoScaling"
                  checked={newGroup.autoScaling}
                  onCheckedChange={(checked) => setNewGroup(prev => ({ ...prev, autoScaling: checked }))}
                />
                <Label htmlFor="autoScaling">Enable Auto-scaling</Label>
              </div>

              <div className="flex justify-end space-x-2 pt-4">
                <Button variant="outline" onClick={() => setIsCreating(false)}>
                  Cancel
                </Button>
                <Button onClick={handleCreateGroup}>
                  Create Group
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
};#### Historia 4.3: Interactive Documentation System
**Como** Developer  
**Quiero** interactive documentation con live examples  
**Para** learn y experiment con APIs easily  

**INVEST**:
- **Independent**: Documentation system standalone
- **Negotiable**: Interactive API exploration
- **Valuable**: Enhanced developer onboarding
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core documentation features
- **Testable**: Documentation generation tests

**Acceptance Criteria**:
- [ ] Auto-generated API documentation
- [ ] Interactive code examples con live execution
- [ ] Search functionality con autocomplete
- [ ] Versioned documentation
- [ ] Code snippet generation
- [ ] Integration con IDE plugins

**Interactive Documentation Implementation**:
```rust
pub struct DocumentationGenerator {
    api_extractor: ApiExtractor,
    template_engine: TemplateEngine,
    code_example_generator: CodeExampleGenerator,
}

impl DocumentationGenerator {
    pub async fn generate_api_docs(&self, code_base: &CodeBase) -> Result<ApiDocumentation, Error> {
        // Extract API definitions
        let api_definitions = self.api_extractor.extract_apis(code_base).await?;
        
        // Generate documentation for each API
        let mut documentation = ApiDocumentation::new();
        
        for api_def in api_definitions {
            let doc = self.generate_api_documentation(&api_def).await?;
            documentation.add_api_doc(doc);
        }
        
        Ok(documentation)
    }
    
    async fn generate_api_documentation(&self, api: &ApiDefinition) -> Result<ApiDoc, Error> {
        let mut doc = ApiDoc::new(api.name.clone());
        
        // Add description
        doc.set_description(&api.description);
        
        // Add parameters documentation
        for param in &api.parameters {
            doc.add_parameter(ParameterDoc {
                name: param.name.clone(),
                type_name: param.rust_type.clone(),
                description: param.description.clone(),
                required: param.is_required,
                default_value: param.default_value.clone(),
            });
        }
        
        // Add return type documentation
        doc.set_return_type(ReturnTypeDoc {
            type_name: api.return_type.rust_type.clone(),
            description: api.return_type.description.clone(),
        });
        
        // Generate code examples
        let examples = self.generate_code_examples(api).await?;
        doc.set_examples(examples);
        
        Ok(doc)
    }
    
    async fn generate_code_examples(&self, api: &ApiDefinition) -> Result<Vec<CodeExample>, Error> {
        let mut examples = Vec::new();
        
        // Rust example
        examples.push(CodeExample {
            language: "rust".to_string(),
            title: "Basic Usage".to_string(),
            code: self.generate_rust_example(api).await?,
            description: "Simple example showing basic API usage".to_string(),
        });
        
        // Python example
        examples.push(CodeExample {
            language: "python".to_string(),
            title: "Python Client".to_string(),
            code: self.generate_python_example(api).await?,
            description: "Equivalent example using Python SDK".to_string(),
        });
        
        // JavaScript example
        examples.push(CodeExample {
            language: "javascript".to_string(),
            title: "Node.js Integration".to_string(),
            code: self.generate_js_example(api).await?,
            description: "Node.js example for server-side usage".to_string(),
        });
        
        Ok(examples)
    }
    
    async fn generate_rust_example(&self, api: &ApiDefinition) -> Result<String, Error> {
        let mut example = String::new();
        
        example.push_str("use cicd_client::CicdClient;\n");
        example.push_str("use tokio;\n\n");
        example.push_str("#[tokio::main]\n");
        example.push_str("async fn main() -> Result<(), Box<dyn std::error::Error>> {\n");
        example.push_str("    // Initialize client\n");
        example.push_str("    let client = CicdClient::new(\n");
        example.push_str("        \"https://api.cicd.example.com\",\n");
        example.push_str("        \"your-api-token\"\n");
        example.push_str("    );\n\n");
        
        // Generate specific example based on API type
        match api.name.as_str() {
            "create_pipeline" => {
                example.push_str("    // Create a new pipeline\n");
                example.push_str("    let pipeline_config = cicd_client::PipelineConfig {\n");
                example.push_str("        name: \"my-pipeline\".to_string(),\n");
                example.push_str("        stages: vec![\n");
                example.push_str("            cicd_client::StageConfig {\n");
                example.push_str("                name: \"build\".to_string(),\n");
                example.push_str("                image: \"rust:1.70\".to_string(),\n");
                example.push_str("                commands: vec![\"cargo build\".to_string()],\n");
                example.push_str("            }\n");
                example.push_str("        ],\n");
                example.push_str("    };\n\n");
                example.push_str("    let pipeline = client.create_pipeline(pipeline_config).await?;\n");
                example.push_str("    println!(\"Created pipeline: {}\", pipeline.name);\n");
            }
            "execute_pipeline" => {
                example.push_str("    // Execute existing pipeline\n");
                example.push_str("    let pipeline_id = \"pipeline-12345\";\n");
                example.push_str("    let job = client.execute_pipeline(pipeline_id).await?;\n");
                example.push_str("    println!(\"Executing pipeline: {}\", job.id);\n");
            }
            _ => {
                example.push_str("    // Example for ");
                example.push_str(&api.name);
                example.push_str("\n");
                example.push_str("    // Implementation depends on specific API\n");
            }
        }
        
        example.push_str("\n    Ok(())\n");
        example.push_str("}\n");
        
        Ok(example)
    }
}

pub struct InteractiveDocumentationServer {
    documentation_generator: DocumentationGenerator,
    web_server: WebServer,
    search_engine: SearchEngine,
}

impl InteractiveDocumentationServer {
    pub async fn start(&mut self) -> Result<(), Error> {
        // Generate initial documentation
        let docs = self.generate_documentation().await?;
        self.search_engine.index_documentation(&docs).await?;
        
        // Start web server
        self.web_server.start().await?;
        
        Ok(())
    }
    
    async fn handle_api_exploration(&self, request: ApiExplorationRequest) -> ApiExplorationResponse {
        match request.action {
            Action::GetApiList => {
                let apis = self.get_available_apis().await;
                ApiExplorationResponse::ApiList { apis }
            }
            Action::GetApiDetails { api_name } => {
                let details = self.get_api_details(&api_name).await;
                ApiExplorationResponse::ApiDetails { details }
            }
            Action::ExecuteExample { api_name, language, example_id } => {
                let result = self.execute_example(&api_name, &language, &example_id).await;
                ApiExplorationResponse::ExampleResult { result }
            }
            Action::Search { query } => {
                let results = self.search_engine.search(&query).await;
                ApiExplorationResponse::SearchResults { results }
            }
        }
    }
}---

### 🔧 Sprint 5: Code Generation & Testing Tools

#### Historia 5.1: Code Generation Templates
**Como** Software Engineer  
**Quiero** code generation templates para common patterns  
**Para** accelerate development con automated code scaffolding  

**INVEST**:
- **Independent**: Code generation standalone system
- **Negotiable**: Extensible template engine
- **Valuable**: Development velocity improvement
- **Estimable**: 8 puntos de historia
- **Small**: Enfoque en core generation logic
- **Testable**: Template generation tests

**Acceptance Criteria**:
- [ ] Pipeline template generation
- [ ] Worker provider template generation
- [ ] Configuration file templates
- [ ] Test fixture generation
- [ ] Custom template support
- [ ] Template customization interface

**Code Generation Implementation**:
```rust
pub struct CodeGenerator {
    template_engine: TemplateEngine,
    dependency_resolver: DependencyResolver,
    project_analyzer: ProjectAnalyzer,
}

impl CodeGenerator {
    pub fn generate_pipeline_template(&self, config: PipelineTemplateConfig) -> GeneratedPipeline {
        let template = self.template_engine.load_template("pipeline")
            .expect("Pipeline template not found");
            
        let mut context = TemplateContext::new();
        context.insert("pipeline_name", &config.name);
        context.insert("description", &config.description);
        context.insert("stages", &config.stages);
        context.insert("environment", &config.environment);
        context.insert("triggers", &config.triggers);
        
        let generated_code = template.render(&context)
            .expect("Failed to render pipeline template");
            
        let dependencies = self.dependency_resolver.resolve_pipeline_dependencies(&config);
        
        GeneratedPipeline {
            files: self.generate_pipeline_files(&generated_code, &dependencies),
            dependencies,
            metadata: PipelineMetadata {
                template_version: "1.0".to_string(),
                generated_at: Utc::now(),
                original_config: config,
            },
        }
    }
    
    pub fn generate_worker_provider_template(&self, config: WorkerProviderConfig) -> GeneratedWorkerProvider {
        let template = self.template_engine.load_template("worker_provider")
            .expect("Worker provider template not found");
            
        let mut context = TemplateContext::new();
        context.insert("provider_name", &config.name);
        context.insert("provider_type", &config.provider_type);
        context.insert("capabilities", &config.capabilities);
        context.insert("configuration_schema", &config.config_schema);
        
        let generated_code = template.render(&context)
            .expect("Failed to render worker provider template");
            
        let files = self.generate_worker_provider_files(&generated_code, &config);
        
        GeneratedWorkerProvider {
            files,
            config,
            generated_at: Utc::now(),
        }
    }
    
    fn generate_pipeline_files(&self, code: &str, dependencies: &[Dependency]) -> Vec<GeneratedFile> {
        let mut files = Vec::new();
        
        // Main pipeline file
        files.push(GeneratedFile {
            path: format!("{}/pipeline.yml", self.get_pipeline_path()),
            content: code.to_string(),
            file_type: FileType::Yaml,
        });
        
        // Cargo.toml if needed
        if dependencies.iter().any(|d| d.language == "rust") {
            files.push(self.generate_cargo_toml(dependencies));
        }
        
        // Package.json if needed
        if dependencies.iter().any(|d| d.language == "node") {
            files.push(self.generate_package_json(dependencies));
        }
        
        // Test files
        files.push(self.generate_pipeline_tests());
        
        files
    }
}

#[derive(Debug)]
pub struct PipelineTemplateConfig {
    pub name: String,
    pub description: String,
    pub stages: Vec<StageConfig>,
    pub environment: EnvironmentConfig,
    pub triggers: Vec<TriggerConfig>,
}

#[derive(Debug)]
pub struct StageConfig {
    pub name: String,
    pub image: String,
    pub commands: Vec<String>,
    pub dependencies: Vec<String>,
    pub environment: HashMap<String, String>,
    pub resources: ResourceRequirements,
}

impl PipelineTemplateConfig {
    pub fn new() -> Self {
        Self {
            name: "my-pipeline".to_string(),
            description: "Auto-generated pipeline".to_string(),
            stages: Vec::new(),
            environment: EnvironmentConfig::default(),
            triggers: Vec::new(),
        }
    }
    
    pub fn add_stage(mut self, stage: StageConfig) -> Self {
        self.stages.push(stage);
        self
    }
    
    pub fn with_triggers(mut self, triggers: Vec<TriggerConfig>) -> Self {
        self.triggers = triggers;
        self
    }
}#### Historia 5.2: Testing Framework Integration
**Como** QA Engineer  
**Quiero** automated testing integration con CI/CD  
**Para** ensure quality en every deployment  

**INVEST**:
- **Independent**: Testing framework integration standalone
- **Negotiable**: Multi-framework support
- **Valuable**: Quality assurance automation
- **Estimable**: 5 puntos de historia
- **Small**: Enfoque en core testing integration
- **Testable**: Framework integration tests

**Acceptance Criteria**:
- [ ] JUnit integration para Java testing
- [ ] Pytest integration para Python testing
- [ ] Jest integration para JavaScript testing
- [ ] Cargo test integration para Rust testing
- [ ] Test result aggregation y reporting
- [ ] Coverage tracking y threshold enforcement

**Testing Framework Implementation**:
```rust
pub struct TestingFramework {
    test_runner: TestRunner,
    result_aggregator: TestResultAggregator,
    coverage_tracker: CoverageTracker,
    reporter: TestReporter,
}

impl TestingFramework {
    pub async fn run_pipeline_tests(&self, pipeline_config: &PipelineConfig) -> Result<TestResults, Error> {
        let mut all_results = Vec::new();
        
        for stage in &pipeline_config.stages {
            // Check if stage has test commands
            if stage.commands.iter().any(|cmd| cmd.contains("test")) {
                let results = self.run_stage_tests(stage).await?;
                all_results.extend(results);
            }
        }
        
        // Aggregate results
        let aggregated_results = self.result_aggregator.aggregate(all_results);
        
        // Generate coverage report
        let coverage_report = self.coverage_tracker.generate_report(&aggregated_results).await?;
        
        // Generate test report
        let test_report = self.reporter.generate_report(&aggregated_results, &coverage_report)?;
        
        Ok(TestResults {
            passed: aggregated_results.passed,
            failed: aggregated_results.failed,
            skipped: aggregated_results.skipped,
            coverage: coverage_report,
            report: test_report,
        })
    }
    
    async fn run_stage_tests(&self, stage: &StageConfig) -> Result<Vec<TestResult>, Error> {
        match self.detect_test_framework(&stage.commands) {
            TestFramework::JUnit => self.run_junit_tests(stage).await,
            TestFramework::Pytest => self.run_pytest_tests(stage).await,
            TestFramework::Jest => self.run_jest_tests(stage).await,
            TestFramework::CargoTest => self.run_cargo_tests(stage).await,
            TestFramework::Custom => self.run_custom_tests(stage).await,
        }
    }
}

pub enum TestFramework {
    JUnit,
    Pytest,
    Jest,
    CargoTest,
    Custom,
}

impl TestingFramework {
    fn detect_test_framework(&self, commands: &[String]) -> TestFramework {
        for cmd in commands {
            if cmd.contains("mvn test") || cmd.contains("gradle test") || cmd.contains("mvn test") {
                return TestFramework::JUnit;
            }
            if cmd.contains("pytest") || cmd.contains("python -m pytest") {
                return TestFramework::Pytest;
            }
            if cmd.contains("npm test") || cmd.contains("jest") {
                return TestFramework::Jest;
            }
            if cmd.contains("cargo test") {
                return TestFramework::CargoTest;
            }
        }
        TestFramework::Custom
    }
    
    async fn run_junit_tests(&self, stage: &StageConfig) -> Result<Vec<TestResult>, Error> {
        let test_command = self.extract_junit_command(stage);
        
        // Run tests in container
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&test_command)
            .output()
            .await?;
            
        // Parse JUnit XML output
        self.parse_junit_xml(&String::from_utf8_lossy(&output.stdout))
    }
    
    async fn run_cargo_tests(&self, stage: &StageConfig) -> Result<Vec<TestResult>, Error> {
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.arg("test")
           .arg("--")
           .arg("--format")
           .arg("json");
        
        // Add any additional cargo test arguments from stage
        for arg in &stage.commands {
            if arg.starts_with("--") {
                let parts: Vec<&str> = arg.split_whitespace().collect();
                cmd.args(&parts[1..]);
            }
        }
        
        let output = cmd.output().await?;
        
        // Parse cargo test JSON output
        self.parse_cargo_test_json(&String::from_utf8_lossy(&output.stdout))
    }
}

pub struct TestResults {
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub coverage: CoverageReport,
    pub report: TestReport,
}

#[derive(Debug)]
pub struct CoverageReport {
    pub lines_covered: u32,
    pub lines_total: u32,
    pub branches_covered: u32,
    pub branches_total: u32,
    pub functions_covered: u32,
    pub functions_total: u32,
    pub percentage: f64,
}

impl CoverageReport {
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.percentage >= threshold
    }
}
```

---

## 📅 Sprint Planning Summary

| Sprint | Historias | Puntos | Dependencias | Deliverables |
|--------|-----------|--------|--------------|--------------|
| **Sprint 1** | 1.1, 1.2, 1.3 | 24 | Épica 5 (CLI) | Multi-Language SDKs |
| **Sprint 2** | 2.1, 2.2, 2.3 | 18 | Sprint 1 | Advanced CLI Tools |
| **Sprint 3** | 3.1, 3.2 | 16 | Sprint 1-2 | IDE Integration |
| **Sprint 4** | 4.1, 4.2, 4.3 | 29 | Sprint 1-3 | Developer Portal |
| **Sprint 5** | 5.1, 5.2 | 13 | Sprint 1-4 | Code Generation & Testing |
| **Total** | **11 historias** | **100 puntos** | | **Complete Developer Experience Stack** |

---

## 🏗️ Arquitectura Hexagonal Implementation

### Core Domain (Ports)
```rust
// Core Port: SDK Management
pub trait SdkManagerPort {
    fn create_client() -> Result<Client, Error>;
    fn register_sdk() -> Result<(), Error>;
    fn get_sdk_version() -> Result<String, Error>;
    fn update_sdk() -> Result<(), Error>;
}

// Core Port: CLI Operations
pub trait CliOperationsPort {
    fn execute_command() -> Result<Output, Error>;
    fn validate_config() -> Result<ValidationResult, Error>;
    fn generate_completions() -> Result<String, Error>;
}

// Core Port: IDE Integration
pub trait IdeIntegrationPort {
    fn connect_ide() -> Result<Connection, Error>;
    fn sync_configuration() -> Result<(), Error>;
    fn execute_integration_command() -> Result<ExecutionResult, Error>;
}
```

### Infrastructure Adapters
```rust
// Rust SDK Adapter
pub struct RustSdkAdapter {
    http_client: reqwest::Client,
    auth_token: String,
    base_url: String,
}

impl SdkManagerPort for RustSdkAdapter {
    fn create_client() -> Result<Client, Error> {
        let client = CicdClient::new(&self.base_url, &self.auth_token);
        Ok(Client::Rust(client))
    }
    
    fn register_sdk() -> Result<(), Error> {
        // Registration logic
        Ok(())
    }
}

// VS Code Extension Adapter
pub struct VsCodeExtensionAdapter {
    extension_context: ExtensionContext,
    language_client: LanguageClient,
}

impl IdeIntegrationPort for VsCodeExtensionAdapter {
    fn connect_ide() -> Result<Connection, Error> {
        // VS Code connection logic
        Ok(Connection::new())
    }
    
    fn sync_configuration() -> Result<(), Error> {
        // Configuration sync logic
        Ok(())
    }
}
```

---

## 🔗 Referencias Técnicas

### Épica 1 - Core Platform & Infrastructure
- **Relevancia**: SDK communication protocols, pipeline management APIs
- **Referencias**: API endpoints, authentication flows, pipeline lifecycle

### Épica 4 - Worker Management & Abstraction  
- **Relevancia**: Worker SDK integration, provider abstraction
- **Referencias**: Worker lifecycle, scaling APIs, provider interfaces

### Épica 5 - Observability & Operations
- **Relevancia**: SDK metrics, CLI monitoring integration
- **Referencias**: Metrics collection, alerting APIs, dashboard data

### Stage 4 - Messaging & Communication
- **Relevancia**: WebSocket integration para real-time updates
- **References**: Message protocols, real-time communication patterns

### Stage 7 - Worker Management
- **Relevancia**: CLI worker management, portal worker controls
- **References**: Worker provisioning, scaling automation, provider integrations

---

## 📊 Success Metrics

### Developer Experience Coverage
- **SDK Language Support**: 4 languages (Rust, Python, JS/TS, Go)
- **IDE Integration**: 3 major IDEs (VS Code, IntelliJ, Visual Studio)
- **CLI Completeness**: 95% feature parity con web portal
- **Documentation Coverage**: 100% API documentation con examples

### Performance Targets
- **SDK Response Time**: < 500ms para standard operations
- **CLI Command Execution**: < 2s para complex operations
- **IDE Plugin Response**: < 1s para code completion
- **Portal Loading Time**: < 3s para dashboard pages

### Developer Productivity Metrics
- **Onboarding Time**: < 30 minutes to first pipeline execution
- **Configuration Time**: < 5 minutes para basic pipeline setup
- **Debugging Efficiency**: 50% faster issue identification
- **Development Velocity**: 40% faster pipeline development

### Integration Quality
- **IDE Plugin Stability**: 99.5% uptime
- **SDK Compatibility**: 99% backward compatibility
- **CLI Error Rate**: < 1% command failure rate
- **Portal User Satisfaction**: > 4.5/5 rating

---

**Conventional Commit Épica 6**: `feat(developer-experience): implement comprehensive developer ecosystem with multi-language SDKs, advanced CLI tools, IDE integrations, self-service portal, and code generation capabilities`
