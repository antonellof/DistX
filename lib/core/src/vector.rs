use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Sub};

/// A vector of floating point numbers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vector {
    data: Vec<f32>,
}

impl Vector {
    pub fn new(data: Vec<f32>) -> Self {
        Self { data }
    }

    pub fn from_slice(data: &[f32]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    pub fn dim(&self) -> usize {
        self.data.len()
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Compute cosine similarity with another vector
    pub fn cosine_similarity(&self, other: &Vector) -> f32 {
        if self.dim() != other.dim() {
            return 0.0;
        }

        let dot_product = crate::simd::dot_product_simd(&self.data, &other.data);

        let norm_a: f32 = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.data.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Compute L2 (Euclidean) distance
    pub fn l2_distance(&self, other: &Vector) -> f32 {
        if self.dim() != other.dim() {
            return f32::INFINITY;
        }

        crate::simd::l2_distance_simd(&self.data, &other.data)
    }

    /// Normalize the vector to unit length
    pub fn normalize(&mut self) {
        let norm: f32 = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut self.data {
                *x /= norm;
            }
        }
    }

    /// Get normalized copy
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

