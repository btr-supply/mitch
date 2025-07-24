//! Integration tests for the Index message type.
//!
//! This file contains tests for:
//! - Correct size of the Index struct.
//! - Packing and unpacking (serialization/deserialization) roundtrip.
//! - Validation logic for all fields.
//! - Correctness of derived data calculations (e.g., percentages, prices).
//! - Batch operations for packing and unpacking multiple messages.

// Allow certain lints for testing purposes
#![allow(clippy::all)]
use mitch::{self, common::*, index::*};

#[cfg(test)]
mod tests {
    use super::*;
    use mitch::{MitchError, pack_index_batch, unpack_index_batch};

    /// Returns a default, valid Index message for testing.
    fn get_default_index() -> Index {
        Index::new(
            0x0A00_6F30_1CD0_0001, // Ticker: Indices, EUR/USD, Venue 1
            1.08750,              // mid
            1_000_000,            // v_bid
            1_200_000,            // v_ask
            15,                   // m_spread (1.5 pips)
            -5,                   // b_bid_o
            5,                    // b_ask_o
            -20,                  // w_bid_o
            20,                   // w_ask_o
            2500,                 // v_force (25.00%)
            7500,                 // l_force (75.00%)
            1500,                 // t_force (15.00%)
            -500,                 // m_force (-5.00%)
            95,                   // confidence (95/100)
            1,                    // rejected
            9,                    // accepted
        )
    }

    #[test]
    fn test_index_size() {
        assert_eq!(core::mem::size_of::<Index>(), message_sizes::INDEX);
        assert_eq!(core::mem::size_of::<Index>(), 64);
    }

    #[test]
    fn test_index_pack_unpack_roundtrip() {
        let original = get_default_index();
        let packed = original.pack();
        let unpacked = Index::unpack(&packed).unwrap();

        assert_eq!(original, unpacked); // Complete struct comparison
    }

    #[test]
    fn test_index_validation() {
        let mut index = get_default_index();
        assert!(index.validate().is_ok(), "Default index should be valid");

        // --- Invalid fields ---

        // Invalid Ticker ID
        index.ticker_id = 0;
        assert!(index.validate().is_err());
        index.ticker_id = get_default_index().ticker_id; // Restore

        // Invalid Mid Price
        index.mid = 0.0;
        assert!(index.validate().is_err());
        index.mid = -1.0;
        assert!(index.validate().is_err());
        index.mid = get_default_index().mid; // Restore

        // Invalid Confidence
        index.confidence = 101;
        assert!(index.validate().is_err());
        index.confidence = get_default_index().confidence; // Restore

        // Invalid VForce
        index.v_force = 10001;
        assert!(index.validate().is_err());
        index.v_force = get_default_index().v_force; // Restore

        // Invalid LForce
        index.l_force = 10001;
        assert!(index.validate().is_err());
        index.l_force = get_default_index().l_force; // Restore

        // Invalid TForce
        index.t_force = -10001;
        assert!(index.validate().is_err());
        index.t_force = 10001;
        assert!(index.validate().is_err());
        index.t_force = get_default_index().t_force; // Restore

        // Invalid MForce
        index.m_force = -10001;
        assert!(index.validate().is_err());
        index.m_force = 10001;
        assert!(index.validate().is_err());
        index.m_force = get_default_index().m_force; // Restore

        // Logical inconsistency: confidence without accepted sources
        index.accepted = 0;
        index.confidence = 1;
        assert!(index.validate().is_err());
    }

    #[test]
    fn test_index_derived_calculations() {
        let index = get_default_index();

        // --- Price Calculations ---
        // Best Bid: 1.08750 * (1 - 5 / 1,000,000,000) = 1.0874999945625
        // Best Ask: 1.08750 * (1 + 5 / 1,000,000,000) = 1.0875000054375
        assert!((index.best_bid_price() - 1.0874999945625).abs() < 1e-12);
        assert!((index.best_ask_price() - 1.0875000054375).abs() < 1e-12);

        // --- Percentage Calculations ---
        assert_eq!(index.volatility_percentage(), 25.0);
        assert_eq!(index.liquidity_percentage(), 75.0);
        assert_eq!(index.trend_percentage(), 15.0);
        assert_eq!(index.momentum_percentage(), -5.0);
    }

    #[test]
    fn test_index_batch_operations() {
        let index1 = get_default_index();
        let mut index2 = get_default_index();
        index2.ticker_id = 0x0A00_6F30_1CD0_0002; // Change ticker for uniqueness
        index2.mid = 1.09000;

        let messages = vec![index1, index2];
        let packed = pack_index_batch(&messages);
        let unpacked = unpack_index_batch(&packed, 2).unwrap();

        assert_eq!(messages.len(), unpacked.len());
        assert_eq!(messages[0], unpacked[0]);
        assert_eq!(messages[1], unpacked[1]);
    }

    #[test]
    fn test_unpack_error_handling() {
        let original = get_default_index();
        let packed = original.pack();

        // Buffer too small
        let res = Index::unpack(&packed[..63]);
        assert!(matches!(
            res,
            Err(MitchError::BufferTooSmall {
                expected: 64,
                actual: 63
            })
        ));

        // Batch buffer too small
        let res_batch = unpack_index_batch(&packed, 2);
        assert!(matches!(
            res_batch,
            Err(MitchError::BufferTooSmall {
                expected: 128,
                actual: 64
            })
        ));
    }
}
