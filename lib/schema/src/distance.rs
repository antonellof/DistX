//! Distance and similarity functions for structured field comparison
//!
//! These functions are used during reranking to compare payload fields.
//! All functions return a similarity score in range [0.0, 1.0] where 1.0 means identical.

use crate::schema::DistanceType;
use std::collections::HashSet;

/// Calculate text similarity between two strings
/// 
/// Uses trigram-based similarity for fuzzy matching during reranking.
/// For semantic similarity, use client-side embeddings + vector search.
pub fn text_similarity(a: &str, b: &str, method: DistanceType) -> f32 {
    match method {
        DistanceType::Semantic => trigram_similarity(a, b),
        DistanceType::Exact => if a.eq_ignore_ascii_case(b) { 1.0 } else { 0.0 },
        DistanceType::Overlap => jaccard_tokens(a, b),
        _ => trigram_similarity(a, b),
    }
}

/// Calculate numeric similarity between two numbers
pub fn number_similarity(a: f64, b: f64, method: DistanceType) -> f32 {
    match method {
        DistanceType::Absolute => {
            // Exponential decay based on difference
            let diff = (a - b).abs();
            let scale = (a.abs() + b.abs() + 1.0) / 2.0;
            (-diff / scale).exp() as f32
        }
        DistanceType::Relative => {
            let max = a.abs().max(b.abs());
            if max == 0.0 {
                1.0 // Both are zero
            } else {
                let relative_diff = (a - b).abs() / max;
                (1.0 - relative_diff).max(0.0) as f32
            }
        }
        DistanceType::Exact => {
            if (a - b).abs() < f64::EPSILON { 1.0 } else { 0.0 }
        }
        _ => {
            // Default: relative
            let max = a.abs().max(b.abs());
            if max == 0.0 { 1.0 } else { (1.0 - (a - b).abs() / max).max(0.0) as f32 }
        }
    }
}

/// Calculate categorical similarity between two values
pub fn categorical_similarity(a: &str, b: &str, method: DistanceType) -> f32 {
    match method {
        DistanceType::Exact => {
            if a.eq_ignore_ascii_case(b) { 1.0 } else { 0.0 }
        }
        DistanceType::Overlap => jaccard_tokens(a, b),
        _ => {
            if a.eq_ignore_ascii_case(b) { 1.0 } else { 0.0 }
        }
    }
}

/// Calculate boolean similarity
pub fn boolean_similarity(a: bool, b: bool) -> f32 {
    if a == b { 1.0 } else { 0.0 }
}

/// Jaccard similarity between token sets
fn jaccard_tokens(a: &str, b: &str) -> f32 {
    let tokens_a: HashSet<String> = a.split_whitespace()
        .map(|s| s.to_lowercase())
        .collect();
    let tokens_b: HashSet<String> = b.split_whitespace()
        .map(|s| s.to_lowercase())
        .collect();
    
    if tokens_a.is_empty() && tokens_b.is_empty() {
        return 1.0;
    }
    
    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();
    
    if union == 0 { 0.0 } else { intersection as f32 / union as f32 }
}

/// Trigram similarity between two strings
fn trigram_similarity(a: &str, b: &str) -> f32 {
    let trigrams_a = generate_trigrams(&a.to_lowercase());
    let trigrams_b = generate_trigrams(&b.to_lowercase());
    
    if trigrams_a.is_empty() && trigrams_b.is_empty() {
        return 1.0;
    }
    
    if trigrams_a.is_empty() || trigrams_b.is_empty() {
        return 0.0;
    }
    
    let intersection = trigrams_a.intersection(&trigrams_b).count();
    let union = trigrams_a.union(&trigrams_b).count();
    
    if union == 0 { 0.0 } else { intersection as f32 / union as f32 }
}

/// Generate character trigrams from a string
fn generate_trigrams(s: &str) -> HashSet<String> {
    let padded = format!("  {}  ", s);
    let chars: Vec<char> = padded.chars().collect();
    
    if chars.len() < 3 {
        return HashSet::new();
    }
    
    chars.windows(3)
        .map(|w| w.iter().collect::<String>())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_exact_similarity() {
        assert_eq!(text_similarity("hello", "HELLO", DistanceType::Exact), 1.0);
        assert_eq!(text_similarity("hello", "world", DistanceType::Exact), 0.0);
    }

    #[test]
    fn test_text_trigram_similarity() {
        let sim = text_similarity("prosciutto cotto", "prosciutto crudo", DistanceType::Semantic);
        assert!(sim > 0.5);
        
        let sim2 = text_similarity("apple", "banana", DistanceType::Semantic);
        assert!(sim2 < 0.3);
    }

    #[test]
    fn test_number_relative_similarity() {
        assert_eq!(number_similarity(10.0, 10.0, DistanceType::Relative), 1.0);
        
        let sim = number_similarity(10.0, 11.0, DistanceType::Relative);
        assert!(sim > 0.9);
        
        let sim2 = number_similarity(10.0, 20.0, DistanceType::Relative);
        assert!(sim2 >= 0.5 && sim2 < 0.6);
    }

    #[test]
    fn test_categorical_similarity() {
        assert_eq!(categorical_similarity("electronics", "ELECTRONICS", DistanceType::Exact), 1.0);
        assert_eq!(categorical_similarity("electronics", "clothing", DistanceType::Exact), 0.0);
    }

    #[test]
    fn test_boolean_similarity() {
        assert_eq!(boolean_similarity(true, true), 1.0);
        assert_eq!(boolean_similarity(false, false), 1.0);
        assert_eq!(boolean_similarity(true, false), 0.0);
    }
}
