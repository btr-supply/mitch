"""
MITCH Protocol Python Example
Complete implementation with packing/unpacking and TCP send/recv
"""

import struct
import socket
import time
from typing import List, Tuple, Optional, Union

from ..model.model import (
    MitchHeader, TradeBody, OrderBody, TickerBody, OrderBookBody,
    MSG_TYPE_TRADE, MSG_TYPE_ORDER, MSG_TYPE_TICKER, MSG_TYPE_ORDER_BOOK, BYTE_ORDER,
    SIDE_BUY, SIDE_SELL, ORDER_TYPE_MARKET, ORDER_TYPE_LIMIT, ORDER_TYPE_STOP, ORDER_TYPE_CANCEL,
    extract_side, extract_order_type, combine_type_and_side
)

# === TIMESTAMP UTILITY FUNCTIONS ===

def get_timestamp_nanos() -> int:
    return int(time.time_ns())

def write_timestamp_48(timestamp: int) -> bytes:
    """Convert 64-bit timestamp to 48-bit big-endian bytes"""
    return struct.pack(f'{BYTE_ORDER}Q', timestamp)[2:]  # Take last 6 bytes

def read_timestamp_48(data: bytes) -> int:
    """Convert 48-bit big-endian bytes to 64-bit timestamp"""
    return struct.unpack(f'{BYTE_ORDER}Q', b'\x00\x00' + data)[0]

# === PACKING FUNCTIONS ===

def pack_header(header: MitchHeader) -> bytes:
    """Pack header into 8 bytes using big-endian format"""
    return struct.pack(f'{BYTE_ORDER}c6sB', 
                      bytes([header.message_type]), 
                      header.timestamp, 
                      header.count)

def pack_trade_body(trade: TradeBody) -> bytes:
    """Pack trade body into 32 bytes using big-endian format"""
    return struct.pack(f'{BYTE_ORDER}QdIIB7s',
                      trade.ticker_id,
                      trade.price,
                      trade.quantity,
                      trade.trade_id,
                      trade.side,
                      trade.padding)

def pack_order_body(order: OrderBody) -> bytes:
    """Pack order body into 32 bytes using big-endian format"""
    return struct.pack(f'{BYTE_ORDER}QIdIB6ss',
                      order.ticker_id,
                      order.order_id,
                      order.price,
                      order.quantity,
                      order.type_and_side,
                      order.expiry,
                      order.padding)

def pack_ticker_body(ticker: TickerBody) -> bytes:
    """Pack ticker body into 32 bytes using big-endian format"""
    return struct.pack(f'{BYTE_ORDER}QddII',
                      ticker.ticker_id,
                      ticker.bid_price,
                      ticker.ask_price,
                      ticker.bid_volume,
                      ticker.ask_volume)

def pack_order_book_body(order_book: OrderBookBody) -> bytes:
    """Pack order book body into variable-sized bytes using big-endian format"""
    header = struct.pack(f'{BYTE_ORDER}QddHB5s',
                        order_book.ticker_id,
                        order_book.first_tick,
                        order_book.tick_size,
                        order_book.num_ticks,
                        order_book.side,
                        order_book.padding)
    
    volumes_data = struct.pack(f'{BYTE_ORDER}{len(order_book.volumes)}I', *order_book.volumes)
    return header + volumes_data

# === UNPACKING FUNCTIONS ===

def unpack_header(data: bytes) -> MitchHeader:
    """Unpack 8 bytes into header using big-endian format"""
    message_type_bytes, timestamp, count = struct.unpack(f'{BYTE_ORDER}c6sB', data[:8])
    return MitchHeader(ord(message_type_bytes), timestamp, count)

def unpack_trade_body(data: bytes) -> TradeBody:
    """Unpack 32 bytes into trade body using big-endian format"""
    ticker_id, price, quantity, trade_id, side, padding = struct.unpack(f'{BYTE_ORDER}QdIIB7s', data[:32])
    return TradeBody(ticker_id, price, quantity, trade_id, side, padding)

def unpack_order_body(data: bytes) -> OrderBody:
    """Unpack 32 bytes into order body using big-endian format"""
    ticker_id, order_id, price, quantity, type_and_side, expiry, padding = \
        struct.unpack(f'{BYTE_ORDER}QIdIB6ss', data[:32])
    return OrderBody(ticker_id, order_id, price, quantity, type_and_side, expiry, padding)

def unpack_ticker_body(data: bytes) -> TickerBody:
    """Unpack 32 bytes into ticker body using big-endian format"""
    ticker_id, bid_price, ask_price, bid_volume, ask_volume = \
        struct.unpack(f'{BYTE_ORDER}QddII', data[:32])
    return TickerBody(ticker_id, bid_price, ask_price, bid_volume, ask_volume)

def unpack_order_book_body(data: bytes) -> OrderBookBody:
    """Unpack variable-sized bytes into order book body using big-endian format"""
    ticker_id, first_tick, tick_size, num_ticks, side, padding = \
        struct.unpack(f'{BYTE_ORDER}QddHB5s', data[:32])
    
    volumes = list(struct.unpack(f'{BYTE_ORDER}{num_ticks}I', data[32:32 + num_ticks * 4]))
    return OrderBookBody(ticker_id, first_tick, tick_size, num_ticks, side, padding, volumes)

# === TCP SEND/RECV FUNCTIONS ===

def mitch_send_tcp(sock: socket.socket, data: bytes) -> None:
    """Send data via TCP, ensuring all bytes are sent"""
    total_sent = 0
    while total_sent < len(data):
        sent = sock.send(data[total_sent:])
        if sent == 0:
            raise ConnectionError("Socket connection broken")
        total_sent += sent

def mitch_recv_tcp(sock: socket.socket, length: int) -> bytes:
    """Receive exact number of bytes via TCP"""
    data = b''
    while len(data) < length:
        chunk = sock.recv(length - len(data))
        if not chunk:
            raise ConnectionError("Socket connection broken")
        data += chunk
    return data

def mitch_recv_message(sock: socket.socket) -> bytes:
    """Receive a complete MITCH message from TCP socket"""
    # First receive the header
    header_data = mitch_recv_tcp(sock, 8)
    header = unpack_header(header_data)
    
    # Calculate message size based on type and count
    if header.message_type in [MSG_TYPE_TRADE, MSG_TYPE_ORDER, MSG_TYPE_TICKER]:
        message_size = 8 + header.count * 32
        
        # Receive the rest of the message
        if message_size > 8:
            body_data = mitch_recv_tcp(sock, message_size - 8)
            return header_data + body_data
        else:
            return header_data
            
    elif header.message_type == MSG_TYPE_ORDER_BOOK:
        # For order books, we need to read the header first to get num_ticks
        body_header_data = mitch_recv_tcp(sock, 32)
        
        # Extract num_ticks from the body header
        num_ticks = struct.unpack(f'{BYTE_ORDER}H', body_header_data[24:26])[0]
        volume_size = num_ticks * 4
        
        # Receive volumes if any
        if volume_size > 0:
            volumes_data = mitch_recv_tcp(sock, volume_size)
            return header_data + body_header_data + volumes_data
        else:
            return header_data + body_header_data
    else:
        raise ValueError(f"Unknown message type: {header.message_type}")

def parse_message(data: bytes) -> Tuple[MitchHeader, Union[List[TradeBody], List[OrderBody], List[TickerBody], OrderBookBody]]:
    """Parse a complete MITCH message"""
    header = unpack_header(data[:8])
    bodies = []
    
    if header.message_type == MSG_TYPE_TRADE:
        for i in range(header.count):
            offset = 8 + i * 32
            bodies.append(unpack_trade_body(data[offset:offset+32]))
    elif header.message_type == MSG_TYPE_ORDER:
        for i in range(header.count):
            offset = 8 + i * 32
            bodies.append(unpack_order_body(data[offset:offset+32]))
    elif header.message_type == MSG_TYPE_TICKER:
        for i in range(header.count):
            offset = 8 + i * 32
            bodies.append(unpack_ticker_body(data[offset:offset+32]))
    elif header.message_type == MSG_TYPE_ORDER_BOOK:
        # Order book is a single body, not a list
        return header, unpack_order_book_body(data[8:])
    else:
        raise ValueError(f"Unknown message type: {header.message_type}")
    
    return header, bodies

# === EXAMPLE USAGE ===

def example_usage():
    # Create a trade message
    header = MitchHeader(
        message_type=MSG_TYPE_TRADE,
        timestamp=write_timestamp_48(get_timestamp_nanos()),
        count=1
    )
    
    trade = TradeBody(
        ticker_id=0x00006F001CD00000,  # EUR/USD
        price=1.0850,
        quantity=1000000,  # 1.0 lot scaled by 1000000
        trade_id=12345,
        side=SIDE_BUY
    )
    
    # Pack the message
    message = pack_header(header) + pack_trade_body(trade)
    
    # Example TCP send/receive
    try:
        # Server side
        server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server_sock.bind(('localhost', 8080))
        server_sock.listen(1)
        
        # Client side
        client_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        client_sock.connect(('localhost', 8080))
        
        # Send message
        mitch_send_tcp(client_sock, message)
        
        # Receive message
        conn, addr = server_sock.accept()
        received_message = mitch_recv_message(conn)
        
        # Parse received message
        received_header, received_bodies = parse_message(received_message)
        
        print(f"Received trade: {received_bodies[0]}")
        
    except Exception as e:
        print(f"Example error: {e}")
    finally:
        # Cleanup
        try:
            client_sock.close()
            server_sock.close()
        except:
            pass

if __name__ == "__main__":
    example_usage()
