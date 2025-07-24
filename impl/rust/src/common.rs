//! Common types, enums, and constants used across MITCH protocol messages
//!
//! This module defines the foundational types that are shared between different
//! message types in the MITCH protocol, ensuring consistency and type safety.

use core::fmt;

// =============================================================================
// MESSAGE TYPE CONSTANTS
// =============================================================================

/// MITCH message type codes (ASCII)
pub mod message_type {
    /// Trade message ('t')
    pub const TRADE: u8 = b't';        // 116
    /// Order message ('o')
    pub const ORDER: u8 = b'o';        // 111
    /// Tick message ('s')
    pub const TICK: u8 = b's';         // 115
    /// Index message ('i')
    pub const INDEX: u8 = b'i';        // 105
    /// Order book message ('b')
    pub const ORDER_BOOK: u8 = b'b';   // 98
}

/// Message size constants in bytes
pub mod message_sizes {
    /// Asset size
    pub const ASSET: usize = 20;
    /// Ticker size
    pub const TICKER: usize = 64;
    /// Header size
    pub const HEADER: usize = 8;
    /// Trade body size
    pub const TRADE: usize = 32;
    /// Order body size
    pub const ORDER: usize = 32;
    /// Tick body size
    pub const TICK: usize = 32;
    /// Index body size
    pub const INDEX: usize = 64;
    /// Order book body size
    pub const ORDER_BOOK: usize = 2072;
    /// Bin size
    pub const BIN: usize = 8;
}

// =============================================================================
// TRADING ENUMS
// =============================================================================

/// Order side enumeration (buy/sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrderSide {
    /// Buy side (0)
    Buy = 0,
    /// Sell side (1)
    Sell = 1,
}

impl Default for OrderSide {
    fn default() -> Self {
        OrderSide::Buy
    }
}

/// Order type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrderType {
    /// Market order (0)
    Market = 0,
    /// Limit order (1)
    Limit = 1,
    /// Stop order (2)
    Stop = 2,
    /// Cancel order (3)
    Cancel = 3,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Market
    }
}

// =============================================================================
// ASSET CLASSIFICATION
// =============================================================================

/// Re-export types from constants module
pub use crate::constants::{AssetClass, InstrumentType, BinAggregator};

// =============================================================================
// ERROR HANDLING
// =============================================================================

/// Custom error type for MITCH protocol operations.
#[derive(Debug, PartialEq)]
pub enum MitchError {
    /// Error representing invalid or corrupt data.
    InvalidData(String),
    /// Error for an unrecognized message type.
    InvalidMessageType(u8),
    /// Error indicating a buffer is too small for a message.
    BufferTooSmall {
        /// The expected size of the buffer.
        expected: usize,
        /// The actual size of the buffer.
        actual: usize,
    },
    /// Error for an invalid ticker ID.
    InvalidTickerId(String),
    /// Error for an invalid channel ID.
    InvalidChannelId(String),
    /// Error for a field containing an invalid value.
    InvalidFieldValue(String),
    /// Error during serialization.
    SerializationError(String),
}

impl fmt::Display for MitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MitchError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            MitchError::InvalidMessageType(t) => write!(f, "Invalid message type: {}", t),
            MitchError::BufferTooSmall { expected, actual } => {
                write!(f, "Buffer too small: expected {}, got {}", expected, actual)
            }
            MitchError::InvalidTickerId(msg) => write!(f, "Invalid ticker ID: {}", msg),
            MitchError::InvalidChannelId(msg) => write!(f, "Invalid channel ID: {}", msg),
            MitchError::InvalidFieldValue(msg) => write!(f, "Invalid field value: {}", msg),
            MitchError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MitchError {}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Extract the side from a combined type_and_side field (bit 0)
pub fn extract_order_side(type_and_side: u8) -> OrderSide {
    match type_and_side & 0x01 {
        0 => OrderSide::Buy,
        _ => OrderSide::Sell,
    }
}

/// Extract the order type from a combined type_and_side field (bits 1-7)
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

/// Get the ASCII character for a message type code
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

/// Validate that a message type is supported
pub fn validate_message_type(msg_type: u8) -> Result<(), MitchError> {
    match message_type_char(msg_type) {
        Some(_) => Ok(()),
        None => Err(MitchError::InvalidMessageType(msg_type)),
    }
}

// =============================================================================
// CONFIDENCE LEVELS
// =============================================================================

/// # Confidence Levels
#[allow(dead_code)]
pub mod confidence {
    /// Real-time, all sources available
    pub const PERFECT: u8 = 95;
    /// Minor delays or 1-2 sources rejected
    pub const HIGH: u8 = 80;
    /// Noticeable delays or some rejections
    pub const MEDIUM: u8 = 60;
    /// Significant delays or many rejections
    pub const LOW: u8 = 40;
    /// Stale or unreliable data
    pub const VERY_LOW: u8 = 20;
    /// Data should not be used
    pub const NO_CONFIDENCE: u8 = 0;
}

/// Validate index confidence score (0-100)
pub fn validate_confidence(confidence: u8) -> bool {
    confidence <= 100 // u8 is already bounded 0-255, just check upper
}
