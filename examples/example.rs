use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime, UNIX_EPOCH};

// By convention, this example assumes `model.rs` is in the same directory
// or available in the crate path.
use crate::model::*;

// === TIMESTAMP UTILITY FUNCTIONS ===

/// Get current timestamp in nanoseconds since midnight UTC
pub fn get_timestamp_nanos() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    
    // Get nanoseconds since midnight UTC
    let total_nanos = now.as_nanos() as u64;
    let nanos_per_day = 24 * 60 * 60 * 1_000_000_000u64;
    total_nanos % nanos_per_day
}

/// Convert a 64-bit timestamp to 6-byte array (48-bit)
pub fn write_timestamp_48(timestamp: u64) -> [u8; 6] {
    let bytes = timestamp.to_be_bytes();
    [bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]
}

/// Convert a 6-byte array to 64-bit timestamp
pub fn read_timestamp_48(bytes: [u8; 6]) -> u64 {
    let mut full_bytes = [0u8; 8];
    full_bytes[2..8].copy_from_slice(&bytes);
    u64::from_be_bytes(full_bytes)
}

// An enum to represent any type of unpacked message body.
#[derive(Debug)]
pub enum MessageBody {
    Trade(Trade),
    Order(Order),
    Ticker(Tick),
    OrderBook(OrderBook, Vec<u32>), // Special case for variable data
}

// An enum to represent a fully unpacked message.
#[derive(Debug)]
pub enum MitchMessage {
    Trades(MitchHeader, Vec<Trade>),
    Orders(MitchHeader, Vec<Order>),
    Tickers(MitchHeader, Vec<Tick>),
    OrderBook(MitchHeader, OrderBook, Vec<u32>),
}

// === Packing Functions ===

pub fn pack_header(header: &MitchHeader) -> [u8; 8] {
    let mut buf = [0; 8];
    buf[0] = header.message_type;
    buf[1..7].copy_from_slice(&header.timestamp);
    buf[7] = header.count;
    buf
}

pub fn pack_trade_body(body: &Trade) -> [u8; 32] {
    let mut buf = [0; 32];
    buf[0..8].copy_from_slice(&body.ticker_id.to_be_bytes());
    buf[8..16].copy_from_slice(&body.price.to_be_bytes());
    buf[16..20].copy_from_slice(&body.quantity.to_be_bytes());
    buf[20..24].copy_from_slice(&body.trade_id.to_be_bytes());
    buf[24] = body.side;
    buf
}

pub fn pack_order_body(body: &Order) -> [u8; 32] {
    let mut buf = [0; 32];
    buf[0..8].copy_from_slice(&body.ticker_id.to_be_bytes());
    buf[8..12].copy_from_slice(&body.order_id.to_be_bytes());
    buf[12..20].copy_from_slice(&body.price.to_be_bytes());
    buf[20..24].copy_from_slice(&body.quantity.to_be_bytes());
    buf[24] = body.type_and_side;
    buf[25..31].copy_from_slice(&body.expiry);
    buf[31] = body.padding;
    buf
}

pub fn pack_ticker_body(body: &Tick) -> [u8; 32] {
    let mut buf = [0; 32];
    buf[0..8].copy_from_slice(&body.ticker_id.to_be_bytes());
    buf[8..16].copy_from_slice(&body.bid_price.to_be_bytes());
    buf[16..24].copy_from_slice(&body.ask_price.to_be_bytes());
    buf[24..28].copy_from_slice(&body.bid_volume.to_be_bytes());
    buf[28..32].copy_from_slice(&body.ask_volume.to_be_bytes());
    buf
}

// === Unpacking Functions ===

pub fn unpack_header(data: &[u8]) -> MitchHeader {
    let mut timestamp = [0; 6];
    timestamp.copy_from_slice(&data[1..7]);
    MitchHeader {
        message_type: data[0],
        timestamp,
        count: data[7],
    }
}

pub fn unpack_trade_body(data: &[u8]) -> Trade {
    Trade {
        ticker_id: u64::from_be_bytes(data[0..8].try_into().unwrap()),
        price: f64::from_be_bytes(data[8..16].try_into().unwrap()),
        quantity: u32::from_be_bytes(data[16..20].try_into().unwrap()),
        trade_id: u32::from_be_bytes(data[20..24].try_into().unwrap()),
        side: data[24],
        ..Default::default()
    }
}

pub fn unpack_order_body(data: &[u8]) -> Order {
    let mut expiry = [0; 6];
    expiry.copy_from_slice(&data[25..31]);
    Order {
        ticker_id: u64::from_be_bytes(data[0..8].try_into().unwrap()),
        order_id: u32::from_be_bytes(data[8..12].try_into().unwrap()),
        price: f64::from_be_bytes(data[12..20].try_into().unwrap()),
        quantity: u32::from_be_bytes(data[20..24].try_into().unwrap()),
        type_and_side: data[24],
        expiry,
        padding: data[31],
    }
}

pub fn unpack_full_message(data: &[u8]) -> Result<MitchMessage, String> {
    if data.len() < 8 {
        return Err("Insufficient data for header".to_string());
    }
    let header = unpack_header(&data[0..8]);
    let body_data = &data[8..];
    let count = header.count as usize;

    match header.message_type {
        MSG_TYPE_TRADE => {
            if body_data.len() < count * 32 {
                return Err("Insufficient data for trade bodies".to_string());
            }
            let mut trades = Vec::with_capacity(count);
            for i in 0..count {
                let offset = i * 32;
                trades.push(unpack_trade_body(&body_data[offset..offset + 32]));
            }
            Ok(MitchMessage::Trades(header, trades))
        }
        MSG_TYPE_ORDER => {
            if body_data.len() < count * 32 {
                return Err("Insufficient data for order bodies".to_string());
            }
            let mut orders = Vec::with_capacity(count);
            for i in 0..count {
                let offset = i * 32;
                orders.push(unpack_order_body(&body_data[offset..offset + 32]));
            }
            Ok(MitchMessage::Orders(header, orders))
        }
        // Ticker and OrderBook would be handled similarly
        _ => Err(format!("Unsupported message type: {}", header.message_type)),
    }
}

// === TCP Functions ===

pub fn mitch_send_tcp(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    stream.write_all(data)
}

pub fn mitch_recv_message(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut header_buf = [0; 8];
    stream.read_exact(&mut header_buf)?;
    let header = unpack_header(&header_buf);

    let body_len = if header.message_type == MSG_TYPE_ORDER_BOOK {
        let mut body_header_buf = [0; 32];
        stream.read_exact(&mut body_header_buf)?;
        let num_ticks = u16::from_be_bytes(body_header_buf[24..26].try_into().unwrap());
        32 + (num_ticks as usize * 4)
    } else {
        header.count as usize * 32
    };

    let mut body_buf = vec![0; body_len];
    stream.read_exact(&mut body_buf)?;

    let mut full_message = Vec::with_capacity(8 + body_len);
    full_message.extend_from_slice(&header_buf);
    full_message.extend_from_slice(&body_buf);

    Ok(full_message)
}


// === Main Example ===

fn main() {
    println!("--- Running MITCH Rust Example ---");

    // 1. Create and pack a batch of two trades
    let trades = vec![
        pack_trade_body(&Trade {
            ticker_id: 1,
            price: 1.2345,
            quantity: 100,
            trade_id: 1001,
            side: SIDE_BUY,
            ..Default::default()
        }),
        pack_trade_body(&Trade {
            ticker_id: 1,
            price: 1.2346,
            quantity: 50,
            trade_id: 1002,
            side: SIDE_SELL,
            ..Default::default()
        }),
    ];

    let mut message_bytes = Vec::new();
    let header = MitchHeader {
        message_type: MSG_TYPE_TRADE,
        timestamp: write_timestamp_48(get_timestamp_nanos()),
        count: trades.len() as u8,
    };
    message_bytes.extend_from_slice(&pack_header(&header));
    for trade_bytes in &trades {
        message_bytes.extend_from_slice(trade_bytes);
    }
    
    println!("Packed batch trade message ({} bytes): {:x?}", message_bytes.len(), message_bytes);

    // 2. Unpack the message
    match unpack_full_message(&message_bytes) {
        Ok(MitchMessage::Trades(hdr, bodies)) => {
            println!("Unpacked Header: {:?}", hdr);
            for (i, body) in bodies.iter().enumerate() {
                println!("Unpacked Trade {}: {:?}", i + 1, body);
            }
        }
        Ok(_) => println!("Unpacked a different message type"),
        Err(e) => println!("Error unpacking: {}", e),
    }
}
