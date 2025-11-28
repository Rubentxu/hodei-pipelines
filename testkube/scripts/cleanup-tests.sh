#!/bin/bash
# Cleanup TestKube tests

NAMESPACE=${NAMESPACE:-testkube}

echo "Cleaning up TestKube tests in namespace: $NAMESPACE"

echo "Deleting test suites..."
kubectl delete testsuites -n $NAMESPACE --all

echo "Deleting tests..."
kubectl delete tests -n $NAMESPACE --all

echo "Deleting test executions..."
kubectl delete testexecutions -n $NAMESPACE --all

echo "Deleting executors..."
kubectl delete executors -n $NAMESPACE --all

echo "Cleanup completed!"
