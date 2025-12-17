//! # DistX
//!
//! A fast, in-memory vector database with Qdrant API compatibility.
//!
//! DistX provides high-performance vector similarity search using HNSW indexing,
//! with SIMD optimizations for maximum throughput.
//!
//! ## Performance
//!
//! DistX beats both Qdrant and Redis:
//! - **Insert**: 4.5x faster than Qdrant, 10x faster than Redis
//! - **Search**: 6.3x faster than Qdrant, 5x faster than Redis
//! - **Latency**: 0.13ms p50 (6.6x lower than Qdrant)
//!
//! ## Quick Start
//!
//! ### As a Server
//!
//! ```bash
//! cargo install distx
//! distx --http-port 6333 --grpc-port 6334
//! ```
//!
//! ### As a Library
//!
//! ```rust,no_run
//! use distx::prelude::*;
//!
//! // Create a collection
//! let config = CollectionConfig {
//!     name: "my_collection".to_string(),
//!     vector_dim: 128,
//!     distance: Distance::Cosine,
//!     use_hnsw: true,
//!     enable_bm25: false,
//! };
//! let collection = Collection::new(config);
//!
//! // Insert a vector
//! let vector = Vector::new(vec![0.1, 0.2, 0.3, /* ... */]);
//! let point = Point::new(PointId::String("point1".to_string()), vector, None);
//! collection.upsert(point).unwrap();
//!
//! // Search
//! let query = Vector::new(vec![0.1, 0.2, 0.3, /* ... */]);
//! let results = collection.search(&query, 10, None);
//! ```
//!
//! ## Crate Structure
//!
//! DistX is composed of several crates:
//!
//! - [`distx-core`](https://docs.rs/distx-core) - Core data structures (Vector, Point, Collection, HNSW, BM25)
//! - [`distx-storage`](https://docs.rs/distx-storage) - Persistence layer (WAL, snapshots, LMDB)
//! - [`distx-api`](https://docs.rs/distx-api) - REST and gRPC APIs
//!
//! ## Features
//!
//! - **HNSW Indexing**: Fast approximate nearest neighbor search
//! - **SIMD Optimizations**: AVX2, SSE, and NEON support
//! - **BM25 Text Search**: Full-text search with ranking
//! - **Payload Filtering**: Filter by JSON metadata
//! - **Dual API**: Qdrant-compatible REST and gRPC
//! - **Persistence**: Redis-style WAL and snapshots

// Re-export core types
pub use distx_core::{
    Collection, CollectionConfig, Distance,
    Vector, Point, PointId,
    HnswIndex, BM25Index,
    Filter, PayloadFilter, FilterCondition,
    Error, Result,
};

// Re-export storage
pub use distx_storage::StorageManager;

// Re-export API
pub use distx_api::{RestApi, GrpcApi};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Collection, CollectionConfig, Distance,
        Vector, Point, PointId,
        HnswIndex, BM25Index,
        Filter, PayloadFilter, FilterCondition,
        Error, Result,
        StorageManager,
        RestApi, GrpcApi,
    };
}

/// SIMD-optimized vector operations
pub mod simd {
    pub use distx_core::simd::{dot_product_simd, l2_distance_simd, norm_simd};
}
