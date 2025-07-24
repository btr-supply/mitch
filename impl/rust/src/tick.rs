//! MITCH Tick message implementation
//!
//! Tick messages (`s`) provide point-in-time bid/ask ticker snapshots representing
//! the best prices and activity in a market. They capture level-1 (top of book) data.

use crate::common::{message_sizes, MitchError};
use core::ptr;

#[cfg(feature = "networking")]
impl_pushable!(Tick, 's', ticker_id);

/// Tick/quote snapshot (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub ticker_id: u64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_volume: u32,
    pub ask_volume: u32,
}

impl Tick {
    /// Create a new Tick message with validation.
    pub fn new(
        ticker_id: u64,
        bid_price: f64,
        ask_price: f64,
        bid_volume: u32,
        ask_volume: u32,
    ) -> Result<Self, MitchError> {
        let tick = Self {
            ticker_id,
            bid_price,
            ask_price,
            bid_volume,
            ask_volume,
        };
        tick.validate()?;
        Ok(tick)
    }

    /// Pack Tick into bytes using zero-copy transmutation.
    pub fn pack(&self) -> [u8; message_sizes::TICK] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Tick from bytes using a zero-copy read.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::TICK {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::TICK,
                actual: bytes.len(),
            });
        }
        let tick = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        tick.validate()?;
        Ok(tick)
    }

    /// Validate the contents of the Tick message.
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker_id == 0 {
            return Err(MitchError::InvalidFieldValue("ticker_id cannot be zero".into()));
        }
        if self.bid_price <= 0.0 {
            return Err(MitchError::InvalidFieldValue("bid_price must be positive".into()));
        }
        if self.ask_price <= 0.0 {
            return Err(MitchError::InvalidFieldValue("ask_price must be positive".into()));
        }
        if self.ask_price < self.bid_price {
            return Err(MitchError::InvalidFieldValue("ask_price cannot be less than bid_price".into()));
        }
        Ok(())
    }

    /// Calculate mid price.
    pub fn mid_price(&self) -> f64 {
        (self.bid_price + self.ask_price) / 2.0
    }

    /// Calculate spread.
    pub fn spread(&self) -> f64 {
        self.ask_price - self.bid_price
    }

    /// Calculate spread in basis points.
    pub fn spread_bps(&self) -> f64 {
        let mid = self.mid_price();
        if mid > 0.0 {
            (self.spread() / mid) * 10000.0
        } else {
            0.0
        }
    }

    /// Calculate total volume (bid + ask).
    pub fn total_volume(&self) -> u64 {
        self.bid_volume as u64 + self.ask_volume as u64
    }

    /// Calculate volume imbalance.
    /// Returns a value between -1.0 (all ask volume) and 1.0 (all bid volume).
    pub fn volume_imbalance(&self) -> f64 {
        let total = self.total_volume() as f64;
        if total > 0.0 {
            (self.ask_volume as f64 - self.bid_volume as f64) / total
        } else {
            0.0
        }
    }

    /// Get the size of the Tick struct in bytes.
    pub const fn size() -> usize {
        message_sizes::TICK
    }
}

// =============================================================================
// BATCH OPERATIONS
// =============================================================================

/// Pack multiple ticks into a single byte vector using a bulk memory copy.
pub fn pack_ticks(ticks: &[Tick]) -> Vec<u8> {
    let total_size = ticks.len() * message_sizes::TICK;
    let mut buffer = Vec::with_capacity(total_size);
    unsafe {
        let src_ptr = ticks.as_ptr() as *const u8;
        ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), total_size);
        buffer.set_len(total_size);
    }
    buffer
}

/// Unpack multiple ticks from a byte slice using efficient, unsafe operations.
pub fn unpack_ticks(bytes: &[u8], count: usize) -> Result<Vec<Tick>, MitchError> {
    let expected_size = count * message_sizes::TICK;
    if bytes.len() < expected_size {
        return Err(MitchError::BufferTooSmall {
            expected: expected_size,
            actual: bytes.len(),
        });
    }

    let mut messages = Vec::with_capacity(count);
    unsafe {
        let ptr = bytes.as_ptr() as *const Tick;
        for i in 0..count {
            messages.push(ptr.add(i).read_unaligned());
        }
    }

    Ok(messages)
}


