use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

/// A vector of floating point numbers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vector {
    data: Vec<f32>,
}

impl Vector {
    #[inline]
    #[must_use]
    pub fn new(data: Vec<f32>) -> Self {
        Self { data }
    }

    #[inline]
    #[must_use]
    pub fn from_slice(data: &[f32]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    #[inline]
    #[must_use]
    pub fn dim(&self) -> usize {
        self.data.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Compute cosine similarity with another vector
    /// Uses SIMD-optimized operations for both dot product and norms
    #[inline]
    pub fn cosine_similarity(&self, other: &Vector) -> f32 {
        if self.dim() != other.dim() {
            return 0.0;
        }

        let dot_product = crate::simd::dot_product_simd(&self.data, &other.data);

        // Use SIMD-optimized norm calculation
        let norm_a = crate::simd::norm_simd(&self.data);
        let norm_b = crate::simd::norm_simd(&other.data);

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Compute L2 (Euclidean) distance
    #[inline]
    pub fn l2_distance(&self, other: &Vector) -> f32 {
        if self.dim() != other.dim() {
            return f32::INFINITY;
        }

        crate::simd::l2_distance_simd(&self.data, &other.data)
    }

    /// Normalize the vector to unit length
    /// Uses SIMD-optimized norm calculation
    #[inline]
    pub fn normalize(&mut self) {
        let norm = crate::simd::norm_simd(&self.data);
        if norm > f32::EPSILON {
            let inv_norm = 1.0 / norm;
            for x in &mut self.data {
                *x *= inv_norm;
            }
        }
    }

    /// Get normalized copy
    #[inline]
    #[must_use]
    pub fn normalized(&self) -> Self {
        let mut v = self.clone();
        v.normalize();
        v
    }
}

impl Add for &Vector {
    type Output = Vector;

    fn add(self, other: &Vector) -> Vector {
        assert_eq!(self.dim(), other.dim());
        Vector::new(
            self.data
                .iter()
                .zip(other.data.iter())
                .map(|(a, b)| a + b)
                .collect(),
        )
    }
}

impl Sub for &Vector {
    type Output = Vector;

    fn sub(self, other: &Vector) -> Vector {
        assert_eq!(self.dim(), other.dim());
        Vector::new(
            self.data
                .iter()
                .zip(other.data.iter())
                .map(|(a, b)| a - b)
                .collect(),
        )
    }
}

impl Mul<f32> for &Vector {
    type Output = Vector;

    fn mul(self, scalar: f32) -> Vector {
        Vector::new(self.data.iter().map(|x| x * scalar).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let v1 = Vector::new(vec![1.0, 0.0]);
        let v2 = Vector::new(vec![1.0, 0.0]);
        assert!((v1.cosine_similarity(&v2) - 1.0).abs() < 1e-6);

        let v3 = Vector::new(vec![1.0, 0.0]);
        let v4 = Vector::new(vec![0.0, 1.0]);
        assert!((v3.cosine_similarity(&v4) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_l2_distance() {
        let v1 = Vector::new(vec![0.0, 0.0]);
        let v2 = Vector::new(vec![3.0, 4.0]);
        assert!((v1.l2_distance(&v2) - 5.0).abs() < 1e-6);
    }
}

