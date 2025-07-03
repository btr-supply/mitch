//+------------------------------------------------------------------+
//|                                                        model.mq4 |
//|                   Copyright 2023, Built Trough Research          |
//|                               https://www.builttorough.com       |
//+------------------------------------------------------------------+
#property strict

// --- Message Type Codes ---
#define MITCH_MSG_TYPE_TRADE        't'
#define MITCH_MSG_TYPE_ORDER        'o'
#define MITCH_MSG_TYPE_TICKER       's'
#define MITCH_MSG_TYPE_ORDER_BOOK   'q'

// --- Side Constants ---
#define MITCH_SIDE_BUY              0
#define MITCH_SIDE_SELL             1

// --- Order Type Constants ---
#define MITCH_ORDER_TYPE_MARKET     0
#define MITCH_ORDER_TYPE_LIMIT      1
#define MITCH_ORDER_TYPE_STOP       2
#define MITCH_ORDER_TYPE_CANCEL     3

// --- Unified Message Header (8 bytes) ---
struct MitchHeader
{
   uchar   messageType;      // ASCII message type code
   uchar   timestamp[6];     // 48-bit nanoseconds since midnight
   uchar   count;            // Number of body entries (1-255)
};

// --- Body Structures (32 bytes each) ---

// TradeBody (32 bytes)
struct TradeBody
{
    ulong tickerId;
    double price;
    uint quantity;
    uint tradeId;
    uchar side;             // 0: Buy, 1: Sell
    uchar padding[7];       // Padding to 32 bytes
};

// OrderBody (32 bytes)
struct OrderBody
{
    ulong tickerId;
    uint orderId;
    double price;
    uint quantity;
    uchar typeAndSide;      // Bit 0: Side, Bits 1-7: Order Type
    uchar expiry[6];
    uchar padding;          // Padding to 32 bytes
};

// TickerBody (32 bytes)
struct TickerBody
{
   ulong   tickerId;
   double  bidPrice;
   double  askPrice;
   uint    bidVolume;
   uint    askVolume;
};

// OrderBookBody (Header: 32 bytes)
// Variable size: 32 bytes header + numTicks * 4 bytes
struct OrderBookBody
{
    ulong tickerId;
    double firstTick;
    double tickSize;
    ushort numTicks;
    uchar side;             // 0: Bids, 1: Asks
    uchar padding[5];       // Padding to 32 bytes
    // uint volumes[] follows
};

//+------------------------------------------------------------------+
//| Utility Functions                                               |
//+------------------------------------------------------------------+

// Extract side from type_and_side field
uchar ExtractSide(uchar typeAndSide)
{
    return typeAndSide & 0x01;
}

// Extract order type from type_and_side field
uchar ExtractOrderType(uchar typeAndSide)
{
    return (typeAndSide >> 1) & 0x7F;
}

// Combine order type and side into type_and_side field
uchar CombineTypeAndSide(uchar orderType, uchar side)
{
    return (orderType << 1) | side;
}
