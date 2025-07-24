//! Ultra-high speed message producers for MITCH protocol benchmarking
//!
//! Optimized for fire-and-forget semantics with minimal memory allocations
//! and maximum throughput for high-frequency trading scenarios.

use crate::{
    benchmarks::{
        BenchmarkConfig, MessageType,
        generate_sample_message,
        inc_message_count, is_benchmark_running, start_benchmark, stop_benchmark,
    },
    networking::{MessageTransport, Pushable},
};
use std::sync::Arc;
use tokio::time::{self};

/// Produces a single type of message for benchmarking.
pub struct SingleTypeProducer {
    clients: Vec<Arc<dyn MessageTransport>>,
    message: Box<dyn Pushable + Send + Sync>,
}

impl std::fmt::Debug for SingleTypeProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingleTypeProducer")
            .field("clients", &self.clients.len())
            .field("message", &"Box<dyn Pushable>")
            .finish()
    }
}

impl SingleTypeProducer {
    pub fn new(clients: Vec<Arc<dyn MessageTransport>>, message_type: MessageType) -> Self {
        Self {
            clients,
            message: generate_sample_message(message_type),
        }
    }

    /// Runs a benchmark to achieve maximum throughput.
    pub async fn run_max_throughput(&self, config: &BenchmarkConfig) -> u64 {
        let mut messages_sent = 0;
        start_benchmark();
        let end_time = time::Instant::now() + config.duration;

        while time::Instant::now() < end_time {
            if !is_benchmark_running() {
                break;
            }
            self.message.push(&self.clients, None).await.unwrap();
            messages_sent += 1;
            inc_message_count();
        }

        stop_benchmark();
        messages_sent
    }
}
