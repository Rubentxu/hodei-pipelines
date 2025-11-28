# Testing Guide for Hodei Pipelines

This guide describes how to run tests for the Hodei Pipelines project using the unified `Makefile`.

## Prerequisites

- **Docker**: For building images.
- **Kubernetes Cluster**: A running cluster (local or remote).
- **Helm**: For chart management.
- **Testkube CLI**: For running E2E tests.
- **kubectl**: For cluster interaction.

## Quick Start

Run the help command to see all available targets:

```bash
make help
```

## Build and Deploy

### Build Images
Build the Docker images for the Server and Agent:

```bash
make build
```

### Deploy to Kubernetes
Deploy the Helm charts to your current Kubernetes context:

```bash
make deploy
```

## Testing

### Helm Chart Tests
Lint and template the Helm charts to ensure validity:

```bash
make test-helm
```

### End-to-End (E2E) Tests
Run the Testkube test suites. This requires Testkube to be installed in the cluster.

Run the default smoke tests:
```bash
make test-e2e
```

Run a specific suite:
```bash
make test-e2e SUITE=hodei-full-suite
```

Run a specific test:
```bash
make test-e2e TEST=server-health-check
```

### Local Development Environment
Setup a local `vcluster` environment with Testkube pre-installed (requires `vcluster` CLI):

```bash
make setup-dev
```

## Clean Up

Remove built images and other artifacts:

```bash
make clean
```
