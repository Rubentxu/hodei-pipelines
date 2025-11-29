#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}🚀 Starting Hodei Jobs Development Environment${NC}"

# Kill existing processes
echo -e "${GREEN}🧹 Cleaning up old processes...${NC}"
pkill -f "hodei-server" || true
pkill -f "hwp-agent" || true
# We don't kill npm/vite as it takes longer to start, usually fine to leave running

# Build server and agent if needed (fast check)
# echo -e "${GREEN}🔨 Building binaries...${NC}"
# cargo build -p hodei-server -p hwp-agent

# Start Server
echo -e "${GREEN}🌐 Starting Hodei Server...${NC}"
cargo run -p hodei-server > server.log 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID"

# Wait for server to be ready
echo "Waiting for server to start..."
sleep 5

# Start Agent
echo -e "${GREEN}🤖 Starting HWP Agent...${NC}"
export HODEI_TOKEN=test-token
cargo run -p hwp-agent > agent.log 2>&1 &
AGENT_PID=$!
echo "Agent PID: $AGENT_PID"

echo -e "${GREEN}✅ Environment is running!${NC}"
echo "Server logs: tail -f server.log"
echo "Agent logs: tail -f agent.log"
echo "Press Ctrl+C to stop everything."

# Cleanup function
cleanup() {
    echo -e "${RED}🛑 Stopping services...${NC}"
    if [ -n "$SERVER_PID" ]; then kill $SERVER_PID 2>/dev/null || true; fi
    if [ -n "$AGENT_PID" ]; then kill $AGENT_PID 2>/dev/null || true; fi
    
    # Force kill ports if still open
    echo "Ensuring ports 8080 and 50051 are free..."
    fuser -k -9 8080/tcp 2>/dev/null || true
    fuser -k -9 50051/tcp 2>/dev/null || true
    
    exit
}

# Trap Ctrl+C to kill background processes
trap cleanup INT TERM

wait
