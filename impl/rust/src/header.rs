//! MITCH unified message header implementation
//!
//! This module implements the 8-byte MITCH header that prefixes all message types.
//! The header contains message type, timestamp, and count information.

use crate::common::{message_sizes, MitchError, validate_message_type};

/// MITCH unified message header (8 bytes)
///
/// Structure per messaging.md specification:
/// - message_type: u8 (ASCII character)
/// - timestamp: u48 (6 bytes, nanoseconds since midnight UTC)
/// - count: u8 (number of body entries, 1-255)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MitchHeader {
    /// u8: ASCII message type ('t', 'o', 's', 'b', 'i')
    pub message_type: u8,
    /// u48: nanoseconds since midnight UTC (6 bytes)
    pub timestamp: [u8; 6],
    /// u8: number of body entries (1-255)
    pub count: u8,
}

impl MitchHeader {
    /// Create new header with timestamp as u64 (automatically truncates to u48)
    ///
    /// # Arguments
    /// * `message_type` - ASCII character for message type ('t', 'o', 's', 'b', 'i')
    /// * `timestamp` - Nanoseconds since midnight UTC (will be truncated to 48 bits)
    /// * `count` - Number of body entries (1-255)
    ///
    /// # Returns
    /// New MitchHeader instance
    ///
    /// # Panics
    /// Panics if message_type is invalid or count is 0
    pub fn new(message_type: u8, timestamp: u64, count: u8) -> Self {
        validate_message_type(message_type).expect("Invalid message type");
        assert!(count > 0, "Count must be greater than 0");

        let mut ts_bytes = [0u8; 6];
        let ts_le_bytes = timestamp.to_le_bytes();
        ts_bytes.copy_from_slice(&ts_le_bytes[0..6]);

        Self {
            message_type,
            timestamp: ts_bytes,
            count,
        }
    }

    /// Create new header with validated inputs
    ///
    /// # Arguments
    /// * `message_type` - ASCII character for message type
    /// * `timestamp` - Nanoseconds since midnight UTC
    /// * `count` - Number of body entries
    ///
    /// # Returns
    /// Result containing new MitchHeader or error
    pub fn new_validated(message_type: u8, timestamp: u64, count: u8) -> Result<Self, MitchError> {
        validate_message_type(message_type)?;

        if count == 0 {
            return Err(MitchError::InvalidData("Count must be greater than 0".to_string()));
        }

        let mut ts_bytes = [0u8; 6];
        let ts_le_bytes = timestamp.to_le_bytes();
        ts_bytes.copy_from_slice(&ts_le_bytes[0..6]);

        Ok(Self {
            message_type,
            timestamp: ts_bytes,
            count,
        })
    }

    /// Get timestamp as u64 from u48 bytes (Little-Endian)
    ///
    /// # Returns
    /// Timestamp in nanoseconds since midnight UTC
    pub fn get_timestamp(&self) -> u64 {
        let mut ts_bytes = [0u8; 8];
        ts_bytes[0..6].copy_from_slice(&self.timestamp);
        u64::from_le_bytes(ts_bytes)
    }

    /// Set timestamp from u64 (automatically truncates to u48, Little-Endian)
    ///
    /// # Arguments
    /// * `timestamp` - Nanoseconds since midnight UTC
    pub fn set_timestamp(&mut self, timestamp: u64) {
        let ts_le_bytes = timestamp.to_le_bytes();
        self.timestamp.copy_from_slice(&ts_le_bytes[0..6]);
    }

        /// Pack MitchHeader into bytes using raw pointer casting (ultra-fast)
    ///
    /// This method uses raw pointer casting for maximum performance.
    /// Safe because MitchHeader is `#[repr(C, packed)]` with only POD types.
    ///
    /// # Returns
    /// 8-byte array containing the packed header
    pub fn pack(&self) -> [u8; message_sizes::HEADER] {
        unsafe {
            // Use raw pointer for maximum performance
            let ptr = self as *const Self as *const u8;
            let mut result = [0u8; message_sizes::HEADER];
            core::ptr::copy_nonoverlapping(ptr, result.as_mut_ptr(), message_sizes::HEADER);
            result
        }
    }

        /// Unpack MitchHeader from bytes using raw pointer casting (ultra-fast)
    ///
    /// This method uses raw pointer casting for maximum performance.
    /// Validates buffer size but trusts memory layout.
    ///
    /// # Arguments
    /// * `bytes` - Byte slice containing header data
    ///
    /// # Returns
    /// Result containing unpacked MitchHeader or error
    ///
    /// # Safety
    /// Caller must ensure bytes contains valid header data
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::HEADER {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::HEADER,
                actual: bytes.len(),
            });
        }

        unsafe {
            // Use raw pointer for maximum performance
            let ptr = bytes.as_ptr() as *const Self;
            let header = ptr.read_unaligned();

            // Validate unpacked data
            validate_message_type(header.message_type)?;
            if header.count == 0 {
                return Err(MitchError::InvalidData("Count cannot be 0".to_string()));
            }

            Ok(header)
        }
    }

    /// Get the total message size including body
    ///
    /// # Arguments
    /// * `body_size` - Size of a single body entry in bytes
    ///
    /// # Returns
    /// Total message size in bytes
    pub fn total_message_size(&self, body_size: usize) -> usize {
        message_sizes::HEADER + (self.count as usize * body_size)
    }

    /// Validate header consistency
    ///
    /// # Returns
    /// Result indicating validation success or error
    pub fn validate(&self) -> Result<(), MitchError> {
        validate_message_type(self.message_type)?;

        if self.count == 0 {
            return Err(MitchError::InvalidData("Count must be greater than 0".to_string()));
        }

        Ok(())
    }

    /// Get message type as character
    ///
    /// # Returns
    /// ASCII character representing the message type
    pub fn message_type_char(&self) -> char {
        self.message_type as char
    }
}

impl Default for MitchHeader {
    fn default() -> Self {
        Self {
            message_type: b't', // Default to trade message
            timestamp: [0; 6],
            count: 1,
        }
    }
}

// =============================================================================
// DISPLAY IMPLEMENTATIONS
// =============================================================================

impl core::fmt::Display for MitchHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "MitchHeader {{ type: '{}', timestamp: {}, count: {} }}",
            self.message_type_char(),
            self.get_timestamp(),
            self.count
        )
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Calculate expected body size based on message type and count
///
/// # Arguments
/// * `header` - Header containing message type and count
///
/// # Returns
/// Expected body size in bytes, or error if message type is invalid
pub fn calculate_body_size(header: &MitchHeader) -> Result<usize, MitchError> {
    let single_body_size = match header.message_type {
        crate::common::message_type::TRADE => message_sizes::TRADE,
        crate::common::message_type::ORDER => message_sizes::ORDER,
        crate::common::message_type::TICK => message_sizes::TICK,
        crate::common::message_type::INDEX => message_sizes::INDEX,
        crate::common::message_type::ORDER_BOOK => message_sizes::ORDER_BOOK,
        _ => return Err(MitchError::InvalidMessageType(header.message_type)),
    };

    Ok(header.count as usize * single_body_size)
}


