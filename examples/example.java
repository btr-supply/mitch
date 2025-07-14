package mitch.examples;

import com.mitch.model.MitchModel.*;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.List;

public class Example {

    // === TIMESTAMP UTILITY FUNCTIONS ===

    /**
     * Converts nanoseconds timestamp to 6-byte array for MITCH protocol
     * @param nanos Nanoseconds since midnight (UTC)
     * @return 6-byte timestamp array in big-endian format
     */
    public static byte[] writeTimestamp48(long nanos) {
        byte[] timestamp = new byte[6];
        // Extract 48 bits (6 bytes) from the long value
        timestamp[0] = (byte) ((nanos >> 40) & 0xFF);
        timestamp[1] = (byte) ((nanos >> 32) & 0xFF);
        timestamp[2] = (byte) ((nanos >> 24) & 0xFF);
        timestamp[3] = (byte) ((nanos >> 16) & 0xFF);
        timestamp[4] = (byte) ((nanos >> 8) & 0xFF);
        timestamp[5] = (byte) (nanos & 0xFF);
        return timestamp;
    }

    /**
     * Converts 6-byte timestamp array to nanoseconds
     * @param timestamp 6-byte timestamp array in big-endian format
     * @return Nanoseconds since midnight (UTC)
     */
    public static long readTimestamp48(byte[] timestamp) {
        return ((long) (timestamp[0] & 0xFF) << 40) |
               ((long) (timestamp[1] & 0xFF) << 32) |
               ((long) (timestamp[2] & 0xFF) << 24) |
               ((long) (timestamp[3] & 0xFF) << 16) |
               ((long) (timestamp[4] & 0xFF) << 8) |
               ((long) (timestamp[5] & 0xFF));
    }

    // Simple container for unpacked messages
    public static class UnpackedMessage {
        public final MitchHeader header;
        public final List<Object> bodies;

        public UnpackedMessage(MitchHeader header, List<Object> bodies) {
            this.header = header;
            this.bodies = bodies;
        }
    }

    // Generic packing function using varargs
    public static byte[] pack(byte messageType, Object... bodies) throws IOException {
        if (bodies.length == 0 || bodies.length > 255) {
            throw new IllegalArgumentException("Invalid body count: " + bodies.length);
        }

        MitchHeader header = new MitchHeader();
        header.messageType = messageType;
        header.timestamp = writeTimestamp48(System.nanoTime());
        header.count = bodies.length;

        // Calculate total message size
        int totalSize = 8; // Header size
        for (Object body : bodies) {
            if (body instanceof OrderBook) {
                OrderBook ob = (OrderBook) body;
                totalSize += 32 + (ob.numTicks * 4); // Header + volumes
            } else {
                totalSize += 32; // Fixed-size bodies
            }
        }

        ByteBuffer buffer = ByteBuffer.allocate(totalSize).order(BYTE_ORDER);
        
        // Pack header
        buffer.put(header.messageType).put(header.timestamp).put((byte) header.count);

        // Pack bodies based on type
        for (Object body : bodies) {
            if (body instanceof Trade) {
                Trade t = (Trade) body;
                buffer.putLong(t.tickerId).putDouble(t.price).putInt((int) t.quantity)
                      .putInt((int) t.tradeId).put(t.side).put(t.padding);
            } else if (body instanceof Order) {
                Order o = (Order) body;
                buffer.putLong(o.tickerId).putInt((int) o.orderId).putDouble(o.price)
                      .putInt((int) o.quantity).put(o.typeAndSide).put(o.expiry).put(o.padding);
            } else if (body instanceof Tick) {
                Tick s = (Tick) body;
                buffer.putLong(s.tickerId).putDouble(s.bidPrice).putDouble(s.askPrice)
                      .putInt((int) s.bidVolume).putInt((int) s.askVolume);
            } else if (body instanceof OrderBook) {
                OrderBook ob = (OrderBook) body;
                buffer.putLong(ob.tickerId).putDouble(ob.firstTick).putDouble(ob.tickSize)
                      .putShort((short) ob.numTicks).put(ob.side).put(ob.padding);
                // Pack volume array
                for (long volume : ob.volumes) {
                    buffer.putInt((int) volume);
                }
            } else {
                throw new IllegalArgumentException("Unsupported body type: " + body.getClass().getName());
            }
        }
        return buffer.array();
    }

    // Generic unpacking function
    public static UnpackedMessage unpack(byte[] data) throws IOException {
        if (data.length < 8) throw new IOException("Insufficient data for header");
        
        ByteBuffer buffer = ByteBuffer.wrap(data).order(BYTE_ORDER);
        MitchHeader header = new MitchHeader();
        header.messageType = buffer.get();
        buffer.get(header.timestamp);
        header.count = Byte.toUnsignedInt(buffer.get());

        List<Object> bodies = new ArrayList<>();

        for (int i = 0; i < header.count; i++) {
            switch (header.messageType) {
                case MSG_TYPE_TRADE:
                    Trade trade = new Trade();
                    trade.tickerId = buffer.getLong();
                    trade.price = buffer.getDouble();
                    trade.quantity = Integer.toUnsignedLong(buffer.getInt());
                    trade.tradeId = Integer.toUnsignedLong(buffer.getInt());
                    trade.side = buffer.get();
                    buffer.get(trade.padding);
                    bodies.add(trade);
                    break;
                case MSG_TYPE_ORDER:
                    Order order = new Order();
                    order.tickerId = buffer.getLong();
                    order.orderId = Integer.toUnsignedLong(buffer.getInt());
                    order.price = buffer.getDouble();
                    order.quantity = Integer.toUnsignedLong(buffer.getInt());
                    order.typeAndSide = buffer.get();
                    buffer.get(order.expiry);
                    order.padding = buffer.get();
                    bodies.add(order);
                    break;
                case MSG_TYPE_TICKER:
                    Tick ticker = new Tick();
                    ticker.tickerId = buffer.getLong();
                    ticker.bidPrice = buffer.getDouble();
                    ticker.askPrice = buffer.getDouble();
                    ticker.bidVolume = Integer.toUnsignedLong(buffer.getInt());
                    ticker.askVolume = Integer.toUnsignedLong(buffer.getInt());
                    bodies.add(ticker);
                    break;
                case MSG_TYPE_ORDER_BOOK:
                    OrderBook orderBook = new OrderBook();
                    orderBook.tickerId = buffer.getLong();
                    orderBook.firstTick = buffer.getDouble();
                    orderBook.tickSize = buffer.getDouble();
                    orderBook.numTicks = Short.toUnsignedInt(buffer.getShort());
                    orderBook.side = buffer.get();
                    buffer.get(orderBook.padding);
                    
                    // Read volume array
                    orderBook.volumes = new long[orderBook.numTicks];
                    for (int j = 0; j < orderBook.numTicks; j++) {
                        orderBook.volumes[j] = Integer.toUnsignedLong(buffer.getInt());
                    }
                    bodies.add(orderBook);
                    break;
                default:
                    throw new IOException("Unknown message type: " + (char) header.messageType);
            }
        }
        return new UnpackedMessage(header, bodies);
    }

    // TCP send function
    public static void sendTCP(Socket socket, byte[] data) throws IOException {
        OutputStream out = socket.getOutputStream();
        out.write(data);
        out.flush();
    }
    
    // TCP receive function with proper variable-size message handling
    public static byte[] recvTCP(Socket socket) throws IOException {
        InputStream in = socket.getInputStream();
        
        // Read header first (8 bytes)
        byte[] headerBytes = new byte[8];
        int totalRead = 0;
        while (totalRead < 8) {
            int read = in.read(headerBytes, totalRead, 8 - totalRead);
            if (read == -1) throw new IOException("Connection closed");
            totalRead += read;
        }
        
        // Parse header to determine body size
        byte messageType = headerBytes[0];
        int count = Byte.toUnsignedInt(headerBytes[7]);
        
        int bodySize;
        if (messageType == MSG_TYPE_ORDER_BOOK) {
            // For order book messages, we need to read each entry to determine size
            // This is a simplified approach - in practice, you might want to peek ahead
            throw new UnsupportedOperationException("Order book TCP receive requires advanced parsing");
        } else {
            // Fixed-size messages
            bodySize = count * 32;
        }
        
        // Read body data
        byte[] bodyBytes = new byte[bodySize];
        totalRead = 0;
        while (totalRead < bodySize) {
            int read = in.read(bodyBytes, totalRead, bodySize - totalRead);
            if (read == -1) throw new IOException("Connection closed");
            totalRead += read;
        }
        
        // Combine header and body
        byte[] fullMessage = new byte[8 + bodySize];
        System.arraycopy(headerBytes, 0, fullMessage, 0, 8);
        System.arraycopy(bodyBytes, 0, fullMessage, 8, bodySize);
        
        return fullMessage;
    }

    public static void main(String[] args) throws IOException {
        System.out.println("--- MITCH Java Example ---");

        // Create batch trade message
        Trade trade1 = new Trade(1L, 1.2345, 100L, 1001L, SIDE_BUY);
        Trade trade2 = new Trade(1L, 1.2346, 50L, 1002L, SIDE_SELL);
        
        byte[] tradeMsg = pack(MSG_TYPE_TRADE, trade1, trade2);
        System.out.printf("Packed batch trades (%d bytes): %s\n", 
                         tradeMsg.length, bytesToHex(tradeMsg));

        // Unpack and display trades
        UnpackedMessage unpacked = unpack(tradeMsg);
        System.out.printf("Unpacked: type=%c, count=%d\n", 
                         (char) unpacked.header.messageType, unpacked.header.count);
        
        for (int i = 0; i < unpacked.bodies.size(); i++) {
            Trade t = (Trade) unpacked.bodies.get(i);
            System.out.printf("  Trade %d: ticker=%d, price=%.4f, qty=%d, side=%d\n", 
                             i + 1, t.tickerId, t.price, t.quantity, t.side);
        }

        // Create single order message
        Order order = new Order(2L, 2001L, 98.5, 25L, 
                                       combineTypeAndSide(ORDER_TYPE_LIMIT, SIDE_BUY), 
                                       new byte[6]);
        
        byte[] orderMsg = pack(MSG_TYPE_ORDER, order);
        System.out.printf("\nPacked order (%d bytes): %s\n", 
                         orderMsg.length, bytesToHex(orderMsg));

        // Unpack and display order
        UnpackedMessage unpackedOrder = unpack(orderMsg);
        Order o = (Order) unpackedOrder.bodies.get(0);
        System.out.printf("Unpacked Order: id=%d, type=%d, side=%d, price=%.1f\n",
                         o.orderId, extractOrderType(o.typeAndSide), 
                         extractSide(o.typeAndSide), o.price);

        // Create order book message
        long[] volumes = {1000L, 500L, 250L};
        OrderBook orderBook = new OrderBook(3L, 100.0, 0.01, 3, SIDE_BUY, volumes);
        
        byte[] orderBookMsg = pack(MSG_TYPE_ORDER_BOOK, orderBook);
        System.out.printf("\nPacked order book (%d bytes): %s\n", 
                         orderBookMsg.length, bytesToHex(orderBookMsg));

        // Unpack and display order book
        UnpackedMessage unpackedOrderBook = unpack(orderBookMsg);
        OrderBook ob = (OrderBook) unpackedOrderBook.bodies.get(0);
        System.out.printf("Unpacked Order Book: ticker=%d, firstTick=%.2f, numTicks=%d, side=%d\n",
                         ob.tickerId, ob.firstTick, ob.numTicks, ob.side);
        System.out.print("  Volumes: ");
        for (long vol : ob.volumes) {
            System.out.printf("%d ", vol);
        }
        System.out.println();

        System.out.println("\n--- Example Complete ---");
    }
    
    private static String bytesToHex(byte[] bytes) {
        StringBuilder sb = new StringBuilder();
        for (byte b : bytes) sb.append(String.format("%02x", b));
        return sb.toString();
    }
}
