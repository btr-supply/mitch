# MITCH: Moded ITCH Protocol Specification

## 1. Overview

**MITCH (Moded ITCH)** is a high-performance, transport-agnostic binary protocol for financial market data serialization. It is designed for ultra-low latency applications where speed and minimal overhead are critical.

Inspired by the NASDAQ ITCH protocol, MITCH adapts the core concepts while making strategic optimizations for read-only multi-casting. **Key divergences from official ITCH:**
- **No Stock Locate nor double order tracking**
- **Consolidated 8-byte trading pair/ticker identifier** instead of separate instrument references
- **Custom Tick, Ticker Snapshot and Order Book structs**
- **Result: 10-40% lighter messages** compared to NASDAQ's official ITCH protocol

This makes MITCH ideal for very low latency messaging, to use with any market data from Forex and Equities to Crypto.

### Core Principles

*   **ITCH-Inspired:** Message structure based on official ITCH format, but tweaked for lower footprint and to be quote agnostic.
*   **Performance:** Fixed-width fields and memory alignment enable zero-copy parsing where possible.
*   **Compactness:** An efficient 8-byte trading pair identifier and minimalist message design keep data payloads small.
*   **Transport-Agnostic:** Pure serialization format suitable for TCP (ZMQ, NATS, Kafka...), UDP (KCP, RUDP, MoldUDP64...), or file storage.
*   **Cross-Platform:** All multi-byte fields use Big-Endian (network byte order) encoding.

## 2. Data Types & Endianness

MITCH uses standard, fixed-width data types. All multi-byte fields MUST be encoded in **Big-Endian** byte order.

| Type     | Size (bytes) | Description                          |
|----------|--------------|--------------------------------------|
| `u8`     | 1            | 8-bit unsigned integer / ASCII char  |
| `u16`    | 2            | 16-bit unsigned integer              |
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

See `Section 5.1` for the detailed header structure.

---

## 4. Ticker ID Encoding (8-Byte Format)

**CRITICAL:** The 8-byte **Ticker ID** is the cornerstone of the MITCH protocol. It uniquely identifies a tradable asset pair by encoding the **base instrument** being traded and the **quote asset** used for its price.

This distinction is vital. For example, `AAPL/USD` and `AAPL/EUR` are different trading pairs and will have different Ticker IDs, even though the base instrument (`AAPL`) is the same.

### Bit Allocation (64 bits)

The 64 bits of the Ticker ID are allocated as follows, allowing for a vast and filterable address space. The "instrument" itself is defined by the first 44 bits, while the final 20 bits specify the currency it is priced in.

```
┌───────────────┬───────────────────────┬───────────────────────┬───────────────────────┐
│ Inst. Type    │ Base Asset (class+id) │ Quote Asset (class+id)│ Sub-Type              │
│ (4 bits)      │ 20 bits    (4+16 bits)│ 20 bits    (4+16 bits)│ (20 bits)             │
└───────────────┴───────────────────────┴───────────────────────┴───────────────────────┘
```

**Important:** The component fields are packed together to form the final `u64` ID:
- **Instrument Type (4 bits):** The category of the financial product.
- **Base Asset (20 bits):** The core asset being traded. Composed of a 4-bit asset class and a 16-bit unique identifier within that class.
- **Quote Asset (20 bits):** The asset used for pricing (e.g., USD, EUR, BTC). Always a spot/cash equivalent.
- **Sub-Type (20 bits):** Additional salt, such as hashed strike price and expiry for options.

### Field Definitions

#### 4.1. Instrument Type (4 bits)
Defines the type of the **base instrument** being traded.

| ID   | Type                 | Description                                    |
|------|----------------------|------------------------------------------------|
| `0x0`| Spot                 | Direct asset trading                           |
| `0x1`| Future               | Standardized futures contract                  |
| `0x2`| Forward              | Custom forward contract                        |
| `0x3`| Swap                 | Interest rate or currency swap                 |
| `0x4`| Perpetual Swap       | Crypto perpetual futures                       |
| `0x5`| CFD                  | Contract for difference                        |
| `0x6`| Call Option          | Call option contract                           |
| `0x7`| Put Option           | Put option contract                            |
| `0x8`| Digital Option       | Binary/digital option                          |
| `0x9`| Barrier Option       | Barrier option contract                        |
| `0xA`| Warrant              | Warrant contract                               |
| `0xB`| Prediction Contract  | Contract based on predicted outcomes          |
| `0xC`| Structured Product   | Financial instruments with multiple components|
| `0xD`| *Reserved*           | *Reserved for future use*                      |
| `0xE`| *Reserved*           | *Reserved for future use*                      |
| `0xF`| *Reserved*           | *Reserved for future use*                      |

#### 4.2. Asset Classes (4 bits per asset)

| ID   | Class                 | Examples                           |
|------|-----------------------|------------------------------------|
| `0x0`| Equities              | AAPL, MSFT, GOOGL                  |
| `0x1`| Corporate Bonds       | Corporate debt securities          |
| `0x2`| Sovereign Debt        | Government bonds, treasuries       |
| `0x3`| Forex                 | EUR, USD, JPY, GBP                 |
| `0x4`| Commodities           | WTI, Brent, Gold, Silver           |
| `0x5`| Precious Metals       | Gold, Silver, Platinum             |
| `0x6`| Real Estate           | REITs, property indices            |
| `0x7`| Crypto Assets         | BTC, ETH, USDC, SOL                |
| `0x8`| Private Markets       | Investments in private companies   |
| `0x9`| Collectibles          | Art, antiques, rare items          |
| `0xA`| Infrastructure        | Investments in physical assets     |
| `0xB`| Indices & Index Products| Market indices and related products|
| `0xC`| Structured Products   | Financial instruments with multiple components|
| `0xD`| Cash & Equivalents    | Cash and cash-like instruments     |
| `0xE`| Loans & Receivables   | Debt instruments and receivables   |
| `0xF`| *Reserved*            | *Reserved for future use*          |

#### 4.3. Trading Pair Examples

**EUR/USD Spot:**
- Instrument Type: `0x0` (Spot)
- Base: `0x3` (Forex) + `111` (EUR) = `0x3006F` (Forex/EUR)
- Quote: `0x3` (Forex) + `461` (USD) = `0x301CD` (Forex/USD)
- Sub-Type: `0x00000`
- **Result:** `0x03006F301CD00000` (`216295034546290688` in decimal)

**AAPL $190 Call Option (30 days) settled in USD:**
- Instrument Type: `0x6` (Call Option)
- Base: `0x0` (Equity) + `831` (AAPL) = `0x0033F` (Equity/AAPL)
- Quote: `0x3` (Forex) + `461` (USD) = `0x301CD` (Forex/USD)
- Sub-Type: Strike + Expiry 20bit encoding (dummy 0x00000)
- **Result:** `0x60033F301CD00000` (`6918442928445587456` in decimal)

---

## 5. Message Specifications

All MITCH messages use a unified structure: a common 8-byte header followed by an array of message-specific body structures. Single messages are simply batches of size 1.

### 5.1. Unified Message Header

Every MITCH message begins with this 8-byte header:

| Field        | Offset | Size | Type  | Description                        |
|--------------|--------|------|-------|------------------------------------|
| Message Type | 0      | 1    | `u8`  | ASCII character for message type   |
| Timestamp    | 1      | 6    | `u48` | Nanoseconds since midnight (UTC)   |
| Count        | 7      | 1    | `u8`  | Number of body entries (1-255)     |

### 5.2. Message Type Codes

The `Message Type` code indicates how to parse the body array that follows.

| Code | Message Name            | Body Structure      | Description                           |
|------|-------------------------|---------------------|---------------------------------------|
| 't'  | Trade Messages          | `TradeBody[]`       | Trade executions (single or batch)    |
| 'o'  | Order Messages          | `OrderBody[]`       | Order events (single or batch)        |
| 's'  | Ticker Messages         | `TickerBody[]`      | Ticker snapshots (single or batch)    |
| 'q'  | Order Book Messages     | `OrderBookBody[]`   | Order book snapshots (single or batch)|

**Note:** Single messages have `Count = 1`, batch messages have `Count > 1`.

---

### 5.3. Trade Messages (`t`)

Trade messages represent executed transactions.

#### Message Structure:
```
┌─────────────────┬──────────────────────────────┐
│ Header (8B)     │ TradeBody Array (Count × 32B)|
└─────────────────┴──────────────────────────────┘
     Bits 0-7                Bits 8-40...
```

#### TradeBody (32 bytes per entry):

| Field     | Offset | Size | Type  | Description                            |
|-----------|--------|------|-------|----------------------------------------|
| Ticker ID | 0      | 8    | `u64` | 8-byte unique ticker identifier        |
| Price     | 8      | 8    | `f64` | Execution price                        |
| Quantity  | 16     | 4    | `u32` | Executed volume/quantity               |
| Trade ID  | 20     | 4    | `u32` | **Required** unique trade identifier   |
| Side      | 24     | 1    | `u8`  | `0`: Buy, `1`: Sell                    |
| Padding   | 25     | 7    | `u8[7]` | Padding to 32 bytes                  |

**Total Message Size:** 8 + (Count × 32) bytes

---

### 5.4. Order Messages (`o`)

Order messages represent order lifecycle events (placement, modification, cancellation).

#### Message Structure:
```
┌─────────────────┬──────────────────────────────┐
│ Header (8B)     │ OrderBody Array (Count × 32B)|
└─────────────────┴──────────────────────────────┘
     Bits 0-7                Bits 8-40...
```

#### OrderBody (32 bytes per entry):

| Field         | Offset | Size | Type    | Description                                    |
|---------------|--------|------|---------|------------------------------------------------|
| Ticker ID     | 0      | 8    | `u64`   | 8-byte unique ticker identifier                |
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

### 5.5. Ticker Messages (`s`)

Ticker messages provide point-in-time bid/ask snapshots.

#### Message Structure:
```
┌─────────────────┬───────────────────────────────┐
│ Header (8B)     │ TickerBody Array (Count × 32B)|
└─────────────────┴───────────────────────────────┘
     Bits 0-7                Bits 8-40...
```

#### TickerBody (32 bytes per entry):

| Field       | Offset | Size | Type  | Description                    |
|-------------|--------|------|-------|--------------------------------|
| Ticker ID   | 0      | 8    | `u64` | 8-byte unique ticker identifier|
| Bid Price   | 8      | 8    | `f64` | Best bid price                 |
| Ask Price   | 16     | 8    | `f64` | Best ask price                 |
| Bid Volume  | 24     | 4    | `u32` | Volume at best bid             |
| Ask Volume  | 28     | 4    | `u32` | Volume at best ask             |

**Total Message Size:** 8 + (Count × 32) bytes

---

### 5.6. Order Book Messages (`q`)

Order book messages provide liquidity snapshots for one side of an order book.

#### Message Structure:
```
┌─────────────────┬────────────────────────────────────┐
│ Header (8B)     │ OrderBookBody Array (Variable)     │
└─────────────────┴────────────────────────────────────┘
     Bits 0-7                Bits 8-N...
```

#### OrderBookBody (Variable size per entry):

**Order Book Header (32 bytes):**
| Field       | Offset | Size    | Type    | Description                                |
|-------------|--------|---------|---------|--------------------------------------------|
| Ticker ID   | 0      | 8       | `u64`   | 8-byte unique ticker identifier            |
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

## 6. Implementation Notes

### 6.1. Bit Field Encoding for Orders

The `Type & Side` field in order messages uses the following bit layout:
```
┌──────────┬────────────────────────────┐
│ Side (1B)│       Order Type (7B)      │
└──────────┴────────────────────────────┘
   Bit 0              Bits 1-7
```

**Extraction:**
- Side: `type_and_side & 0x01`
- Order Type: `(type_and_side >> 1) & 0x7F`

**Encoding:**
- Combined: `(order_type << 1) | side`

### 6.2. Alignment and Performance

All message bodies (`TradeBody`, `OrderBody`, `TickerBody`, and `OrderBookBody`'s header) are padded to be **32 bytes**. This ensures 8-byte alignment, which can improve performance for certain memory access patterns and zero-copy operations, at the cost of a few extra bytes.

### 6.3. Batching

This unified approach provides several benefits:
- **Consistent parsing:** All messages follow the same header + array pattern
- **Reduced complexity:** No separate single/batch message types
- **Space efficiency:** Single messages have minimal overhead
- **Scalability:** Support for up to 255 entries per message
