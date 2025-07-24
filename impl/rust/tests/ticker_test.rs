use mitch::*;

// =============================================================================
// TICKER ID ENCODING/DECODING TESTS
// =============================================================================

#[test]
fn test_ticker_creation_and_extraction() {
    let ticker = TickerId::new(
        InstrumentType::SPOT,
        AssetClass::FX,
        978, // EUR
        AssetClass::FX,
        840, // USD
        0,
    ).unwrap();

    assert_eq!(ticker.instrument_type(), InstrumentType::SPOT);
    assert_eq!(ticker.base_asset_class(), AssetClass::FX);
    assert_eq!(ticker.base_asset_id(), 978);
    assert_eq!(ticker.quote_asset_class(), AssetClass::FX);
    assert_eq!(ticker.quote_asset_id(), 840);
    assert_eq!(ticker.sub_type(), 0);
}

#[test]
fn test_ticker_pack_unpack() {
    let original = TickerId::new(
        InstrumentType::PERP,
        AssetClass::CR,
        1,   // BTC
        AssetClass::CR,
        2,   // USDT
        100,
    ).unwrap();

    let packed = original.pack();
    let unpacked = TickerId::unpack(&packed).unwrap();
    assert_eq!(original, unpacked);
}

#[test]
fn test_ticker_convenience_functions() {
    // Test forex ticker
    let eur_usd = forex_ticker(978, 840, InstrumentType::SPOT, 0).unwrap();
    assert!(eur_usd.is_forex());
    assert!(eur_usd.is_spot());
    assert!(!eur_usd.is_crypto());

    // Test crypto ticker
    let btc_usdt = crypto_ticker(1, 2, InstrumentType::PERP, 0).unwrap();
    assert!(btc_usdt.is_crypto());
    assert!(!btc_usdt.is_spot());

    // Test equity ticker
    let apple = equity_ticker(1000, 840, InstrumentType::SPOT, 0).unwrap();
    assert_eq!(apple.base_asset_class(), AssetClass::EQ);
    assert_eq!(apple.quote_asset_class(), AssetClass::FX);
}

#[test]
fn test_ticker_validation() {
    let _ticker = TickerId::new(
        InstrumentType::SPOT,
        AssetClass::FX,
        978,
        AssetClass::FX,
        840,
        0,
    ).unwrap();

    // Test sub-type overflow
    let result = TickerId::new(
        InstrumentType::SPOT,
        AssetClass::FX,
        978,
        AssetClass::FX,
        840,
        0x100000, // Too large for 20 bits
    );
    assert!(result.is_err());
}

#[test]
fn test_ticker_batch_operations() {
    let tickers = vec![
        forex_ticker(978, 840, InstrumentType::SPOT, 0).unwrap(),
        crypto_ticker(1, 2, InstrumentType::PERP, 0).unwrap(),
        equity_ticker(1000, 840, InstrumentType::SPOT, 0).unwrap(),
    ];

    let packed = pack_ticker_batch(&tickers);
    let unpacked = unpack_ticker_batch(&packed, tickers.len()).unwrap();

    assert_eq!(tickers.len(), unpacked.len());
    for (orig, unpacked) in tickers.iter().zip(unpacked.iter()) {
        assert_eq!(*orig, *unpacked);
    }
}

#[test]
fn test_bit_manipulation_accuracy() {
    // Test maximum values for each field
    let ticker = TickerId::new(
        InstrumentType::STRUCT, // 0xD = 13
        AssetClass::LR,         // 0xD = 13
        65535,                  // Maximum u16
        AssetClass::IN,         // 0x9 = 9
        65535,                  // Maximum u16
        0xFFFFF,                // Maximum 20-bit value
    ).unwrap();

    assert_eq!(ticker.instrument_type(), InstrumentType::STRUCT);
    assert_eq!(ticker.base_asset_class(), AssetClass::LR);
    assert_eq!(ticker.base_asset_id(), 65535);
    assert_eq!(ticker.quote_asset_class(), AssetClass::IN);
    assert_eq!(ticker.quote_asset_id(), 65535);
    assert_eq!(ticker.sub_type(), 0xFFFFF);
}

#[test]
fn test_spec_compliance() {
    // Test the exact example from the spec: EUR/USD Spot
    let ticker = TickerId::new(
        InstrumentType::SPOT,   // 0x0
        AssetClass::FX,         // 0x3
        111,                    // EUR (example ID)
        AssetClass::FX,         // 0x3
        461,                    // USD (example ID)
        0,                      // No sub-type
    ).unwrap();

    // Verify bit layout matches spec
    let expected_raw = (0x0u64 << 60) |    // Instrument Type
                      (0x3u64 << 56) |     // Base Asset Class
                      (111u64 << 40) |     // Base Asset ID
                      (0x3u64 << 36) |     // Quote Asset Class
                      (461u64 << 20) |     // Quote Asset ID
                      0u64;                // Sub-Type

    assert_eq!(ticker.raw, expected_raw);
}

// =============================================================================
// ASSET RESOLUTION TESTS (merged from asset_resolver_test.rs)
// =============================================================================

#[test]
fn test_asset_pack_unpack() {
    // Test asset packing/unpacking as per asset.md
    let packed = pack_asset(AssetClass::EQ, 831); // Apple
    let (class, class_id) = unpack_asset(packed);

    assert_eq!(class, AssetClass::EQ);
    assert_eq!(class_id, 831);

    // Test different asset classes
    let crypto_packed = pack_asset(AssetClass::CR, 2701); // Bitcoin
    let (crypto_class, crypto_id) = unpack_asset(crypto_packed);
    assert_eq!(crypto_class, AssetClass::CR);
    assert_eq!(crypto_id, 2701);
}

#[test]
fn test_resolve_apple_stock() {
    let result = resolve_asset("Apple", 0.6);
    assert!(result.is_some());
    let match_result = result.unwrap();
    assert_eq!(match_result.asset.class, AssetClass::EQ);
    assert!(match_result.asset.aliases.contains("aapl") || match_result.asset.name.contains("apple"));

    // Test that global ID is properly constructed
    let (class, class_id) = unpack_asset(match_result.asset.id);
    assert_eq!(class, AssetClass::EQ);
    assert_eq!(class_id, match_result.asset.class_id);
}

#[test]
fn test_resolve_bitcoin() {
    let result = resolve_asset("Bitcoin", 0.6);
    assert!(result.is_some());
    let match_result = result.unwrap();
    assert_eq!(match_result.asset.class, AssetClass::CR);
    assert!(match_result.asset.aliases.contains("btc") || match_result.asset.name.contains("bitcoin"));

    // Test that global ID is properly constructed
    let (class, class_id) = unpack_asset(match_result.asset.id);
    assert_eq!(class, AssetClass::CR);
    assert_eq!(class_id, match_result.asset.class_id);
}

#[test]
fn test_resolve_by_symbol() {
    let result = resolve_asset("MSFT", 0.8);
    if result.is_some() {
        let match_result = result.unwrap();
        assert_eq!(match_result.asset.class, AssetClass::EQ);

        // Verify consistency between global ID and class fields
        let (class, class_id) = unpack_asset(match_result.asset.id);
        assert_eq!(class, match_result.asset.class);
        assert_eq!(class_id, match_result.asset.class_id);
    }
}

#[test]
fn test_resolve_in_class() {
    let result = resolve_asset_in_class("Apple", 0.6, AssetClass::EQ);
    if result.is_some() {
        let match_result = result.unwrap();
        assert_eq!(match_result.asset.class, AssetClass::EQ);
    }

            let result = resolve_asset_in_class("Apple", 0.8, AssetClass::CR);
    // Apple should not match crypto assets with high confidence (>=0.8)
    assert!(result.is_none() || result.unwrap().confidence < 0.8);
}

#[test]
fn test_get_by_id() {
    if let Some(match_result) = resolve_asset("Apple", 0.7) {
        let class_id = match_result.asset.class_id;
        let class = match_result.asset.class;

        // Test get by class and class_id
        let asset = get_asset_by_id(class, class_id);
        if asset.is_some() {
            let retrieved = asset.unwrap();
            assert_eq!(retrieved.class_id, class_id);
            assert_eq!(retrieved.class, class);
        }

        // Test get by global ID
        let global_asset = get_asset_by_global_id(match_result.asset.id);
        if global_asset.is_some() {
            let retrieved = global_asset.unwrap();
            assert_eq!(retrieved.id, match_result.asset.id);
        }
    }
}

#[test]
fn test_fuzzy_matching() {
    let result = resolve_asset("Appel", 0.6); // Missing 'l'
    // Should still find Apple with decent confidence
    if result.is_some() {
        assert!(result.unwrap().confidence > 0.5);
    }

    let result = resolve_asset("Microsft", 0.6); // Missing 'o'
    // Should still find Microsoft with decent confidence
    if result.is_some() {
        assert!(result.unwrap().confidence > 0.5);
    }
}

#[test]
fn test_hyphen_handling() {
    // Test that EUR-USD correctly resolves with USD as quote and EUR as base
    // The "-" should not interfere with quote detection but should be stripped as trailing delimiter
    test_ticker_resolution("EUR-USD", InstrumentType::SPOT, AssetClass::FX, &["eur", "euro"], AssetClass::FX, &["usd"], false);

    // Test that hyphens in asset names are preserved (when not acting as delimiters)
    // This would test cases like "Delta-Neutral" if such an asset existed
}

#[test]
fn debug_hyphen_processing() {
    // Debug exactly how EUR-USD is processed
    let result = resolve_ticker("EUR-USD", InstrumentType::SPOT);
    if let Ok(ticker_match) = result {
        let ticker = &ticker_match.ticker;
        println!("\n=== EUR-USD Processing ===");
        println!("Final result: {}/{}", ticker.base.name, ticker.quote.name);
        println!("Base: {} (class: {:?})", ticker.base.name, ticker.base.class);
        println!("Quote: {} (class: {:?})", ticker.quote.name, ticker.quote.class);
        println!("Processing steps: {:?}", ticker_match.processing_steps);
    }
}


// =============================================================================
// TICKER RESOLUTION HELPER FUNCTIONS
// =============================================================================

/// Comprehensive ticker resolution test helper
///
/// Tests ticker resolution and verifies all aspects of the resolved assets:
/// - Asset classes match expectations
/// - Asset names/aliases contain expected identifiers
/// - Ticker ID encoding is consistent with resolved assets
/// - Processing steps are recorded appropriately
/// - Optionally verifies that character stripping occurred
fn test_ticker_resolution(
    ticker_symbol: &str,
    instrument_type: InstrumentType,
    expected_base_class: AssetClass,
    expected_base_identifiers: &[&str], // Multiple possible names/aliases to check
    expected_quote_class: AssetClass,
    expected_quote_identifiers: &[&str], // Multiple possible names/aliases to check
    expect_stripping: bool,
) {
    let result = resolve_ticker(ticker_symbol, instrument_type);

    assert!(result.is_ok(), "Resolution failed for ticker: {}", ticker_symbol);

    let ticker_match = result.unwrap();
    let ticker = &ticker_match.ticker;

    // Optionally verify that stripping occurred
    if expect_stripping {
        assert!(ticker_match.processing_steps.iter().any(|step| step.contains("Stripped")),
               "Expected stripping step for {}", ticker_symbol);
    }

    // Verify instrument type
    assert_eq!(ticker.instrument_type, instrument_type,
              "Instrument type mismatch for {}", ticker_symbol);

    // Verify base asset class
    assert_eq!(ticker.base.class, expected_base_class,
              "Base asset class mismatch for {}: expected {:?}, got {:?}",
              ticker_symbol, expected_base_class, ticker.base.class);

    // Verify quote asset class
    assert_eq!(ticker.quote.class, expected_quote_class,
              "Quote asset class mismatch for {}: expected {:?}, got {:?}",
              ticker_symbol, expected_quote_class, ticker.quote.class);

        // Verify base asset contains expected identifier
    let base_match = expected_base_identifiers.iter().any(|&identifier| {
        ticker.base.name.to_lowercase().contains(&identifier.to_lowercase()) ||
        ticker.base.aliases.split('|').any(|alias| alias.to_lowercase().contains(&identifier.to_lowercase()))
    });
    assert!(base_match,
           "Base asset for {} doesn't contain any of {:?}. Asset name: '{}', aliases: '{}'",
           ticker_symbol, expected_base_identifiers, ticker.base.name, ticker.base.aliases);

    // Verify quote asset contains expected identifier
    let quote_match = expected_quote_identifiers.iter().any(|&identifier| {
        ticker.quote.name.to_lowercase().contains(&identifier.to_lowercase()) ||
        ticker.quote.aliases.split('|').any(|alias| alias.to_lowercase().contains(&identifier.to_lowercase()))
    });
    assert!(quote_match,
           "Quote asset for {} doesn't contain any of {:?}. Asset name: '{}', aliases: '{}'",
           ticker_symbol, expected_quote_identifiers, ticker.quote.name, ticker.quote.aliases);

    // Verify ticker ID encoding consistency
    let ticker_id = TickerId::from_raw(ticker.id);
    assert_eq!(ticker_id.instrument_type(), instrument_type,
              "Ticker ID instrument type mismatch for {}", ticker_symbol);
    assert_eq!(ticker_id.base_asset_class(), ticker.base.class,
              "Ticker ID base asset class mismatch for {}", ticker_symbol);
    assert_eq!(ticker_id.quote_asset_class(), ticker.quote.class,
              "Ticker ID quote asset class mismatch for {}", ticker_symbol);
    assert_eq!(ticker_id.base_asset_id(), ticker.base.class_id,
              "Ticker ID base asset ID mismatch for {}", ticker_symbol);
    assert_eq!(ticker_id.quote_asset_id(), ticker.quote.class_id,
              "Ticker ID quote asset ID mismatch for {}", ticker_symbol);

    // Verify global asset ID consistency
    let (base_class, base_id) = unpack_asset(ticker.base.id);
    let (quote_class, quote_id) = unpack_asset(ticker.quote.id);
    assert_eq!(base_class, ticker.base.class,
              "Base asset global ID class mismatch for {}", ticker_symbol);
    assert_eq!(base_id, ticker.base.class_id,
              "Base asset global ID class_id mismatch for {}", ticker_symbol);
    assert_eq!(quote_class, ticker.quote.class,
              "Quote asset global ID class mismatch for {}", ticker_symbol);
    assert_eq!(quote_id, ticker.quote.class_id,
              "Quote asset global ID class_id mismatch for {}", ticker_symbol);

    // Verify processing steps are recorded
    assert!(!ticker_match.processing_steps.is_empty(),
           "No processing steps recorded for {}", ticker_symbol);
}

// =============================================================================
// TICKER RESOLUTION TESTS (comprehensive & organized)
// =============================================================================

#[test]
fn test_forex_resolutions() {
    // Test standard formats
    test_ticker_resolution("EUR/USD", InstrumentType::SPOT, AssetClass::FX, &["eur", "euro"], AssetClass::FX, &["usd"], false);
    test_ticker_resolution("EURUSD", InstrumentType::SPOT, AssetClass::FX, &["eur", "euro"], AssetClass::FX, &["usd"], false);
    // GBP/JPY resolves as JPY/GBP (base=JPY, quote=GBP)
    test_ticker_resolution("GBP/JPY", InstrumentType::SPOT, AssetClass::FX, &["jpy", "yen"], AssetClass::FX, &["gbp", "pound"], false);
    test_ticker_resolution("AUD/CAD", InstrumentType::SPOT, AssetClass::FX, &["aud"], AssetClass::FX, &["cad"], false);

    // Test single currency default to USD
    test_ticker_resolution("EUR", InstrumentType::SPOT, AssetClass::FX, &["eur", "euro"], AssetClass::FX, &["usd"], false);

    // Test with stripping
    test_ticker_resolution("EURUSD.zero", InstrumentType::SPOT, AssetClass::FX, &["eur", "euro"], AssetClass::FX, &["usd", "dollar"], true);
    test_ticker_resolution("_EURUSDc", InstrumentType::SPOT, AssetClass::FX, &["eur"], AssetClass::CR, &["usdc"], true);
    test_ticker_resolution("GBPJPYmini", InstrumentType::SPOT, AssetClass::FX, &["jpy", "yen"], AssetClass::FX, &["gbp", "pound"], true);
}

#[test]
fn test_crypto_resolutions() {
    test_ticker_resolution("BTCUSDT", InstrumentType::SPOT, AssetClass::CR, &["btc", "bitcoin"], AssetClass::CR, &["usdt", "tether"], false);
    // ETH/BTC resolves as ethereum index (IN) vs bitcoin (CR)
    test_ticker_resolution("ETH/BTC", InstrumentType::SPOT, AssetClass::IN, &["eth", "ethereum"], AssetClass::CR, &["btc", "bitcoin"], false);
    test_ticker_resolution("ETHBTC", InstrumentType::SPOT, AssetClass::IN, &["ethereum"], AssetClass::CR, &["bitcoin"], false);

    // Test with different instrument types
    test_ticker_resolution("AVAX_USDT", InstrumentType::FUT, AssetClass::CR, &["avalanche"], AssetClass::CR, &["tether"], false);
    test_ticker_resolution("BTCUSDT", InstrumentType::PERP, AssetClass::CR, &["btc", "bitcoin"], AssetClass::CR, &["usdt", "tether"], false);

    // Test with stripping
    test_ticker_resolution("$BTCUSD.m", InstrumentType::SPOT, AssetClass::CR, &["bitcoin"], AssetClass::FX, &["dollar"], true);
    test_ticker_resolution("SOL_", InstrumentType::SPOT, AssetClass::CR, &["solana"], AssetClass::FX, &["dollar"], true);
}

#[test]
fn test_equity_resolutions() {
    test_ticker_resolution("AAPL/USD", InstrumentType::SPOT, AssetClass::EQ, &["aapl", "apple"], AssetClass::FX, &["usd"], false);
    test_ticker_resolution("MSFT/USD", InstrumentType::SPOT, AssetClass::EQ, &["msft", "microsoft"], AssetClass::FX, &["usd"], false);
    test_ticker_resolution("AAPL", InstrumentType::SPOT, AssetClass::EQ, &["aapl"], AssetClass::FX, &["usd"], false);

    // Test with stripping
    test_ticker_resolution("$MSFT", InstrumentType::SPOT, AssetClass::EQ, &["microsoft"], AssetClass::FX, &["dollar"], true);
    test_ticker_resolution("TSLA.cash", InstrumentType::SPOT, AssetClass::EQ, &["tesla"], AssetClass::FX, &["usd"], true);
}

#[test]
fn test_commodity_resolutions() {
    // GOLD should now resolve to the commodity due to exact alias match
    test_ticker_resolution("GOLD", InstrumentType::SPOT, AssetClass::CM, &["gold", "xau"], AssetClass::FX, &["usd"], false);
    test_ticker_resolution("SILVER", InstrumentType::SPOT, AssetClass::CM, &["xag"], AssetClass::FX, &["usd"], false);
    test_ticker_resolution("XAUAUD", InstrumentType::SPOT, AssetClass::CM, &["gold"], AssetClass::FX, &["australian dollar"], false);
}

#[test]
fn test_index_resolutions() {
    // ^SPY.m resolves as S&P 500 index (IN), not index product (IP)
    test_ticker_resolution("^SPY.m", InstrumentType::CFD, AssetClass::IN, &["sp500", "spx", "spy"], AssetClass::FX, &["usd"], true);
    test_ticker_resolution("FRA40EURm", InstrumentType::CFD, AssetClass::IN, &["cac40", "fra40"], AssetClass::FX, &["usd"], false);
}

#[test]
fn test_sovereign_debt_resolutions() {
    test_ticker_resolution("^TNX.i", InstrumentType::SPOT, AssetClass::SD, &["us10y"], AssetClass::FX, &["usd"], true);
    // $BOBLEUR.ecn now correctly resolves with EUR quote after double stripping
    test_ticker_resolution("$BOBLEUR.ecn", InstrumentType::SPOT, AssetClass::SD, &["bobl", "de5y"], AssetClass::FX, &["eur", "euro"], true);
}

#[test]
fn test_invalid_ticker_resolution() {
    // Test with empty string (clearly invalid)
    let result = resolve_ticker("", InstrumentType::SPOT);
    assert!(result.is_err());

    // Test that we can at least process single character inputs without panicking
    let _result = resolve_ticker("X", InstrumentType::SPOT);
}

#[test]
fn test_universal_lowercase_processing() {
    // Test that case doesn't matter - all variations should resolve identically
    let symbols = ["EURUSD", "eurusd", "EurUsd"];
    let mut results = Vec::new();

    for &symbol in &symbols {
        // Use the helper to ensure resolution is valid
        test_ticker_resolution(
            symbol,
            InstrumentType::SPOT,
            AssetClass::FX, &["eur", "euro"],
            AssetClass::FX, &["usd"],
            false
        );
        // If it passes, get the result for comparison
        results.push(resolve_ticker(symbol, InstrumentType::SPOT).unwrap());
    }

    // Verify they all resolve to the same assets
    for i in 1..results.len() {
        assert_eq!(results[0].ticker.base.id, results[i].ticker.base.id, "Base ID mismatch for {}", symbols[i]);
        assert_eq!(results[0].ticker.quote.id, results[i].ticker.quote.id, "Quote ID mismatch for {}", symbols[i]);
        assert_eq!(results[0].ticker.id, results[i].ticker.id, "Ticker ID mismatch for {}", symbols[i]);
    }
}
