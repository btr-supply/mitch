//! Index Message Implementation (64 bytes)
//!
//! Index messages provide synthetic aggregated market data reflecting the state of a financial instrument across all markets.
//! They are enriched with metrics for volatility, liquidity, trend, and data quality.
//!
//! # Message Layout (64 bytes total)
//!
//! ```text
//! Offset | Field       | Size | Type    | Description
//! -------|-------------|------|---------|------------------------------------
//! 0      | Ticker ID   | 8    | u64     | Ticker identifier
//! 8      | Mid         | 8    | f64     | Synthetic mid price
//! 16     | VBid        | 4    | u32     | Aggregated bid volume
//! 20     | VAsk        | 4    | u32     | Aggregated ask volume
//! 24     | MSpread     | 4    | i32     | Mean spread (1e-9 pbp)
//! 28     | BBidO       | 4    | i32     | Best bid offset (1e-9 pbp)
//! 32     | BAskO       | 4    | i32     | Best ask offset (1e-9 pbp)
//! 36     | WBidO       | 4    | i32     | Worst bid offset (1e-9 pbp)
//! 40     | WAskO       | 4    | i32     | Worst ask offset (1e-9 pbp)
//! 44     | VForce      | 2    | u16     | Volatility force (0-10000)
//! 46     | LForce      | 2    | u16     | Liquidity force (0-10000)
//! 48     | TForce      | 2    | i16     | Trend force (-10000 to 10000)
//! 50     | MForce      | 2    | i16     | Momentum force (-10000 to 10000)
//! 52     | Confidence  | 1    | u8      | Data quality (0-100)
//! 53     | Rejected    | 1    | u8      | Number of sources rejected
//! 54     | Accepted    | 1    | u8      | Number of sources accepted
//! 55     | Padding     | 9    | u8[9]   | Padding to 64 bytes
//! ```

use crate::common::{message_sizes, MitchError};
use core::{fmt, ptr};

#[cfg(feature = "networking")]
use crate::impl_pushable;

#[cfg(feature = "networking")]
impl_pushable!(Index, 'i', ticker_id);

/// Index message structure (64 bytes)
///
/// Provides synthetic aggregated market data with metrics for volatility,
/// liquidity, trend, and data quality.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Index {
    /// Ticker identifier (8 bytes)
    pub ticker_id: u64,
    /// Synthetic mid price (8 bytes)
    pub mid: f64,
    /// Aggregated bid volume (4 bytes)
    pub v_bid: u32,
    /// Aggregated ask volume (4 bytes)
    pub v_ask: u32,
    /// Mean spread in 1e-9 price basis points (4 bytes)
    pub m_spread: i32,
    /// Best bid offset in 1e-9 pbp (4 bytes)
    pub b_bid_o: i32,
    /// Best ask offset in 1e-9 pbp (4 bytes)
    pub b_ask_o: i32,
    /// Worst bid offset in 1e-9 pbp (4 bytes)
    pub w_bid_o: i32,
    /// Worst ask offset in 1e-9 pbp (4 bytes)
    pub w_ask_o: i32,
    /// Volatility force: 0-10000 (2 bytes)
    pub v_force: u16,
    /// Liquidity force: 0-10000 (2 bytes)
    pub l_force: u16,
    /// Trend force: -10000 to 10000 (2 bytes)
    pub t_force: i16,
    /// Momentum force: -10000 to 10000 (2 bytes)
    pub m_force: i16,
    /// Data quality confidence score: 0-100 (1 byte)
    pub confidence: u8,
    /// Number of sources rejected (1 byte)
    pub rejected: u8,
    /// Number of sources accepted (1 byte)
    pub accepted: u8,
    /// Padding to 64 bytes (9 bytes)
    pub _padding: [u8; 9],
}

impl Index {
    /// Create a new Index message.
    ///
    /// # Arguments
    /// All fields are passed directly. `_padding` is automatically set to zeros.
    pub fn new(
        ticker_id: u64,
        mid: f64,
        v_bid: u32,
        v_ask: u32,
        m_spread: i32,
        b_bid_o: i32,
        b_ask_o: i32,
        w_bid_o: i32,
        w_ask_o: i32,
        v_force: u16,
        l_force: u16,
        t_force: i16,
        m_force: i16,
        confidence: u8,
        rejected: u8,
        accepted: u8,
    ) -> Self {
        Self {
            ticker_id,
            mid,
            v_bid,
            v_ask,
            m_spread,
            b_bid_o,
            b_ask_o,
            w_bid_o,
            w_ask_o,
            v_force,
            l_force,
            t_force,
            m_force,
            confidence,
            rejected,
            accepted,
            _padding: [0; 9],
        }
    }

    /// Pack Index message to bytes using zero-copy raw pointer operations
    pub fn pack(&self) -> [u8; message_sizes::INDEX] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Index message from bytes using zero-copy raw pointer operations
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::INDEX {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::INDEX,
                actual: bytes.len(),
            });
        }
        unsafe {
            let ptr = bytes.as_ptr() as *const Self;
            Ok(ptr.read_unaligned())
        }
    }

    /// Unpack from buffer slice without bounds checking (maximum performance)
    pub unsafe fn unpack_unchecked(bytes: &[u8]) -> Self {
        let ptr = bytes.as_ptr() as *const Self;
        ptr.read_unaligned()
    }

    // --- Price calculations ---
    /// Calculates the best bid price based on the mid price and the best bid offset.
    pub fn best_bid_price(&self) -> f64 {
        self.mid * (1.0 + (self.b_bid_o as f64) / 1e9)
    }
    /// Calculates the best ask price based on the mid price and the best ask offset.
    pub fn best_ask_price(&self) -> f64 {
        self.mid * (1.0 + (self.b_ask_o as f64) / 1e9)
    }

    /// Get the size of the Index struct in bytes.
    pub const fn size() -> usize {
        message_sizes::INDEX
    }

    // --- Force analysis ---
    /// Returns the volatility force as a percentage (0.00 - 100.00).
    pub fn volatility_percentage(&self) -> f64 { self.v_force as f64 / 100.0 }
    /// Returns the liquidity force as a percentage (0.00 - 100.00).
    pub fn liquidity_percentage(&self) -> f64 { self.l_force as f64 / 100.0 }
    /// Returns the trend force as a percentage (-100.00 - 100.00).
    pub fn trend_percentage(&self) -> f64 { self.t_force as f64 / 100.0 }
    /// Returns the momentum force as a percentage (-100.00 - 100.00).
    pub fn momentum_percentage(&self) -> f64 { self.m_force as f64 / 100.0 }


    /// Validate message data integrity
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker_id == 0 { return Err(MitchError::InvalidFieldValue("Ticker ID cannot be zero".into())); }
        if self.mid <= 0.0 { return Err(MitchError::InvalidFieldValue("Mid price must be positive".into())); }
        if self.confidence > 100 { return Err(MitchError::InvalidFieldValue("Confidence must be 0-100".into())); }
        if self.v_force > 10000 || self.l_force > 10000 { return Err(MitchError::InvalidFieldValue("Forces out of range".into())); }
        if self.t_force < -10000 || self.t_force > 10000 { return Err(MitchError::InvalidFieldValue("Trend force out of range".into())); }
        if self.m_force < -10000 || self.m_force > 10000 { return Err(MitchError::InvalidFieldValue("Momentum force out of range".into())); }
        if self.accepted == 0 && self.confidence > 0 { return Err(MitchError::InvalidFieldValue("Cannot have confidence without sources".into())); }
        Ok(())
    }
}

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ticker_id = unsafe { ptr::addr_of!(self.ticker_id).read_unaligned() };
        let mid = unsafe { ptr::addr_of!(self.mid).read_unaligned() };
        let confidence = unsafe { ptr::addr_of!(self.confidence).read_unaligned() };

        write!(
            f,
            "INDEX | Ticker: {:#018X} | Mid: {:.5} | Confidence: {} | VForce: {:.2}% | LForce: {:.2}% | TForce: {:.2}% | MForce: {:.2}%",
            ticker_id,
            mid,
            confidence,
            self.volatility_percentage(),
            self.liquidity_percentage(),
            self.trend_percentage(),
            self.momentum_percentage(),
        )
    }
}

// =============================================================================
// BATCH OPERATIONS
// =============================================================================

/// Unpack multiple Index messages from a buffer using zero-copy operations
pub fn unpack_index_batch(buffer: &[u8], count: u8) -> Result<Vec<Index>, MitchError> {
    let expected_size = count as usize * message_sizes::INDEX;
    if buffer.len() < expected_size {
        return Err(MitchError::BufferTooSmall {
            expected: expected_size,
            actual: buffer.len(),
        });
    }

    let mut messages = Vec::with_capacity(count as usize);
    unsafe {
        let ptr = buffer.as_ptr() as *const Index;
        for i in 0..count as usize {
            messages.push(ptr.add(i).read_unaligned());
        }
    }
    Ok(messages)
}

/// Pack multiple Index messages to a buffer using zero-copy operations
pub fn pack_index_batch(messages: &[Index]) -> Vec<u8> {
    let total_size = messages.len() * message_sizes::INDEX;
    let mut buffer = Vec::with_capacity(total_size);
    unsafe {
        let src_ptr = messages.as_ptr() as *const u8;
        let dst_ptr = buffer.as_mut_ptr();
        ptr::copy_nonoverlapping(src_ptr, dst_ptr, total_size);
        buffer.set_len(total_size);
    }
    buffer
}

// =============================================================================
// NETWORKING IMPLEMENTATION
// =============================================================================
