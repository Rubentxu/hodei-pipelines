# Integration Tests Framework

This directory contains comprehensive integration tests for the Hodei Jobs system, validating end-to-end behavior with real infrastructure components.

## 🎯 Overview

The integration test framework validates:
- **PostgreSQL**: Database connectivity, schema migration, CRUD operations
- **NATS JetStream**: Event publishing, subscribing, serialization
- **Job Lifecycle**: Complete job lifecycle from creation to completion
- **Pipeline Execution**: Sequential and parallel pipeline execution
- **Worker Assignment**: Job scheduling and worker matching

## 🚀 Running Tests

### Prerequisites

The integration tests require external services to be running:

```bash
# Start PostgreSQL
docker run -d --name postgres-test \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_DB=postgres \
  -p 5432:5432 \
  postgres:16

# Start NATS
docker run -d --name nats-test \
  -p 4222:4222 \
  -p 8222:8222 \
  nats:2.10-alpine
```

### Running Tests

```bash
# Run all integration tests
cargo test -p integration-tests

# Run specific test category
cargo test -p integration-tests basic_postgres
cargo test -p integration-tests nats_integration
cargo test -p integration-tests job_lifecycle
cargo test -p integration-tests pipeline_execution

# Run with verbose output
cargo test -p integration-tests -- --nocapture

# Run tests that don't require external services
cargo test -p integration-tests -- --ignored
```

## 📋 Test Suites

### 1. PostgreSQL Tests (`basic_postgres_test.rs`)

Validates PostgreSQL connectivity and basic operations.

**Tests**:
- ✅ Database connection establishment
- ✅ Schema initialization
- ✅ Job CRUD operations (Create, Read, Update, Delete)
- ✅ Job listing and filtering

**Requirements**: PostgreSQL running on localhost:5432

### 2. Redb Tests (`redb_repository_test.rs`)

Validates Redb (embedded database) operations without external dependencies.

**Tests**:
- ✅ Job save and retrieve
- ✅ Compare-and-swap operations
- ✅ Job deletion
- ✅ Pending and running jobs filtering
- ✅ State transitions

**Requirements**: None (uses temporary directories)

### 3. NATS Event Bus Tests (`nats_integration_test.rs`)

Validates NATS event publishing and receiving.

**Tests**:
- ✅ Publish and subscribe to events
- ✅ Event serialization/deserialization
- ✅ Multiple subscribers (fan-out)
- ✅ Consumer groups (work queue pattern)

**Requirements**: NATS running on localhost:4222

### 4. Job Lifecycle Tests (`job_lifecycle_integration_test.rs`)

Validates complete job lifecycle from creation to completion.

**Tests**:
- ✅ **Complete Lifecycle**: PENDING → SCHEDULED → RUNNING → SUCCESS
- ✅ **Failure Handling**: RUNNING → FAILED
- ✅ **Timeout Handling**: RUNNING → TIMEOUT
- ✅ **Retry Mechanism**: FAILED → PENDING (retry) → SUCCESS
- ✅ Event publication at each lifecycle stage

**Requirements**: PostgreSQL and NATS

### 5. Pipeline Execution Tests (`pipeline_execution_integration_test.rs`)

Validates pipeline creation and execution with DAG dependencies.

**Tests**:
- ✅ **Sequential Pipeline**: A → B → C
- ✅ **Parallel Pipeline**: A → {B, C} → D (B and C execute in parallel)
- ✅ **Error Handling**: Step failure stops dependent steps
- ✅ **DAG Validation**: No cycles, valid dependencies

**Requirements**: PostgreSQL and NATS

## 🏗️ Architecture

### Test Structure

```
tests/
├── basic_postgres_test.rs                 # PostgreSQL connectivity
├── redb_repository_test.rs               # Redb operations
├── nats_integration_test.rs              # Event bus
├── job_lifecycle_integration_test.rs     # Job lifecycle
└── pipeline_execution_integration_test.rs # Pipeline execution
```

### Test Organization

Each test suite is structured as:

1. **Setup**: Initialize infrastructure connections
2. **Execute**: Run the operation(s) being tested
3. **Verify**: Assert expected outcomes
4. **Cleanup**: Reset state (automatic via temp dirs for Redb)

### Shared Components

```rust
// Helper for connecting to test infrastructure
struct TestSetup {
    job_repo: PostgreSqlJobRepository,
    nats_bus: NatsBus,
    // ... other components
}
```

## 🔧 Configuration

Tests automatically detect if required services are available and skip gracefully if not:

```rust
for i in 0..30 {
    match sqlx::PgPool::connect(connection_string).await {
        Ok(pool) => {
            // Tests run
            break;
        }
        Err(e) => {
            if i < 29 {
                sleep(Duration::from_secs(1)).await;
            } else {
                info!("✅ Test skipped - Service not available");
                return;
            }
        }
    }
}
```

This allows tests to:
- Run locally with real services
- Skip gracefully in CI/CD without services
- Provide clear feedback about missing dependencies

## 📊 Test Coverage

| Component | Unit Tests | Integration Tests | Coverage Target |
|-----------|------------|-------------------|-----------------|
| Job Repository | ✅ | ✅ PostgreSQL + Redb | ≥90% |
| Event Bus | ✅ | ✅ NATS | ≥90% |
| Pipeline Execution | ✅ | ✅ E2E | ≥90% |
| Worker Matching | ✅ | - | ≥90% |

## 🎓 Best Practices

### 1. Idempotent Tests
Each test can run independently without affecting others:
- Use unique IDs for test data
- Clean up after execution (or use temp dirs)
- No hardcoded assumptions about state

### 2. Clear Assertions
Use descriptive assertions with context:

```rust
assert_eq!(retrieved_job.id, job_id, "Job ID should match");
assert_eq!(retrieved_job.name, "Test Job", "Job name should match");
```

### 3. Descriptive Logging
Log each step of the test for debugging:

```rust
info!("✅ Job saved successfully");
info!("✅ Job retrieved and verified");
```

### 4. Graceful Skipping
Detect unavailable services and skip tests gracefully:

```rust
if !services_available {
    info!("✅ Test skipped - Infrastructure not available");
    return;
}
```

## 🚨 Troubleshooting

### PostgreSQL Connection Issues

```bash
# Check if PostgreSQL is running
docker ps | grep postgres

# Check logs
docker logs postgres-test

# Reset PostgreSQL
docker stop postgres-test && docker rm postgres-test
```

### NATS Connection Issues

```bash
# Check if NATS is running
docker ps | grep nats

# Check logs
docker logs nats-test

# Reset NATS
docker stop nats-test && docker rm nats-test
```

### Test Hanging

If tests hang:
1. Check service availability
2. Look for connection timeouts
3. Verify no stale connections
4. Restart test containers

## 📈 CI/CD Integration

Tests are designed to run in CI/CD:

```yaml
# Example GitHub Actions
- name: Run Integration Tests
  run: |
    docker run -d --name postgres -p 5432:5432 postgres:16
    docker run -d --name nats -p 4222:4222 nats:2.10-alpine
    sleep 5  # Wait for services to start
    cargo test -p integration-tests
```

Tests will automatically:
- Skip if services unavailable
- Run if services are provided
- Report clear status for debugging

## 🎉 Success Criteria

✅ All integration tests pass when run with:
- PostgreSQL 16+
- NATS 2.10+
- Rust 1.75+

✅ Tests demonstrate:
- Complete job lifecycle working end-to-end
- Pipeline execution with proper DAG handling
- Event bus delivering events reliably
- Database operations working correctly

✅ No breaking changes to production code

## 📚 Additional Resources

- [PostgreSQL Docker Hub](https://hub.docker.com/_/postgres)
- [NATS Docker Hub](https://hub.docker.com/_/nats)
- [TestContainers Documentation](https://docs.rs/testcontainers/)
- [Async NATS Client](https://docs.rs/async-nats/)
- [SQLx Documentation](https://docs.rs/sqlx/)

---

**Note**: These tests validate real infrastructure behavior. For unit tests without external dependencies, see individual crate test modules.
