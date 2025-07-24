//! Redis pub/sub integration test

#[cfg(all(test, feature = "networking", feature = "redis-client"))]
mod tests {
    use mitch::networking::redis::RedisTransport;
    use mitch::networking::{MessageTransport, Pushable};
    use mitch::{ChannelId, Tick, Trade, Order, OrderSide, OrderType};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    // use std::time::Duration;
    // use tokio::time::sleep;

    #[tokio::test]
    async fn test_redis_pubsub_basic() {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let transport = match RedisTransport::new(&redis_url).await {
        Ok(client) => client,
        Err(e) => {
            println!("[Redis] Skipping test - cannot connect to Redis at {}: {}", redis_url, e);
            if e.to_string().contains("NOAUTH") {
                println!("[Redis] Hint: Set REDIS_PASSWORD environment variable for authenticated Redis");
            }
            return;
        }
    };

    // Test basic publish functionality
    let tick_channel = ChannelId::new(1, 't');
    let trade_channel = ChannelId::new(1, 'r');
    let order_channel = ChannelId::new(1, 'o');

    println!("[Redis] Testing pub/sub with binary channel IDs");

    // Test Tick message
    let tick = Tick::new(1, 100.0, 100.5, 1000, 1100).unwrap();
    let tick_bytes = tick.to_bytes();
    assert_eq!(tick_bytes.len(), 32, "Tick should be 32 bytes");

    match transport.publish(tick_channel, &tick_bytes).await {
        Ok(_) => println!("  ✓ Published Tick message to channel {:?}", tick_channel),
        Err(e) => {
            if e.to_string().contains("NOAUTH") {
                println!("[Redis] Skipping test - Redis requires authentication");
                println!("[Redis] Hint: Set REDIS_PASSWORD environment variable");
                return;
            }
            panic!("Failed to publish Tick: {}", e);
        }
    }

    // Test Trade message
    let trade = Trade::new(2, 100.25, 500, 1, OrderSide::Buy).unwrap();
    let trade_bytes = trade.to_bytes();
    assert_eq!(trade_bytes.len(), 32, "Trade should be 32 bytes");

    match transport.publish(trade_channel, &trade_bytes).await {
        Ok(_) => println!("  ✓ Published Trade message to channel {:?}", trade_channel),
        Err(e) => panic!("Failed to publish Trade: {}", e),
    }

    // Test Order message
    let order = Order::new(3, 1, 99.75, 200, OrderType::Limit, OrderSide::Sell, 0).unwrap();
    let order_bytes = order.to_bytes();
    assert_eq!(order_bytes.len(), 32, "Order should be 32 bytes");

    match transport.publish(order_channel, &order_bytes).await {
        Ok(_) => println!("  ✓ Published Order message to channel {:?}", order_channel),
        Err(e) => panic!("Failed to publish Order: {}", e),
    }

    // Test high-throughput publishing
    println!("\n[Redis] Testing high-throughput publishing - 1000 messages");
    let sent_count = Arc::new(AtomicU64::new(0));
    let start = std::time::Instant::now();

    for seq in 1..=1000 {
        let channel = match seq % 3 {
            0 => tick_channel,
            1 => trade_channel,
            _ => order_channel,
        };

        let tick = Tick::new(seq, 100.0 + (seq as f64 * 0.01), 100.5 + (seq as f64 * 0.01), 1000, 1100).unwrap();

        if transport.publish(channel, &tick.to_bytes()).await.is_ok() {
            sent_count.fetch_add(1, Ordering::Relaxed);
        }

        if seq % 100 == 0 {
            print!("\r  Progress: {}/1000", seq);
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
    }

    let elapsed = start.elapsed();
    let total_sent = sent_count.load(Ordering::Relaxed);
    let rate = total_sent as f64 / elapsed.as_secs_f64();

    println!("\r  ✓ Published {} messages in {:.2}s ({:.0} msg/s)", total_sent, elapsed.as_secs_f64(), rate);
    assert_eq!(total_sent, 1000, "Should have sent all 1000 messages");

    // Test batch publishing with Push trait
    println!("\n[Redis] Testing batch publishing with Push trait");
    let clients: Vec<Arc<dyn MessageTransport>> = vec![Arc::new(transport)];

    let tick = Tick::new(9999, 105.0, 105.5, 2000, 2100).unwrap();
    match tick.push(&clients, None).await {
        Ok(_) => println!("  ✓ Successfully used Push trait for Tick message"),
        Err(e) => panic!("Failed to push Tick: {}", e),
    }

    println!("\n[Redis] All tests passed!");
    }
}
