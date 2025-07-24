//! String formatting and normalization utilities
//!
//! Provides standardized string normalization functions for consistent
//! fuzzy matching across assets and market providers.

/// Normalize a string for asset search
///
/// This function applies comprehensive normalization suitable for asset names,
/// including removal of common business suffixes and punctuation cleanup.
pub fn normalize_asset_name(input: &str) -> String {
    let mut normalized = input.trim().to_lowercase();

    // Remove trailing punctuation
    while let Some(last_char) = normalized.chars().last() {
        if last_char.is_ascii_punctuation() {
            normalized.pop();
        } else {
            break;
        }
    }

    // Remove common prefixes
    if normalized.starts_with("the ") {
        normalized = normalized[4..].to_string();
    }

    // Remove common business suffixes
    for suffix in &[
        " corporation", " company", " inc", " corp", " ltd", " llc",
        " limited", " group", " cie"
    ] {
        if normalized.ends_with(suffix) {
            normalized = normalized[..normalized.len() - suffix.len()].trim().to_string();
            break;
        }
    }

    // Keep only alphanumeric characters
    normalized.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Normalize a string for market provider search
///
/// This function applies normalization suitable for exchange/venue names,
/// being more conservative with suffix removal for shorter names.
pub fn normalize_provider_name(input: &str) -> String {
    let mut normalized = input.trim().to_lowercase();

    // Remove common exchange suffixes only from longer names
    for suffix in &[
        " global", " trading", " limited", " group",
        " ltd", " inc", " llc", " corp"
    ] {
        if normalized.ends_with(suffix) && normalized.len() > 12 {
            normalized = normalized[..normalized.len() - suffix.len()].trim().to_string();
            break;
        }
    }

    // Keep only alphanumeric characters
    normalized.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Generic string normalization with customizable rules
///
/// This is the base normalization function that can be customized
/// for different use cases.
pub fn normalize_string(
    input: &str,
    remove_suffixes: &[&str],
    min_length_for_suffix_removal: usize
) -> String {
    let mut normalized = input.trim().to_lowercase();

    // Remove trailing punctuation
    while let Some(last_char) = normalized.chars().last() {
        if last_char.is_ascii_punctuation() {
            normalized.pop();
        } else {
            break;
        }
    }

    // Remove specified suffixes if name is long enough
    for suffix in remove_suffixes {
        if normalized.ends_with(suffix) && normalized.len() > min_length_for_suffix_removal {
            normalized = normalized[..normalized.len() - suffix.len()].trim().to_string();
            break;
        }
    }

    // Keep only alphanumeric characters
    normalized.chars().filter(|c| c.is_alphanumeric()).collect()
}


