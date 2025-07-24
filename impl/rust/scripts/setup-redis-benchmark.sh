#!/bin/bash
# Simple Redis benchmark setup for MITCH protocol

echo "=== MITCH Redis Benchmark Setup ==="
echo "Testing connection to Redis..."

if redis-cli -u "$REDIS_URL" ping > /dev/null 2>&1; then
    echo "✓ Redis connection successful"
    echo
    echo "Run benchmarks with:"
    echo "  export REDIS_URL=\"$REDIS_URL\""
    echo "  cargo bench --features redis-client,benchmarking"
else
    echo "✗ Redis connection failed"
    echo "Make sure Redis is running at localhost:40001"
    exit 1
fi
