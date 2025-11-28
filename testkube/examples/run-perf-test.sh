#!/bin/bash
# Run performance test with k6

BASE_URL=${BASE_URL:-http://hodei-server:8080}

echo "Running performance test..."
echo "Base URL: $BASE_URL"

k6 run \
  --env BASE_URL=$BASE_URL \
  --out influxdb=http://localhost:8086/testkube \
  testkube/examples/performance-test-k6.js
