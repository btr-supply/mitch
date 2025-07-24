# MITCH Index Message Specification

*Part of the [MITCH Protocol](./overview.md) | Message Type: `'i'` | See [Messaging Architecture](./messaging.md)*

## Overview

Index messages (`i`) provide synthetic aggregated market data reflecting the state of a financial instrument across all markets. These enriched messages combine data from multiple sources with additional metrics for volatility, liquidity, trend analysis, and data quality assessment.

## Message Structure (64 bytes)

| Field       | Offset | Size | Type  | Description                              |
|-------------|--------|------|-------|------------------------------------------|
| Ticker ID   | 0      | 8    | `u64` | See [ticker.md](ticker.md) for encoding details |
| Mid         | 8      | 8    | `f64` | Mid price (synthetic)                    |
| VBid        | 16     | 4    | `u32` | Bid volume (aggregated sell volume)      |
| VAsk        | 20     | 4    | `u32` | Ask volume (aggregated buy volume)       |
| MSpread     | 24     | 4    | `i32` | Mean spread (1e-9 pbp)                   |
| BBidO       | 28     | 4    | `i32` | Best bid offset (1e-9 pbp)               |
| BAskO       | 32     | 4    | `i32` | Best ask offset (1e-9 pbp)               |
| WBidO       | 36     | 4    | `i32` | Worst bid offset (1e-9 pbp)              |
| WAskO       | 40     | 4    | `i32` | Worst ask offset (1e-9 pbp)              |
| VForce      | 44     | 2    | `u16` | Volatility force (0-10000)               |
| LForce      | 46     | 2    | `u16` | Liquidity force (0-10000)                |
| TForce      | 48     | 2    | `i16` | Trend force (-10000-10000)               |
| MForce      | 50     | 2    | `i16` | Momentum force (-10000-10000)            |
| Confidence  | 52     | 1    | `u8`  | Data quality (0-100, 100=best)           |
| Rejected    | 53     | 1    | `u8`  | Number of sources rejected               |
| Accepted    | 54     | 1    | `u8`  | Number of sources accepted               |
| Padding     | 55     | 9    | `u8[9]` | Padding to 64 bytes                    |

## Field Specifications

### Ticker ID (8 bytes)
**Reference**: [ticker.md](ticker.md) - Complete 8-byte ticker encoding specification
**Note**: For index messages, typically uses `InstrumentType::Indices` (0xA)

### Mid Price (8 bytes)
- **Type**: `f64` - Synthetic mid price aggregated across multiple markets
- **Calculation**: Volume-weighted average or other aggregation method

### VBid/VAsk - Aggregated Volumes (8 bytes total)
- **Type**: `u32` each - Total bid/ask volumes aggregated across all sources
- **Units**: Normalized to common base units across exchanges

### Price Offsets (20 bytes total)
- **Type**: `i32` each - Price basis points in 1e-9 precision
- **Range**: ±2.1 billion nano-basis-points (±21% price movement vs mid)
- **MSpread**: Average spread across all sources
- **BBidO/BAskO**: Best bid/ask deviation from synthetic mid
- **WBidO/WAskO**: Worst bid/ask deviation from synthetic mid

### Force Metrics (8 bytes total)
- **VForce**: `u16` (0-10000) - Volatility index, 0=constant, 10000=extreme, relative standard deviation
- **LForce**: `u16` (0-10000) - Liquidity index, 0=illiquid, 10000=infinite, based on volumes, spread and market depth
- **TForce**: `i16` (-10000-10000) - Trend direction, negative=down, positive=up, short term trend relative to long term volatility
- **MForce**: `i16` (-10000-10000) - Momentum strength, trend acceleration, short term trend acceleration relative to long term volatility

### Data Quality Fields (3 bytes total)
- **Confidence**: `u8` (0-100) - Overall data quality indicator
- **Rejected/Accepted**: `u8` each - Source counts for transparency

## Confidence Levels

| **Confidence**       | **Max Latency (ms)** | **Max Rejections (%)** | **Max Divergence (%)** |
| ---------------------| -------------------- | ---------------------- | ---------------------- |
| 100–95 (Very High)   | 0–10                 | 2%                     | 0.10%                  |
| 95–80 (High)         | 10–30                | 5%                     | 0.25%                  |
| 80–60 (Medium)       | 30–50                | 8%                     | 0.50%                  |
| 60–40 (Medium-Low)   | 50–70                | 13%                    | 1.00%                  |
| 40–20 (Low)          | 70–100               | 20%                    | 2.00%                  |
| 20–0 (Very Low)      | 100+                 | 20%+                   | 2.00%+                 |

## Utility Functions

```rust
// Price calculations from offsets
pub fn best_bid_price(&self) -> f64 {
    self.mid * (1.0 + (self.bbido as f64) / 1e9)
}

pub fn best_ask_price(&self) -> f64 {
    self.mid * (1.0 + (self.basko as f64) / 1e9)
}

// Force analysis
pub fn volatility_percentage(&self) -> f64 { self.vforce as f64 / 10000.0 }
pub fn liquidity_percentage(&self) -> f64 { self.lforce as f64 / 10000.0 }
pub fn trend_percentage(&self) -> f64 { self.tforce as f64 / 10000.0 }
pub fn momentum_percentage(&self) -> f64 { self.mforce as f64 / 10000.0 }
```

## Validation Rules

```rust
pub fn validate(&self) -> Result<(), &'static str> {
    if self.ticker_id == 0 { return Err("Ticker ID cannot be zero"); }
    if self.mid <= 0.0 { return Err("Mid price must be positive"); }
    if self.confidence > 100 { return Err("Confidence must be 0-100"); }
    if self.vforce > 10000 || self.lforce > 10000 { return Err("Forces out of range"); }
    if self.tforce < -10000 || self.tforce > 10000 { return Err("Trend force out of range"); }
    if self.mforce < -10000 || self.mforce > 10000 { return Err("Momentum force out of range"); }
    if self.accepted == 0 && self.confidence > 0 { return Err("Cannot have confidence without sources"); }
    Ok(())
}
```
