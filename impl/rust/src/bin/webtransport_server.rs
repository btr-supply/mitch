//! WebTransport/TCP benchmark server binary for MITCH protocol

use mitch::benchmarks::webtransport_server::run_benchmark_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get address from environment or use default
    let addr = std::env::var("SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4433".to_string());
    
    println!("Starting MITCH benchmark server on {}", addr);
    println!("Fire-and-forget mode enabled for maximum throughput");
    println!("No message acknowledgments or replay guarantees");
    println!();
    println!("Set ECHO_MODE=1 to enable echo mode for latency testing");
    
    run_benchmark_server(&addr).await
}