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

.PHONY: help build push deploy test-helm test-e2e setup-dev clean manifest

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
# Cleanup
# =============================================================================

clean: ## Cleanup resources
	@echo "Cleaning up..."
	@$(SCRIPT_CI_CD) cleanup
