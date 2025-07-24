//! OrderBook Message Implementation (2072 bytes)
//!
//! OrderBook messages provide complete market depth snapshots with aggregated
//! price levels in adaptive bins. Optimized for real-time order book state
//! distribution with minimal bandwidth overhead.
//!
//! # Message Layout (2072 bytes total)
//!
//! ```text
//! Offset | Field             | Type      | Size | Description
//! -------|-------------------|-----------|------|---------------------------
//! 0      | ticker_id         | u64       | 8    | Ticker identifier
//! 8      | mid_price         | f64       | 8    | Calculated mid price
//! 16     | bin_aggregator    | u8        | 1    | Aggregation function used
//! 17     | _padding          | [u8; 7]   | 7    | Padding for alignment
//! 24     | bids              | [Bin; 128]| 1024 | Bid levels (aggregated)
//! 1048   | asks              | [Bin; 128]| 1024 | Ask levels (aggregated)
//! ```

use crate::common::{message_sizes, MitchError, BinAggregator};
use core::ptr;

#[cfg(feature = "networking")]
impl_pushable!(OrderBook, 'b', ticker_id);

/// Bin structure for aggregated price levels (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bin {
    /// Number of orders (4 bytes)
    pub order_count: u32,
    /// Total volume (4 bytes)
    pub volume: u32,
}

impl Bin {
    /// Create a new bin
    pub fn new(order_count: u32, volume: u32) -> Self {
        Self { order_count, volume }
    }

    /// Check if bin is empty
    pub fn is_empty(&self) -> bool {
        self.volume == 0
    }
}

/// OrderBook message structure (2072 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct OrderBook {
    /// Ticker identifier (8 bytes)
    pub ticker_id: u64,
    /// Mid price (8 bytes)
    pub mid_price: f64,
    /// Bin aggregator ID (1 byte)
    pub bin_aggregator: u8,
    /// Padding (7 bytes)
    pub _padding: [u8; 7],
    /// Bid bins (1024 bytes)
    pub bids: [Bin; 128],
    /// Ask bins (1024 bytes)
    pub asks: [Bin; 128],
}

impl OrderBook {
    /// Create a new OrderBook message
    ///
    /// # Arguments
    /// * `ticker_id` - Ticker identifier
    /// * `mid_price` - Calculated mid price
    /// * `bin_aggregator` - Aggregation function ID (0-3)
    /// * `bids` - Array of 128 bid bins
    /// * `asks` - Array of 128 ask bins
    pub fn new(
        ticker_id: u64,
        mid_price: f64,
        bin_aggregator: u8,
        bids: [Bin; 128],
        asks: [Bin; 128],
    ) -> Self {
        Self {
            ticker_id,
            mid_price,
            bin_aggregator,
            _padding: [0; 7],
            bids,
            asks,
        }
    }

    /// Pack to bytes using zero-copy transmutation
    pub fn pack(&self) -> [u8; message_sizes::ORDER_BOOK] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack from bytes using zero-copy read
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::ORDER_BOOK {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::ORDER_BOOK,
                actual: bytes.len(),
            });
        }
        unsafe {
            let ptr = bytes.as_ptr() as *const Self;
            Ok(ptr.read_unaligned())
        }
    }

    /// Unpack without bounds check (unsafe, max performance)
    pub unsafe fn unpack_unchecked(bytes: &[u8]) -> Self {
        let ptr = bytes.as_ptr() as *const Self;
        ptr.read_unaligned()
    }

    /// Calculate total bid volume
    pub fn total_bid_volume(&self) -> u64 {
        let bids = unsafe { std::ptr::addr_of!(self.bids).read_unaligned() };
        bids.iter().map(|bin| bin.volume as u64).sum()
    }

    /// Calculate total ask volume
    pub fn total_ask_volume(&self) -> u64 {
        let asks = unsafe { std::ptr::addr_of!(self.asks).read_unaligned() };
        asks.iter().map(|bin| bin.volume as u64).sum()
    }

    /// Get bin aggregator enum
    pub fn aggregator_type(&self) -> BinAggregator {
        match self.bin_aggregator {
            0 => BinAggregator::DEFAULT_LINGAUSSIAN,
            1 => BinAggregator::DEFAULT_LINGEOFLAT,
            2 => BinAggregator::DEFAULT_BILINGEO,
            3 => BinAggregator::DEFAULT_TRILINEAR,
            _ => BinAggregator::DEFAULT_LINGAUSSIAN,
        }
    }

    /// Validate message integrity
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.mid_price <= 0.0 {
            return Err(MitchError::InvalidFieldValue("mid_price".into()));
        }
        if self.bin_aggregator > 3 {
            return Err(MitchError::InvalidFieldValue("bin_aggregator".into()));
        }
        Ok(())
    }

    /// Get the size of the OrderBook struct in bytes.
    pub const fn size() -> usize {
        message_sizes::ORDER_BOOK
    }
}

impl core::fmt::Display for OrderBook {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ticker_id = unsafe { std::ptr::addr_of!(self.ticker_id).read_unaligned() };
        let mid_price = unsafe { std::ptr::addr_of!(self.mid_price).read_unaligned() };
        let bin_aggregator = unsafe { std::ptr::addr_of!(self.bin_aggregator).read_unaligned() };
        write!(
            f,
            "OrderBook(ticker={:016X}, mid={:.6}, aggregator={}, bid_vol={}, ask_vol={})",
            ticker_id,
            mid_price,
            bin_aggregator,
            self.total_bid_volume(),
            self.total_ask_volume()
        )
    }
}

// Batch unpack
pub fn unpack_order_book_batch(buffer: &[u8], count: u8) -> Result<Vec<OrderBook>, MitchError> {
    let size = message_sizes::ORDER_BOOK * count as usize;
    if buffer.len() < size {
        return Err(MitchError::BufferTooSmall { expected: size, actual: buffer.len() });
    }
    let mut vec = Vec::with_capacity(count as usize);
    unsafe {
        let ptr = buffer.as_ptr() as *const OrderBook;
        for i in 0..count as usize {
            vec.push(ptr.add(i).read_unaligned());
        }
    }
    Ok(vec)
}

// Batch pack
pub fn pack_order_book_batch(messages: &[OrderBook]) -> Vec<u8> {
    let size = message_sizes::ORDER_BOOK * messages.len();
    let mut buffer = Vec::with_capacity(size);
    unsafe {
        ptr::copy_nonoverlapping(messages.as_ptr() as *const u8, buffer.as_mut_ptr(), size);
        buffer.set_len(size);
    }
    buffer
}


