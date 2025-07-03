import * as net from 'net';
import {
    MitchHeader, TradeBody, OrderBody, TickerBody, OrderBookBody,
    TradeMessage, OrderMessage, TickerMessage, OrderBookMessage,
    MSG_TYPE_TRADE, MSG_TYPE_ORDER, MSG_TYPE_TICKER, MSG_TYPE_ORDER_BOOK,
    SIDE_BUY, SIDE_SELL, ORDER_TYPE_MARKET, ORDER_TYPE_LIMIT,
    extractSide, extractOrderType, combineTypeAndSide
} from '../model/model';

// === TIMESTAMP UTILITY FUNCTIONS ===

/**
 * Write 48-bit timestamp to Uint8Array
 */
export function writeTimestamp48(dest: Uint8Array, nanos: bigint): void {
    if (dest.length !== 6) {
        throw new Error('Timestamp buffer must be exactly 6 bytes');
    }
    dest[0] = Number((nanos >> 40n) & 0xFFn);
    dest[1] = Number((nanos >> 32n) & 0xFFn);
    dest[2] = Number((nanos >> 24n) & 0xFFn);
    dest[3] = Number((nanos >> 16n) & 0xFFn);
    dest[4] = Number((nanos >> 8n) & 0xFFn);
    dest[5] = Number(nanos & 0xFFn);
}

/**
 * Read 48-bit timestamp from Uint8Array
 */
export function readTimestamp48(src: Uint8Array): bigint {
    if (src.length !== 6) {
        throw new Error('Timestamp buffer must be exactly 6 bytes');
    }
    return (BigInt(src[0]) << 40n) |
           (BigInt(src[1]) << 32n) |
           (BigInt(src[2]) << 24n) |
           (BigInt(src[3]) << 16n) |
           (BigInt(src[4]) << 8n) |
           BigInt(src[5]);
}

/**
 * Get current timestamp in nanoseconds since midnight UTC
 */
export function getCurrentTimestampNanos(): bigint {
    const now = new Date();
    const midnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const nanosSinceMidnight = BigInt((now.getTime() - midnight.getTime()) * 1000000);
    return nanosSinceMidnight;
}

// === HEADER PACKING/UNPACKING ===

/**
 * Pack MITCH header into 8-byte buffer
 */
export function packHeader(header: MitchHeader): Uint8Array {
    const buffer = new Uint8Array(8);
    buffer[0] = header.messageType;
    buffer.set(header.timestamp, 1);
    buffer[7] = header.count;
    return buffer;
}

/**
 * Unpack MITCH header from 8-byte buffer
 */
export function unpackHeader(buffer: Uint8Array): MitchHeader {
    if (buffer.length < 8) {
        throw new Error('Header buffer must be at least 8 bytes');
    }
    
    const timestamp = new Uint8Array(6);
    timestamp.set(buffer.subarray(1, 7));
    
    return {
        messageType: buffer[0],
        timestamp: timestamp,
        count: buffer[7]
    };
}

// === BODY PACKING/UNPACKING FUNCTIONS ===

/**
 * Pack TradeBody into 32-byte buffer
 */
export function packTradeBody(trade: TradeBody): Uint8Array {
    const buffer = new ArrayBuffer(32);
    const view = new DataView(buffer);
    let offset = 0;
    
    // Ticker ID (8 bytes, big-endian)
    view.setBigUint64(offset, trade.tickerId, false);
    offset += 8;
    
    // Price (8 bytes, big-endian double)
    view.setFloat64(offset, trade.price, false);
    offset += 8;
    
    // Quantity (4 bytes, big-endian)
    view.setUint32(offset, trade.quantity, false);
    offset += 4;
    
    // Trade ID (4 bytes, big-endian)
    view.setUint32(offset, trade.tradeId, false);
    offset += 4;
    
    // Side (1 byte)
    view.setUint8(offset, trade.side);
    offset += 1;
    
    // Padding (7 bytes) - set to zero and copy provided padding if any
    const result = new Uint8Array(buffer);
    if (trade.padding && trade.padding.length >= 7) {
        result.set(trade.padding.subarray(0, 7), 25);
    }
    
    return result;
}

/**
 * Unpack TradeBody from 32-byte buffer
 */
export function unpackTradeBody(buffer: Uint8Array): TradeBody {
    if (buffer.length < 32) {
        throw new Error('TradeBody buffer must be at least 32 bytes');
    }
    
    const view = new DataView(buffer.buffer, buffer.byteOffset);
    let offset = 0;
    
    const tickerId = view.getBigUint64(offset, false);
    offset += 8;
    
    const price = view.getFloat64(offset, false);
    offset += 8;
    
    const quantity = view.getUint32(offset, false);
    offset += 4;
    
    const tradeId = view.getUint32(offset, false);
    offset += 4;
    
    const side = view.getUint8(offset);
    offset += 1;
    
    const padding = new Uint8Array(7);
    padding.set(buffer.subarray(25, 32));
    
    return {
        tickerId,
        price,
        quantity,
        tradeId,
        side,
        padding
    };
}

/**
 * Pack OrderBody into 32-byte buffer
 */
export function packOrderBody(order: OrderBody): Uint8Array {
    const buffer = new ArrayBuffer(32);
    const view = new DataView(buffer);
    let offset = 0;
    
    // Ticker ID (8 bytes, big-endian)
    view.setBigUint64(offset, order.tickerId, false);
    offset += 8;
    
    // Order ID (4 bytes, big-endian)
    view.setUint32(offset, order.orderId, false);
    offset += 4;
    
    // Price (8 bytes, big-endian double)
    view.setFloat64(offset, order.price, false);
    offset += 8;
    
    // Quantity (4 bytes, big-endian)
    view.setUint32(offset, order.quantity, false);
    offset += 4;
    
    // Type and Side (1 byte)
    view.setUint8(offset, order.typeAndSide);
    offset += 1;
    
    // Expiry (6 bytes)
    const result = new Uint8Array(buffer);
    result.set(order.expiry, 25);
    
    // Padding (1 byte)
    result[31] = order.padding;
    
    return result;
}

/**
 * Unpack OrderBody from 32-byte buffer
 */
export function unpackOrderBody(buffer: Uint8Array): OrderBody {
    if (buffer.length < 32) {
        throw new Error('OrderBody buffer must be at least 32 bytes');
    }
    
    const view = new DataView(buffer.buffer, buffer.byteOffset);
    let offset = 0;
    
    const tickerId = view.getBigUint64(offset, false);
    offset += 8;
    
    const orderId = view.getUint32(offset, false);
    offset += 4;
    
    const price = view.getFloat64(offset, false);
    offset += 8;
    
    const quantity = view.getUint32(offset, false);
    offset += 4;
    
    const typeAndSide = view.getUint8(offset);
    offset += 1;
    
    const expiry = new Uint8Array(6);
    expiry.set(buffer.subarray(25, 31));
    
    const padding = buffer[31];
    
    return {
        tickerId,
        orderId,
        price,
        quantity,
        typeAndSide,
        expiry,
        padding
    };
}

/**
 * Pack TickerBody into 32-byte buffer
 */
export function packTickerBody(ticker: TickerBody): Uint8Array {
    const buffer = new ArrayBuffer(32);
    const view = new DataView(buffer);
    let offset = 0;
    
    // Ticker ID (8 bytes, big-endian)
    view.setBigUint64(offset, ticker.tickerId, false);
    offset += 8;
    
    // Bid Price (8 bytes, big-endian double)
    view.setFloat64(offset, ticker.bidPrice, false);
    offset += 8;
    
    // Ask Price (8 bytes, big-endian double)
    view.setFloat64(offset, ticker.askPrice, false);
    offset += 8;
    
    // Bid Volume (4 bytes, big-endian)
    view.setUint32(offset, ticker.bidVolume, false);
    offset += 4;
    
    // Ask Volume (4 bytes, big-endian)
    view.setUint32(offset, ticker.askVolume, false);
    
    return new Uint8Array(buffer);
}

/**
 * Unpack TickerBody from 32-byte buffer
 */
export function unpackTickerBody(buffer: Uint8Array): TickerBody {
    if (buffer.length < 32) {
        throw new Error('TickerBody buffer must be at least 32 bytes');
    }
    
    const view = new DataView(buffer.buffer, buffer.byteOffset);
    let offset = 0;
    
    const tickerId = view.getBigUint64(offset, false);
    offset += 8;
    
    const bidPrice = view.getFloat64(offset, false);
    offset += 8;
    
    const askPrice = view.getFloat64(offset, false);
    offset += 8;
    
    const bidVolume = view.getUint32(offset, false);
    offset += 4;
    
    const askVolume = view.getUint32(offset, false);
    
    return {
        tickerId,
        bidPrice,
        askPrice,
        bidVolume,
        askVolume
    };
}

/**
 * Pack OrderBookBody into 32-byte buffer
 */
export function packOrderBookBody(orderBook: OrderBookBody): Uint8Array {
    const buffer = new ArrayBuffer(32);
    const view = new DataView(buffer);
    let offset = 0;
    
    // Ticker ID (8 bytes, big-endian)
    view.setBigUint64(offset, orderBook.tickerId, false);
    offset += 8;
    
    // First Tick (8 bytes, big-endian double)
    view.setFloat64(offset, orderBook.firstTick, false);
    offset += 8;
    
    // Tick Size (8 bytes, big-endian double)
    view.setFloat64(offset, orderBook.tickSize, false);
    offset += 8;
    
    // Num Ticks (2 bytes, big-endian)
    view.setUint16(offset, orderBook.numTicks, false);
    offset += 2;
    
    // Side (1 byte)
    view.setUint8(offset, orderBook.side);
    offset += 1;
    
    // Padding (5 bytes)
    const result = new Uint8Array(buffer);
    if (orderBook.padding && orderBook.padding.length >= 5) {
        result.set(orderBook.padding.subarray(0, 5), 27);
    }
    
    return result;
}

/**
 * Unpack OrderBookBody from 32-byte buffer
 */
export function unpackOrderBookBody(buffer: Uint8Array): OrderBookBody {
    if (buffer.length < 32) {
        throw new Error('OrderBookBody buffer must be at least 32 bytes');
    }
    
    const view = new DataView(buffer.buffer, buffer.byteOffset);
    let offset = 0;
    
    const tickerId = view.getBigUint64(offset, false);
    offset += 8;
    
    const firstTick = view.getFloat64(offset, false);
    offset += 8;
    
    const tickSize = view.getFloat64(offset, false);
    offset += 8;
    
    const numTicks = view.getUint16(offset, false);
    offset += 2;
    
    const side = view.getUint8(offset);
    offset += 1;
    
    const padding = new Uint8Array(5);
    padding.set(buffer.subarray(27, 32));
    
    return {
        tickerId,
        firstTick,
        tickSize,
        numTicks,
        side,
        padding
    };
}

// === TCP HELPER FUNCTIONS ===

/**
 * Send complete data via TCP socket
 */
export function mitchSendTCP(socket: net.Socket, data: Uint8Array): Promise<void> {
    return new Promise((resolve, reject) => {
        socket.write(Buffer.from(data), (error) => {
            if (error) {
                reject(error);
            } else {
                resolve();
            }
        });
    });
}

/**
 * Receive exact amount of data via TCP socket
 */
export function mitchRecvTCP(socket: net.Socket, length: number): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
        let buffer = Buffer.alloc(0);
        
        const onData = (chunk: Buffer) => {
            buffer = Buffer.concat([buffer, chunk]);
            
            if (buffer.length >= length) {
                socket.removeListener('data', onData);
                socket.removeListener('error', onError);
                socket.removeListener('end', onEnd);
                
                const result = new Uint8Array(buffer.subarray(0, length));
                resolve(result);
            }
        };
        
        const onError = (error: Error) => {
            socket.removeListener('data', onData);
            socket.removeListener('error', onError);
            socket.removeListener('end', onEnd);
            reject(error);
        };
        
        const onEnd = () => {
            socket.removeListener('data', onData);
            socket.removeListener('error', onError);
            socket.removeListener('end', onEnd);
            reject(new Error('Connection ended before receiving complete data'));
        };
        
        socket.on('data', onData);
        socket.on('error', onError);
        socket.on('end', onEnd);
    });
}

// === COMPLETE MESSAGE FUNCTIONS ===

/**
 * Pack complete MITCH message (Header + Bodies)
 */
export function packTradeMessage(message: TradeMessage): Uint8Array {
    const headerBuffer = packHeader(message.header);
    const bodyBuffers = message.trades.map(trade => packTradeBody(trade));
    
    const totalLength = 8 + bodyBuffers.length * 32;
    const result = new Uint8Array(totalLength);
    
    result.set(headerBuffer, 0);
    let offset = 8;
    for (const bodyBuffer of bodyBuffers) {
        result.set(bodyBuffer, offset);
        offset += 32;
    }
    
    return result;
}

/**
 * Pack complete Order message
 */
export function packOrderMessage(message: OrderMessage): Uint8Array {
    const headerBuffer = packHeader(message.header);
    const bodyBuffers = message.orders.map(order => packOrderBody(order));
    
    const totalLength = 8 + bodyBuffers.length * 32;
    const result = new Uint8Array(totalLength);
    
    result.set(headerBuffer, 0);
    let offset = 8;
    for (const bodyBuffer of bodyBuffers) {
        result.set(bodyBuffer, offset);
        offset += 32;
    }
    
    return result;
}

/**
 * Pack complete Ticker message
 */
export function packTickerMessage(message: TickerMessage): Uint8Array {
    const headerBuffer = packHeader(message.header);
    const bodyBuffers = message.tickers.map(ticker => packTickerBody(ticker));
    
    const totalLength = 8 + bodyBuffers.length * 32;
    const result = new Uint8Array(totalLength);
    
    result.set(headerBuffer, 0);
    let offset = 8;
    for (const bodyBuffer of bodyBuffers) {
        result.set(bodyBuffer, offset);
        offset += 32;
    }
    
    return result;
}

/**
 * Pack complete Order Book message
 */
export function packOrderBookMessage(message: OrderBookMessage): Uint8Array {
    const headerBuffer = packHeader(message.header);
    
    // Calculate total size
    let totalBodySize = message.orderBooks.length * 32;
    for (let i = 0; i < message.orderBooks.length; i++) {
        totalBodySize += message.volumes[i].length * 4;
    }
    
    const result = new Uint8Array(8 + totalBodySize);
    result.set(headerBuffer, 0);
    
    let offset = 8;
    for (let i = 0; i < message.orderBooks.length; i++) {
        // Pack order book header
        const bodyBuffer = packOrderBookBody(message.orderBooks[i]);
        result.set(bodyBuffer, offset);
        offset += 32;
        
        // Pack volumes
        const view = new DataView(result.buffer, result.byteOffset + offset);
        for (let j = 0; j < message.volumes[i].length; j++) {
            view.setUint32(j * 4, message.volumes[i][j], false);
        }
        offset += message.volumes[i].length * 4;
    }
    
    return result;
}

/**
 * Receive and parse complete MITCH message from TCP socket
 */
export async function mitchRecvMessage(socket: net.Socket): Promise<TradeMessage | OrderMessage | TickerMessage | OrderBookMessage> {
    // Receive header first
    const headerBuffer = await mitchRecvTCP(socket, 8);
    const header = unpackHeader(headerBuffer);
    
    switch (header.messageType) {
        case MSG_TYPE_TRADE: {
            const bodySize = header.count * 32;
            const bodyBuffer = await mitchRecvTCP(socket, bodySize);
            
            const trades: TradeBody[] = [];
            for (let i = 0; i < header.count; i++) {
                const tradeBuffer = bodyBuffer.subarray(i * 32, (i + 1) * 32);
                trades.push(unpackTradeBody(tradeBuffer));
            }
            
            return { header, trades };
        }
        
        case MSG_TYPE_ORDER: {
            const bodySize = header.count * 32;
            const bodyBuffer = await mitchRecvTCP(socket, bodySize);
            
            const orders: OrderBody[] = [];
            for (let i = 0; i < header.count; i++) {
                const orderBuffer = bodyBuffer.subarray(i * 32, (i + 1) * 32);
                orders.push(unpackOrderBody(orderBuffer));
            }
            
            return { header, orders };
        }
        
        case MSG_TYPE_TICKER: {
            const bodySize = header.count * 32;
            const bodyBuffer = await mitchRecvTCP(socket, bodySize);
            
            const tickers: TickerBody[] = [];
            for (let i = 0; i < header.count; i++) {
                const tickerBuffer = bodyBuffer.subarray(i * 32, (i + 1) * 32);
                tickers.push(unpackTickerBody(tickerBuffer));
            }
            
            return { header, tickers };
        }
        
        case MSG_TYPE_ORDER_BOOK: {
            const orderBooks: OrderBookBody[] = [];
            const volumes: number[][] = [];
            
            let offset = 0;
            for (let i = 0; i < header.count; i++) {
                // Read order book header (32 bytes)
                const headerBuffer = await mitchRecvTCP(socket, 32);
                const orderBook = unpackOrderBookBody(headerBuffer);
                orderBooks.push(orderBook);
                
                // Read volumes (numTicks * 4 bytes)
                const volumeSize = orderBook.numTicks * 4;
                const volumeBuffer = await mitchRecvTCP(socket, volumeSize);
                
                const bookVolumes: number[] = [];
                const view = new DataView(volumeBuffer.buffer, volumeBuffer.byteOffset);
                for (let j = 0; j < orderBook.numTicks; j++) {
                    bookVolumes.push(view.getUint32(j * 4, false));
                }
                volumes.push(bookVolumes);
            }
            
            return { header, orderBooks, volumes };
        }
        
        default:
            throw new Error(`Unknown message type: ${header.messageType}`);
    }
}

// === EXAMPLE USAGE ===

/**
 * Utility function to encode trading pair ID (from README specification)
 */
export function encodeTradingPairID(
    instrType: number, 
    baseClass: number, baseID: number,
    quoteClass: number, quoteID: number, 
    subType: number
): bigint {
    return (BigInt(instrType & 0xF) << 60n) |
           (BigInt(baseClass & 0xF) << 56n) |
           (BigInt(baseID & 0xFFFF) << 40n) |
           (BigInt(quoteClass & 0xF) << 36n) |
           (BigInt(quoteID & 0xFFFF) << 20n) |
           BigInt(subType & 0xFFFFF);
}

/**
 * Example: Create and send a trade message
 */
export async function exampleTradeMessage(): Promise<void> {
    // Create EUR/USD ticker ID (from README spec)
    const eurUsdTickerId = encodeTradingPairID(0x0, 0x3, 0x6F, 0x3, 0x1CD, 0x0);
    
    const timestamp = new Uint8Array(6);
    writeTimestamp48(timestamp, getCurrentTimestampNanos());
    
    const tradeMessage: TradeMessage = {
        header: {
            messageType: MSG_TYPE_TRADE,
            timestamp: timestamp,
            count: 1
        },
        trades: [{
            tickerId: eurUsdTickerId,
            price: 1.0850,
            quantity: 1000000, // 1.0 lot scaled by 1000000
            tradeId: 12345,
            side: SIDE_BUY,
            padding: new Uint8Array(7)
        }]
    };
    
    const buffer = packTradeMessage(tradeMessage);
    console.log(`Packed trade message: ${buffer.length} bytes`);
    
    // To send via TCP:
    // await mitchSendTCP(socket, buffer);
}

/**
 * Example: Create order message with proper type/side encoding
 */
export async function exampleOrderMessage(): Promise<void> {
    const eurUsdTickerId = encodeTradingPairID(0x0, 0x3, 0x6F, 0x3, 0x1CD, 0x0);
    
    const timestamp = new Uint8Array(6);
    writeTimestamp48(timestamp, getCurrentTimestampNanos());
    
    const expiry = new Uint8Array(6);
    // Set expiry to 0 for GTC (Good Till Cancelled)
    
    const orderMessage: OrderMessage = {
        header: {
            messageType: MSG_TYPE_ORDER,
            timestamp: timestamp,
            count: 1
        },
        orders: [{
            tickerId: eurUsdTickerId,
            orderId: 67890,
            price: 1.0840,
            quantity: 500000, // 0.5 lot
            typeAndSide: combineTypeAndSide(ORDER_TYPE_LIMIT, SIDE_BUY),
            expiry: expiry,
            padding: 0
        }]
    };
    
    const buffer = packOrderMessage(orderMessage);
    console.log(`Packed order message: ${buffer.length} bytes`);
}

// Run examples if this file is executed directly
if (typeof require !== 'undefined' && require.main === module) {
    exampleTradeMessage().catch(console.error);
    exampleOrderMessage().catch(console.error);
}
