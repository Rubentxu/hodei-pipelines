#!/bin/bash
set -e

echo "🧪 Running Log Streaming E2E tests..."
echo "  → Cleaning up any existing worker-manager process..."
pkill -9 -f worker-manager 2>/dev/null || true
sleep 2

echo "  → Building worker-manager..."
cargo build -p hodei-worker-lifecycle-manager

echo "  → Starting worker-manager on port 8082..."
./target/debug/worker-manager > /tmp/worker-manager.log 2>&1 &
WM_PID=$!

echo "  → Waiting for service to be ready (max 30s)..."
timeout 30 sh -c 'until curl -s http://localhost:8082/health > /dev/null 2>&1; do sleep 1; done' && echo "  ✅ Worker Manager is ready!" || {
    echo "  ❌ Timeout waiting for worker-manager"
    cat /tmp/worker-manager.log
    kill $WM_PID 2>/dev/null || true
    exit 1
}

echo "  → Verifying log streaming endpoint..."
curl -s -X POST http://localhost:8082/api/v1/execute -H "Content-Type: application/json" -d '{"command":"echo test"}' > /dev/null 2>&1 && \
    echo "  ✅ Log streaming endpoint is accessible!" || \
    echo "  ⚠️  Log streaming endpoint test failed (continuing anyway)..."

echo "  → Running log streaming tests..."
cargo test -p e2e-tests --test log_streaming_test -- --ignored --nocapture

echo "  → Stopping worker-manager..."
pkill -9 -f worker-manager 2>/dev/null || true
echo "✅ Log Streaming tests completed!"
