#!/bin/bash
# Quick start script for all Hodei Jobs services

set -e

echo "🚀 Starting Hodei Jobs Platform Services"
echo "========================================"

# Check if binaries exist
if [ ! -f "./target/debug/orchestrator" ]; then
    echo "❌ Orchestrator binary not found. Building..."
    cargo build --bin orchestrator
fi

if [ ! -f "./target/debug/scheduler" ]; then
    echo "❌ Scheduler binary not found. Building..."
    cargo build --bin scheduler
fi

if [ ! -f "./target/debug/worker-manager" ]; then
    echo "❌ Worker Manager binary not found. Building..."
    cargo build --bin worker-manager
fi

# Start services
echo ""
echo "📦 Starting Orchestrator on port 8080..."
./target/debug/orchestrator > /tmp/orchestrator.log 2>&1 &
ORCHESTRATOR_PID=$!
echo "   PID: $ORCHESTRATOR_PID"

echo ""
echo "📦 Starting Scheduler on port 8081..."
./target/debug/scheduler > /tmp/scheduler.log 2>&1 &
SCHEDULER_PID=$!
echo "   PID: $SCHEDULER_PID"

echo ""
echo "📦 Starting Worker Manager on port 8082..."
./target/debug/worker-manager > /tmp/worker-manager.log 2>&1 &
WORKER_PID=$!
echo "   PID: $WORKER_PID"

# Wait for services to start
echo ""
echo "⏳ Waiting for services to start..."
sleep 3

# Test services
echo ""
echo "🔍 Testing service health..."
echo ""

# Test Orchestrator
if curl -s http://localhost:8080/health > /dev/null; then
    echo "✅ Orchestrator: http://localhost:8080"
    echo "   Swagger UI: http://localhost:8080/swagger-ui"
else
    echo "❌ Orchestrator: Not responding"
fi

# Test Scheduler
if curl -s http://localhost:8081/health > /dev/null; then
    echo "✅ Scheduler: http://localhost:8081"
else
    echo "❌ Scheduler: Not responding"
fi

# Test Worker Manager
if curl -s http://localhost:8082/health > /dev/null; then
    echo "✅ Worker Manager: http://localhost:8082"
else
    echo "❌ Worker Manager: Not responding"
fi

echo ""
echo "🎉 All services started!"
echo ""
echo "📊 Quick Test Commands:"
echo "  curl http://localhost:8080/health"
echo "  curl http://localhost:8081/health"
echo "  curl http://localhost:8082/health"
echo ""
echo "📝 View logs:"
echo "  tail -f /tmp/orchestrator.log"
echo "  tail -f /tmp/scheduler.log"
echo "  tail -f /tmp/worker-manager.log"
echo ""
echo "🛑 To stop services:"
echo "  kill $ORCHESTRATOR_PID $SCHEDULER_PID $WORKER_PID"
echo ""
echo "Or use:"
echo "  pkill -f orchestrator"
echo "  pkill -f scheduler"
echo "  pkill -f worker-manager"
