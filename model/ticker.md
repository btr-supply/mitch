# MITCH Ticker ID System

## Overview

The **Ticker ID** is the cornerstone of the MITCH protocol - a unique 8-byte identifier that encodes any tradable financial instrument. It distinguishes between different trading pairs by encoding both the **base instrument** being traded and the **quote asset** used for pricing.

## 64-Bit Encoding Format

The Ticker ID uses a carefully designed bit allocation to maximize addressable space while enabling efficient filtering and routing.

```
┌───────────────┬───────────────────────┬───────────────────────┬───────────────────────┐
│ Inst. Type    │ Base Asset (class+id) │ Quote Asset (class+id)│ Sub-Type              │
│ (4 bits)      │ 20 bits    (4+16 bits)│ 20 bits    (4+16 bits)│ (20 bits)             │
└───────────────┴───────────────────────┴───────────────────────┴───────────────────────┘
```

### Field Breakdown

| Field | Bits | Size | Purpose | Range |
|-------|------|------|---------|--------|
| Instrument Type | 60-63 | 4 bits | Financial product category | 0-15 types |
| Base Asset | 40-59 | 20 bits | Core traded asset | 16 classes × 65,536 IDs |
| Quote Asset | 20-39 | 20 bits | Pricing/settlement asset | 16 classes × 65,536 IDs |
| Sub-Type | 0-19 | 20 bits | Additional specification | 1M variations |

## Instrument Types (4 bits)

| ID | Type | Description | Use Cases |
|----|------|-------------|-----------|
| `0x0` | Spot | Direct asset trading | FX spot, stock shares, crypto spot |
| `0x1` | Future | Standardized futures contract | WTI oil, E-mini S&P 500 |
| `0x2` | Forward | Custom forward contract | FX forwards, commodity forwards |
| `0x3` | Swap | Interest rate or currency swap | IRS, CCS |
| `0x4` | Perpetual Swap | Crypto perpetual futures | BTC-PERP, ETH-PERP |
| `0x5` | CFD | Contract for difference | Stock CFDs, commodity CFDs |
| `0x6` | Call Option | Call option contract | Stock options, FX options |
| `0x7` | Put Option | Put option contract | Protective puts, hedging |
| `0x8` | Digital Option | Binary/digital option | Touch/no-touch, range binaries |
| `0x9` | Barrier Option | Barrier option contract | Knock-in/knock-out options |
| `0xA` | Warrant | Warrant contract | Stock warrants, covered warrants |
| `0xB` | Prediction Contract | Contract based on predicted outcomes | Sports betting, election markets |
| `0xC` | Structured Product | Multi-component financial instruments | Autocallables, reverse convertibles |
| `0xD-0xF` | *Reserved* | *Reserved for future use* | *Future instrument types* |

## Asset Components (20 bits each)

Each asset (base and quote) combines a 4-bit class identifier with a 16-bit unique ID within that class:
┌─────────────┬─────────────────────┐
│ Asset Class │ Asset ID            │
│ (4 bits)    │ (16 bits)           │
└─────────────┴─────────────────────┘

cf. [../asset.md]

## Memory Layout

```rust
#[repr(C, packed)]
pub struct Ticker {
    pub instrument_type: u8,    // Extracted from bits 60-63
    pub base_class: u8,         // Extracted from bits 56-59  
    pub base_id: u16,           // Extracted from bits 40-55
    pub quote_class: u8,        // Extracted from bits 36-39
    pub quote_id: u16,          // Extracted from bits 20-35
    pub sub_type: u32,          // Extracted from bits 0-19
}
```

## Encoding/Decoding Functions

### Pack Ticker ID
```rust
impl TickerId {
    pub fn generate(
        instrument_type: InstrumentType,
        base_class: AssetClass,
        base_id: u16,
        quote_class: AssetClass,
        quote_id: u16,
        sub_type: u32,
    ) -> Result<u64, &'static str> {
        if sub_type > 0xFFFFF {
            return Err("Sub-type must fit in 20 bits");
        }

        let base_asset = ((base_class as u32) << 16) | (base_id as u32);
        let quote_asset = ((quote_class as u32) << 16) | (quote_id as u32);

        let ticker_id = ((instrument_type as u64) << 60) |
                       ((base_asset as u64) << 40) |
                       ((quote_asset as u64) << 20) |
                       (sub_type as u64);

        Ok(ticker_id)
    }
}
```

### Unpack Ticker ID  
```rust
impl TickerId {
    pub fn extract(ticker_id: u64) -> Ticker {
        let instrument_type = ((ticker_id >> 60) & 0xF) as u8;
        let base_asset = ((ticker_id >> 40) & 0xFFFFF) as u32;
        let quote_asset = ((ticker_id >> 20) & 0xFFFFF) as u32;
        let sub_type = (ticker_id & 0xFFFFF) as u32;

        let base_class = ((base_asset >> 16) & 0xF) as u8;
        let base_id = (base_asset & 0xFFFF) as u16;
        let quote_class = ((quote_asset >> 16) & 0xF) as u8;
        let quote_id = (quote_asset & 0xFFFF) as u16;

        Ticker {
            instrument_type,
            base_class,
            base_id,
            quote_class,
            quote_id,
            sub_type,
        }
    }
}
```

### Instant Serialization
```rust
/// Pack ticker ID into bytes using unsafe casting (instant, no-copy)
pub fn pack_ticker_id(ticker_id: u64) -> [u8; 8] {
    unsafe { std::mem::transmute(ticker_id) }
}

/// Unpack ticker ID from bytes using unsafe casting (instant, no-copy)
pub fn unpack_ticker_id(bytes: &[u8]) -> Result<u64, MitchError> {
    if bytes.len() < 8 {
        return Err(MitchError::InvalidData("Not enough bytes for ticker ID"));
    }
    
    let mut array = [0u8; 8];
    array.copy_from_slice(&bytes[0..8]);
    Ok(unsafe { std::mem::transmute(array) })
}
```

## Real-World Examples

### EUR/USD Spot Forex
```
Instrument Type: 0x0 (Spot)
Base Asset:      0x3 (Forex) + 111 (EUR) = 0x3006F
Quote Asset:     0x3 (Forex) + 461 (USD) = 0x301CD  
Sub-Type:        0x00000 (No additional specification)

Result: 0x03006F301CD00000 (216295034546290688 decimal)
```

### EUR/USD Index (Synthetic)
```
Instrument Type: 0xA (Indices & Index Products)
Base Asset:      0x3 (Forex) + 111 (EUR) = 0x3006F
Quote Asset:     0x3 (Forex) + 461 (USD) = 0x301CD
Sub-Type:        0x00000

Result: 0xA3006F301CD00000 (11745510080614760448 decimal)
```

### AAPL $190 Call Option (30 days)
```
Instrument Type: 0x6 (Call Option)
Base Asset:      0x0 (Equity) + 831 (AAPL) = 0x0033F
Quote Asset:     0x3 (Forex) + 461 (USD) = 0x301CD
Sub-Type:        0x1CAFE (Encoded strike + expiry)

Result: 0x60033F301CD1CAFE (6918442928445704958 decimal)
```

### BTC/USDT Perpetual Swap
```
Instrument Type: 0x4 (Perpetual Swap)
Base Asset:      0x6 (Crypto) + 1 (BTC) = 0x60001
Quote Asset:     0x6 (Crypto) + 17601 (USDT) = 0x644C1
Sub-Type:        0x00000

Result: 0x460001644C100000 (5043847208648490496 decimal)
```

## Sub-Type Encoding Examples

The instrument sub-type, aka. ticker suffix, should fit in 20bits.
The recommended implementation is using a rolling list with fixed ID space (2^20−1=1,048,575 values) mapping to relevant unique traits.

### Options Traits
For options, sub-type should encode strike price and expiry.

### Futures Traits
For futures, sub-type encodes the contract delivery date.

## Performance Characteristics

### Memory Efficiency
- **Storage**: 8 bytes per ticker ID
- **Index Size**: Supports 2^64 unique instruments
- **Collision Risk**: Effectively zero with proper ID sub-type management

## Validation Rules

### Required Checks
```rust
pub fn validate_ticker_id(ticker_id: u64) -> bool {
    let components = TickerId::extract(ticker_id);
    
    // Check instrument type is valid
    if components.instrument_type > 0xC { return false; }
    
    // Check asset classes are valid  
    if components.base_class > 0xD { return false; }
    if components.quote_class > 0xD { return false; }
    
    // Check sub-type fits in 20 bits
    if components.sub_type > 0xFFFFF { return false; }
    
    true
}
```

### Common Validation Errors
- **Invalid instrument type**: Type > 0xC
- **Invalid asset class**: Class > 0xD  
- **Sub-type overflow**: sub_type > 0xFFFFF
- **Self-referencing pair**: base_asset == quote_asset (usually invalid)

## Integration with Channel IDs

Ticker IDs work seamlessly with Channel IDs for complete message routing:

```rust
// Create channel ID for EURUSD ticks from Interactive Brokers
let ticker_id = 0x03006F301CD00000u64;  // EUR/USD spot
let channel_id = Channel::generate(00691, b's');  // IBKR + ticks

// Subscriber can filter by both
subscribe_to_ticker_on_channel(ticker_id, channel_id);
```

This ticker ID system provides a compact, efficient way to uniquely identify any financial instrument in existence while enabling fast lookups, filtering, and routing in high-frequency and/or low-latency environments.
