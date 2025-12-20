//! # DistX Schema
//!
//! **DistX does not store vectors that represent objects.
//! It stores objects, and derives vectors from their structure.**
//!
//! This crate provides the Similarity Contract engine for DistX — a schema-driven
//! approach to structured similarity search over tabular data.
//!
//! ## The Similarity Contract
//!
//! The schema is not just configuration — it's a **contract** that governs:
//!
//! - **Ingest**: How objects are converted to vectors (deterministic, reproducible)
//! - **Query**: How similarity is computed across multiple field types
//! - **Ranking**: How results are scored and ordered
//! - **Explainability**: How each field contributes to the final score
//!
//! ## What This Is NOT
//!
//! DistX Schema is **not**:
//! - A neural embedding model (no ML, no training, no drift)
//! - A semantic LLM system (deterministic, not probabilistic)
//! - A black-box recommender (fully explainable scores)
//!
//! It **is**:
//! - A contract-based similarity engine for structured data
//! - Deterministic and reproducible (same input → same output, always)
//! - Designed for ERP, e-commerce, CRM, and tabular datasets
//!
//! ## Supported Field Types
//!
//! | Type | Distance Methods | Use Case |
//! |------|-----------------|----------|
//! | `text` | trigram hashing | Product names, descriptions |
//! | `number` | relative, absolute | Prices, quantities, scores |
//! | `categorical` | exact match hashing | Categories, brands, status |
//! | `boolean` | equality | Flags, availability |
//!
//! ## Example
//!
//! ```rust
//! use distx_schema::{SimilaritySchema, FieldConfig, DistanceType, StructuredEmbedder, Reranker};
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! // Define a Similarity Contract
//! let mut fields = HashMap::new();
//! fields.insert("name".to_string(), FieldConfig::text(0.5));
//! fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
//! fields.insert("category".to_string(), FieldConfig::categorical(0.2));
//!
//! let mut schema = SimilaritySchema::new(fields);
//! schema.validate_and_normalize().unwrap();
//!
//! // Derive vector from object structure
//! let embedder = StructuredEmbedder::new(schema.clone());
//! let payload = json!({
//!     "name": "Prosciutto cotto",
//!     "price": 1.99,
//!     "category": "salumi"
//! });
//! let vector = embedder.embed(&payload);
//!
//! // Rerank with explainable scoring
//! let reranker = Reranker::new(schema);
//! // ... rerank ANN candidates with per-field scoring
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    SIMILARITY CONTRACT                          │
//! │  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
//! │  │   Schema    │────>│  Embedder   │────>│   Vector    │       │
//! │  │ (contract)  │     │(deterministic)    │   Store     │       │
//! │  └─────────────┘     └─────────────┘     └─────────────┘       │
//! │        │                                        │               │
//! │        │              ┌─────────────┐           │               │
//! │        └─────────────>│  Reranker   │<──────────┘               │
//! │                       │(structured) │                           │
//! │                       └─────────────┘                           │
//! │                              │                                  │
//! │                       ┌─────────────┐                           │
//! │                       │  Explain    │                           │
//! │                       │(per-field)  │                           │
//! │                       └─────────────┘                           │
//! └─────────────────────────────────────────────────────────────────┘
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
