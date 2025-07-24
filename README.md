<div align="center">
  <img border-radius="25px" max-height="250px" src="./banner.png" />
  <h1>MITCH</h1>
  <p>
    <strong>Market data, faster than light</strong>
  </p>
  <p>
    <a href="./model/overview.md"><img alt="Docs" src="https://img.shields.io/badge/Docs-212121?style=flat-square&logo=readthedocs&logoColor=white" width="auto"/></a>
    <a href="https://opensource.org/licenses/MIT"><img alt="License" src="https://img.shields.io/badge/license-MIT-000000?style=flat-square&logo=open-source-initiative&logoColor=white&labelColor=4c9c3d" width="auto"/></a>
    <a href="https://t.me/BTRSupply"><img alt="Telegram" src="https://img.shields.io/badge/Telegram-24b3e3?style=flat-square&logo=telegram&logoColor=white" width="auto"/></a>
    <a href="https://twitter.com/BTRSupply"><img alt="X (Twitter)" src="https://img.shields.io/badge/@BTRSupply-000000?style=flat-square&logo=x&logoColor=white" width="auto"/></a>
    </p>
</div>

## Overview

**MITCH (Moded Individual Trade Clearing and Handling)** is a transport-agnostic binary protocol designed for ultra-low latency market data packing and transmission. See [model/overview.md](./model/overview.md) for detailed protocol overview. Inspired by [NASDAQ's ITCH](https://www.nasdaqtrader.com/content/technicalsupport/specifications/dataproducts/NQTVITCHSpecification.pdf), with altered types and batch packing.

## Key Features

- **🌐 Cross-Platform**: Consistent Little-Endian, byte aligned, C-types encoding across all platforms for cast-only decoding
- **🔄 Transport Agnostic**: Tested over multicast and unicast transports: MoldUDP64, QUIC, ZMQ, RESP3 and more
- **📦 Language Agnostic**: Native implementations in rust, can be used over ffi with any of C/C++/Go/Python/Node or re-implemented from specs
- **⚡ Performance Optimized**: Flatbuffer inspired fixed-width fields, unsafe casting, zero-copy parsing
- **🛡️ Production Ready**: Comprehensive reference implementations and examples
- **📊 Order Book Snapshots**: 2KB fixed-size full order books l2 depth aggregated by adaptive bins
- **🎯 Universal IDs**: Low-footprint, 64-bit ticker identifiers and 32-bit exchange+message type channel identifiers for efficient pub/sub routing

## Protocol Specifications

**📖 Complete Protocol Documentation**: [model/overview.md](./model/overview.md)

| Component | Description |
|-----------|-------------|
| **[Messaging](./model/messaging.md)** | Unified 8-byte header, batching, Channel IDs |
| **[Ticker IDs](./model/ticker.md)** | 8-byte encoding for any financial instrument |
| **[Assets](./model/asset.md)** | Standardized asset classification system |
| **[Message Types](./model/overview.md#message-types)** | Trade, Order, Tick, Index, OrderBook formats |

## Implementation Languages

Complete reference implementations in `./impl/`:

| Language | File | Target Environment |
|----------|------|--------------------|
| **Rust** | `mitch.rs` | Reference implementation |
| **TypeScript** | `mitch.ts` | Bun, Node, Deno targets |
| **MQL4** | `mitch.mq4` | MetaTrader 4 port |

## Performance Characteristics

- **Message Sizes**: 8-byte header + 32/64/2072-byte bodies
- **Serialization**: 10-20x faster with zero-copy operations  
- **Bandwidth**: 40% reduction vs. standard ITCH
- **Memory**: Fixed-size, 8-byte aligned structures

*See [Performance Details](./model/overview.md#performance--implementation) for optimization strategies.*

## Getting Started

1. **📖 Study the Protocol**: Start with [Protocol Overview](./model/overview.md)
2. **💻 Choose Implementation**: Select from `./impl/` (Rust, TypeScript, MQL4)
3. **🚀 Build Your First App**: See examples below and in `./impl/examples/`

*Complete implementation guide: [Protocol Overview](./model/overview.md#getting-started)*

## Quick Examples

### Basic Trade Message (Rust)
```rust
let trade = Trade {
    ticker_id: 0x03006F301CD00000,  // EUR/USD spot
    price: 1.08750,
    quantity: 1000000,
    trade_id: 123456,
    side: OrderSide::Buy,
    _padding: [0; 7],
};
let bytes = trade.pack(); // 32 bytes, zero-copy
```

### Pub/Sub Filtering (TypeScript)
```typescript
// Subscribe to Binance EUR/USD ticks
const channelId = Channel.generate(101, 's');
subscriber.subscribe(channelId.toString());
```

*Complete examples and use cases: [Protocol Overview](./model/overview.md)*

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
