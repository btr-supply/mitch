// MITCH Protocol Message Structures
// All multi-byte fields are in Big-Endian byte order

// Message type constants
pub const MSG_TYPE_TRADE: u8 = b't';
pub const MSG_TYPE_ORDER: u8 = b'o';
pub const MSG_TYPE_TICKER: u8 = b's';
pub const MSG_TYPE_ORDER_BOOK: u8 = b'q';

// Side constants
pub const SIDE_BUY: u8 = 0;
pub const SIDE_SELL: u8 = 1;

// Order type constants
pub const ORDER_TYPE_MARKET: u8 = 0;
pub const ORDER_TYPE_LIMIT: u8 = 1;
pub const ORDER_TYPE_STOP: u8 = 2;
pub const ORDER_TYPE_CANCEL: u8 = 3;

/// Unified Message Header (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MitchHeader {
    pub message_type: u8,   // ASCII message type code
    pub timestamp: [u8; 6], // 48-bit nanoseconds since midnight
    pub count: u8,          // Number of body entries (1-255)
}

/// Body Structures (32 bytes each)

/// Trade (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Trade {
    pub ticker_id: u64,
    pub price: f64,
    pub quantity: u32,
    pub trade_id: u32,
    pub side: u8,           // 0: Buy, 1: Sell
    pub padding: [u8; 7],
}

/// Order (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Order {
    pub ticker_id: u64,
    pub order_id: u32,
    pub price: f64,
    pub quantity: u32,
    pub type_and_side: u8,  // Bit 0: Side, Bits 1-7: Order Type
    pub expiry: [u8; 6],
    pub padding: u8,
}

/// Tick (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Tick {
    pub ticker_id: u64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_volume: u32,
    pub ask_volume: u32,
}

/// OrderBook (Header: 32 bytes)
/// Variable size: 32 bytes header + num_ticks * 4 bytes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrderBook {
    pub ticker_id: u64,
    pub first_tick: f64,
    pub tick_size: f64,
    pub num_ticks: u16,
    pub side: u8,           // 0: Bids, 1: Asks
    pub padding: [u8; 5],
    // volumes: [u32; num_ticks] follows
}

/// Utility Functions

/// Extract the side from a type_and_side field
pub fn extract_side(type_and_side: u8) -> u8 {
    type_and_side & 0x01
}

/// Extract the order type from a type_and_side field
pub fn extract_order_type(type_and_side: u8) -> u8 {
    (type_and_side >> 1) & 0x7F
}

/// Combine order type and side into a single field
pub fn combine_type_and_side(order_type: u8, side: u8) -> u8 {
    (order_type << 1) | side
}
