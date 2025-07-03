package main

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"io"
	"net"
	"time"

	"mt4-forwarder/mitch/model"
)

// === TIMESTAMP UTILITY FUNCTIONS ===

// WriteTimestamp48 converts a 64-bit timestamp to a 48-bit timestamp (6 bytes)
// Input timestamp should be nanoseconds since midnight (UTC)
func WriteTimestamp48(timestamp uint64) [6]byte {
	var result [6]byte
	// Take only the lower 48 bits
	result[0] = byte(timestamp >> 40)
	result[1] = byte(timestamp >> 32)
	result[2] = byte(timestamp >> 24)
	result[3] = byte(timestamp >> 16)
	result[4] = byte(timestamp >> 8)
	result[5] = byte(timestamp)
	return result
}

// ReadTimestamp48 converts a 48-bit timestamp (6 bytes) to a 64-bit timestamp
// Returns nanoseconds since midnight (UTC)
func ReadTimestamp48(timestamp [6]byte) uint64 {
	return uint64(timestamp[0])<<40 |
		uint64(timestamp[1])<<32 |
		uint64(timestamp[2])<<24 |
		uint64(timestamp[3])<<16 |
		uint64(timestamp[4])<<8 |
		uint64(timestamp[5])
}

// === Generic Packing Logic ===

// PackMessage packs a header and a slice of message bodies into a single byte slice.
func PackMessage(messageType byte, bodies ...interface{}) ([]byte, error) {
	if len(bodies) == 0 || len(bodies) > 255 {
		return nil, fmt.Errorf("invalid number of message bodies: %d", len(bodies))
	}

	// Calculate nanoseconds since midnight UTC as per MITCH specification
	now := time.Now().UTC()
	midnight := time.Date(now.Year(), now.Month(), now.Day(), 0, 0, 0, 0, time.UTC)
	nanosSinceMidnight := uint64(now.Sub(midnight).Nanoseconds())

	header := model.MitchHeader{
		MessageType: messageType,
		Timestamp:   WriteTimestamp48(nanosSinceMidnight),
		Count:       uint8(len(bodies)),
	}

	buf := new(bytes.Buffer)
	binary.Write(buf, model.ByteOrder, header)

	for _, body := range bodies {
		err := binary.Write(buf, model.ByteOrder, body)
		if err != nil {
			return nil, fmt.Errorf("failed to pack body: %v", err)
		}
	}

	return buf.Bytes(), nil
}

// UnpackMessage unpacks a byte slice into a header and a slice of message bodies.
func UnpackMessage(data []byte) (*model.MitchHeader, []interface{}, error) {
	if len(data) < 8 {
		return nil, nil, fmt.Errorf("insufficient data for header")
	}

	reader := bytes.NewReader(data)
	header := &model.MitchHeader{}
	if err := binary.Read(reader, model.ByteOrder, header); err != nil {
		return nil, nil, fmt.Errorf("failed to read header: %v", err)
	}

	var bodies []interface{}

	for i := 0; i < int(header.Count); i++ {
		var body interface{}
		switch header.MessageType {
		case model.MsgTypeTrade:
			body = &model.TradeBody{}
		case model.MsgTypeOrder:
			body = &model.OrderBody{}
		case model.MsgTypeTicker:
			body = &model.TickerBody{}
		case model.MsgTypeOrderBook:
			// OrderBook is special, it's variable size, so this generic function is not suitable.
			// A specific function should be used for it.
			return nil, nil, fmt.Errorf("order book unpacking requires a specialized function")
		default:
			return nil, nil, fmt.Errorf("unknown message type: %c", header.MessageType)
		}

		if err := binary.Read(reader, model.ByteOrder, body); err != nil {
			return nil, nil, fmt.Errorf("failed to read body %d: %v", i+1, err)
		}
		bodies = append(bodies, body)
	}

	return header, bodies, nil
}

// === TCP Functions ===

func MitchSendTCP(conn net.Conn, data []byte) error {
	_, err := conn.Write(data)
	return err
}

func MitchRecvTCP(conn net.Conn, length int) ([]byte, error) {
	buf := make([]byte, length)
	if _, err := io.ReadFull(conn, buf); err != nil {
		return nil, err
	}
	return buf, nil
}

func MitchRecvMessage(conn net.Conn) ([]byte, error) {
	headerData, err := MitchRecvTCP(conn, 8)
	if err != nil {
		return nil, err
	}

	header := &model.MitchHeader{}
	if err := binary.Read(bytes.NewReader(headerData), model.ByteOrder, header); err != nil {
		return nil, fmt.Errorf("failed to decode header: %v", err)
	}

	bodyLength := int(header.Count) * 32
	// This simple logic does not handle variable size messages like OrderBook.
	// A more robust implementation would peek at message type for special handling.

	bodyData, err := MitchRecvTCP(conn, bodyLength)
	if err != nil {
		return nil, err
	}

	return append(headerData, bodyData...), nil
}

// === Example Usage ===

func main() {
	fmt.Println("--- Running MITCH Go Example ---")

	// 1. Create and pack a batch of two trades
	trade1 := model.TradeBody{
		TickerID: 1,
		Price:    1.2345,
		Quantity: 100,
		TradeID:  1001,
		Side:     model.SideBuy,
	}
	trade2 := model.TradeBody{
		TickerID: 1,
		Price:    1.2346,
		Quantity: 50,
		TradeID:  1002,
		Side:     model.SideSell,
	}

	tradeMessage, err := PackMessage(model.MsgTypeTrade, trade1, trade2)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Packed batch trade message (%d bytes): %x\n", len(tradeMessage), tradeMessage)

	// 2. Unpack the batch of trades
	unpackedHeader, unpackedBodies, err := UnpackMessage(tradeMessage)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Unpacked Header: %+v\n", unpackedHeader)
	for i, body := range unpackedBodies {
		if t, ok := body.(*model.TradeBody); ok {
			fmt.Printf("Unpacked Trade %d: %+v\n", i+1, t)
		}
	}

	// 3. Create and pack a single order
	order := model.OrderBody{
		TickerID:    2,
		OrderID:     2001,
		Price:       98.5,
		Quantity:    25,
		TypeAndSide: model.CombineTypeAndSide(model.OrderTypeLimit, model.SideBuy),
	}

	orderMessage, err := PackMessage(model.MsgTypeOrder, order)
	if err != nil {
		panic(err)
	}
	fmt.Printf("\nPacked order message (%d bytes): %x\n", len(orderMessage), orderMessage)

	// 4. Unpack the single order
	unpackedHeader, unpackedBodies, err = UnpackMessage(orderMessage)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Unpacked Header: %+v\n", unpackedHeader)
	if o, ok := unpackedBodies[0].(*model.OrderBody); ok {
		fmt.Printf("Unpacked Order: %+v\n", o)
		fmt.Printf("  -> Extracted Side: %d, Type: %d\n", model.ExtractSide(o.TypeAndSide), model.ExtractOrderType(o.TypeAndSide))
	}

	fmt.Println("\n--- Example Complete ---")
}
