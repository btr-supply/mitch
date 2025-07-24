//! Tests for string similarity algorithms

use mitch::utils::similarity::*;

#[test]
fn test_jaro_winkler_identical() {
    assert_eq!(jaro_winkler_similarity("test", "test"), 1.0);
    assert_eq!(jaro_winkler_similarity("", ""), 1.0);
}

#[test]
fn test_jaro_winkler_empty() {
    assert_eq!(jaro_winkler_similarity("test", ""), 0.0);
    assert_eq!(jaro_winkler_similarity("", "test"), 0.0);
}

#[test]
fn test_jaro_winkler_similar() {
    let sim = jaro_winkler_similarity("apple", "appel");
    assert!(sim > 0.8); // Should be quite similar

    let sim = jaro_winkler_similarity("microsoft", "microsft");
    assert!(sim > 0.8); // Should be quite similar
}

#[test]
fn test_jaro_winkler_prefix_bonus() {
    // Jaro-Winkler should give higher scores for strings with common prefixes
    let sim1 = jaro_winkler_similarity("apple", "apply");
    let sim2 = jaro_winkler_similarity("apple", "grape");
    assert!(sim1 > sim2); // "apple"/"apply" should have higher similarity due to prefix
}

#[test]
fn test_enhanced_similarity_bonuses() {
    // Test substring bonus - "bit" is contained in "bitcoin"
    let sim = enhanced_similarity("bit", "bitcoin");
    assert!(sim > jaro_winkler_similarity("bit", "bitcoin"));

    // Test prefix bonus
    let sim = enhanced_similarity("micro", "microsoft");
    assert!(sim > jaro_winkler_similarity("micro", "microsoft"));
}

#[test]
fn test_length_weighted_similarity() {
    // Test that similar length strings get a bonus
    let sim1 = length_weighted_similarity("apple", "grape");
    let sim2 = jaro_winkler_similarity("apple", "grape");
    assert!(sim1 >= sim2); // Should be same or slightly higher due to similar lengths
}
