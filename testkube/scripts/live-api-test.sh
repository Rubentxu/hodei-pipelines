#!/bin/bash
# Check Kubernetes connection
if ! kubectl cluster-info &> /dev/null; then
    echo "❌ No se puede conectar a Kubernetes"
    echo "💡 Ejecuta: ./testkube/scripts/setup-k8s.sh"
    exit 1
fi

# Live API testing script with interactive output

set -e

NAMESPACE=${NAMESPACE:-hodei-jobs}
SERVER_URL=${SERVER_URL:-http://hodei-server:8080}

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║     Hodei Pipelines - Live API Testing                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "Namespace: $NAMESPACE"
echo "Server URL: $SERVER_URL"
echo ""

# Test 1: Health check
echo "1️⃣  Testing Server Health..."
if curl -s -f $SERVER_URL/health > /dev/null 2>&1; then
    echo "   ✅ Server is healthy"
else
    echo "   ❌ Server health check failed"
fi
echo ""

# Test 2: API endpoints
echo "2️⃣  Testing API Endpoints..."
endpoints=("/health" "/api/v1/version" "/api/v1/jobs" "/api/v1/workers" "/api/v1/pipelines")

for endpoint in "${endpoints[@]}"; do
    response=$(curl -s -o /dev/null -w "%{http_code}" $SERVER_URL$endpoint 2>/dev/null || echo "000")
    if [ "$response" = "200" ]; then
        echo "   ✅ $endpoint (HTTP $response)"
    elif [ "$response" = "401" ] || [ "$response" = "403" ]; then
        echo "   ✅ $endpoint (HTTP $response - Auth required)"
    else
        echo "   ⚠️  $endpoint (HTTP $response)"
    fi
done
echo ""

# Test 3: Workers
echo "3️⃣  Checking Workers..."
worker_count=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=worker --no-headers 2>/dev/null | wc -l)
if [ "$worker_count" -gt 0 ]; then
    echo "   ✅ Found $worker_count worker(s)"
    kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=worker --no-headers | awk '{print "      - " $1 " (" $3 ")"}'
else
    echo "   ⚠️  No workers found"
fi
echo ""

# Test 4: Jobs
echo "4️⃣  Checking Jobs..."
job_pods=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=job --no-headers 2>/dev/null | wc -l)
running_jobs=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=job --field-selector=status.phase=Running --no-headers 2>/dev/null | wc -l)

echo "   Total job pods: $job_pods"
echo "   Running jobs: $running_jobs"
echo ""

# Test 5: Logs
echo "5️⃣  Checking Recent Logs..."
server_pod=$(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=server -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")

if [ -n "$server_pod" ]; then
    echo "   Server logs (last 5 lines):"
    kubectl logs $server_pod -n $NAMESPACE --tail=5 2>/dev/null | sed 's/^/   | /' || echo "   Unable to fetch logs"
fi
echo ""

# Test 6: System status
echo "6️⃣  System Status Summary..."
echo "   Server pods: $(kubectl get pods -n $NAMESPACE -l app.kubernetes.io/component=server --no-headers 2>/dev/null | wc -l)"
echo "   Worker pods: $worker_count"
echo "   Job pods: $job_pods"
echo "   Services: $(kubectl get svc -n $NAMESPACE --no-headers 2>/dev/null | wc -l)"
echo ""

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║               ✅ Live API Test Complete                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "💡 To monitor logs in real-time, run:"
echo "   ./testkube/scripts/monitor-logs.sh --component all"
echo ""
echo "💡 To run comprehensive tests, execute:"
echo "   ./testkube/scripts/run-tests.sh --suite hodei-api-comprehensive"
