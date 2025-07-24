//! Redis pub/sub complete message integrity verification test

#[cfg(all(test, feature = "networking", feature = "redis-client"))]
mod tests {
    use mitch::networking::redis::RedisTransport;
    use mitch::networking::{MessageTransport, Pushable};
    use mitch::{ChannelId, Tick, Trade, Order, OrderSide, OrderType};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::time::sleep;
    use futures::StreamExt;
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    struct SentMessage {
        ticker_id: u64,
        message_type: char,
        // Store the original message as specific type for verification
        tick: Option<Tick>,
        trade: Option<Trade>,
        order: Option<Order>,
    }

    #[tokio::test]
    async fn test_redis_pubsub_complete_integrity() {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    // Create publisher transport
    let publisher = match RedisTransport::new(&redis_url).await {
        Ok(client) => client,
        Err(e) => {
            println!("[Redis] Skipping test - cannot connect to Redis at {}: {}", redis_url, e);
            return;
        }
    };

    // Create Redis client for subscriber
    let redis_client = redis::Client::open(redis_url.as_str()).expect("Failed to create Redis client");
    let mut pubsub_conn = redis_client.get_async_pubsub().await.expect("Failed to get pubsub connection");

    // Store sent messages for comparison
    let sent_messages = Arc::new(Mutex::new(HashMap::<u64, SentMessage>::new()));
    
    // Message counters
    let sent_count = Arc::new(AtomicU64::new(0));
    let received_count = Arc::new(AtomicU64::new(0));
    let field_verified = Arc::new(AtomicU64::new(0));
    let field_errors = Arc::new(AtomicU64::new(0));

    // Channels
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

    println!("[Redis] Starting complete integrity verification test");
    println!("  - Verifying all fields in Tick, Trade, and Order messages");

    // Spawn subscriber task
    let recv_count = received_count.clone();
    let verified = field_verified.clone();
    let field_errs = field_errors.clone();
    let sent_msgs = sent_messages.clone();
    
    let sub_handle = tokio::spawn(async move {
        let mut stream = pubsub_conn.on_message();
        let start = std::time::Instant::now();
        
        while let Some(msg) = stream.next().await {
            // Get channel as binary
            let channel_bytes: Vec<u8> = msg.get_channel().unwrap();
            if channel_bytes.len() != 4 {
                field_errs.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            
            // Reconstruct channel ID
            let mut channel_id_bytes = [0u8; 4];
            channel_id_bytes.copy_from_slice(&channel_bytes);
            let channel_id = ChannelId { raw: u32::from_le_bytes(channel_id_bytes) };
            
            // Get message data
            let data: Vec<u8> = msg.get_payload().unwrap();
            let count = recv_count.fetch_add(1, Ordering::Relaxed) + 1;
            
            // Extract ticker_id for lookup
            if data.len() < 8 {
                field_errs.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            
            let ticker_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
            
            // Look up original message
            let sent_map = sent_msgs.lock().unwrap();
            if let Some(original) = sent_map.get(&ticker_id) {
                match channel_id.msg_type() {
                    't' => {
                        if let (Ok(received_tick), Some(sent_tick)) = (Tick::unpack(&data), &original.tick) {
                            let mut errors = 0;
                            
                            // Copy fields to locals to avoid alignment issues
                            let recv_ticker_id = received_tick.ticker_id;
                            let sent_ticker_id = sent_tick.ticker_id;
                            let recv_bid = received_tick.bid_price;
                            let sent_bid = sent_tick.bid_price;
                            let recv_ask = received_tick.ask_price;
                            let sent_ask = sent_tick.ask_price;
                            let recv_bid_vol = received_tick.bid_volume;
                            let sent_bid_vol = sent_tick.bid_volume;
                            let recv_ask_vol = received_tick.ask_volume;
                            let sent_ask_vol = sent_tick.ask_volume;
                            
                            if recv_ticker_id != sent_ticker_id {
                                errors += 1;
                                eprintln!("\n[Redis] Tick ticker_id mismatch: {} vs {}", recv_ticker_id, sent_ticker_id);
                            }
                            if (recv_bid - sent_bid).abs() > f64::EPSILON {
                                errors += 1;
                                eprintln!("\n[Redis] Tick bid mismatch: {} vs {}", recv_bid, sent_bid);
                            }
                            if (recv_ask - sent_ask).abs() > f64::EPSILON {
                                errors += 1;
                                eprintln!("\n[Redis] Tick ask mismatch: {} vs {}", recv_ask, sent_ask);
                            }
                            if recv_bid_vol != sent_bid_vol {
                                errors += 1;
                                eprintln!("\n[Redis] Tick bid_volume mismatch: {} vs {}", recv_bid_vol, sent_bid_vol);
                            }
                            if recv_ask_vol != sent_ask_vol {
                                errors += 1;
                                eprintln!("\n[Redis] Tick ask_volume mismatch: {} vs {}", recv_ask_vol, sent_ask_vol);
                            }
                            
                            if errors == 0 {
                                verified.fetch_add(1, Ordering::Relaxed);
                                print!("\r[Redis] Tick #{} ✓ bid={:.2} ask={:.2} bvol={} avol={} ({})", 
                                       ticker_id, recv_bid, recv_ask, recv_bid_vol, recv_ask_vol, count);
                            } else {
                                field_errs.fetch_add(errors, Ordering::Relaxed);
                            }
                        }
                    }
                    'r' => {
                        if let (Ok(received_trade), Some(sent_trade)) = (Trade::unpack(&data), &original.trade) {
                            let mut errors = 0;
                            
                            // Copy fields to locals
                            let recv_ticker_id = received_trade.ticker_id;
                            let sent_ticker_id = sent_trade.ticker_id;
                            let recv_price = received_trade.price;
                            let sent_price = sent_trade.price;
                            let recv_qty = received_trade.quantity;
                            let sent_qty = sent_trade.quantity;
                            let recv_trade_id = received_trade.trade_id;
                            let sent_trade_id = sent_trade.trade_id;
                            let recv_side = received_trade.side;
                            let sent_side = sent_trade.side;
                            
                            if recv_ticker_id != sent_ticker_id {
                                errors += 1;
                                eprintln!("\n[Redis] Trade ticker_id mismatch");
                            }
                            if (recv_price - sent_price).abs() > f64::EPSILON {
                                errors += 1;
                                eprintln!("\n[Redis] Trade price mismatch: {} vs {}", recv_price, sent_price);
                            }
                            if recv_qty != sent_qty {
                                errors += 1;
                                eprintln!("\n[Redis] Trade quantity mismatch");
                            }
                            if recv_trade_id != sent_trade_id {
                                errors += 1;
                                eprintln!("\n[Redis] Trade trade_id mismatch");
                            }
                            if recv_side != sent_side {
                                errors += 1;
                                eprintln!("\n[Redis] Trade side mismatch");
                            }
                            
                            if errors == 0 {
                                verified.fetch_add(1, Ordering::Relaxed);
                                let side = if recv_side == OrderSide::Buy { "BUY" } else { "SELL" };
                                print!("\r[Redis] Trade #{} ✓ price={:.2} qty={} side={} ({})", 
                                       ticker_id, recv_price, recv_qty, side, count);
                            } else {
                                field_errs.fetch_add(errors, Ordering::Relaxed);
                            }
                        }
                    }
                    'o' => {
                        if let (Ok(received_order), Some(sent_order)) = (Order::unpack(&data), &original.order) {
                            let mut errors = 0;
                            
                            // Copy fields to locals
                            let recv_ticker_id = received_order.ticker_id;
                            let sent_ticker_id = sent_order.ticker_id;
                            let recv_order_id = received_order.order_id;
                            let sent_order_id = sent_order.order_id;
                            let recv_price = received_order.price;
                            let sent_price = sent_order.price;
                            let recv_qty = received_order.quantity;
                            let sent_qty = sent_order.quantity;
                            let recv_type_side = received_order.type_and_side;
                            let sent_type_side = sent_order.type_and_side;
                            
                            if recv_ticker_id != sent_ticker_id {
                                errors += 1;
                                eprintln!("\n[Redis] Order ticker_id mismatch");
                            }
                            if recv_order_id != sent_order_id {
                                errors += 1;
                                eprintln!("\n[Redis] Order order_id mismatch");
                            }
                            if (recv_price - sent_price).abs() > f64::EPSILON {
                                errors += 1;
                                eprintln!("\n[Redis] Order price mismatch: {} vs {}", recv_price, sent_price);
                            }
                            if recv_qty != sent_qty {
                                errors += 1;
                                eprintln!("\n[Redis] Order quantity mismatch");
                            }
                            if recv_type_side != sent_type_side {
                                errors += 1;
                                eprintln!("\n[Redis] Order type_and_side mismatch");
                            }
                            
                            if errors == 0 {
                                verified.fetch_add(1, Ordering::Relaxed);
                                let side = if recv_type_side & 1 == 1 { "SELL" } else { "BUY" };
                                let order_type = if recv_type_side & 2 == 2 { "LIMIT" } else { "MARKET" };
                                print!("\r[Redis] Order #{} ✓ price={:.2} qty={} {} {} ({})", 
                                       ticker_id, recv_price, recv_qty, side, order_type, count);
                            } else {
                                field_errs.fetch_add(errors, Ordering::Relaxed);
                            }
                        }
                    }
                    _ => {
                        field_errs.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
            
            // Stop after receiving all messages or timeout
            if count >= 30 || start.elapsed() > Duration::from_secs(5) {
                break;
            }
        }
        
        println!("\n[Redis] Subscriber finished");
    });

    // Give subscriber time to set up
    sleep(Duration::from_millis(100)).await;

    // Publisher task - send 30 test messages with specific values
    let sent = sent_count.clone();
    let sent_msgs = sent_messages.clone();
    
    let pub_handle = tokio::spawn(async move {
        let mut seq = 1u64;
        
        for i in 0..30 {
            let msg_num = i + 1;
            
            // Create messages with specific test values
            match msg_num % 3 {
                0 => {
                    // Tick with specific values
                    let bid = 100.0 + (seq as f64 * 0.25);
                    let ask = bid + 0.50;
                    let bid_vol = 1000 + (seq as u32 * 10);
                    let ask_vol = 1100 + (seq as u32 * 10);
                    
                    let tick = Tick::new(seq, bid, ask, bid_vol, ask_vol).unwrap();
                    let data = tick.to_bytes();
                    
                    // Store for verification
                    sent_msgs.lock().unwrap().insert(seq, SentMessage {
                        ticker_id: seq,
                        message_type: 't',
                        tick: Some(tick),
                        trade: None,
                        order: None,
                    });
                    
                    if let Err(e) = publisher.publish(tick_channel, &data).await {
                        eprintln!("\n[Redis] Failed to publish Tick: {}", e);
                        continue;
                    }
                }
                1 => {
                    // Trade with specific values
                    let price = 99.99 + (seq as f64 * 0.01);
                    let quantity = 500 + (seq as u32 * 50);
                    let side = if seq % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell };
                    
                    let trade = Trade::new(seq, price, quantity, seq as u32, side).unwrap();
                    let data = trade.to_bytes();
                    
                    sent_msgs.lock().unwrap().insert(seq, SentMessage {
                        ticker_id: seq,
                        message_type: 'r',
                        tick: None,
                        trade: Some(trade),
                        order: None,
                    });
                    
                    if let Err(e) = publisher.publish(trade_channel, &data).await {
                        eprintln!("\n[Redis] Failed to publish Trade: {}", e);
                        continue;
                    }
                }
                _ => {
                    // Order with specific values
                    let price = 98.50 + (seq as f64 * 0.10);
                    let quantity = 250 + (seq as u32 * 25);
                    let order_type = if seq % 3 == 0 { OrderType::Market } else { OrderType::Limit };
                    let side = if seq % 2 == 1 { OrderSide::Buy } else { OrderSide::Sell };
                    
                    let order = Order::new(seq, seq as u32, price, quantity, order_type, side, 0).unwrap();
                    let data = order.to_bytes();
                    
                    sent_msgs.lock().unwrap().insert(seq, SentMessage {
                        ticker_id: seq,
                        message_type: 'o',
                        tick: None,
                        trade: None,
                        order: Some(order),
                    });
                    
                    if let Err(e) = publisher.publish(order_channel, &data).await {
                        eprintln!("\n[Redis] Failed to publish Order: {}", e);
                        continue;
                    }
                }
            }
            
            sent.fetch_add(1, Ordering::Relaxed);
            seq += 1;
            
            // Small delay to ensure ordering
            sleep(Duration::from_millis(10)).await;
        }
        
        println!("\n[Redis] Publisher sent 30 test messages");
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
    let total_verified = field_verified.load(Ordering::Relaxed);
    let total_field_errors = field_errors.load(Ordering::Relaxed);
    
    println!("\n[Redis] Complete Integrity Test Results:");
    println!("  Messages sent:        {}", total_sent);
    println!("  Messages received:    {}", total_received);
    println!("  Fields verified:      {}", total_verified);
    println!("  Field errors:         {}", total_field_errors);
    println!("  Loss rate:            {:.2}%", if total_sent > 0 { ((total_sent - total_received) as f64 / total_sent as f64) * 100.0 } else { 0.0 });
    println!("  Binary encoding:      {}", if total_received > 0 { "✓ PASS" } else { "✗ FAIL" });
    println!("  Field integrity:      {}", if total_field_errors == 0 && total_verified == total_received { "✓ PASS" } else { "✗ FAIL" });
    
    println!("\n  Verified message fields:");
    println!("  - Tick: ticker_id, bid_price, ask_price, bid_volume, ask_volume");
    println!("  - Trade: ticker_id, price, quantity, trade_id, side");
    println!("  - Order: ticker_id, order_id, price, quantity, type_and_side");
    
    // Verify test results
    if total_sent > 0 {
        assert_eq!(total_sent, 30, "Should have sent exactly 30 messages");
        assert_eq!(total_field_errors, 0, "Should have no field errors");
        assert_eq!(total_verified, total_received, "All received messages should be verified");
        
        println!("\n[Redis] ✓ Complete message integrity verified - all fields match!");
    }
    }
}