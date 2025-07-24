//! High-performance benchmarking utilities for MITCH protocol
//!
//! This module provides ultra-fast message producers, consumers, and performance
//! measurement tools optimized for high-frequency trading scenarios.

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::time;
use crate::{Trade, Order, Tick, Index, OrderBook, OrderSide, OrderType};
use crate::networking::{Pushable};

pub mod producers;
pub mod consumers;
pub mod metrics;

#[cfg(feature = "webtransport-client")]
pub mod webtransport_server;

/// Global atomic counters for lock-free performance tracking
pub static MESSAGE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static BYTE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static ERROR_COUNT: AtomicU64 = AtomicU64::new(0);
pub static BENCHMARK_RUNNING: AtomicBool = AtomicBool::new(false);

/// Reset all global counters
pub fn reset_counters() {
    MESSAGE_COUNT.store(0, Ordering::Relaxed);
    BYTE_COUNT.store(0, Ordering::Relaxed);
    ERROR_COUNT.store(0, Ordering::Relaxed);
}

/// Start benchmark timing
pub fn start_benchmark() {
    BENCHMARK_RUNNING.store(true, Ordering::Relaxed);
}

/// Stop benchmark timing
pub fn stop_benchmark() {
    BENCHMARK_RUNNING.store(false, Ordering::Relaxed);
}

/// Check if benchmark is running
pub fn is_benchmark_running() -> bool {
    BENCHMARK_RUNNING.load(Ordering::Relaxed)
}

/// Increment message count atomically
#[inline(always)]
pub fn inc_message_count() {
    MESSAGE_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Increment byte count atomically
#[inline(always)]
pub fn inc_byte_count(bytes: u64) {
    BYTE_COUNT.fetch_add(bytes, Ordering::Relaxed);
}

/// Increment error count atomically
#[inline(always)]
pub fn inc_error_count() {
    ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Get current message count
pub fn get_message_count() -> u64 {
    MESSAGE_COUNT.load(Ordering::Relaxed)
}

/// Get current byte count
pub fn get_byte_count() -> u64 {
    BYTE_COUNT.load(Ordering::Relaxed)
}

/// Get current error count
pub fn get_error_count() -> u64 {
    ERROR_COUNT.load(Ordering::Relaxed)
}

/// Message type for benchmarking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageType {
    Trade,
    Order,
    Tick,
    Index,
    OrderBook,
}

impl MessageType {
    /// Get message size in bytes
    pub fn size(&self) -> usize {
        match self {
            MessageType::Trade => 32,
            MessageType::Order => 32,
            MessageType::Tick => 32,
            MessageType::Index => 64,
            MessageType::OrderBook => 2072,
        }
    }

    /// Get expected throughput target (messages/second)
    pub fn target_throughput(&self) -> u64 {
        match self {
            MessageType::Trade => 1_000_000,
            MessageType::Order => 1_000_000,
            MessageType::Tick => 2_000_000,
            MessageType::Index => 500_000,
            MessageType::OrderBook => 100_000,
        }
    }
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub duration: Duration,
    pub warmup: Duration,
    pub message_type: MessageType,
    pub target_rate: Option<u64>,
    pub burst_mode: bool,
    pub mixed_workload: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(30),
            warmup: Duration::from_secs(5),
            message_type: MessageType::Trade,
            target_rate: None,
            burst_mode: false,
            mixed_workload: false,
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub message_type: MessageType,
    pub duration: Duration,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
    pub throughput_msg_per_sec: f64,
    pub throughput_mb_per_sec: f64,
    pub loss_rate: f64,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
}

impl BenchmarkResults {
    /// Create results from counters and timing
    pub fn from_counters(
        config: &BenchmarkConfig,
        actual_duration: Duration,
        messages_sent: u64,
        cpu_usage: f64,
        memory_usage: f64,
    ) -> Self {
        let messages_received = get_message_count();
        let bytes_received = get_byte_count();
        let errors = get_error_count();
        let bytes_sent = messages_sent * config.message_type.size() as u64;

        let duration_secs = actual_duration.as_secs_f64();
        let throughput_msg_per_sec = messages_received as f64 / duration_secs;
        let throughput_mb_per_sec = (bytes_received as f64 / 1_000_000.0) / duration_secs;
        let loss_rate = if messages_sent > 0 {
            ((messages_sent - messages_received) as f64 / messages_sent as f64) * 100.0
        } else {
            0.0
        };

        Self {
            message_type: config.message_type,
            duration: actual_duration,
            messages_sent,
            messages_received,
            bytes_sent,
            bytes_received,
            errors,
            throughput_msg_per_sec,
            throughput_mb_per_sec,
            loss_rate,
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: memory_usage,
        }
    }

    /// Print formatted results
    pub fn print(&self) {
        println!("\n----- Benchmark Results -----");
        println!("Message Type: {:?}", self.message_type);
        println!("Duration: {:.2}s", self.duration.as_secs_f64());
        println!("Messages Sent: {}", self.messages_sent);
        println!("Messages Received: {}", self.messages_received);
        println!(
            "Throughput: {:.2} msg/sec",
            self.throughput_msg_per_sec
        );
        println!("Errors: {}", self.errors);
        println!("CPU Usage: {:.1}%", self.cpu_usage_percent);
        println!("Memory Peak: {:.1} MB", self.memory_usage_mb);

        // Performance assessment
        let target = self.message_type.target_throughput() as f64;
        let performance_ratio = self.throughput_msg_per_sec / target;
        println!("\nPerformance Assessment:");
        println!("Target: {:.0} msg/s", target);
        println!("Achieved: {:.1}% of target", performance_ratio * 100.0);

        if performance_ratio >= 1.0 {
            println!("Status: ✅ EXCEEDS TARGET");
        } else if performance_ratio >= 0.8 {
            println!("Status: ✅ MEETS TARGET");
        } else if performance_ratio >= 0.5 {
            println!("Status: ⚠️  BELOW TARGET");
        } else {
            println!("Status: ❌ POOR PERFORMANCE");
        }
        println!("========================================\n");
    }
}

/// Generate a sample message of a given type.
pub fn generate_sample_message(message_type: MessageType) -> Box<dyn Pushable> {
    match message_type {
        MessageType::Trade => Box::new(Trade::new(12345, 99.95, 1000, 42, OrderSide::Buy).unwrap()),
        MessageType::Order => Box::new(
            Order::new(12346, 1001, 100.05, 500, OrderType::Market, OrderSide::Sell, 85).unwrap(),
        ),
        MessageType::Tick => Box::new(Tick::new(12347, 98.50, 101.50, 2000, 1500).unwrap()),
        MessageType::Index => {
            let index = Index::new(
                12348, 1500.25, 1000000, 500000, 400000, 100, 1, 20, 2, 3, 4, 5, 6, 95, 0, 1
            );
            Box::new(index)
        }
        MessageType::OrderBook => {
            let bids = [(1, 100), (2, 200)];
            let asks = [(3, 300), (4, 400)];
            let mut bid_bins = [Default::default(); 128];
            let mut ask_bins = [Default::default(); 128];
            for (i, (order_count, volume)) in bids.iter().enumerate() {
                bid_bins[i] = crate::order_book::Bin { order_count: *order_count, volume: *volume };
            }
            for (i, (order_count, volume)) in asks.iter().enumerate() {
                ask_bins[i] = crate::order_book::Bin { order_count: *order_count, volume: *volume };
            }
            let order_book = OrderBook::new(12349, 1000.5, 1, bid_bins, ask_bins);
            Box::new(order_book)
        }
    }
}

/// Mixed workload message distribution (realistic trading day)
pub fn create_mixed_message_batch(batch_size: usize) -> Vec<(MessageType, Vec<u8>)> {
    let mut messages = Vec::with_capacity(batch_size);

    for i in 0..batch_size {
        let message_data = match i % 100 {
            0..=59 => {
                // 60% Ticks
                let tick = Tick::new(12347, 98.50, 101.50, 2000, 1500).unwrap();
                (MessageType::Tick, tick.to_bytes())
            },
            60..=79 => {
                // 20% Trades
                let trade = Trade::new(12345, 99.95, 1000, 42, OrderSide::Buy).unwrap();
                (MessageType::Trade, trade.to_bytes())
            },
            80..=94 => {
                // 15% Orders
                let order = Order::new(12346, 1001, 100.05, 500, OrderType::Market, OrderSide::Sell, 85).unwrap();
                (MessageType::Order, order.to_bytes())
            },
            95..=98 => {
                // 4% Index
                let index = Index::new(12348, 1500.25, 1000000, 500000, 400000, 100, 1, 20, 2, 3, 4, 5, 6, 95, 0, 1);
                (MessageType::Index, index.to_bytes())
            },
            99 => {
                // 1% OrderBook
                let mut bid_bins = [Default::default(); 128];
                let mut ask_bins = [Default::default(); 128];
                bid_bins[0] = crate::order_book::Bin { order_count: 1, volume: 100 };
                ask_bins[0] = crate::order_book::Bin { order_count: 3, volume: 300 };
                let order_book = OrderBook::new(12349, 1000.5, 1, bid_bins, ask_bins);
                (MessageType::OrderBook, order_book.to_bytes())
            },
            _ => unreachable!(),
        };
        messages.push(message_data);
    }

    messages
}

/// Wait for benchmark condition with timeout
pub async fn wait_for_condition<F>(
    condition: F,
    timeout: Duration,
    check_interval: Duration,
) -> bool
where
    F: Fn() -> bool,
{
    let start = Instant::now();

    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        time::sleep(check_interval).await;
    }

    false
}
