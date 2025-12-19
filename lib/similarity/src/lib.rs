//! # DistX Similarity
//!
//! A schema-driven similarity engine for tabular rows.
//!
//! This crate provides structured similarity queries on top of DistX,
//! enabling similarity search over tabular data with explainable results.
//!
//! ## Features
//!
//! - **Similarity Schema**: Declarative schema defining which fields matter and their weights
//! - **Structured Embedding**: Automatic vector generation from payloads
//! - **Multi-field Reranking**: Accurate similarity scores combining multiple field types
//! - **Explainability**: Per-field contribution breakdown for transparency
//!
//! ## Example
//!
//! ```rust
//! use distx_similarity::{SimilaritySchema, FieldConfig, DistanceType, StructuredEmbedder, Reranker};
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! // Define a similarity schema
//! let mut fields = HashMap::new();
//! fields.insert("name".to_string(), FieldConfig::text(0.5));
//! fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
//! fields.insert("category".to_string(), FieldConfig::categorical(0.2));
//!
//! let mut schema = SimilaritySchema::new(fields);
//! schema.validate_and_normalize().unwrap();
//!
//! // Create embedder and embed a payload
//! let embedder = StructuredEmbedder::new(schema.clone());
//! let payload = json!({
//!     "name": "Prosciutto cotto",
//!     "price": 1.99,
//!     "category": "salumi"
//! });
//! let vector = embedder.embed(&payload);
//!
//! // Use reranker for accurate similarity scoring
//! let reranker = Reranker::new(schema);
//! // ... rerank ANN candidates with structured scoring
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Schema    │────>│  Embedder   │────>│   Vector    │
//! │  (fields)   │     │ (payload→v) │     │   Store     │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!       │                                        │
//!       │              ┌─────────────┐           │
//!       └─────────────>│  Reranker   │<──────────┘
//!                      │ (candidates)│
//!                      └─────────────┘
//!                             │
//!                      ┌─────────────┐
//!                      │  Explain    │
//!                      │  (results)  │
//!                      └─────────────┘
//! ```

pub mod schema;
pub mod distance;
pub mod embedder;
pub mod rerank;
pub mod explain;

// Re-export main types for convenience
pub use schema::{
    SimilaritySchema, 
    FieldConfig, 
    FieldType, 
    DistanceType, 
    EmbeddingType,
    SchemaError,
};
pub use embedder::{StructuredEmbedder, EmbedderBuilder, DEFAULT_TEXT_DIM, DEFAULT_CATEGORICAL_DIM};
pub use rerank::{Reranker, RankedResult};
pub use explain::{ExplainedResult, SimilarResponse, SimilarityStats, PointIdSer};
