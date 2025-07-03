#pragma once
#include <stdint.h>

// Enforce byte alignment and packing
#pragma pack(push, 1)

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

// --- Byte Order (All MITCH messages use Big-Endian) ---
#ifdef __BYTE_ORDER__
    #if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
        #define MITCH_NEEDS_BYTESWAP 1
    #else
        #define MITCH_NEEDS_BYTESWAP 0
    #endif
#else
    // Assume little-endian if unknown (most common)
    #define MITCH_NEEDS_BYTESWAP 1
#endif

// --- Unified Message Header (8 bytes) ---
// All MITCH messages start with this header
typedef struct {
    uint8_t message_type;  // ASCII message type code
    uint8_t timestamp[6];  // 48-bit nanoseconds since midnight
    uint8_t count;         // Number of body entries (1-255)
} MitchHeader;

// --- Body Structures (32 bytes each) ---

// TradeBody (32 bytes)
typedef struct {
    uint64_t ticker_id;
    double   price;
    uint32_t quantity;
    uint32_t trade_id;
    uint8_t  side;         // 0: Buy, 1: Sell
    uint8_t  padding[7];   // Padding to 32 bytes
} TradeBody;

// OrderBody (32 bytes)
typedef struct {
    uint64_t ticker_id;
    uint32_t order_id;
    double   price;
    uint32_t quantity;
    uint8_t  type_and_side; // Bit 0: Side, Bits 1-7: Order Type
    uint8_t  expiry[6];
    uint8_t  padding;      // Padding to 32 bytes
} OrderBody;

// TickerBody (32 bytes)
typedef struct {
    uint64_t ticker_id;
    double   bid_price;
    double   ask_price;
    uint32_t bid_volume;
    uint32_t ask_volume;
} TickerBody;

// OrderBookBody (Header: 32 bytes)
// Variable size: 32 bytes header + num_ticks * 4 bytes
typedef struct {
    uint64_t ticker_id;
    double   first_tick;
    double   tick_size;
    uint16_t num_ticks;
    uint8_t  side;         // 0: Bids, 1: Asks
    uint8_t  padding[5];   // Padding to 32 bytes
    // uint32_t volumes[] follows immediately after
} OrderBookBody;

// Volume Entry (4 bytes)
typedef struct {
    uint32_t volume;
} VolumeEntry;

// --- Message Structures ---
// These represent complete messages: Header + Body Array

// Trade Message: Header + TradeBody[]
typedef struct {
    MitchHeader header;
    TradeBody   trades[];  // Flexible array member
} TradeMessage;

// Order Message: Header + OrderBody[]
typedef struct {
    MitchHeader header;
    OrderBody   orders[];  // Flexible array member
} OrderMessage;

// Ticker Message: Header + TickerBody[]
typedef struct {
    MitchHeader header;
    TickerBody  tickers[];  // Flexible array member
} TickerMessage;

// Order Book Message: Header + OrderBookBody[]
// Note: Each OrderBookBody is variable-sized
typedef struct {
    MitchHeader    header;
    OrderBookBody  order_books[];  // Flexible array member
} OrderBookMessage;

// --- Utility Macros ---

// Extract side from type_and_side field
#define EXTRACT_SIDE(type_and_side) ((type_and_side) & 0x01)

// Extract order type from type_and_side field  
#define EXTRACT_ORDER_TYPE(type_and_side) (((type_and_side) >> 1) & 0x7F)

// Combine order type and side into type_and_side field
#define COMBINE_TYPE_AND_SIDE(order_type, side) (((order_type) << 1) | (side))

#pragma pack(pop) 