# MITCH: Ultra-light Market-Data

## Overview

**MITCH, or Moded ITCH** is a transport-agnostic binary protocol designed for ultra-low latency financial market data transmission. This project provides comprehensive model implementations across 7 programming languages, plus integration tools for MT4 platforms.

### Key Features

- **🚀 Ultra-Low Latency**: 10-40% lighter messages than NASDAQ ITCH
- **🔄 Transport Agnostic**: Works with TCP, UDP, file storage, message queues
- **🌐 Cross-Platform**: Consistent Big-Endian encoding across all platforms  
- **📦 Multi-Language**: Native implementations in TypeScript, Python, Rust, Go, Java, C, and MQL4
- **⚡ Performance Optimized**: Fixed-width fields, memory alignment, zero-copy parsing
- **🛡️ Production Ready**: Strict separation of concerns, comprehensive examples

## Quick Start

### 1. Clone Repository
```bash
git clone https://github.com/btr-supply/mitch
cd mitch
```

### 2. Choose Your Language Implementation

Navigate to the language-specific examples:

```bash
# TypeScript/JavaScript
cd mitch/examples && node example.ts

# Python  
cd mitch/examples && python example.py

# Rust
cd mitch/examples && cargo run example.rs

# Go
cd mitch/examples && go run example.go

# Java
cd mitch/examples && javac example.java && java example

# C
cd mitch/examples && gcc example.c -o example && ./example
```

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

- **Message Overhead**: Fixed 8-byte header
- **Ticker Encoding**: Single 8-byte ID vs multiple reference lookups
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
