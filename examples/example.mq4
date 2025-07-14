//+------------------------------------------------------------------+
//|                                                      example.mq4 |
//| BTR MITCH Protocol Example & Test Suite                         |
//| Copyright BTR Supply                                             |
//| https://btr.supply                                               |
//+------------------------------------------------------------------+
#property copyright "Copyright BTR Supply"
#property link      "https://btr.supply"
#property version   "2.00"
#property script_show_inputs
#property strict

#include "../model/model.mq4"

//+------------------------------------------------------------------+
//| Input Parameters                                                 |
//+------------------------------------------------------------------+
input int TestIterations = 1000;      // Number of iterations for performance tests
input bool TestBasicFunctions = true; // Test basic currency and parsing functions
input bool TestSpecification = true;  // Test EURUSD specification compliance
input bool TestSerialization = true;  // Test serialization/deserialization
input bool TestPerformance = true;    // Test performance benchmarks

//+------------------------------------------------------------------+
//| Global Variables for Performance                                 |
//+------------------------------------------------------------------+

// Cached timestamp for performance
ulong g_lastTimestamp = 0;
datetime g_lastTime = 0;

//+------------------------------------------------------------------+
//| Timestamp and Utility Functions                                 |
//+------------------------------------------------------------------+

// Write 64-bit timestamp to 48-bit (6-byte) array
void WriteTimestamp48(ulong timestamp, uchar &output[])
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
ulong ReadTimestamp48(uchar &data[])
{
   return ((ulong)data[0] << 40) | ((ulong)data[1] << 32) | 
          ((ulong)data[2] << 24) | ((ulong)data[3] << 16) | 
          ((ulong)data[4] << 8) | ((ulong)data[5]);
}

// Get current timestamp with caching
ulong GetCurrentTimestamp()
{
   datetime now = TimeCurrent();
   if(now != g_lastTime)
   {
      datetime midnight = now - (now % 86400);
      g_lastTimestamp = (ulong)(now - midnight) * 1000000000;
      g_lastTime = now;
   }
   return g_lastTimestamp;
}

// Simple IEEE 754 double to 64-bit conversion for big-endian
ulong DoubleToBigEndianBits(double value)
{
   // Simple approach for MQL4 compatibility
   if(value == 0.0) return 0;
   
   bool negative = value < 0;
   if(negative) value = -value;
   
   // Extract exponent and mantissa
   int exp = 0;
   while(value >= 2.0) { value /= 2.0; exp++; }
   while(value < 1.0 && value > 0.0) { value *= 2.0; exp--; }
   
   // IEEE 754 format
   ulong sign = negative ? 1 : 0;
   ulong exponent = (exp + 1023) & 0x7FF;
   ulong mantissa = (ulong)((value - 1.0) * 4503599627370496.0) & 0xFFFFFFFFFFFFF; // 2^52
   
   return (sign << 63) | (exponent << 52) | mantissa;
}

// Convert 64-bit to double
double BigEndianBitsToDouble(ulong bits)
{
   if(bits == 0) return 0.0;
   
   bool negative = (bits >> 63) != 0;
   int exp = (int)((bits >> 52) & 0x7FF) - 1023;
    ulong mantissa = bits & 0xFFFFFFFFFFFFF;
    
   double value = 1.0 + (double)mantissa / 4503599627370496.0;
   
   for(int i = 0; i < MathAbs(exp); i++)
   {
      if(exp > 0) value *= 2.0;
      else value /= 2.0;
   }
   
   return negative ? -value : value;
}

//+------------------------------------------------------------------+
//| Packing Functions (Serialize to Big-Endian Binary)              |
//+------------------------------------------------------------------+

// Pack message header
int PackHeader(const MitchHeader &header, uchar &buffer[])
{
   ArrayResize(buffer, 8);
   buffer[0] = header.messageType;
   ArrayCopy(buffer, header.timestamp, 1, 0, 6);
   buffer[7] = header.count;
   return 8;
}

// Pack ticker body with proper MITCH ticker ID
int PackTick(const Tick &ticker, uchar &buffer[])
{
   ArrayResize(buffer, 32);
   
   // Pack ticker_id (8 bytes, big-endian)
   for(int i = 0; i < 8; i++)
      buffer[i] = (uchar)((ticker.tickerId >> (56 - i * 8)) & 0xFF);
   
   // Pack bid_price (8 bytes, big-endian double)
   ulong bidBits = DoubleToBigEndianBits(ticker.bidPrice);
   for(int i = 0; i < 8; i++)
      buffer[8 + i] = (uchar)((bidBits >> (56 - i * 8)) & 0xFF);
   
   // Pack ask_price (8 bytes, big-endian double)
   ulong askBits = DoubleToBigEndianBits(ticker.askPrice);
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

// Pack complete ticker message
int PackTickerMessageFast(const Tick &ticker, uchar &buffer[])
{
   // Create header
   MitchHeader header;
   header.messageType = MITCH_MSG_TYPE_TICKER;
   WriteTimestamp48(GetCurrentTimestamp(), header.timestamp);
   header.count = 1;
   
   // Pack header and body
   uchar headerBuffer[], bodyBuffer[];
   int headerSize = PackHeader(header, headerBuffer);
   int bodySize = PackTick(ticker, bodyBuffer);
   
   // Combine into single message
   ArrayResize(buffer, headerSize + bodySize);
   ArrayCopy(buffer, headerBuffer, 0, 0, headerSize);
   ArrayCopy(buffer, bodyBuffer, headerSize, 0, bodySize);
   
   return headerSize + bodySize;
}

// Create ticker from MT4 symbol
Tick CreateTickerFromSymbol(string symbol)
{
   Tick ticker;
   
   // Generate ticker ID using basic implementation
   ticker.tickerId = GenerateForexTickerID(symbol);
   
   // Get current market data
   ticker.bidPrice = MarketInfo(symbol, MODE_BID);
   ticker.askPrice = MarketInfo(symbol, MODE_ASK);
   ticker.bidVolume = 0; // MT4 doesn't provide volume since last snapshot
   ticker.askVolume = 0;
   
   return ticker;
}

//+------------------------------------------------------------------+
//| Unpacking Functions (Deserialize from Big-Endian Binary)        |
//+------------------------------------------------------------------+

// Unpack message header
bool UnpackHeader(uchar &buffer[], MitchHeader &header)
{
   if(ArraySize(buffer) < 8) return false;
   
   header.messageType = buffer[0];
   ArrayCopy(header.timestamp, buffer, 0, 1, 6);
   header.count = buffer[7];
   
   return true;
}

// Unpack ticker body
bool UnpackTick(uchar &buffer[], Tick &ticker)
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
   ticker.bidPrice = BigEndianBitsToDouble(bidBits);
   
   // Unpack ask_price (8 bytes, big-endian double)
   ulong askBits = 0;
   for(int i = 0; i < 8; i++)
      askBits |= ((ulong)buffer[16 + i] << (56 - i * 8));
   ticker.askPrice = BigEndianBitsToDouble(askBits);
   
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

// Unpack complete ticker message
bool UnpackTickerMessageFast(uchar &buffer[], MitchHeader &header, Tick &ticker)
{
   if(ArraySize(buffer) < 40) return false; // 8 header + 32 body
   
   // Unpack header
   if(!UnpackHeader(buffer, header)) return false;
   
   // Validate message type
   if(header.messageType != MITCH_MSG_TYPE_TICKER) return false;
   if(header.count != 1) return false;
   
   // Unpack ticker body (skip 8-byte header)
   uchar bodyBuffer[];
   ArrayResize(bodyBuffer, 32);
   ArrayCopy(bodyBuffer, buffer, 0, 8, 32);
   
   return UnpackTick(bodyBuffer, ticker);
}

//+------------------------------------------------------------------+
//| File I/O Functions                                              |
//+------------------------------------------------------------------+

// Write binary data to file
bool WriteToFileFast(string filename, uchar &data[])
{
   int handle = FileOpen(filename, FILE_WRITE | FILE_BIN);
   if(handle == INVALID_HANDLE) return false;
   
   uint written = FileWriteArray(handle, data, 0, ArraySize(data));
   FileClose(handle);
   
   return written == ArraySize(data);
}

// Read binary data from file
bool ReadFromFileFast(string filename, uchar &data[])
{
   int handle = FileOpen(filename, FILE_READ | FILE_BIN);
   if(handle == INVALID_HANDLE) return false;
   
   uint fileSize = (uint)FileSize(handle);
   ArrayResize(data, fileSize);
   
   uint read = FileReadArray(handle, data, 0, fileSize);
   FileClose(handle);
   
   return read == fileSize;
}

//+------------------------------------------------------------------+
//| Test Functions                                                   |
//+------------------------------------------------------------------+

// Test basic currency and parsing functions
bool TestBasicCurrencyFunctions()
{
   Print("--- Testing Basic Currency Functions ---");
   
   // Test ticker ID generation
   ulong eur_usd_id = GenerateForexTickerID("EURUSD");
   ulong gbp_usd_id = GenerateForexTickerID("GBPUSD");
   
   Print("  EURUSD Ticker ID: 0x" + IntegerToString(eur_usd_id, 16));
   Print("  GBPUSD Ticker ID: 0x" + IntegerToString(gbp_usd_id, 16));
   
   bool tickerOK = (eur_usd_id > 0 && gbp_usd_id > 0);
   
   Print("  Ticker generation: " + (tickerOK ? "PASSED" : "FAILED"));
   
   return tickerOK;
}

// Test EURUSD specification compliance
bool TestEURUSDSpecification()
{
   Print("--- Testing EURUSD Specification Compliance ---");
   
   // Test the exact specification example
   ulong ticker_id = GenerateForexTickerID("EURUSD");
   ulong expected_id = 0x03006F301CD00000;
   
   Print("  Calculated EURUSD ticker ID: 0x" + IntegerToString(ticker_id, 16));
   Print("  Expected ticker ID: 0x" + IntegerToString(expected_id, 16));
   
   bool specMatch = (ticker_id == expected_id);
   if(specMatch)
   {
      Print("  ✓ SUCCESS: Ticker ID matches specification exactly!");
   }
   else
   {
      Print("  ✗ FAIL: Ticker ID does not match specification");
   }
   
   return specMatch;
}

// Test serialization/deserialization
bool TestSerializationRoundTrip()
{
   Print("--- Testing Serialization Round-Trip ---");
   
   // Create test ticker
   Tick originalTicker = CreateTickerFromSymbol("EURUSD");
   originalTicker.bidPrice = 1.0950;
   originalTicker.askPrice = 1.0952;
   originalTicker.bidVolume = 1000000;
   originalTicker.askVolume = 750000;
   
   Print("  Original ticker ID: 0x" + IntegerToString(originalTicker.tickerId, 16));
   
   // Serialize
   uchar buffer[];
   int size = PackTickerMessageFast(originalTicker, buffer);
   
   if(size == 40)
   {
      Print("  Serialization: PASSED (" + IntegerToString(size) + " bytes)");
      
      // Deserialize
      MitchHeader header;
      Tick deserializedTicker;
      
      if(UnpackTickerMessageFast(buffer, header, deserializedTicker))
      {
         // Verify round-trip
         bool roundTripOK = (
            deserializedTicker.tickerId == originalTicker.tickerId &&
            MathAbs(deserializedTicker.bidPrice - originalTicker.bidPrice) < 0.0001 &&
            MathAbs(deserializedTicker.askPrice - originalTicker.askPrice) < 0.0001 &&
            deserializedTicker.bidVolume == originalTicker.bidVolume &&
            deserializedTicker.askVolume == originalTicker.askVolume
         );
         
         Print("  Deserialization: " + (roundTripOK ? "PASSED" : "FAILED"));
         Print("  Message Type: " + CharToString(header.messageType));
         Print("  Bid/Ask: " + DoubleToString(deserializedTicker.bidPrice, 5) + "/" + DoubleToString(deserializedTicker.askPrice, 5));
         
         // Test file I/O
         string filename = "mitch_test_" + IntegerToString(GetTickCount()) + ".bin";
         if(WriteToFileFast(filename, buffer))
         {
            uchar readBuffer[];
            if(ReadFromFileFast(filename, readBuffer))
            {
               Print("  File I/O: PASSED");
               return roundTripOK;
            }
         }
         Print("  File I/O: FAILED");
         return roundTripOK;
      }
      else
      {
         Print("  Deserialization: FAILED");
         return false;
      }
   }
   else
   {
      Print("  Serialization: FAILED (size=" + IntegerToString(size) + ")");
      return false;
   }
}

// Test performance benchmarks
bool TestPerformanceBenchmarks()
{
   Print("--- Testing Performance Benchmarks ---");
   
   // Test ticker creation performance
   uint startTime = GetTickCount();
   for(int i = 0; i < TestIterations; i++)
   {
      CreateTickerFromSymbol("EURUSD");
   }
   uint endTime = GetTickCount();
   double elapsed = (endTime - startTime) / 1000.0;
   double creationRate = elapsed > 0 ? TestIterations / elapsed : 0;
   Print("  Ticker creation rate: " + DoubleToString(creationRate, 0) + " ops/sec");
   
   // Test serialization performance
   Tick ticker = CreateTickerFromSymbol("EURUSD");
   uchar buffer[];
   
   startTime = GetTickCount();
   for(int i = 0; i < TestIterations; i++)
   {
      PackTickerMessageFast(ticker, buffer);
   }
   endTime = GetTickCount();
   double serializationElapsed = (endTime - startTime) / 1000.0;
   double serializationRate = serializationElapsed > 0 ? TestIterations / serializationElapsed : 0;
   Print("  Serialization rate: " + DoubleToString(serializationRate, 0) + " ops/sec");
   
   // Performance targets
   bool performanceOK = ((creationRate > 1000 || creationRate == 0) && 
                        (serializationRate > 1000 || serializationRate == 0));
   
   Print("  Performance targets: " + (performanceOK ? "MET" : "NOT MET"));
   
   return performanceOK;
}

//+------------------------------------------------------------------+
//| Script program start function                                    |
//+------------------------------------------------------------------+
void OnStart()
{
   Print("=== BTR MITCH Protocol Example & Test Suite ===");
   Print("Test iterations: " + IntegerToString(TestIterations));
   Print("");
   
   int totalTests = 0;
   int passedTests = 0;
   
   // Run tests based on input parameters
   if(TestBasicFunctions)
   {
      totalTests++;
      if(TestBasicCurrencyFunctions())
      {
         passedTests++;
         Print("✓ Basic Functions Test - PASSED");
      }
      else
      {
         Print("✗ Basic Functions Test - FAILED");
      }
      Print("");
   }
   
   if(TestSpecification)
   {
      totalTests++;
      if(TestEURUSDSpecification())
      {
         passedTests++;
         Print("✓ EURUSD Specification Test - PASSED");
      }
      else
      {
         Print("✗ EURUSD Specification Test - FAILED");
      }
      Print("");
   }
   
   if(TestSerialization)
   {
      totalTests++;
      if(TestSerializationRoundTrip())
      {
         passedTests++;
         Print("✓ Serialization Test - PASSED");
      }
      else
      {
         Print("✗ Serialization Test - FAILED");
      }
      Print("");
   }
   
   if(TestPerformance)
   {
      totalTests++;
      if(TestPerformanceBenchmarks())
      {
         passedTests++;
         Print("✓ Performance Test - PASSED");
      }
      else
      {
         Print("✗ Performance Test - FAILED");
      }
      Print("");
   }
   
   // Summary
   Print("=== Test Summary ===");
   Print("Total test categories: " + IntegerToString(totalTests));
   Print("Passed: " + IntegerToString(passedTests));
   Print("Success rate: " + DoubleToString(100.0 * passedTests / totalTests, 1) + "%");
   
   if(passedTests == totalTests)
      Print("✓✓ ALL TESTS PASSED!");
   else
      Print("✗✗ SOME TESTS FAILED!");
   
   Print("=== BTR MITCH Example Complete ===");
}
