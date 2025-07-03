"""
MITCH Protocol Model - Python Implementation
Defines all message structures for the MITCH binary protocol.
All messages use big-endian byte order for network compatibility.
"""

from dataclasses import dataclass
from typing import List

# --- Message Type Codes ---
MSG_TYPE_TRADE = ord('t')
MSG_TYPE_ORDER = ord('o')
MSG_TYPE_TICKER = ord('s')
MSG_TYPE_ORDER_BOOK = ord('q')

# --- Side Constants ---
SIDE_BUY = 0
SIDE_SELL = 1

# --- Order Type Constants ---
ORDER_TYPE_MARKET = 0
ORDER_TYPE_LIMIT = 1
ORDER_TYPE_STOP = 2
ORDER_TYPE_CANCEL = 3

# All MITCH messages use big-endian byte order
BYTE_ORDER = '>'

# --- Unified Message Header (8 bytes) ---
@dataclass
class MitchHeader:
    message_type: int       # ASCII message type code
    timestamp: bytes        # 6-byte nanoseconds since midnight
    count: int              # Number of body entries (1-255)

# --- Body Structures (32 bytes each) ---

# TradeBody (32 bytes)
@dataclass
class TradeBody:
    ticker_id: int
    price: float
    quantity: int
    trade_id: int
    side: int               # 0: Buy, 1: Sell
    padding: bytes = b'\x00' * 7

# OrderBody (32 bytes)
@dataclass
class OrderBody:
    ticker_id: int
    order_id: int
    price: float
    quantity: int
    type_and_side: int      # Bit 0: Side, Bits 1-7: Order Type
    expiry: bytes           # 6-byte expiry timestamp
    padding: bytes = b'\x00'

# TickerBody (32 bytes)
@dataclass
class TickerBody:
    ticker_id: int
    bid_price: float
    ask_price: float
    bid_volume: int
    ask_volume: int

# OrderBookBody (Header: 32 bytes)
# Variable size: 32 bytes header + num_ticks * 4 bytes
@dataclass
class OrderBookBody:
    ticker_id: int
    first_tick: float
    tick_size: float
    num_ticks: int
    side: int               # 0: Bids, 1: Asks
    padding: bytes = b'\x00' * 5
    volumes: List[int] = None  # Volume at each price level

    def __post_init__(self):
        if self.volumes is None:
            self.volumes = []

# --- Utility Functions ---

def extract_side(type_and_side: int) -> int:
    """Extract the side from a type_and_side field."""
    return type_and_side & 0x01

def extract_order_type(type_and_side: int) -> int:
    """Extract the order type from a type_and_side field."""
    return (type_and_side >> 1) & 0x7F

def combine_type_and_side(order_type: int, side: int) -> int:
    """Combine order type and side into a single field."""
    return (order_type << 1) | side
