/**
 * MITCH Protocol Message Structures
 * All multi-byte fields are in Big-Endian byte order
 */

// Message type constants
export const MSG_TYPE_TRADE = 0x74;        // 't'
export const MSG_TYPE_ORDER = 0x6F;        // 'o'
export const MSG_TYPE_TICKER = 0x73;       // 's'
export const MSG_TYPE_ORDER_BOOK = 0x71;   // 'q'

// Side constants
export const SIDE_BUY = 0;
export const SIDE_SELL = 1;

// Order type constants
export const ORDER_TYPE_MARKET = 0;
export const ORDER_TYPE_LIMIT = 1;
export const ORDER_TYPE_STOP = 2;
export const ORDER_TYPE_CANCEL = 3;

/**
 * Unified Message Header (8 bytes)
 */
export interface MitchHeader {
    messageType: number;    // ASCII message type code (1 byte)
    timestamp: Uint8Array;  // 48-bit timestamp (6 bytes)
    count: number;          // Number of body entries (1-255)
}

/**
 * Body Structures (32 bytes each, except OrderBook volumes)
 */

/**
 * Trade (32 bytes)
 */
export interface Trade {
    tickerId: bigint;        // 8-byte ticker identifier
    price: number;           // Execution price (8 bytes)
    quantity: number;        // Executed volume/quantity (4 bytes)
    tradeId: number;         // Unique trade identifier (4 bytes)
    side: number;            // 0: Buy, 1: Sell (1 byte)
    padding: Uint8Array;     // 7-byte padding
}

/**
 * Order (32 bytes)
 */
export interface Order {
    tickerId: bigint;        // 8-byte ticker identifier
    orderId: number;         // Unique order identifier (4 bytes)
    price: number;           // Limit/stop price (8 bytes)
    quantity: number;        // Order volume/quantity (4 bytes)
    typeAndSide: number;     // Bit 0: Side, Bits 1-7: Order Type
    expiry: Uint8Array;      // 6-byte expiry timestamp
    padding: number;         // 1-byte padding
}

/**
 * Tick (32 bytes)
 */
export interface Tick {
    tickerId: bigint;        // 8-byte ticker identifier
    bidPrice: number;        // Best bid price (8 bytes)
    askPrice: number;        // Best ask price (8 bytes)
    bidVolume: number;       // Volume at best bid (4 bytes)
    askVolume: number;       // Volume at best ask (4 bytes)
}

/**
 * OrderBook (32 bytes header only)
 * Note: Volumes are handled separately as per specification
 */
export interface OrderBook {
    tickerId: bigint;        // 8-byte ticker identifier
    firstTick: number;       // Starting price level (8 bytes)
    tickSize: number;        // Price increment per tick (8 bytes)
    numTicks: number;        // Number of price levels (2 bytes)
    side: number;            // 0: Bids, 1: Asks (1 byte)
    padding: Uint8Array;     // 5-byte padding
}

/**
 * Complete Message Structures (Header + Bodies)
 */

/**
 * Trade Message: Header + Trade[]
 */
export interface TradeMessage {
    header: MitchHeader;
    trades: Trade[];
}

/**
 * Order Message: Header + Order[]
 */
export interface OrderMessage {
    header: MitchHeader;
    orders: Order[];
}

/**
 * Ticker Message: Header + Tick[]
 */
export interface TickerMessage {
    header: MitchHeader;
    tickers: Tick[];
}

/**
 * Order Book Message: Header + OrderBook[] + volumes for each
 */
export interface OrderBookMessage {
    header: MitchHeader;
    orderBooks: OrderBook[];
    volumes: number[][]; // Array of volume arrays, one per order book
}

// --- Utility Functions ---

/**
 * Extract the side from a type_and_side field
 */
export function extractSide(typeAndSide: number): number {
    return typeAndSide & 0x01;
}

/**
 * Extract the order type from a type_and_side field
 */
export function extractOrderType(typeAndSide: number): number {
    return (typeAndSide >> 1) & 0x7F;
}

/**
 * Combine order type and side into a single field
 */
export function combineTypeAndSide(orderType: number, side: number): number {
    return (orderType << 1) | side;
}
