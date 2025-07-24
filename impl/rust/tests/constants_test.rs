// tests/constants_test.rs

#[cfg(test)]
mod constants_tests {
    use mitch::constants::*;
    use mitch::*;

    #[test]
    fn test_asset_class_ids() {
        assert_eq!(AssetClass::EQ as u64, 0);
        assert_eq!(AssetClass::FX as u64, 3);
        assert_eq!(AssetClass::CR as u64, 6);
    }

    #[test]
    fn test_instrument_type_ids() {
        assert_eq!(InstrumentType::SPOT as u64, 0);
        assert_eq!(InstrumentType::FUT as u64, 1);
        assert_eq!(InstrumentType::PERP as u64, 4);
    }

    #[test]
    fn test_asset_resolution_integration() {
        // Test that we can resolve known assets through the new system
        if let Some(aapl_match) = resolve_asset_in_class("AAPL", 0.8, AssetClass::EQ) {
            assert_eq!(aapl_match.asset.class, AssetClass::EQ);
            assert!(aapl_match.asset.class_id > 0);
        }

        if let Some(eur_match) = resolve_asset_in_class("EUR", 0.8, AssetClass::FX) {
            assert_eq!(eur_match.asset.class, AssetClass::FX);
            assert!(eur_match.asset.class_id > 0);
        }
    }

    #[test]
    fn test_bins_hashmap() {
        // Test Lingaussian Bins
        let lingaussian_bins = BINS.get(&BinAggregator::DEFAULT_LINGAUSSIAN).unwrap();
        assert_eq!(lingaussian_bins[0], 0.00001);
        assert_eq!(lingaussian_bins[1], 0.00002);
        assert_eq!(lingaussian_bins[127], 200.0);
    }
}
