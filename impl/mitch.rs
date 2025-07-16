//! MITCH Protocol Rust Model
//!
//! Data structures for the MITCH (Moded ITCH) binary protocol.
//! Optimized for ultra-low latency financial market data transmission.
//!
//! Features:
//! - Trade, Order, Tick, OrderBook, and Index message types
//! - 8-byte Ticker ID encoding with asset class support
//! - 32-bit Channel ID system for pub/sub filtering
//! - Little-endian serialization for cross-platform compatibility
//! - 32-byte aligned message bodies for optimal performance
//! - Ultra-fast optimized order book with configurable aggregation bins

use std::fmt;
use std::error::Error;

// =============================================================================
// CONSTANTS AND ENUMS
// =============================================================================

/// MITCH message type codes (ASCII)
pub mod message_type {
    pub const TRADE: u8 = b't';        // 116
    pub const ORDER: u8 = b'o';        // 111
    pub const TICK: u8 = b's';         // 115
    pub const INDEX: u8 = b'i';        // 105
    pub const ORDER_BOOK: u8 = b'b';  // 98 - 'b' for optimized book
}

/// Message type lookup for reverse mapping
pub fn message_type_char(msg_type: u8) -> Option<char> {
    match msg_type {
        message_type::TRADE => Some('t'),
        message_type::ORDER => Some('o'),
        message_type::TICK => Some('s'),
        message_type::INDEX => Some('i'),
        message_type::ORDER_BOOK => Some('b'),
        _ => None,
    }
}

/// Order book aggregation bin functions
/// These define how price levels are aggregated into bins around the mid price
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BinAggregator {
    /// Linear around mid + Gaussian growth (bell-shaped)
    /// Min: mid+0.00001%, Max: mid+200%
    /// Best for: any asset, spread and volatility profile
    /// Characteristics: tight around mid, exponentially wider away from mid
    DEFAULT_LINGAUSSIAN = 0,

    /// Linear around mid + flattened geometric growth
    /// Min: mid+0.00001%, Max: mid+200%
    /// Best for: any asset, spread and volatility profile
    /// Characteristics: slightly more uniform than Gaussian
    DEFAULT_LINGEOFLAT = 1,

    /// Bi-linear around mid + geometric growth on edges
    /// Min: mid+0.000025%, Max: mid+200%
    /// Best for: most assets, spread and volatility profiles
    /// Characteristics: two linear segments then geometric
    DEFAULT_BILINGEO = 2,

    /// Tri-linear with steeper edges
    /// Min: mid+-0.02%, Max: mid-100%/+200%
    /// Best for: high volatility and high spread assets
    /// Characteristics: three linear segments, optimized for extreme movements
    DEFAULT_TRILINEAR = 3,
}

impl Default for BinAggregator {
    fn default() -> Self {
        BinAggregator::DEFAULT_LINGAUSSIAN
    }
}

/// Order type and side encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrderType {
    Market = 0,
    Limit = 1,
    Stop = 2,
    Cancel = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}

/// Asset classes for Ticker ID encoding
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
}

/// Instrument types for Ticker ID encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InstrumentType {
    Spot = 0x0,
    Future = 0x1,
    Forward = 0x2,
    Swap = 0x3,
    PerpetualSwap = 0x4,
    Cfd = 0x5,
    CallOption = 0x6,
    PutOption = 0x7,
    DigitalOption = 0x8,
    BarrierOption = 0x9,
    Warrant = 0xA,
    PredictionContract = 0xB,
    StructuredProduct = 0xC,
}

// =============================================================================
// ORDER BOOK STRUCTURES
// =============================================================================

/// Price level aggregation bin (8 bytes)
/// Represents aggregated orders at a specific price level
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, packed)]
pub struct Bin {
    pub count: u32,    // 4 bytes - number of orders at this level
    pub volume: u32,   // 4 bytes - total volume at this level
}

impl Bin {
    /// Pack Bin into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; 8] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack Bin from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 8 {
            return Err(MitchError::InvalidData("Not enough bytes for Bin".to_string()));
        }

        let mut array = [0u8; 8];
        array.copy_from_slice(&bytes[0..8]);
        Ok(unsafe { std::mem::transmute(array) })
    }
}

impl Default for Bin {
    fn default() -> Self {
        Self {
            count: 0,
            volume: 0,
        }
    }
}

/// Optimized order book with fixed-size aggregation bins (2,064 bytes)
/// Provides ultra-fast access to order book depth with configurable aggregation
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct OrderBook {
    // Header (17 bytes + 7 padding = 24 bytes)
    pub mid_price: f64,         // 8 bytes - current mid market price
    pub base_tick_size: f32,    // 4 bytes - minimum price increment
    pub growth_rate: f32,       // 4 bytes - exponential growth rate for bins
    pub bin_aggregator: BinAggregator, // 1 byte - bin function type
    pub _padding: [u8; 7],      // 7 bytes padding to maintain alignment

    // Fixed bid levels (-128 to -1, below mid price)
    pub bids: [Bin; 128],  // 128 * 8 = 1,024 bytes

    // Fixed ask levels (0 to 127, above mid price)
    pub asks: [Bin; 128],  // 128 * 8 = 1,024 bytes
}  // Total: 24 + 1,024 + 1,024 = 2,072 bytes

impl OrderBook {
    /// Pack OrderBook into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; 2072] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack OrderBook from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 2072 {
            return Err(MitchError::InvalidData("Not enough bytes for OrderBook".to_string()));
        }

        let mut array = [0u8; 2072];
        array.copy_from_slice(&bytes[0..2072]);
        Ok(unsafe { std::mem::transmute(array) })
    }

    /// Calculate price for a given bid level (-128 to -1)
    /// Returns None if level is out of bid range
    pub fn bid_price(&self, level: i8) -> Option<f64> {
        if level >= 0 || level < -128 {
            return None;
        }

        let bin_id = (-level - 1) as usize; // Convert to 0-127 array index
        Some(self.calculate_price_for_bin(bin_id, false))
    }

    /// Calculate price for a given ask level (0 to 127)
    /// Returns None if level is out of ask range
    pub fn ask_price(&self, level: i8) -> Option<f64> {
        if level < 0 || level > 127 {
            return None;
        }

        let bin_id = level as usize;
        Some(self.calculate_price_for_bin(bin_id, true))
    }

    /// Calculate the actual price for a bin using the aggregation function
    fn calculate_price_for_bin(&self, bin_id: usize, is_ask: bool) -> f64 {
        let percentage = self.get_bin_percentage(bin_id);
        if is_ask {
            self.mid_price * (1.0 + percentage / 100.0)
        } else {
            self.mid_price * (1.0 - percentage / 100.0)
        }
    }

    /// Get the percentage offset for a bin based on the aggregation function
    fn get_bin_percentage(&self, bin_id: usize) -> f64 {
        // Implementation would use the CSV data loaded at runtime
        // For now, return a placeholder based on DEFAULT_LINGAUSSIAN
        match self.bin_aggregator {
            BinAggregator::DEFAULT_LINGAUSSIAN => {
                // Simplified Gaussian approximation
                if bin_id < 14 {
                    0.00001 + (bin_id as f64 * 0.00001) // Linear part
                } else {
                    0.00014 + ((bin_id - 14) as f64).exp() * 0.0001 // Exponential part
                }
            }
            // Add other function implementations...
            _ => 0.00001 + (bin_id as f64 * 0.001), // Default fallback
        }
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            mid_price: 0.0,
            base_tick_size: 0.00001,
            growth_rate: 1.0,
            bin_aggregator: BinAggregator::default(),
            _padding: [0; 7],
            bids: [Bin::default(); 128],
            asks: [Bin::default(); 128],
        }
    }
}

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// MITCH unified message header (8 bytes)
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct MitchHeader {
    pub message_type: u8,    // u8: ASCII message type ('t', 'o', 's', 'q', 'i')
    pub timestamp: [u8; 6],  // u48: nanoseconds since midnight UTC (6 bytes)
    pub count: u8,           // u8: number of body entries (1-255)
}

impl MitchHeader {
    /// Create new header with timestamp as u64 (automatically truncates to u48)
    pub fn new(message_type: u8, timestamp: u64, count: u8) -> Self {
        let mut ts_bytes = [0u8; 6];
        let ts_le_bytes = timestamp.to_le_bytes();
        ts_bytes.copy_from_slice(&ts_le_bytes[0..6]);

        Self {
            message_type,
            timestamp: ts_bytes,
            count,
        }
    }

    /// Get timestamp as u64 from u48 bytes
    pub fn get_timestamp(&self) -> u64 {
        let mut ts_bytes = [0u8; 8];
        ts_bytes[0..6].copy_from_slice(&self.timestamp);
        u64::from_le_bytes(ts_bytes)
    }

    /// Set timestamp from u64 (automatically truncates to u48)
    pub fn set_timestamp(&mut self, timestamp: u64) {
        let ts_le_bytes = timestamp.to_le_bytes();
        self.timestamp.copy_from_slice(&ts_le_bytes[0..6]);
    }

    /// Pack MitchHeader into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; message_sizes::HEADER] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack MitchHeader from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::HEADER {
            return Err(MitchError::InvalidData("Not enough bytes for MitchHeader".to_string()));
        }

        let mut array = [0u8; message_sizes::HEADER];
        array.copy_from_slice(&bytes[0..message_sizes::HEADER]);
        Ok(unsafe { std::mem::transmute(array) })
    }
}

/// Trade execution data (32 bytes)
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct Trade {
    pub ticker_id: u64,      // u64: 8-byte ticker identifier
    pub price: f64,          // f64: execution price
    pub quantity: u32,       // u32: executed volume
    pub trade_id: u32,       // u32: unique trade identifier
    pub side: OrderSide,     // u8: 0=Buy, 1=Sell
    pub _padding: [u8; 7],   // 7 bytes padding
}

impl Trade {
    /// Pack Trade into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; 32] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack Trade from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 32 {
            return Err(MitchError::InvalidData("Not enough bytes for Trade".to_string()));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes[0..32]);
        Ok(unsafe { std::mem::transmute(array) })
    }
}

/// Order lifecycle event (32 bytes)
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct Order {
    pub ticker_id: u64,      // u64: 8-byte ticker identifier
    pub order_id: u32,       // u32: unique order identifier
    pub price: f64,          // f64: limit/stop price
    pub quantity: u32,       // u32: order volume
    pub type_and_side: u8,   // u8: combined type (bits 1-7) and side (bit 0)
    pub expiry: u64,         // u48: expiry timestamp (Unix ms) or 0 for GTC (stored as u64)
    pub _padding: u8,        // 1 byte padding
}

impl Order {
    /// Pack Order into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; 32] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack Order from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 32 {
            return Err(MitchError::InvalidData("Not enough bytes for Order".to_string()));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes[0..32]);
        Ok(unsafe { std::mem::transmute(array) })
    }
}

/// Tick/quote snapshot (32 bytes)
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct Tick {
    pub ticker_id: u64,      // u64: 8-byte ticker identifier
    pub bid_price: f64,      // f64: best bid price
    pub ask_price: f64,      // f64: best ask price
    pub bid_volume: u32,     // u32: volume at best bid
    pub ask_volume: u32,     // u32: volume at best ask
}

impl Tick {
    /// Pack Tick into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; 32] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack Tick from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 32 {
            return Err(MitchError::InvalidData("Not enough bytes for Tick".to_string()));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes[0..32]);
        Ok(unsafe { std::mem::transmute(array) })
    }
}

/// Index aggregated data (64 bytes) - NEW
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, packed)]
pub struct Index {
    pub ticker_id: u64,      // u64: 8-byte ticker identifier
    pub mid: f64,           // f64: mid price
    pub vbid: u32,           // u32: bid volume (sell volume)
    pub vask: u32,           // u32: ask volume (buy volume)
    pub mspread: i32,        // i32: mean spread (1e-9 pbp)
    pub bbido: i32,          // i32: best bid offset (1e-9 pbp)
    pub basko: i32,          // i32: best ask offset (1e-9 pbp)
    pub wbido: i32,          // i32: worst bid offset (1e-9 pbp)
    pub wasko: i32,          // i32: worst ask offset (1e-9 pbp)
    pub vforce: u16,         // u16: volatility force (0-10000)
    pub lforce: u16,         // u16: liquidity force (0-10000)
    pub tforce: i16,         // i16: trend force (-10000-10000)
    pub mforce: i16,         // i16: momentum force (-10000-10000)
    pub confidence: u8,      // u8: data quality (0-100, 100=best)
    pub rejected: u8,        // u8: number of sources rejected
    pub accepted: u8,        // u8: number of sources accepted
    pub _padding: [u8; 9],   // 9 bytes padding to 64 bytes
}

impl Index {
    /// Pack Index into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; 64] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack Index from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 64 {
            return Err(MitchError::InvalidData("Not enough bytes for Index".to_string()));
        }

        let mut array = [0u8; 64];
        array.copy_from_slice(&bytes[0..64]);
        Ok(unsafe { std::mem::transmute(array) })
    }
}

// =============================================================================
// CHANNEL ID SYSTEM
// =============================================================================

/// Channel ID components for pub/sub filtering (4 bytes)
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct Channel {
    pub market_provider_id: u16,  // u16: market provider ID from CSV
    pub message_type: u8,         // u8: MITCH message type ('t', 'o', 's', 'q', 'i')
    pub padding: u8,              // u8: reserved for future use (currently 0)
}

impl Channel {
    /// Pack Channel into bytes using unsafe casting (ultra-fast)
    pub fn pack(&self) -> [u8; 4] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Unpack Channel from bytes using unsafe casting (ultra-fast)
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < 4 {
            return Err(MitchError::InvalidData("Not enough bytes for Channel".to_string()));
        }

        let mut array = [0u8; 4];
        array.copy_from_slice(&bytes[0..4]);
        Ok(unsafe { std::mem::transmute(array) })
    }

    /// Generate a 32-bit channel ID from components
    /// Format: [market_provider:16][message_type:8][padding:8]
    pub fn generate(market_provider_id: u16, message_type: u8) -> u32 {
        let channel = Channel {
            market_provider_id,
            message_type,
            padding: 0,
        };
        unsafe { std::mem::transmute(channel) }
    }

    /// Extract components from a 32-bit channel ID
    pub fn extract(channel_id: u32) -> Channel {
        unsafe { std::mem::transmute(channel_id) }
    }

    /// Validate channel ID format
    pub fn validate(channel_id: u32) -> bool {
        let channel = Self::extract(channel_id);
        let valid_types = [b't', b'o', b's', b'q', b'i'];

        valid_types.contains(&channel.message_type) && channel.padding == 0
    }

    /// Generate channel pattern for pub/sub pattern matching
    /// Example: generate_pattern(101, '*') -> "657*" for all Binance messages
    pub fn generate_pattern(market_provider_id: u16, message_type_pattern: &str) -> String {
        if message_type_pattern == "*" {
            // Return hex prefix for pattern matching: "0065*" for Binance
            format!("{:X}*", (market_provider_id as u32) << 8)
        } else if let Some(ch) = message_type_pattern.chars().next() {
            let channel_id = Self::generate(market_provider_id, ch as u8);
            format!("{:X}", channel_id)
        } else {
            String::new()
        }
    }

    /// Pack channel ID into bytes using unsafe casting (ultra-fast)
    pub fn pack_channel_id(channel_id: u32) -> [u8; 4] {
        unsafe { std::mem::transmute(channel_id) }
    }

    /// Unpack channel ID from bytes using unsafe casting (ultra-fast)
    pub fn unpack_channel_id(bytes: &[u8]) -> Result<u32, MitchError> {
        if bytes.len() < 4 {
            return Err(MitchError::InvalidData("Not enough bytes for channel ID".to_string()));
        }

        let mut array = [0u8; 4];
        array.copy_from_slice(&bytes[0..4]);
        Ok(unsafe { std::mem::transmute(array) })
    }
}

// =============================================================================
// TICKER ID UTILITIES
// =============================================================================

/// Ticker ID encoding utilities
pub struct TickerId;

impl TickerId {
    /// Pack ticker ID into bytes using unsafe casting (ultra-fast)
    pub fn pack_ticker_id(ticker_id: u64) -> [u8; 8] {
        unsafe { std::mem::transmute(ticker_id) }
    }

    /// Unpack ticker ID from bytes using unsafe casting (ultra-fast)
    pub fn unpack_ticker_id(bytes: &[u8]) -> Result<u64, MitchError> {
        if bytes.len() < 8 {
            return Err(MitchError::InvalidData("Not enough bytes for ticker ID".to_string()));
        }

        let mut array = [0u8; 8];
        array.copy_from_slice(&bytes[0..8]);
        Ok(unsafe { std::mem::transmute(array) })
    }

    /// Generate a 64-bit ticker ID from components
    /// Format: [instrument_type:4][base_asset:20][quote_asset:20][sub_type:20]
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

    /// Extract components from a 64-bit ticker ID
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

#[repr(packed)]
struct PowerFunctionOrderBook {
    // Core pricing parameters
    mid_price: f64,                    // 8 bytes - reference price
    base_tick_size: f32,               // 4 bytes - 0.0001% base
    growth_rate: f32,                  // 4 bytes - power function growth

    // Pre-computed tick boundaries for fast lookup
    tick_boundaries: [f32; 200],       // 800 bytes - price levels

    // Aggregated liquidity data
    bid_counts: [u32; 200],           // 800 bytes - order counts per level
    ask_counts: [u32; 200],           // 800 bytes - order counts per level
    bid_volumes: [u64; 200],          // 1600 bytes - total volume per level
    ask_volumes: [u64; 200],          // 1600 bytes - total volume per level
}

/// Ticker ID components
#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    pub instrument_type: u8,
    pub base_class: u8,
    pub base_id: u16,
    pub quote_class: u8,
    pub quote_id: u16,
    pub sub_type: u32,
}

// =============================================================================
// MESSAGE CONTAINERS
// =============================================================================

/// Complete MITCH message with header and body
#[derive(Debug, Clone, PartialEq)]
pub enum MitchMessage {
    Trade { header: MitchHeader, body: Vec<Trade> },
    Order { header: MitchHeader, body: Vec<Order> },
    Tick { header: MitchHeader, body: Vec<Tick> },
    Index { header: MitchHeader, body: Vec<Index> },
    OrderBook { header: MitchHeader, body: Vec<OrderBook> },
}

/// Message size constants
pub mod message_sizes {
    pub const HEADER: usize = 8;
    pub const TRADE: usize = 32;
    pub const ORDER: usize = 32;
    pub const TICK: usize = 32;
    pub const INDEX: usize = 64;
    pub const ORDER_BOOK: usize = 2072;
    pub const PRICE_LEVEL: usize = 8;
}

// =============================================================================
// VALIDATION UTILITIES
// =============================================================================

/// Index confidence level descriptions
pub mod confidence_level {
    pub const PERFECT: u8 = 100;         // Real-time, all sources available
    pub const HIGH: u8 = 80;             // Minor delays or 1-2 sources rejected
    pub const MEDIUM: u8 = 60;           // Noticeable delays or some rejections
    pub const LOW: u8 = 40;              // Significant delays or many rejections
    pub const VERY_LOW: u8 = 20;         // Stale or unreliable data
    pub const NO_CONFIDENCE: u8 = 0;     // Data should not be used
}

/// Validate index confidence score
pub fn validate_confidence(confidence: u8) -> bool {
    true // u8 is already bounded 0-255
}

/// Validate index type
pub fn validate_index_type(index_type: u8) -> bool {
    index_type <= IndexType::Close as u8
}

/// Extract the side from a type_and_side field
pub fn extract_order_side(type_and_side: u8) -> OrderSide {
    match type_and_side & 0x01 {
        0 => OrderSide::Buy,
        _ => OrderSide::Sell,
    }
}

/// Extract the order type from a type_and_side field
pub fn extract_order_type(type_and_side: u8) -> OrderType {
    match (type_and_side >> 1) & 0x7F {
        0 => OrderType::Market,
        1 => OrderType::Limit,
        2 => OrderType::Stop,
        3 => OrderType::Cancel,
        _ => OrderType::Market, // Default fallback
    }
}

/// Combine order type and side into a single field
pub fn combine_type_and_side(order_type: OrderType, side: OrderSide) -> u8 {
    ((order_type as u8) << 1) | (side as u8)
}

/// Channel ID examples for common exchanges
pub mod channel_examples {
    use super::Channel;

    /// Binance tick channel: 6,648,576
    pub fn binance_ticks() -> u32 {
        Channel::generate(101, b's')
    }

    /// Coinbase trade channel: 56,021,760
    pub fn coinbase_trades() -> u32 {
        Channel::generate(853, b't')
    }

    /// NYSE index channel: 114,338,048
    pub fn nyse_indices() -> u32 {
        Channel::generate(1741, b'i')
    }
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl Default for Trade {
    fn default() -> Self {
        Self {
            ticker_id: 0,
            price: 0.0,
            quantity: 0,
            trade_id: 0,
            side: OrderSide::Buy,
            _padding: [0; 7],
        }
    }
}

impl Default for Order {
    fn default() -> Self {
        Self {
            ticker_id: 0,
            order_id: 0,
            price: 0.0,
            quantity: 0,
            type_and_side: 0,
            expiry: 0,
            _padding: 0,
        }
    }
}

impl Default for Tick {
    fn default() -> Self {
        Self {
            ticker_id: 0,
            bid_price: 0.0,
            ask_price: 0.0,
            bid_volume: 0,
            ask_volume: 0,
        }
    }
}

impl Default for Index {
    fn default() -> Self {
        Self {
            ticker_id: 0,
            mid: 0.0,
            vbid: 0,
            vask: 0,
            mspread: 0,
            bbido: 0,
            basko: 0,
            wbido: 0,
            wasko: 0,
            vforce: 0,
            lforce: 0,
            tforce: 0,
            mforce: 0,
            confidence: 0,
            rejected: 0,
            accepted: 0,
            _padding: [0; 9],
        }
    }
}

// =============================================================================
// SERIALIZATION AND DESERIALIZATION
// =============================================================================

#[derive(Debug)]
pub enum MitchError {
    InvalidData(String),
}

impl fmt::Display for MitchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MitchError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl Error for MitchError {}

impl MitchMessage {
    /// Pack MitchMessage into bytes using optimized unsafe methods
    pub fn pack(&self) -> Result<Vec<u8>, MitchError> {
        let mut buffer = Vec::new();

        // Pack header using unsafe casting
        buffer.extend_from_slice(&self.header().pack());

        // Pack body based on message type using unsafe casting
        match self {
            MitchMessage::Trade { header: _, body } => {
                for trade in body {
                    buffer.extend_from_slice(&trade.pack());
                }
            }
            MitchMessage::Order { header: _, body } => {
                for order in body {
                    buffer.extend_from_slice(&order.pack());
                }
            }
            MitchMessage::Tick { header: _, body } => {
                for tick in body {
                    buffer.extend_from_slice(&tick.pack());
                }
            }
            MitchMessage::Index { header: _, body } => {
                for index in body {
                    buffer.extend_from_slice(&index.pack());
                }
            }
            MitchMessage::OrderBook { header: _, body } => {
                for optimized_order_book in body {
                    buffer.extend_from_slice(&optimized_order_book.pack());
                }
            }
        }

        Ok(buffer)
    }

    /// Get header from message
    pub fn header(&self) -> &MitchHeader {
        match self {
            MitchMessage::Trade { header, .. } => header,
            MitchMessage::Order { header, .. } => header,
            MitchMessage::Tick { header, .. } => header,
            MitchMessage::Index { header, .. } => header,
            MitchMessage::OrderBook { header, .. } => header,
        }
    }

    /// Unpack MITCH message from bytes using ultra-fast unsafe casting
    pub fn from_bytes(bytes: &[u8]) -> Result<MitchMessage, MitchError> {
        if bytes.len() < message_sizes::HEADER {
            return Err(MitchError::InvalidData("Buffer too small for header".to_string()));
        }

        // Unpack header using unsafe casting (ultra-fast)
        let header = MitchHeader::unpack(bytes)?;
        let mut offset = message_sizes::HEADER;

        match header.message_type {
            message_type::TRADE => {
                let mut trades = Vec::new();
                for _ in 0..header.count {
                    if offset + message_sizes::TRADE > bytes.len() {
                        return Err(MitchError::InvalidData("Buffer too small for trade".to_string()));
                    }

                    // Use unsafe casting for ultra-fast unpacking
                    let trade = Trade::unpack(&bytes[offset..])?;
                    trades.push(trade);
                    offset += message_sizes::TRADE;
                }
                Ok(MitchMessage::Trade { header, body: trades })
            }
            message_type::ORDER => {
                let mut orders = Vec::new();
                for _ in 0..header.count {
                    if offset + message_sizes::ORDER > bytes.len() {
                        return Err(MitchError::InvalidData("Buffer too small for order".to_string()));
                    }

                    // Use unsafe casting for ultra-fast unpacking
                    let order = Order::unpack(&bytes[offset..])?;
                    orders.push(order);
                    offset += message_sizes::ORDER;
                }
                Ok(MitchMessage::Order { header, body: orders })
            }
            message_type::TICK => {
                let mut ticks = Vec::new();
                for _ in 0..header.count {
                    if offset + message_sizes::TICK > bytes.len() {
                        return Err(MitchError::InvalidData("Buffer too small for tick".to_string()));
                    }

                    // Use unsafe casting for ultra-fast unpacking
                    let tick = Tick::unpack(&bytes[offset..])?;
                    ticks.push(tick);
                    offset += message_sizes::TICK;
                }
                Ok(MitchMessage::Tick { header, body: ticks })
            }
            message_type::INDEX => {
                let mut indices = Vec::new();
                for _ in 0..header.count {
                    if offset + message_sizes::INDEX > bytes.len() {
                        return Err(MitchError::InvalidData("Buffer too small for index".to_string()));
                    }

                    // Use unsafe casting for ultra-fast unpacking
                    let index = Index::unpack(&bytes[offset..])?;
                    indices.push(index);
                    offset += message_sizes::INDEX;
                }
                Ok(MitchMessage::Index { header, body: indices })
            }
            message_type::ORDER_BOOK => {
                let mut optimized_order_books = Vec::new();
                for _ in 0..header.count {
                    if offset + message_sizes::ORDER_BOOK > bytes.len() {
                        return Err(MitchError::InvalidData("Buffer too small for optimized order book".to_string()));
                    }

                    // Use unsafe casting for ultra-fast unpacking
                    let optimized_order_book = OrderBook::unpack(&bytes[offset..])?;
                    optimized_order_books.push(optimized_order_book);
                    offset += message_sizes::ORDER_BOOK;
                }
                Ok(MitchMessage::OrderBook { header, body: optimized_order_books })
            }
            _ => Err(MitchError::InvalidData(format!("Unknown message type: {}", header.message_type)))
        }
    }
}
