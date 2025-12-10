# Makefile for Hodei Jobs Platform
# Provides convenient commands for development and CI/CD

.PHONY: help install-deps test test-coverage lint format check clean build docs

# Default target
help: ## Show this help message
	@echo "Hodei Jobs Platform - Development Commands"
	@echo ""
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Installation:"
	@echo "  install-deps    Install all development dependencies"
	@echo ""
	@echo "Testing:"
	@echo "  test           Run all tests"
	@echo "  test-coverage  Run tests with coverage report"
	@echo "  bench          Run benchmarks"
	@echo ""
	@echo "Code Quality:"
	@echo "  lint           Run clippy lints"
	@echo "  format         Format all code"
	@echo "  check          Check code formatting and linting"
	@echo "  audit          Run security audit"
	@echo ""
	@echo "Build & Docs:"
	@echo "  build          Build the project"
	@echo "  build-release  Build in release mode"
	@echo "  docs           Generate documentation"
	@echo "  serve-docs     Serve documentation locally"
	@echo ""
	@echo "CI/CD:"
	@echo "  ci-local       Run CI pipeline locally"
	@echo "  pre-commit     Run pre-commit hooks"

# Installation
install-deps: ## Install all development dependencies
	@echo "Installing Rust toolchain..."
	rustup update stable
	rustup component add clippy rustfmt rust-docs
	@echo "Installing cargo tools..."
	cargo install cargo-llvm-cov cargo-audit cargo-outdated
	@echo "Installing pre-commit..."
	@which pre-commit > /dev/null || pip install pre-commit
	@pre-commit install
	@echo "Dependencies installed successfully!"

# Testing
test: ## Run all tests
	@echo "Running all tests..."
	cargo test --workspace --verbose
	@echo "✓ All tests passed"

test-coverage: ## Run tests with coverage report
	@echo "Running tests with coverage..."
	cargo llvm-cov --workspace --html
	@echo "Coverage report generated in target/llvm-cov/html/"

bench: ## Run benchmarks
	@echo "Running benchmarks..."
	cargo bench --workspace

# Code Quality
lint: ## Run clippy lints
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "✓ No linting issues found"

format: ## Format all code
	@echo "Formatting code..."
	cargo fmt --all
	@echo "✓ Code formatted"

check: ## Check code formatting and linting
	@echo "Checking code formatting..."
	cargo fmt --all -- --check
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "✓ Code quality checks passed"

audit: ## Run security audit
	@echo "Running security audit..."
	cargo audit
	@echo "✓ Security audit passed"

# Build
build: ## Build the project
	@echo "Building project..."
	cargo build
	@echo "✓ Build successful"

build-release: ## Build in release mode
	@echo "Building project in release mode..."
	cargo build --release
	@echo "✓ Release build successful"

# Documentation
docs: ## Generate documentation
	@echo "Generating documentation..."
	cargo doc --workspace --no-deps
	@echo "✓ Documentation generated"

serve-docs: docs ## Serve documentation locally
	@echo "Serving documentation at http://localhost:3000"
	cargo doc --workspace --no-deps --open || true
	@python3 -m http.server 3000 -d target/doc 2>/dev/null || true

# Clean
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "✓ Cleaned"

clean-all: clean ## Clean all including cargo registry
	@echo "Cleaning all artifacts..."
	cargo clean
	rm -rf target/llvm-cov
	@echo "✓ Fully cleaned"

# CI/CD
ci-local: check audit test test-coverage ## Run CI pipeline locally
	@echo "✓ CI pipeline completed successfully"

pre-commit: ## Run pre-commit hooks
	@echo "Running pre-commit hooks..."
	@which pre-commit > /dev/null || (echo "Please install pre-commit: pip install pre-commit" && exit 1)
	pre-commit run --all-files
	@echo "✓ Pre-commit checks passed"

# Docker
docker-build: ## Build Docker image
	@echo "Building Docker image..."
	docker build -t hodei-jobs:latest .

docker-run: ## Run Docker container
	@echo "Running Docker container..."
	docker run -p 8080:8080 hodei-jobs:latest

# Development helpers
watch-test: ## Watch tests and run on changes
	@echo "Watching for test changes..."
	cargo watch -x test

watch-build: ## Watch build and rebuild on changes
	@echo "Watching for source changes..."
	cargo watch -x build

# Performance profiling
profile: ## Run performance profiling
	@echo "Running performance profiling..."
	cargo build --release
	perf record -g target/release/hodei-jobs
	perf report

# Dependency management
update-deps: ## Update all dependencies
	@echo "Updating dependencies..."
	cargo update
	@echo "Dependencies updated"

outdated: ## Check for outdated dependencies
	@echo "Checking for outdated dependencies..."
	cargo outdated

# Generate coverage badge
coverage-badge: test-coverage
	@echo "Generating coverage badge..."
	cargo llvm-cov --workspace --summary-only | grep -E "TOTAL.*[0-9]+\.[0-9]+%" || echo "Coverage: $$(cargo llvm-cov --workspace --summary-only | tail -1 | awk '{print $$4}')"

# Release workflow
prepare-release: check audit test build-release ## Prepare for release
	@echo "Release preparation completed"
	@echo "Next steps:"
	@echo "  1. Update version in Cargo.toml files"
	@echo "  2. Update CHANGELOG.md"
	@echo "  3. Create release tag: git tag vX.Y.Z"
	@echo "  4. Push tag: git push origin vX.Y.Z"
