//! Ultra-fast message consumers for MITCH protocol benchmarking
//!
//! Optimized for non-blocking message processing with lock-free atomic
//! counters for maximum throughput measurement accuracy.

use crate::{
    benchmarks::{inc_message_count, inc_byte_count},
    ChannelId,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// Global message counter for simplified benchmarking
pub static GLOBAL_MESSAGE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static GLOBAL_BYTE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static GLOBAL_ERROR_COUNT: AtomicU64 = AtomicU64::new(0);

/// Simple consumer that counts messages without complex type dependencies
#[derive(Debug)]
pub struct SimpleConsumer {
    pub messages_received: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub errors: Arc<AtomicU64>,
}

impl SimpleConsumer {
    pub fn new() -> Self {
        Self {
            messages_received: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get a counting handler closure
    pub fn get_counting_handler(&self) -> impl Fn(ChannelId, &[u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> + Clone {
        let messages = self.messages_received.clone();
        let bytes = self.bytes_received.clone();
        
        move |_channel_id: ChannelId, data: &[u8]| {
            // Update global counters
            inc_message_count();
            inc_byte_count(data.len() as u64);
            GLOBAL_MESSAGE_COUNT.fetch_add(1, Ordering::Relaxed);
            GLOBAL_BYTE_COUNT.fetch_add(data.len() as u64, Ordering::Relaxed);
            
            // Update local counters
            messages.fetch_add(1, Ordering::Relaxed);
            bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
            
            Box::pin(async move {})
        }
    }

    /// Get a ticker-filtered counting handler closure
    pub fn get_filtered_handler(&self) -> impl Fn(ChannelId, u64, &[u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> + Clone {
        let messages = self.messages_received.clone();
        let bytes = self.bytes_received.clone();
        
        move |_channel_id: ChannelId, _ticker_id: u64, data: &[u8]| {
            // Update global counters
            inc_message_count();
            inc_byte_count(data.len() as u64);
            GLOBAL_MESSAGE_COUNT.fetch_add(1, Ordering::Relaxed);
            GLOBAL_BYTE_COUNT.fetch_add(data.len() as u64, Ordering::Relaxed);
            
            // Update local counters
            messages.fetch_add(1, Ordering::Relaxed);
            bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
            
            Box::pin(async move {})
        }
    }

    pub fn messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.messages_received.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
    }
}

/// Reset all global counters
pub fn reset_global_counters() {
    GLOBAL_MESSAGE_COUNT.store(0, Ordering::Relaxed);
    GLOBAL_BYTE_COUNT.store(0, Ordering::Relaxed);
    GLOBAL_ERROR_COUNT.store(0, Ordering::Relaxed);
}

/// Get global message count
pub fn get_global_message_count() -> u64 {
    GLOBAL_MESSAGE_COUNT.load(Ordering::Relaxed)
}

/// Get global byte count
pub fn get_global_byte_count() -> u64 {
    GLOBAL_BYTE_COUNT.load(Ordering::Relaxed)
}

/// Get global error count
pub fn get_global_error_count() -> u64 {
    GLOBAL_ERROR_COUNT.load(Ordering::Relaxed)
}