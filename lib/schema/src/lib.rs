//! # DistX Schema
//!
//! Structured similarity layer for vector databases.
//!
//! ## Overview
//!
//! DistX is a Qdrant-compatible vector database. The Schema module adds
//! structured similarity scoring on top of vector search results.
//!
//! **How it works:**
//! 1. Client generates embeddings for text fields (OpenAI, Cohere, etc.)
//! 2. Client sends vectors + payload to DistX
//! 3. DistX performs vector search (ANN)
//! 4. Schema reranks results using structured field comparisons
//!
//! ## Use Cases
//!
//! - **Product similarity**: Vector search finds semantically similar products,
//!   schema reranks by price proximity, category match, availability
//! - **Customer matching**: Vector search on profile text, schema scores by
//!   industry match, company size, location
//! - **Document search**: Vector search on content, schema weights by
//!   date recency, author, document type
//!
//! ## Schema Definition
//!
//! ```rust
//! use distx_schema::{SimilaritySchema, FieldConfig, DistanceType};
//! use std::collections::HashMap;
//!
//! let mut fields = HashMap::new();
//! // Text fields use vector search (no schema config needed for embedding)
//! fields.insert("name".to_string(), FieldConfig::text(0.3));
//! // Numeric fields use relative distance
//! fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
//! // Categorical fields use exact match
//! fields.insert("category".to_string(), FieldConfig::categorical(0.2));
//! fields.insert("availability".to_string(), FieldConfig::categorical(0.2));
//!
//! let schema = SimilaritySchema::new(fields);
//! ```
//!
//! ## Reranking Flow
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Client    │────>│   DistX     │────>│  Reranker   │
//! │ (embedding) │     │ (ANN search)│     │ (schema)    │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!                                                │
//!                                         ┌──────┴──────┐
//!                                         │  Explained  │
//!                                         │   Results   │
//!                                         └─────────────┘
//! ```

pub mod schema;
pub mod distance;
pub mod rerank;
pub mod explain;

// Re-export main types
pub use schema::{
    SimilaritySchema, 
    FieldConfig, 
    FieldType, 
    DistanceType, 
    SchemaError,
};
pub use rerank::{Reranker, RankedResult};
pub use explain::{ExplainedResult, SimilarResponse, SimilarityStats, PointIdSer};
