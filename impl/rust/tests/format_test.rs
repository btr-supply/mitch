//! Tests for string formatting and normalization utilities

use mitch::utils::format::*;

#[test]
fn test_normalize_asset_name() {
    assert_eq!(normalize_asset_name("Apple Inc."), "apple");
    assert_eq!(normalize_asset_name("Microsoft Corporation"), "microsoft");
    assert_eq!(normalize_asset_name("The Walt Disney Company"), "waltdisney");
    assert_eq!(normalize_asset_name("Bitcoin"), "bitcoin");
    assert_eq!(normalize_asset_name("JPMorgan Chase & Co."), "jpmorganchaseco");
}

#[test]
fn test_normalize_provider_name() {
    assert_eq!(normalize_provider_name("BTR Markets"), "btrmarkets");
    assert_eq!(normalize_provider_name("Gate.io"), "gateio");
    assert_eq!(normalize_provider_name("Charles Schwab"), "charlesschwab");
    assert_eq!(normalize_provider_name("New York Stock Exchange"), "newyorkstockexchange");
    assert_eq!(normalize_provider_name("Interactive Brokers LLC"), "interactivebrokers");
}

#[test]
fn test_generic_normalization() {
    let suffixes = &[" corp", " inc"];
    assert_eq!(normalize_string("Apple Inc.", suffixes, 5), "apple");
    assert_eq!(normalize_string("IBM", suffixes, 5), "ibm"); // Too short for suffix removal
}
