//! Foreign Function Interface (FFI) module for MITCH protocol
//! 
//! This module provides C-compatible exports for using MITCH from other languages,
//! specifically designed for MetaTrader 4 integration.
//! 
//! # Features
//! - Asset resolution with fuzzy matching
//! - Ticker ID creation and parsing
//! - Message encoding/decoding (tick, trade, order, etc.)
//! - Redis publishing for real-time data
//! - Market provider resolution
//! 
//! # Safety
//! All FFI functions are marked `unsafe` as they work with raw pointers.
//! Proper null checks and bounds validation are performed internally.

#[cfg(feature = "ffi")]
pub mod mt4;

#[cfg(feature = "ffi")]
pub use mt4::*;