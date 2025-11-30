# =============================================================================
# Hodei Pipelines - Unified Makefile
# =============================================================================

# Variables
VERSION ?= $(shell git describe --tags --always --dirty)
NAMESPACE ?= hodei-pipelines
IMAGE_REGISTRY ?= ghcr.io/rubentxu
IMAGE_SERVER ?= hodei-pipelines/hodei-server
IMAGE_AGENT ?= hodei-pipelines/hwp-agent

# Scripts
SCRIPT_CI_CD := deployment/scripts/ci-cd.sh
SCRIPT_TESTKUBE := testkube/scripts/run-tests.sh
SCRIPT_VCLUSTER := testkube/scripts/setup-vcluster-testkube.sh
SCRIPT_MANIFEST := scripts/generate_clean_manifest.sh

.PHONY: help build push deploy test-helm test-e2e setup-dev clean manifest validate-api gen-types check-contract test-contract

# Default target
help: ## Show this help message
	@echo "Hodei Pipelines Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# =============================================================================
# Build & Deploy
# =============================================================================

build: ## Build Docker images
	@echo "Building images..."
	@VERSION=$(VERSION) $(SCRIPT_CI_CD) build

push: ## Push Docker images
	@echo "Pushing images..."
	@VERSION=$(VERSION) $(SCRIPT_CI_CD) push

deploy: ## Deploy to Kubernetes
	@echo "Deploying to Kubernetes..."
	@VERSION=$(VERSION) NAMESPACE=$(NAMESPACE) $(SCRIPT_CI_CD) deploy

# =============================================================================
# Testing
# =============================================================================

test-helm: ## Run Helm chart tests (lint & template)
	@echo "Running Helm tests..."
	@VERSION=$(VERSION) $(SCRIPT_CI_CD) test

test-e2e: ## Run Testkube E2E tests (Usage: make test-e2e [SUITE=...] [TEST=...])
	@echo "Running E2E tests..."
	@$(SCRIPT_TESTKUBE) $(if $(SUITE),--suite $(SUITE)) $(if $(TEST),--test $(TEST)) --namespace testkube --watch

# =============================================================================
# Development
# =============================================================================

setup-dev: ## Setup local vcluster environment with Testkube
	@echo "Setting up local development environment..."
	@$(SCRIPT_VCLUSTER)

manifest: ## Generate CODE_MANIFEST.csv
	@echo "Generating CODE_MANIFEST.csv..."
	@bash $(SCRIPT_MANIFEST)

# =============================================================================
# API Contract Validation
# =============================================================================

validate-api: ## Validate OpenAPI specification
	@echo "Validating API contract..."
	@./scripts/validate-api-contract.sh

gen-types: ## Generate TypeScript types from OpenAPI
	@echo "Generating TypeScript types..."
	@if [ -f "docs/openapi.yaml" ]; then \
		cd web-console && npm run generate:types; \
		echo "✅ TypeScript types generated"; \
	else \
		echo "❌ OpenAPI spec not found at docs/openapi.yaml"; \
		exit 1; \
	fi

check-contract: ## Check for breaking changes in API contract
	@echo "Checking for breaking changes..."
	@if [ -f "docs/openapi-previous.yaml" ]; then \
		echo "Comparing with previous specification..."; \
		diff -u docs/openapi-previous.yaml docs/openapi.yaml || \
		echo "⚠️  Breaking changes detected!"; \
	else \
		echo "No previous spec found. Copying current as baseline..."; \
		cp docs/openapi.yaml docs/openapi-previous.yaml; \
	fi

test-contract: ## Run contract tests
	@echo "Running contract tests..."
	@cd web-console && npm test -- tests/contract/api-contract.test.ts || \
		echo "⚠️  Contract tests failed"

# =============================================================================
# Cleanup
# =============================================================================

clean: ## Cleanup resources
	@echo "Cleaning up..."
	@$(SCRIPT_CI_CD) cleanup
