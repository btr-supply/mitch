#include <stdint.h>
#include <string.h>
#include <arpa/inet.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>
#include <time.h>

// Include the model header
#include "../model/model.h"

// Helper functions for endianness conversion

// Helper function to convert double to big-endian bytes
static void write_double_be(uint8_t *dest, double value) {
    uint64_t bits;
    memcpy(&bits, &value, 8);
    bits = htobe64(bits);
    memcpy(dest, &bits, 8);
}

// Helper function to read double from big-endian bytes
static double read_double_be(const uint8_t *src) {
    uint64_t bits;
    memcpy(&bits, src, 8);
    bits = be64toh(bits);
    double value;
    memcpy(&value, &bits, 8);
    return value;
}

static uint64_t get_timestamp_ns() {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + ts.tv_nsec;
}

// === PACKING FUNCTIONS ===

int pack_header(const MitchHeader *header, uint8_t *buffer) {
    buffer[0] = header->message_type;
    memcpy(buffer + 1, header->timestamp, 6);
    buffer[7] = header->count;
    return 8;
}

int pack_trade_body(const Trade *trade, uint8_t *buffer) {
    uint8_t *ptr = buffer;
    
    uint64_t ticker_id = htobe64(trade->ticker_id);
    memcpy(ptr, &ticker_id, 8); ptr += 8;
    
    write_double_be(ptr, trade->price); ptr += 8;
    
    uint32_t quantity = htonl(trade->quantity);
    memcpy(ptr, &quantity, 4); ptr += 4;
    
    uint32_t trade_id = htonl(trade->trade_id);
    memcpy(ptr, &trade_id, 4); ptr += 4;
    
    *ptr++ = trade->side;
    memset(ptr, 0, 7); // padding
    
    return 32;
}

int pack_order_body(const Order *order, uint8_t *buffer) {
    uint8_t *ptr = buffer;
    
    uint64_t ticker_id = htobe64(order->ticker_id);
    memcpy(ptr, &ticker_id, 8); ptr += 8;
    
    uint32_t order_id = htonl(order->order_id);
    memcpy(ptr, &order_id, 4); ptr += 4;
    
    write_double_be(ptr, order->price); ptr += 8;
    
    uint32_t quantity = htonl(order->quantity);
    memcpy(ptr, &quantity, 4); ptr += 4;
    
    *ptr++ = order->type_and_side;
    memcpy(ptr, order->expiry, 6); ptr += 6;
    *ptr = order->padding;
    
    return 32;
}

int pack_ticker_body(const Tick *ticker, uint8_t *buffer) {
    uint8_t *ptr = buffer;
    
    uint64_t ticker_id = htobe64(ticker->ticker_id);
    memcpy(ptr, &ticker_id, 8); ptr += 8;
    
    write_double_be(ptr, ticker->bid_price); ptr += 8;
    write_double_be(ptr, ticker->ask_price); ptr += 8;
    
    uint32_t bid_volume = htonl(ticker->bid_volume);
    memcpy(ptr, &bid_volume, 4); ptr += 4;
    
    uint32_t ask_volume = htonl(ticker->ask_volume);
    memcpy(ptr, &ask_volume, 4);
    
    return 32;
}

int pack_order_book_body(const OrderBook *order_book, const uint32_t *volumes, uint8_t *buffer) {
    uint8_t *ptr = buffer;
    
    uint64_t ticker_id = htobe64(order_book->ticker_id);
    memcpy(ptr, &ticker_id, 8); ptr += 8;
    
    write_double_be(ptr, order_book->first_tick); ptr += 8;
    write_double_be(ptr, order_book->tick_size); ptr += 8;
    
    uint16_t num_ticks = htons(order_book->num_ticks);
    memcpy(ptr, &num_ticks, 2); ptr += 2;
    
    *ptr++ = order_book->side;
    memset(ptr, 0, 5); ptr += 5; // padding
    
    // Pack volumes
    for (int i = 0; i < order_book->num_ticks; i++) {
        uint32_t volume = htonl(volumes[i]);
        memcpy(ptr, &volume, 4);
        ptr += 4;
    }
    
    return 32 + order_book->num_ticks * 4;
}

// === UNPACKING FUNCTIONS ===

int unpack_header(const uint8_t *buffer, MitchHeader *header) {
    header->message_type = buffer[0];
    memcpy(header->timestamp, buffer + 1, 6);
    header->count = buffer[7];
    return 8;
}

int unpack_trade_body(const uint8_t *buffer, Trade *trade) {
    const uint8_t *ptr = buffer;
    
    memcpy(&trade->ticker_id, ptr, 8);
    trade->ticker_id = be64toh(trade->ticker_id); ptr += 8;
    
    trade->price = read_double_be(ptr); ptr += 8;
    
    memcpy(&trade->quantity, ptr, 4);
    trade->quantity = ntohl(trade->quantity); ptr += 4;
    
    memcpy(&trade->trade_id, ptr, 4);
    trade->trade_id = ntohl(trade->trade_id); ptr += 4;
    
    trade->side = *ptr++;
    memcpy(trade->padding, ptr, 7);
    
    return 32;
}

int unpack_order_body(const uint8_t *buffer, Order *order) {
    const uint8_t *ptr = buffer;
    
    memcpy(&order->ticker_id, ptr, 8);
    order->ticker_id = be64toh(order->ticker_id); ptr += 8;
    
    memcpy(&order->order_id, ptr, 4);
    order->order_id = ntohl(order->order_id); ptr += 4;
    
    order->price = read_double_be(ptr); ptr += 8;
    
    memcpy(&order->quantity, ptr, 4);
    order->quantity = ntohl(order->quantity); ptr += 4;
    
    order->type_and_side = *ptr++;
    memcpy(order->expiry, ptr, 6); ptr += 6;
    order->padding = *ptr;
    
    return 32;
}

int unpack_ticker_body(const uint8_t *buffer, Tick *ticker) {
    const uint8_t *ptr = buffer;
    
    memcpy(&ticker->ticker_id, ptr, 8);
    ticker->ticker_id = be64toh(ticker->ticker_id); ptr += 8;
    
    ticker->bid_price = read_double_be(ptr); ptr += 8;
    ticker->ask_price = read_double_be(ptr); ptr += 8;
    
    memcpy(&ticker->bid_volume, ptr, 4);
    ticker->bid_volume = ntohl(ticker->bid_volume); ptr += 4;
    
    memcpy(&ticker->ask_volume, ptr, 4);
    ticker->ask_volume = ntohl(ticker->ask_volume);
    
    return 32;
}

int unpack_order_book_body(const uint8_t *buffer, OrderBook *order_book, uint32_t *volumes) {
    const uint8_t *ptr = buffer;
    
    memcpy(&order_book->ticker_id, ptr, 8);
    order_book->ticker_id = be64toh(order_book->ticker_id); ptr += 8;
    
    order_book->first_tick = read_double_be(ptr); ptr += 8;
    order_book->tick_size = read_double_be(ptr); ptr += 8;
    
    memcpy(&order_book->num_ticks, ptr, 2);
    order_book->num_ticks = ntohs(order_book->num_ticks); ptr += 2;
    
    order_book->side = *ptr++;
    memcpy(order_book->padding, ptr, 5); ptr += 5;
    
    // Unpack volumes
    for (int i = 0; i < order_book->num_ticks; i++) {
        memcpy(&volumes[i], ptr, 4);
        volumes[i] = ntohl(volumes[i]);
        ptr += 4;
    }
    
    return 32 + order_book->num_ticks * 4;
}

// === TCP SEND/RECV FUNCTIONS ===

int mitch_send_tcp(int socket, const uint8_t *data, size_t length) {
    size_t total_sent = 0;
    while (total_sent < length) {
        ssize_t sent = send(socket, data + total_sent, length - total_sent, 0);
        if (sent <= 0) {
            return -1; // Error
        }
        total_sent += sent;
    }
    return 0;
}

int mitch_recv_tcp(int socket, uint8_t *buffer, size_t length) {
    size_t total_received = 0;
    while (total_received < length) {
        ssize_t received = recv(socket, buffer + total_received, length - total_received, 0);
        if (received <= 0) {
            return -1; // Error or connection closed
        }
        total_received += received;
    }
    return 0;
}

int mitch_recv_message(int socket, uint8_t *buffer, size_t buffer_size) {
    // First receive the 8-byte header
    if (mitch_recv_tcp(socket, buffer, 8) != 0) {
        return -1;
    }

    MitchHeader header;
    unpack_header(buffer, &header);

    size_t body_size;

    if (header.message_type == MITCH_MSG_TYPE_ORDER_BOOK) {
        if (header.count != 1) {
            return -3; // Order book messages should have count = 1
        }

        // Read the fixed part of the order book body (32 bytes)
        if (mitch_recv_tcp(socket, buffer + 8, 32) != 0) {
            return -1;
        }

        // Safely extract num_ticks (at offset 24 in the body)
        uint16_t num_ticks_be;
        memcpy(&num_ticks_be, buffer + 8 + 24, 2);
        uint16_t num_ticks = ntohs(num_ticks_be);

        size_t volumes_size = (size_t)num_ticks * 4;
        body_size = 32 + volumes_size;

        if (8 + body_size > buffer_size) {
            return -2; // Buffer too small
        }

        // Read the variable part (volumes)
        if (volumes_size > 0) {
            if (mitch_recv_tcp(socket, buffer + 8 + 32, volumes_size) != 0) {
                return -1;
            }
        }
    } else {
        body_size = (size_t)header.count * 32;
        if (8 + body_size > buffer_size) {
            return -2; // Buffer too small
        }

        // Receive the body of the message
        if (body_size > 0) {
            if (mitch_recv_tcp(socket, buffer + 8, body_size) != 0) {
                return -1;
            }
        }
    }

    return 8 + body_size;
}

// === EXAMPLE USAGE ===

// --- Utility Functions (for consistency with other implementations) ---

// Write 48-bit timestamp from nanoseconds
static void write_timestamp_48(uint8_t *dest, uint64_t nanos) {
    dest[0] = (nanos >> 40) & 0xFF;
    dest[1] = (nanos >> 32) & 0xFF;
    dest[2] = (nanos >> 24) & 0xFF;
    dest[3] = (nanos >> 16) & 0xFF;
    dest[4] = (nanos >> 8) & 0xFF;
    dest[5] = nanos & 0xFF;
}

// Read 48-bit timestamp to nanoseconds
static uint64_t read_timestamp_48(const uint8_t *src) {
    return ((uint64_t)src[0] << 40) |
           ((uint64_t)src[1] << 32) |
           ((uint64_t)src[2] << 24) |
           ((uint64_t)src[3] << 16) |
           ((uint64_t)src[4] << 8) |
           (uint64_t)src[5];
}

// Combine order type and side into single byte
static uint8_t combine_type_and_side(uint8_t order_type, uint8_t side) {
    return COMBINE_TYPE_AND_SIDE(order_type, side);
}

// Extract side from type_and_side field
static uint8_t extract_side(uint8_t type_and_side) {
    return EXTRACT_SIDE(type_and_side);
}

// Extract order type from type_and_side field
static uint8_t extract_order_type(uint8_t type_and_side) {
    return EXTRACT_ORDER_TYPE(type_and_side);
}

void example_usage() {
    // Create a trade message
    MitchHeader header = {
        .message_type = MITCH_MSG_TYPE_TRADE,
        .count = 1
    };
    uint64_t timestamp = get_timestamp_ns();
    write_timestamp_48(header.timestamp, timestamp);
    
    Trade trade = {
        .ticker_id = 0x00006F001CD00000ULL, // EUR/USD
        .price = 1.0850,
        .quantity = 1000000, // 1.0 lot scaled by 1000000
        .trade_id = 12345,
        .side = MITCH_SIDE_BUY
    };
    
    uint8_t buffer[40];
    int offset = 0;
    
    offset += pack_header(&header, buffer + offset);
    offset += pack_trade_body(&trade, buffer + offset);
    
    // Now buffer contains the complete MITCH trade message
    // Send via TCP: mitch_send_tcp(socket, buffer, offset);
}
