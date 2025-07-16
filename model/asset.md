# MITCH Asset Classification System

## Overview

The MITCH protocol uses a standardized asset classification system to organize financial instruments into logical categories. Each asset is identified by a **4-bit asset class** combined with a **16-bit unique identifier** within that class, providing a total address space of 16 classes × 65,536 assets per class.

## Asset Class Hierarchy

### Structure (20 bits total, can be padded to 32)
```
┌─────────────┬─────────────────────┐
│ Asset Class │ Asset ID            │
│ (4 bits)    │ (16 bits)           │
└─────────────┴─────────────────────┘
```

### Memory Representation
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AssetClass {
    Equities = 0x0,
    CorporateBonds = 0x1,
    SovereignDebt = 0x2,
    Forex = 0x3,
    Commodities = 0x4,
    RealEstate = 0x5,
    CryptoAssets = 0x6,
    PrivateMarkets = 0x7,
    Collectibles = 0x8,
    Infrastructure = 0x9,
    Indices = 0xA,
    StructuredProducts = 0xB,
    CashEquivalents = 0xC,
    LoansReceivables = 0xD,
    // 0xE-0xF reserved
}
```

## Asset Class Specifications

### 0x0: Equities
**Description**: Selected publicly traded company shares (world-wide)
**Identifier Range**: 0-65,535 assets
**Examples**:
- `00531`: GOOGL (Alphabet Inc.)
- `00831`: AAPL (Apple Inc.)
- `09101`: MSFT (Microsoft Corporation)

cf. [../ids/stocks.csv]

**Sub-Categories**:
- Common stock
- Preferred shares
- ADRs (American Depositary Receipts)
- REITs (Real Estate Investment Trusts)

### 0x1: Corporate Bonds
**Description**: Selected debt securities issued by corporations (world-wide)
**Identifier Range**: 0-65,535 bonds
**Sub-Categories**:
- Investment grade bonds
- High-yield (junk) bonds
- Convertible bonds
- Floating rate notes

### 0x2: Sovereign Debt
**Description**: Government-issued debt securities
**Identifier Range**: 0-65,535 instruments
**Sub-Categories**:
- Treasury bills (< 1 year)
- Treasury notes (1-10 years)
- Treasury bonds (> 10 years)
- Municipal bonds
- Foreign government bonds

### 0x3: Forex
**Description**: Fiat currencies and foreign exchange instruments
**Identifier Range**: 0-65,535 currencies
**Examples**:
- `01301`: EUR (Euro)
- `05001`: USD (US Dollar)
- `00401`: GBP (British Pound)
- `02001`: JPY (Japanese Yen)
cf. [../ids/currencies.csv]

### 0x4: Commodities
**Description**: Physical goods, raw materials, and agricultural products
**Identifier Range**: 0-65,535 commodities
**Examples**:
- `00021`: Brent Crude Oil
- `00521`: WTI Crude Oil
- `00161`: Gold
- `00411`: Silver
- `00491`: Wheat
- `00111`: Corn
cf. [../ids/commodities.csv]

**Sub-Categories**:
- Energy: Oil, natural gas, gasoline
- Precious metals: Gold, silver, platinum
- Industrial metals: Copper, aluminum, steel
- Agricultural: Wheat, corn, soybeans, coffee

### 0x5: Real Estate
**Description**: Real estate securities and property investments
**Identifier Range**: 0-65,535 properties/securities
**Sub-Categories**:
- REITs (Real Estate Investment Trusts)
- Direct property ownership
- Real estate indices
- Property derivatives

### 0x6: Crypto Assets
**Description**: Cryptocurrencies, tokens, and digital assets
**Identifier Range**: 0-65,535 crypto assets
**Examples**:
- `02701`: BTC (Bitcoin)
- `05801`: ETH (Ethereum)
- `14701`: SOL (Solana)
- `17601`: USDT (Tether)
- `18501`: USDC (USD Coin)
cf. [../ids/crypto-assets.csv]

### 0x7: Private Markets
**Description**: Private equity, venture capital, and non-public investments
**Identifier Range**: 0-65,535 investments
**Sub-Categories**:
- Private equity funds
- Venture capital investments
- Private debt
- Direct investments in private companies

### 0x8: Collectibles
**Description**: Art, antiques, and collectible items with investment value
**Identifier Range**: 0-65,535 collectibles
**Sub-Categories**:
- Fine art and paintings
- Vintage automobiles
- Wine and spirits
- Sports memorabilia
- Rare books and manuscripts

### 0x9: Infrastructure
**Description**: Infrastructure investments and utilities
**Identifier Range**: 0-65,535 infrastructure assets
**Sub-Categories**:
- Transportation infrastructure
- Energy infrastructure
- Water and utilities
- Telecommunications infrastructure
- Social infrastructure

### 0xA: Indices
**Description**: Market indices and index-related products
**Identifier Range**: 0-65,535 indices
**Examples**:
- `00671`: S&P 500 Index
- `00461`: NASDAQ 100 Index
- `00281`: FTSE 100 Index
cf. [../ids/indices.csv]

**Sub-Categories**:
- Equity indices
- Bond indices
- Commodity indices
- Custom synthetic indices
- Factor-based indices

### 0xB: Structured Products
**Description**: Complex financial instruments with multiple components
**Identifier Range**: 0-65,535 products
**Sub-Categories**:
- Equity-linked notes
- Market-linked CDs
- Autocallable securities
- Principal-protected notes
- Reverse convertibles

### 0xC: Cash Equivalents
**Description**: Cash and highly liquid, short-term instruments
**Identifier Range**: 0-65,535 instruments
**Sub-Categories**:
- Cash deposits
- Money market funds
- Short-term Treasury bills
- Commercial paper
- Certificates of deposit

### 0xD: Loans & Receivables
**Description**: Loan instruments and receivable assets
**Identifier Range**: 0-65,535 instruments
**Sub-Categories**:
- Bank loans
- Consumer loans
- Mortgage loans
- Trade receivables
- Loan portfolios

## Encoding/Decoding Functions

### Pack Asset
```rust
pub fn pack_asset(asset_class: AssetClass, asset_id: u16) -> u32 {
    ((asset_class as u32) << 16) | (asset_id as u32)
}
```

### Unpack Asset
```rust
pub fn unpack_asset(packed_asset: u32) -> (AssetClass, u16) {
    let class_id = ((packed_asset >> 16) & 0xF) as u8;
    let asset_id = (packed_asset & 0xFFFF) as u16;
    
    let asset_class = match class_id {
        0x0 => AssetClass::Equities,
        0x1 => AssetClass::CorporateBonds,
        0x2 => AssetClass::SovereignDebt,
        0x3 => AssetClass::Forex,
        0x4 => AssetClass::Commodities,
        0x5 => AssetClass::RealEstate,
        0x6 => AssetClass::CryptoAssets,
        0x7 => AssetClass::PrivateMarkets,
        0x8 => AssetClass::Collectibles,
        0x9 => AssetClass::Infrastructure,
        0xA => AssetClass::Indices,
        0xB => AssetClass::StructuredProducts,
        0xC => AssetClass::CashEquivalents,
        0xD => AssetClass::LoansReceivables,
        _ => AssetClass::Equities, // Default fallback
    };
    
    (asset_class, asset_id)
}
```

## Validation Rules

### Asset Class Validation
```rust
pub fn is_valid_asset_class(class_id: u8) -> bool {
    class_id <= 0xD
}
```

### Asset ID Validation
```rust
pub fn is_valid_asset_id(asset_id: u16) -> bool {
    // All 16-bit values are valid (0-65535)
    true
}
```

### Combined Validation
```rust
pub fn validate_asset(asset_class: AssetClass, asset_id: u16) -> Result<(), &'static str> {
    // Perform class-specific validation
    match asset_class {
        AssetClass::Forex => {
            if asset_id == 0 {
                return Err("Currency ID 0 is reserved");
            }
        }
        AssetClass::CashEquivalents => {
            if asset_id == 0 {
                return Err("Cash equivalent ID 0 is reserved");
            }
        }
        _ => {} // Other classes allow any ID
    }
    
    Ok(())
}
```

## Usage Examples

### Creating Trading Pairs
```rust
// EUR/USD spot forex
let eur = pack_asset(AssetClass::Forex, 111);       // EUR
let usd = pack_asset(AssetClass::Forex, 461);       // USD

// Apple stock in USD
let aapl = pack_asset(AssetClass::Equities, 831);   // AAPL
let usd = pack_asset(AssetClass::Forex, 461);       // USD

// Bitcoin in USDT
let btc = pack_asset(AssetClass::CryptoAssets, 1);   // BTC
let usdt = pack_asset(AssetClass::CryptoAssets, 3);  // USDT
```

### Asset Class Statistics
```rust
pub fn get_asset_class_stats() -> HashMap<AssetClass, (u32, u32)> {
    // Returns (total_allocated, max_capacity) for each class
    [
        (AssetClass::Equities, (5000, 65536)),
        (AssetClass::Forex, (200, 65536)),
        (AssetClass::CryptoAssets, (1500, 65536)),
        // ... other classes
    ].into_iter().collect()
}
```

## Integration with Reference Data

Asset IDs can be coordinated and matched with external reference data sources:

### Generic Identifiers

The below standardized IDs are applicable to instruments in all asset-classes:
- [FIGI Codes](https://www.openfigi.com/)
- [ISIN Codes](https://www.isin.org/)
- [CUSIP Codes](https://www.cusip.com/identifiers.html)

### Specific Identifiers

- **Forex**: ISO 4217 currency codes (used as aliases in [../ids/forex.csv])
- **Crypto Assets**: On-chain addresses, ENS, SNS, CMC/CG identifiers
- **Commodities**: CBOE, LME, NYMEX, ICE etc. symbols

Resolving and converting these standard identifiers to MITCH IDs is required to comply with the protocol specifications when communicating with third party MITCH-enabled services.
Unlike the above, MITCH's ticker ID space is designed to be anti-collision and transport optimized, but not human friendly.
