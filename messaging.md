# MITCH Messaging Architecture

*Part of the [MITCH Protocol](./model/overview.md) - see [complete specification](./model/overview.md) for context.*

## Unified Message Format

All MITCH messages use a consistent structure: a fixed 8-byte header followed by an array of message-specific body structures.

```
┌─────────────────┬──────────────────────────────────┐
│ Header (8B)     │ Message Body Array (Variable)    │
└─────────────────┴──────────────────────────────────┘
```

## Message Header Structure

Every MITCH message begins with this unified 8-byte header:

| Field        | Offset | Size | Type  | Description                        |
|--------------|--------|------|-------|------------------------------------|
| Message Type | 0      | 1    | `u8`  | ASCII character for message type   |
| Timestamp    | 1      | 6    | `u48` | Nanoseconds since midnight (UTC)   |
| Count        | 7      | 1    | `u8`  | Number of body entries (1-255)     |

**Note**: All multi-byte fields use **Little-Endian** byte order encoding.

## Message Type Codes

| Code | Message Name       | Body Structure     | Body Size | Description                           |
|------|--------------------|--------------------|-----------|---------------------------------------|
| `t`  | [Trade Messages](./model/trade.md)     | `Trade[]`          | 32B each  | Trade executions (single or batch)    |
| `o`  | [Order Messages](./model/order.md)     | `Order[]`          | 32B each  | Order events (single or batch)        |
| `s`  | [Tick Messages](./model/tick.md)      | `Tick[]`           | 32B each  | Tick snapshots (single or batch)      |
| `b`  | [Order Book](./model/order-book.md)         | `OrderBook[]`      | 2072B each| Order book snapshots                  |
| `i`  | [Index Messages](./model/index.md)     | `Index[]`          | 64B each  | Synthetic aggregated data             |

## Batching Architecture

### Single vs Batch Messages
- **Count = 1**: Single message payload (Total: 8 + body_size bytes)
- **Count > 1**: Multiple messages of same type (Total: 8 + count × body_size bytes)
- **Maximum**: Up to 255 entries per message

### Benefits
- Consistent parsing (header + array pattern)
- Reduced syscall overhead for bulk transfers
- Fixed 8-byte overhead regardless of batch size

## Timestamp Handling

- **Type**: `u48` (48-bit) packed into 6 bytes
- **Units**: Nanoseconds since midnight UTC
- **Range**: 0 to 281,474,976,710,655 (covers >24 hours)

```rust
// Pack u48 timestamp into 6 bytes
let timestamp_bytes = timestamp.to_le_bytes();
header_bytes[1..7].copy_from_slice(&timestamp_bytes[0..6]);

// Unpack 6 bytes to u64 timestamp  
let mut timestamp_bytes = [0u8; 8];
timestamp_bytes[0..6].copy_from_slice(&header_bytes[1..7]);
let timestamp = u64::from_le_bytes(timestamp_bytes);
```

## Performance Optimizations

### Memory Alignment
- **32-byte**: Trade, Order, Tick messages
- **64-byte**: Index messages (enriched data)
- **2072-byte**: Order Book messages (fixed-size)

### Zero-Copy Operations
Direct memory mapping without intermediate copying using `unsafe { std::mem::transmute() }` for byte-aligned structs.

**See [overview.md](overview.md#performance--implementation) for performance optimization strategies and implementation details.**

## Channel ID System

**MITCH v2 introduces a standardized 32-bit Channel ID system** for efficient message filtering in pub/sub environments (Redis RESP3, Kafka topics, MQTT topics, gRPC streaming, etc.).

### Channel ID Format (32-bit)

```
┌─────────────────────────┬──────────────────┬──────────────────┐
│ Market Provider ID      │ Message Type     │ Padding          │
│ (16 bits)               │ (8 bits)         │ (8 bits)         │
└─────────────────────────┴──────────────────┴──────────────────┘
```

### Field Definitions

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

### Channel ID Examples

**Binance Tick Channel:**
- Market Provider ID: 00101 (0x0065), Message Type: 's' (0x73), Padding: 0x00
- **Channel ID:** `0x00657300` = `6648576`

**NYSE Index Channel:**
- Market Provider ID: 00861 (0x035D), Message Type: 'i' (0x69), Padding: 0x00
- **Channel ID:** `0x035D6900` = `56453376`

### Usage in Pub/Sub Systems

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

### Integration with Batching

Channel IDs work seamlessly with the batching system for complete message routing:

```rust
// Create channel ID for EURUSD ticks from Interactive Brokers
let ticker_id = 0x03006F301CD00000u64;  // EUR/USD spot
let channel_id = Channel::generate(00691, b's');  // IBKR + ticks

// Subscriber can filter by both ticker and channel
subscribe_to_ticker_on_channel(ticker_id, channel_id);
```

---

# Networking Integration

## Transport-Agnostic Design

MITCH protocol supports multiple transport layers through a unified interface:

- **Redis** (RESP3): Pub/sub + key-value storage
- **WebTransport** (HTTP/3): High-performance streaming over QUIC with singleton server
- **Multiple Transports**: Push to Redis and WebTransport simultaneously

### WebTransport Singleton Architecture

MITCH v2 introduces a **singleton WebTransport server** that auto-instantiates on first use, eliminating the need for external brokers:

#### Key Features

1. **Auto-Instantiation**: Server starts automatically when first publisher is created
2. **Zero Configuration**: No external setup required
3. **Multiplexed Streams**: Handles multiple publishers and subscribers concurrently
4. **Fire-and-Forget**: No acknowledgment overhead for maximum throughput

#### Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Publisher 1   │     │   Publisher 2   │     │   Publisher N   │
│ (Local Client)  │     │ (Local Client)  │     │ (Local Client)  │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         └───────────────────────┴───────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │ WebTransport Singleton │
                    │    Server (Local)      │
                    │   127.0.0.1:4433      │
                    └────────────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Consumer 1    │     │   Consumer 2    │     │   Consumer N    │
│ (External Conn) │     │ (External Conn) │     │ (External Conn) │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

#### Usage Examples

```rust
// Publisher - uses local singleton server
let publisher = WebTransportClient::new_local().await?;
publisher.publish(channel_id, &trade.to_bytes()).await?;

// Consumer - connects to local singleton server
let consumer = WebTransportClient::new_consumer(None).await?;
MessageSubscriber::subscribe(&consumer, &[channel_id], handler)?;
```

#### Performance Benefits

- **No Network Hops**: Publishers communicate directly with local server
- **Shared Memory**: Potential for zero-copy operations between publishers
- **Automatic Scaling**: Server handles concurrent connections efficiently
- **Resource Efficiency**: Single server instance for all local publishers

## Message Publishing

### Basic Usage

```rust
use mitch::{Trade, OrderSide, networking::*};

// Create transport clients
let redis_client = RedisTransport::new("redis://localhost:6379").await?;
let wt_client = WebTransportClient::new("https://localhost:4433").await?;

// Create trade message
let trade = Trade::new(0x1234, 100.0, 1000, 1, OrderSide::Buy)?;

// Push to multiple transports with TTL
let clients: Vec<Box<dyn MessageTransport>> = vec![
    Box::new(redis_client),
    Box::new(wt_client),
];

// Publish with 5-minute TTL for storage
trade.push(&clients, Some(300_000)).await?;
```

### Concurrent Operations

All networking operations are **fully concurrent** for maximum performance:

- **Pub/Sub + Storage**: Run simultaneously, not sequentially
- **Multiple Clients**: All clients receive data in parallel
- **Batch Operations**: Optimized for high-throughput scenarios

### TTL Support

Messages can be stored with expiration times to prevent stale data:

```rust
// Store with 1-hour TTL
trade.push(&clients, Some(3_600_000)).await?;

// Store permanently (no expiration)
trade.push(&clients, None).await?;
```

## Message Subscription

### Basic Subscription

```rust
// Subscribe to trade messages from Binance
let binance_trades = ChannelId::new(101, 't'); // Provider 101 = Binance

redis_client.publish(
    &[binance_trades],
    |channel_id, data| Box::pin(async move {
        println!("Received {} bytes on channel {}", data.len(), channel_id.raw);
        // Process message data
    })
).await?;
```

### Filtered Subscription

```rust
// Subscribe to specific tickers only
let ticker_filter = vec![
    0x1234567890ABCDEF,  // EURUSD
    0x2345678901BCDEF0,  // GBPUSD
];

redis_client.sub_filter(
    &[binance_trades],
    &ticker_filter,
    |channel_id, ticker_id, data| Box::pin(async move {
        println!("Received data for ticker {} on channel {}", 
                ticker_id, channel_id.raw);
        // Process filtered message
    })
).await?;
```

### Multi-Channel Subscription

```rust
// Subscribe to multiple message types
let channels = vec![
    ChannelId::new(101, 't'), // Binance trades
    ChannelId::new(101, 's'), // Binance ticks
    ChannelId::new(102, 't'), // Coinbase trades
];

redis_client.publish(&channels, handler).await?;
```

## Storage Operations

### Key-Value Storage

```rust
// Store single message
let key = format!("trade:{}", trade.trade_id);
redis_client.set(&key, &trade.to_bytes()).await?;

// Store with expiration
redis_client.set_ex(&key, &trade.to_bytes(), 3600_000).await?;

// Bulk operations
let pairs = vec![
    ("trade:1", trade1.to_bytes().as_slice()),
    ("trade:2", trade2.to_bytes().as_slice()),
];
redis_client.mset(&pairs).await?;
```

### Automatic Key Generation

When pushing messages, keys are automatically generated:

```
Format: mitch:{channel_id}:{ticker_id}:{timestamp_ms}
Example: mitch:6648576:1234567890ABCDEF:1699123456789
```

## Batch Operations

### High-Throughput Batching

```rust
use mitch::networking::batch::MessageBatch;

let channel_id = ChannelId::new(101, 't');
let mut batch = MessageBatch::new(channel_id);

// Add multiple trades
for trade in trades {
    batch.add(trade);
}

// Push entire batch (up to 255 messages)
batch.push(&clients, Some(300_000)).await?;
```

### Batch Benefits

- **Single Network Operation**: Reduces syscall overhead
- **MITCH Protocol Compliant**: Includes proper header with count
- **Concurrent Execution**: All transports receive data simultaneously
- **Memory Efficient**: Zero-copy serialization

## Error Handling

### Transport Errors

```rust
match trade.push(&clients, None).await {
    Ok(()) => println!("Success"),
    Err(NetworkError::Redis(msg)) => eprintln!("Redis error: {}", msg),
    Err(NetworkError::WebTransport(msg)) => eprintln!("WebTransport error: {}", msg),
    Err(NetworkError::Connection(msg)) => eprintln!("Connection error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

### Auto-Reconnection

Transport clients handle reconnection logic externally:

```rust
// Redis with connection pooling
let redis_client = RedisTransport::new("redis://localhost:6379").await?;

// Connection manager handles reconnections automatically
// No manual retry logic needed in application code
```

## Performance Characteristics

### Concurrent Publishing

All operations execute in parallel:

```rust
// This executes both operations simultaneously:
// 1. PUBLISH to Redis channel
// 2. SET to Redis key-value store
trade.push_one(&redis_client, Some(ttl)).await?;
```

### Zero-Copy Operations

- **Message Serialization**: Direct memory mapping
- **Batch Operations**: Contiguous memory layout
- **Transport Layer**: Minimal data copying

### Throughput Optimization

- **Batching**: Up to 255 messages per operation
- **Concurrent Clients**: Multiple transports in parallel
- **Async I/O**: Non-blocking operations throughout
- **Connection Pooling**: Efficient resource utilization

## Integration Examples

### Market Data Publisher

```rust
async fn publish_market_data(
    clients: &[Box<dyn MessageTransport>],
    trades: Vec<Trade>
) -> Result<(), NetworkError> {
    // Create batch for high throughput
    let channel_id = ChannelId::new(101, 't');
    let mut batch = MessageBatch::new(channel_id);
    
    for trade in trades {
        batch.add(trade);
        
        // Flush batch when full
        if batch.len() >= 255 {
            batch.push(clients, Some(300_000)).await?;
            batch = MessageBatch::new(channel_id);
        }
    }
    
    // Flush remaining messages
    if !batch.is_empty() {
        batch.push(clients, Some(300_000)).await?;
    }
    
    Ok(())
}
```

### Market Data Consumer

```rust
async fn consume_market_data(
    redis_client: &RedisTransport
) -> Result<(), NetworkError> {
    let channels = vec![
        ChannelId::new(101, 't'), // Binance trades
        ChannelId::new(101, 's'), // Binance ticks
    ];
    
    redis_client.sub_filter(
        &channels,
        &[0x1234567890ABCDEF], // EURUSD only
        |channel_id, ticker_id, data| Box::pin(async move {
            match channel_id.msg_type() {
                't' => {
                    if let Ok(trade) = Trade::unpack(&data) {
                        println!("Trade: {} @ {}", trade.quantity, trade.price);
                    }
                },
                's' => {
                    if let Ok(tick) = Tick::unpack(&data) {
                        println!("Tick: bid={} ask={}", tick.bid_price, tick.ask_price);
                    }
                },
                _ => {}
            }
        })
    ).await
}
