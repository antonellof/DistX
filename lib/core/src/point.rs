use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use crate::vector::Vector;
use crate::multivector::MultiVector;

/// Sparse vector with indices and values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SparseVector {
    /// Indices of non-zero elements
    pub indices: Vec<u32>,
    /// Values at those indices
    pub values: Vec<f32>,
}

impl SparseVector {
    /// Create a new sparse vector
    pub fn new(indices: Vec<u32>, values: Vec<f32>) -> Self {
        Self { indices, values }
    }
    
    /// Compute dot product with another sparse vector
    pub fn dot(&self, other: &SparseVector) -> f32 {
        let mut result = 0.0f32;
        
        // Create a map of indices to values for efficient lookup
        let other_map: HashMap<u32, f32> = other.indices.iter()
            .zip(other.values.iter())
            .map(|(&i, &v)| (i, v))
            .collect();
        
        // Sum products for matching indices
        for (&idx, &val) in self.indices.iter().zip(self.values.iter()) {
            if let Some(&other_val) = other_map.get(&idx) {
                result += val * other_val;
            }
        }
        
        result
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Vector data - can be a single dense vector or a multivector (ColBERT-style)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum VectorData {
    /// Single dense vector
    Single(Vector),
    /// Multiple vectors per point (ColBERT-style late interaction)
    Multi(MultiVector),
}

impl VectorData {
    /// Get dimension of the vector(s)
    pub fn dim(&self) -> usize {
        match self {
            VectorData::Single(v) => v.dim(),
            VectorData::Multi(mv) => mv.dim(),
        }
    }
    
    /// Check if this is a multivector
    pub fn is_multi(&self) -> bool {
        matches!(self, VectorData::Multi(_))
    }
    
    /// Get as single vector (for backwards compatibility)
    /// For multivector, returns the first sub-vector
    pub fn as_single(&self) -> Vector {
        match self {
            VectorData::Single(v) => v.clone(),
            VectorData::Multi(mv) => mv.to_single_vector(),
        }
    }
    
    /// Get as slice (for single vectors only)
    pub fn as_slice(&self) -> &[f32] {
        match self {
            VectorData::Single(v) => v.as_slice(),
            VectorData::Multi(mv) => mv.vectors().first().map(|v| v.as_slice()).unwrap_or(&[]),
        }
    }
}

impl From<Vector> for VectorData {
    fn from(v: Vector) -> Self {
        VectorData::Single(v)
    }
}

impl From<MultiVector> for VectorData {
    fn from(mv: MultiVector) -> Self {
        VectorData::Multi(mv)
    }
}

/// A point in the vector space with optional payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub id: PointId,
    /// Version number - incremented on each update
    #[serde(default)]
    pub version: u64,
    /// Vector data - backwards compatible field name
    #[serde(alias = "vectors")]
    pub vector: Vector,
    /// Optional multivector data for ColBERT-style search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multivector: Option<MultiVector>,
    /// Named sparse vectors (e.g., {"keywords": SparseVector})
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub sparse_vectors: HashMap<String, SparseVector>,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointId {
    String(String),
    Uuid(Uuid),
    Integer(u64),
}

impl std::fmt::Display for PointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PointId::String(s) => write!(f, "{}", s),
            PointId::Uuid(u) => write!(f, "{}", u),
            PointId::Integer(i) => write!(f, "{}", i),
        }
    }
}

impl From<String> for PointId {
    fn from(s: String) -> Self {
        PointId::String(s)
    }
}

impl From<u64> for PointId {
    fn from(i: u64) -> Self {
        PointId::Integer(i)
    }
}

impl From<Uuid> for PointId {
    fn from(u: Uuid) -> Self {
        PointId::Uuid(u)
    }
}

impl Point {
    /// Create a new point with a single dense vector
    #[inline]
    #[must_use]
    pub fn new(id: PointId, vector: Vector, payload: Option<serde_json::Value>) -> Self {
        Self {
            id,
            version: 0,
            vector,
            multivector: None,
            sparse_vectors: HashMap::new(),
            payload,
        }
    }
    
    /// Create a new point with a multivector (ColBERT-style)
    #[inline]
    #[must_use]
    pub fn new_multi(id: PointId, multivector: MultiVector, payload: Option<serde_json::Value>) -> Self {
        // Store first sub-vector as the primary vector for backwards compatibility
        let vector = multivector.to_single_vector();
        Self {
            id,
            version: 0,
            vector,
            multivector: Some(multivector),
            sparse_vectors: HashMap::new(),
            payload,
        }
    }
    
    /// Create a new point with sparse vectors
    #[inline]
    #[must_use]
    pub fn new_sparse(id: PointId, sparse_vectors: HashMap<String, SparseVector>, payload: Option<serde_json::Value>) -> Self {
        Self {
            id,
            version: 0,
            vector: Vector::new(vec![0.0]), // Placeholder for sparse-only points
            multivector: None,
            sparse_vectors,
            payload,
        }
    }
    
    /// Add a sparse vector to this point
    pub fn add_sparse_vector(&mut self, name: String, sparse: SparseVector) {
        self.sparse_vectors.insert(name, sparse);
    }
    
    /// Get a sparse vector by name
    pub fn get_sparse_vector(&self, name: &str) -> Option<&SparseVector> {
        self.sparse_vectors.get(name)
    }
    
    /// Check if this point has multivector data
    #[inline]
    pub fn has_multivector(&self) -> bool {
        self.multivector.is_some()
    }
    
    /// Get the multivector if present
    #[inline]
    pub fn get_multivector(&self) -> Option<&MultiVector> {
        self.multivector.as_ref()
    }

    #[inline]
    #[must_use]
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }
    
    #[inline]
    #[must_use]
    pub fn with_multivector(mut self, multivector: MultiVector) -> Self {
        self.multivector = Some(multivector);
        self
    }
}

