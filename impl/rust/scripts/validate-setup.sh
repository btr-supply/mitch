#!/bin/bash
# Validate MITCH setup and demonstrate usage

set -e

echo "=== MITCH Protocol Validation ==="
echo

# Test compilation
echo "✓ Testing compilation..."
cargo check --all-features --quiet
echo "  - All features compile successfully"

# Test benchmarks compilation
echo "✓ Testing benchmark compilation..."
cargo bench --features redis-client,benchmarking --no-run --quiet
echo "  - Benchmarks compile successfully"

# Test binary compilation
echo "✓ Testing binary compilation..."
cargo build --bin webtransport_server --features webtransport-client,benchmarking --quiet
echo "  - WebTransport server compiles successfully"

# Test Redis connection if URL provided
if [[ -n "${REDIS_URL}" ]]; then
    echo "✓ Testing Redis connection..."
    if redis-cli -u "$REDIS_URL" ping > /dev/null 2>&1; then
        echo "  - Redis connection successful: $REDIS_URL"
    else
        echo "  ⚠️  Redis connection failed: $REDIS_URL"
    fi
fi

echo
echo "🚀 Setup validation complete!"
echo
echo "Next steps:"
echo "1. Set environment variables:"
echo "   export REDIS_URL=\"redis://user:password@localhost:40001\""
echo
echo "2. Run Redis benchmarks:"
echo "   cargo bench --features redis-client,benchmarking"
echo
echo "3. Run WebTransport benchmarks:"
echo "   # Terminal 1:"
echo "   cargo run --bin webtransport_server --features webtransport-client,benchmarking"
echo "   # Terminal 2:"
echo "   export WEBTRANSPORT_URL=\"https://localhost:4433\""
echo "   cargo bench --features webtransport-client,benchmarking"
echo
echo "📊 Fire-and-forget mode: No acknowledgments, maximum throughput!"
