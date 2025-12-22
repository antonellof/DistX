//! # vectX Core
//!
//! Core library for the vectX vector database.
//!
//! This crate provides the fundamental data structures and algorithms:
//!
//! - [`Vector`] - Dense vector representation with SIMD operations
//! - [`Point`] - A vector with ID and optional payload
//! - [`Collection`] - Container for points with indexing
//! - [`HnswIndex`] - HNSW approximate nearest neighbor index
//! - [`BM25Index`] - Full-text search with BM25 ranking
//!
//! ## Example
//!
//! ```rust
//! use vectx_core::{Vector, Point, PointId, Collection, CollectionConfig, Distance};
//!
//! // Create a collection
//! let config = CollectionConfig {
//!     name: "test".to_string(),
//!     vector_dim: 3,
//!     distance: Distance::Cosine,
//!     use_hnsw: true,
//!     enable_bm25: false,
//! };
//! let collection = Collection::new(config);
//!
//! // Insert a point
//! let vector = Vector::new(vec![1.0, 0.0, 0.0]);
//! let point = Point::new(PointId::String("p1".to_string()), vector, None);
//! collection.upsert(point).unwrap();
//!
//! // Search
//! let query = Vector::new(vec![1.0, 0.0, 0.0]);
//! let results = collection.search(&query, 10, None);
//! ```

pub mod collection;
pub mod vector;
pub mod error;
pub mod point;
pub mod hnsw;
pub mod graph;
pub mod bm25;
pub mod filter;
pub mod background;
pub mod multivector;

/// SIMD-optimized vector operations
///
/// Provides hardware-accelerated distance calculations:
/// - AVX2/FMA on x86_64
/// - SSE on x86
/// - NEON on ARM64/Apple Silicon
pub mod simd;

pub use collection::{Collection, CollectionConfig, Distance, PayloadIndexType};
pub use vector::Vector;
pub use error::{Error, Result};
pub use point::{Point, PointId, VectorData, SparseVector};
pub use hnsw::HnswIndex;
pub use graph::{Node, Edge, NodeId, EdgeId};
pub use bm25::BM25Index;
pub use filter::{Filter, PayloadFilter, FilterCondition};
pub use multivector::{MultiVector, MultiVectorConfig, MultiVectorComparator};

