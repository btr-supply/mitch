//! WebTransport (HTTP/3) transport for MITCH protocol
//!
//! Provides a high-performance, low-latency streaming transport over QUIC.
//! WebTransport is ideal for real-time data distribution where pub/sub
//! is required, but key-value storage is not.

use super::{MessageTransport, MessageStream, MessageSubscriber, NetworkError};
use super::webtransport_singleton::{WebTransportServerSingleton, WebTransportPublisher};
use crate::ChannelId;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::io::AsyncReadExt;
use wtransport::{ClientConfig, Endpoint, Connection};
use std::net::SocketAddr;

/// WebTransport client for MITCH protocol
#[derive(Clone)]
pub enum WebTransportClient {
    /// External server connection
    External(Arc<Connection>),
    /// Local singleton publisher
    Local(WebTransportPublisher),
}

impl std::fmt::Debug for WebTransportClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::External(_) => write!(f, "WebTransportClient::External(Connection)"),
            Self::Local(_) => write!(f, "WebTransportClient::Local"),
        }
    }
}

impl WebTransportClient {
    /// Create a new WebTransport client
    /// 
    /// # Arguments
    /// * `url` - WebTransport URL with optional authentication in query string (e.g., "https://host:port?secret=key")
    ///           If url is "local://" or empty, uses the singleton server
    pub async fn new(url: &str) -> Result<Self, NetworkError> {
        // Check if we should use local singleton
        if url.is_empty() || url == "local://" || url.starts_with("local://") {
            // Initialize singleton server if not already done
            let addr: SocketAddr = if url.starts_with("local://") {
                url.strip_prefix("local://")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| "127.0.0.1:4433".parse().unwrap())
            } else {
                "127.0.0.1:4433".parse().unwrap()
            };
            
            // Initialize singleton if needed
            WebTransportServerSingleton::initialize(addr).await?;
            
            // Create publisher
            let publisher = WebTransportPublisher::new()?;
            return Ok(Self::Local(publisher));
        }
        
        // Connect to external server
        let config = ClientConfig::builder()
            .with_bind_default()
            .with_native_certs()
            .build();

        let endpoint = Endpoint::client(config)
            .map_err(|e| NetworkError::WebTransport(format!("Failed to create endpoint: {}", e)))?;

        let connection = endpoint
            .connect(url)
            .await
            .map_err(|e| NetworkError::WebTransport(format!("Failed to connect: {}", e)))?;

        // Note: Authentication should be handled via query string in the URL if needed
        // e.g., "https://host:port?secret=key"

        Ok(Self::External(Arc::new(connection)))
    }
    
    /// Create a client that uses the local singleton server for publishing
    pub async fn new_local() -> Result<Self, NetworkError> {
        Self::new("local://").await
    }
    
    /// Create a consumer client that connects to the local singleton server
    pub async fn new_consumer(addr: Option<SocketAddr>) -> Result<Self, NetworkError> {
        let addr = addr.unwrap_or_else(|| "127.0.0.1:4433".parse().unwrap());
        
        // Make sure singleton is initialized
        WebTransportServerSingleton::initialize(addr).await?;
        
        // Connect as external client to the local server
        let url = format!("https://{}", addr);
        Self::new(&url).await
    }
}

#[async_trait]
impl MessageTransport for WebTransportClient {
    /// Publish message to a WebTransport stream
    ///
    /// Note: WebTransport does not have "channels" in the same way as Redis.
    /// We simulate this by writing the channel_id as a prefix to the data.
    async fn publish(&self, channel_id: ChannelId, data: &[u8]) -> Result<(), NetworkError> {
        match self {
            Self::Local(publisher) => publisher.publish(channel_id, data).await,
            Self::External(connection) => {
                let mut stream = connection.open_uni().await
                    .map_err(|e| NetworkError::WebTransport(format!("Failed to open send stream: {}", e)))?
                    .await
                    .map_err(|e| NetworkError::WebTransport(format!("Failed to connect stream: {}", e)))?;

                // Prepend channel_id to data
                let mut message = Vec::with_capacity(4 + data.len());
                message.extend_from_slice(&channel_id.raw.to_le_bytes());
                message.extend_from_slice(data);

                stream.write_all(&message).await
                    .map_err(|e| NetworkError::WebTransport(format!("Failed to write to stream: {}", e)))?;

                Ok(())
            }
        }
    }

    /// Subscribe to incoming streams
    async fn subscribe(&self, _channel_id: ChannelId) -> Result<Box<dyn MessageStream>, NetworkError> {
        match self {
            Self::Local(_publisher) => {
                // For local publisher, we need to connect as a consumer to the singleton server
                Err(NetworkError::UnsupportedOperation(
                    "Local publisher does not support subscription. Use a separate WebTransportClient connected to local://".to_string()
                ))
            }
            Self::External(connection) => {
                let (tx, rx) = mpsc::unbounded_channel();
                let connection = connection.clone();

                // Spawn a background task to listen for incoming streams
                tokio::spawn(async move {
                    loop {
                        match connection.accept_uni().await {
                            Ok(mut stream) => {
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let mut buffer = Vec::new();
                                    if stream.read_to_end(&mut buffer).await.is_ok() {
                                        if tx_clone.send(buffer).is_err() {
                                            return; // Receiver dropped, stop listening
                                        }
                                    }
                                });
                            }
                            Err(_) => {
                                return; // Connection closed
                            }
                        }
                    }
                });

                Ok(Box::new(WebTransportMessageStream { rx }))
            }
        }
    }

    // Storage operations are not supported by WebTransport
    async fn set(&self, _key: &str, _data: &[u8]) -> Result<(), NetworkError> {
        Err(NetworkError::UnsupportedOperation("WebTransport does not support key-value storage".to_string()))
    }

    async fn set_ex(&self, _key: &str, _data: &[u8], _expire_ms: u64) -> Result<(), NetworkError> {
        Err(NetworkError::UnsupportedOperation("WebTransport does not support key-value storage".to_string()))
    }

    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>, NetworkError> {
        Err(NetworkError::UnsupportedOperation("WebTransport does not support key-value storage".to_string()))
    }

    async fn mset(&self, _pairs: &[(&str, &[u8])]) -> Result<(), NetworkError> {
        Err(NetworkError::UnsupportedOperation("WebTransport does not support key-value storage".to_string()))
    }

    async fn mget(&self, _keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>, NetworkError> {
        Err(NetworkError::UnsupportedOperation("WebTransport does not support key-value storage".to_string()))
    }

    fn supports_storage(&self) -> bool {
        false
    }

    fn supports_pubsub(&self) -> bool {
        true
    }

    fn transport_name(&self) -> &'static str {
        match self {
            Self::Local(_) => "WebTransport-Local",
            Self::External(_) => "WebTransport-External",
        }
    }
}

impl MessageSubscriber for WebTransportClient {
    /// Subscribe with an async handler - non-blocking for high-frequency trading
    fn subscribe<F>(&self, channel_ids: &[ChannelId], handler: Arc<F>) -> Result<(), NetworkError>
    where
        F: for<'a> Fn(ChannelId, &'a [u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> + Send + Sync + 'static,
    {
        match self {
            Self::Local(_) => {
                Err(NetworkError::UnsupportedOperation(
                    "Local publisher does not support subscription. Use a separate WebTransportClient connected to local://".to_string()
                ))
            }
            Self::External(connection) => {
                let connection = connection.clone();
                let channel_set: std::collections::HashSet<u32> = channel_ids.iter().map(|c| c.raw).collect();
                let filter_active = !channel_ids.is_empty();

                tokio::spawn(async move {
                    loop {
                        if let Ok(mut stream) = connection.accept_uni().await {
                            let handler_clone = handler.clone();
                            let channel_set_clone = channel_set.clone();

                            tokio::spawn(async move {
                                let mut buffer = Vec::new();
                                if stream.read_to_end(&mut buffer).await.is_ok() {
                                    if buffer.len() >= 4 {
                                        // Extract channel_id from prefix
                                        let mut channel_bytes = [0u8; 4];
                                        channel_bytes.copy_from_slice(&buffer[0..4]);
                                        let channel_raw = u32::from_le_bytes(channel_bytes);

                                        if !filter_active || channel_set_clone.contains(&channel_raw) {
                                            let channel_id = ChannelId { raw: channel_raw };
                                            // Spawn handler to avoid blocking stream processing
                                            let handler_spawn = handler_clone.clone();
                                            let data = buffer[4..].to_vec();
                                            tokio::spawn(async move {
                                                handler_spawn(channel_id, &data).await;
                                            });
                                        }
                                    }
                                }
                            });
                        } else {
                            return; // Connection closed
                        }
                    }
                });
                Ok(())
            }
        }
    }

    /// Subscribe with ticker filtering - non-blocking for high-frequency trading
    fn subscribe_filter<F>(
        &self,
        channel_ids: &[ChannelId],
        ticker_filter: &[u64],
        handler: Arc<F>,
    ) -> Result<(), NetworkError>
    where
        F: for<'a> Fn(ChannelId, u64, &'a [u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> + Send + Sync + 'static,
    {
        match self {
            Self::Local(_) => {
                Err(NetworkError::UnsupportedOperation(
                    "Local publisher does not support subscription. Use a separate WebTransportClient connected to local://".to_string()
                ))
            }
            Self::External(connection) => {
                let connection = connection.clone();
                let channel_set: std::collections::HashSet<u32> = channel_ids.iter().map(|c| c.raw).collect();
                let ticker_set: std::collections::HashSet<u64> = ticker_filter.iter().copied().collect();

                let channel_filter_active = !channel_ids.is_empty();
                let ticker_filter_active = !ticker_filter.is_empty();

                tokio::spawn(async move {
                    loop {
                        if let Ok(mut stream) = connection.accept_uni().await {
                            let handler_clone = handler.clone();
                            let channel_set_clone = channel_set.clone();
                            let ticker_set_clone = ticker_set.clone();

                            tokio::spawn(async move {
                                let mut buffer = Vec::new();
                                if stream.read_to_end(&mut buffer).await.is_ok() {
                                    if buffer.len() >= 12 { // 4 bytes channel_id + 8 bytes ticker_id minimum
                                        // Extract channel_id from prefix
                                        let mut channel_bytes = [0u8; 4];
                                        channel_bytes.copy_from_slice(&buffer[0..4]);
                                        let channel_raw = u32::from_le_bytes(channel_bytes);

                                        if !channel_filter_active || channel_set_clone.contains(&channel_raw) {
                                            // Extract ticker_id from message payload (first 8 bytes after channel_id)
                                            let mut ticker_bytes = [0u8; 8];
                                            ticker_bytes.copy_from_slice(&buffer[4..12]);
                                            let ticker_id = u64::from_le_bytes(ticker_bytes);

                                            if !ticker_filter_active || ticker_set_clone.contains(&ticker_id) {
                                                let channel_id = ChannelId { raw: channel_raw };
                                                // Spawn handler to avoid blocking stream processing
                                                let handler_spawn = handler_clone.clone();
                                                let data = buffer[4..].to_vec();
                                                tokio::spawn(async move {
                                                    handler_spawn(channel_id, ticker_id, &data).await;
                                                });
                                            }
                                        }
                                    }
                                }
                            });
                        } else {
                            return; // Connection closed
                        }
                    }
                });
                Ok(())
            }
        }
    }
}

/// Message stream for WebTransport
#[derive(Debug)]
pub struct WebTransportMessageStream {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

#[async_trait]
impl MessageStream for WebTransportMessageStream {
    async fn next(&mut self) -> Result<Option<Vec<u8>>, NetworkError> {
        Ok(self.rx.recv().await)
    }

    async fn close(&mut self) -> Result<(), NetworkError> {
        self.rx.close();
        Ok(())
    }
}
