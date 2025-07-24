//! Networking Layer for MITCH Protocol
//!
//! Provides transport-agnostic interfaces for publishing and subscribing to MITCH messages
//! over Redis (RESP3) and WebTransport (HTTP/3) protocols.
//!
//! ## Design Philosophy
//!
//! - **Transport Agnostic**: Message types have push() methods that accept any transport client
//! - **Multiple Transports**: Support Redis and WebTransport simultaneously
//! - **Channel-Based Routing**: Uses MITCH Channel IDs for efficient pub/sub filtering
//! - **Async by Default**: All operations are async for high-performance I/O
//! - **Auto-Reconnection**: Transport clients handle reconnection logic externally
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use mitch::{Trade, OrderSide, networking::*};
//! use std::sync::Arc;
//!
//! # #[cfg(all(feature = "redis-client", feature = "webtransport-client"))]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create clients (externally managed)
//! let redis_client = redis::RedisTransport::new("redis://localhost:6379").await?;
//! let wt_client = webtransport::WebTransportClient::new("https://localhost:4433").await?;
//!
//! // Create trade message
//! let trade = Trade::new(0x1234, 100.0, 1000, 1, OrderSide::Buy)?;
//!
//! // Push to multiple transports with ultra-low latency
//! let clients: Vec<Arc<dyn MessageTransport>> = vec![
//!     Arc::new(redis_client),
//!     Arc::new(wt_client),
//! ];
//!
//! trade.push(&clients, None).await?;
//! # Ok(())
//! # }
//! ```

#[macro_use]
pub mod macros;

use crate::{ChannelId, MitchError};
use async_trait::async_trait;
use std::error::Error as StdError;
use thiserror::Error;
use std::sync::Arc;

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Networking-related errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Transport error: {0}")]
    Transport(#[from] Box<dyn StdError + Send + Sync>),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("WebTransport error: {0}")]
    WebTransport(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Invalid channel: {0}")]
    InvalidChannel(String),

    #[error("Message too large: {size} bytes (max: {max})")]
    MessageTooLarge { size: usize, max: usize },

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<NetworkError> for MitchError {
    fn from(err: NetworkError) -> Self {
        MitchError::SerializationError(err.to_string())
    }
}

// =============================================================================
// KEY GENERATION UTILITIES
// =============================================================================

/// Utilities for generating storage keys
pub mod keys {
    use crate::ChannelId;

    /// Generate a message storage key
    pub fn message_key(channel_id: ChannelId, ticker_id: u64, timestamp: u64) -> String {
        format!("mitch:{}:{}:{}", channel_id.raw, ticker_id, timestamp)
    }

    /// Generate a ticker-specific key
    pub fn ticker_key(ticker_id: u64) -> String {
        format!("mitch:ticker:{}", ticker_id)
    }

    /// Generate a channel-specific key (returns binary key for efficiency)
    pub fn channel_key(channel_id: ChannelId) -> Vec<u8> {
        // Return raw 4-byte channel ID
        channel_id.raw.to_le_bytes().to_vec()
    }
}

// =============================================================================
// TRANSPORT TRAITS
// =============================================================================

/// Generic transport interface for MITCH messages
///
/// Supports both pub/sub messaging and key-value storage operations
#[async_trait]
pub trait MessageTransport: Send + Sync {
    /// Publish a message to a channel
    async fn publish(&self, channel_id: ChannelId, data: &[u8]) -> Result<(), NetworkError>;

    /// Subscribe to a channel (returns stream/receiver)
    async fn subscribe(&self, channel_id: ChannelId) -> Result<Box<dyn MessageStream>, NetworkError>;

    /// Store a single key-value pair
    async fn set(&self, key: &str, data: &[u8]) -> Result<(), NetworkError>;

    /// Store a single key-value pair with TTL expiration
    async fn set_ex(&self, key: &str, data: &[u8], expire_ms: u64) -> Result<(), NetworkError>;

    /// Retrieve a single value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, NetworkError>;

    /// Store multiple key-value pairs atomically
    async fn mset(&self, pairs: &[(&str, &[u8])]) -> Result<(), NetworkError>;

    /// Retrieve multiple values by keys
    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>, NetworkError>;

    /// Check if transport supports storage operations
    fn supports_storage(&self) -> bool;

    /// Check if transport supports pub/sub operations
    fn supports_pubsub(&self) -> bool;

    /// Get transport name for debugging
    fn transport_name(&self) -> &'static str;
}

/// Extension trait for subscription operations with handlers
/// Separated to maintain object safety of MessageTransport
pub trait MessageSubscriber: MessageTransport {
    /// Subscribe to one or multiple channels with async message handler
    fn subscribe<F>(&self, channel_ids: &[ChannelId], handler: Arc<F>) -> Result<(), NetworkError>
    where
        F: for<'a> Fn(ChannelId, &'a [u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> + Send + Sync + 'static;

    /// Subscribe to channels with ticker filtering and async message handler
    fn subscribe_filter<F>(&self, channel_ids: &[ChannelId], ticker_filter: &[u64], handler: Arc<F>) -> Result<(), NetworkError>
    where
        F: for<'a> Fn(ChannelId, u64, &'a [u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> + Send + Sync + 'static;
}

/// Message stream interface for subscriptions
#[async_trait]
pub trait MessageStream: Send + Sync {
    /// Receive next message (blocks until available)
    async fn next(&mut self) -> Result<Option<Vec<u8>>, NetworkError>;

    /// Close the stream
    async fn close(&mut self) -> Result<(), NetworkError>;
}

// =============================================================================
// PUSHABLE TRAIT
// =============================================================================

/// Trait for messages that can be pushed through message transports
///
/// Enables pushing individual messages or batches to multiple transports
#[async_trait]
pub trait Pushable: Send + Sync {
    /// Get the channel ID for this message type
    fn get_channel_id(&self, provider_id: u16) -> ChannelId;

    /// Serialize message to bytes for transmission
    fn to_bytes(&self) -> Vec<u8>;

    /// Get the message type character
    fn get_message_type() -> char where Self: Sized;

    /// Push message to multiple transports concurrently with ultra-low latency
    ///
    /// Uses Arc for efficient sharing and fire-and-forget semantics for maximum throughput
    async fn push(&self, clients: &[Arc<dyn MessageTransport>], ttl_ms: Option<u64>) -> Result<(), NetworkError>;
}

/// A trait that combines `MessageTransport` and `MessageSubscriber`.
#[async_trait]
pub trait FullTransport: MessageTransport + MessageSubscriber {}

impl<T: MessageTransport + MessageSubscriber> FullTransport for T {}

/// Represents a subscription to a channel.
#[allow(missing_debug_implementations)]
pub struct Subscription {
    /// The channel ID being subscribed to.
    pub channel_id: ChannelId,
    /// The ticker ID filter for this subscription.
    pub ticker_filter: Option<Vec<u64>>,
    /// The handler for incoming messages.
    pub handler: Arc<dyn Fn(ChannelId, u64, &[u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>,
}

// =============================================================================
// STORAGE UTILITIES
// =============================================================================

/// Batch operation utilities for high-throughput scenarios
pub mod batch {
    use super::*;
    use crate::MitchHeader;

    /// Batch multiple messages of the same type for efficient transmission
    #[derive(Debug)]
    pub struct MessageBatch<T> {
        messages: Vec<T>,
        channel_id: ChannelId,
        timestamp: u64,
    }

    impl<T> MessageBatch<T>
    where
        T: Pushable + Clone,
    {
        /// Create new batch
        pub fn new(channel_id: ChannelId) -> Self {
            Self {
                messages: Vec::new(),
                channel_id,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
            }
        }

        /// Add message to batch
        pub fn add(&mut self, message: T) {
            self.messages.push(message);
        }

        /// Get batch size
        pub fn len(&self) -> usize {
            self.messages.len()
        }

        /// Check if batch is empty
        pub fn is_empty(&self) -> bool {
            self.messages.is_empty()
        }

        /// Serialize batch to MITCH protocol format with header
        pub fn to_bytes(&self) -> Result<Vec<u8>, NetworkError> {
            if self.messages.is_empty() {
                return Ok(Vec::new());
            }

            if self.messages.len() > 255 {
                return Err(NetworkError::MessageTooLarge {
                    size: self.messages.len(),
                    max: 255
                });
            }

            // Create header
            let msg_type = T::get_message_type() as u8;
            let count = self.messages.len() as u8;
            let header = MitchHeader::new(msg_type, self.timestamp, count);

            // Serialize messages
            let mut buffer = Vec::new();
            buffer.extend_from_slice(&header.pack());

            for message in &self.messages {
                buffer.extend_from_slice(&message.to_bytes());
            }

            Ok(buffer)
        }

        /// Push batch to multiple transport clients with ultra-low latency
        ///
        /// Uses fire-and-forget semantics for maximum throughput in high-frequency scenarios.
        /// All operations are spawned immediately without waiting for completion.
        pub async fn push(&self, clients: &[Arc<dyn MessageTransport>], ttl_ms: Option<u64>) -> Result<(), NetworkError> {
            if clients.is_empty() {
                return Ok(());
            }

            let data = self.to_bytes()?;
            let channel_id = self.channel_id;
            let timestamp = self.timestamp;

            // Fire-and-forget: spawn all operations immediately using Arc for efficient sharing
            for client in clients {
                let client_name = client.transport_name();

                // Publish operation (if supported) - Arc clone and spawn immediately
                if client.supports_pubsub() {
                    let data_clone = data.clone();
                    let client_publish = client.clone();

                    tokio::spawn(async move {
                        if let Err(e) = client_publish.publish(channel_id, &data_clone).await {
                            eprintln!("Non-blocking batch publish failed on {}: {}", client_name, e);
                        }
                    });
                }

                // Storage operation (if supported) - Arc clone and spawn immediately
                if client.supports_storage() {
                    let data_clone = data.clone();
                    let client_storage = client.clone();

                    tokio::spawn(async move {
                        use crate::networking::keys;
                        let key = keys::message_key(channel_id, 0, timestamp); // Use 0 for batch ticker_id

                        let result = match ttl_ms {
                            Some(ttl) => client_storage.set_ex(&key, &data_clone, ttl).await,
                            None => client_storage.set(&key, &data_clone).await,
                        };

                        if let Err(e) = result {
                            eprintln!("Non-blocking batch storage failed on {}: {}", client_name, e);
                        }
                    });
                }
            }

            // Return immediately - all operations are now running in background
            Ok(())
        }
    }
}

// Re-export submodules when features are enabled
#[cfg(feature = "redis-client")]
pub mod redis;

#[cfg(feature = "webtransport-client")]
pub mod webtransport;

#[cfg(feature = "webtransport-client")]
pub mod webtransport_server;

#[cfg(feature = "webtransport-client")]
pub mod webtransport_singleton;
