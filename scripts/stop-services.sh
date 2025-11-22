#!/bin/bash
# Stop all Hodei Jobs services

echo "🛑 Stopping Hodei Jobs Platform Services"
echo "========================================"

# Kill processes by name
if pgrep -f orchestrator > /dev/null; then
    echo "📦 Stopping Orchestrator..."
    pkill -f orchestrator
fi

if pgrep -f scheduler > /dev/null; then
    echo "📦 Stopping Scheduler..."
    pkill -f scheduler
fi

if pgrep -f worker-manager > /dev/null; then
    echo "📦 Stopping Worker Manager..."
    pkill -f worker-manager
fi

# Wait for processes to stop
sleep 2

# Verify all services stopped
if ! pgrep -f "orchestrator|scheduler|worker-manager" > /dev/null; then
    echo "✅ All services stopped successfully"
else
    echo "⚠️  Some processes may still be running"
    echo ""
    echo "Force kill with:"
    echo "  pkill -9 -f orchestrator"
    echo "  pkill -9 -f scheduler"
    echo "  pkill -9 -f worker-manager"
fi
