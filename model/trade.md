# MITCH Trade Message Specification

## Overview

Trade messages (`t`) represent executed transactions in a market, capturing price, volume, participant, and timing information.

## Message Structure (32 bytes)

| Field     | Offset | Size | Type        | Description                            |
|-----------|--------|------|-------------|----------------------------------------|
| Ticker ID | 0      | 8    | `u64`       | See [ticker.md](ticker.md) for encoding details |
| Price     | 8      | 8    | `f64`       | Execution price                        |
| Quantity  | 16     | 4    | `u32`       | Executed volume/quantity               |
| Trade ID  | 20     | 4    | `u32`       | **Required** unique trade identifier   |
| Side      | 24     | 1    | `OrderSide` | `0`: Buy, `1`: Sell                    |
| Padding   | 25     | 7    | `u8[7]`     | Padding to 32 bytes                    |

## Field Specifications

### Ticker ID (8 bytes)
**Reference**: [ticker.md](ticker.md) - Complete 8-byte ticker encoding specification

### Price (8 bytes)
- **Type**: `f64` - Full double precision range, instrument-dependent precision
- **Purpose**: Execution price of the trade

### Quantity (4 bytes)
- **Type**: `u32` - Range 0 to 4,294,967,295
- **Units**: Instrument-dependent (shares, lots, contracts, tokens)

### Trade ID (4 bytes)
- **Type**: `u32` - Range 1 to 4,294,967,295 (0 reserved)
- **Purpose**: Unique identifier for the trade within the system

### Side (1 byte)
- **Type**: `OrderSide` enum - `0`: Buy (aggressor buying), `1`: Sell (aggressor selling)
- **Purpose**: Indicates which side initiated the trade

## Validation Rules

```rust
pub fn validate(&self) -> Result<(), &'static str> {
    if self.trade_id == 0 { return Err("Trade ID cannot be zero"); }
    if self.price <= 0.0 { return Err("Price must be positive"); }
    if self.quantity == 0 { return Err("Quantity must be positive"); }
    if self.ticker_id == 0 { return Err("Ticker ID cannot be zero"); }
    Ok(())
}
```

