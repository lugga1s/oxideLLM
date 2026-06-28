#!/bin/bash
set -e

# Increase file descriptor limit
ulimit -n 65536 || true

# Ensure no leftovers
killall -9 oxidellm || true
killall -9 oxidellm-mock || true

# Clean up old heaptrack files
rm -f heaptrack.oxidellm.*

echo "Starting mock server..."
./mock/target/release/oxidellm-mock --host 127.0.0.1 --port 9000 --chunks 5 --chunk-delay-us 100 > mock.log 2>&1 &
MOCK_PID=$!

echo "Starting gateway under heaptrack..."
heaptrack ./target/release/oxidellm > gateway.log 2>&1 &
GATEWAY_PID=$!

# Wait for servers to spin up
sleep 3

echo "Running k6 benchmark..."
TARGET_URL="http://127.0.0.1:8080/v1/chat/completions" VUS=100 DURATION=10s k6 run k6/proxy-vs-direct.js

echo "Stopping gateway gracefully..."
kill -INT $GATEWAY_PID || true

# Wait for gateway to dump heaptrack data
sleep 5

echo "Stopping mock..."
kill -9 $MOCK_PID || true

echo "Analyzing heaptrack file..."
HEAPTRACK_FILE=$(ls -t heaptrack.oxidellm.* 2>/dev/null | head -n 1)

if [ -n "$HEAPTRACK_FILE" ] && [ -f "$HEAPTRACK_FILE" ]; then
    echo "Found heaptrack file: $HEAPTRACK_FILE"
    heaptrack_print "$HEAPTRACK_FILE" > heaptrack_report.txt
    echo "Analysis complete. Report written to heaptrack_report.txt"
else
    echo "No heaptrack file found! Checking gateway.log:"
    cat gateway.log
    exit 1
fi
