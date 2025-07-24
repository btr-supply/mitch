//! MITCH Ticker and Asset Resolution Module
//!
//! This module provides comprehensive ticker identification, asset resolution, and parsing
//! utilities for the MITCH protocol. It handles:
//! - 64-bit ticker ID encoding/decoding with asset classification
//! - Asset resolution with fuzzy string matching
//! - Ticker symbol parsing with automatic suffix stripping and quote detection
//! - Standardized lowercase processing for optimal performance

use crate::common::{MitchError, AssetClass, InstrumentType};
use crate::constants::{
    COMMODITIES_DATA, CRYPTO_ASSETS_DATA, EQUITIES_DATA,
    FOREX_DATA, INDICES_DATA, SOVEREIGN_DEBT_DATA, DataEntry
};
use crate::utils::{normalize_asset_name, jaro_winkler_similarity};
use std::collections::HashMap;
use std::sync::LazyLock;
use core::fmt;

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Pack asset class and ID into 32-bit global identifier (as per asset.md)
pub fn pack_asset(asset_class: AssetClass, class_id: u16) -> u32 {
    ((asset_class as u32) << 16) | (class_id as u32)
}

/// Unpack global asset ID into class and class_id (as per asset.md)
pub fn unpack_asset(packed_asset: u32) -> (AssetClass, u16) {
    let class_id = ((packed_asset >> 16) & 0xF) as u8;
    let asset_id = (packed_asset & 0xFFFF) as u16;

    let asset_class = AssetClass::from_id(class_id);
    (asset_class, asset_id)
}

/// Asset with pre-normalized search keys
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    /// Global asset ID (32-bit: 4-bit class + 16-bit class_id, as per asset.md)
    pub id: u32,
    /// Asset ID within its class (16-bit)
    pub class_id: u16,
    /// Asset class (4-bit encoded)
    pub class: AssetClass,
    /// Human-readable name
    pub name: String,
    /// Pipe-separated aliases for resolution
    pub aliases: String,
}

/// Complete ticker representation for internal use
#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    /// 64-bit ticker ID for MITCH protocol messages
    pub id: u64,
    /// Human-readable name/symbol
    pub name: String,
    /// Instrument type (spot, futures, etc.)
    pub instrument_type: InstrumentType,
    /// Base asset
    pub base: Asset,
    /// Quote asset
    pub quote: Asset,
    /// Sub-type specification (20-bit)
    pub sub_type: u32,
}

/// Asset search result with confidence score
#[derive(Debug, Clone, PartialEq)]
pub struct AssetMatch {
    pub asset: Asset,
    pub confidence: f64,
    pub matched_field: String,
}

/// Ticker resolution result
#[derive(Debug, Clone, PartialEq)]
pub struct TickerMatch {
    pub ticker: Ticker,
    pub confidence: f64,
    pub processing_steps: Vec<String>,
}

// =============================================================================
// TICKER ID ENCODING/DECODING (64-bit)
// =============================================================================

/// Ticker ID structure for encoding/decoding operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickerId {
    /// Complete 64-bit ticker identifier
    pub raw: u64,
}

impl TickerId {
    /// Create a new ticker ID from components
    ///
    /// # Ticker ID Layout (64 bits / 8 bytes) - SPEC COMPLIANT
    ///
    /// ```text
    /// Bits   | Field              | Size | Description
    /// -------|--------------------|----- |---------------------------
    /// 63-60  | Instrument Type    | 4    | Financial product category (0-15)
    /// 59-56  | Base Asset Class   | 4    | Base asset category (0-15)
    /// 55-40  | Base Asset ID      | 16   | Base asset identifier (0-65535)
    /// 39-36  | Quote Asset Class  | 4    | Quote asset category (0-15)
    /// 35-20  | Quote Asset ID     | 16   | Quote asset identifier (0-65535)
    /// 19-0   | Sub-Type           | 20   | Additional specification (0-1048575)
    /// ```
    pub fn new(
        instrument_type: InstrumentType,
        base_class: AssetClass,
        base_id: u16,
        quote_class: AssetClass,
        quote_id: u16,
        sub_type: u32,
    ) -> Result<Self, MitchError> {
        if sub_type > 0xFFFFF {
            return Err(MitchError::InvalidData("Sub-type must fit in 20 bits".to_string()));
        }

        let raw = ((instrument_type as u64) << 60)
            | ((base_class as u64) << 56)
            | ((base_id as u64) << 40)
            | ((quote_class as u64) << 36)
            | ((quote_id as u64) << 20)
            | (sub_type as u64);

        Ok(Self { raw })
    }

    /// Create ticker ID from raw 64-bit value
    pub fn from_raw(raw: u64) -> Self {
        Self { raw }
    }

    /// Extract instrument type from ticker ID
    pub fn instrument_type(&self) -> InstrumentType {
        let value = ((self.raw >> 60) & 0x0F) as u8;
        InstrumentType::from_id(value)
    }

    /// Extract base asset class from ticker ID
    pub fn base_asset_class(&self) -> AssetClass {
        let value = ((self.raw >> 56) & 0x0F) as u8;
        AssetClass::from_id(value)
    }

    /// Extract base asset ID from ticker ID
    pub fn base_asset_id(&self) -> u16 {
        ((self.raw >> 40) & 0xFFFF) as u16
    }

    /// Extract quote asset class from ticker ID
    pub fn quote_asset_class(&self) -> AssetClass {
        let value = ((self.raw >> 36) & 0x0F) as u8;
        AssetClass::from_id(value)
    }

    /// Extract quote asset ID from ticker ID
    pub fn quote_asset_id(&self) -> u16 {
        ((self.raw >> 20) & 0xFFFF) as u16
    }

    /// Extract sub-type from ticker ID
    pub fn sub_type(&self) -> u32 {
        (self.raw & 0xFFFFF) as u32
    }

    /// Pack ticker ID to bytes using zero-copy operations
    pub fn pack(&self) -> [u8; 8] {
        self.raw.to_le_bytes()
    }

    /// Unpack ticker ID from bytes using zero-copy operations
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 8 {
            return Err(MitchError::BufferTooSmall {
                expected: 8,
                actual: bytes.len(),
            });
        }

        unsafe {
            let ptr = bytes.as_ptr() as *const u64;
            let raw = ptr.read_unaligned().to_le();
            Ok(Self::from_raw(raw))
        }
    }

    /// Check if this ticker represents a forex pair
    pub fn is_forex(&self) -> bool {
        matches!(self.base_asset_class(), AssetClass::FX) ||
        matches!(self.quote_asset_class(), AssetClass::FX)
    }

    /// Check if this ticker represents a crypto pair
    pub fn is_crypto(&self) -> bool {
        matches!(self.base_asset_class(), AssetClass::CR) ||
        matches!(self.quote_asset_class(), AssetClass::CR)
    }

    /// Check if this is a spot instrument
    pub fn is_spot(&self) -> bool {
        matches!(self.instrument_type(), InstrumentType::SPOT)
    }
}

impl From<u64> for TickerId {
    fn from(raw: u64) -> Self {
        Self::from_raw(raw)
    }
}

impl From<TickerId> for u64 {
    fn from(ticker: TickerId) -> Self {
        ticker.raw
    }
}

impl fmt::Display for TickerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TickerId({:016X}: {:?} base={:?}:{} quote={:?}:{} sub={})",
            self.raw,
            self.instrument_type(),
            self.base_asset_class(),
            self.base_asset_id(),
            self.quote_asset_class(),
            self.quote_asset_id(),
            self.sub_type()
        )
    }
}

// =============================================================================
// SUFFIX STRIPPING LOGIC
// =============================================================================

/// Strip common prefixes and suffixes from ticker symbols as per README.md specifications
fn strip_ticker_suffixes(symbol: &str) -> String {
    let mut cleaned = symbol.to_lowercase();

    // Strip common prefixes
    for prefix in &["^", ".", "$", "#", "_"] {
        if cleaned.starts_with(prefix) {
            cleaned = cleaned[1..].to_string();
            break;
        }
    }

    // Run suffix stripping twice to handle compound suffixes
    for _pass in 0..2 {
        let mut changed = false;

        // Delimiter-based suffixes (stripped when following delimiters)
        for delimiter in &["-", "_", ".", "$", "^", "#"] {
            if let Some(pos) = cleaned.rfind(delimiter) {
                let suffix = &cleaned[pos + 1..];
                if matches!(suffix, "us" | "m" | "c" | "z" | "b" | "r" | "d" | "i") {
                    cleaned = cleaned[..pos].to_string();
                    changed = true;
                    break;
                }
            }
        }

        // Standalone suffixes (stripped regardless of delimiters)
        if !changed {
            for suffix in &["usx", "mini", "micro", "cash", "spot", "ecn", "zero"] {
                if cleaned.ends_with(suffix) && cleaned.len() > suffix.len() {
                    cleaned = cleaned[..cleaned.len() - suffix.len()].to_string();
                    changed = true;
                    break;
                }
            }
        }

        // Strip trailing delimiters in any pass
        if !changed {
            for delimiter in &["-", "_", ".", "$", "^", "#"] {
                if cleaned.ends_with(delimiter) && cleaned.len() > 1 {
                    cleaned = cleaned[..cleaned.len() - 1].to_string();
                    changed = true;
                    break;
                }
            }
        }

        // If no changes were made, we can exit early
        if !changed {
            break;
        }
    }

    cleaned
}

// =============================================================================
// ASSET RESOLVER
// =============================================================================

/// High-performance asset resolver
#[derive(Debug)]
struct AssetResolver {
    by_id: HashMap<(AssetClass, u16), Asset>,
    by_normalized: HashMap<String, Asset>,
    by_class: HashMap<AssetClass, Vec<Asset>>,
    all: Vec<Asset>,
}

impl AssetResolver {
    fn new() -> Self {
        let mut resolver = Self {
            by_id: HashMap::new(),
            by_normalized: HashMap::new(),
            by_class: HashMap::new(),
            all: Vec::new(),
        };
        resolver.load_data();
        resolver
    }

    fn load_data(&mut self) {
        self.load_class(AssetClass::CM, COMMODITIES_DATA);
        self.load_class(AssetClass::CR, CRYPTO_ASSETS_DATA);
        self.load_class(AssetClass::EQ, EQUITIES_DATA);
        self.load_class(AssetClass::FX, FOREX_DATA);
        self.load_class(AssetClass::IN, INDICES_DATA);
        self.load_class(AssetClass::SD, SOVEREIGN_DEBT_DATA);
    }

    fn load_class(&mut self, class: AssetClass, data: &[DataEntry]) {
        let mut class_assets = Vec::new();

        for entry in data {
            let class_id = entry.id as u16;
            let asset = Asset {
                id: pack_asset(class, class_id),
                class_id,
                class,
                name: entry.name.to_string(),
                aliases: entry.aliases.to_string(),
            };

            // Index by ID and class
            self.by_id.insert((class, class_id), asset.clone());

            // Index by normalized name (lowercase)
            let normalized_name = normalize_asset_name(entry.name);
            if !normalized_name.is_empty() {
                self.by_normalized.insert(normalized_name, asset.clone());
            }

            // Index by normalized aliases (lowercase)
            for alias in entry.aliases.split('|').filter(|s| !s.is_empty()) {
                let normalized_alias = normalize_asset_name(alias);
                if !normalized_alias.is_empty() {
                    self.by_normalized.insert(normalized_alias, asset.clone());
                }
            }

            class_assets.push(asset.clone());
            self.all.push(asset);
        }

        self.by_class.insert(class, class_assets);
    }

    fn find(&self, query: &str, min_confidence: f64, class_filter: Option<AssetClass>) -> Option<AssetMatch> {
        if query.trim().is_empty() { return None; }

        // Apply suffix stripping first
        let cleaned_query = strip_ticker_suffixes(query);
        let norm_query = normalize_asset_name(&cleaned_query);

        // Try exact match first
        if let Some(asset) = self.by_normalized.get(&norm_query) {
            if class_filter.map_or(true, |c| c == asset.class) {
                return Some(AssetMatch {
                    asset: asset.clone(),
                    confidence: 1.0,
                    matched_field: "exact".to_string(),
                });
            }
        }

        // Find candidates based on class filter
        let candidates: Vec<&Asset> = match class_filter {
            Some(class) => self.by_class.get(&class)?.iter().collect(),
            None => self.all.iter().collect(),
        };

        // Check for exact alias match first for a significant confidence boost
        for asset in &candidates {
            if asset.aliases.split('|').any(|alias| alias == norm_query) {
                return Some(AssetMatch {
                    asset: (*asset).clone(),
                    confidence: 1.0, // Exact match = highest confidence
                    matched_field: format!("Exact alias match on '{}'", norm_query),
                });
            }
        }

        // Fuzzy matching
        let mut best_match = None;
        for asset in candidates {
            let name_sim = jaro_winkler_similarity(&norm_query, &normalize_asset_name(&asset.name));
            let mut best_sim = name_sim;
            let mut matched_field = "name".to_string();

            for alias in asset.aliases.split('|').filter(|s| !s.is_empty()) {
                let alias_sim = jaro_winkler_similarity(&norm_query, &normalize_asset_name(alias));
                if alias_sim > best_sim {
                    best_sim = alias_sim;
                    matched_field = format!("alias:{}", alias);
                }
            }

            if best_sim >= min_confidence {
                let is_better = best_match.as_ref().map_or(true, |current: &AssetMatch| {
                    best_sim > current.confidence ||
                    (best_sim == current.confidence && asset.name.len() <= current.asset.name.len())
                });

                if is_better {
                    best_match = Some(AssetMatch {
                        asset: asset.clone(),
                        confidence: best_sim,
                        matched_field,
                    });
                }
            }
        }

        best_match
    }
}

// Global resolver instance
static RESOLVER: LazyLock<AssetResolver> = LazyLock::new(AssetResolver::new);

// =============================================================================
// TICKER RESOLUTION LOGIC
// =============================================================================

/// Major quote currencies for automatic detection (in priority order)
const MAJOR_QUOTE_SYMBOLS: &[&str] = &[
    "usdt",  // Tether
    "usdc",  // USD Coin
    "usd",   // US Dollar
    "eur",   // Euro
    "gbp",   // British Pound
    "jpy",   // Japanese Yen
    "cad",   // Canadian Dollar
    "aud",   // Australian Dollar
    "chf",   // Swiss Franc
    "btc",   // Bitcoin
    "eth",   // Ethereum
];

/// Detect quote currency in a ticker symbol using dynamic resolution
fn detect_quote_currency(symbol: &str) -> Option<(Asset, String, String)> {
    let lower_symbol = symbol.to_lowercase();

    for &quote_symbol in MAJOR_QUOTE_SYMBOLS {
        // Check if quote is at the end
        if lower_symbol.ends_with(quote_symbol) && lower_symbol.len() > quote_symbol.len() {
            let remaining = &lower_symbol[..lower_symbol.len() - quote_symbol.len()];
            // Remove common separators (excluding '-' to preserve asset names like "Delta-Neutral")
            let remaining = remaining.trim_end_matches(&['/', '_', '.'][..]);

            if !remaining.is_empty() {
                // Try to resolve the quote symbol dynamically
                if let Some(quote_match) = RESOLVER.find(quote_symbol, 0.95, None) {
                    return Some((quote_match.asset, remaining.to_string(), "end".to_string()));
                }
            }
        }

        // Check if quote is at the beginning
        if lower_symbol.starts_with(quote_symbol) && lower_symbol.len() > quote_symbol.len() {
            let remaining = &lower_symbol[quote_symbol.len()..];
            // Remove common separators (excluding '-' to preserve asset names like "Delta-Neutral")
            let remaining = remaining.trim_start_matches(&['/', '_', '.'][..]);

            if !remaining.is_empty() {
                // Try to resolve the quote symbol dynamically
                if let Some(quote_match) = RESOLVER.find(quote_symbol, 0.95, None) {
                    return Some((quote_match.asset, remaining.to_string(), "start".to_string()));
                }
            }
        }
    }

    None
}

/// Resolve a ticker symbol across all asset classes
pub fn resolve_ticker(symbol: &str, instrument_type: InstrumentType) -> Result<TickerMatch, MitchError> {
    let mut processing_steps = Vec::new();
    let original_symbol = symbol.to_string();

    // Step 1: Strip common suffixes
    let cleaned_symbol = strip_ticker_suffixes(symbol);
    if cleaned_symbol != symbol.to_lowercase() {
        processing_steps.push(format!("Stripped suffixes: {} -> {}", symbol, cleaned_symbol));
    }

    // Step 2: Detect quote currency
    if let Some((quote_asset, remaining_symbol, position)) = detect_quote_currency(&cleaned_symbol) {
        processing_steps.push(format!("Detected quote {} at {}: remaining '{}'", quote_asset.name, position, remaining_symbol));

        // Step 3: Resolve remaining symbol as base asset
                 if remaining_symbol.is_empty() {
             // Special case: just the quote currency (e.g., "EUR")
             // Use the detected asset as base and USD as quote
             if let Some(usd_match) = RESOLVER.find("usd", 0.95, Some(AssetClass::FX)) {
                 let ticker_id = TickerId::new(
                     instrument_type,
                     quote_asset.class,
                     quote_asset.class_id,
                     usd_match.asset.class,
                     usd_match.asset.class_id,
                     0,
                 )?;

                             let ticker = Ticker {
                     id: ticker_id.raw,
                     name: format!("{}/USD", quote_asset.name),
                     instrument_type,
                     base: quote_asset,
                     quote: usd_match.asset,
                     sub_type: 0,
                 };

                 processing_steps.push("Used detected asset as base with USD quote".to_string());

                 return Ok(TickerMatch {
                     ticker,
                     confidence: 0.9, // High confidence for this special case
                     processing_steps,
                 });
             } else {
                 return Err(MitchError::InvalidData("Could not resolve USD as quote currency".to_string()));
             }
         }

                         // Resolve remaining symbol across all asset classes
         if let Some(base_match) = RESOLVER.find(&remaining_symbol, 0.7, None) {
                         let ticker_id = TickerId::new(
                 instrument_type,
                 base_match.asset.class,
                 base_match.asset.class_id,
                 quote_asset.class,
                 quote_asset.class_id,
                 0,
             )?;

            let base_asset_name = base_match.asset.name.clone();
            let confidence = base_match.confidence;

            let ticker = Ticker {
                id: ticker_id.raw,
                name: format!("{}/{}", base_asset_name, quote_asset.name),
                instrument_type,
                base: base_match.asset,
                quote: quote_asset,
                sub_type: 0,
            };

            processing_steps.push(format!("Resolved base asset: {} (confidence: {:.2})", base_asset_name, confidence));

            return Ok(TickerMatch {
                ticker,
                confidence: base_match.confidence,
                processing_steps,
            });
        }
    }

            // Step 4: Try to resolve the entire cleaned symbol as a single asset across all classes
    if let Some(asset_match) = RESOLVER.find(&cleaned_symbol, 0.7, None) {
        // Use USD as default quote for single-asset tickers
        if let Some(usd_match) = RESOLVER.find("usd", 0.95, Some(AssetClass::FX)) {
            let ticker_id = TickerId::new(
                instrument_type,
                asset_match.asset.class,
                asset_match.asset.class_id,
                usd_match.asset.class,
                usd_match.asset.class_id,
                0,
            )?;

                    let asset_name = asset_match.asset.name.clone();
            let confidence = asset_match.confidence;

            let ticker = Ticker {
                id: ticker_id.raw,
                name: format!("{}/USD", asset_name),
                instrument_type,
                base: asset_match.asset,
                quote: usd_match.asset,
                sub_type: 0,
            };

            processing_steps.push(format!("Resolved as single asset with USD quote: {} (confidence: {:.2})", asset_name, confidence));

            return Ok(TickerMatch {
                ticker,
                confidence: asset_match.confidence,
                processing_steps,
            });
        } else {
            return Err(MitchError::InvalidData("Could not resolve USD as quote currency".to_string()));
        }
    }

    Err(MitchError::InvalidData(format!("Unable to resolve ticker: {}", original_symbol)))
}

// =============================================================================
// PUBLIC API FUNCTIONS
// =============================================================================

/// Resolve asset by name across all asset types
pub fn resolve_asset(name: &str, min_confidence: f64) -> Option<AssetMatch> {
    RESOLVER.find(name, min_confidence, None)
}

/// Resolve asset within specific asset class
pub fn resolve_asset_in_class(name: &str, min_confidence: f64, asset_class: AssetClass) -> Option<AssetMatch> {
    RESOLVER.find(name, min_confidence, Some(asset_class))
}

/// Get asset by exact class_id and class
pub fn get_asset_by_id(asset_class: AssetClass, class_id: u16) -> Option<Asset> {
    RESOLVER.by_id.get(&(asset_class, class_id)).cloned()
}

/// Get asset by global ID (32-bit packed)
pub fn get_asset_by_global_id(global_id: u32) -> Option<Asset> {
    let (asset_class, class_id) = unpack_asset(global_id);
    get_asset_by_id(asset_class, class_id)
}

/// Create a forex ticker using convenient parameters
pub fn forex_ticker(base_id: u16, quote_id: u16, instrument_type: InstrumentType, sub_type: u32) -> Result<TickerId, MitchError> {
    TickerId::new(
        instrument_type,
        AssetClass::FX,
        base_id,
        AssetClass::FX,
        quote_id,
        sub_type,
    )
}

/// Create a crypto ticker using convenient parameters
pub fn crypto_ticker(base_id: u16, quote_id: u16, instrument_type: InstrumentType, sub_type: u32) -> Result<TickerId, MitchError> {
    TickerId::new(
        instrument_type,
        AssetClass::CR,
        base_id,
        AssetClass::CR,
        quote_id,
        sub_type,
    )
}

/// Create an equity ticker using convenient parameters (equity vs currency)
pub fn equity_ticker(equity_id: u16, quote_currency_id: u16, instrument_type: InstrumentType, sub_type: u32) -> Result<TickerId, MitchError> {
    TickerId::new(
        instrument_type,
        AssetClass::EQ,
        equity_id,
        AssetClass::FX,
        quote_currency_id,
        sub_type,
    )
}

// =============================================================================
// BATCH OPERATIONS
// =============================================================================

/// Unpack multiple ticker IDs from a buffer
pub fn unpack_ticker_batch(buffer: &[u8], count: usize) -> Result<Vec<TickerId>, MitchError> {
    let expected_size = count * 8;
    if buffer.len() < expected_size {
        return Err(MitchError::BufferTooSmall {
            expected: expected_size,
            actual: buffer.len(),
        });
    }

    let mut tickers = Vec::with_capacity(count);
    unsafe {
        let ptr = buffer.as_ptr() as *const u64;
        for i in 0..count {
            let raw = ptr.add(i).read_unaligned().to_le();
            tickers.push(TickerId::from_raw(raw));
        }
    }
    Ok(tickers)
}

/// Pack multiple ticker IDs to a buffer
pub fn pack_ticker_batch(tickers: &[TickerId]) -> Vec<u8> {
    let total_size = tickers.len() * 8;
    let mut buffer = Vec::with_capacity(total_size);

    unsafe {
        buffer.set_len(total_size);
        let mut ptr = buffer.as_mut_ptr() as *mut u64;
        for ticker in tickers {
            ptr.write_unaligned(ticker.raw.to_le());
            ptr = ptr.add(1);
        }
    }
    buffer
}
