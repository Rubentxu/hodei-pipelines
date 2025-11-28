# E2E Testing Framework

This directory contains the comprehensive end-to-end testing framework for the hodei-pipelines platform.

## Architecture

The E2E testing framework is built with the following components:

### Core Modules

- **Infrastructure**: Container orchestration, service management, observability
  - `containers.rs`: Docker container management (NATS, PostgreSQL, Prometheus)
  - `services.rs`: Application service handles (Orchestrator, Scheduler, Worker Manager)
  - `observability.rs`: Metrics and tracing collection
  - `config.rs`: Test configuration management

- **Scenarios**: Pre-defined test scenarios
  - `mod.rs`: Test scenario traits and implementations
  - Happy Path: Validates complete workflow
  - Error Handling: Tests error scenarios
  - Performance: Tests system under load

- **Helpers**: Reusable utilities
  - `mod.rs`: Test data generators, assertion helpers, HTTP utilities, logging

- **Fixtures**: Pre-defined test data and configurations
  - `mod.rs`: Standard pipelines, workers, Prometheus configs, Docker Compose

### Test Suites

1. **Basic Integration** (`tests/integration/basic_integration.rs`)
   - Environment initialization
   - Service health checks
   - Metrics/tracing collection

2. **API Endpoints** (`tests/api/api_endpoints.rs`)
   - Pipeline creation/listing
   - Job creation/status retrieval
   - Invalid request handling
   - Health checks

3. **SDK Integration** (`tests/sdk/sdk_integration.rs`)
   - Rust SDK client
   - Fluent builder pattern
   - Pipeline/job operations

4. **Full System** (`tests/integration/full_system.rs`)
   - Complete job lifecycle
   - Concurrent job execution
   - Scenario runner
   - Observability validation
   - Error recovery

## Configuration

Environment variables for configuration:

```bash
TEST_ORCHESTRATOR_PORT=8080
TEST_SCHEDULER_PORT=8081
TEST_WORKER_MANAGER_PORT=8082
TEST_PROMETHEUS_PORT=9090
TEST_JAEGER_PORT=16686
TEST_POSTGRES_PORT=5432
TEST_NATS_PORT=4222
TEST_TIMEOUT_SECS=300
TEST_MAX_RETRIES=3
TEST_LOG_LEVEL=info
```

## Usage

### Running All E2E Tests

```bash
# Run all tests
cargo test --package e2e-tests --all-features

# Run specific test type
cargo test --package e2e-tests basic_integration
cargo test --package e2e-tests api_endpoints
cargo test --package e2e-tests full_system

# Run with output directory
TEST_OUTPUT_DIR=./target/e2e-results cargo test
```

### Running with Test Environment

```rust
use e2e_tests::infrastructure::TestEnvironment;

#[tokio::test]
async fn my_e2e_test() -> Result<()> {
    let env = TestEnvironment::new().await?;
    
    // Use the environment
    let client = env.orchestrator_client().await?;
    
    // Your test logic here
    
    env.cleanup().await?;
    Ok(())
}
```

### Running Test Scenarios

```rust
use e2e_tests::scenarios::{ScenarioRunner, HappyPathScenario};

#[tokio::test]
async fn test_scenario() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let mut runner = ScenarioRunner::new();
    
    runner.add_scenario(HappyPathScenario::new());
    let results = runner.run_all(&env).await?;
    
    for result in results {
        println!("{}: {}", result.name, if result.passed { "PASS" } else { "FAIL" });
    }
    
    env.cleanup().await?;
    Ok(())
}
```

### Custom Test Data

```rust
use e2e_tests::helpers::TestDataGenerator;

let generator = TestDataGenerator::new();
let pipeline = generator.create_pipeline();
let job = generator.create_job();
let worker = generator.create_worker("rust");
```

## Infrastructure

The test environment uses:

1. **Docker Containers** (via Testcontainers)
   - NATS JetStream (message broker)
   - PostgreSQL (database)
   - Prometheus (metrics)

2. **Application Services**
   - Orchestrator (pipeline/job management)
   - Scheduler (job scheduling)
   - Worker Manager (worker lifecycle)

3. **Observability Stack**
   - Metrics collection via Prometheus
   - Distributed tracing via OpenTelemetry/Jaeger
   - Structured logging

## Best Practices

1. **Test Isolation**: Each test creates its own environment
2. **Cleanup**: Always call `env.cleanup()` after tests
3. **Timeouts**: Tests should complete within `TEST_TIMEOUT_SECS`
4. **Logging**: Use `logging::log_test_step()` for test progress
5. **Assertions**: Use helper functions from `assertions` module
6. **Idempotency**: Tests should be able to run multiple times

## Troubleshooting

### Container Startup Failures

If Docker containers fail to start:
```bash
# Check Docker daemon
docker ps

# Check available disk space
df -h

# Check memory
free -h
```

### Service Health Issues

If services don't become healthy:
```bash
# Increase timeout
TEST_TIMEOUT_SECS=600 cargo test
```

### Port Conflicts

If ports are already in use:
```bash
# Use different ports
TEST_ORCHESTRATOR_PORT=9080 TEST_SCHEDULER_PORT=9081 cargo test
```

## Future Enhancements

- [ ] Chaos engineering tests (killing services mid-execution)
- [ ] Performance benchmarking
- [ ] Long-running soak tests
- [ ] Multi-node cluster testing
- [ ] Real-world load testing scenarios
- [ ] Cross-browser testing (for web UI)
- [ ] Security testing (authentication, authorization)
- [ ] Disaster recovery testing

## Contributing

When adding new tests:

1. Follow the existing structure
2. Use appropriate test categories (integration, api, sdk)
3. Add proper cleanup
4. Include assertions for verification
5. Document complex test scenarios
6. Ensure tests are idempotent

## License

MIT
