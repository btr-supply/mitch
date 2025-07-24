//! Networking macros for MITCH protocol

/// Generates push implementation for message types with ultra-low latency, non-blocking semantics
///
/// # Arguments
///
/// * `$msg_type` - The message struct name (e.g., Trade, Order, Tick)
/// * `$msg_type_char` - The character identifier for the message type
/// * `$ticker_id_field` - The field name containing the ticker_id
#[macro_export]
macro_rules! impl_pushable {
    ($msg_type:ident, $msg_type_char:expr, $ticker_id_field:ident) => {
        #[async_trait::async_trait]
        impl $crate::networking::Pushable for $msg_type {
            /// Push this message to multiple transport clients with ultra-low latency
            ///
            /// This implementation uses fire-and-forget semantics for maximum throughput.
            /// Each transport operation is spawned immediately as a separate task,
            /// avoiding any blocking on individual client operations.
            ///
            /// # Arguments
            /// * `clients` - Vector of Arc-wrapped transport clients for efficient sharing
            /// * `ttl_ms` - Optional TTL for storage operations
            async fn push(
                &self,
                clients: &[std::sync::Arc<dyn $crate::networking::MessageTransport>],
                ttl_ms: Option<u64>,
            ) -> Result<(), $crate::networking::NetworkError> {
                if clients.is_empty() {
                    return Ok(());
                }

                let channel_id = self.get_channel_id(0); // Use provider_id = 0 for default
                let data = self.to_bytes();
                let ticker_id = self.$ticker_id_field;

                // Fire-and-forget: spawn all operations immediately using Arc for efficient sharing
                for client in clients {
                    let client_name = client.transport_name();

                    // Publish operation (if supported) - Arc clone and spawn immediately
                    if client.supports_pubsub() {
                        let data_clone = data.clone();
                        let client_publish = client.clone();

                        tokio::spawn(async move {
                            if let Err(e) = client_publish.publish(channel_id, &data_clone).await {
                                eprintln!("Non-blocking publish failed on {}: {}", client_name, e);
                            }
                        });
                    }

                    // Storage operation (if supported) - Arc clone and spawn immediately
                    if client.supports_storage() {
                        let data_clone = data.clone();
                        let client_storage = client.clone();
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        tokio::spawn(async move {
                            use $crate::networking::keys;
                            let key = keys::message_key(channel_id, ticker_id, timestamp);

                            let result = match ttl_ms {
                                Some(ttl) => client_storage.set_ex(&key, &data_clone, ttl).await,
                                None => client_storage.set(&key, &data_clone).await,
                            };

                            if let Err(e) = result {
                                eprintln!("Non-blocking storage failed on {}: {}", client_name, e);
                            }
                        });
                    }
                }

                // Return immediately - all operations are now running in background
                Ok(())
            }

            /// Get the channel ID for this message type.
            fn get_channel_id(&self, provider_id: u16) -> $crate::ChannelId {
                $crate::ChannelId::new(provider_id, Self::get_message_type())
            }

            /// Get the message type character.
            fn get_message_type() -> char {
                $msg_type_char
            }

            /// Serialize the message to bytes.
            fn to_bytes(&self) -> Vec<u8> {
                self.pack().to_vec()
            }
        }
    };
}
