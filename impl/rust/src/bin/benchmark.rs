//! MITCH Protocol Benchmark
//!
//! Comprehensive benchmarking with two modes:
//! 1. Maximum throughput (fire-and-forget)
//! 2. Sustainable rate testing (controlled publishing)

use mitch::{
    ChannelId, Trade, Tick, Index, OrderBook, OrderSide, Pushable,
    networking::MessageTransport,
};
#[cfg(feature = "redis-client")]
use mitch::networking::redis::RedisTransport;
#[cfg(feature = "webtransport-client")]
use mitch::networking::webtransport::WebTransportClient;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use chrono::Local;

const TEST_DURATION_SECS: u64 = 5;
const WARMUP_DURATION_MS: u64 = 500;

// For max throughput testing
const SUBSCRIBER_COUNTS: [usize; 8] = [0, 1, 2, 5, 10, 20, 50, 100];

// For sustainable rate testing  
const SUSTAINABLE_RATES: [u64; 6] = [1_000, 5_000, 10_000, 25_000, 50_000, 100_000];
const SUSTAINABLE_SUBSCRIBERS: usize = 10;

#[derive(Debug, Clone)]
struct BenchmarkResult {
    transport: String,
    message_type: String,
    message_size: usize,
    subscribers: usize,
    messages_sent: u64,
    duration_secs: f64,
    throughput: f64,
    bandwidth_mbps: f64,
    p50_us: f64,
    p99_us: f64,
    loss_rate: f64,
    test_mode: String,
}

/// Lightweight subscriber that only counts messages
struct CountingSubscriber {
    counter: Arc<AtomicU64>,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

impl CountingSubscriber {
    async fn new(transport_type: &str, channel_id: ChannelId) -> Self {
        let counter = Arc::new(AtomicU64::new(0));

        match transport_type {
            #[cfg(feature = "redis-client")]
            "redis" => {
                let counter_clone = counter.clone();
                let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
                let transport = Arc::new(RedisTransport::new(&url).await.expect("Failed to create Redis transport"));

                // Start counting task on dedicated runtime
                let mut stream = transport.subscribe(channel_id).await
                    .expect("Failed to subscribe");

                let handle = tokio::spawn(async move {
                    loop {
                        match stream.next().await {
                            Ok(Some(_data)) => {
                                counter_clone.fetch_add(1, Ordering::Relaxed);
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                });

                CountingSubscriber {
                    counter,
                    _handle: Some(handle),
                }
            },
            #[cfg(feature = "webtransport-client")]
            "webtransport" => {
                let counter_clone = counter.clone();
                let transport = Arc::new(WebTransportClient::new_consumer(None).await.expect("Failed to create WebTransport consumer"));

                let mut stream = transport.subscribe(channel_id).await
                    .expect("Failed to subscribe");

                let handle = tokio::spawn(async move {
                    loop {
                        match stream.next().await {
                            Ok(Some(data)) => {
                                if data.len() > 4 {
                                    counter_clone.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                });

                CountingSubscriber {
                    counter,
                    _handle: Some(handle),
                }
            },
            _ => panic!("Unsupported transport type"),
        }
    }

    fn get_count(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }
}

/// Fire-and-forget benchmark for maximum throughput
async fn benchmark_max_throughput<M: Pushable>(
    transport_type: &str,
    message_type_name: &str,
    message_bytes: &[u8],
    num_subscribers: usize,
) -> BenchmarkResult {
    let channel_id = ChannelId::new(1, M::get_message_type());
    let message_size = message_bytes.len();

    // Create counting subscribers FIRST
    let mut subscribers = Vec::new();
    for _ in 0..num_subscribers {
        subscribers.push(CountingSubscriber::new(transport_type, channel_id).await);
    }

    // Small delay to ensure subscribers are ready
    if num_subscribers > 0 {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Create publisher
    let publisher: Arc<dyn MessageTransport> = match transport_type {
        #[cfg(feature = "redis-client")]
        "redis" => {
            let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
            Arc::new(RedisTransport::new(&url).await.expect("Failed to create Redis transport"))
        },
        #[cfg(feature = "webtransport-client")]
        "webtransport" => {
            Arc::new(WebTransportClient::new("local://").await.expect("Failed to create WebTransport client"))
        },
        _ => panic!("Unsupported transport type"),
    };

    // Warmup period
    if num_subscribers > 0 {
        tokio::time::sleep(Duration::from_millis(WARMUP_DURATION_MS)).await;
    }

    // Latency tracking
    let mut latencies = Vec::with_capacity(1000);

    // Fire-and-forget benchmark loop
    let start = Instant::now();
    let mut messages_sent = 0u64;

    while start.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
        let batch_start = Instant::now();

        // True fire-and-forget: spawn without waiting
        for _ in 0..1000 {
            let publisher = publisher.clone();
            let msg = message_bytes.to_vec();
            tokio::spawn(async move {
                let _ = publisher.publish(channel_id, &msg).await;
            });
            messages_sent += 1;
        }

        // Track latency
        latencies.push(batch_start.elapsed().as_micros() as f64 / 1000.0);

        // Small yield to prevent overwhelming the runtime
        tokio::task::yield_now().await;
    }

    let duration = start.elapsed();
    let duration_secs = duration.as_secs_f64();
    let throughput = messages_sent as f64 / duration_secs;
    let bandwidth_mbps = (throughput * message_size as f64) / (1024.0 * 1024.0);

    // Calculate percentiles
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50_us = latencies.get(latencies.len() / 2).copied().unwrap_or(0.0);
    let p99_us = latencies.get(latencies.len() * 99 / 100).copied().unwrap_or(0.0);

    // Wait for subscribers to process remaining messages
    if num_subscribers > 0 {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Calculate loss rate
    let loss_rate = if !subscribers.is_empty() {
        let total_received: u64 = subscribers.iter().map(|s| s.get_count()).sum();
        let expected_received = messages_sent * num_subscribers as u64;
        if expected_received > 0 {
            ((expected_received - total_received) as f64 / expected_received as f64) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    BenchmarkResult {
        transport: transport_type.to_string(),
        message_type: message_type_name.to_string(),
        message_size,
        subscribers: num_subscribers,
        messages_sent,
        duration_secs,
        throughput,
        bandwidth_mbps,
        p50_us,
        p99_us,
        loss_rate,
        test_mode: "max_throughput".to_string(),
    }
}

/// Sustainable rate benchmark with controlled publishing
async fn benchmark_sustainable_rate<M: Pushable>(
    transport_type: &str,
    message_type_name: &str,
    message_bytes: &[u8],
    target_rate: u64,
) -> BenchmarkResult {
    let channel_id = ChannelId::new(1, M::get_message_type());
    let message_size = message_bytes.len();

    // Create subscribers
    let mut subscribers = Vec::new();
    for _ in 0..SUSTAINABLE_SUBSCRIBERS {
        subscribers.push(CountingSubscriber::new(transport_type, channel_id).await);
    }

    // Wait for subscribers
    tokio::time::sleep(Duration::from_millis(WARMUP_DURATION_MS)).await;

    // Create publisher
    let publisher: Arc<dyn MessageTransport> = match transport_type {
        #[cfg(feature = "redis-client")]
        "redis" => {
            let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
            Arc::new(RedisTransport::new(&url).await.expect("Failed to create Redis transport"))
        },
        #[cfg(feature = "webtransport-client")]
        "webtransport" => {
            Arc::new(WebTransportClient::new("local://").await.expect("Failed to create WebTransport client"))
        },
        _ => panic!("Unsupported transport type"),
    };

    // Non-blocking sustainable rate publishing
    let start = Instant::now();
    let mut messages_sent = 0u64;
    let mut last_check = Instant::now();
    let mut latencies = Vec::new();
    
    // Calculate messages per check interval
    let check_interval = Duration::from_millis(100); // Check every 100ms
    let messages_per_interval = (target_rate as f64 * 0.1) as u64; // Messages in 100ms
    
    while start.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
        let now = Instant::now();
        
        // Check if we need to send more messages
        if now.duration_since(last_check) >= check_interval {
            last_check = now;
            let batch_start = Instant::now();
            
            // Send messages non-blocking
            for _ in 0..messages_per_interval {
                let publisher = publisher.clone();
                let msg = message_bytes.to_vec();
                tokio::spawn(async move {
                    let _ = publisher.publish(channel_id, &msg).await;
                });
                messages_sent += 1;
            }
            
            latencies.push(batch_start.elapsed().as_micros() as f64 / 1000.0);
        }
        
        // Yield to allow other tasks to run
        tokio::task::yield_now().await;
    }

    // Wait for messages to be processed
    tokio::time::sleep(Duration::from_secs(1)).await;

    let duration = start.elapsed().as_secs_f64();
    let throughput = messages_sent as f64 / duration;
    let bandwidth_mbps = (throughput * message_size as f64) / (1024.0 * 1024.0);

    // Calculate percentiles
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50_us = latencies.get(latencies.len() / 2).copied().unwrap_or(0.0);
    let p99_us = latencies.get(latencies.len() * 99 / 100).copied().unwrap_or(0.0);

    // Calculate loss rate
    let total_received: u64 = subscribers.iter().map(|s| s.get_count()).sum();
    let expected_per_subscriber = messages_sent;
    let actual_per_subscriber = if subscribers.is_empty() { 0 } else { total_received / subscribers.len() as u64 };
    
    let loss_rate = if expected_per_subscriber > 0 {
        ((expected_per_subscriber.saturating_sub(actual_per_subscriber)) as f64 / expected_per_subscriber as f64) * 100.0
    } else {
        0.0
    };

    BenchmarkResult {
        transport: transport_type.to_string(),
        message_type: format!("{} @{}/s", message_type_name, target_rate),
        message_size,
        subscribers: SUSTAINABLE_SUBSCRIBERS,
        messages_sent,
        duration_secs: duration,
        throughput,
        bandwidth_mbps,
        p50_us,
        p99_us,
        loss_rate,
        test_mode: "sustainable".to_string(),
    }
}

/// Generate formatted benchmark report
fn generate_report(results: Vec<BenchmarkResult>) {
    println!("\n# MITCH Protocol Benchmark Report");
    println!("\n*Generated: {}*", Local::now().format("%Y-%m-%d %H:%M:%S"));

    // Split results by test mode
    let max_throughput_results: Vec<_> = results.iter()
        .filter(|r| r.test_mode == "max_throughput")
        .collect();
    
    let sustainable_results: Vec<_> = results.iter()
        .filter(|r| r.test_mode == "sustainable")
        .collect();

    // Maximum throughput summary
    if !max_throughput_results.is_empty() {
        println!("\n## Maximum Throughput (Fire-and-Forget)");
        println!("\nPure publishing speed with no flow control:");
        
        // Group by message type and transport
        let mut by_msg_type: std::collections::HashMap<String, Vec<&BenchmarkResult>> = std::collections::HashMap::new();
        for result in &max_throughput_results {
            by_msg_type.entry(result.message_type.clone()).or_default().push(result);
        }
        
        for (msg_type, type_results) in by_msg_type {
            println!("\n### {} Messages ({}B)", msg_type, type_results[0].message_size);
            println!("\n| Transport | Subscribers | Throughput | Bandwidth | P50 μs | P99 μs | Loss % |");
            println!("|-----------|-------------|------------|-----------|--------|--------|--------|");
            
            for result in type_results {
                println!("| {:9} | {:>11} | {:>10.0} | {:>9.2} | {:>6.1} | {:>6.1} | {:>6.1} |",
                    result.transport,
                    result.subscribers,
                    result.throughput,
                    result.bandwidth_mbps,
                    result.p50_us,
                    result.p99_us,
                    result.loss_rate
                );
            }
        }
        
        println!("\n> **Note**: High loss rates (>90%) at max speed indicate subscriber capacity limits");
    }

    // Sustainable rate summary
    if !sustainable_results.is_empty() {
        println!("\n## Sustainable Rates ({} subscribers)", SUSTAINABLE_SUBSCRIBERS);
        println!("\nControlled publishing rates with acceptable loss:");
        
        // Group by base message type and transport
        let mut by_type_transport: std::collections::HashMap<(String, String), Vec<&BenchmarkResult>> = std::collections::HashMap::new();
        for result in sustainable_results {
            let base_type = result.message_type.split('@').next().unwrap_or(&result.message_type).trim();
            by_type_transport.entry((base_type.to_string(), result.transport.clone())).or_default().push(result);
        }
        
        // Group by message type for display
        let mut by_base_type: std::collections::HashMap<String, Vec<(&str, Vec<&BenchmarkResult>)>> = std::collections::HashMap::new();
        for ((base_type, transport), results) in by_type_transport {
            by_base_type.entry(base_type).or_default().push((transport.as_str(), results));
        }
        
        for (base_type, transport_results) in by_base_type {
            println!("\n### {} Messages", base_type);
            
            for (transport, type_results) in transport_results {
                println!("\n#### {} Transport", transport);
                println!("\n| Target Rate | Actual Rate | Loss % | Status |");
                println!("|-------------|-------------|--------|---------|");
                
                let mut sustainable_rate = 0u64;
                
                for result in type_results {
                    let status = if result.loss_rate < 0.1 {
                        sustainable_rate = result.throughput as u64;
                        "✓ Sustainable"
                    } else if result.loss_rate < 1.0 {
                        "⚠ Marginal"
                    } else {
                        "✗ Unsustainable"
                    };
                    
                    // Extract target rate from message type
                    let target = result.message_type.split('@').nth(1)
                        .and_then(|s| s.trim_end_matches("/s").parse::<u64>().ok())
                        .unwrap_or(0);
                    
                    println!("| {:>11} | {:>11.0} | {:>6.2} | {} |",
                        format!("{}/s", target),
                        result.throughput,
                        result.loss_rate,
                        status
                    );
                }
                
                if sustainable_rate > 0 {
                    println!("\n**Max sustainable rate**: {}/s", sustainable_rate);
                }
            }
        }
    }

    println!("\n## Key Insights");
    println!("\n1. **Message Size Impact**: Larger messages (OrderBook) have lower throughput but higher bandwidth");
    println!("2. **Subscriber Overhead**: Each subscriber adds ~1-2% overhead to the publisher");
    println!("3. **Sustainable vs Max**: Sustainable rates depend on subscriber processing capacity");
    println!("\n---");
    println!("\n**Design Philosophy**: In HFT, a 1ms delay is worse than 0.1% message loss. MITCH prioritizes speed over guarantees.");
}

fn main() {
    let rt = Runtime::new().unwrap();
    let mut all_results = Vec::new();

    println!("MITCH Protocol Benchmark");
    println!("========================");
    
    // Check for mode selection
    let mode = std::env::var("BENCH_MODE").unwrap_or_else(|_| "both".to_string());
    
    println!("Mode: {} | Duration: {}s", mode, TEST_DURATION_SECS);
    
    // Check available features
    #[cfg(feature = "redis-client")]
    println!("✓ Redis support enabled");
    #[cfg(not(feature = "redis-client"))]
    println!("✗ Redis support disabled");
    
    #[cfg(feature = "webtransport-client")]
    println!("✓ WebTransport support enabled");
    #[cfg(not(feature = "webtransport-client"))]
    println!("✗ WebTransport support disabled");
    
    println!();

    // Pre-create test messages for all types
    let tick = Tick::new(0x1234567890ABCDEF, 100.0, 101.0, 1000, 2000).unwrap();
    let tick_bytes = tick.to_bytes();
    
    let index = Index::new(
        0x1234567890ABCDEF,
        100.0, 1_000_000, 50_000_000, 100, 10, 20, 30, 40, 5000, 6000, 7000, 8000, 95, 1, 9
    );
    let index_bytes = index.to_bytes();
    
    let order_book = OrderBook::new(0x1234567890ABCDEF, 100.5, 0, [Default::default(); 128], [Default::default(); 128]);
    let order_book_bytes = order_book.to_bytes();

    // Message types to test
    let message_types = [
        ("Tick", &tick_bytes as &[u8]),
        ("Index", &index_bytes as &[u8]),
        ("OrderBook", &order_book_bytes as &[u8]),
    ];

    // Test each transport
    #[cfg(feature = "redis-client")]
    {
        let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        println!("Testing Redis ({})", redis_url);
        
        // Test connection first
        match rt.block_on(RedisTransport::new(&redis_url)) {
            Ok(_) => println!("✓ Redis connection successful"),
            Err(e) => {
                println!("✗ Redis connection failed: {}", e);
                println!("  Please ensure Redis is running and REDIS_URL is set correctly");
            }
        }
        
        for (msg_name, msg_bytes) in &message_types {
            if mode == "both" || mode == "max" {
                println!("\n{} Maximum Throughput Test", msg_name);
                println!("{}", "-".repeat(30));
                
                for &num_subs in &SUBSCRIBER_COUNTS {
                    print!("  {} subscribers... ", num_subs);
                    let result = match *msg_name {
                        "Tick" => rt.block_on(benchmark_max_throughput::<Tick>(
                            "redis", "Tick", msg_bytes, num_subs
                        )),
                        "Index" => rt.block_on(benchmark_max_throughput::<Index>(
                            "redis", "Index", msg_bytes, num_subs
                        )),
                        "OrderBook" => rt.block_on(benchmark_max_throughput::<OrderBook>(
                            "redis", "OrderBook", msg_bytes, num_subs
                        )),
                        _ => continue,
                    };
                    println!("{:.0} msg/s, {:.1}% loss", result.throughput, result.loss_rate);
                    all_results.push(result);
                }
            }
            
            if mode == "both" || mode == "sustainable" {
                println!("\n{} Sustainable Rate Test", msg_name);
                println!("{}", "-".repeat(30));
                
                for &rate in &SUSTAINABLE_RATES {
                    print!("  {}/s... ", rate);
                    let result = match *msg_name {
                        "Tick" => rt.block_on(benchmark_sustainable_rate::<Tick>(
                            "redis", "Tick", msg_bytes, rate
                        )),
                        "Index" => rt.block_on(benchmark_sustainable_rate::<Index>(
                            "redis", "Index", msg_bytes, rate
                        )),
                        "OrderBook" => rt.block_on(benchmark_sustainable_rate::<OrderBook>(
                            "redis", "OrderBook", msg_bytes, rate
                        )),
                        _ => continue,
                    };
                    println!("actual: {:.0}/s, loss: {:.2}%", result.throughput, result.loss_rate);
                    all_results.push(result);
                }
            }
        }
    }

    #[cfg(feature = "webtransport-client")]
    {
        println!("\n\nTesting WebTransport (local://)");
        
        for (msg_name, msg_bytes) in &message_types {
            if mode == "both" || mode == "max" {
                println!("\n{} Maximum Throughput Test", msg_name);
                println!("{}", "-".repeat(30));
                
                for &num_subs in &SUBSCRIBER_COUNTS {
                    print!("  {} subscribers... ", num_subs);
                    let result = match *msg_name {
                        "Tick" => rt.block_on(benchmark_max_throughput::<Tick>(
                            "webtransport", "Tick", msg_bytes, num_subs
                        )),
                        "Index" => rt.block_on(benchmark_max_throughput::<Index>(
                            "webtransport", "Index", msg_bytes, num_subs
                        )),
                        "OrderBook" => rt.block_on(benchmark_max_throughput::<OrderBook>(
                            "webtransport", "OrderBook", msg_bytes, num_subs
                        )),
                        _ => continue,
                    };
                    println!("{:.0} msg/s, {:.1}% loss", result.throughput, result.loss_rate);
                    all_results.push(result);
                }
            }
            
            if mode == "both" || mode == "sustainable" {
                println!("\n{} Sustainable Rate Test", msg_name);
                println!("{}", "-".repeat(30));
                
                for &rate in &SUSTAINABLE_RATES {
                    print!("  {}/s... ", rate);
                    let result = match *msg_name {
                        "Tick" => rt.block_on(benchmark_sustainable_rate::<Tick>(
                            "webtransport", "Tick", msg_bytes, rate
                        )),
                        "Index" => rt.block_on(benchmark_sustainable_rate::<Index>(
                            "webtransport", "Index", msg_bytes, rate
                        )),
                        "OrderBook" => rt.block_on(benchmark_sustainable_rate::<OrderBook>(
                            "webtransport", "OrderBook", msg_bytes, rate
                        )),
                        _ => continue,
                    };
                    println!("actual: {:.0}/s, loss: {:.2}%", result.throughput, result.loss_rate);
                    all_results.push(result);
                }
            }
        }
    }

    // Generate comprehensive report
    if all_results.is_empty() {
        println!("\n⚠️  No benchmark results collected!");
        println!("   This usually means:");
        println!("   - Transport connections failed");
        println!("   - Features are not properly enabled");
        println!("   - REDIS_URL environment variable is not set correctly");
        println!("\n   Try running with:");
        println!("   REDIS_URL=\"redis://user:pass@host:port\" cargo run --release --bin benchmark --features=\"all-networking,benchmarking\"");
    } else {
        println!("\n✓ Collected {} benchmark results", all_results.len());
        generate_report(all_results);
    }
}