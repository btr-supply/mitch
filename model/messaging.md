# MITCH Messaging Architecture

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
| `t`  | Trade Messages     | `Trade[]`          | 32B each  | Trade executions (single or batch)    |
| `o`  | Order Messages     | `Order[]`          | 32B each  | Order events (single or batch)        |
| `s`  | Tick Messages      | `Tick[]`           | 32B each  | Tick snapshots (single or batch)      |
| `b`  | Order Book         | `OrderBook[]`      | 2072B each| Order book snapshots                  |
| `i`  | Index Messages     | `Index[]`          | 64B each  | Synthetic aggregated data             |

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

## Channel ID Integration

Channel IDs work alongside messaging for pub/sub filtering. See [README.md](README.md#52-channel-id-encoding) for complete channel ID specification.

```rust
// Generate channel ID for message routing
let channel_id = Channel::generate(market_provider_id, header.message_type);
```
