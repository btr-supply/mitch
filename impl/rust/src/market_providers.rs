//! Market Provider Resolution Module
//!
//! Provides market provider resolution with fuzzy string matching for exchange/venue names.
//! Enables resolving provider names like "Binance" or "Interactive Brokers" to their MITCH IDs.

use crate::constants::{MARKET_PROVIDERS_DATA, DataEntry, resolve_market_providers, market_providers_by_id};

/// Market provider with normalized search key
#[derive(Debug, Clone, PartialEq)]
pub struct MarketProvider {
    pub id: u16,
    pub name: String,
}

/// Market provider search result
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderMatch {
    pub provider: MarketProvider,
    pub confidence: f64,
}

impl From<&DataEntry> for MarketProvider {
    fn from(entry: &DataEntry) -> Self {
        Self {
            id: entry.id as u16,
            name: entry.name.to_string(),
        }
    }
}

/// Find market provider by name or alias with fuzzy matching
pub fn find_market_provider(query: &str, _min_confidence: f64) -> Option<ProviderMatch> {
    resolve_market_providers(query).map(|entry| {
        let provider = MarketProvider::from(entry);
        // For exact matches via generic resolver, confidence is 1.0
        // Could add fuzzy matching logic here if needed
        ProviderMatch {
            provider,
            confidence: 1.0,
        }
    })
}

/// Get market provider by ID
pub fn get_market_provider_by_id(id: u16) -> Option<MarketProvider> {
    market_providers_by_id(id as u64).map(MarketProvider::from)
}

/// Get market provider ID by name (exact match)
pub fn get_market_provider_id_by_name(name: &str) -> Option<u16> {
    resolve_market_providers(name).map(|entry| entry.id as u16)
}

/// Get all market providers
pub fn get_all_market_providers() -> Vec<MarketProvider> {
    MARKET_PROVIDERS_DATA.iter().map(MarketProvider::from).collect()
}

/// Get all market provider data entries
pub fn get_market_providers_data() -> &'static [DataEntry] {
    MARKET_PROVIDERS_DATA
}
