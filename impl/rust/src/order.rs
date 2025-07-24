//! MITCH Order message implementation
//!
//! Order messages (`o`) represent order lifecycle events in financial markets,
//! capturing order placement, modification, and cancellation events.

use crate::common::{
    message_sizes, combine_type_and_side, extract_order_side, extract_order_type, MitchError,
    OrderSide, OrderType,
};
use crate::utils::{timestamp_to_u48, u48_to_timestamp};
use core::ptr;

#[cfg(feature = "networking")]
impl_pushable!(Order, 'o', ticker_id);

/// Order lifecycle event (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    pub ticker_id: u64,
    pub order_id: u32,
    pub price: f64,
    pub quantity: u32,
    pub type_and_side: u8,
    pub expiry: [u8; 6],
    pub _padding: u8,
}

impl Order {
    /// Create a new Order message with validation.
    pub fn new(
        ticker_id: u64,
        order_id: u32,
        price: f64,
        quantity: u32,
        order_type: OrderType,
        side: OrderSide,
        expiry_ms: u64,
    ) -> Result<Self, MitchError> {
        let order = Self {
            ticker_id,
            order_id,
            price,
            quantity,
            type_and_side: combine_type_and_side(order_type, side),
            expiry: timestamp_to_u48(expiry_ms),
            _padding: 0,
        };
        order.validate()?;
        Ok(order)
    }

    /// Pack Order into bytes using zero-copy transmutation.
    pub fn pack(&self) -> [u8; message_sizes::ORDER] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Order from bytes using a zero-copy read.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::ORDER {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::ORDER,
                actual: bytes.len(),
            });
        }
        let order = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        order.validate()?;
        Ok(order)
    }

    /// Get the order type from the combined field.
    pub fn get_order_type(&self) -> OrderType {
        extract_order_type(self.type_and_side)
    }

    /// Get the order side from the combined field.
    pub fn get_order_side(&self) -> OrderSide {
        extract_order_side(self.type_and_side)
    }

    /// Get expiry timestamp as u64 from u48 bytes (in milliseconds).
    pub fn get_expiry(&self) -> u64 {
        u48_to_timestamp(self.expiry)
    }

    /// Set expiry timestamp from u64 (in milliseconds).
    pub fn set_expiry(&mut self, expiry_ms: u64) {
        self.expiry = timestamp_to_u48(expiry_ms);
    }

    /// Check if this is a Good Till Cancel (GTC) order.
    pub fn is_gtc(&self) -> bool {
        self.get_expiry() == 0
    }

    /// Check if this order has expired at a given timestamp.
    pub fn is_expired(&self, current_time_ms: u64) -> bool {
        let expiry = self.get_expiry();
        expiry != 0 && current_time_ms > expiry
    }

    /// Validate the contents of the Order message.
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker_id == 0 {
            return Err(MitchError::InvalidFieldValue("ticker_id cannot be zero".into()));
        }
        if self.order_id == 0 {
            return Err(MitchError::InvalidFieldValue("order_id cannot be zero".into()));
        }

        match self.get_order_type() {
            OrderType::Market | OrderType::Limit | OrderType::Stop => {
                if self.price <= 0.0 {
                    return Err(MitchError::InvalidFieldValue("price must be positive for this order type".into()));
                }
                if self.quantity == 0 {
                    return Err(MitchError::InvalidFieldValue("quantity must be positive for this order type".into()));
                }
            }
            OrderType::Cancel => {} // No price/quantity validation for Cancel orders
        }

        Ok(())
    }

    /// Check if this is a buy order.
    pub fn is_buy(&self) -> bool {
        matches!(self.get_order_side(), OrderSide::Buy)
    }

    /// Check if this is a sell order.
    pub fn is_sell(&self) -> bool {
        matches!(self.get_order_side(), OrderSide::Sell)
    }

    /// Calculate the notional value (price * quantity).
    pub fn notional_value(&self) -> f64 {
        self.price * self.quantity as f64
    }

    /// Get the size of the Order struct in bytes.
    pub const fn size() -> usize {
        message_sizes::ORDER
    }
}

// =============================================================================
// BATCH OPERATIONS
// =============================================================================

/// Pack multiple orders into a single byte vector using a bulk memory copy.
pub fn pack_orders(orders: &[Order]) -> Vec<u8> {
    let total_size = orders.len() * message_sizes::ORDER;
    let mut buffer = Vec::with_capacity(total_size);
    unsafe {
        let src_ptr = orders.as_ptr() as *const u8;
        ptr::copy_nonoverlapping(src_ptr, buffer.as_mut_ptr(), total_size);
        buffer.set_len(total_size);
    }
    buffer
}

/// Unpack multiple orders from a byte slice using efficient, unsafe operations.
pub fn unpack_orders(bytes: &[u8], count: usize) -> Result<Vec<Order>, MitchError> {
    let expected_size = count * message_sizes::ORDER;
    if bytes.len() < expected_size {
        return Err(MitchError::BufferTooSmall {
            expected: expected_size,
            actual: bytes.len(),
        });
    }

    let mut messages = Vec::with_capacity(count);
    unsafe {
        let ptr = bytes.as_ptr() as *const Order;
        for i in 0..count {
            messages.push(ptr.add(i).read_unaligned());
        }
    }

    Ok(messages)
}
