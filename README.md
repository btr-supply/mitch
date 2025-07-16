# MITCH: Ultra-light Market-Data Protocol

## Overview

**MITCH (Moded Individual Trade Clearing and Handling)** is a transport-agnostic binary protocol designed for ultra-low latency market data packing and transmission. See [model/overview.md](./model/overview.md) for detailed protocol overview. Inspired by [NASDAQ's ITCH](https://www.nasdaqtrader.com/content/technicalsupport/specifications/dataproducts/NQTVITCHSpecification.pdf), with extended types, no bloat, performant.

## Key Features

- **🌐 Cross-Platform**: Consistent Little-Endian, byte aligned, C-types encoding across all platforms for cast-only decoding
- **🔄 Transport Agnostic**: Tested over multicast and unicast transports: MoldUDP64, QUIC, ZMQ, RESP3 and more
- **📦 Language Agnostic**: Native implementations in rust, can be used over ffi with any of C/C++/Go/Python/Node or re-implemented from specs
- **⚡ Performance Optimized**: Flatbuffer inspired fixed-width fields, unsafe casting, zero-copy parsing
- **🛡️ Production Ready**: Comprehensive reference implementations and examples
- **📊 Order Book Snapshots**: 2KB fixed-size full order books l2 depth aggregated by adaptive bins
- **🎯 Universal IDs**: Low-footprint, 64-bit ticker identifiers and 32-bit exchange+message type channel identifiers for efficient pub/sub routing
- **🚀 Ultra-Light**: 10-40% lighter messages than NASDAQ ITCH

## Protocol Specifications

### Core Components
- **[Protocol Overview](./model/overview.md)**: High-level design principles and architecture
- **[Messaging Architecture](./model/messaging.md)**: Unified header format and batching
- **[Ticker ID System](./model/ticker.md)**: 8-byte encoding for any financial instrument
- **[Asset Classification](./model/asset.md)**: Standardized asset class and instrument types

### Message Types
- **[Trade Messages](./model/trade.md)**: Executed transaction data (32 bytes)
- **[Order Messages](./model/order.md)**: Order lifecycle events (32 bytes)
- **[Tick Messages](./model/tick.md)**: Bid/ask price snapshots (32 bytes)
- **[Index Messages](./model/index.md)**: Synthetic aggregated market data (64 bytes)
- **[Order Books](./model/order-book.md)**: Traditional and optimized order book formats

## Implementation Languages

Complete reference implementations in `./impl/`:

| Language | File | Target Environment |
|----------|------|-------------------|
| **Rust** | `mitch.rs` | High-performance systems, core libraries |
| **TypeScript** | `mitch.ts` | Web browsers, Node.js applications |
| **Python** | `mitch.py` | Data science, backend services, research |
| **Go** | `mitch.go` | Microservices, cloud-native applications |
| **Java** | `mitch.java` | Enterprise applications, Android |
| **C** | `mitch.h` | Embedded systems, high-frequency trading |
| **MQL4** | `mitch.mq4` | MetaTrader 4 trading platforms |

## Performance Characteristics

### Message Sizes
- **Header**: 8 bytes (all message types)
- **Trade/Order/Tick**: 32 bytes each
- **Index**: 64 bytes (enriched with analytics)
- **Order Book**: 2,072 bytes (complete market depth)

### Throughput Benchmarks
- **Serialization**: 10-20x faster with unsafe casting
- **Network Efficiency**: 40% reduction in bandwidth vs. standard ITCH
- **Memory Usage**: Fixed-size structures for predictable allocation
- **Cache Performance**: 8-byte aligned, single cache line access

## Getting Started

### 1. Choose Your Implementation
Select the appropriate reference implementation from `./impl/` based on your target environment.

### 2. Study the Protocol
- Start with [Protocol Overview](./model/overview.md)
- Review [Messaging Architecture](./model/messaging.md)
- Understand [Ticker ID System](./model/ticker.md)

### 3. Implement Message Types
- Begin with [Trade Messages](./model/trade.md) for basic market data
- Add [Tick Messages](./model/tick.md) for real-time quotes
- Include [Index Messages](./model/index.md) for multi-market aggregation

### 4. Add Advanced Features
- Implement [Order Messages](./model/order.md) for order management
- Use [Order Books](./model/order-book.md) for market depth
- Add Channel IDs for pub/sub routing

## Architecture

### Project Structure
```
mitch/
├── README.md              # This file - getting started guide
├── model/                 # Protocol specifications
│   ├── overview.md        # High-level protocol overview
│   ├── messaging.md       # Unified header and batching
│   ├── ticker.md          # 8-byte ticker ID system
│   ├── asset.md           # Asset classification system
│   ├── trade.md           # Trade message specification
│   ├── order.md           # Order message specification
│   ├── tick.md            # Tick message specification
│   ├── index.md           # Index message specification
│   └── order-book.md      # Order book specifications
├── impl/                  # Reference implementations
│   ├── mitch.rs           # Rust implementation
│   ├── mitch.ts           # TypeScript implementation
│   ├── mitch.mq4          # MQL4 implementation
│   └── examples/          # Usage examples per language
├── bins/                  # Order book aggregation bin definitions
└── ids/                   # Reference data (currencies, assets, exchanges)
```

## Use Cases

### Real-Time Trading Systems
- **High-frequency trading**: Sub-microsecond message processing
- **Multi-exchange arbitrage**: Cross-venue price discovery with Index messages
- **Risk management**: Real-time position monitoring with aggregated data

### Market Data Distribution  
- **Pub/Sub filtering**: Clients subscribe to specific instrument/venue combinations
- **Topic-based routing**: Efficient Kafka/Redis topic organization using Channel IDs
- **Bandwidth optimization**: Transmit only required data streams

### Analytics & Research
- **Market microstructure**: Analyze spreads, liquidity, and force metrics
- **Cross-venue analysis**: Compare execution quality across exchanges
- **Data quality monitoring**: Use confidence scores for research reliability

## Examples

### Basic Message Creation (Rust)
```rust
use mitch::*;

// Create a trade message
let trade = Trade {
    ticker_id: 0x03006F301CD00000,  // EUR/USD spot
    price: 1.08750,
    quantity: 1000000,
    trade_id: 123456,
    side: OrderSide::Buy,
    _padding: [0; 7],
};

// Pack for network transmission
let bytes = trade.pack(); // 32 bytes, ultra-fast
```

### Channel-Based Pub/Sub (TypeScript)
```typescript
// Subscribe to Binance EUR/USD ticks
const channelId = Channel.generate(101, 's'); // Binance + ticks
subscriber.subscribe(channelId.toString()); // "6648576"

// Process incoming messages
subscriber.on('message', (data) => {
    const message = MitchMessage.fromBytes(data);
    if (message.type === 'tick') {
        updatePriceDisplay(message.body);
    }
});
```

## Contributing

1. **Follow specifications**: Ensure implementations match the model definitions
2. **Maintain cross-language consistency**: Keep field names and behaviors identical
3. **Performance first**: Prioritize speed and memory efficiency
4. **Test thoroughly**: Validate serialization round-trips across all languages
5. **Document changes**: Update relevant specification files

## License

MIT License - see [LICENSE](./LICENSE)

## References

- [Original NASDAQ ITCH Protocol](./itch/v5-specs.pdf)
- [Model Specifications](./model/)
- [Implementation Examples](./impl/examples/)

---

**BTR Supply** | https://btr.supply | Production-Ready MITCH Implementation
