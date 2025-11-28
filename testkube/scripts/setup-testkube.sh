#!/bin/bash
# Setup TestKube for Hodei Pipelines

set -e

NAMESPACE=${NAMESPACE:-testkube}

echo "Setting up TestKube for Hodei Pipelines..."

# Install TestKube if not already installed
if ! helm list -n testkube &>/dev/null; then
    echo "Installing TestKube..."
    helm repo add testkube https://kubeshop.github.io/helm-charts
    helm repo update
    helm install testkube testkube/testkube --namespace $NAMESPACE --create-namespace --set testkube-dashboard.enabled=true
else
    echo "TestKube already installed"
fi

# Wait for TestKube to be ready
echo "Waiting for TestKube to be ready..."
kubectl wait --for=condition=available --timeout=300s deployment/testkube-dashboard -n $NAMESPACE
kubectl wait --for=condition=available --timeout=300s deployment/testkube-api-server -n $NAMESPACE
kubectl wait --for=condition=available --timeout=300s deployment/testkube-operator -n $NAMESPACE

echo "TestKube installed successfully!"
echo ""
echo "Dashboard URL:"
kubectl get svc testkube-dashboard -n $NAMESPACE
echo ""
echo "To access the dashboard:"
echo "  kubectl port-forward svc/testkube-dashboard 8088:8088 -n $NAMESPACE"
echo "  Then open http://localhost:8088"
