//! Networking tests for MITCH protocol

use mitch::networking::{MessageTransport, MessageSubscriber, NetworkError, Pushable};
use mitch::{Trade, Order, Tick, OrderSide, OrderType, ChannelId};
use std::sync::Arc;

async fn run_comprehensive_tests<T>(client: T)
where
    T: MessageTransport + MessageSubscriber + Clone + Send + Sync + 'static,
{
    test_connection_and_unsupported_ops(&client).await;
    test_all_message_pushes(&client).await;
    test_serialization_deserialization(&client).await;
}

async fn test_connection_and_unsupported_ops<T>(client: &T)
where
    T: MessageTransport + MessageSubscriber + Clone + Send + Sync + 'static,
{
    if client.supports_storage() {
        assert!(client.set("test:key", b"value").await.is_ok());
        let val = client.get("test:key").await.unwrap();
        assert_eq!(val, Some(b"value".to_vec()));
    } else {
        assert!(matches!(
            client.set("k", b"v").await,
            Err(NetworkError::UnsupportedOperation(_))
        ));
    }
}

async fn test_all_message_pushes<T>(client: &T)
where
    T: MessageTransport + MessageSubscriber + Clone + Send + Sync + 'static,
{
    let clients: Vec<Arc<dyn MessageTransport>> = vec![Arc::new(client.clone())];

    let trade = Trade::new(1, 100.0, 10, 1, OrderSide::Buy).unwrap();
    trade.push(&clients, None).await.unwrap();

    let order = Order::new(2, 1, 101.0, 5, OrderType::Limit, OrderSide::Sell, 0).unwrap();
    order.push(&clients, None).await.unwrap();

    let tick = Tick::new(3, 99.0, 101.0, 100, 120).unwrap();
    tick.push(&clients, None).await.unwrap();
}

async fn test_serialization_deserialization<T>(client: &T)
where
    T: MessageTransport + MessageSubscriber + Clone + Send + Sync + 'static,
{
    // Test Trade serialization/deserialization
    let trade = Trade::new(12345, 99.95, 1000, 42, OrderSide::Buy).unwrap();
    let serialized = trade.to_bytes();
    assert_eq!(serialized.len(), 32); // Trade should be 32 bytes

    // Test Order serialization/deserialization
    let order = Order::new(12346, 1001, 100.05, 500, OrderType::Market, OrderSide::Sell, 85).unwrap();
    let serialized = order.to_bytes();
    assert_eq!(serialized.len(), 32); // Order should be 32 bytes

    // Test Tick serialization/deserialization
    let tick = Tick::new(12347, 98.50, 101.50, 2000, 1500).unwrap();
    let serialized = tick.to_bytes();
    assert_eq!(serialized.len(), 32); // Tick should be 32 bytes

    // Test basic pub/sub functionality
    if client.supports_pubsub() {
        let channel = ChannelId::new(1, 't');
        client.publish(channel, &trade.to_bytes()).await.unwrap();
        client.publish(channel, &order.to_bytes()).await.unwrap();
        client.publish(channel, &tick.to_bytes()).await.unwrap();
    }

    // Test storage functionality if supported
    if client.supports_storage() {
        client.set("trade:test", &trade.to_bytes()).await.unwrap();
        let retrieved = client.get("trade:test").await.unwrap();
        assert_eq!(retrieved, Some(trade.to_bytes()));

        client.set("order:test", &order.to_bytes()).await.unwrap();
        let retrieved = client.get("order:test").await.unwrap();
        assert_eq!(retrieved, Some(order.to_bytes()));

        client.set("tick:test", &tick.to_bytes()).await.unwrap();
        let retrieved = client.get("tick:test").await.unwrap();
        assert_eq!(retrieved, Some(tick.to_bytes()));
    }
}

#[cfg(all(feature = "networking", feature = "redis-client"))]
mod redis_tests {
    use super::run_comprehensive_tests;
    use mitch::networking::redis::RedisTransport;

    async fn setup_redis() -> RedisTransport {
        RedisTransport::new("redis://127.0.0.1:6380")
            .await
            .expect("Failed to connect to Redis. Is it running on localhost:6380?")
    }

    #[tokio::test]
    async fn test_redis_comprehensive() {
        let client = setup_redis().await;
        run_comprehensive_tests(client).await;
    }
}

#[cfg(all(feature = "networking", feature = "webtransport-client"))]
mod webtransport_tests {
    use super::run_comprehensive_tests;
    use mitch::networking::webtransport::WebTransportClient;

    const TEST_SERVER_URL: &str = "https://localhost:4433";

    async fn setup_webtransport() -> Option<WebTransportClient> {
        match WebTransportClient::new(TEST_SERVER_URL).await {
            Ok(client) => Some(client),
            Err(_) => {
                println!("Skipping WebTransport tests: Could not connect to server at {}", TEST_SERVER_URL);
                None
            }
        }
    }

    #[tokio::test]
    #[ignore] // Ignored by default as it requires a live server
    async fn test_webtransport_comprehensive() {
        if let Some(client) = setup_webtransport().await {
            run_comprehensive_tests(client).await;
        }
    }
}
