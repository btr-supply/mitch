# MITCH: Protocol Specification

## 1. Overview

**MITCH (Moded ITCH)** is a high-performance, transport-agnostic binary protocol for financial market data serialization. It is designed for ultra-low latency applications where speed and minimal overhead are critical.

Inspired by the NASDAQ ITCH protocol, MITCH adapts the core concepts while making strategic optimizations for read-only multi-casting. **Key divergences from official ITCH:**
- **No Stock Locate nor double order tracking**
- **Consolidated 8-byte trading pair/ticker identifier** instead of separate instrument references
- **Custom Tick, Ticker Snapshot and Order Book structs**
- **Index message support** for synthetic aggregated market data across exchanges
- **Standardized channel IDs** for efficient pub/sub filtering and routing
- **Result: 10-40% lighter messages** compared to NASDAQ's official ITCH protocol

This makes MITCH ideal for very low latency messaging, to use with any market data from Forex and Equities to Crypto.

### Core Principles

*   **ITCH-Inspired:** Message structure based on official ITCH format, but tweaked for lower footprint and to be quote agnostic.
*   **Performance:** Fixed-width fields and memory alignment enable zero-copy parsing where possible.
*   **Compactness:** An efficient 8-byte trading pair identifier and minimalist message design keep data payloads small.
*   **Transport-Agnostic:** Pure serialization format suitable for TCP (ZMQ, NATS, Kafka...), UDP (KCP, RUDP, MoldUDP64...), or file storage.
*   **Cross-Platform:** All multi-byte fields use Little-Endian byte order encoding.
*   **Pub/Sub Optimized:** Channel ID system enables efficient message filtering and routing in distributed systems.

## 2. Data Types & Endianness

MITCH uses standard, fixed-width data types. All multi-byte fields MUST be encoded in **Little-Endian** byte order.

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

## 3. Message Format

All MITCH messages use a unified 8-byte header, which provides basic information about the message payload that follows. This consistent structure simplifies parsing logic.

```
┌─────────────────┬──────────────────────────────────┐
│ Header (8B)     │ Message Body Array (Variable)    │
└─────────────────┴──────────────────────────────────┘
```

See [messaging.md](messaging.md) for detailed header structure and messaging architecture.

---

## 4. Message Specifications

All MITCH messages use a unified structure: a common 8-byte header followed by an array of message-specific body structures. Single messages are simply batches of size 1.

### 4.1. Unified Message Header

Every MITCH message begins with this 8-byte header:

| Field        | Offset | Size | Type  | Description                        |
|--------------|--------|------|-------|------------------------------------|
| Message Type | 0      | 1    | `u8`  | ASCII character for message type   |
| Timestamp    | 1      | 6    | `u48` | Nanoseconds since midnight (UTC)   |
| Count        | 7      | 1    | `u8`  | Number of body entries (1-255)     |

### 4.2. Message Type Codes

| Code | Message Name            | Body Structure  | Description                           |
|------|-------------------------|-----------------|---------------------------------------|
| 't'  | Trade Messages          | `Trade[]`       | Trade executions (single or batch)    |
| 'o'  | Order Messages          | `Order[]`       | Order events (single or batch)        |
| 's'  | Tick Messages           | `Tick[]`        | Tick or Ticker snapshots (single or batch)|
| 'b'  | Order Book Messages     | `OrderBook[]`   | Order book snapshots (single or batch)|
| 'i'  | Index Messages          | `Index[]`       | Synthetic aggregated index data (single or batch)|

**Note:** Single messages have `Count = 1`, batch messages have `Count > 1`.

---

### 4.3. Trade Messages (`t`)

Trade messages represent executed transactions.

**Message Structure:**
```
┌─────────────────┬──────────────────────────────┐
│ Header (8B)     │ Trade Array (Count × 32B)    |
└─────────────────┴──────────────────────────────┘
```

**Trade (32 bytes per entry):**

| Field     | Offset | Size | Type  | Description                            |
|-----------|--------|------|-------|----------------------------------------|
| Ticker ID | 0      | 8    | `u64` | See [ticker.md](ticker.md)            |
| Price     | 8      | 8    | `f64` | Execution price                        |
| Quantity  | 16     | 4    | `u32` | Executed volume/quantity               |
| Trade ID  | 20     | 4    | `u32` | **Required** unique trade identifier   |
| Side      | 24     | 1    | `u8`  | `0`: Buy, `1`: Sell                    |
| Padding   | 25     | 7    | `u8[7]` | Padding to 32 bytes                  |

**Total Message Size:** 8 + (Count × 32) bytes

---

### 4.4. Order Messages (`o`)

Order messages represent order lifecycle events (placement, modification, cancellation).

**Message Structure:**
```
┌─────────────────┬──────────────────────────────┐
│ Header (8B)     │ Order Array (Count × 32B)    |
└─────────────────┴──────────────────────────────┘
```

**Order (32 bytes per entry):**

| Field         | Offset | Size | Type    | Description                                    |
|---------------|--------|------|---------|------------------------------------------------|
| Ticker ID     | 0      | 8    | `u64`   | See [ticker.md](ticker.md)                    |
| Order ID      | 8      | 4    | `u32`   | **Required** unique order identifier           |
| Price         | 12     | 8    | `f64`   | Limit/stop price                               |
| Quantity      | 20     | 4    | `u32`   | Order volume/quantity                          |
| Type & Side   | 24     | 1    | `u8`    | Bit 0: Side (0=Buy, 1=Sell), Bits 1-7: Type    |
| Expiry        | 25     | 6    | `u48`   | Expiry timestamp (Unix ms) or 0 for GTC        |
| Padding       | 31     | 1    | `u8`    | Padding to 32 bytes                            |

**Order Type Encoding (Bits 1-7):**
- `0`: Market
- `1`: Limit  
- `2`: Stop
- `3`: Cancel
- `4-127`: Reserved for future use

**Total Message Size:** 8 + (Count × 32) bytes

---

### 4.5. Tick Messages (`s`)

Tick messages provide point-in-time bid/ask snapshots.

**Message Structure:**
```
┌─────────────────┬───────────────────────────────┐
│ Header (8B)     │ Tick Array (Count × 32B)      |
└─────────────────┴───────────────────────────────┘
```

**Tick (32 bytes per entry):**

| Field       | Offset | Size | Type  | Description                    |
|-------------|--------|------|-------|--------------------------------|
| Ticker ID   | 0      | 8    | `u64` | See [ticker.md](ticker.md)    |
| Bid Price   | 8      | 8    | `f64` | Best bid price                 |
| Ask Price   | 16     | 8    | `f64` | Best ask price                 |
| Bid Volume  | 24     | 4    | `u32` | Volume at best bid             |
| Ask Volume  | 28     | 4    | `u32` | Volume at best ask             |

**Total Message Size:** 8 + (Count × 32) bytes

---

### 4.6. Order Book Messages (`b`)

Order book messages provide liquidity snapshots for one side of an order book.

**Message Structure:**
```
┌─────────────────┬────────────────────────────────────┐
│ Header (8B)     │ OrderBook Array (Variable)         │
└─────────────────┴────────────────────────────────────┘
```

**OrderBook (Variable size per entry):**

**Order Book Header (32 bytes):**
| Field       | Offset | Size    | Type    | Description                                |
|-------------|--------|---------|---------|--------------------------------------------|
| Ticker ID   | 0      | 8       | `u64`   | See [ticker.md](ticker.md)                |
| First Tick  | 8      | 8       | `f64`   | Starting price level                       |
| Tick Size   | 16     | 8       | `f64`   | Price increment per tick                   |
| Num Ticks   | 24     | 2       | `u16`   | Number of `Volume` entries that follow     |
| Side        | 26     | 1       | `u8`    | `0`: Bids, `1`: Asks                       |
| Padding     | 27     | 5       | `u8[5]` | Padding to 32 bytes                        |

**Liquidity Array (repeats `Num Ticks` times):**
| Field  | Offset | Size | Type  | Description                 |
|--------|--------|------|-------|-----------------------------|
| Volume | 0      | 4    | `u32` | Volume at this price level  |

**Total Message Size:** 8 + Σ(32 + Num Ticks × 4) bytes per order book entry

---

### 4.7. Index Messages (`i`)

Index messages provide synthetic aggregated market data that reflects the state of a financial instrument across all of its markets. It is a derived, enriched version of a Tick message with additional metrics for multi-market analysis.

**Message Structure:**
```
┌─────────────────┬───────────────────────────────┐
│ Header (8B)     │ Index Array (Count × 64B)     |
└─────────────────┴───────────────────────────────┘
```

**Index (64 bytes per entry):**

| Field       | Offset | Size | Type  | Description                              |
|-------------|--------|------|-------|------------------------------------------|
| Ticker ID   | 0      | 8    | `u64` | See [ticker.md](ticker.md)              |
| Mid         | 8      | 8    | `f64` | Mid price (synthetic)                    |
| vbid        | 16     | 4    | `u32` | Bid volume (aggregated sell volume)      |
| vask        | 20     | 4    | `u32` | Ask volume (aggregated buy volume)       |
| mspread     | 24     | 4    | `i32` | Mean spread (1e-9 pbp)                   |
| bbido       | 28     | 4    | `i32` | Best bid offset (1e-9 pbp)               |
| basko       | 32     | 4    | `i32` | Best ask offset (1e-9 pbp)               |
| wbido       | 36     | 4    | `i32` | Worst bid offset (1e-9 pbp)              |
| wasko       | 40     | 4    | `i32` | Worst ask offset (1e-9 pbp)              |
| vforce      | 44     | 2    | `u16` | Volatility force (0-10000)               |
| lforce      | 46     | 2    | `u16` | Liquidity force (0-10000)                |
| tforce      | 48     | 2    | `i16` | Trend force (-10000-10000)               |
| mforce      | 50     | 2    | `i16` | Momentum force (-10000-10000)            |
| confidence  | 52     | 1    | `u8`  | Data quality (0-100, 100=best)           |
| rejected    | 53     | 1    | `u8`  | Number of sources rejected               |
| accepted    | 54     | 1    | `u8`  | Number of sources accepted               |
| Padding     | 55     | 9    | `u8[9]` | Padding to 64 bytes                    |

**Field Explanations:**
- **Mid**: Synthetic mid price aggregated across markets
- **vbid/vask**: Aggregated volumes on bid/ask sides
- **mspread**: Mean spread in price basis points (1e-9 precision)
- **Offsets (bbido, basko, etc.)**: Relative offsets from reference price in 1e-9 pbp
- **Forces (vforce, lforce, etc.)**: Quantitative metrics for volatility, liquidity, trend, momentum
- **confidence/rejected/accepted**: Data quality indicators showing aggregation reliability

**Total Message Size:** 8 + (Count × 64) bytes

---

## 5. Custom ID Systems

### 5.1 Ticker ID Encoding (8-Byte Format)

**CRITICAL:** The 8-byte **Ticker ID** is the cornerstone of the MITCH protocol. It uniquely identifies a tradable asset pair by encoding the **base instrument** being traded and the **quote asset** used for its price.

**Complete Specification:** See [ticker.md](ticker.md) for comprehensive 8-byte ticker encoding details including:
- 64-bit field allocation (instrument type, base/quote assets, sub-type)
- Asset class definitions and examples
- Encoding/decoding functions
- Validation rules
- Real-world examples (EUR/USD, BTC/USDT, options, etc.)

### 5.2 Channel ID Encoding

**MITCH v2 introduces a standardized 32-bit Channel ID system** for efficient message filtering in pub/sub environments (Redis RESP3, Kafka topics, MQTT topics, gRPC streaming, etc.).

#### Channel ID Format (32-bit)

```
┌─────────────────────────┬──────────────────┬──────────────────┐
│ Market Provider ID      │ Message Type     │ Padding          │
│ (16 bits)               │ (8 bits)         │ (8 bits)         │
└─────────────────────────┴──────────────────┴──────────────────┘
```

**Field Definitions:**

**Market Provider ID (16 bits):** References the market provider from `market-providers.csv`
- Examples: Binance (00101), NYSE (00861)
- Range: 0-65535

**Message Type (8 bits):** ASCII-encoded MITCH message type
- `'t'` (116): Trade messages
- `'o'` (111): Order messages  
- `'s'` (115): Tick messages
- `'b'` (113): Order Book messages
- `'i'` (105): Index messages

**Padding (8 bits):** Reserved for future extensions (currently `0x00`)

#### Channel ID Examples:

**Binance Tick Channel:**
- Market Provider ID: 00101 (0x0065), Message Type: 's' (0x73), Padding: 0x00
- **Channel ID:** `0x00657300` = `6648576`

**NYSE Index Channel:**
- Market Provider ID: 00861 (0x035D), Message Type: 'i' (0x69), Padding: 0x00
- **Channel ID:** `0x035D6900` = `56453376`

#### Usage in Pub/Sub Systems

**Redis RESP3:**
```redis
SUBSCRIBE 6648576        # Binance ticks only
SUBSCRIBE 56453376       # NYSE indices only
```

**Kafka Topics:**
```
6648576            # Binance tick topic
56453376           # NYSE indices topic
```

## 6. Implementation Notes

### 6.1. Performance Optimizations: Unsafe Casting vs. Bit Manipulation

**MITCH implementations use two distinct optimization strategies:**

#### 6.1.1. Unsafe Memory Casting (No-Copy)

**Used for:** All main message structs with byte-aligned fields
- `Trade`, `Order`, `Tick` (32 bytes), `Index` (64 bytes), `OrderBook` components
- **Method:** Direct `unsafe { std::mem::transmute() }` casting
- **Performance:** 10-20x faster than byte-by-byte serialization
- **Requirements:** Little-endian target, `#[repr(C, packed)]` attribute

#### 6.1.2. Bit Manipulation (Non-Byte-Aligned Fields)

**Used for:** Ticker ID and Channel ID generation/extraction
- **Reason:** Fields don't align to byte boundaries, requiring bit-level manipulation
- **Method:** Manual bit shifting and masking operations

### 6.2. Memory Alignment and Performance

All message bodies are padded to be multiples of **32 bytes** (except Index at 64 bytes). This ensures 8-byte alignment for improved memory access performance and zero-copy operations.

### 6.3. Batching and Channel Integration

Channel IDs work seamlessly with the batching system for complete message routing:

```rust
// Create channel ID for EURUSD ticks from Interactive Brokers
let ticker_id = 0x03006F301CD00000u64;  // EUR/USD spot
let channel_id = Channel::generate(00691, b's');  // IBKR + ticks

// Subscriber can filter by both ticker and channel
subscribe_to_ticker_on_channel(ticker_id, channel_id);
```

### 6.4. Index Aggregation and Confidence

The `confidence` field (0-100) indicates data quality:
- **95-100**: High confidence (0-10ms latency, <2% rejections)
- **80-94**: Good confidence (10-30ms latency, <5% rejections)
- **60-79**: Medium confidence (30-50ms latency, <8% rejections)
- **40-59**: Low confidence (50-70ms latency, <13% rejections)
- **0-39**: Very low confidence (>70ms latency, >13% rejections)
