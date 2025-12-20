//! Distance and similarity functions for different field types
//!
//! Provides per-field distance calculations used in structured similarity.
//! All functions return a similarity score in range [0.0, 1.0] where 1.0 means identical.

use crate::schema::DistanceType;
use std::collections::HashSet;

/// Calculate text similarity between two strings
/// 
/// # Arguments
/// * `a` - First text value
/// * `b` - Second text value  
/// * `method` - The similarity method to use
/// 
/// # Returns
/// Similarity score in [0.0, 1.0]
pub fn text_similarity(a: &str, b: &str, method: DistanceType) -> f32 {
    match method {
        DistanceType::Semantic => trigram_similarity(a, b),
        DistanceType::Exact => if a.eq_ignore_ascii_case(b) { 1.0 } else { 0.0 },
        DistanceType::Overlap => jaccard_tokens(a, b),
        _ => trigram_similarity(a, b), // Default to trigram for text
    }
}

/// Calculate numeric distance/similarity between two numbers
/// 
/// # Arguments
/// * `a` - First numeric value
/// * `b` - Second numeric value
/// * `method` - The distance method to use
/// 
/// # Returns
/// Similarity score in [0.0, 1.0]
pub fn number_similarity(a: f64, b: f64, method: DistanceType) -> f32 {
    match method {
        DistanceType::Absolute => {
            // Assumes values are typically in similar ranges
            // Uses exponential decay for difference
            let diff = (a - b).abs();
            let scale = (a.abs() + b.abs() + 1.0) / 2.0; // Adaptive scale
            (-diff / scale).exp() as f32
        }
        DistanceType::Relative => {
            let max = a.abs().max(b.abs());
            if max == 0.0 {
                1.0 // Both are zero, perfect match
            } else {
                let relative_diff = (a - b).abs() / max;
                (1.0 - relative_diff).max(0.0) as f32
            }
        }
        DistanceType::Exact => {
            if (a - b).abs() < f64::EPSILON {
                1.0
            } else {
                0.0
            }
        }
        _ => {
            // Default: relative distance
            let max = a.abs().max(b.abs());
            if max == 0.0 {
                1.0
            } else {
                (1.0 - (a - b).abs() / max).max(0.0) as f32
            }
        }
    }
}

/// Calculate categorical similarity between two category values
/// 
/// # Arguments
/// * `a` - First category value
/// * `b` - Second category value
/// * `method` - The similarity method to use
/// 
/// # Returns
/// Similarity score in [0.0, 1.0]
pub fn categorical_similarity(a: &str, b: &str, method: DistanceType) -> f32 {
    match method {
        DistanceType::Exact => {
            if a.eq_ignore_ascii_case(b) { 1.0 } else { 0.0 }
        }
        DistanceType::Overlap => jaccard_tokens(a, b),
        _ => {
            // Default: exact match
            if a.eq_ignore_ascii_case(b) { 1.0 } else { 0.0 }
        }
    }
}

/// Calculate boolean similarity
/// 
/// # Returns
/// 1.0 if both values are the same, 0.0 otherwise
pub fn boolean_similarity(a: bool, b: bool) -> f32 {
    if a == b { 1.0 } else { 0.0 }
}

/// Calculate Jaccard similarity between token sets
/// 
/// Tokenizes strings by whitespace and computes Jaccard index
fn jaccard_tokens(a: &str, b: &str) -> f32 {
    let tokens_a: HashSet<&str> = a.split_whitespace()
        .map(|s| s.to_lowercase().leak() as &str)
        .collect();
    let tokens_b: HashSet<&str> = b.split_whitespace()
        .map(|s| s.to_lowercase().leak() as &str)
        .collect();
    
    if tokens_a.is_empty() && tokens_b.is_empty() {
        return 1.0;
    }
    
    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();
    
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

/// Calculate trigram similarity between two strings
/// 
/// Uses character trigrams for fuzzy text matching
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
    
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
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

/// Hash a string to a fixed-size vector for embedding
/// 
/// Uses a simple but effective hash-based approach for v1.
/// Can be replaced with ML embeddings in future versions.
pub fn hash_text_to_vector(text: &str, dim: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut vector = vec![0.0f32; dim];
    let normalized = text.to_lowercase();
    
    // Generate trigrams and hash them to vector positions
    let trigrams = generate_trigrams(&normalized);
    
    for trigram in trigrams {
        let mut hasher = DefaultHasher::new();
        trigram.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Map hash to vector position
        let pos = (hash as usize) % dim;
        vector[pos] += 1.0;
    }
    
    // Also add word-level hashing
    for word in normalized.split_whitespace() {
        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        let hash = hasher.finish();
        
        let pos = (hash as usize) % dim;
        vector[pos] += 2.0; // Words contribute more
    }
    
    // Normalize the vector
    let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for v in &mut vector {
            *v /= magnitude;
        }
    }
    
    vector
}

/// Hash a categorical value to a fixed-size vector
pub fn hash_categorical_to_vector(value: &str, dim: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut vector = vec![0.0f32; dim];
    let normalized = value.to_lowercase();
    
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Use multiple hash positions for better distribution
    for i in 0..4 {
        let pos = ((hash >> (i * 16)) as usize) % dim;
        vector[pos] = 1.0;
    }
    
    // Normalize
    let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for v in &mut vector {
            *v /= magnitude;
        }
    }
    
    vector
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
    fn test_text_semantic_similarity() {
        let sim = text_similarity("prosciutto cotto", "prosciutto crudo", DistanceType::Semantic);
        assert!(sim > 0.5); // Should have high similarity due to shared trigrams
        
        let sim2 = text_similarity("apple", "banana", DistanceType::Semantic);
        assert!(sim2 < 0.3); // Should have low similarity
    }

    #[test]
    fn test_number_relative_similarity() {
        // Same values
        assert_eq!(number_similarity(10.0, 10.0, DistanceType::Relative), 1.0);
        
        // Close values
        let sim = number_similarity(10.0, 11.0, DistanceType::Relative);
        assert!(sim > 0.9);
        
        // Different values
        let sim2 = number_similarity(10.0, 20.0, DistanceType::Relative);
        assert!(sim2 >= 0.5 && sim2 < 0.6);
    }

    #[test]
    fn test_number_absolute_similarity() {
        // Same values
        let sim = number_similarity(10.0, 10.0, DistanceType::Absolute);
        assert!((sim - 1.0).abs() < 0.001);
        
        // Close values should have high similarity
        let sim2 = number_similarity(10.0, 11.0, DistanceType::Absolute);
        assert!(sim2 > 0.5);
    }

    #[test]
    fn test_categorical_exact_similarity() {
        assert_eq!(categorical_similarity("electronics", "ELECTRONICS", DistanceType::Exact), 1.0);
        assert_eq!(categorical_similarity("electronics", "clothing", DistanceType::Exact), 0.0);
    }

    #[test]
    fn test_boolean_similarity() {
        assert_eq!(boolean_similarity(true, true), 1.0);
        assert_eq!(boolean_similarity(false, false), 1.0);
        assert_eq!(boolean_similarity(true, false), 0.0);
    }

    #[test]
    fn test_hash_text_to_vector() {
        let vec1 = hash_text_to_vector("hello world", 64);
        let vec2 = hash_text_to_vector("hello world", 64);
        let vec3 = hash_text_to_vector("goodbye moon", 64);
        
        assert_eq!(vec1.len(), 64);
        assert_eq!(vec1, vec2); // Same text should produce same vector
        assert_ne!(vec1, vec3); // Different text should produce different vector
        
        // Vector should be normalized
        let magnitude: f32 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_trigram_generation() {
        let trigrams = generate_trigrams("hello");
        assert!(!trigrams.is_empty());
        assert!(trigrams.contains("hel"));
        assert!(trigrams.contains("ell"));
        assert!(trigrams.contains("llo"));
    }
}
