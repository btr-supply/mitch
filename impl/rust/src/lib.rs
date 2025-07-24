//! MITCH (Moded Individual Trade Clearing and Handling) Protocol
//!
//! A transport-agnostic binary protocol for ultra-low latency market data.
//! See the [protocol overview](https://github.com/btr-trading/mitch/blob/main/model/overview.md) for more details.
//!
//! # Features
//! - `std`: (Default) Enables standard library features.
//! - `no_std`: For embedded and resource-constrained environments.
//! - `networking`: Enables network transports (Redis, WebTransport).
//! - `redis-client`: Enables Redis transport.
//! - `webtransport-client`: Enables WebTransport (HTTP/3) transport.
//! - `all-networking`: Enables all available networking transports.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use mitch::{Trade, OrderSide};
//! use mitch::networking::{MessageTransport, Pushable};
//! use std::sync::Arc;
//!
//! # #[cfg(feature = "redis-client")]
//! # use mitch::networking::redis::RedisTransport;
//! # #[cfg(feature = "webtransport-client")]
//! # use mitch::networking::webtransport::WebTransportClient;
//! #
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # #[cfg(all(feature = "redis-client", feature = "webtransport-client"))]
//! # {
//! // Create transport clients
//! let redis_client = RedisTransport::new("redis://localhost:6379").await?;
//! let wt_client = WebTransportClient::new("https://localhost:4433").await?;
//!
//! // Create a list of transports to push messages to
//! let clients: Vec<Arc<dyn MessageTransport>> = vec![
//!     Arc::new(redis_client),
//!     Arc::new(wt_client),
//! ];
//!
//! // Create a new trade message
//! let trade = Trade::new(12345, 99.95, 1000, 42, OrderSide::Buy)?;
//!
//! // Push the message to all clients concurrently with ultra-low latency
//! trade.push(&clients, None).await?;
//! # }
//!
//! # Ok(())
//! # }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
// #![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![allow(clippy::all)]

// =============================================================================
// MODULE DECLARATIONS
// =============================================================================

/// Ticker ID and channel implementation
pub mod channel;
/// Common types, enums, and constants used across all message types
pub mod common;
/// Generated constants from CSV files.
pub mod constants;
/// MITCH unified message header (8 bytes)
pub mod header;
/// Index message implementation (64 bytes)
pub mod index;
/// Market provider name resolution and fuzzy matching
pub mod market_providers;
/// Networking layer for Redis and WebTransport
#[cfg(feature = "networking")]
#[macro_use]
pub mod networking;
/// Order message implementation (32 bytes)
pub mod order;
/// OrderBook message implementation (2072 bytes)
pub mod order_book;
/// Tick message implementation (32 bytes)
pub mod tick;
/// Ticker and asset resolution with comprehensive parsing and fuzzy matching
pub mod ticker;
/// Trade message implementation (32 bytes)
pub mod trade;
/// Utility functions for message packing, unpacking, and string processing
pub mod utils;

/// High-performance benchmarking utilities
#[cfg(feature = "all-networking")]
pub mod benchmarks;

/// FFI exports for C compatibility
pub mod ffi;


// Re-export public API
pub use crate::common::*;
pub use crate::header::*;
pub use crate::trade::*;
pub use crate::order::*;
pub use crate::tick::*;
pub use crate::index::*;
pub use crate::order_book::*;
pub use crate::ticker::*;
pub use crate::channel::*;
pub use crate::market_providers::{MarketProvider, ProviderMatch, find_market_provider, get_market_provider_by_id, get_market_provider_id_by_name, get_all_market_providers};

// Re-export networking types when feature is enabled
#[cfg(feature = "networking")]
pub use crate::networking::*;

// FFI functions
// TODO: Add FFI functions for packing and unpacking messages

// =============================================================================
// FFI ERROR HANDLING
// =============================================================================

/// FFI-compatible error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MitchErrorCode {
    Success = 0,
    InvalidData = 1,
    InvalidMessageType = 2,
    BufferTooSmall = 3,
    InvalidTickerId = 4,
    InvalidChannelId = 5,
    InvalidFieldValue = 6,
    SerializationError = 7,
    UnknownError = 99,
}

impl From<&MitchError> for MitchErrorCode {
    fn from(error: &MitchError) -> Self {
        match error {
            MitchError::InvalidData(_) => MitchErrorCode::InvalidData,
            MitchError::InvalidMessageType(_) => MitchErrorCode::InvalidMessageType,
            MitchError::BufferTooSmall { .. } => MitchErrorCode::BufferTooSmall,
            MitchError::InvalidTickerId(_) => MitchErrorCode::InvalidTickerId,
            MitchError::InvalidChannelId(_) => MitchErrorCode::InvalidChannelId,
            MitchError::InvalidFieldValue(_) => MitchErrorCode::InvalidFieldValue,
            MitchError::SerializationError(_) => MitchErrorCode::SerializationError,
        }
    }
}

// =============================================================================
// FFI HELPER MACROS
// =============================================================================

macro_rules! ffi_try {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(_) => return MitchErrorCode::UnknownError,
        }
    };
}

// =============================================================================
// MESSAGE TYPE FFI FUNCTIONS
// =============================================================================

/// Pack a Trade message to bytes
/// Returns error code, writes bytes to output buffer
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_pack_trade(
    ticker_id: u64,
    price: f64,
    quantity: u32,
    trade_id: u32,
    side: u8, // 0=Buy, 1=Sell
    output: *mut u8,
) -> MitchErrorCode {
    if output.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let order_side = if side == 0 { OrderSide::Buy } else { OrderSide::Sell };
    let trade = ffi_try!(Trade::new(ticker_id, price, quantity, trade_id, order_side));
    let packed = trade.pack();

    unsafe {
        std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::TRADE);
    }

    MitchErrorCode::Success
}

/// Unpack a Trade message from bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_unpack_trade(
    bytes: *const u8,
    len: usize,
    ticker_id: *mut u64,
    price: *mut f64,
    quantity: *mut u32,
    trade_id: *mut u32,
    side: *mut u8,
) -> MitchErrorCode {
    if bytes.is_null() || ticker_id.is_null() || price.is_null() ||
       quantity.is_null() || trade_id.is_null() || side.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let trade = ffi_try!(Trade::unpack(slice));

    unsafe {
        *ticker_id = trade.ticker_id;
        *price = trade.price;
        *quantity = trade.quantity;
        *trade_id = trade.trade_id;
        *side = trade.side as u8;
    }

    MitchErrorCode::Success
}

/// Pack an Order message to bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_pack_order(
    ticker_id: u64,
    order_id: u32,
    price: f64,
    quantity: u32,
    order_type: u8,
    side: u8,
    expiry_ms: u64,
    output: *mut u8,
) -> MitchErrorCode {
    if output.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let order_type = match order_type {
        0 => OrderType::Market,
        1 => OrderType::Limit,
        2 => OrderType::Stop,
        3 => OrderType::Cancel,
        _ => return MitchErrorCode::InvalidFieldValue,
    };

    let order_side = if side == 0 { OrderSide::Buy } else { OrderSide::Sell };
    let order = ffi_try!(Order::new(ticker_id, order_id, price, quantity, order_type, order_side, expiry_ms));
    let packed = order.pack();

    unsafe {
        std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::ORDER);
    }

    MitchErrorCode::Success
}

/// Unpack an Order message from bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_unpack_order(
    bytes: *const u8,
    len: usize,
    ticker_id: *mut u64,
    order_id: *mut u32,
    price: *mut f64,
    quantity: *mut u32,
    order_type: *mut u8,
    side: *mut u8,
    expiry_ms: *mut u64,
) -> MitchErrorCode {
    if bytes.is_null() || ticker_id.is_null() || order_id.is_null() ||
       price.is_null() || quantity.is_null() || order_type.is_null() ||
       side.is_null() || expiry_ms.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let order = ffi_try!(Order::unpack(slice));

    unsafe {
        *ticker_id = order.ticker_id;
        *order_id = order.order_id;
        *price = order.price;
        *quantity = order.quantity;
        *order_type = order.get_order_type() as u8;
        *side = order.get_order_side() as u8;
        *expiry_ms = order.get_expiry();
    }

    MitchErrorCode::Success
}

/// Pack a Tick message to bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_pack_tick(
    ticker_id: u64,
    bid_price: f64,
    ask_price: f64,
    bid_volume: u32,
    ask_volume: u32,
    output: *mut u8,
) -> MitchErrorCode {
    if output.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let tick = ffi_try!(Tick::new(ticker_id, bid_price, ask_price, bid_volume, ask_volume));
    let packed = tick.pack();

    unsafe {
        std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::TICK);
    }

    MitchErrorCode::Success
}

/// Unpack a Tick message from bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_unpack_tick(
    bytes: *const u8,
    len: usize,
    ticker_id: *mut u64,
    bid_price: *mut f64,
    ask_price: *mut f64,
    bid_volume: *mut u32,
    ask_volume: *mut u32,
) -> MitchErrorCode {
    if bytes.is_null() || ticker_id.is_null() || bid_price.is_null() ||
       ask_price.is_null() || bid_volume.is_null() || ask_volume.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let tick = ffi_try!(Tick::unpack(slice));

    unsafe {
        *ticker_id = tick.ticker_id;
        *bid_price = tick.bid_price;
        *ask_price = tick.ask_price;
        *bid_volume = tick.bid_volume;
        *ask_volume = tick.ask_volume;
    }

    MitchErrorCode::Success
}

/// Pack an Index message to bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_pack_index(
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
    output: *mut u8,
) -> MitchErrorCode {
    if output.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let index = Index::new(
        ticker_id, mid, v_bid, v_ask, m_spread, b_bid_o, b_ask_o,
        w_bid_o, w_ask_o, v_force, l_force, t_force, m_force,
        confidence, rejected, accepted
    );
    let packed = index.pack();

    unsafe {
        std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::INDEX);
    }

    MitchErrorCode::Success
}

/// Unpack an Index message from bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_unpack_index(
    bytes: *const u8,
    len: usize,
    ticker_id: *mut u64,
    mid: *mut f64,
    v_bid: *mut u32,
    v_ask: *mut u32,
    m_spread: *mut i32,
    b_bid_o: *mut i32,
    b_ask_o: *mut i32,
    w_bid_o: *mut i32,
    w_ask_o: *mut i32,
    v_force: *mut u16,
    l_force: *mut u16,
    t_force: *mut i16,
    m_force: *mut i16,
    confidence: *mut u8,
    rejected: *mut u8,
    accepted: *mut u8,
) -> MitchErrorCode {
    if bytes.is_null() || ticker_id.is_null() || mid.is_null() ||
       v_bid.is_null() || v_ask.is_null() || m_spread.is_null() ||
       b_bid_o.is_null() || b_ask_o.is_null() || w_bid_o.is_null() ||
       w_ask_o.is_null() || v_force.is_null() || l_force.is_null() ||
       t_force.is_null() || m_force.is_null() || confidence.is_null() ||
       rejected.is_null() || accepted.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let index = ffi_try!(Index::unpack(slice));

    unsafe {
        *ticker_id = index.ticker_id;
        *mid = index.mid;
        *v_bid = index.v_bid;
        *v_ask = index.v_ask;
        *m_spread = index.m_spread;
        *b_bid_o = index.b_bid_o;
        *b_ask_o = index.b_ask_o;
        *w_bid_o = index.w_bid_o;
        *w_ask_o = index.w_ask_o;
        *v_force = index.v_force;
        *l_force = index.l_force;
        *t_force = index.t_force;
        *m_force = index.m_force;
        *confidence = index.confidence;
        *rejected = index.rejected;
        *accepted = index.accepted;
    }

    MitchErrorCode::Success
}

/// Get message size constants
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_get_message_sizes(
    header: *mut usize,
    trade: *mut usize,
    order: *mut usize,
    tick: *mut usize,
    index: *mut usize,
    order_book: *mut usize,
) -> MitchErrorCode {
    if header.is_null() || trade.is_null() || order.is_null() ||
       tick.is_null() || index.is_null() || order_book.is_null() {
        return MitchErrorCode::InvalidData;
    }

    unsafe {
        *header = message_sizes::HEADER;
        *trade = message_sizes::TRADE;
        *order = message_sizes::ORDER;
        *tick = message_sizes::TICK;
        *index = message_sizes::INDEX;
        *order_book = message_sizes::ORDER_BOOK;
    }

    MitchErrorCode::Success
}

// =============================================================================
// HEADER FFI FUNCTIONS
// =============================================================================

/// Create and pack a MITCH header
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_pack_header(
    message_type: u8,
    timestamp: u64,
    count: u8,
    output: *mut u8,
) -> MitchErrorCode {
    if output.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let header = ffi_try!(MitchHeader::new_validated(message_type, timestamp, count));
    let packed = header.pack();

    unsafe {
        std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::HEADER);
    }

    MitchErrorCode::Success
}

/// Unpack a MITCH header
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_unpack_header(
    bytes: *const u8,
    len: usize,
    message_type: *mut u8,
    timestamp: *mut u64,
    count: *mut u8,
) -> MitchErrorCode {
    if bytes.is_null() || message_type.is_null() ||
       timestamp.is_null() || count.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let header = ffi_try!(MitchHeader::unpack(slice));

    unsafe {
        *message_type = header.message_type;
        *timestamp = header.get_timestamp();
        *count = header.count;
    }

    MitchErrorCode::Success
}

// =============================================================================
// ASSET RESOLUTION FFI FUNCTIONS
// =============================================================================

/// Resolve asset by name with confidence threshold
/// Returns asset information via output parameters
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_resolve_asset(
    name: *const std::os::raw::c_char,
    min_confidence: f64,
    asset_id: *mut u32,
    class_id: *mut u16,
    asset_class: *mut u8,
    name_out: *mut std::os::raw::c_char,
    name_len: usize,
    aliases_out: *mut std::os::raw::c_char,
    aliases_len: usize,
    confidence: *mut f64,
) -> MitchErrorCode {
    if name.is_null() || asset_id.is_null() || class_id.is_null() ||
       asset_class.is_null() || name_out.is_null() || aliases_out.is_null() ||
       confidence.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let name_str = unsafe {
        match std::ffi::CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return MitchErrorCode::InvalidData,
        }
    };

    let asset_match = match resolve_asset(name_str, min_confidence) {
        Some(asset_match) => asset_match,
        None => return MitchErrorCode::InvalidData,
    };

    unsafe {
        *asset_id = asset_match.asset.id;
        *class_id = asset_match.asset.class_id;
        *asset_class = asset_match.asset.class as u8;
        *confidence = asset_match.confidence;

        // Copy name string
        let name_bytes = asset_match.asset.name.as_bytes();
        let copy_len = std::cmp::min(name_bytes.len(), name_len - 1);
        std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
        *((name_out as *mut u8).add(copy_len)) = 0; // null terminate

        // Copy aliases string
        let aliases_bytes = asset_match.asset.aliases.as_bytes();
        let copy_len = std::cmp::min(aliases_bytes.len(), aliases_len - 1);
        std::ptr::copy_nonoverlapping(aliases_bytes.as_ptr(), aliases_out as *mut u8, copy_len);
        *((aliases_out as *mut u8).add(copy_len)) = 0; // null terminate
    }

    MitchErrorCode::Success
}

/// Get asset by ID
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_get_asset_by_id(
    asset_class: u8,
    class_id: u16,
    asset_id: *mut u32,
    name_out: *mut std::os::raw::c_char,
    name_len: usize,
    aliases_out: *mut std::os::raw::c_char,
    aliases_len: usize,
) -> MitchErrorCode {
    if asset_id.is_null() || name_out.is_null() || aliases_out.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let asset_class_enum = AssetClass::from_id(asset_class);
    let asset = match get_asset_by_id(asset_class_enum, class_id) {
        Some(asset) => asset,
        None => return MitchErrorCode::InvalidData,
    };

    unsafe {
        *asset_id = asset.id;

        // Copy name string
        let name_bytes = asset.name.as_bytes();
        let copy_len = std::cmp::min(name_bytes.len(), name_len - 1);
        std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
        *((name_out as *mut u8).add(copy_len)) = 0; // null terminate

        // Copy aliases string
        let aliases_bytes = asset.aliases.as_bytes();
        let copy_len = std::cmp::min(aliases_bytes.len(), aliases_len - 1);
        std::ptr::copy_nonoverlapping(aliases_bytes.as_ptr(), aliases_out as *mut u8, copy_len);
        *((aliases_out as *mut u8).add(copy_len)) = 0; // null terminate
    }

    MitchErrorCode::Success
}

// =============================================================================
// TICKER RESOLUTION FFI FUNCTIONS
// =============================================================================

/// Resolve ticker symbol to ticker ID and components
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_resolve_ticker(
    symbol: *const std::os::raw::c_char,
    instrument_type: u8,
    ticker_id: *mut u64,
    base_asset_id: *mut u32,
    quote_asset_id: *mut u32,
    confidence: *mut f64,
) -> MitchErrorCode {
    if symbol.is_null() || ticker_id.is_null() || base_asset_id.is_null() ||
       quote_asset_id.is_null() || confidence.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let symbol_str = unsafe {
        match std::ffi::CStr::from_ptr(symbol).to_str() {
            Ok(s) => s,
            Err(_) => return MitchErrorCode::InvalidData,
        }
    };

    let instrument_type_enum = InstrumentType::from_id(instrument_type);

    let ticker_match = match resolve_ticker(symbol_str, instrument_type_enum) {
        Ok(ticker_match) => ticker_match,
        Err(_) => return MitchErrorCode::InvalidData,
    };

    unsafe {
        *ticker_id = ticker_match.ticker.id;
        *base_asset_id = ticker_match.ticker.base.id;
        *quote_asset_id = ticker_match.ticker.quote.id;
        *confidence = ticker_match.confidence;
    }

    MitchErrorCode::Success
}

/// Create ticker ID from components
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_create_ticker_id(
    instrument_type: u8,
    base_class: u8,
    base_id: u16,
    quote_class: u8,
    quote_id: u16,
    sub_type: u32,
    ticker_id: *mut u64,
) -> MitchErrorCode {
    if ticker_id.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let instrument_type_enum = InstrumentType::from_id(instrument_type);
    let base_class_enum = AssetClass::from_id(base_class);
    let quote_class_enum = AssetClass::from_id(quote_class);

    let ticker_id_obj = ffi_try!(TickerId::new(
        instrument_type_enum,
        base_class_enum,
        base_id,
        quote_class_enum,
        quote_id,
        sub_type,
    ));

    unsafe {
        *ticker_id = ticker_id_obj.raw;
    }

    MitchErrorCode::Success
}

/// Decode ticker ID into components
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_decode_ticker_id(
    ticker_id: u64,
    instrument_type: *mut u8,
    base_class: *mut u8,
    base_id: *mut u16,
    quote_class: *mut u8,
    quote_id: *mut u16,
    sub_type: *mut u32,
) -> MitchErrorCode {
    if instrument_type.is_null() || base_class.is_null() || base_id.is_null() ||
       quote_class.is_null() || quote_id.is_null() || sub_type.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let ticker_id_obj = TickerId::from_raw(ticker_id);

    unsafe {
        *instrument_type = ticker_id_obj.instrument_type() as u8;
        *base_class = ticker_id_obj.base_asset_class() as u8;
        *base_id = ticker_id_obj.base_asset_id();
        *quote_class = ticker_id_obj.quote_asset_class() as u8;
        *quote_id = ticker_id_obj.quote_asset_id();
        *sub_type = ticker_id_obj.sub_type();
    }

    MitchErrorCode::Success
}

// =============================================================================
// MARKET PROVIDER FFI FUNCTIONS
// =============================================================================

/// Find market provider by name
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_find_market_provider(
    name: *const std::os::raw::c_char,
    min_confidence: f64,
    provider_id: *mut u16,
    name_out: *mut std::os::raw::c_char,
    name_len: usize,
    confidence: *mut f64,
) -> MitchErrorCode {
    if name.is_null() || provider_id.is_null() || name_out.is_null() || confidence.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let name_str = unsafe {
        match std::ffi::CStr::from_ptr(name).to_str() {
            Ok(s) => s,
            Err(_) => return MitchErrorCode::InvalidData,
        }
    };

    let provider_match = match find_market_provider(name_str, min_confidence) {
        Some(provider_match) => provider_match,
        None => return MitchErrorCode::InvalidData,
    };

    unsafe {
        *provider_id = provider_match.provider.id;
        *confidence = provider_match.confidence;

        // Copy name string
        let name_bytes = provider_match.provider.name.as_bytes();
        let copy_len = std::cmp::min(name_bytes.len(), name_len - 1);
        std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
        *((name_out as *mut u8).add(copy_len)) = 0; // null terminate
    }

    MitchErrorCode::Success
}

/// Get market provider by ID
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_get_market_provider_by_id(
    provider_id: u16,
    name_out: *mut std::os::raw::c_char,
    name_len: usize,
) -> MitchErrorCode {
    if name_out.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let provider = match get_market_provider_by_id(provider_id) {
        Some(provider) => provider,
        None => return MitchErrorCode::InvalidData,
    };

    unsafe {
        // Copy name string
        let name_bytes = provider.name.as_bytes();
        let copy_len = std::cmp::min(name_bytes.len(), name_len - 1);
        std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
        *((name_out as *mut u8).add(copy_len)) = 0; // null terminate
    }

    MitchErrorCode::Success
}

// =============================================================================
// CHANNEL FFI FUNCTIONS
// =============================================================================

/// Create channel ID from provider and message type
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_create_channel(
    provider_id: u16,
    msg_type: std::os::raw::c_char,
    channel_id: *mut u32,
) -> MitchErrorCode {
    if channel_id.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let channel = ChannelId::new(provider_id, msg_type as u8 as char);

    unsafe {
        *channel_id = channel.raw;
    }

    MitchErrorCode::Success
}

/// Pack channel ID to bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_pack_channel(
    channel_id: u32,
    output: *mut u8,
) -> MitchErrorCode {
    if output.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let channel = ChannelId { raw: channel_id };
    let packed = channel.pack();

    unsafe {
        std::ptr::copy_nonoverlapping(packed.as_ptr(), output, 4);
    }

    MitchErrorCode::Success
}

/// Unpack channel ID from bytes
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_unpack_channel(
    bytes: *const u8,
    len: usize,
    channel_id: *mut u32,
) -> MitchErrorCode {
    if bytes.is_null() || channel_id.is_null() {
        return MitchErrorCode::InvalidData;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    let channel = ffi_try!(ChannelId::unpack(slice));

    unsafe {
        *channel_id = channel.raw;
    }

    MitchErrorCode::Success
}

// =============================================================================
// LIBRARY VERSION AND METADATA
// =============================================================================

/// MITCH protocol version implemented by this crate
pub const MITCH_VERSION: &str = "1.0.0";

/// Library version
pub const LIB_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build information
pub const BUILD_INFO: &str = concat!(
    "mitch-rust v", env!("CARGO_PKG_VERSION"),
    " (MITCH protocol v1.0.0)"
);

// =============================================================================
// FFI EXPORTS (for .so/.dll usage)
// =============================================================================

/// Get library version for FFI consumers
///
/// # Safety
/// This function is safe to call from FFI
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_get_version() -> *const u8 {
    LIB_VERSION.as_ptr()
}

/// Get library version length for FFI consumers
///
/// # Safety
/// This function is safe to call from FFI
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_get_version_len() -> usize {
    LIB_VERSION.len()
}

/// Get MITCH protocol version for FFI consumers
///
/// # Safety
/// This function is safe to call from FFI
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_get_protocol_version() -> *const u8 {
    MITCH_VERSION.as_ptr()
}

/// Get MITCH protocol version length for FFI consumers
///
/// # Safety
/// This function is safe to call from FFI
#[cfg(not(feature = "ffi"))]
#[no_mangle]
pub extern "C" fn mitch_get_protocol_version_len() -> usize {
    MITCH_VERSION.len()
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Calculate message size for given type and count
///
/// # Arguments
/// * `message_type` - ASCII message type code
/// * `count` - Number of message bodies
///
/// # Returns
/// Total message size in bytes (header + bodies) or error
///
/// # Example
/// ```rust
/// use mitch::*;
///
/// // Size for 10 trade messages
/// let size = calculate_message_size(message_type::TRADE, 10).unwrap();
/// assert_eq!(size, 8 + (10 * 32)); // 8-byte header + 10 * 32-byte trades
/// ```
pub fn calculate_message_size(message_type: u8, count: u8) -> Result<usize, MitchError> {
    validate_message_type(message_type)?;

    let single_body_size = match message_type {
        message_type::TRADE => message_sizes::TRADE,
        message_type::ORDER => message_sizes::ORDER,
        message_type::TICK => message_sizes::TICK,
        message_type::INDEX => message_sizes::INDEX,
        message_type::ORDER_BOOK => message_sizes::ORDER_BOOK,
        _ => return Err(MitchError::InvalidMessageType(message_type)),
    };

    Ok(message_sizes::HEADER + (count as usize * single_body_size))
}

/// Validate that a buffer contains a valid MITCH message
///
/// # Arguments
/// * `bytes` - Buffer to validate
///
/// # Returns
/// Result containing message type and count, or error
///
/// # Example
/// ```rust
/// use mitch::*;
///
/// let trade = Trade::new(0x1, 100.0, 1000, 1, OrderSide::Buy).unwrap();
/// let header = MitchHeader::new(message_type::TRADE, 123456, 1);
///
/// let mut buffer = Vec::new();
/// buffer.extend_from_slice(&header.pack());
/// buffer.extend_from_slice(&trade.pack());
///
/// let (msg_type, count) = validate_message_buffer(&buffer).unwrap();
/// assert_eq!(msg_type, message_type::TRADE);
/// assert_eq!(count, 1);
/// ```
pub fn validate_message_buffer(bytes: &[u8]) -> Result<(u8, u8), MitchError> {
    if bytes.len() < message_sizes::HEADER {
        return Err(MitchError::BufferTooSmall {
            expected: message_sizes::HEADER,
            actual: bytes.len(),
        });
    }

    let header = MitchHeader::unpack(bytes)?;
    let expected_size = calculate_message_size(header.message_type, header.count)?;

    if bytes.len() < expected_size {
        return Err(MitchError::BufferTooSmall {
            expected: expected_size,
            actual: bytes.len(),
        });
    }

    Ok((header.message_type, header.count))
}
