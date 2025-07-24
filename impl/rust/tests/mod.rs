pub mod channel_test;
pub mod constants_test;
pub mod format_test;
pub mod header_test;
pub mod index_test;
pub mod lib_test;
pub mod networking;
pub mod order_book_test;
pub mod order_test;
pub mod similarity_test;
pub mod tick_test;
pub mod ticker_test;
pub mod trade_test;

#[cfg(all(feature = "networking", feature = "redis-client"))]
pub mod redis_pubsub_test;

#[cfg(all(feature = "networking", feature = "redis-client"))]
pub mod redis_pubsub_integrity_test;

#[cfg(all(feature = "networking", feature = "redis-client"))]
pub mod redis_pubsub_complete_integrity_test;
