//! MultiVector support for ColBERT-style late interaction retrieval
//!
//! Implements MaxSim (Maximum Similarity) scoring as described in:
//! <https://arxiv.org/pdf/2112.01488.pdf>

use serde::{Deserialize, Serialize};
use crate::Vector;

/// Configuration for multivector comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MultiVectorComparator {
    /// MaxSim: For each query vector, find max similarity with any document vector, then sum
    /// This is the ColBERT algorithm for late interaction retrieval
    #[default]
    MaxSim,
}

/// Configuration for multivector storage and search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiVectorConfig {
    pub comparator: MultiVectorComparator,
}

/// A multivector - multiple dense vectors per point (ColBERT-style)
/// Each sub-vector typically represents a token embedding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiVector {
    /// The sub-vectors (each row is a dense vector)
    vectors: Vec<Vec<f32>>,
    /// Dimension of each sub-vector (all must be the same)
    dim: usize,
}

impl MultiVector {
    /// Create a new multivector from a list of sub-vectors
    /// All sub-vectors must have the same dimension
    pub fn new(vectors: Vec<Vec<f32>>) -> Result<Self, &'static str> {
        if vectors.is_empty() {
            return Err("MultiVector cannot be empty");
        }
        
        let dim = vectors[0].len();
        if dim == 0 {
            return Err("Sub-vectors cannot be empty");
        }
        
        // Verify all vectors have the same dimension
        if !vectors.iter().all(|v| v.len() == dim) {
            return Err("All sub-vectors must have the same dimension");
        }
        
        Ok(Self { vectors, dim })
    }
    
    /// Create from a single dense vector (wraps it as a multivector with one sub-vector)
    pub fn from_single(vector: Vec<f32>) -> Result<Self, &'static str> {
        if vector.is_empty() {
            return Err("Vector cannot be empty");
        }
        let dim = vector.len();
        Ok(Self { vectors: vec![vector], dim })
    }
    
    /// Get the dimension of each sub-vector
    #[inline]
    pub fn dim(&self) -> usize {
        self.dim
    }
    
    /// Get the number of sub-vectors
    #[inline]
    pub fn len(&self) -> usize {
        self.vectors.len()
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
    
    /// Get a reference to the sub-vectors
    #[inline]
    pub fn vectors(&self) -> &[Vec<f32>] {
        &self.vectors
    }
    
    /// Get the first sub-vector (useful for backwards compatibility)
    #[inline]
    pub fn first(&self) -> Option<&Vec<f32>> {
        self.vectors.first()
    }
    
    /// Convert to a single Vector (using first sub-vector)
    /// Used for backwards compatibility with non-multivector operations
    pub fn to_single_vector(&self) -> Vector {
        Vector::new(self.vectors[0].clone())
    }
    
    /// Compute MaxSim score between two multivectors
    /// 
    /// For each sub-vector in `self` (query), find the maximum similarity 
    /// with any sub-vector in `other` (document), then sum all maximums.
    /// 
    /// This is the ColBERT scoring algorithm.
    pub fn max_sim(&self, other: &MultiVector) -> f32 {
        if self.dim != other.dim {
            return 0.0;
        }
        
        let mut total_score = 0.0;
        
        // For each query sub-vector
        for query_vec in &self.vectors {
            let mut max_sim = f32::NEG_INFINITY;
            
            // Find max similarity with any document sub-vector
            for doc_vec in &other.vectors {
                let sim = dot_product(query_vec, doc_vec);
                if sim > max_sim {
                    max_sim = sim;
                }
            }
            
            // Only add if we found a valid similarity
            if max_sim > f32::NEG_INFINITY {
                total_score += max_sim;
            }
        }
        
        total_score
    }
    
    /// Compute MaxSim with cosine similarity (normalized dot product)
    pub fn max_sim_cosine(&self, other: &MultiVector) -> f32 {
        if self.dim != other.dim {
            return 0.0;
        }
        
        let mut total_score = 0.0;
        
        for query_vec in &self.vectors {
            let query_norm = norm(query_vec);
            if query_norm < f32::EPSILON {
                continue;
            }
            
            let mut max_sim = f32::NEG_INFINITY;
            
            for doc_vec in &other.vectors {
                let doc_norm = norm(doc_vec);
                if doc_norm < f32::EPSILON {
                    continue;
                }
                
                let sim = dot_product(query_vec, doc_vec) / (query_norm * doc_norm);
                if sim > max_sim {
                    max_sim = sim;
                }
            }
            
            if max_sim > f32::NEG_INFINITY {
                total_score += max_sim;
            }
        }
        
        total_score
    }
    
    /// Compute MaxSim with negative L2 distance (for Euclidean)
    pub fn max_sim_l2(&self, other: &MultiVector) -> f32 {
        if self.dim != other.dim {
            return f32::NEG_INFINITY;
        }
        
        let mut total_score = 0.0;
        
        for query_vec in &self.vectors {
            let mut min_dist = f32::INFINITY;
            
            for doc_vec in &other.vectors {
                let dist = l2_distance(query_vec, doc_vec);
                if dist < min_dist {
                    min_dist = dist;
                }
            }
            
            if min_dist < f32::INFINITY {
                // Negative because we want higher scores for closer vectors
                total_score -= min_dist;
            }
        }
        
        total_score
    }
}

/// Simple dot product (can be replaced with SIMD version)
#[inline]
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Vector norm
#[inline]
fn norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// L2 distance
#[inline]
fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multivector_creation() {
        let mv = MultiVector::new(vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
        ]).unwrap();
        
        assert_eq!(mv.dim(), 3);
        assert_eq!(mv.len(), 2);
    }
    
    #[test]
    fn test_max_sim_identical() {
        let mv1 = MultiVector::new(vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
        ]).unwrap();
        
        let mv2 = MultiVector::new(vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
        ]).unwrap();
        
        // Each query vector should match perfectly with one doc vector
        // Score = 1.0 + 1.0 = 2.0
        let score = mv1.max_sim(&mv2);
        assert!((score - 2.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_max_sim_different() {
        let query = MultiVector::new(vec![
            vec![1.0, 0.0],
        ]).unwrap();
        
        let doc = MultiVector::new(vec![
            vec![0.5, 0.5],
            vec![1.0, 0.0],
        ]).unwrap();
        
        // Query has 1 vector [1,0], max sim with doc vectors is [1,0] = 1.0
        let score = query.max_sim(&doc);
        assert!((score - 1.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_max_sim_cosine() {
        let query = MultiVector::new(vec![
            vec![2.0, 0.0],  // Not normalized
        ]).unwrap();
        
        let doc = MultiVector::new(vec![
            vec![1.0, 0.0],
        ]).unwrap();
        
        // Cosine similarity should be 1.0 regardless of magnitude
        let score = query.max_sim_cosine(&doc);
        assert!((score - 1.0).abs() < 1e-6);
    }
}
