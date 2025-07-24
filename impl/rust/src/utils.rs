//! Utility functions for the MITCH protocol

pub mod format;
pub mod similarity;

// Re-export commonly used functions
pub use format::*;
pub use similarity::*;

/// Converts a u64 timestamp (nanoseconds) to a u48, returning it as a u64.
/// The upper 16 bits are truncated. This is a potential loss of data if the timestamp
/// is very large, but is required for the MITCH protocol.
#[inline]
pub fn timestamp_to_u48(timestamp: u64) -> [u8; 6] {
    let truncated = timestamp & 0x0000_FFFF_FFFF_FFFF;
    let bytes = truncated.to_le_bytes();
    let mut result = [0u8; 6];
    result.copy_from_slice(&bytes[0..6]);
    result
}

/// Converts a u48 timestamp (as a u64) back to a full u64 timestamp.
/// This function is trivial as the upper bits are simply zero, but is provided
/// for symmetry and clarity.
#[inline]
pub fn u48_to_timestamp(timestamp_u48: [u8; 6]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes[0..6].copy_from_slice(&timestamp_u48);
    u64::from_le_bytes(bytes)
}
