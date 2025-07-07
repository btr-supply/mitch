# BTR IDS Data Files

This directory contains the core data files used for identifying and categorizing financial instruments within the BTR ecosystem.

## Standardized CSV Format

All asset data files follow a consistent three-column structure:

**Standard Format:**
- `btr_id` - Unique numeric identifier (BTR ID)
- `name` - Full product/instrument name or description
- `aliases` - Pipe-separated list of trading symbols and alternative names

**Metadata Files Exception:**
- `asset-classes.csv` - Only has `btr_id` and `name` (no aliases needed)
- `instrument-types.csv` - Only has `btr_id` and `name` (no aliases needed)
- `market-providers.csv` - Only has `btr_id` and `name` (no aliases needed)

## File Structure
- `asset-classes.csv`: Master list of asset classes and their BTR IDs
- `instrument-types.csv`: Master list of instrument types and their BTR IDs
- `currencies.csv`: Fiat currencies with BTR IDs and ISO codes/aliases
- `commodities.csv`: Commodities with BTR IDs and trading symbol aliases
- `indices.csv`: Stock indices with BTR IDs and trading symbol aliases
- `tokens.csv`: Cryptocurrency tokens with BTR IDs and trading symbol aliases
- `stocks.csv`: Individual stocks with BTR IDs and trading symbol aliases
- `market-providers.csv`: Exchanges, brokers, and market data providers

## Symbol Resolution Conventions

The BTR system standardizes instrument symbols by programmatically handling common platform-specific prefixes and suffixes rather than storing them as explicit aliases.

### Automatic Prefix/Suffix Stripping Rules

**Common Prefixes:** `^`, `.`, `$` (e.g., `^SPX`, `.DJI`, `$INDU`)

**Suffix Categories:**
1. **Delimiter-Based Suffixes** (stripped when following `-`, `_`, `.`, `$`, `^`):
   - `US`, `USD`, `USX`, `C`, `M`, `MINI`, `MICRO`, `CASH`, `SPOT` (case-insensitive)
   - Examples: `SPX.CASH`, `DJI_MINI`, `GOLD$USD`, `NDQ-MICRO`

2. **Standalone Suffixes** (stripped regardless of delimiters):
   - `MINI`, `MICRO`, `CASH`, `SPOT` (case-insensitive)
   - Examples: `SPXMINI`, `DJICASH`, `GOLDSPOT`

3. **Single Character Suffixes:** `M`, `C`, `Z`, `B`, `R`, `D`, `I`, `_`, `$`

**Compound Suffix Handling:** The resolution logic runs **twice** to handle compound suffixes like `XAG.CASH` or `NDQ$MICRO`, where the first pass removes the descriptive suffix and the second pass removes the delimiter.

## Data Consistency Rules

1. **Primary Symbol**: The most commonly used trading symbol should be the first in the aliases list
2. **ISO Codes**: Currency ISO codes (USD, EUR, etc.) are stored as aliases
3. **Platform Independence**: Common prefixes/suffixes are handled programmatically, not stored as aliases
4. **Pipe Separation**: Multiple aliases are separated by `|` character
5. **Case Sensitivity**: All symbol matching is case-insensitive in the system

## Default Conventions

**Currency Denominations:** All commodities and indices are assumed USD-denominated unless explicitly specified otherwise.

**Automatic Handling:** The MQL4 framework (`BTRIds.mqh`) automatically applies symbol cleanup rules during resolution, ensuring platform-specific symbols resolve to standardized instruments.

## Commodity Symbol Conventions

A critical aspect of the BTR system is the standardization of instrument symbols, especially for commodities, which have a wide variety of tickers across different platforms.

**Default Currency:** All commodities are assumed USD-denominated unless explicitly specified otherwise.

**Automatic Handling:** The MQL4 framework (`BTRIds.mqh`) automatically applies these rules during symbol cleanup, ensuring platform-specific symbols resolve to standardized instruments.

## Index Symbol Conventions

### Prefix and Suffix Handling

**Common Prefixes/Suffixes:** Stock indices often have platform-specific prefixes or suffixes appended by data providers:
- **Prefixes:** `^`, `.`, `$` (e.g., `^SPX`, `.DJI`, `$INDU`)
- **Suffixes:** `_`, `.`, `$` (e.g., `SPX_`, `DJI.`, `INDU$`)
- **Descriptive Suffixes:** `CASH`, `SPOT`, `ECN`, `M`, `m`, `micro` (e.g., `SPX.cash`, `NDQ$micro`)

To maintain clean alias lists, these **should NOT be included** in the `alias` column of `indices-ids.csv`.

**Compound Suffix Handling:** The symbol resolution logic runs **twice** to handle compound suffixes like `XAG.cash` or `NDQ$micro`, where the first pass removes `cash`/`micro` and the second pass removes `.`/`$`.

The MQL4 framework (`BTRIds.mqh`) automatically strips these prefixes and suffixes during symbol cleanup, ensuring that platform-specific symbols like `^SPX` or `DJI.cash` correctly resolve to standardized instruments.

**Platform Variations:** Stock indices use diverse prefixes and suffixes across data providers, all handled automatically by the resolution system.
