# Hodei Jobs Platform - Makefile
#
# This Makefile provides convenient commands for building, testing,
# and running the distributed job orchestration platform.
#
# Usage:
#   make help              # Show all available commands
#   make build             # Build all services
#   make test              # Run all tests
#   make test-e2e          # Run E2E tests
#   make start-services    # Start all services
#   make stop-services     # Stop all services
#   make clean             # Clean build artifacts

.PHONY: help build test test-e2e test-basic test-real test-all start-services stop-services clean fmt lint

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
SERVICES := orchestrator scheduler worker-manager
TEST_PACKAGE := --package e2e-tests

## 📚 Help: Show all available commands
help:
	@echo "Hodei Jobs Platform - Available Commands"
	@echo "=========================================="
	@echo ""
	@echo "🏗️  BUILD COMMANDS:"
	@echo "  make build              Build all services"
	@echo "  make build-<service>    Build specific service (orchestrator|scheduler|worker-manager)"
	@echo ""
	@echo "🧪 TEST COMMANDS:"
	@echo "  make test               Run all tests (unit + integration)"
	@echo "  make test-e2e           Run E2E tests only"
	@echo "  make test-basic         Run basic integration tests"
	@echo "  make test-real          Run real services tests"
	@echo "  make test-real-services Run services tests (starts services first)"
	@echo "  make test-log-streaming Run log streaming tests (starts worker-manager)"
	@echo "  make test-all           Run comprehensive test suite"
	@echo ""
	@echo "🚀 RUNTIME COMMANDS:"
	@echo "  make start-services     Start all services in background"
	@echo "  make stop-services      Stop all running services"
	@echo "  make status             Check service status"
	@echo ""
	@echo "🔧 UTILITY COMMANDS:"
	@echo "  make clean              Clean build artifacts"
	@echo "  make fmt                Format code"
	@echo "  make lint               Run lints (clippy)"
	@echo ""

## 🏗️ BUILD TARGETS

### Build all services
build:
	@echo "🏗️  Building all services..."
	$(CARGO) build --bin orchestrator
	$(CARGO) build --bin scheduler
	$(CARGO) build --bin worker-manager
	@echo "✅ All services built successfully"

### Build individual services
build-orchestrator:
	@echo "🏗️  Building Orchestrator..."
	$(CARGO) build --bin orchestrator
	@echo "✅ Orchestrator built"

build-scheduler:
	@echo "🏗️  Building Scheduler..."
	$(CARGO) build --bin scheduler
	@echo "✅ Scheduler built"

build-worker-manager:
	@echo "🏗️  Building Worker Manager..."
	$(CARGO) build --bin worker-manager
	@echo "✅ Worker Manager built"

## 🧪 TEST TARGETS

### Run all tests
test:
	@echo "🧪 Running all tests..."
	$(CARGO) test $(TEST_PACKAGE) --all-features

### Run E2E tests only
test-e2e:
	@echo "🧪 Running E2E tests..."
	$(CARGO) test $(TEST_PACKAGE) --all-features

### Run basic integration tests only
test-basic:
	@echo "🧪 Running basic integration tests..."
	$(CARGO) test $(TEST_PACKAGE) --test basic_integration --all-features

### Run real services tests only
test-real:
	@echo "🧪 Running real services tests..."
	$(CARGO) test $(TEST_PACKAGE) real_services_test --all-features

### Run log streaming E2E tests (requires worker-manager running on port 8082)
test-log-streaming:
	@echo "🧪 Running Log Streaming E2E tests..."
	@./scripts/test-log-streaming.sh

### Run real services tests that need running services
test-real-services:
	@echo "🧪 Running real services tests..."
	@echo "  → Building services..."
	$(CARGO) build --bin orchestrator --bin scheduler --bin worker-manager
	@echo "  → Starting all services..."
	@make start-services -s
	@echo "  → Waiting for services to be healthy..."
	@sleep 5
	@echo "  → Running real services tests..."
	$(CARGO) test $(TEST_PACKAGE) real_services_test --all-features
	@echo "  → Stopping services..."
	@make stop-services -s
	@echo "✅ Real services tests completed!"

### Run comprehensive test suite
test-all:
	@echo "🧪 Running comprehensive test suite..."
	@echo "  → Building services..."
	$(CARGO) build --bin orchestrator --bin scheduler --bin worker-manager
	@echo "  → Running E2E tests..."
	$(CARGO) test $(TEST_PACKAGE) --all-features
	@echo "✅ All tests completed successfully"

### Run specific test
test-%:
	@echo "🧪 Running test: $*"
	$(CARGO) test $(TEST_PACKAGE) $* --all-features

## 🚀 RUNTIME TARGETS

### Start all services in background
start-services:
	@echo "🚀 Starting all services..."
	@if [ ! -f "./target/debug/orchestrator" ]; then \
		echo "❌ Binaries not found. Run 'make build' first"; \
		exit 1; \
	fi
	@echo "📦 Starting Orchestrator on port 8080..."
	./target/debug/orchestrator > /tmp/orchestrator.log 2>&1 &
	@echo "📦 Starting Scheduler on port 8081..."
	./target/debug/scheduler > /tmp/scheduler.log 2>&1 &
	@echo "📦 Starting Worker Manager on port 8082..."
	./target/debug/worker-manager > /tmp/worker-manager.log 2>&1 &
	@echo "⏳ Waiting for services to start..."
	@sleep 3
	@echo "🔍 Checking service health..."
	@curl -s http://localhost:8080/health > /dev/null && echo "✅ Orchestrator: http://localhost:8080" || echo "❌ Orchestrator: Not responding"
	@curl -s http://localhost:8081/health > /dev/null && echo "✅ Scheduler: http://localhost:8081" || echo "❌ Scheduler: Not responding"
	@curl -s http://localhost:8082/health > /dev/null && echo "✅ Worker Manager: http://localhost:8082" || echo "❌ Worker Manager: Not responding"
	@echo "🎉 All services started!"

### Stop all running services
stop-services:
	@echo "🛑 Stopping all services..."
	@if pgrep -f orchestrator > /dev/null; then \
		echo "  → Stopping Orchestrator..."; \
		pkill -f orchestrator; \
	fi
	@if pgrep -f scheduler > /dev/null; then \
		echo "  → Stopping Scheduler..."; \
		pkill -f scheduler; \
	fi
	@if pgrep -f worker-manager > /dev/null; then \
		echo "  → Stopping Worker Manager..."; \
		pkill -f worker-manager; \
	fi
	@echo "✅ All services stopped"

### Check service status
status:
	@echo "📊 Service Status:"
	@echo "=================="
	@echo ""
	@echo "Orchestrator:"
	@curl -s http://localhost:8080/health 2>/dev/null | jq '.' || echo "  ❌ Not running"
	@echo ""
	@echo "Scheduler:"
	@curl -s http://localhost:8081/health 2>/dev/null | jq '.' || echo "  ❌ Not running"
	@echo ""
	@echo "Worker Manager:"
	@curl -s http://localhost:8082/health 2>/dev/null | jq '.' || echo "  ❌ Not running"

### View service logs
logs:
	@echo "📝 Service Logs:"
	@echo "================"
	@echo ""
	@echo "--- Orchestrator (last 20 lines) ---"
	@tail -20 /tmp/orchestrator.log 2>/dev/null || echo "No logs available"
	@echo ""
	@echo "--- Scheduler (last 20 lines) ---"
	@tail -20 /tmp/scheduler.log 2>/dev/null || echo "No logs available"
	@echo ""
	@echo "--- Worker Manager (last 20 lines) ---"
	@tail -20 /tmp/worker-manager.log 2>/dev/null || echo "No logs available"

## 🐳 DOCKER COMPOSE TARGETS

### Start with Docker Compose
docker-up:
	@echo "🐳 Starting all services with Docker Compose..."
	docker-compose up -d
	@echo "⏳ Waiting for services to be healthy..."
	@sleep 10
	@make status

### Stop Docker Compose services
docker-down:
	@echo "🐳 Stopping Docker Compose services..."
	docker-compose down
	@echo "✅ All services stopped"

### View Docker Compose logs
docker-logs:
	@echo "🐳 Docker Compose Logs:"
	docker-compose logs -f

## 🔧 UTILITY TARGETS

### Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	$(CARGO) clean
	@echo "✅ Clean completed"

### Format code
fmt:
	@echo "🎨 Formatting code..."
	$(CARGO) fmt --all
	@echo "✅ Code formatted"

### Run lints (clippy)
lint:
	@echo "🔍 Running lints..."
	$(CARGO) clippy --all-targets --all-features -- -D warnings
	@echo "✅ Lints passed"

### Check code
check:
	@echo "✅ Running cargo check..."
	$(CARGO) check --all-features
	@echo "✅ Check completed"

## 🔄 CI/CD TARGETS

### Run CI pipeline
ci:
	@echo "🔄 Running CI pipeline..."
	@echo "  → Formatting code..."
	$(CARGO) fmt --all -- --check
	@echo "  → Running lints..."
	$(CARGO) clippy --all-targets --all-features -- -D warnings
	@echo "  → Building..."
	$(CARGO) build --all-features
	@echo "  → Running tests..."
	$(CARGO) test $(TEST_PACKAGE) --all-features
	@echo "✅ CI pipeline completed successfully"

### Quick development cycle
dev:
	@echo "🔄 Development cycle..."
	@echo "  → Building..."
	$(CARGO) build --bin orchestrator --bin scheduler --bin worker-manager
	@echo "  → Running basic tests..."
	$(CARGO) test $(TEST_PACKAGE) basic_integration --all-features
	@echo "✅ Development cycle completed"

## 📊 METRICS TARGETS

### Show test coverage
coverage:
	@echo "📊 Test coverage not implemented yet"
	@echo "   (Run with: cargo tarpaulin --out html)"

### Show build size
build-size:
	@echo "📊 Build Size:"
	@ls -lh target/debug/orchestrator 2>/dev/null | awk '{print "  Orchestrator: " $$5}'
	@ls -lh target/debug/scheduler 2>/dev/null | awk '{print "  Scheduler: " $$5}'
	@ls -lh target/debug/worker-manager 2>/dev/null | awk '{print "  Worker Manager: " $$5}'

## 🎯 QUICK START TARGETS

### Complete setup and test
setup:
	@echo "🎯 Setting up complete environment..."
	@echo "  → Building services..."
	$(CARGO) build --bin orchestrator --bin scheduler --bin worker-manager
	@echo "  → Starting services..."
	./scripts/start-services.sh
	@echo "  → Running tests..."
	$(CARGO) test $(TEST_PACKAGE) --all-features
	@echo "✅ Setup completed!"
	@echo ""
	@echo "📊 Services are running:"
	@echo "   Orchestrator: http://localhost:8080"
	@echo "   Scheduler: http://localhost:8081"
	@echo "   Worker Manager: http://localhost:8082"
	@echo ""
	@echo "🔗 Quick access:"
	@echo "   Swagger UI: http://localhost:8080/swagger-ui"
	@echo ""
	@echo "🛑 To stop: make stop-services"

### Complete teardown
teardown:
	@echo "🧹 Tearing down environment..."
	@echo "  → Stopping services..."
	./scripts/stop-services.sh
	@echo "  → Cleaning artifacts..."
	$(CARGO) clean
	@echo "✅ Teardown completed!"
