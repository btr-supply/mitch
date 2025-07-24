//! MetaTrader 4 FFI bindings for MITCH protocol
//! 
//! Provides C-compatible functions for MT4 integration including:
//! - Asset and ticker resolution
//! - Message encoding/decoding
//! - Redis networking
//! - Market provider lookup

use crate::*;
use std::os::raw::{c_int, c_uchar, c_char, c_double, c_uint, c_ulonglong};
use std::ffi::CStr;
use std::sync::Mutex;
use std::slice;

#[cfg(feature = "redis-client")]
use ::redis::{Client, Connection, cmd};

#[cfg(feature = "redis-client")]
use lazy_static::lazy_static;

#[cfg(feature = "redis-client")]
lazy_static! {
    static ref REDIS_CONNECTION: Mutex<Option<Connection>> = Mutex::new(None);
}

// Windows-specific unwind stubs for 32-bit compatibility
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[no_mangle]
pub extern "C" fn _Unwind_RaiseException() -> u32 {
    0
}

// =============================================================================
// ASSET RESOLUTION FUNCTIONS
// =============================================================================

/// Resolve asset by name with confidence threshold
/// 
/// # Safety
/// All pointer parameters must be valid and properly allocated
#[no_mangle]
pub unsafe extern "C" fn mitch_resolve_asset(
    name: *const c_char,
    min_confidence: c_double,
    asset_id: *mut c_uint,
    class_id: *mut c_uint,
    asset_class: *mut c_uchar,
    name_out: *mut c_char,
    name_len: c_int,
    aliases_out: *mut c_char,
    aliases_len: c_int,
    confidence: *mut c_double,
) -> c_int {
    if name.is_null() || asset_id.is_null() || class_id.is_null() ||
       asset_class.is_null() || name_out.is_null() || aliases_out.is_null() ||
       confidence.is_null() {
        return 0;
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match resolve_asset(name_str, min_confidence) {
        Some(asset_match) => {
            *asset_id = asset_match.asset.id;
            *class_id = asset_match.asset.class_id as c_uint;
            *asset_class = asset_match.asset.class as c_uchar;
            *confidence = asset_match.confidence;

            // Copy name string
            let name_bytes = asset_match.asset.name.as_bytes();
            let copy_len = std::cmp::min(name_bytes.len(), (name_len - 1) as usize);
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
            *((name_out as *mut u8).add(copy_len)) = 0;

            // Copy aliases string
            let aliases_bytes = asset_match.asset.aliases.as_bytes();
            let copy_len = std::cmp::min(aliases_bytes.len(), (aliases_len - 1) as usize);
            std::ptr::copy_nonoverlapping(aliases_bytes.as_ptr(), aliases_out as *mut u8, copy_len);
            *((aliases_out as *mut u8).add(copy_len)) = 0;

            1
        }
        None => 0,
    }
}

/// Get asset by ID
/// 
/// # Safety
/// All pointer parameters must be valid and properly allocated
#[no_mangle]
pub unsafe extern "C" fn mitch_get_asset_by_id(
    asset_class: c_uchar,
    class_id: c_uint,
    asset_id: *mut c_uint,
    name_out: *mut c_char,
    name_len: c_int,
    aliases_out: *mut c_char,
    aliases_len: c_int,
) -> c_int {
    if asset_id.is_null() || name_out.is_null() || aliases_out.is_null() {
        return 0;
    }

    let asset_class_enum = AssetClass::from_id(asset_class);
    match get_asset_by_id(asset_class_enum, class_id as u16) {
        Some(asset) => {
            *asset_id = asset.id;

            // Copy name string
            let name_bytes = asset.name.as_bytes();
            let copy_len = std::cmp::min(name_bytes.len(), (name_len - 1) as usize);
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
            *((name_out as *mut u8).add(copy_len)) = 0;

            // Copy aliases string
            let aliases_bytes = asset.aliases.as_bytes();
            let copy_len = std::cmp::min(aliases_bytes.len(), (aliases_len - 1) as usize);
            std::ptr::copy_nonoverlapping(aliases_bytes.as_ptr(), aliases_out as *mut u8, copy_len);
            *((aliases_out as *mut u8).add(copy_len)) = 0;

            1
        }
        None => 0,
    }
}

// =============================================================================
// TICKER RESOLUTION FUNCTIONS
// =============================================================================

/// Resolve ticker symbol to ticker ID and components
/// 
/// # Safety
/// All pointer parameters must be valid and properly allocated
#[no_mangle]
pub unsafe extern "C" fn mitch_resolve_ticker(
    symbol: *const c_char,
    instrument_type: c_uchar,
    ticker_id: *mut c_ulonglong,
    base_asset_id: *mut c_uint,
    quote_asset_id: *mut c_uint,
    confidence: *mut c_double,
) -> c_int {
    if symbol.is_null() || ticker_id.is_null() || base_asset_id.is_null() ||
       quote_asset_id.is_null() || confidence.is_null() {
        return 0;
    }

    let symbol_str = match CStr::from_ptr(symbol).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let instrument_type_enum = InstrumentType::from_id(instrument_type);

    match resolve_ticker(symbol_str, instrument_type_enum) {
        Ok(ticker_match) => {
            *ticker_id = ticker_match.ticker.id;
            *base_asset_id = ticker_match.ticker.base.id;
            *quote_asset_id = ticker_match.ticker.quote.id;
            *confidence = ticker_match.confidence;
            1
        }
        Err(_) => 0,
    }
}

/// Create ticker ID from components
/// 
/// # Safety
/// ticker_id pointer must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_create_ticker_id(
    instrument_type: c_uchar,
    base_class: c_uchar,
    base_id: c_uint,
    quote_class: c_uchar,
    quote_id: c_uint,
    sub_type: c_uint,
    ticker_id: *mut c_ulonglong,
) -> c_int {
    if ticker_id.is_null() {
        return 0;
    }

    let instrument_type_enum = InstrumentType::from_id(instrument_type);
    let base_class_enum = AssetClass::from_id(base_class);
    let quote_class_enum = AssetClass::from_id(quote_class);

    match TickerId::new(
        instrument_type_enum,
        base_class_enum,
        base_id as u16,
        quote_class_enum,
        quote_id as u16,
        sub_type,
    ) {
        Ok(ticker_id_obj) => {
            *ticker_id = ticker_id_obj.raw;
            1
        }
        Err(_) => 0,
    }
}

/// Decode ticker ID into components
/// 
/// # Safety
/// All pointer parameters must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_decode_ticker_id(
    ticker_id: c_ulonglong,
    instrument_type: *mut c_uchar,
    base_class: *mut c_uchar,
    base_id: *mut c_uint,
    quote_class: *mut c_uchar,
    quote_id: *mut c_uint,
    sub_type: *mut c_uint,
) -> c_int {
    if instrument_type.is_null() || base_class.is_null() || base_id.is_null() ||
       quote_class.is_null() || quote_id.is_null() || sub_type.is_null() {
        return 0;
    }

    let ticker_id_obj = TickerId::from_raw(ticker_id);

    *instrument_type = ticker_id_obj.instrument_type() as c_uchar;
    *base_class = ticker_id_obj.base_asset_class() as c_uchar;
    *base_id = ticker_id_obj.base_asset_id() as c_uint;
    *quote_class = ticker_id_obj.quote_asset_class() as c_uchar;
    *quote_id = ticker_id_obj.quote_asset_id() as c_uint;
    *sub_type = ticker_id_obj.sub_type();

    1
}

// =============================================================================
// MESSAGE ENCODING/DECODING FUNCTIONS
// =============================================================================

/// Pack a tick message
/// 
/// # Safety
/// output must point to a buffer of at least 32 bytes
#[no_mangle]
pub unsafe extern "C" fn mitch_pack_tick(
    ticker_id: c_ulonglong,
    bid_price: c_double,
    ask_price: c_double,
    bid_volume: c_uint,
    ask_volume: c_uint,
    output: *mut c_uchar,
) -> c_int {
    if output.is_null() {
        return 0;
    }

    match Tick::new(ticker_id, bid_price, ask_price, bid_volume, ask_volume) {
        Ok(tick) => {
            let packed = tick.pack();
            std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::TICK);
            message_sizes::TICK as c_int
        }
        Err(_) => 0,
    }
}

/// Unpack a tick message
/// 
/// # Safety
/// All pointer parameters must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_unpack_tick(
    bytes: *const c_uchar,
    len: c_int,
    ticker_id: *mut c_ulonglong,
    bid_price: *mut c_double,
    ask_price: *mut c_double,
    bid_volume: *mut c_uint,
    ask_volume: *mut c_uint,
) -> c_int {
    if bytes.is_null() || ticker_id.is_null() || bid_price.is_null() ||
       ask_price.is_null() || bid_volume.is_null() || ask_volume.is_null() {
        return 0;
    }

    let slice = std::slice::from_raw_parts(bytes, len as usize);
    match Tick::unpack(slice) {
        Ok(tick) => {
            *ticker_id = tick.ticker_id;
            *bid_price = tick.bid_price;
            *ask_price = tick.ask_price;
            *bid_volume = tick.bid_volume;
            *ask_volume = tick.ask_volume;
            1
        }
        Err(_) => 0,
    }
}

/// Pack a trade message
/// 
/// # Safety
/// output must point to a buffer of at least 32 bytes
#[no_mangle]
pub unsafe extern "C" fn mitch_pack_trade(
    ticker_id: c_ulonglong,
    price: c_double,
    quantity: c_uint,
    trade_id: c_uint,
    side: c_uchar,
    output: *mut c_uchar,
) -> c_int {
    if output.is_null() {
        return 0;
    }

    let order_side = if side == 0 { OrderSide::Buy } else { OrderSide::Sell };
    match Trade::new(ticker_id, price, quantity, trade_id, order_side) {
        Ok(trade) => {
            let packed = trade.pack();
            std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::TRADE);
            message_sizes::TRADE as c_int
        }
        Err(_) => 0,
    }
}

/// Unpack a trade message
/// 
/// # Safety
/// All pointer parameters must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_unpack_trade(
    bytes: *const c_uchar,
    len: c_int,
    ticker_id: *mut c_ulonglong,
    price: *mut c_double,
    quantity: *mut c_uint,
    trade_id: *mut c_uint,
    side: *mut c_uchar,
) -> c_int {
    if bytes.is_null() || ticker_id.is_null() || price.is_null() ||
       quantity.is_null() || trade_id.is_null() || side.is_null() {
        return 0;
    }

    let slice = std::slice::from_raw_parts(bytes, len as usize);
    match Trade::unpack(slice) {
        Ok(trade) => {
            *ticker_id = trade.ticker_id;
            *price = trade.price;
            *quantity = trade.quantity;
            *trade_id = trade.trade_id;
            *side = trade.side as c_uchar;
            1
        }
        Err(_) => 0,
    }
}

/// Pack an order message
/// 
/// # Safety
/// output must point to a buffer of at least 32 bytes
#[no_mangle]
pub unsafe extern "C" fn mitch_pack_order(
    ticker_id: c_ulonglong,
    order_id: c_uint,
    price: c_double,
    quantity: c_uint,
    order_type: c_uchar,
    side: c_uchar,
    expiry_ms: c_ulonglong,
    output: *mut c_uchar,
) -> c_int {
    if output.is_null() {
        return 0;
    }

    let order_type_enum = match order_type {
        0 => OrderType::Market,
        1 => OrderType::Limit,
        2 => OrderType::Stop,
        3 => OrderType::Cancel,
        _ => return 0,
    };

    let order_side = if side == 0 { OrderSide::Buy } else { OrderSide::Sell };
    match Order::new(ticker_id, order_id, price, quantity, order_type_enum, order_side, expiry_ms) {
        Ok(order) => {
            let packed = order.pack();
            std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::ORDER);
            message_sizes::ORDER as c_int
        }
        Err(_) => 0,
    }
}

/// Unpack an order message
/// 
/// # Safety
/// All pointer parameters must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_unpack_order(
    bytes: *const c_uchar,
    len: c_int,
    ticker_id: *mut c_ulonglong,
    order_id: *mut c_uint,
    price: *mut c_double,
    quantity: *mut c_uint,
    order_type: *mut c_uchar,
    side: *mut c_uchar,
    expiry_ms: *mut c_ulonglong,
) -> c_int {
    if bytes.is_null() || ticker_id.is_null() || order_id.is_null() ||
       price.is_null() || quantity.is_null() || order_type.is_null() ||
       side.is_null() || expiry_ms.is_null() {
        return 0;
    }

    let slice = std::slice::from_raw_parts(bytes, len as usize);
    match Order::unpack(slice) {
        Ok(order) => {
            *ticker_id = order.ticker_id;
            *order_id = order.order_id;
            *price = order.price;
            *quantity = order.quantity;
            *order_type = order.get_order_type() as c_uchar;
            *side = order.get_order_side() as c_uchar;
            *expiry_ms = order.get_expiry();
            1
        }
        Err(_) => 0,
    }
}

// =============================================================================
// HEADER FUNCTIONS
// =============================================================================

/// Pack a MITCH header
/// 
/// # Safety
/// output must point to a buffer of at least 8 bytes
#[no_mangle]
pub unsafe extern "C" fn mitch_pack_header(
    message_type: c_uchar,
    timestamp: c_ulonglong,
    count: c_uchar,
    output: *mut c_uchar,
) -> c_int {
    if output.is_null() {
        return 0;
    }

    match MitchHeader::new_validated(message_type, timestamp, count) {
        Ok(header) => {
            let packed = header.pack();
            std::ptr::copy_nonoverlapping(packed.as_ptr(), output, message_sizes::HEADER);
            message_sizes::HEADER as c_int
        }
        Err(_) => 0,
    }
}

/// Unpack a MITCH header
/// 
/// # Safety
/// All pointer parameters must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_unpack_header(
    bytes: *const c_uchar,
    len: c_int,
    message_type: *mut c_uchar,
    timestamp: *mut c_ulonglong,
    count: *mut c_uchar,
) -> c_int {
    if bytes.is_null() || message_type.is_null() ||
       timestamp.is_null() || count.is_null() {
        return 0;
    }

    let slice = std::slice::from_raw_parts(bytes, len as usize);
    match MitchHeader::unpack(slice) {
        Ok(header) => {
            *message_type = header.message_type;
            *timestamp = header.get_timestamp();
            *count = header.count;
            1
        }
        Err(_) => 0,
    }
}

// =============================================================================
// COMPLETE MESSAGE WITH HEADER FUNCTIONS
// =============================================================================

/// Pack a complete tick message with header
/// 
/// # Safety
/// output must point to a buffer of at least 40 bytes (8 header + 32 body)
#[no_mangle]
pub unsafe extern "C" fn mitch_pack_tick_message(
    ticker_id: c_ulonglong,
    bid_price: c_double,
    ask_price: c_double,
    bid_volume: c_uint,
    ask_volume: c_uint,
    output: *mut c_uchar,
) -> c_int {
    if output.is_null() {
        return 0;
    }

    match Tick::new(ticker_id, bid_price, ask_price, bid_volume, ask_volume) {
        Ok(tick) => {
            // Create header
            let header = MitchHeader::new(message_type::TICK, 0, 1);
            let header_bytes = header.pack();
            
            // Pack tick body
            let tick_bytes = tick.pack();
            
            // Copy header and body to output
            std::ptr::copy_nonoverlapping(header_bytes.as_ptr(), output, message_sizes::HEADER);
            std::ptr::copy_nonoverlapping(tick_bytes.as_ptr(), output.add(message_sizes::HEADER), message_sizes::TICK);
            
            (message_sizes::HEADER + message_sizes::TICK) as c_int
        }
        Err(_) => 0,
    }
}

/// Pack a complete trade message with header
/// 
/// # Safety
/// output must point to a buffer of at least 40 bytes (8 header + 32 body)
#[no_mangle]
pub unsafe extern "C" fn mitch_pack_trade_message(
    ticker_id: c_ulonglong,
    price: c_double,
    quantity: c_uint,
    trade_id: c_uint,
    side: c_uchar,
    output: *mut c_uchar,
) -> c_int {
    if output.is_null() {
        return 0;
    }

    let order_side = if side == 0 { OrderSide::Buy } else { OrderSide::Sell };
    match Trade::new(ticker_id, price, quantity, trade_id, order_side) {
        Ok(trade) => {
            // Create header
            let header = MitchHeader::new(message_type::TRADE, 0, 1);
            let header_bytes = header.pack();
            
            // Pack trade body
            let trade_bytes = trade.pack();
            
            // Copy header and body to output
            std::ptr::copy_nonoverlapping(header_bytes.as_ptr(), output, message_sizes::HEADER);
            std::ptr::copy_nonoverlapping(trade_bytes.as_ptr(), output.add(message_sizes::HEADER), message_sizes::TRADE);
            
            (message_sizes::HEADER + message_sizes::TRADE) as c_int
        }
        Err(_) => 0,
    }
}

// =============================================================================
// MARKET PROVIDER FUNCTIONS
// =============================================================================

/// Find market provider by name
/// 
/// # Safety
/// All pointer parameters must be valid and properly allocated
#[no_mangle]
pub unsafe extern "C" fn mitch_find_market_provider(
    name: *const c_char,
    min_confidence: c_double,
    provider_id: *mut c_uint,
    name_out: *mut c_char,
    name_len: c_int,
    confidence: *mut c_double,
) -> c_int {
    if name.is_null() || provider_id.is_null() || name_out.is_null() || confidence.is_null() {
        return 0;
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match find_market_provider(name_str, min_confidence) {
        Some(provider_match) => {
            *provider_id = provider_match.provider.id as c_uint;
            *confidence = provider_match.confidence;

            // Copy name string
            let name_bytes = provider_match.provider.name.as_bytes();
            let copy_len = std::cmp::min(name_bytes.len(), (name_len - 1) as usize);
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
            *((name_out as *mut u8).add(copy_len)) = 0;

            1
        }
        None => 0,
    }
}

/// Get market provider by ID
/// 
/// # Safety
/// All pointer parameters must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_get_market_provider_by_id(
    provider_id: c_uint,
    name_out: *mut c_char,
    name_len: c_int,
) -> c_int {
    if name_out.is_null() {
        return 0;
    }

    match get_market_provider_by_id(provider_id as u16) {
        Some(provider) => {
            // Copy name string
            let name_bytes = provider.name.as_bytes();
            let copy_len = std::cmp::min(name_bytes.len(), (name_len - 1) as usize);
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_out as *mut u8, copy_len);
            *((name_out as *mut u8).add(copy_len)) = 0;
            1
        }
        None => 0,
    }
}

// =============================================================================
// REDIS CONNECTION FUNCTIONS
// =============================================================================

#[cfg(feature = "redis-client")]
/// Connect to Redis server
/// 
/// # Safety
/// url_bytes must be a valid UTF-8 string
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_connect(url_bytes: *const c_uchar, url_len: c_int) -> c_int {
    if url_bytes.is_null() || url_len <= 0 {
        return 0;
    }
    
    let url_vec = slice::from_raw_parts(url_bytes, url_len as usize).to_vec();
    let url_str = match std::str::from_utf8(&url_vec) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    
    match Client::open(url_str) {
        Ok(client) => {
            match client.get_connection() {
                Ok(conn) => {
                    if let Ok(mut redis_conn) = REDIS_CONNECTION.lock() {
                        *redis_conn = Some(conn);
                        1
                    } else {
                        0
                    }
                }
                Err(_) => 0,
            }
        }
        Err(_) => 0,
    }
}

#[cfg(feature = "redis-client")]
/// Disconnect from Redis
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_disconnect() -> c_int {
    if let Ok(mut conn) = REDIS_CONNECTION.lock() {
        if conn.is_some() {
            *conn = None;
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[cfg(feature = "redis-client")]
/// Check if connected to Redis
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_is_connected() -> c_int {
    if let Ok(conn) = REDIS_CONNECTION.lock() {
        if conn.is_some() {
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[cfg(feature = "redis-client")]
/// Publish tick message to Redis
/// 
/// # Safety
/// Must be connected to Redis
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_publish_tick(
    channel_id: c_uint,
    ticker_id: c_ulonglong,
    bid_price: c_double,
    ask_price: c_double,
    bid_volume: c_uint,
    ask_volume: c_uint,
) -> c_int {
    if let Ok(mut conn) = REDIS_CONNECTION.lock() {
        if let Some(ref mut connection) = *conn {
            match Tick::new(ticker_id, bid_price, ask_price, bid_volume, ask_volume) {
                Ok(tick) => {
                    // Create full message with header
                    let header = MitchHeader::new(message_type::TICK, 0, 1);
                    let mut message = Vec::new();
                    message.extend_from_slice(&header.pack());
                    message.extend_from_slice(&tick.pack());
                    
                    // Publish to channel
                    let channel_bytes = channel_id.to_le_bytes();
                    match cmd("PUBLISH").arg(&channel_bytes[..]).arg(&message).query::<i64>(connection) {
                        Ok(_) => 1,
                        Err(_) => 0,
                    }
                }
                Err(_) => 0,
            }
        } else {
            0
        }
    } else {
        0
    }
}

#[cfg(feature = "redis-client")]
/// Publish trade message to Redis
/// 
/// # Safety
/// Must be connected to Redis
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_publish_trade(
    channel_id: c_uint,
    ticker_id: c_ulonglong,
    price: c_double,
    quantity: c_uint,
    trade_id: c_uint,
    side: c_uchar,
) -> c_int {
    if let Ok(mut conn) = REDIS_CONNECTION.lock() {
        if let Some(ref mut connection) = *conn {
            let order_side = if side == 0 { OrderSide::Buy } else { OrderSide::Sell };
            match Trade::new(ticker_id, price, quantity, trade_id, order_side) {
                Ok(trade) => {
                    // Create full message with header
                    let header = MitchHeader::new(message_type::TRADE, 0, 1);
                    let mut message = Vec::new();
                    message.extend_from_slice(&header.pack());
                    message.extend_from_slice(&trade.pack());
                    
                    // Publish to channel
                    let channel_bytes = channel_id.to_le_bytes();
                    match cmd("PUBLISH").arg(&channel_bytes[..]).arg(&message).query::<i64>(connection) {
                        Ok(_) => 1,
                        Err(_) => 0,
                    }
                }
                Err(_) => 0,
            }
        } else {
            0
        }
    } else {
        0
    }
}

// Stub functions when Redis is not enabled
#[cfg(not(feature = "redis-client"))]
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_connect(_url_bytes: *const c_uchar, _url_len: c_int) -> c_int { 0 }

#[cfg(not(feature = "redis-client"))]
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_disconnect() -> c_int { 0 }

#[cfg(not(feature = "redis-client"))]
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_is_connected() -> c_int { 0 }

#[cfg(not(feature = "redis-client"))]
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_publish_tick(
    _channel_id: c_uint,
    _ticker_id: c_ulonglong,
    _bid_price: c_double,
    _ask_price: c_double,
    _bid_volume: c_uint,
    _ask_volume: c_uint,
) -> c_int { 0 }

#[cfg(not(feature = "redis-client"))]
#[no_mangle]
pub unsafe extern "C" fn mitch_redis_publish_trade(
    _channel_id: c_uint,
    _ticker_id: c_ulonglong,
    _price: c_double,
    _quantity: c_uint,
    _trade_id: c_uint,
    _side: c_uchar,
) -> c_int { 0 }

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get message sizes
/// 
/// # Safety
/// All pointer parameters must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_get_message_sizes(
    header: *mut c_int,
    trade: *mut c_int,
    order: *mut c_int,
    tick: *mut c_int,
    index: *mut c_int,
    order_book: *mut c_int,
) -> c_int {
    if header.is_null() || trade.is_null() || order.is_null() ||
       tick.is_null() || index.is_null() || order_book.is_null() {
        return 0;
    }

    *header = message_sizes::HEADER as c_int;
    *trade = message_sizes::TRADE as c_int;
    *order = message_sizes::ORDER as c_int;
    *tick = message_sizes::TICK as c_int;
    *index = message_sizes::INDEX as c_int;
    *order_book = message_sizes::ORDER_BOOK as c_int;

    1
}

/// Create channel ID
/// 
/// # Safety
/// channel_id pointer must be valid
#[no_mangle]
pub unsafe extern "C" fn mitch_create_channel(
    provider_id: c_uint,
    msg_type: c_char,
    channel_id: *mut c_uint,
) -> c_int {
    if channel_id.is_null() {
        return 0;
    }

    let channel = ChannelId::new(provider_id as u16, msg_type as u8 as char);
    *channel_id = channel.raw;
    1
}

/// Test echo function
/// 
/// # Safety
/// All pointer parameters must be valid and properly allocated
#[no_mangle]
pub unsafe extern "C" fn mitch_test_echo(
    input: *const c_uchar, 
    input_len: c_int, 
    output: *mut c_uchar, 
    output_len: c_int
) -> c_int {
    if input.is_null() || output.is_null() || input_len <= 0 || output_len <= 0 {
        return 0;
    }
    
    let input_slice = slice::from_raw_parts(input, input_len as usize);
    let output_slice = slice::from_raw_parts_mut(output, output_len as usize);
    
    let copy_len = std::cmp::min(input_len as usize, output_len as usize);
    output_slice[..copy_len].copy_from_slice(&input_slice[..copy_len]);
    
    copy_len as c_int
}

/// Get library version
/// 
/// # Safety
/// version_out must point to a buffer of at least version_len bytes
#[no_mangle]
pub unsafe extern "C" fn mitch_get_version(
    version_out: *mut c_char,
    version_len: c_int,
) -> c_int {
    if version_out.is_null() || version_len <= 0 {
        return 0;
    }

    let version = env!("CARGO_PKG_VERSION");
    let version_bytes = version.as_bytes();
    let copy_len = std::cmp::min(version_bytes.len(), (version_len - 1) as usize);
    std::ptr::copy_nonoverlapping(version_bytes.as_ptr(), version_out as *mut u8, copy_len);
    *((version_out as *mut u8).add(copy_len)) = 0;

    1
}

// =============================================================================
// BACKWARD COMPATIBILITY ALIASES
// =============================================================================

// These provide compatibility with the old redis_client.dll interface

#[cfg(feature = "redis-client")]
#[no_mangle]
pub unsafe extern "C" fn redis_connect(url_bytes: *const c_uchar, url_len: c_int) -> c_int {
    mitch_redis_connect(url_bytes, url_len)
}

#[cfg(feature = "redis-client")]
#[no_mangle]
pub unsafe extern "C" fn redis_disconnect() -> c_int {
    mitch_redis_disconnect()
}

#[cfg(feature = "redis-client")]
#[no_mangle]
pub unsafe extern "C" fn redis_is_connected() -> c_int {
    mitch_redis_is_connected()
}

#[cfg(feature = "redis-client")]
#[no_mangle]
pub unsafe extern "C" fn redis_test_byte_echo(
    input: *const c_uchar, 
    input_len: c_int, 
    output: *mut c_uchar, 
    output_len: c_int
) -> c_int {
    mitch_test_echo(input, input_len, output, output_len)
}