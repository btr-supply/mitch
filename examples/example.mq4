//+------------------------------------------------------------------+
//|                                                example.mq4       |
//|                               https://www.builttorough.com       |
//+------------------------------------------------------------------+
#property strict

#include "../model/model.mq4"

//+------------------------------------------------------------------+
//| Timestamp Utility Functions                                     |
//+------------------------------------------------------------------+

// Write 64-bit timestamp to 48-bit (6-byte) array
void WriteTimestamp48(ulong timestamp, uchar &output[6])
{
    // Take only the lower 48 bits (6 bytes) 
    output[0] = (uchar)((timestamp >> 40) & 0xFF);
    output[1] = (uchar)((timestamp >> 32) & 0xFF);
    output[2] = (uchar)((timestamp >> 24) & 0xFF);
    output[3] = (uchar)((timestamp >> 16) & 0xFF);
    output[4] = (uchar)((timestamp >> 8) & 0xFF);
    output[5] = (uchar)(timestamp & 0xFF);
}

// Read 48-bit (6-byte) timestamp to 64-bit
ulong ReadTimestamp48(const uchar &input[6])
{
    return ((ulong)input[0] << 40) |
           ((ulong)input[1] << 32) |
           ((ulong)input[2] << 24) |
           ((ulong)input[3] << 16) |
           ((ulong)input[4] << 8) |
           ((ulong)input[5]);
}

// Convert double to 64-bit representation (IEEE 754)
// NOTE: This is a simplified implementation due to MQL4 limitations.
// For production use, consider using external libraries or DLL calls for precise IEEE 754 conversion.
ulong DoubleToBits(double value)
{
    // MQL4 doesn't have direct bit manipulation for doubles
    // We'll use a union-like approach with arrays
    uchar bytes[8];
    string str = DoubleToString(value, 16); // High precision
    double reconstructed = StringToDouble(str);
    
    // Since MQL4 lacks direct double-to-bits conversion,
    // we'll use a memory copy approach
    if(value == 0.0) return 0;
    if(value != value) return 0x7FF8000000000000; // NaN
    if(value == EMPTY_VALUE) return 0x7FF0000000000000; // Positive infinity
    
    // For normal values, we need to manually extract the IEEE 754 components
    // This is a simplified version - for production use, consider a more robust implementation
    bool isNegative = (value < 0);
    if(isNegative) value = -value;
    
    int exponent = 0;
    if(value >= 1.0)
    {
        while(value >= 2.0) { value /= 2.0; exponent++; }
    }
    else if(value < 1.0 && value > 0.0)
    {
        while(value < 1.0) { value *= 2.0; exponent--; }
    }
    
    // IEEE 754 double: 1 sign bit + 11 exponent bits + 52 mantissa bits
    ulong sign = isNegative ? 1UL : 0UL;
    ulong exp = (ulong)(exponent + 1023) & 0x7FF; // Bias of 1023
    ulong mantissa = (ulong)((value - 1.0) * (1UL << 52)) & 0xFFFFFFFFFFFFF;
    
    return (sign << 63) | (exp << 52) | mantissa;
}

// Convert 64-bit representation to double (IEEE 754)
// NOTE: This is a simplified implementation due to MQL4 limitations.
// For production use, consider using external libraries or DLL calls for precise IEEE 754 conversion.
double BitsToDouble(ulong bits)
{
    // Extract IEEE 754 components
    bool isNegative = (bits >> 63) != 0;
    int exponent = (int)((bits >> 52) & 0x7FF) - 1023;
    ulong mantissa = bits & 0xFFFFFFFFFFFFF;
    
    // Handle special cases
    if(exponent == 1024) // Infinity or NaN
    {
        if(mantissa == 0)
            return isNegative ? -EMPTY_VALUE : EMPTY_VALUE; // Infinity
        else
            return 0.0/0.0; // NaN (though MQL4 may not handle this well)
    }
    
    if(exponent == -1023 && mantissa == 0)
        return isNegative ? -0.0 : 0.0; // Zero
    
    // Normal number: (1 + mantissa/2^52) * 2^exponent
    double value = 1.0 + (double)mantissa / (1UL << 52);
    value *= MathPow(2.0, exponent);
    
    return isNegative ? -value : value;
}

// === PACKING FUNCTIONS ===

int PackHeader(const MitchHeader &header, uchar &buffer[])
{
   ArrayResize(buffer, 8);
   buffer[0] = header.messageType;
   ArrayCopy(buffer, header.timestamp, 1, 0, 6);
   buffer[7] = header.count;
   return 8;
}

int PackTradeBody(const TradeBody &trade, uchar &buffer[])
{
   ArrayResize(buffer, 32);
   
   // Pack ticker_id (8 bytes, big-endian)
   for(int i = 0; i < 8; i++)
      buffer[i] = (uchar)((trade.tickerId >> (56 - i * 8)) & 0xFF);
   
   // Pack price (8 bytes, big-endian double)
   ulong priceBits = DoubleToBits(trade.price);
   for(int i = 0; i < 8; i++)
      buffer[8 + i] = (uchar)((priceBits >> (56 - i * 8)) & 0xFF);
   
   // Pack quantity (4 bytes, big-endian)
   for(int i = 0; i < 4; i++)
      buffer[16 + i] = (uchar)((trade.quantity >> (24 - i * 8)) & 0xFF);
   
   // Pack trade_id (4 bytes, big-endian)
   for(int i = 0; i < 4; i++)
      buffer[20 + i] = (uchar)((trade.tradeId >> (24 - i * 8)) & 0xFF);
   
   buffer[24] = trade.side;
   
   // Padding (7 bytes)
   for(int i = 25; i < 32; i++)
      buffer[i] = 0;
   
   return 32;
}

int PackOrderBody(const OrderBody &order, uchar &buffer[])
{
   ArrayResize(buffer, 32);
   
   // Pack ticker_id (8 bytes, big-endian)
   for(int i = 0; i < 8; i++)
      buffer[i] = (uchar)((order.tickerId >> (56 - i * 8)) & 0xFF);
   
   // Pack order_id (4 bytes, big-endian)
   for(int i = 0; i < 4; i++)
      buffer[8 + i] = (uchar)((order.orderId >> (24 - i * 8)) & 0xFF);
   
   // Pack price (8 bytes, big-endian double)
   ulong priceBits = DoubleToBits(order.price);
   for(int i = 0; i < 8; i++)
      buffer[12 + i] = (uchar)((priceBits >> (56 - i * 8)) & 0xFF);
   
   // Pack quantity (4 bytes, big-endian)
   for(int i = 0; i < 4; i++)
      buffer[20 + i] = (uchar)((order.quantity >> (24 - i * 8)) & 0xFF);
   
   buffer[24] = order.typeAndSide;
   ArrayCopy(buffer, order.expiry, 25, 0, 6);
   buffer[31] = order.padding;
   
   return 32;
}

int PackTickerBody(const TickerBody &ticker, uchar &buffer[])
{
   ArrayResize(buffer, 32);
   
   // Pack ticker_id (8 bytes, big-endian)
   for(int i = 0; i < 8; i++)
      buffer[i] = (uchar)((ticker.tickerId >> (56 - i * 8)) & 0xFF);
   
   // Pack bid_price (8 bytes, big-endian double)
   ulong bidBits = DoubleToBits(ticker.bidPrice);
   for(int i = 0; i < 8; i++)
      buffer[8 + i] = (uchar)((bidBits >> (56 - i * 8)) & 0xFF);
   
   // Pack ask_price (8 bytes, big-endian double)
   ulong askBits = DoubleToBits(ticker.askPrice);
   for(int i = 0; i < 8; i++)
      buffer[16 + i] = (uchar)((askBits >> (56 - i * 8)) & 0xFF);
   
   // Pack bid_volume (4 bytes, big-endian)
   for(int i = 0; i < 4; i++)
      buffer[24 + i] = (uchar)((ticker.bidVolume >> (24 - i * 8)) & 0xFF);
   
   // Pack ask_volume (4 bytes, big-endian)
   for(int i = 0; i < 4; i++)
      buffer[28 + i] = (uchar)((ticker.askVolume >> (24 - i * 8)) & 0xFF);
   
   return 32;
}

int PackOrderBookBody(const OrderBookBody &orderBook, const uint &volumes[], uchar &buffer[])
{
   int totalSize = 32 + orderBook.numTicks * 4;
   ArrayResize(buffer, totalSize);
   
   // Pack ticker_id (8 bytes, big-endian)
   for(int i = 0; i < 8; i++)
      buffer[i] = (uchar)((orderBook.tickerId >> (56 - i * 8)) & 0xFF);
   
   // Pack first_tick (8 bytes, big-endian double)
   ulong firstTickBits = DoubleToBits(orderBook.firstTick);
   for(int i = 0; i < 8; i++)
      buffer[8 + i] = (uchar)((firstTickBits >> (56 - i * 8)) & 0xFF);
   
   // Pack tick_size (8 bytes, big-endian double)
   ulong tickSizeBits = DoubleToBits(orderBook.tickSize);
   for(int i = 0; i < 8; i++)
      buffer[16 + i] = (uchar)((tickSizeBits >> (56 - i * 8)) & 0xFF);
   
   // Pack num_ticks (2 bytes, big-endian)
   buffer[24] = (uchar)((orderBook.numTicks >> 8) & 0xFF);
   buffer[25] = (uchar)(orderBook.numTicks & 0xFF);
   
   buffer[26] = orderBook.side;
   
   // Padding (5 bytes)
   for(int i = 27; i < 32; i++)
      buffer[i] = 0;
   
   // Pack volumes array
   for(int i = 0; i < orderBook.numTicks; i++)
   {
      int offset = 32 + i * 4;
      for(int j = 0; j < 4; j++)
         buffer[offset + j] = (uchar)((volumes[i] >> (24 - j * 8)) & 0xFF);
   }
   
   return totalSize;
}

// === UNPACKING FUNCTIONS ===

bool UnpackHeader(const uchar &buffer[], MitchHeader &header)
{
   if(ArraySize(buffer) < 8) return false;
   
   header.messageType = buffer[0];
   ArrayCopy(header.timestamp, buffer, 0, 1, 6);
   header.count = buffer[7];
   
   return true;
}

bool UnpackTradeBody(const uchar &buffer[], TradeBody &trade)
{
   if(ArraySize(buffer) < 32) return false;
   
   // Unpack ticker_id (8 bytes, big-endian)
   trade.tickerId = 0;
   for(int i = 0; i < 8; i++)
      trade.tickerId |= ((ulong)buffer[i] << (56 - i * 8));
   
   // Unpack price (8 bytes, big-endian double)
   ulong priceBits = 0;
   for(int i = 0; i < 8; i++)
      priceBits |= ((ulong)buffer[8 + i] << (56 - i * 8));
   trade.price = BitsToDouble(priceBits);
   
   // Unpack quantity (4 bytes, big-endian)
   trade.quantity = 0;
   for(int i = 0; i < 4; i++)
      trade.quantity |= ((uint)buffer[16 + i] << (24 - i * 8));
   
   // Unpack trade_id (4 bytes, big-endian)
   trade.tradeId = 0;
   for(int i = 0; i < 4; i++)
      trade.tradeId |= ((uint)buffer[20 + i] << (24 - i * 8));
   
   trade.side = buffer[24];
   ArrayCopy(trade.padding, buffer, 0, 25, 7);
   
   return true;
}

bool UnpackOrderBody(const uchar &buffer[], OrderBody &order)
{
   if(ArraySize(buffer) < 32) return false;
   
   // Unpack ticker_id (8 bytes, big-endian)
   order.tickerId = 0;
   for(int i = 0; i < 8; i++)
      order.tickerId |= ((ulong)buffer[i] << (56 - i * 8));
   
   // Unpack order_id (4 bytes, big-endian)
   order.orderId = 0;
   for(int i = 0; i < 4; i++)
      order.orderId |= ((uint)buffer[8 + i] << (24 - i * 8));
   
   // Unpack price (8 bytes, big-endian double)
   ulong priceBits = 0;
   for(int i = 0; i < 8; i++)
      priceBits |= ((ulong)buffer[12 + i] << (56 - i * 8));
   order.price = BitsToDouble(priceBits);
   
   // Unpack quantity (4 bytes, big-endian)
   order.quantity = 0;
   for(int i = 0; i < 4; i++)
      order.quantity |= ((uint)buffer[20 + i] << (24 - i * 8));
   
   order.typeAndSide = buffer[24];
   ArrayCopy(order.expiry, buffer, 0, 25, 6);
   order.padding = buffer[31];
   
   return true;
}

bool UnpackTickerBody(const uchar &buffer[], TickerBody &ticker)
{
   if(ArraySize(buffer) < 32) return false;
   
   // Unpack ticker_id (8 bytes, big-endian)
   ticker.tickerId = 0;
   for(int i = 0; i < 8; i++)
      ticker.tickerId |= ((ulong)buffer[i] << (56 - i * 8));
   
   // Unpack bid_price (8 bytes, big-endian double)
   ulong bidBits = 0;
   for(int i = 0; i < 8; i++)
      bidBits |= ((ulong)buffer[8 + i] << (56 - i * 8));
   ticker.bidPrice = BitsToDouble(bidBits);
   
   // Unpack ask_price (8 bytes, big-endian double)
   ulong askBits = 0;
   for(int i = 0; i < 8; i++)
      askBits |= ((ulong)buffer[16 + i] << (56 - i * 8));
   ticker.askPrice = BitsToDouble(askBits);
   
   // Unpack bid_volume (4 bytes, big-endian)
   ticker.bidVolume = 0;
   for(int i = 0; i < 4; i++)
      ticker.bidVolume |= ((uint)buffer[24 + i] << (24 - i * 8));
   
   // Unpack ask_volume (4 bytes, big-endian)
   ticker.askVolume = 0;
   for(int i = 0; i < 4; i++)
      ticker.askVolume |= ((uint)buffer[28 + i] << (24 - i * 8));
   
   return true;
}

bool UnpackOrderBookBody(const uchar &buffer[], OrderBookBody &orderBook, uint &volumes[])
{
   if(ArraySize(buffer) < 32) return false;
   
   // Unpack ticker_id (8 bytes, big-endian)
   orderBook.tickerId = 0;
   for(int i = 0; i < 8; i++)
      orderBook.tickerId |= ((ulong)buffer[i] << (56 - i * 8));
   
   // Unpack first_tick (8 bytes, big-endian double)
   ulong firstTickBits = 0;
   for(int i = 0; i < 8; i++)
      firstTickBits |= ((ulong)buffer[8 + i] << (56 - i * 8));
   orderBook.firstTick = BitsToDouble(firstTickBits);
   
   // Unpack tick_size (8 bytes, big-endian double)
   ulong tickSizeBits = 0;
   for(int i = 0; i < 8; i++)
      tickSizeBits |= ((ulong)buffer[16 + i] << (56 - i * 8));
   orderBook.tickSize = BitsToDouble(tickSizeBits);
   
   // Unpack num_ticks (2 bytes, big-endian)
   orderBook.numTicks = ((ushort)buffer[24] << 8) | (ushort)buffer[25];
   
   orderBook.side = buffer[26];
   ArrayCopy(orderBook.padding, buffer, 0, 27, 5);
   
   // Check if buffer is large enough for volumes
   int expectedSize = 32 + orderBook.numTicks * 4;
   if(ArraySize(buffer) < expectedSize) return false;
   
   // Resize volumes array and unpack
   ArrayResize(volumes, orderBook.numTicks);
   for(int i = 0; i < orderBook.numTicks; i++)
   {
      int offset = 32 + i * 4;
      volumes[i] = 0;
      for(int j = 0; j < 4; j++)
         volumes[i] |= ((uint)buffer[offset + j] << (24 - j * 8));
   }
   
   return true;
}

// === FILE I/O FUNCTIONS (MQL4 doesn't support TCP directly) ===

bool MitchWriteToFile(string filename, const uchar &data[])
{
   int handle = FileOpen(filename, FILE_WRITE | FILE_BIN);
   if(handle == INVALID_HANDLE) return false;
   
   uint written = FileWriteArray(handle, data, 0, ArraySize(data));
   FileClose(handle);
   
   return written == ArraySize(data);
}

bool MitchReadFromFile(string filename, uchar &data[])
{
   int handle = FileOpen(filename, FILE_READ | FILE_BIN);
   if(handle == INVALID_HANDLE) return false;
   
   uint fileSize = (uint)FileSize(handle);
   ArrayResize(data, fileSize);
   
   uint read = FileReadArray(handle, data, 0, fileSize);
   FileClose(handle);
   
   return read == fileSize;
}

// === EXAMPLE USAGE ===

void ExampleUsage()
{
   // Create a trade message
   MitchHeader header;
   header.messageType = MITCH_MSG_TYPE_TRADE;
   WriteTimestamp48(GetTickCount64() * 1000000, header.timestamp);
   header.count = 1;
   
   TradeBody trade;
   trade.tickerId = 0x00006F001CD00000; // EUR/USD
   trade.price = 1.0850;
   trade.quantity = 1000000; // 1.0 lot scaled by 1000000
   trade.tradeId = 12345;
   trade.side = 0; // Buy
   ArrayInitialize(trade.padding, 0);
   
   // Pack the message
   uchar headerBuffer[], tradeBuffer[];
   PackHeader(header, headerBuffer);
   PackTradeBody(trade, tradeBuffer);
   
   // Combine into single message
   uchar message[];
   ArrayResize(message, ArraySize(headerBuffer) + ArraySize(tradeBuffer));
   ArrayCopy(message, headerBuffer, 0, 0, ArraySize(headerBuffer));
   ArrayCopy(message, tradeBuffer, ArraySize(headerBuffer), 0, ArraySize(tradeBuffer));
   
   // Write to file (since MQL4 doesn't support TCP directly)
   if(MitchWriteToFile("trade_message.bin", message))
   {
      Print("Trade message written to file: ", ArraySize(message), " bytes");
   }
   
   // Read back and parse
   uchar readMessage[];
   if(MitchReadFromFile("trade_message.bin", readMessage))
   {
      MitchHeader readHeader;
      TradeBody readTrade;
      
      uchar headerPart[], tradePart[];
      ArrayResize(headerPart, 8);
      ArrayResize(tradePart, 32);
      
      ArrayCopy(headerPart, readMessage, 0, 0, 8);
      ArrayCopy(tradePart, readMessage, 0, 8, 32);
      
      if(UnpackHeader(headerPart, readHeader) && UnpackTradeBody(tradePart, readTrade))
      {
         Print("Successfully read trade: Price=", readTrade.price, " Quantity=", readTrade.quantity);
      }
   }
}

//+------------------------------------------------------------------+
//| Script program start function                                    |
//+------------------------------------------------------------------+
void OnStart()
{
   ExampleUsage();
}
