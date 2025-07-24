//! Redis Transport Implementation for MITCH Protocol
//!
//! Provides Redis-based transport using RESP3 protocol with pub/sub and key-value operations.
//! Compatible with Redis, Valkey, and DragonflyDB.

use redis::{
    Client,
    aio::{PubSub, MultiplexedConnection},
    AsyncCommands,
};
use std::sync::Arc;
use futures::StreamExt;
use url::Url;
use tokio::sync::Mutex;
use crate::{ChannelId, NetworkError, MessageTransport, MessageSubscriber};

/// Redis transport implementation using async Redis client with persistent connections
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct RedisTransport {
    client: Arc<Client>,
    /// Pre-established multiplexed connection for ultra-low latency operations
    connection: Arc<Mutex<MultiplexedConnection>>,
}

impl RedisTransport {
    /// Create a new Redis transport with pre-established connection
    /// 
    /// # Arguments
    /// * `redis_url` - Redis URL with optional authentication (e.g., "redis://user:pass@host:port")
    pub async fn new(redis_url: &str) -> Result<Self, NetworkError> {
        let url = Url::parse(redis_url)
            .map_err(|e| NetworkError::Config(format!("Invalid Redis URL: {}", e)))?;

        let client = Client::open(url.as_str())
            .map_err(|e| NetworkError::Redis(format!("Failed to create Redis client: {}", e)))?;

        // Pre-establish a persistent multiplexed connection for ultra-low latency
        let connection = client.get_multiplexed_async_connection().await
            .map_err(|e| NetworkError::Redis(format!("Failed to establish initial connection: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Get a connection reference for immediate use (no await needed in hot path)
    async fn get_connection(&self) -> tokio::sync::MutexGuard<'_, MultiplexedConnection> {
        self.connection.lock().await
    }
}

#[async_trait::async_trait]
impl MessageTransport for RedisTransport {
    async fn publish(&self, channel_id: ChannelId, data: &[u8]) -> Result<(), NetworkError> {
        let mut conn = self.get_connection().await;
        // Use binary channel ID (4 bytes) for efficiency
        let channel_key = channel_id.raw.to_le_bytes();

        conn.publish::<&[u8], &[u8], ()>(&channel_key, data).await
            .map_err(|e| NetworkError::Redis(format!("PUBLISH failed: {}", e)))?;

        Ok(())
    }

    async fn subscribe(&self, channel_id: ChannelId) -> Result<Box<dyn crate::networking::MessageStream>, NetworkError> {
        let mut pubsub = self.client.get_async_pubsub().await
            .map_err(|e| NetworkError::Redis(format!("Failed to create pubsub connection: {}", e)))?;

        // Use binary channel ID (4 bytes) for efficiency
        let channel_key = channel_id.raw.to_le_bytes();
        pubsub.subscribe(&channel_key).await
            .map_err(|e| NetworkError::Redis(format!("SUBSCRIBE failed: {}", e)))?;

        Ok(Box::new(RedisMessageStream { pubsub }))
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, NetworkError> {
        let mut conn = self.get_connection().await;

        let result: Option<Vec<u8>> = conn.get(key).await
            .map_err(|e| NetworkError::Redis(format!("GET failed: {}", e)))?;

        Ok(result)
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), NetworkError> {
        let mut conn = self.get_connection().await;

        conn.set::<&str, &[u8], ()>(key, value).await
            .map_err(|e| NetworkError::Redis(format!("SET failed: {}", e)))?;

        Ok(())
    }

    async fn set_ex(&self, key: &str, value: &[u8], ttl_ms: u64) -> Result<(), NetworkError> {
        let mut conn = self.get_connection().await;

        conn.set_ex::<&str, &[u8], ()>(key, value, ttl_ms / 1000).await
            .map_err(|e| NetworkError::Redis(format!("SETEX failed: {}", e)))?;

        Ok(())
    }

    async fn mset(&self, pairs: &[(&str, &[u8])]) -> Result<(), NetworkError> {
        let mut conn = self.get_connection().await;

        conn.mset::<_, _, ()>(pairs).await
            .map_err(|e| NetworkError::Redis(format!("MSET failed: {}", e)))?;

        Ok(())
    }

    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>, NetworkError> {
        let mut conn = self.get_connection().await;

        let result: Vec<Option<Vec<u8>>> = conn.mget(keys).await
            .map_err(|e| NetworkError::Redis(format!("MGET failed: {}", e)))?;

        Ok(result)
    }

    fn supports_pubsub(&self) -> bool {
        true
    }

    fn supports_storage(&self) -> bool {
        true
    }

    fn transport_name(&self) -> &'static str {
        "Redis"
    }
}

/// Redis message stream wrapper
pub struct RedisMessageStream {
    pubsub: PubSub,
}

impl std::fmt::Debug for RedisMessageStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisMessageStream")
            .field("pubsub", &"redis::aio::PubSub")
            .finish()
    }
}

#[async_trait::async_trait]
impl crate::networking::MessageStream for RedisMessageStream {
    async fn next(&mut self) -> Result<Option<Vec<u8>>, NetworkError> {
        let mut stream = self.pubsub.on_message();
        if let Some(msg) = stream.next().await {
            if let Ok(data) = msg.get_payload::<Vec<u8>>() {
                return Ok(Some(data));
            }
        }
        Ok(None)
    }

    async fn close(&mut self) -> Result<(), NetworkError> {
        // Redis pubsub connection will be closed when dropped
        Ok(())
    }
}

impl MessageSubscriber for RedisTransport {
    fn subscribe<F>(&self, channel_ids: &[ChannelId], handler: Arc<F>) -> Result<(), NetworkError>
    where
        F: for<'a> Fn(ChannelId, &'a [u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> + Send + Sync + 'static,
    {
        let client = self.client.clone();
        let channel_set: std::collections::HashSet<u32> = channel_ids.iter().map(|c| c.raw).collect();
        let filter_active = !channel_ids.is_empty();

        tokio::spawn(async move {
            if let Ok(mut pubsub) = client.get_async_pubsub().await {
                // Subscribe to all channels
                for &channel_id in &channel_set {
                    let channel_key = channel_id.to_le_bytes();
                    let _ = pubsub.subscribe(&channel_key[..]).await;
                }

                let mut stream = pubsub.on_message();
                while let Some(msg) = stream.next().await {
                    // Get channel name as bytes
                    let channel_name = msg.get_channel::<Vec<u8>>();
                    if let Ok(channel_bytes) = channel_name {
                        if channel_bytes.len() == 4 {
                            let mut channel_id_bytes = [0u8; 4];
                            channel_id_bytes.copy_from_slice(&channel_bytes);
                            let channel_id = u32::from_le_bytes(channel_id_bytes);
                            let channel_obj = ChannelId { raw: channel_id };

                            // Apply filtering if specified
                            if filter_active && !channel_set.contains(&channel_id) {
                                continue;
                            }

                            if let Ok(data) = msg.get_payload::<Vec<u8>>() {
                                // Spawn each handler call to avoid blocking the message loop
                                let handler_clone = handler.clone();
                                tokio::spawn(async move {
                                    handler_clone(channel_obj, &data).await;
                                });
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn subscribe_filter<F>(&self, channel_ids: &[ChannelId], ticker_filter: &[u64], handler: Arc<F>) -> Result<(), NetworkError>
    where
        F: for<'a> Fn(ChannelId, u64, &'a [u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> + Send + Sync + 'static,
    {
        let client = self.client.clone();
        let channel_set: std::collections::HashSet<u32> = channel_ids.iter().map(|c| c.raw).collect();
        let ticker_set: std::collections::HashSet<u64> = ticker_filter.iter().copied().collect();
        let filter_active = !channel_ids.is_empty();
        let ticker_filter_active = !ticker_filter.is_empty();

        tokio::spawn(async move {
            if let Ok(mut pubsub) = client.get_async_pubsub().await {
                // Subscribe to all channels
                for &channel_id in &channel_set {
                    let channel_key = channel_id.to_le_bytes();
                    let _ = pubsub.subscribe(&channel_key[..]).await;
                }

                let mut stream = pubsub.on_message();
                while let Some(msg) = stream.next().await {
                    // Get channel name as bytes
                    let channel_name = msg.get_channel::<Vec<u8>>();
                    if let Ok(channel_bytes) = channel_name {
                        if channel_bytes.len() == 4 {
                            let mut channel_id_bytes = [0u8; 4];
                            channel_id_bytes.copy_from_slice(&channel_bytes);
                            let channel_id = u32::from_le_bytes(channel_id_bytes);
                            let channel_obj = ChannelId { raw: channel_id };

                            // Apply channel filtering if specified
                            if filter_active && !channel_set.contains(&channel_id) {
                                continue;
                            }

                            if let Ok(data) = msg.get_payload::<Vec<u8>>() {
                                // Extract ticker_id from the first 8 bytes (assuming all MITCH messages start with ticker_id)
                                if data.len() >= 8 {
                                    let mut ticker_bytes = [0u8; 8];
                                    ticker_bytes.copy_from_slice(&data[0..8]);
                                    let ticker_id = u64::from_le_bytes(ticker_bytes);

                                    // Apply ticker filtering if specified
                                    if ticker_filter_active && !ticker_set.contains(&ticker_id) {
                                        continue;
                                    }

                                    // Spawn each handler call to avoid blocking the message loop
                                    let handler_clone = handler.clone();
                                    tokio::spawn(async move {
                                        handler_clone(channel_obj, ticker_id, &data).await;
                                    });
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
