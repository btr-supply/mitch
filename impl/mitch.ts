/**
 * MITCH Protocol TypeScript Model
 *
 * Data structures for the MITCH (Moded ITCH) binary protocol.
 * Optimized for ultra-low latency financial market data transmission.
 *
 * Features:
 * - Trade, Order, Tick, OrderBook, and Index message types
 * - 8-byte Ticker ID encoding with asset class support
 * - 32-bit Channel ID system for pub/sub filtering
 * - Little-endian serialization for cross-platform compatibility
 * - 32-byte aligned message bodies for optimal performance
 */

// =============================================================================
// CONSTANTS AND ENUMS
// =============================================================================

/** MITCH message type codes (ASCII) */
export const MessageType = {
  TRADE: 't'.charCodeAt(0),     // 116
  ORDER: 'o'.charCodeAt(0),     // 111
  TICK: 's'.charCodeAt(0),      // 115
  ORDER_BOOK: 'b'.charCodeAt(0), // 98
  INDEX: 'i'.charCodeAt(0),     // 105
} as const;

/** Message type lookup for reverse mapping */
export const MessageTypeChar: Record<number, string> = {
  [MessageType.TRADE]: 't',
  [MessageType.ORDER]: 'o',
  [MessageType.TICK]: 's',
  [MessageType.ORDER_BOOK]: 'b',
  [MessageType.INDEX]: 'i',
};

/** Index calculation types */
export enum IndexType {
  MID = 0x00,        // Mid-market price: (bid + ask) / 2
  BBID = 0x01,       // Best bid across sources: max(all_bids)
  BASK = 0x02,       // Best ask across sources: min(all_asks)
  WBID = 0x03,       // Worst bid across sources: min(all_bids)
  WASK = 0x04,       // Worst ask across sources: max(all_asks)
  MBID = 0x05,       // Minimum bid (floor): min(all_bids)
  MASK = 0x06,       // Maximum ask (ceiling): max(all_asks)
  VWAP = 0x07,       // Volume-weighted average price
  TWAP = 0x08,       // Time-weighted average price
  LAST = 0x09,       // Last trade price
  OPEN = 0x0A,       // Opening price
  HIGH = 0x0B,       // Session high
  LOW = 0x0C,        // Session low
  CLOSE = 0x0D,      // Closing price
}

/** Order type and side encoding */
export enum OrderType {
  MARKET = 0,
  LIMIT = 1,
  STOP = 2,
  CANCEL = 3,
}

export enum OrderSide {
  BUY = 0,
  SELL = 1,
}

/** Asset classes for Ticker ID encoding */
export enum AssetClass {
  EQUITIES = 0x0,
  CORPORATE_BONDS = 0x1,
  SOVEREIGN_DEBT = 0x2,
  FOREX = 0x3,
  COMMODITIES = 0x4,
  REAL_ESTATE = 0x5,
  CRYPTO_ASSETS = 0x6,
  PRIVATE_MARKETS = 0x7,
  COLLECTIBLES = 0x8,
  INFRASTRUCTURE = 0x9,
  INDICES = 0xA,
  STRUCTURED_PRODUCTS = 0xB,
  CASH_EQUIVALENTS = 0xC,
  LOANS_RECEIVABLES = 0xD,
}

/** Instrument types for Ticker ID encoding */
export enum InstrumentType {
  SPOT = 0x0,
  FUTURE = 0x1,
  FORWARD = 0x2,
  SWAP = 0x3,
  PERPETUAL_SWAP = 0x4,
  CFD = 0x5,
  CALL_OPTION = 0x6,
  PUT_OPTION = 0x7,
  DIGITAL_OPTION = 0x8,
  BARRIER_OPTION = 0x9,
  WARRANT = 0xA,
  PREDICTION_CONTRACT = 0xB,
  STRUCTURED_PRODUCT = 0xC,
}

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/** MITCH unified message header (8 bytes) */
export interface MitchHeader {
  messageType: number;  // u8: ASCII message type ('t', 'o', 's', 'b', 'i')
  timestamp: Uint8Array; // u48: nanoseconds since midnight UTC (6 bytes)
  count: number;        // u8: number of body entries (1-255)
}

/** Helper functions for timestamp conversion */
export const TimestampUtils = {
  /** Convert u64 timestamp to u48 bytes */
  timestampToBytes(timestamp: bigint): Uint8Array {
    const bytes = new Uint8Array(8);
    const view = new DataView(bytes.buffer);
    view.setBigUint64(0, timestamp, true); // little-endian
    return bytes.slice(0, 6); // Take only first 6 bytes (u48)
  },

  /** Convert u48 bytes to u64 timestamp */
  bytesToTimestamp(bytes: Uint8Array): bigint {
    const fullBytes = new Uint8Array(8);
    fullBytes.set(bytes.slice(0, 6)); // Copy 6 bytes, rest remain 0
    const view = new DataView(fullBytes.buffer);
    return view.getBigUint64(0, true); // little-endian
  }
};

/** Trade execution data (32 bytes) */
export interface Trade {
  tickerId: bigint;     // u64: 8-byte ticker identifier
  price: number;        // f64: execution price
  quantity: number;     // u32: executed volume
  tradeId: number;      // u32: unique trade identifier
  side: OrderSide;      // u8: 0=Buy, 1=Sell
  // 7 bytes padding
}

/** Order lifecycle event (32 bytes) */
export interface Order {
  tickerId: bigint;     // u64: 8-byte ticker identifier
  orderId: number;      // u32: unique order identifier
  price: number;        // f64: limit/stop price
  quantity: number;     // u32: order volume
  typeAndSide: number;  // u8: combined type (bits 1-7) and side (bit 0)
  expiry: bigint;       // u48: expiry timestamp (Unix ms) or 0 for GTC
  // 1 byte padding
}

/** Tick/quote snapshot (32 bytes) */
export interface Tick {
  tickerId: bigint;     // u64: 8-byte ticker identifier
  bidPrice: number;     // f64: best bid price
  askPrice: number;     // f64: best ask price
  bidVolume: number;    // u32: volume at best bid
  askVolume: number;    // u32: volume at best ask
}

/** Index aggregated data (64 bytes) */
export interface Index {
  tickerId: bigint;     // u64: 8-byte ticker identifier
  mid: number;         // f64: mid price
  vbid: number;         // u32: bid volume (sell volume)
  vask: number;         // u32: ask volume (buy volume)
  mspread: number;      // i32: mean spread (1e-9 pbp)
  bbido: number;        // i32: best bid offset (1e-9 pbp)
  basko: number;        // i32: best ask offset (1e-9 pbp)
  wbido: number;        // i32: worst bid offset (1e-9 pbp)
  wasko: number;        // i32: worst ask offset (1e-9 pbp)
  vforce: number;       // u16: volatility force (0-10000)
  lforce: number;       // u16: liquidity force (0-10000)
  tforce: number;       // i16: trend force (-10000-10000)
  mforce: number;       // i16: momentum force (-10000-10000)
  confidence: number;   // u8: data quality (0-100, 100=best)
  rejected: number;     // u8: number of sources rejected
  accepted: number;     // u8: number of sources accepted
  // 9 bytes padding to 64 bytes
}

/** Bin aggregation methods for order books */
export enum BinAggregator {
  DEFAULT_LINGAUSSIAN = 0,  // Linear + Gaussian growth (default)
  DEFAULT_LINGEOFLAT = 1,   // Linear + flattened geometric
  DEFAULT_BILINGEO = 2,     // Bi-linear + geometric
  DEFAULT_TRILINEAR = 3,    // Tri-linear for high volatility
}

/** Price level structure (8 bytes) */
export interface Bin {
  count: number;        // u32: number of standing orders in this bin
  volume: number;       // u32: total volume from mid to this bin boundary
}

/** Optimized order book snapshot (2072 bytes) */
export interface OptimizedOrderBook {
  tickerId: bigint;             // u64: 8-byte ticker identifier
  midPrice: number;             // f64: current mid market price
  binAggregator: BinAggregator; // u8: aggregation method
  // 7 bytes padding
  bids: Bin[];           // 128 fixed bid levels (1024 bytes)
  asks: Bin[];           // 128 fixed ask levels (1024 bytes)
}

// =============================================================================
// CHANNEL ID SYSTEM
// =============================================================================

/** Channel ID components for pub/sub filtering */
export interface ChannelIdComponents {
  marketProviderId: number;  // u16: market provider ID from CSV
  messageType: string;       // char: MITCH message type ('t', 'o', 's', 'q', 'i')
  padding: number;           // u8: reserved for future use (currently 0)
}

/** Channel ID utilities for pub/sub systems */
export class ChannelId {
  /**
   * Generate a 32-bit channel ID from components
   * Format: [market_provider:16][message_type:8][padding:8]
   */
  static generate(marketProviderId: number, messageType: string): number {
    if (marketProviderId > 0xFFFF) {
      throw new Error('Market provider ID must fit in 16 bits');
    }

    const typeByte = messageType.charCodeAt(0);
    const padding = 0x00;

    // Pack as big-endian: [provider:16][type:8][padding:8]
    return (marketProviderId << 16) | (typeByte << 8) | padding;
  }

  /**
   * Extract components from a 32-bit channel ID
   */
  static extract(channelId: number): ChannelIdComponents {
    const marketProviderId = (channelId >>> 16) & 0xFFFF;
    const messageType = String.fromCharCode((channelId >>> 8) & 0xFF);
    const padding = channelId & 0xFF;

    return { marketProviderId, messageType, padding };
  }

  /**
   * Validate channel ID format
   */
  static validate(channelId: number): boolean {
    if (channelId < 0 || channelId > 0xFFFFFFFF) return false;

    const components = ChannelId.extract(channelId);
    const validTypes = ['t', 'o', 's', 'q', 'i'];

    return validTypes.includes(components.messageType) &&
           components.padding === 0;
  }

  /**
   * Generate channel pattern for pub/sub pattern matching
   * Example: generatePattern(101, '*') -> "657*" for all Binance messages
   */
  static generatePattern(marketProviderId: number, messageTypePattern: string): string {
    if (messageTypePattern === '*') {
      // Return hex prefix for pattern matching: "0065*" for Binance
      return (marketProviderId << 8).toString(16).toUpperCase() + '*';
    }

    return ChannelId.generate(marketProviderId, messageTypePattern).toString();
  }
}

// =============================================================================
// TICKER ID UTILITIES
// =============================================================================

/** Ticker ID encoding utilities */
export class TickerId {
  /**
   * Generate a 64-bit ticker ID from components
   * Format: [instrument_type:4][base_asset:20][quote_asset:20][sub_type:20]
   */
  static generate(
    instrumentType: InstrumentType,
    baseClass: AssetClass,
    baseId: number,
    quoteClass: AssetClass,
    quoteId: number,
    subType: number = 0
  ): bigint {
    if (baseId > 0xFFFF || quoteId > 0xFFFF || subType > 0xFFFFF) {
      throw new Error('Asset IDs must fit in 16 bits, sub-type in 20 bits');
    }

    const baseAsset = (baseClass << 16) | baseId;
    const quoteAsset = (quoteClass << 16) | quoteId;

    const tickerId =
      (BigInt(instrumentType) << 60n) |
      (BigInt(baseAsset) << 40n) |
      (BigInt(quoteAsset) << 20n) |
      BigInt(subType);

    return tickerId;
  }

  /**
   * Extract components from a 64-bit ticker ID
   */
  static extract(tickerId: bigint): {
    instrumentType: InstrumentType;
    baseClass: AssetClass;
    baseId: number;
    quoteClass: AssetClass;
    quoteId: number;
    subType: number;
  } {
    const instrumentType = Number((tickerId >> 60n) & 0xFn);
    const baseAsset = Number((tickerId >> 40n) & 0xFFFFFn);
    const quoteAsset = Number((tickerId >> 20n) & 0xFFFFFn);
    const subType = Number(tickerId & 0xFFFFFn);

    const baseClass = (baseAsset >> 16) & 0xF;
    const baseId = baseAsset & 0xFFFF;
    const quoteClass = (quoteAsset >> 16) & 0xF;
    const quoteId = quoteAsset & 0xFFFF;

    return {
      instrumentType,
      baseClass,
      baseId,
      quoteClass,
      quoteId,
      subType,
    };
  }
}

// =============================================================================
// MESSAGE CONTAINERS
// =============================================================================

/** Complete MITCH message with header and body */
export type MitchMessage =
  | { header: MitchHeader; body: Trade[] }
  | { header: MitchHeader; body: Order[] }
  | { header: MitchHeader; body: Tick[] }
  | { header: MitchHeader; body: Index[] }
  | { header: MitchHeader; body: OptimizedOrderBook[] };

/** Message size constants */
export const MESSAGE_SIZES = {
  HEADER: 8,
  TRADE: 32,
  ORDER: 32,
  TICK: 32,
  INDEX: 64,          // Updated to 64
  ORDER_BOOK_HEADER: 32,
  ORDER_BOOK_VOLUME: 4,
} as const;

// =============================================================================
// VALIDATION UTILITIES
// =============================================================================

/** Index confidence level descriptions */
export const confidenceLevel = {
  PERFECT: 100,       // Real-time, all sources available
  HIGH: 80,           // Minor delays or 1-2 sources rejected
  MEDIUM: 60,         // Noticeable delays or some rejections
  LOW: 40,            // Significant delays or many rejections
  VERY_LOW: 20,       // Stale or unreliable data
  NO_CONFIDENCE: 0,   // Data should not be used
} as const;

/** Validate index confidence score */
export function validateconfidence(confidence: number): boolean {
  return Number.isInteger(confidence) && confidence >= 0 && confidence <= 255;
}

/** Validate index type */
export function validateIndexType(indexType: number): boolean {
  return Object.values(IndexType).includes(indexType);
}

/** Channel ID examples for common exchanges */
export const CHANNEL_EXAMPLES = {
  BINANCE_TICKS: ChannelId.generate(101, 's'),     // 6,648,576
  COINBASE_TRADES: ChannelId.generate(853, 't'),   // 56,021,760
  NYSE_INDICES: ChannelId.generate(1741, 'i'),     // 114,338,048
} as const;

// =============================================================================
// SERIALIZATION AND DESERIALIZATION
// =============================================================================

// Complete packMitchMessage
function packMitchMessage(msg: MitchMessage): Uint8Array {
  const header = msg.header;
  const type = header.messageType;
  const count = header.count;
  let totalSize = MESSAGE_SIZES.HEADER;

  switch (type) {
    case MessageType.TRADE:
    case MessageType.ORDER:
    case MessageType.TICK:
      totalSize += count * 32;
      break;
    case MessageType.INDEX:
      totalSize += count * 64;
      break;
    case MessageType.ORDER_BOOK:
      (msg.body as OptimizedOrderBook[]).forEach(ob => {
        totalSize += MESSAGE_SIZES.ORDER_BOOK_HEADER + ob.bids.length * 8 + ob.asks.length * 8;
      });
      break;
  }

  const buffer = new Uint8Array(totalSize);
  const dv = new DataView(buffer.buffer);
  let offset = 0;

  // Pack header
  dv.setUint8(offset, type);
  offset += 1;

  // Pack u48 timestamp in little-endian
  const timestamp = header.timestamp;
  dv.setUint8(offset, Number(timestamp & 0xFFn));
  dv.setUint8(offset + 1, Number((timestamp >> 8n) & 0xFFn));
  dv.setUint8(offset + 2, Number((timestamp >> 16n) & 0xFFn));
  dv.setUint8(offset + 3, Number((timestamp >> 24n) & 0xFFn));
  dv.setUint8(offset + 4, Number((timestamp >> 32n) & 0xFFn));
  dv.setUint8(offset + 5, Number((timestamp >> 40n) & 0xFFn));
  offset += 6;

  dv.setUint8(offset, count);
  offset += 1;

  // Pack body based on type
  switch (type) {
    case MessageType.TRADE:
      for (const trade of msg.body as Trade[]) {
        dv.setBigUint64(offset, trade.tickerId, true); // little-endian
        offset += 8;
        dv.setFloat64(offset, trade.price, true); // little-endian
        offset += 8;
        dv.setUint32(offset, trade.quantity, true); // little-endian
        offset += 4;
        dv.setUint32(offset, trade.tradeId, true); // little-endian
        offset += 4;
        dv.setUint8(offset, trade.side);
        offset += 1;
        offset += 7; // padding
      }
      break;

    case MessageType.ORDER:
      for (const order of msg.body as Order[]) {
        dv.setBigUint64(offset, order.tickerId, true); // little-endian
        offset += 8;
        dv.setUint32(offset, order.orderId, true); // little-endian
        offset += 4;
        dv.setFloat64(offset, order.price, true); // little-endian
        offset += 8;
        dv.setUint32(offset, order.quantity, true); // little-endian
        offset += 4;
        const typeAndSide = (order.orderType << 1) | order.side;
        dv.setUint8(offset, typeAndSide);
        offset += 1;
        // Pack u48 expiry in little-endian
        const expiry = order.expiry;
        dv.setUint8(offset, Number(expiry & 0xFFn));
        dv.setUint8(offset + 1, Number((expiry >> 8n) & 0xFFn));
        dv.setUint8(offset + 2, Number((expiry >> 16n) & 0xFFn));
        dv.setUint8(offset + 3, Number((expiry >> 24n) & 0xFFn));
        dv.setUint8(offset + 4, Number((expiry >> 32n) & 0xFFn));
        dv.setUint8(offset + 5, Number((expiry >> 40n) & 0xFFn));
        offset += 6;
        offset += 1; // padding
      }
      break;

    case MessageType.TICK:
      for (const tick of msg.body as Tick[]) {
        dv.setBigUint64(offset, tick.tickerId, true); // little-endian
        offset += 8;
        dv.setFloat64(offset, tick.bidPrice, true); // little-endian
        offset += 8;
        dv.setFloat64(offset, tick.askPrice, true); // little-endian
        offset += 8;
        dv.setUint32(offset, tick.bidVolume, true); // little-endian
        offset += 4;
        dv.setUint32(offset, tick.askVolume, true); // little-endian
        offset += 4;
      }
      break;

    case MessageType.ORDER_BOOK:
      for (const orderBook of msg.body as OptimizedOrderBook[]) {
        dv.setBigUint64(offset, orderBook.tickerId, true); // little-endian
        offset += 8;
        dv.setFloat64(offset, orderBook.midPrice, true); // little-endian
        offset += 8;
        dv.setUint8(offset, orderBook.binAggregator); // little-endian
        offset += 1;
        offset += 7; // padding

        for (const bid of orderBook.bids) {
          dv.setUint32(offset, bid.count, true); // little-endian
          offset += 4;
          dv.setUint32(offset, bid.volume, true); // little-endian
          offset += 4;
        }

        for (const ask of orderBook.asks) {
          dv.setUint32(offset, ask.count, true); // little-endian
          offset += 4;
          dv.setUint32(offset, ask.volume, true); // little-endian
          offset += 4;
        }
      }
      break;

    case MessageType.INDEX:
      for (const index of msg.body as Index[]) {
        dv.setBigUint64(offset, index.tickerId, true); // little-endian
        offset += 8;
        dv.setFloat64(offset, index.mid, true); // little-endian
        offset += 8;
        dv.setUint32(offset, index.vbid, true); // little-endian
        offset += 4;
        dv.setUint32(offset, index.vask, true); // little-endian
        offset += 4;
        dv.setInt32(offset, index.mspread, true); // little-endian
        offset += 4;
        dv.setInt32(offset, index.bbido, true); // little-endian
        offset += 4;
        dv.setInt32(offset, index.basko, true); // little-endian
        offset += 4;
        dv.setInt32(offset, index.wbido, true); // little-endian
        offset += 4;
        dv.setInt32(offset, index.wasko, true); // little-endian
        offset += 4;
        dv.setUint16(offset, index.vforce, true); // little-endian
        offset += 2;
        dv.setUint16(offset, index.lforce, true); // little-endian
        offset += 2;
        dv.setInt16(offset, index.tforce, true); // little-endian
        offset += 2;
        dv.setInt16(offset, index.mforce, true); // little-endian
        offset += 2;
        dv.setUint8(offset, index.confidence);
        offset += 1;
        dv.setUint8(offset, index.rejected);
        offset += 1;
        dv.setUint8(offset, index.accepted);
        offset += 1;
        offset += 9; // padding
      }
      break;
  }

  return buffer;
}

// Complete the unpack
function unpackMitchMessage(bytes: Uint8Array): MitchMessage {
  const dv = new DataView(bytes.buffer);
  let offset = 0;

  const type = dv.getUint8(offset);
  offset += 1;

  // Unpack u48 timestamp from little-endian
  let timestamp = 0n;
  timestamp |= BigInt(dv.getUint8(offset));
  timestamp |= BigInt(dv.getUint8(offset + 1)) << 8n;
  timestamp |= BigInt(dv.getUint8(offset + 2)) << 16n;
  timestamp |= BigInt(dv.getUint8(offset + 3)) << 24n;
  timestamp |= BigInt(dv.getUint8(offset + 4)) << 32n;
  timestamp |= BigInt(dv.getUint8(offset + 5)) << 40n;
  offset += 6;

  const count = dv.getUint8(offset);
  offset += 1;

  const header: MitchHeader = { messageType: type, timestamp, count };

  let body: any[] = [];

  switch (type) {
    case MessageType.TRADE:
      for (let i = 0; i < count; i++) {
        const trade: Trade = {
          tickerId: dv.getBigUint64(offset, true), // little-endian
          price: dv.getFloat64(offset + 8, true), // little-endian
          quantity: dv.getUint32(offset + 16, true), // little-endian
          tradeId: dv.getUint32(offset + 20, true), // little-endian
          side: dv.getUint8(offset + 24)
        };
        body.push(trade);
        offset += 32;
      }
      break;

    case MessageType.ORDER:
      for (let i = 0; i < count; i++) {
        const typeAndSide = dv.getUint8(offset + 24);
        const side = typeAndSide & 0x01;
        const orderType = (typeAndSide >> 1) & 0x7F;

        // Unpack u48 expiry from little-endian
        let expiry = 0n;
        expiry |= BigInt(dv.getUint8(offset + 25));
        expiry |= BigInt(dv.getUint8(offset + 26)) << 8n;
        expiry |= BigInt(dv.getUint8(offset + 27)) << 16n;
        expiry |= BigInt(dv.getUint8(offset + 28)) << 24n;
        expiry |= BigInt(dv.getUint8(offset + 29)) << 32n;
        expiry |= BigInt(dv.getUint8(offset + 30)) << 40n;

        const order: Order = {
          tickerId: dv.getBigUint64(offset, true), // little-endian
          orderId: dv.getUint32(offset + 8, true), // little-endian
          price: dv.getFloat64(offset + 12, true), // little-endian
          quantity: dv.getUint32(offset + 20, true), // little-endian
          orderType,
          side,
          expiry
        };
        body.push(order);
        offset += 32;
      }
      break;

    case MessageType.TICK:
      for (let i = 0; i < count; i++) {
        const tick: Tick = {
          tickerId: dv.getBigUint64(offset, true), // little-endian
          bidPrice: dv.getFloat64(offset + 8, true), // little-endian
          askPrice: dv.getFloat64(offset + 16, true), // little-endian
          bidVolume: dv.getUint32(offset + 24, true), // little-endian
          askVolume: dv.getUint32(offset + 28, true) // little-endian
        };
        body.push(tick);
        offset += 32;
      }
      break;

    case MessageType.ORDER_BOOK:
      for (let i = 0; i < count; i++) {
        const tickerId = dv.getBigUint64(offset, true); // little-endian
        const midPrice = dv.getFloat64(offset + 8, true); // little-endian
        const binAggregator = dv.getUint8(offset + 16); // little-endian
        offset += 17; // skip header and padding

        let bids: Bin[] = [];
        for (let j = 0; j < 128; j++) { // Assuming 128 bid levels
          bids.push({ count: dv.getUint32(offset, true), volume: dv.getUint32(offset + 4, true) }); // little-endian
          offset += 8;
        }

        let asks: Bin[] = [];
        for (let j = 0; j < 128; j++) { // Assuming 128 ask levels
          asks.push({ count: dv.getUint32(offset, true), volume: dv.getUint32(offset + 4, true) }); // little-endian
          offset += 8;
        }

        body.push({
          tickerId,
          midPrice,
          binAggregator,
          bids,
          asks
        });
      }
      break;

    case MessageType.INDEX:
      for (let i = 0; i < count; i++) {
        const index: Index = {
          tickerId: dv.getBigUint64(offset, true), // little-endian
          mid: dv.getFloat64(offset + 8, true), // little-endian
          vbid: dv.getUint32(offset + 16, true), // little-endian
          vask: dv.getUint32(offset + 20, true), // little-endian
          mspread: dv.getInt32(offset + 24, true), // little-endian
          bbido: dv.getInt32(offset + 28, true), // little-endian
          basko: dv.getInt32(offset + 32, true), // little-endian
          wbido: dv.getInt32(offset + 36, true), // little-endian
          wasko: dv.getInt32(offset + 40, true), // little-endian
          vforce: dv.getUint16(offset + 44, true), // little-endian
          lforce: dv.getUint16(offset + 46, true), // little-endian
          tforce: dv.getInt16(offset + 48, true), // little-endian
          mforce: dv.getInt16(offset + 50, true), // little-endian
          confidence: dv.getUint8(offset + 52),
          rejected: dv.getUint8(offset + 53),
          accepted: dv.getUint8(offset + 54)
        };
        body.push(index);
        offset += 64;
      }
      break;
  }

  return { header, body };
}
