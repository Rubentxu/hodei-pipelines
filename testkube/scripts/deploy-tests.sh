#!/bin/bash
# Deploy all Hodei Pipelines tests to TestKube

set -e

NAMESPACE=${NAMESPACE:-testkube}

echo "Deploying Hodei Pipelines tests to TestKube..."

# Create namespace if it doesn't exist
kubectl create namespace $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -

# Deploy tests
echo "Deploying tests..."
for test in testkube/tests/test-*.yaml; do
    if [ -f "$test" ]; then
        echo "  Deploying $(basename $test)..."
        kubectl apply -f "$test" -n $NAMESPACE
    fi
done

# Deploy test suites
echo "Deploying test suites..."
for suite in testkube/test-suites/hodei-*.yaml; do
    if [ -f "$suite" ]; then
        echo "  Deploying $(basename $suite)..."
        kubectl apply -f "$suite" -n $NAMESPACE
    fi
done

# Deploy executors
echo "Deploying custom executors..."
for executor in testkube/executors/*.yaml; do
    if [ -f "$executor" ]; then
        echo "  Deploying $(basename $executor)..."
        kubectl apply -f "$executor" -n $NAMESPACE
    fi
done

echo ""
echo "Tests deployed successfully!"
echo ""
echo "List all tests:"
echo "  kubectl get tests -n $NAMESPACE"
echo ""
echo "List all test suites:"
echo "  kubectl get testsuites -n $NAMESPACE"
echo ""
echo "Run smoke tests:"
echo "  ./testkube/scripts/run-tests.sh --suite hodei-smoke-tests"
