package com.mitch.model;

import java.nio.ByteOrder;

/**
 * MITCH Protocol Model - Java Implementation
 * Defines all message structures for the MITCH binary protocol.
 * All messages use big-endian byte order for network compatibility.
 */
public class MitchModel {

    // --- Message Type Codes ---
    public static final byte MSG_TYPE_TRADE = (byte) 't';
    public static final byte MSG_TYPE_ORDER = (byte) 'o';
    public static final byte MSG_TYPE_TICKER = (byte) 's';
    public static final byte MSG_TYPE_ORDER_BOOK = (byte) 'q';

    // --- Side Constants ---
    public static final byte SIDE_BUY = 0;
    public static final byte SIDE_SELL = 1;

    // --- Order Type Constants ---
    public static final byte ORDER_TYPE_MARKET = 0;
    public static final byte ORDER_TYPE_LIMIT = 1;
    public static final byte ORDER_TYPE_STOP = 2;
    public static final byte ORDER_TYPE_CANCEL = 3;

    // All MITCH messages use big-endian byte order
    public static final ByteOrder BYTE_ORDER = ByteOrder.BIG_ENDIAN;

    // --- Unified Message Header (8 bytes) ---
    public static class MitchHeader {
        public byte messageType;    // ASCII message type code
        public byte[] timestamp;    // 6-byte nanoseconds since midnight (u48)
        public int count;          // Number of body entries (1-255) - using int to handle unsigned byte

        public MitchHeader() {
            this.timestamp = new byte[6];
        }

        public MitchHeader(byte messageType, byte[] timestamp, int count) {
            this.messageType = messageType;
            this.timestamp = timestamp.clone();
            this.count = count;
        }
    }

    // --- Body Structures (32 bytes each) ---

    // TradeBody (32 bytes)
    public static class TradeBody {
        public long tickerId;      // u64
        public double price;       // f64
        public long quantity;      // u32 - using long to handle unsigned int
        public long tradeId;       // u32 - using long to handle unsigned int
        public byte side;          // u8: 0: Buy, 1: Sell
        public byte[] padding = new byte[7]; // Padding to 32 bytes

        public TradeBody() {}

        public TradeBody(long tickerId, double price, long quantity, long tradeId, byte side) {
            this.tickerId = tickerId;
            this.price = price;
            this.quantity = quantity;
            this.tradeId = tradeId;
            this.side = side;
        }
    }

    // OrderBody (32 bytes)
    public static class OrderBody {
        public long tickerId;      // u64
        public long orderId;       // u32 - using long to handle unsigned int
        public double price;       // f64
        public long quantity;      // u32 - using long to handle unsigned int
        public byte typeAndSide;   // u8: Bit 0: Side, Bits 1-7: Order Type
        public byte[] expiry;      // 6-byte expiry timestamp (u48)
        public byte padding;       // Padding to 32 bytes

        public OrderBody() {
            this.expiry = new byte[6];
        }

        public OrderBody(long tickerId, long orderId, double price, long quantity, 
                        byte typeAndSide, byte[] expiry) {
            this.tickerId = tickerId;
            this.orderId = orderId;
            this.price = price;
            this.quantity = quantity;
            this.typeAndSide = typeAndSide;
            this.expiry = expiry.clone();
        }
    }

    // TickerBody (32 bytes)
    public static class TickerBody {
        public long tickerId;      // u64
        public double bidPrice;    // f64
        public double askPrice;    // f64
        public long bidVolume;     // u32 - using long to handle unsigned int
        public long askVolume;     // u32 - using long to handle unsigned int

        public TickerBody() {}

        public TickerBody(long tickerId, double bidPrice, double askPrice, 
                         long bidVolume, long askVolume) {
            this.tickerId = tickerId;
            this.bidPrice = bidPrice;
            this.askPrice = askPrice;
            this.bidVolume = bidVolume;
            this.askVolume = askVolume;
        }
    }

    // OrderBookBody (Header: 32 bytes)
    // Variable size: 32 bytes header + numTicks * 4 bytes
    public static class OrderBookBody {
        public long tickerId;      // u64
        public double firstTick;   // f64
        public double tickSize;    // f64
        public int numTicks;       // u16 - using int to handle unsigned short
        public byte side;          // u8: 0: Bids, 1: Asks
        public byte[] padding = new byte[5]; // Padding to 32 bytes
        public long[] volumes;     // u32[] - volume array, using long[] to handle unsigned ints

        public OrderBookBody() {
            this.volumes = new long[0];
        }

        public OrderBookBody(long tickerId, double firstTick, double tickSize, 
                           int numTicks, byte side, long[] volumes) {
            this.tickerId = tickerId;
            this.firstTick = firstTick;
            this.tickSize = tickSize;
            this.numTicks = numTicks;
            this.side = side;
            this.volumes = volumes.clone();
        }
    }

    // --- Utility Methods ---

    /**
     * Extracts the side from a type_and_side field
     * @param typeAndSide Combined type and side byte
     * @return Side (0: Buy, 1: Sell)
     */
    public static byte extractSide(byte typeAndSide) {
        return (byte) (typeAndSide & 0x01);
    }

    /**
     * Extracts the order type from a type_and_side field
     * @param typeAndSide Combined type and side byte
     * @return Order type (0-127)
     */
    public static byte extractOrderType(byte typeAndSide) {
        return (byte) ((typeAndSide >> 1) & 0x7F);
    }

    /**
     * Combines order type and side into a single field
     * @param orderType Order type (0-127)
     * @param side Side (0: Buy, 1: Sell)
     * @return Combined type and side byte
     */
    public static byte combineTypeAndSide(byte orderType, byte side) {
        return (byte) ((orderType << 1) | side);
    }
}
