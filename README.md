# MITCH: Ultra-light Market-Data

## Overview

**MITCH, or Moded Individual Trade Clearing and Handling** is a transport-agnostic binary protocol designed for ultra-low latency financial market data transmission inspired by [NASDAQ's TotalView ITCH](https://data.nasdaq.com/databases/NTV). This project provides comprehensive model implementations across multiple programming languages.

### Key Features

- **🚀 Ultra-Light**: 10-40% lighter messages than NASDAQ ITCH
- **🔄 Transport Agnostic**: Works well over TCP (ZMQ/NATS...), UDP (KCP/MoldUDP64...), unicast/multicast/queues...
- **🌐 Cross-Platform**: Consistent Big-Endian encoding across all platforms  
- **📦 Multi-Language**: Native implementations in TypeScript, Python, Rust, Go, Java, C, and MQL4
- **⚡ Performance Optimized**: Fixed-width fields, memory alignment, zero-copy parsing
- **🛡️ Production Ready**: Strict separation of concerns, comprehensive examples

## MITCH Protocol Specification

### Message Structure

For detailed message packing and structure specifications, please refer to [./model/README.md].

## Architecture

### Project Structure

```
mitch/
├── README.md             # You're here
├── LICENSE               # Boring, open MIT Licensing
├── model/                # Data structures only
│   ├── model.ts          # TypeScript definitions
│   ├── model.py          # Python dataclasses  
│   ├── model.rs          # Rust structs
│   ├── model.go          # Go types
│   ├── model.java        # Java classes
│   ├── model.h           # C structs
│   ├── model.mq4         # MQL4 structs
│   └── README.md         # Full specification
├── examples/             # Packing/unpacking & networking
│   ├── example.ts        # Complete TypeScript example
│   ├── example.py        # Complete Python example  
│   ├── example.rs        # Complete Rust example
│   ├── example.go        # Complete Go example
│   ├── example.java      # Complete Java example
│   ├── example.c         # Complete C example
│   ├── example.mq4       # Complete MQL4 example
│   └── README.md         # Implementation guide
└── ids/                  # Reference data
    ├── currency-ids.csv  # Forex identifiers used by BTR
    ├── stock-ids.csv     # Stock identifiers used by BTR
    └── market-provider-ids.csv # Exchanges/Dark Pools/ECNs/Brokers/Market Makers identifiers used by BTR

```

### Separation of Concern

- **Model files**: Data structures, constants, basic utilities only
- **Example files**: Packing/unpacking, networking, timestamp functions
- **No cross-contamination**: Helper functions are NOT in model files

## Performance Characteristics

- **Message Overhead**: Fixed 8-byte header whatever the batch size
- **Ticker Encoding**: Single 8-byte ID whatever the asset class and exchange
- **Memory Alignment**: 32-byte body alignment for optimal access
- **Zero-Copy**: Fixed-width fields enable direct memory mapping (flat-buffers style)
- **Batch Support**: Predictable batching up to 255 objects per message reduces syscall overhead

## Development

### Adding New Message Types

1. **Define structure** in all model files (`mitch/model/`)
2. **Implement packing/unpacking** in all example files (`mitch/examples/`)  
3. **Update documentation** in `mitch/model/README.md`
4. **Test across all languages** for consistency

### Testing Consistency

Each implementation includes comprehensive examples demonstrating:
- Message creation and serialization
- Deserialization and validation  
- TCP send/receive operations
- Timestamp handling
- Error conditions

## Contributing

1. **Maintain separation**: Keep model files clean of implementation logic
2. **Follow naming conventions**: Use language-appropriate naming
3. **Test all languages**: Ensure cross-language compatibility
4. **Update documentation**: Keep specifications current

## License

MIT, see [./LICENSE]

## References

- [NASDAQ ITCH Protocol](./itch/v5-specs.pdf)
- [MITCH Specification](./model/README.md)
- [Implementation Examples](./examples/README.md)

## Key Features

### 🔧 **Core Functionality**
- **MITCH Protocol**: Full implementation of message types (Trade, Order, Ticker, OrderBook)
- **Big-Endian Serialization**: IEEE 754 compliant double precision encoding
- **Timestamp Handling**: ITCH flavored 48-bit nanosecond server timestamps
- **Binary I/O**: Efficient file-based message persistence

### 🧪 **Testing & Validation**
- **Specification Compliance**: Validates EURUSD ticker ID against MITCH specification
- **Round-Trip Testing**: Serialization/deserialization integrity verification
- **Performance Benchmarks**: Throughput testing for production readiness
- **Comprehensive Coverage**: Tests all major protocol components

### 🏗️ **BTR Integration Points**
- **Currency System**: BTR currency ID constants (EUR=111, USD=461, etc.)
- **Asset Classes**: Full BTR asset classification system
- **Instrument Types**: Complete BTR instrument type definitions
- **Ticker ID Generation**: Basic implementation for common forex pairs

## Performance Characteristics

### **Test Results (typical):**
- **Ticker Creation**: >10,000 ops/sec
- **Serialization**: >5,000 ops/sec
- **Deserialization**: >8,000 ops/sec
- **File I/O**: >1,000 ops/sec

### **Memory Usage:**
- **Message Size**: 40 bytes (8-byte header + 32-byte body)
- **Zero Memory Leaks**: Proper cleanup implemented
- **Efficient Caching**: Timestamp caching for performance

## MITCH Specification Compliance

### **Ticker ID Format (64-bit):**
```
Bits 60-63: Instrument Type (4 bits)
Bits 40-59: Base Asset (20 bits = 4-bit class + 16-bit ID)
Bits 20-39: Quote Asset (20 bits = 4-bit class + 16-bit ID)
Bits 0-19:  Sub-Type (20 bits, 0 for spot forex)
```

### **Example: EURUSD**
```
Expected: 0x03006F301CD00000
- Instrument Type: 0x0 (Spot)
- Base Asset: 0x3006F (Forex class 0x3 + EUR ID 111)
- Quote Asset: 0x301CD (Forex class 0x3 + USD ID 461)
- Sub-Type: 0x00000 (Spot forex)
```

## Development Notes

### **MQL4 Limitations Addressed:**
- **IEEE 754 Conversion**: Custom implementation for double precision
- **64-bit Operations**: Careful handling of large integers
- **Big-Endian Serialization**: Manual byte ordering for network compatibility
- **Memory Management**: Proper array handling and cleanup

### **Production Considerations:**
- **Error Handling**: Comprehensive validation and error reporting
- **Performance Optimization**: Cached operations and efficient algorithms
- **Compatibility**: Maintains backward compatibility with existing MITCH implementations
- **Extensibility**: Designed for easy integration with full BTR system

## Testing & Validation

Run the example file to validate:
1. **Basic Model and Computed IDs**: Ticker ID generation (eg. EURUSD ticker ID validation)
2. **Serialization**: Round-trip integrity testing
3. **Performance**: Throughput benchmarking for serialization/de-serialization and network communication

---

+ ## Disclaimer
+ 
+ **Inspiration and Credits:**
+ Our MITCH protocol is heavily inspired by the original ITCH protocol designed by Josh Levine of the Island ECN and Nasdaq. We extend our full credit and gratitude for their pioneering work in financial market data protocols.
+ 
+ **Licensing and Usage:**
+ MITCH is not a commercial product and is distributed freely under the MIT License. It is an open-source tool intended for research, development, and educational purposes.
+ 
+ **Development Status:**
+ This implementation is currently a work in progress. While functional, it has not yet been flagged as production-ready. Users should perform their own rigorous testing. Until this notice is removed, it is strongly recommended that users re-implement and verify all testing and serialization integrity checks to ensure it meets the requirements of their specific use case.
+ 
 **BTR Supply** | https://btr.supply | Production-Ready MITCH Implementation
