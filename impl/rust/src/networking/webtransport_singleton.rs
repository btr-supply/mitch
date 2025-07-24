//! WebTransport Singleton Server
//!
//! Provides a singleton WebTransport server that can be shared across the application.
//! The server accepts connections from WebTransport consumers and broadcasts messages to them.

use super::{MessageTransport, NetworkError};
use crate::ChannelId;
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};
use std::collections::HashMap;
use tokio::sync::RwLock;
use wtransport::{ServerConfig, Endpoint, Connection, Identity};
use wtransport::endpoint::endpoint_side::Server;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global singleton for the WebTransport server
static WEBTRANSPORT_SERVER: OnceLock<Arc<WebTransportServerSingleton>> = OnceLock::new();

/// WebTransport server singleton that manages all client connections
pub struct WebTransportServerSingleton {
    clients: Arc<RwLock<HashMap<String, Arc<Connection>>>>,
    stats: Arc<ServerStats>,
}

impl std::fmt::Debug for WebTransportServerSingleton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebTransportServerSingleton")
            .field("clients", &"HashMap<String, Connection>")
            .field("stats", &self.stats)
            .finish()
    }
}

/// Statistics for the server
#[derive(Debug)]
struct ServerStats {
    messages_sent: AtomicU64,
    clients_connected: AtomicU64,
    send_errors: AtomicU64,
}

impl ServerStats {
    fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            clients_connected: AtomicU64::new(0),
            send_errors: AtomicU64::new(0),
        }
    }
}

impl WebTransportServerSingleton {
    /// Initialize the global WebTransport server singleton
    pub async fn initialize(bind_addr: SocketAddr) -> Result<(), NetworkError> {
        if WEBTRANSPORT_SERVER.get().is_some() {
            return Ok(()); // Already initialized
        }

        let clients = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(ServerStats::new());
        
        // Generate self-signed certificate for testing
        let identity = Identity::self_signed(["localhost", "127.0.0.1", "::1"])
            .map_err(|e| NetworkError::WebTransport(format!("Failed to generate identity: {}", e)))?;
        
        let server_config = ServerConfig::builder()
            .with_bind_default(bind_addr.port())
            .with_identity(identity)
            .build();

        let server = Endpoint::server(server_config)
            .map_err(|e| NetworkError::WebTransport(format!("Failed to create server: {}", e)))?;

        println!("✓ WebTransport singleton server listening on {}", bind_addr);

        // Clone for the server task
        let clients_server = clients.clone();
        let stats_server = stats.clone();

        // Start server task
        tokio::spawn(async move {
            Self::run_server(server, clients_server, stats_server).await;
        });

        let singleton = Arc::new(WebTransportServerSingleton { clients, stats });
        
        WEBTRANSPORT_SERVER.set(singleton)
            .map_err(|_| NetworkError::WebTransport("Failed to set singleton".to_string()))?;
        
        Ok(())
    }

    /// Get the singleton instance
    pub fn instance() -> Option<Arc<WebTransportServerSingleton>> {
        WEBTRANSPORT_SERVER.get().cloned()
    }

    /// Run the server loop accepting new connections
    async fn run_server(
        server: Endpoint<Server>,
        clients: Arc<RwLock<HashMap<String, Arc<Connection>>>>,
        stats: Arc<ServerStats>,
    ) {
        loop {
            let connecting = server.accept().await;
            let clients_clone = clients.clone();
            let stats_clone = stats.clone();
            
            tokio::spawn(async move {
                match connecting.await {
                    Ok(session_request) => {
                        // Accept the session
                        let connection = match session_request.accept().await {
                            Ok(conn) => conn,
                            Err(e) => {
                                eprintln!("Failed to accept session: {}", e);
                                return;
                            }
                        };
                        
                        let client_id = format!("client_{}", connection.stable_id());
                        println!("✓ WebTransport consumer connected: {}", client_id);
                        
                        // Add client to active connections
                        {
                            let mut clients_lock = clients_clone.write().await;
                            clients_lock.insert(client_id.clone(), Arc::new(connection.clone()));
                        }
                        stats_clone.clients_connected.fetch_add(1, Ordering::Relaxed);
                        
                        // Wait for connection to close
                        connection.closed().await;
                        
                        // Remove client when disconnected
                        {
                            let mut clients_lock = clients_clone.write().await;
                            clients_lock.remove(&client_id);
                        }
                        stats_clone.clients_connected.fetch_sub(1, Ordering::Relaxed);
                        println!("✗ WebTransport consumer disconnected: {}", client_id);
                    }
                    Err(e) => {
                        eprintln!("WebTransport connection failed: {}", e);
                    }
                }
            });
        }
    }

    /// Broadcast message to all connected clients (fire-and-forget)
    pub async fn broadcast(&self, channel_id: ChannelId, data: &[u8]) -> Result<(), NetworkError> {
        let clients_lock = self.clients.read().await;
        
        // If no clients connected, just increment counter and return success (fire-and-forget)
        if clients_lock.is_empty() {
            self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }

        // Prepare message with channel prefix
        let mut message = Vec::with_capacity(4 + data.len());
        message.extend_from_slice(&channel_id.raw.to_le_bytes());
        message.extend_from_slice(data);

        // Send to all clients concurrently (fire-and-forget)
        for connection in clients_lock.values() {
            let connection_clone = connection.clone();
            let message_clone = message.clone();
            let stats_clone = self.stats.clone();
            
            tokio::spawn(async move {
                match connection_clone.open_uni().await {
                    Ok(send_stream) => {
                        match send_stream.await {
                            Ok(mut stream) => {
                                if let Err(_) = stream.write_all(&message_clone).await {
                                    stats_clone.send_errors.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                            Err(_) => {
                                stats_clone.send_errors.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    Err(_) => {
                        stats_clone.send_errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
        
        self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.stats.messages_sent.load(Ordering::Relaxed),
            self.stats.clients_connected.load(Ordering::Relaxed),
            self.stats.send_errors.load(Ordering::Relaxed),
        )
    }
}

/// WebTransport publisher client that uses the singleton server
#[derive(Debug, Clone)]
pub struct WebTransportPublisher;

impl WebTransportPublisher {
    /// Create a new publisher (requires singleton to be initialized)
    pub fn new() -> Result<Self, NetworkError> {
        if WebTransportServerSingleton::instance().is_none() {
            return Err(NetworkError::WebTransport(
                "WebTransport server singleton not initialized. Call WebTransportServerSingleton::initialize() first".to_string()
            ));
        }
        Ok(Self)
    }
}

#[async_trait]
impl MessageTransport for WebTransportPublisher {
    /// Publish message through the singleton server to all connected consumers
    async fn publish(&self, channel_id: ChannelId, data: &[u8]) -> Result<(), NetworkError> {
        match WebTransportServerSingleton::instance() {
            Some(server) => server.broadcast(channel_id, data).await,
            None => Err(NetworkError::WebTransport("Server singleton not initialized".to_string())),
        }
    }

    /// Subscribe not supported for publisher
    async fn subscribe(&self, _channel_id: ChannelId) -> Result<Box<dyn super::MessageStream>, NetworkError> {
        Err(NetworkError::UnsupportedOperation("WebTransport publisher does not support subscription".to_string()))
    }

    // Storage operations are not supported
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
        "WebTransport-Publisher"
    }
}