//! MITCH Trade message implementation
//!
//! Trade messages (`t`) represent executed transactions in a market,
//! capturing price, volume, participant, and timing information.

use crate::common::{message_sizes, OrderSide, MitchError};
use core::ptr;

#[cfg(feature = "networking")]
impl_pushable!(Trade, 't', ticker_id);

/// Trade execution data (32 bytes)
///
/// Structure per trade.md specification:
/// | Field     | Offset | Size | Type        | Description                            |
/// |-----------|--------|------|-------------|----------------------------------------|
/// | Ticker ID | 0      | 8    | `u64`       | 8-byte ticker identifier               |
/// | Price     | 8      | 8    | `f64`       | Execution price                        |
/// | Quantity  | 16     | 4    | `u32`       | Executed volume/quantity               |
/// | Trade ID  | 20     | 4    | `u32`       | Required unique trade identifier       |
/// | Side      | 24     | 1    | `OrderSide` | 0: Buy, 1: Sell                       |
/// | Padding   | 25     | 7    | `u8[7]`     | Padding to 32 bytes                    |
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trade {
    /// Ticker identifier (8 bytes)
    pub ticker_id: u64,
    /// Execution price (8 bytes)
    pub price: f64,
    /// Executed volume (4 bytes)
    pub quantity: u32,
    /// Unique trade identifier (4 bytes)
    pub trade_id: u32,
    /// Trade side (1 byte: 0=Buy, 1=Sell)
    pub side: OrderSide,
    /// Padding to 32 bytes (7 bytes)
    pub _padding: [u8; 7],
}

impl Trade {
    /// Create a new Trade message with validation.
    ///
    /// # Arguments
    /// * `ticker_id` - 8-byte ticker identifier (must be non-zero).
    /// * `price` - Execution price (must be positive).
    /// * `quantity` - Executed volume (must be positive).
    /// * `trade_id` - Unique trade identifier (must be non-zero).
    /// * `side` - `OrderSide::Buy` or `OrderSide::Sell`.
    ///
    /// # Returns
    /// A `Result` containing the new `Trade` instance or a `MitchError`.
    pub fn new(
        ticker_id: u64,
        price: f64,
        quantity: u32,
        trade_id: u32,
        side: OrderSide,
    ) -> Result<Self, MitchError> {
        let trade = Self {
            ticker_id,
            price,
            quantity,
            trade_id,
            side,
            _padding: [0; 7],
        };
        trade.validate()?;
        Ok(trade)
    }

    /// Pack Trade into bytes using zero-copy transmutation.
    pub fn pack(&self) -> [u8; message_sizes::TRADE] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Trade from bytes using a zero-copy read.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::TRADE {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::TRADE,
                actual: bytes.len(),
            });
        }
        let trade = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        trade.validate()?;
        Ok(trade)
    }

    /// Validate the contents of the Trade message.
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker_id == 0 {
            return Err(MitchError::InvalidFieldValue("ticker_id cannot be zero".into()));
        }
        if self.price <= 0.0 {
            return Err(MitchError::InvalidFieldValue("price must be positive".into()));
        }
        if self.quantity == 0 {
            return Err(MitchError::InvalidFieldValue("quantity must be positive".into()));
        }
        if self.trade_id == 0 {
            return Err(MitchError::InvalidFieldValue("trade_id cannot be zero".into()));
        }
        Ok(())
    }

    /// Get the size of the Trade struct in bytes.
    pub const fn size() -> usize {
        message_sizes::TRADE
    }

    /// Get notional value (price * quantity).
    pub fn notional_value(&self) -> f64 {
        self.price * self.quantity as f64
    }

    /// Check if this is a buy trade.
    pub fn is_buy(&self) -> bool {
        matches!(self.side, OrderSide::Buy)
    }

    /// Check if this is a sell trade.
    pub fn is_sell(&self) -> bool {
        matches!(self.side, OrderSide::Sell)
    }
}

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

// =============================================================================
// DISPLAY IMPLEMENTATIONS
// =============================================================================

impl core::fmt::Display for Trade {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let ticker_id = self.ticker_id;
        let price = self.price;
        let quantity = self.quantity;
        let trade_id = self.trade_id;
        let side = self.side;

        write!(
            f,
            "Trade {{ ticker: {:#X}, price: {}, qty: {}, id: {}, side: {:?} }}",
            ticker_id, price, quantity, trade_id, side
        )
    }
}

// =============================================================================
// BATCH OPERATIONS
// =============================================================================

/// Pack multiple trades into a single byte vector using a bulk memory copy.
pub fn pack_trades(trades: &[Trade]) -> Vec<u8> {
    let total_size = trades.len() * message_sizes::TRADE;
    let mut buffer = Vec::with_capacity(total_size);
    unsafe {
        let src_ptr = trades.as_ptr() as *const u8;
        ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), total_size);
        buffer.set_len(total_size);
    }
    buffer
}

/// Unpack multiple trades from a byte slice using efficient, unsafe operations.
pub fn unpack_trades(bytes: &[u8], count: usize) -> Result<Vec<Trade>, MitchError> {
    let expected_size = count * message_sizes::TRADE;
    if bytes.len() < expected_size {
        return Err(MitchError::BufferTooSmall {
            expected: expected_size,
            actual: bytes.len(),
        });
    }

    let mut messages = Vec::with_capacity(count);
    unsafe {
        let ptr = bytes.as_ptr() as *const Trade;
        for i in 0..count {
            messages.push(ptr.add(i).read_unaligned());
        }
    }

    // Optional: Validate all messages after unpacking
    // for msg in &messages {
    //     msg.validate()?;
    // }

    Ok(messages)
}

// =============================================================================
// NETWORKING IMPLEMENTATION
// =============================================================================
