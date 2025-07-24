//! String similarity algorithms for fuzzy matching
//!
//! Implements Jaro-Winkler distance and additional bonuses for high-quality
//! fuzzy string matching in asset and market provider resolution.

/// Calculate Jaro-Winkler similarity between two strings
///
/// The Jaro-Winkler distance is a string metric measuring an edit distance
/// between two sequences. It is a variant of the Jaro distance metric designed
/// to give more favorable ratings to strings with common prefixes.
///
/// Returns a value between 0.0 (no similarity) and 1.0 (identical strings).
pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 { return 1.0; }
    if s1.is_empty() || s2.is_empty() { return 0.0; }

    let jaro = jaro_similarity(s1, s2);

    // Jaro-Winkler adds a prefix bonus
    let prefix_length = common_prefix_length(s1, s2).min(4) as f64; // Max 4 chars
    let winkler_bonus = 0.1 * prefix_length * (1.0 - jaro);

    jaro + winkler_bonus
}

/// Calculate Jaro similarity between two strings
///
/// The Jaro distance is a string metric measuring an edit distance between two sequences.
/// Returns a value between 0.0 and 1.0.
fn jaro_similarity(s1: &str, s2: &str) -> f64 {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();

    let len1 = chars1.len();
    let len2 = chars2.len();

    if len1 == 0 && len2 == 0 { return 1.0; }
    if len1 == 0 || len2 == 0 { return 0.0; }

    // Calculate the match window
    let match_window = if len1.max(len2) <= 2 {
        0
    } else {
        (len1.max(len2) / 2) - 1
    };

    let mut matches1 = vec![false; len1];
    let mut matches2 = vec![false; len2];

    let mut matches = 0;

    // Find matches
    for i in 0..len1 {
        let start = if i >= match_window { i - match_window } else { 0 };
        let end = (i + match_window + 1).min(len2);

        for j in start..end {
            if matches2[j] || chars1[i] != chars2[j] { continue; }
            matches1[i] = true;
            matches2[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 { return 0.0; }

    // Count transpositions
    let mut transpositions = 0;
    let mut k = 0;

    for i in 0..len1 {
        if !matches1[i] { continue; }
        while !matches2[k] { k += 1; }
        if chars1[i] != chars2[k] { transpositions += 1; }
        k += 1;
    }

    let jaro = (matches as f64 / len1 as f64 +
                matches as f64 / len2 as f64 +
                (matches as f64 - transpositions as f64 / 2.0) / matches as f64) / 3.0;

    jaro
}

/// Count the length of common prefix between two strings (up to 4 characters)
fn common_prefix_length(s1: &str, s2: &str) -> usize {
    s1.chars()
        .zip(s2.chars())
        .take(4)
        .take_while(|(a, b)| a == b)
        .count()
}

/// Calculate enhanced similarity with additional bonuses
///
/// This function combines Jaro-Winkler similarity with contextual bonuses
/// for substring matches and other patterns commonly found in asset/provider names.
pub fn enhanced_similarity(s1: &str, s2: &str) -> f64 {
    let base_similarity = jaro_winkler_similarity(s1, s2);

    // Add contextual bonuses
    let substring_bonus = if s2.contains(s1) || s1.contains(s2) { 0.15 } else { 0.0 };
    let prefix_bonus = if s2.starts_with(s1) || s1.starts_with(s2) { 0.1 } else { 0.0 };

    (base_similarity + substring_bonus + prefix_bonus).min(1.0)
}

/// Calculate similarity with length-based weighting
///
/// Gives preference to matches where the strings are similar in length,
/// which is useful for distinguishing between similar asset names.
pub fn length_weighted_similarity(s1: &str, s2: &str) -> f64 {
    let base_similarity = jaro_winkler_similarity(s1, s2);

    // Length difference penalty
    let len1 = s1.len() as f64;
    let len2 = s2.len() as f64;
    let length_ratio = len1.min(len2) / len1.max(len2);
    let length_bonus = length_ratio * 0.05; // Small bonus for similar lengths

    (base_similarity + length_bonus).min(1.0)
}


