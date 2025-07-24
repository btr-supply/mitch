//! Simple WebTransport echo server for MITCH protocol benchmarking
//!
//! This is a minimal server implementation focused on throughput testing.
//! It simply receives messages and counts them without processing.

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Global counters for benchmarking
pub static SERVER_MESSAGE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static SERVER_BYTE_COUNT: AtomicU64 = AtomicU64::new(0);
pub static SERVER_ERROR_COUNT: AtomicU64 = AtomicU64::new(0);

/// Simple TCP server that mimics WebTransport behavior for benchmarking
/// Note: This is a simplified implementation for benchmarking purposes only
#[derive(Debug)]
pub struct BenchmarkServer {
    listener: TcpListener,
}

impl BenchmarkServer {
    pub async fn new(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    /// Run the server with fire-and-forget message handling
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Benchmark server listening on: {}", self.listener.local_addr()?);
        println!("Fire-and-forget mode: No acknowledgments or guarantees");

        loop {
            let (socket, addr) = self.listener.accept().await?;
            println!("New connection from: {}", addr);

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket).await {
                    eprintln!("Connection error: {}", e);
                    SERVER_ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    }
}

/// Handle a connection with fire-and-forget semantics
async fn handle_connection(mut socket: tokio::net::TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; 4096];

    loop {
        // Read message length (4 bytes)
        match socket.read_exact(&mut buffer[..4]).await {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let msg_len = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        
        if msg_len > buffer.len() {
            buffer.resize(msg_len, 0);
        }

        // Read the message
        socket.read_exact(&mut buffer[..msg_len]).await?;

        // Update counters (fire-and-forget, no processing)
        SERVER_MESSAGE_COUNT.fetch_add(1, Ordering::Relaxed);
        SERVER_BYTE_COUNT.fetch_add(msg_len as u64, Ordering::Relaxed);

        // Optional: Echo back for latency testing (can be disabled for pure throughput)
        if std::env::var("ECHO_MODE").is_ok() {
            let len_bytes = (msg_len as u32).to_be_bytes();
            socket.write_all(&len_bytes).await?;
            socket.write_all(&buffer[..msg_len]).await?;
        }
    }

    Ok(())
}

/// Get server statistics
pub fn get_server_stats() -> (u64, u64, u64) {
    (
        SERVER_MESSAGE_COUNT.load(Ordering::Relaxed),
        SERVER_BYTE_COUNT.load(Ordering::Relaxed),
        SERVER_ERROR_COUNT.load(Ordering::Relaxed),
    )
}

/// Reset server counters
pub fn reset_server_counters() {
    SERVER_MESSAGE_COUNT.store(0, Ordering::Relaxed);
    SERVER_BYTE_COUNT.store(0, Ordering::Relaxed);
    SERVER_ERROR_COUNT.store(0, Ordering::Relaxed);
}

/// Run a simple benchmark server
pub async fn run_benchmark_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let server = BenchmarkServer::new(addr).await?;
    
    // Spawn a task to print stats periodically
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let (msgs, bytes, errors) = get_server_stats();
            let mb = bytes as f64 / 1_000_000.0;
            println!("Server stats: {} messages, {:.2} MB, {} errors", msgs, mb, errors);
        }
    });
    
    server.run().await
}