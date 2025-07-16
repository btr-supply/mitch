# MITCH Protocol Overview

**MITCH (Moded ITCH)** is a high-performance, transport-agnostic binary protocol for financial market data serialization. Designed for ultra-low latency applications where speed and minimal overhead are critical.

## Key Design Principles

*   **ITCH-Inspired:** Message structure based on official ITCH format, but optimized for lower footprint
*   **Performance:** Fixed-width fields and memory alignment enable zero-copy parsing  
*   **Compactness:** Efficient 8-byte trading pair identifier and minimalist message design
*   **Transport-Agnostic:** Pure serialization format suitable for TCP, UDP, or file storage
*   **Cross-Platform:** All multi-byte fields use Little-Endian byte order encoding
*   **Pub/Sub Optimized:** Channel ID system enables efficient message filtering and routing

## Protocol Benefits

- **10-40% lighter messages** compared to NASDAQ's official ITCH protocol
- **Fixed 8-byte header** regardless of batch size
- **Single 8-byte ticker ID** for any asset class and exchange
- **32-byte body alignment** for optimal memory access
- **Zero-copy parsing** with fixed-width fields
- **Predictable batching** up to 255 objects per message
- **O(1) hash-based message routing** in pub/sub systems

## Message Types

| Type | Code | Purpose | Specifications |
|------|------|---------|----------------|
| Trade | `t` | Executed transactions | [trade.md](./trade.md) |
| Order | `o` | Order lifecycle events | [order.md](./order.md) |
| Tick | `s` | Ticker Bid/ask snapshots | [tick.md](./tick.md) |
| OrderBook | `b` | Fixed-size order book snapshots | [order-book.md](./order-book.md) |
| Index | `i` | Synthetic aggregated data | [index.md](./index.md) |

## Core Components

- **[Ticker ID System](./ticker.md)**: 8-byte encoding for any financial instrument
- **[Asset Classification](./asset.md)**: Standardized asset class and instrument type system
- **[Messaging Format](./messaging.md)**: Unified header and batching architecture
- **[Order Book Aggregation](./order-book.md)**: Ultra-light 2KB order book with configurable bins

## Implementation Languages

Complete reference implementations available in:
- **Rust** (`../impl/mitch.rs`) - Core binary implementation
- **TypeScript** (`../impl/mitch.ts`) - Web/Node.js environments  
- **MQL4** (`../impl/mitch.mq4`) - MetaTrader trading systems

## Use Cases

### Real-Time Trading
- Low latency, high frequency index prices are ideal for quantitative modeling of price action
- Index prices by nature filter out noise from signal
- Multi-exchange arbitrage
- Real-time risk management and position monitoring

### Compliance, Oracles & Accounting
- Precision accounting using verifiable, trusted data
- Monitor your own organization's spreads and quotes using high quality aggregates

### Settlement & Cross-border Payments
- High frequency marked price are ideal for transaction settlement
- High precision conversion for payment processing

### Market Data Distribution  
- Pub/sub filtering with Channel IDs
- Topic-based routing for Kafka/Redis/ZMQ/NATS etc.
- Bandwidth optimization with multiplexing and selective subscriptions

### Analytics & Research
- Market microstructure and price-action analysis with spread metrics
- Liquidity analysis with aggregated volumes
- Data quality monitoring with confidence scores

## Architecture Requirements

- **Little-Endian Assumption**: x86_64, ARM64 and RISC-V architectures
- **Memory Alignment**: 8-byte aligned structures
- **Fixed-Width Fields**: All data types have predictable sizes
- **Unsafe Casting**: Direct memory mapping for zero-copy operations

## Data Types & Endianness

MITCH uses standard, fixed-width data types. All multi-byte fields MUST be encoded in **Little-Endian** byte order. Floating points follow IEEE 754 standard.

| Type     | Size (bytes) | Description                          |
|----------|--------------|--------------------------------------|
| `u8`     | 1            | 8-bit unsigned integer / ASCII char  |
| `i16`    | 2            | 16-bit signed integer                |
| `u16`    | 2            | 16-bit unsigned integer              |
| `i32`    | 4            | 32-bit signed integer                |
| `u32`    | 4            | 32-bit unsigned integer              |
| `u48`    | 6            | 48-bit unsigned integer (timestamp)  |
| `u64`    | 8            | 64-bit unsigned integer              |
| `f64`    | 8            | 64-bit IEEE 754 floating-point number|

## Getting Started

1. **Choose Implementation**: Select language-specific model from `../impl/`
2. **Review Message Specs**: Study relevant message type documentation
3. **Implement Pack/Unpack**: Use provided serialization functions
4. **Add Channel IDs**: Implement pub/sub filtering as needed
5. **Test Integration**: Validate against example implementations

For detailed specifications, see the individual message type documentation linked above.
