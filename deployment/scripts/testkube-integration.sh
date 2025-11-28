#!/bin/bash
# TestKube CI/CD Integration Script

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

NAMESPACE=${NAMESPACE:-testkube}
SKIP_TESTS=${SKIP_TESTS:-false}

log_info() {
    echo -e "${BLUE}[CI/CD]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[CI/CD]${NC} ✅ $1"
}

log_warning() {
    echo -e "${YELLOW}[CI/CD]${NC} ⚠️  $1"
}

# Main CI/CD integration
main() {
    log_info "TestKube CI/CD Integration"
    
    if [ "$SKIP_TESTS" = "true" ]; then
        log_warning "Skipping tests (SKIP_TESTS=true)"
        return 0
    fi
    
    # Check if TestKube is available
    if ! kubectl testkube version &>/dev/null; then
        log_warning "TestKube not available, skipping tests"
        return 0
    fi
    
    # Get image tag
    TAG=${VERSION:-latest}
    
    # Update test image tags
    log_info "Updating test image tags to: $TAG"
    kubectl patch test server-health-check -n $NAMESPACE -p "{\"metadata\":{\"labels\":{\"version\":\"$TAG\"}}}" 2>/dev/null || true
    
    # Run smoke tests
    log_info "Running smoke tests..."
    if kubectl testkube run testsuite hodei-smoke-tests -n $NAMESPACE --watch; then
        log_success "Smoke tests passed"
    else
        log_error "Smoke tests failed"
        exit 1
    fi
    
    # Run integration tests (if not in fast mode)
    if [ "$CI_TEST_LEVEL" != "fast" ]; then
        log_info "Running integration tests..."
        if kubectl testkube run testsuite hodei-integration-tests -n $NAMESPACE --watch; then
            log_success "Integration tests passed"
        else
            log_warning "Integration tests failed"
        fi
    fi
    
    # Collect test results
    log_info "Collecting test results..."
    kubectl testkube get testsuiteexecution -n $NAMESPACE --latest
    
    log_success "TestKube CI/CD integration completed"
}

main "$@"
