#!/bin/bash
# Check Kubernetes connection
if ! kubectl cluster-info &> /dev/null; then
    echo "❌ No se puede conectar a Kubernetes"
    echo "💡 Ejecuta: ./testkube/scripts/setup-k8s.sh"
    exit 1
fi

# Quick test script for immediate feedback

NAMESPACE=${NAMESPACE:-hodei-jobs}

echo "🚀 Quick Test - Hodei Pipelines"
echo "==============================="
echo ""

# Pods status
echo "📊 Pods Status:"
kubectl get pods -n $NAMESPACE 2>/dev/null | grep -E "hodei|STATUS" | head -10
echo ""

# Health check
echo "❤️  Health Check:"
server_pod=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=server -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
if [ -n "$server_pod" ]; then
    if kubectl exec -n $NAMESPACE $server_pod -- curl -sf http://localhost:8080/health >/dev/null 2>&1; then
        echo "   ✅ Server is healthy"
    else
        echo "   ⚠️  Server health unknown"
    fi
else
    echo "   ⚠️  Server pod not found"
fi
echo ""

# Workers
worker_count=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=worker --no-headers 2>/dev/null | wc -l)
echo "👷 Workers: $worker_count running"
echo ""

# Quick log check
echo "📝 Recent Logs (last 3 lines):"
if [ -n "$server_pod" ]; then
    kubectl logs $server_pod -n $NAMESPACE --tail=3 2>/dev/null | sed 's/^/   /'
fi
echo ""

echo "==============================="
echo "✅ Quick test complete!"
echo ""
echo "For detailed testing:"
echo "  ./testkube/scripts/live-api-test.sh"
echo "  ./testkube/scripts/run-tests.sh --suite hodei-smoke-tests"
