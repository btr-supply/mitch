//! Integration tests for the MITCH Header implementation.
//!
//! This file contains tests for:
//! - Header creation and field handling
//! - Packing and unpacking (serialization/deserialization) roundtrip
//! - Validation logic for message types and counts
//! - Timestamp handling and 48-bit truncation

#![allow(clippy::all)]
use mitch::header::*;
use mitch::common::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new() {
        let header = MitchHeader::new(message_type::TRADE, 1234567890, 5);
        assert_eq!(header.message_type, message_type::TRADE);
        assert_eq!(header.get_timestamp(), 1234567890);
        assert_eq!(header.count, 5);
    }

    #[test]
    fn test_header_pack_unpack() {
        let original = MitchHeader::new(message_type::TICK, 9876543210, 100);
        let packed = original.pack();
        assert_eq!(packed.len(), 8);

        let unpacked = MitchHeader::unpack(&packed).unwrap();
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_timestamp_handling() {
        let mut header = MitchHeader::new(message_type::ORDER, 0, 1);

        // Test setting large timestamp (should truncate to 48 bits)
        let large_timestamp = 0xFFFFFFFFFFFFFFFF_u64;
        header.set_timestamp(large_timestamp);
        let retrieved = header.get_timestamp();

        // Should be truncated to 48 bits
        assert_eq!(retrieved, large_timestamp & 0xFFFFFFFFFFFF);
    }

    #[test]
    fn test_invalid_message_type() {
        let result = MitchHeader::new_validated(b'x', 1000, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_count() {
        let result = MitchHeader::new_validated(message_type::TRADE, 1000, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_size_calculation() {
        let header = MitchHeader::new(message_type::TRADE, 1000, 10);
        assert_eq!(header.total_message_size(32), 8 + 10 * 32);
    }

    #[test]
    fn test_message_type_validation() {
        // Test all valid message types
        assert!(MitchHeader::new_validated(message_type::TRADE, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::ORDER, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::TICK, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::INDEX, 1000, 1).is_ok());
        assert!(MitchHeader::new_validated(message_type::ORDER_BOOK, 1000, 1).is_ok());

        // Test invalid message type
        assert!(MitchHeader::new_validated(b'z', 1000, 1).is_err());
    }

    #[test]
    fn test_header_size() {
        assert_eq!(core::mem::size_of::<MitchHeader>(), 8);
    }

    #[test]
    fn test_display_format() {
        let header = MitchHeader::new(message_type::TRADE, 123456, 5);
        let display_str = format!("{}", header);
        assert!(display_str.contains("type: 't'"));
        assert!(display_str.contains("timestamp: 123456"));
        assert!(display_str.contains("count: 5"));
    }
}
