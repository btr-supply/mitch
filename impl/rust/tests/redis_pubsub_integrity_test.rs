//! Redis pub/sub integrity test using direct Redis API

#[cfg(all(test, feature = "networking", feature = "redis-client"))]
mod tests {
    use mitch::networking::redis::RedisTransport;
    use mitch::networking::{MessageTransport, Pushable};
    use mitch::{ChannelId, Tick, Trade, Order, OrderSide, OrderType};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    // use redis::AsyncCommands;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_redis_pubsub_integrity() {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    // Create publisher transport
    let publisher = match RedisTransport::new(&redis_url).await {
        Ok(client) => client,
        Err(e) => {
            println!("[Redis] Skipping test - cannot connect to Redis at {}: {}", redis_url, e);
            if e.to_string().contains("NOAUTH") {
                println!("[Redis] Hint: Set REDIS_PASSWORD environment variable for authenticated Redis");
            }
            return;
        }
    };

    // Create Redis client for subscriber (using direct Redis API)
    let redis_client = redis::Client::open(redis_url.as_str()).expect("Failed to create Redis client");
    let mut pubsub_conn = redis_client.get_async_pubsub().await.expect("Failed to get pubsub connection");

    // Message counters and integrity verification
    let sent_count = Arc::new(AtomicU64::new(0));
    let received_count = Arc::new(AtomicU64::new(0));
    let integrity_errors = Arc::new(AtomicU64::new(0));
    let integrity_passed = Arc::new(AtomicU64::new(0));

    // Channels for different message types
    let tick_channel = ChannelId::new(1, 't');
    let trade_channel = ChannelId::new(1, 'r');
    let order_channel = ChannelId::new(1, 'o');

    // Subscribe to binary channels
    let tick_channel_bytes = tick_channel.raw.to_le_bytes();
    let trade_channel_bytes = trade_channel.raw.to_le_bytes();
    let order_channel_bytes = order_channel.raw.to_le_bytes();

    pubsub_conn.subscribe(&tick_channel_bytes[..]).await.expect("Failed to subscribe to tick channel");
    pubsub_conn.subscribe(&trade_channel_bytes[..]).await.expect("Failed to subscribe to trade channel");
    pubsub_conn.subscribe(&order_channel_bytes[..]).await.expect("Failed to subscribe to order channel");

    println!("[Redis] Starting pub/sub integrity test");
    println!("  - Subscribed to 3 binary channels");
    println!("  - Will publish 300 messages and verify integrity");

    // Spawn subscriber task
    let recv_count = received_count.clone();
    let errors = integrity_errors.clone();
    let passed = integrity_passed.clone();

    let sub_handle = tokio::spawn(async move {
        let mut stream = pubsub_conn.on_message();
        let start = std::time::Instant::now();

        while let Some(msg) = stream.next().await {
            // Get channel as binary
            let channel_bytes: Vec<u8> = msg.get_channel().unwrap();
            if channel_bytes.len() != 4 {
                errors.fetch_add(1, Ordering::Relaxed);
                eprintln!("\n[Redis] Invalid channel length: {} bytes", channel_bytes.len());
                continue;
            }

            // Reconstruct channel ID
            let mut channel_id_bytes = [0u8; 4];
            channel_id_bytes.copy_from_slice(&channel_bytes);
            let channel_id = ChannelId { raw: u32::from_le_bytes(channel_id_bytes) };

            // Get message data
            let data: Vec<u8> = msg.get_payload().unwrap();
            let count = recv_count.fetch_add(1, Ordering::Relaxed) + 1;

            // Verify message integrity
            match channel_id.msg_type() {
                't' => {
                    if data.len() != 32 {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\n[Redis] Tick size error: expected 32, got {}", data.len());
                    } else {
                        // Verify we can decode the ticker_id
                        let ticker_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
                        passed.fetch_add(1, Ordering::Relaxed);
                        print!("\r[Redis] Received Tick #{} (total: {}, passed: {})",
                               ticker_id, count, passed.load(Ordering::Relaxed));
                    }
                }
                'r' => {
                    if data.len() != 32 {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\n[Redis] Trade size error: expected 32, got {}", data.len());
                    } else {
                        let ticker_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
                        passed.fetch_add(1, Ordering::Relaxed);
                        print!("\r[Redis] Received Trade #{} (total: {}, passed: {})",
                               ticker_id, count, passed.load(Ordering::Relaxed));
                    }
                }
                'o' => {
                    if data.len() != 32 {
                        errors.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\n[Redis] Order size error: expected 32, got {}", data.len());
                    } else {
                        let ticker_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
                        passed.fetch_add(1, Ordering::Relaxed);
                        print!("\r[Redis] Received Order #{} (total: {}, passed: {})",
                               ticker_id, count, passed.load(Ordering::Relaxed));
                    }
                }
                _ => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("\n[Redis] Unknown message type: {}", channel_id.msg_type());
                }
            }

            use std::io::{self, Write};
            io::stdout().flush().unwrap();

            // Stop after receiving all messages or timeout
            if count >= 300 || start.elapsed() > Duration::from_secs(5) {
                break;
            }
        }

        println!("\n[Redis] Subscriber finished");
    });

    // Give subscriber time to set up
    sleep(Duration::from_millis(100)).await;

    // Publisher task - send 300 messages at controlled rate
    let sent = sent_count.clone();
    let pub_handle = tokio::spawn(async move {
        let mut seq = 1u64;

        for batch in 0..30 { // 30 batches of 10 messages
            for i in 0..10 {
                let msg_num = batch * 10 + i + 1;

                // Rotate through message types and publish
                match msg_num % 3 {
                    0 => {
                        let tick = Tick::new(seq, 100.0 + (seq as f64 * 0.01), 100.5 + (seq as f64 * 0.01), 1000, 1100).unwrap();
                        if let Err(e) = publisher.publish(tick_channel, &tick.to_bytes()).await {
                            if e.to_string().contains("NOAUTH") {
                                println!("\n[Redis] Authentication required - skipping test");
                                return;
                            }
                            eprintln!("\n[Redis] Failed to publish Tick: {}", e);
                            continue;
                        }
                    }
                    1 => {
                        let trade = Trade::new(seq, 100.0 + (seq as f64 * 0.01), 1000, 1, OrderSide::Buy).unwrap();
                        if let Err(e) = publisher.publish(trade_channel, &trade.to_bytes()).await {
                            if e.to_string().contains("NOAUTH") {
                                println!("\n[Redis] Authentication required - skipping test");
                                return;
                            }
                            eprintln!("\n[Redis] Failed to publish Trade: {}", e);
                            continue;
                        }
                    }
                    _ => {
                        let order = Order::new(seq, 1, 99.0 + (seq as f64 * 0.01), 500, OrderType::Limit, OrderSide::Sell, 0).unwrap();
                        if let Err(e) = publisher.publish(order_channel, &order.to_bytes()).await {
                            if e.to_string().contains("NOAUTH") {
                                println!("\n[Redis] Authentication required - skipping test");
                                return;
                            }
                            eprintln!("\n[Redis] Failed to publish Order: {}", e);
                            continue;
                        }
                    }
                }

                sent.fetch_add(1, Ordering::Relaxed);
                seq += 1;
            }

            // Small delay between batches to ensure ordering
            sleep(Duration::from_millis(10)).await;
        }

        println!("\n[Redis] Publisher sent 300 messages");
    });

    // Wait for publisher to finish
    pub_handle.await.unwrap();

    // Give subscriber time to receive remaining messages
    sleep(Duration::from_millis(500)).await;

    // Cancel subscriber if still running
    sub_handle.abort();

    // Print final statistics
    let total_sent = sent_count.load(Ordering::Relaxed);
    let total_received = received_count.load(Ordering::Relaxed);
    let total_errors = integrity_errors.load(Ordering::Relaxed);
    let total_passed = integrity_passed.load(Ordering::Relaxed);
    let loss_rate = if total_sent > 0 {
        ((total_sent - total_received) as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };

    println!("\n[Redis] Integrity Test Results:");
    println!("  Messages sent:      {}", total_sent);
    println!("  Messages received:  {}", total_received);
    println!("  Integrity passed:   {}", total_passed);
    println!("  Integrity errors:   {}", total_errors);
    println!("  Loss rate:          {:.2}%", loss_rate);
    println!("  Channel routing:    {}", if total_errors == 0 { "✓ PASS" } else { "✗ FAIL" });
    println!("  Binary encoding:    {}", if total_passed == total_received { "✓ PASS" } else { "✗ FAIL" });

    // Verify test results
    if total_sent > 0 {
        assert!(total_sent >= 290, "Should have sent at least 290 messages");
        assert_eq!(total_errors, 0, "Should have no integrity errors");
        assert_eq!(total_passed, total_received, "All received messages should pass integrity check");
        assert!(loss_rate < 10.0, "Loss rate should be under 10%");

        println!("\n[Redis] ✓ All integrity checks PASSED!");
    }
    }
}
