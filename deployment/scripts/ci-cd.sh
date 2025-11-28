#!/bin/bash
# =============================================================================
# CI/CD Pipeline para Hodei Pipelines
# =============================================================================

set -e

# Variables
IMAGE_REGISTRY="${IMAGE_REGISTRY:-ghcr.io/rubentxu}"
IMAGE_SERVER="${IMAGE_SERVER:-hodei-pipelines/hodei-server}"
IMAGE_AGENT="${IMAGE_AGENT:-hodei-pipelines/hwp-agent}"
VERSION="${VERSION:-$(git describe --tags --always --dirty)}"
NAMESPACE="${NAMESPACE:-hodei-pipelines}"

# Colores
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[CI/CD]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[CI/CD]${NC} ✅ $1"
}

log_warning() {
    echo -e "${YELLOW}[CI/CD]${NC} ⚠️  $1"
}

log_error() {
    echo -e "${RED}[CI/CD]${NC} ✗ $1"
}

# Build Docker Images
build_images() {
    log_info "Building Docker images..."

    # Build Server
    log_info "Building Hodei Server image: ${IMAGE_REGISTRY}/${IMAGE_SERVER}:${VERSION}"
    docker build -t ${IMAGE_REGISTRY}/${IMAGE_SERVER}:${VERSION} \
                 -t ${IMAGE_REGISTRY}/${IMAGE_SERVER}:latest \
                 -f Dockerfile .

    # Build Agent
    log_info "Building HWP Agent image: ${IMAGE_REGISTRY}/${IMAGE_AGENT}:${VERSION}"
    docker build -t ${IMAGE_REGISTRY}/${IMAGE_AGENT}:${VERSION} \
                 -t ${IMAGE_REGISTRY}/${IMAGE_AGENT}:latest \
                 -f crates/hwp-agent/Dockerfile .

    log_success "Docker images built successfully"
}

# Push Docker Images
push_images() {
    if [ "$SKIP_PUSH" = "true" ]; then
        log_warning "Skipping image push (SKIP_PUSH=true)"
        return 0
    fi

    log_info "Pushing Docker images to registry..."

    # Push Server
    docker push ${IMAGE_REGISTRY}/${IMAGE_SERVER}:${VERSION}
    docker push ${IMAGE_REGISTRY}/${IMAGE_SERVER}:latest

    # Push Agent
    docker push ${IMAGE_REGISTRY}/${IMAGE_AGENT}:${VERSION}
    docker push ${IMAGE_REGISTRY}/${IMAGE_AGENT}:latest

    log_success "Docker images pushed successfully"
}

# Run Helm Tests
test_helm() {
    log_info "Running Helm chart tests..."

    # Lint Helm charts
    helm lint deployment/helm/

    # Template test
    helm template hodei deployment/helm/ \
        -f deployment/helm/values-dev.yaml \
        --set hodei-server.image.tag=${VERSION} \
        --set hwp-agent.image.tag=${VERSION} \
        > /tmp/hodei-template.yaml

    # Dry-run install
    helm install hodei deployment/helm/ \
        --namespace ${NAMESPACE} \
        --create-namespace \
        -f deployment/helm/values-dev.yaml \
        --set hodei-server.image.tag=${VERSION} \
        --set hwp-agent.image.tag=${VERSION} \
        --dry-run

    log_success "Helm chart tests passed"
}

# Security Scanning
security_scan() {
    if [ "$SKIP_SECURITY" = "true" ]; then
        log_warning "Skipping security scan (SKIP_SECURITY=true)"
        return 0
    fi

    log_info "Running security scan..."

    # Trivy scanner
    if command -v trivy &> /dev/null; then
        trivy image --format json --output security-report.json ${IMAGE_REGISTRY}/${IMAGE_SERVER}:${VERSION}
        trivy image --format json --output security-report-agent.json ${IMAGE_REGISTRY}/${IMAGE_AGENT}:${VERSION}
        log_success "Security scan completed"
    else
        log_warning "Trivy not found, skipping security scan"
    fi
}

# Deploy to ArgoCD
deploy_argocd() {
    if [ "$SKIP_DEPLOY" = "true" ]; then
        log_warning "Skipping deployment (SKIP_DEPLOY=true)"
        return 0
    fi

    log_info "Deploying to ArgoCD..."

    # Update ArgoCD Application
    if [ -n "$ARGO_SERVER" ]; then
        argocd --server ${ARGO_SERVER} \
               --auth-token ${ARGO_TOKEN} \
               app set hodei-pipelines \
               --parameter hodei-server.image.tag=${VERSION} \
               --parameter hwp-agent.image.tag=${VERSION}

        # Sync application
        argocd --server ${ARGO_SERVER} \
               --auth-token ${ARGO_TOKEN} \
               app sync hodei-pipelines

        log_success "Application synced successfully"
    else
        log_warning "ARGO_SERVER not set, skipping ArgoCD deployment"
    fi
}

# Generate SBOM
generate_sbom() {
    if [ "$SKIP_SBOM" = "true" ]; then
        log_warning "Skipping SBOM generation (SKIP_SBOM=true)"
        return 0
    fi

    log_info "Generating SBOM..."

    if command -v syft &> /dev/null; then
        syft ${IMAGE_REGISTRY}/${IMAGE_SERVER}:${VERSION} -o json > sbom-server.json
        syft ${IMAGE_REGISTRY}/${IMAGE_AGENT}:${VERSION} -o json > sbom-agent.json
        log_success "SBOM generated successfully"
    else
        log_warning "Syft not found, skipping SBOM generation"
    fi
}

# Cleanup
cleanup() {
    log_info "Cleaning up..."
    # Remove built images if SKIP_PUSH is true
    if [ "$SKIP_PUSH" = "true" ]; then
        docker rmi ${IMAGE_REGISTRY}/${IMAGE_SERVER}:${VERSION} 2>/dev/null || true
        docker rmi ${IMAGE_REGISTRY}/${IMAGE_AGENT}:${VERSION} 2>/dev/null || true
    fi
}

# Run TestKube Tests
run_testkube_tests() {
    if [ "$SKIP_TESTKUBE" = "true" ]; then
        log_warning "Skipping TestKube tests (SKIP_TESTKUBE=true)"
        return 0
    fi

    log_info "Running TestKube tests..."

    # Check if TestKube is available
    if ! kubectl testkube version &>/dev/null; then
        log_warning "TestKube not available, skipping tests"
        return 0
    fi

    # Deploy tests if they don't exist
    log_info "Ensuring TestKube tests are deployed..."
    if ! kubectl get testsuite hodei-smoke-tests -n testkube &>/dev/null; then
        log_info "Deploying TestKube tests..."
        ./testkube/scripts/deploy-tests.sh
    fi

    # Run smoke tests
    log_info "Running smoke tests..."
    if ./deployment/scripts/testkube-integration.sh; then
        log_success "TestKube tests passed"
    else
        log_warning "TestKube tests failed or skipped"
        return 1
    fi
}

# Show usage
show_usage() {
    cat << EOF
CI/CD Pipeline for Hodei Pipelines

USAGE:
    ./ci-cd.sh [COMMAND] [OPTIONS]

COMMANDS:
    build       Build Docker images
    test        Run Helm tests
    testkube    Run TestKube tests
    scan        Run security scan
    deploy      Deploy to ArgoCD
    sbom        Generate SBOM
    all         Run all steps (build, test, testkube, scan, deploy, sbom)
    cleanup     Clean up resources

ENVIRONMENT VARIABLES:
    VERSION                    Version tag (default: git describe)
    IMAGE_REGISTRY             Docker registry (default: ghcr.io/rubentxu)
    IMAGE_SERVER               Server image name (default: hodei-pipelines/hodei-server)
    IMAGE_AGENT                Agent image name (default: hodei-pipelines/hwp-agent)
    NAMESPACE                  Kubernetes namespace (default: hodei-pipelines)
    TESTKUBE_NAMESPACE         TestKube namespace (default: testkube)
    ARGO_SERVER                ArgoCD server URL
    ARGO_TOKEN                 ArgoCD auth token
    SKIP_PUSH                  Skip image push (default: false)
    SKIP_SECURITY              Skip security scan (default: false)
    SKIP_TESTKUBE              Skip TestKube tests (default: false)
    SKIP_DEPLOY                Skip deployment (default: false)
    SKIP_SBOM                  Skip SBOM generation (default: false)

EXAMPLES:
    # Build and test
    ./ci-cd.sh all --skip-push --skip-deploy

    # Build and deploy
    ./ci-cd.sh all --skip-scan

    # Run only TestKube tests
    ./ci-cd.sh testkube

    # Production deployment
    VERSION=v0.1.0 ./ci-cd.sh all

EOF
}

# Main
main() {
    local command=$1
    shift || true

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-push)
                SKIP_PUSH=true
                shift
                ;;
            --skip-security)
                SKIP_SECURITY=true
                shift
                ;;
            --skip-deploy)
                SKIP_DEPLOY=true
                shift
                ;;
            --skip-sbom)
                SKIP_SBOM=true
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    # Show configuration
    log_info "CI/CD Configuration:"
    log_info "  Version: ${VERSION}"
    log_info "  Registry: ${IMAGE_REGISTRY}"
    log_info "  Server Image: ${IMAGE_REGISTRY}/${IMAGE_SERVER}:${VERSION}"
    log_info "  Agent Image: ${IMAGE_REGISTRY}/${IMAGE_AGENT}:${VERSION}"
    echo ""

    # Execute command
    case $command in
        build)
            build_images
            ;;
        test)
            test_helm
            ;;
        testkube)
            run_testkube_tests
            ;;
        scan)
            security_scan
            ;;
        deploy)
            deploy_argocd
            ;;
        sbom)
            generate_sbom
            ;;
        all)
            build_images
            push_images
            test_helm
            run_testkube_tests
            security_scan
            deploy_argocd
            generate_sbom
            ;;
        cleanup)
            cleanup
            ;;
        *)
            log_error "Unknown command: $command"
            show_usage
            exit 1
            ;;
    esac

    log_success "CI/CD pipeline completed successfully"
}

main "$@"
