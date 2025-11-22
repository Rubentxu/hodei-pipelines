# Hodei Rust SDK

Official Rust SDK for the Hodei CI/CD platform.

[![Crates.io](https://img.shields.io/crates/v/hodei-rust-sdk.svg)](https://crates.io/crates/hodei-rust-sdk)
[![Documentation](https://docs.rs/hodei-rust-sdk/badge.svg)](https://docs.rs/hodei-rust-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🦀 **Idiomatic Rust** with full type safety
- ⚡ **Async/await** support with Tokio
- 🏗️ **Fluent builder pattern** for pipeline configurations
- 🔒 **Type-safe** API interactions
- 🧪 **Comprehensive testing** with >90% coverage
- 📝 **Complete documentation** with examples

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hodei-rust-sdk = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

```rust
use hodei_rust_sdk::{CicdClient, PipelineBuilder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = CicdClient::new("https://api.hodei.example.com", "your-token")?;
    
    // Build pipeline
    let pipeline = PipelineBuilder::new("my-pipeline")
        .description("Build and test Rust project")
        .stage("build", "rust:1.70", vec!["cargo build"])
        .stage("test", "rust:1.70", vec!["cargo test"])
        .trigger_on_push("main", "https://github.com/user/repo")
        .build();
    
    // Create and execute
    let created = client.create_pipeline(pipeline).await?;
    let job = client.execute_pipeline(&created.id).await?;
    
    // Wait for completion
    let completed = client.wait_for_completion(&job.id, Duration::from_secs(5)).await?;
    println!("Job status: {:?}", completed.status);
    
    Ok(())
}
```

## Usage Examples

### Creating a Pipeline

```rust
use hodei_rust_sdk::PipelineBuilder;
use std::collections::HashMap;

let pipeline = PipelineBuilder::new("rust-build-pipeline")
    .description("Build and test Rust project")
    .env("RUST_LOG", "info")
    .env("CARGO_TERM_COLOR", "always")
    .stage("build", "rust:1.70", vec!["cargo build --release"])
    .stage_with_resources(
        "test",
        "rust:1.70",
        vec!["cargo test"],
        2.0,  // 2 CPU cores
        2048, // 2GB RAM
    )
    .stage_with_deps(
        "deploy",
        "alpine:latest",
        vec!["./deploy.sh"],
        vec!["build", "test"],
    )
    .trigger_on_push("main", "https://github.com/user/repo")
    .trigger_on_schedule("0 2 * * *")
    .build();

let created = client.create_pipeline(pipeline).await?;
```

### Executing and Monitoring

```rust
// Execute pipeline
let job = client.execute_pipeline(&pipeline_id).await?;
println!("Job started: {}", job.id);

// Wait for completion
let completed = client.wait_for_completion(&job.id, Duration::from_secs(5)).await?;
println!("Job status: {:?}", completed.status);

// Get logs
if let Ok(logs) = client.get_job_logs(&job.id).await {
    println!("Logs: {}", logs);
}
```

### Managing Workers

```rust
// List all workers
let workers = client.list_workers().await?;
for worker in workers {
    println!("{}: {:?}", worker.name, worker.status);
    println!("  CPU: {:.1}%", worker.cpu_usage * 100.0);
    println!("  Memory: {:.1}%", worker.memory_usage * 100.0);
}

// Scale worker group
client.scale_workers("kubernetes-workers", 10).await?;
```

### Error Handling

```rust
use hodei_sdk_core::SdkError;

match client.get_pipeline("non-existent").await {
    Ok(pipeline) => println!("Found: {}", pipeline.name),
    Err(e) => {
        if e.is_not_found() {
            println!("Pipeline not found");
        } else if e.is_auth_error() {
            println!("Authentication failed");
        }
    }
}
```

## API Reference

### CicdClient

Main client for interacting with the Hodei CI/CD platform.

#### Constructor

```rust
pub fn new(base_url: impl Into<String>, api_token: impl Into<String>) -> SdkResult<Self>
pub fn with_config(config: ClientConfig) -> SdkResult<Self>
```

#### Methods

##### Pipeline Operations

```rust
pub async fn create_pipeline(&self, config: PipelineConfig) -> SdkResult<Pipeline>
pub async fn get_pipeline(&self, pipeline_id: &str) -> SdkResult<Pipeline>
pub async fn list_pipelines(&self) -> SdkResult<Vec<Pipeline>>
pub async fn execute_pipeline(&self, pipeline_id: &str) -> SdkResult<Job>
pub async fn delete_pipeline(&self, pipeline_id: &str) -> SdkResult<()>
```

##### Job Operations

```rust
pub async fn get_job_status(&self, job_id: &str) -> SdkResult<Job>
pub async fn get_job_logs(&self, job_id: &str) -> SdkResult<String>
pub async fn wait_for_completion(&self, job_id: &str, poll_interval: Duration) -> SdkResult<Job>
```

##### Worker Operations

```rust
pub async fn list_workers(&self) -> SdkResult<Vec<Worker>>
pub async fn get_worker(&self, worker_id: &str) -> SdkResult<Worker>
pub async fn scale_workers(&self, worker_group: &str, target_count: u32) -> SdkResult<()>
```

### PipelineBuilder

Fluent builder for creating pipeline configurations.

#### Constructor

```rust
pub fn new(name: impl Into<String>) -> Self
```

#### Methods

```rust
pub fn description(self, description: impl Into<String>) -> Self
pub fn stage(self, name: impl Into<String>, image: impl Into<String>, commands: Vec<impl Into<String>>) -> Self
pub fn stage_with_resources(self, name: impl Into<String>, image: impl Into<String>, commands: Vec<impl Into<String>>, cpu: f32, memory: u32) -> Self
pub fn stage_with_deps(self, name: impl Into<String>, image: impl Into<String>, commands: Vec<impl Into<String>>, dependencies: Vec<impl Into<String>>) -> Self
pub fn env(self, key: impl Into<String>, value: impl Into<String>) -> Self
pub fn envs(self, vars: HashMap<String, String>) -> Self
pub fn trigger_on_push(self, branch: impl Into<String>, repository: impl Into<String>) -> Self
pub fn trigger_on_schedule(self, cron: impl Into<String>) -> Self
pub fn trigger_manual(self) -> Self
pub fn build(self) -> PipelineConfig
```

## Testing

Run all tests:

```bash
cargo test -p hodei-rust-sdk
```

Run with output:

```bash
cargo test -p hodei-rust-sdk -- --nocapture
```

## Examples

See the [examples](examples/) directory for more usage examples:

- [basic_pipeline.rs](examples/basic_pipeline.rs) - Basic pipeline creation and execution
- More examples coming soon...

## License

MIT © Hodei Team

## Contributing

Contributions are welcome! Please read our [Contributing Guide](../../CONTRIBUTING.md) for details.

## Documentation

- 📖 API Docs: [docs.rs/hodei-rust-sdk](https://docs.rs/hodei-rust-sdk)
- 🌐 Platform Docs: [docs.hodei.example.com](https://docs.hodei.example.com)
- 💬 Discussions: [GitHub Discussions](https://github.com/Rubentxu/hodei-jobs/discussions)
