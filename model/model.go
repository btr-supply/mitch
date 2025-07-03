package model

import "encoding/binary"

// Message type constants
const (
	MsgTypeTrade     = 't'
	MsgTypeOrder     = 'o'
	MsgTypeTicker    = 's'
	MsgTypeOrderBook = 'q'
)

// Side constants
const (
	SideBuy  = 0
	SideSell = 1
)

// Order type constants
const (
	OrderTypeMarket = 0
	OrderTypeLimit  = 1
	OrderTypeStop   = 2
	OrderTypeCancel = 3
)

// ByteOrder is Big Endian for all MITCH messages
var ByteOrder = binary.BigEndian

// --- Unified Message Header (8 bytes) ---
type MitchHeader struct {
	MessageType byte
	Timestamp   [6]byte
	Count       uint8
}

// --- Body Structures (32 bytes each) ---

// TradeBody defines a trade body (32 bytes)
type TradeBody struct {
	TickerID uint64
	Price    float64
	Quantity uint32
	TradeID  uint32
	Side     uint8 // 0: Buy, 1: Sell
	Padding  [7]byte
}

// OrderBody defines an order body (32 bytes)
type OrderBody struct {
	TickerID    uint64
	OrderID     uint32
	Price       float64
	Quantity    uint32
	TypeAndSide uint8 // Bit 0: Side, Bits 1-7: Order Type
	Expiry      [6]byte
	Padding     byte
}

// TickerBody defines a ticker body (32 bytes)
type TickerBody struct {
	TickerID  uint64
	BidPrice  float64
	AskPrice  float64
	BidVolume uint32
	AskVolume uint32
}

// OrderBookBody defines an order book body (variable size)
// Size: 32 bytes header + NumTicks * 4 bytes
type OrderBookBody struct {
	TickerID  uint64
	FirstTick float64
	TickSize  float64
	NumTicks  uint16
	Side      uint8 // 0: Bids, 1: Asks
	Padding   [5]byte
	// Volumes []uint32 follows
}

// --- Utility Functions ---

// ExtractSide extracts the side from a type_and_side field
func ExtractSide(typeAndSide uint8) uint8 {
	return typeAndSide & 0x01
}

// ExtractOrderType extracts the order type from a type_and_side field
func ExtractOrderType(typeAndSide uint8) uint8 {
	return (typeAndSide >> 1) & 0x7F
}

// CombineTypeAndSide combines order type and side into a single field
func CombineTypeAndSide(orderType, side uint8) uint8 {
	return (orderType << 1) | side
}
