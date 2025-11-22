# Hodei Jobs Platform - Quick Start Guide

## 🎯 What Was Implemented

**Option B** has been successfully implemented with:
- ✅ Complete, real HTTP servers for all services
- ✅ OpenAPI 3.0 specifications
- ✅ Swagger UI interface
- ✅ Docker containerization
- ✅ Docker Compose orchestration
- ✅ E2E testing framework
- ✅ Observability stack (Prometheus, Jaeger)

## 🚀 Quick Start (Development)

### Option 1: Using Scripts
```bash
# Start all services
./scripts/start-services.sh

# Test services
curl http://localhost:8080/health
curl http://localhost:8081/health
curl http://localhost:8082/health

# Stop all services
./scripts/stop-services.sh
```

### Option 2: Manual
```bash
# Build services
cargo build --bin orchestrator --bin scheduler --bin worker-manager

# Start each service
./target/debug/orchestrator &
./target/debug/scheduler &
./target/debug/worker-manager &
```

## 🐳 Quick Start (Docker Compose)

```bash
# Start all services and infrastructure
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f

# Stop all services
docker-compose down
```

## 🌐 Service URLs

| Service | Port | URL | Swagger UI |
|---------|------|-----|------------|
| Orchestrator | 8080 | http://localhost:8080 | http://localhost:8080/swagger-ui |
| Scheduler | 8081 | http://localhost:8081 | http://localhost:8081/openapi.json |
| Worker Manager | 8082 | http://localhost:8082 | http://localhost:8082/openapi.json |

## 📡 Test APIs

### Create Pipeline
```bash
curl -X POST http://localhost:8080/api/v1/pipelines \
  -H "Content-Type: application/json" \
  -d '{"name":"my-pipeline","description":"Test"}'
```

### Create Job
```bash
curl -X POST http://localhost:8080/api/v1/jobs \
  -H "Content-Type: application/json" \
  -d '{"pipeline_id":"<pipeline-id>"}'
```

### Register Worker
```bash
curl -X POST http://localhost:8081/api/v1/workers \
  -H "Content-Type: application/json" \
  -d '{"type":"rust"}'
```

### Start Worker
```bash
curl -X POST http://localhost:8082/api/v1/workers \
  -H "Content-Type: application/json" \
  -d '{"type":"rust"}'
```

## 📊 Observability

- **Prometheus**: http://localhost:9090
- **Jaeger**: http://localhost:16686
- **PostgreSQL**: localhost:5432 (postgres/postgres)

## 📁 Key Files

- `docker-compose.yml` - Full system orchestration
- `monitoring/prometheus/prometheus.yml` - Metrics configuration
- `crates/orchestration/orchestrator/src/main.rs` - Orchestrator HTTP server
- `crates/scheduler/src/main.rs` - Scheduler HTTP server
- `crates/worker-lifecycle-manager/src/main.rs` - Worker Manager HTTP server
- `E2E_TESTING_SUMMARY.md` - Detailed implementation documentation

## 🧪 E2E Tests

```bash
# Run basic integration tests
cargo test --package e2e-tests basic_integration --lib
```

## ✅ Verification

All services are currently running and tested:

```bash
# Health checks
curl http://localhost:8080/health  # ✅ Orchestrator
curl http://localhost:8081/health  # ✅ Scheduler
curl http://localhost:8082/health  # ✅ Worker Manager

# Test full workflow
curl -X POST http://localhost:8080/api/v1/pipelines \
  -d '{"name":"test","description":"demo"}'
```

## 📚 Documentation

See `E2E_TESTING_SUMMARY.md` for complete implementation details.

---

**Status**: ✅ All services operational
**Date**: November 22, 2025
