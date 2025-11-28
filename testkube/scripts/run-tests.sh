#!/bin/bash
# TestKube Test Runner Script for Hodei Pipelines

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

NAMESPACE=${NAMESPACE:-testkube}
SUITE=${SUITE:-hodei-smoke-tests}
TEST=${TEST:-}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_usage() {
    cat << EOF
TestKube Test Runner for Hodei Pipelines

USAGE:
    ./run-tests.sh [OPTIONS]

OPTIONS:
    -s, --suite SUITE      Test suite to run (default: hodei-smoke-tests)
                          Available suites:
                            - hodei-smoke-tests
                            - hodei-integration-tests
                            - hodei-full-suite
    -t, --test TEST        Run specific test instead of suite
    -n, --namespace NS     Namespace (default: testkube)
    -w, --watch            Watch test execution
    -l, --list             List available tests and suites
    -h, --help             Show this help

EXAMPLES:
    # Run smoke tests
    ./run-tests.sh --suite hodei-smoke-tests

    # Run specific test
    ./run-tests.sh --test server-health-check

    # Watch test execution
    ./run-tests.sh --suite hodei-integration-tests --watch

EOF
}

list_tests() {
    log_info "Available Tests:"
    kubectl get tests -n $NAMESPACE -o custom-columns=NAME:.metadata.name,TYPE:.spec.type 2>/dev/null || echo "Run 'kubectl get tests' in testkube namespace"
    
    echo ""
    log_info "Available Test Suites:"
    kubectl get testsuites -n $NAMESPACE -o custom-columns=NAME:.metadata.name,DESCRIPTION:.spec.description 2>/dev/null || echo "Run 'kubectl get testsuites' in testkube namespace"
}

run_test_suite() {
    local suite_name=$1
    log_info "Running test suite: $suite_name"
    
    if ! kubectl get testsuite $suite_name -n $NAMESPACE &>/dev/null; then
        log_warning "Test suite $suite_name not found"
        return 1
    fi
    
    local run_name="test-suite-$(date +%s)"
    kubectl testkube run testsuite $suite_name -n $NAMESPACE --name $run_name --watch
}

run_specific_test() {
    local test_name=$1
    log_info "Running test: $test_name"
    
    if ! kubectl get test $test_name -n $NAMESPACE &>/dev/null; then
        log_warning "Test $test_name not found"
        return 1
    fi
    
    local run_name="test-$(date +%s)"
    kubectl testkube run test $test_name -n $NAMESPACE --name $run_name --watch
}

# Parse arguments
WATCH=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--suite)
            SUITE="$2"
            shift 2
            ;;
        -t|--test)
            TEST="$2"
            shift 2
            ;;
        -n|--namespace)
            NAMESPACE="$2"
            shift 2
            ;;
        -w|--watch)
            WATCH=true
            shift
            ;;
        -l|--list)
            list_tests
            exit 0
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

if ! command -v kubectl testkube &> /dev/null; then
    log_error "TestKube CLI not found. Please install it first."
    exit 1
fi

if ! kubectl get namespace $NAMESPACE &>/dev/null; then
    log_warning "Namespace $NAMESPACE not found. Creating it..."
    kubectl create namespace $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -
fi

if [ -n "$TEST" ]; then
    run_specific_test "$TEST"
else
    run_test_suite "$SUITE"
fi

log_success "Test execution completed"
