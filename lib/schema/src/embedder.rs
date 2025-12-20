//! Structured Embedder
//!
//! Converts payload fields into a single composite vector based on the similarity schema.
//! This enables vector search over structured/tabular data without requiring external embeddings.

use crate::schema::{SimilaritySchema, FieldType};
use crate::distance::{hash_text_to_vector, hash_categorical_to_vector};
use distx_core::Vector;
use serde_json::Value;

/// Default dimension for text embeddings
pub const DEFAULT_TEXT_DIM: usize = 64;

/// Default dimension for categorical embeddings
pub const DEFAULT_CATEGORICAL_DIM: usize = 64;

/// Structured embedder that converts payloads to vectors based on schema
#[derive(Debug, Clone)]
pub struct StructuredEmbedder {
    schema: SimilaritySchema,
    text_dim: usize,
    categorical_dim: usize,
}

impl StructuredEmbedder {
    /// Create a new structured embedder with the given schema
    pub fn new(schema: SimilaritySchema) -> Self {
        Self {
            schema,
            text_dim: DEFAULT_TEXT_DIM,
            categorical_dim: DEFAULT_CATEGORICAL_DIM,
        }
    }

    /// Create embedder with custom dimensions
    pub fn with_dimensions(schema: SimilaritySchema, text_dim: usize, categorical_dim: usize) -> Self {
        Self {
            schema,
            text_dim,
            categorical_dim,
        }
    }

    /// Get the total vector dimension for this embedder
    pub fn vector_dim(&self) -> usize {
        use crate::schema::FieldType;
        self.schema.fields.values().map(|config| {
            match config.field_type {
                FieldType::Text => self.text_dim,
                FieldType::Number => 1,
                FieldType::Categorical => self.categorical_dim,
                FieldType::Boolean => 1,
            }
        }).sum()
    }

    /// Get a reference to the schema
    pub fn schema(&self) -> &SimilaritySchema {
        &self.schema
    }

    /// Convert a payload to a composite vector
    /// 
    /// The vector is constructed by:
    /// 1. Iterating through schema fields in sorted order
    /// 2. Extracting values from the payload
    /// 3. Embedding each field according to its type
    /// 4. Applying weights
    /// 5. Concatenating all embeddings
    pub fn embed(&self, payload: &Value) -> Vector {
        let mut components: Vec<f32> = Vec::with_capacity(self.vector_dim());
        
        // Process fields in sorted order for consistency
        for field_name in self.schema.sorted_field_names() {
            let config = self.schema.get_field(field_name).unwrap();
            let weight_sqrt = config.weight.sqrt(); // Apply sqrt of weight to vector
            
            let field_vector = self.embed_field(payload, field_name, config);
            
            // Apply weight and extend
            components.extend(field_vector.iter().map(|v| v * weight_sqrt));
        }
        
        // Create and normalize the vector
        let mut vector = Vector::new(components);
        vector.normalize();
        vector
    }

    /// Embed a single field from the payload
    fn embed_field(&self, payload: &Value, field_name: &str, config: &crate::schema::FieldConfig) -> Vec<f32> {
        let value = payload.get(field_name);
        
        match config.field_type {
            FieldType::Text => self.embed_text(value),
            FieldType::Number => self.embed_number(value),
            FieldType::Categorical => self.embed_categorical(value),
            FieldType::Boolean => self.embed_boolean(value),
        }
    }

    /// Embed a text field
    fn embed_text(&self, value: Option<&Value>) -> Vec<f32> {
        match value.and_then(|v| v.as_str()) {
            Some(text) => hash_text_to_vector(text, self.text_dim),
            None => vec![0.0; self.text_dim], // Missing field gets zero vector
        }
    }

    /// Embed a number field
    fn embed_number(&self, value: Option<&Value>) -> Vec<f32> {
        match value {
            Some(v) => {
                let num = v.as_f64().unwrap_or(0.0);
                // Normalize using sigmoid-like function for unbounded values
                let normalized = num.tanh() as f32;
                vec![normalized]
            }
            None => vec![0.0], // Missing field
        }
    }

    /// Embed a categorical field
    fn embed_categorical(&self, value: Option<&Value>) -> Vec<f32> {
        match value.and_then(|v| v.as_str()) {
            Some(category) => hash_categorical_to_vector(category, self.categorical_dim),
            None => vec![0.0; self.categorical_dim],
        }
    }

    /// Embed a boolean field
    fn embed_boolean(&self, value: Option<&Value>) -> Vec<f32> {
        match value.and_then(|v| v.as_bool()) {
            Some(true) => vec![1.0],
            Some(false) => vec![-1.0],
            None => vec![0.0],
        }
    }
}

/// Builder for creating StructuredEmbedder with custom options
#[derive(Debug, Clone)]
pub struct EmbedderBuilder {
    schema: SimilaritySchema,
    text_dim: usize,
    categorical_dim: usize,
}

impl EmbedderBuilder {
    pub fn new(schema: SimilaritySchema) -> Self {
        Self {
            schema,
            text_dim: DEFAULT_TEXT_DIM,
            categorical_dim: DEFAULT_CATEGORICAL_DIM,
        }
    }

    pub fn text_dim(mut self, dim: usize) -> Self {
        self.text_dim = dim;
        self
    }

    pub fn categorical_dim(mut self, dim: usize) -> Self {
        self.categorical_dim = dim;
        self
    }

    pub fn build(self) -> StructuredEmbedder {
        StructuredEmbedder::with_dimensions(self.schema, self.text_dim, self.categorical_dim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldConfig, DistanceType};
    use std::collections::HashMap;
    use serde_json::json;

    fn create_test_schema() -> SimilaritySchema {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), FieldConfig::text(0.5));
        fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
        fields.insert("category".to_string(), FieldConfig::categorical(0.2));
        SimilaritySchema::new(fields)
    }

    #[test]
    fn test_embedder_creation() {
        let schema = create_test_schema();
        let embedder = StructuredEmbedder::new(schema);
        
        // Vector dim = text(64) + number(1) + categorical(64) = 129
        assert_eq!(embedder.vector_dim(), 64 + 1 + 64);
    }

    #[test]
    fn test_embed_complete_payload() {
        let schema = create_test_schema();
        let embedder = StructuredEmbedder::new(schema);
        
        let payload = json!({
            "name": "Prosciutto cotto",
            "price": 1.99,
            "category": "salumi"
        });
        
        let vector = embedder.embed(&payload);
        assert_eq!(vector.dim(), embedder.vector_dim());
        
        // Vector should be normalized
        let magnitude: f32 = vector.as_slice().iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_embed_partial_payload() {
        let schema = create_test_schema();
        let embedder = StructuredEmbedder::new(schema);
        
        let payload = json!({
            "name": "Prosciutto"
            // Missing price and category
        });
        
        let vector = embedder.embed(&payload);
        assert_eq!(vector.dim(), embedder.vector_dim());
    }

    #[test]
    fn test_same_payload_same_vector() {
        let schema = create_test_schema();
        let embedder = StructuredEmbedder::new(schema);
        
        let payload = json!({
            "name": "Product A",
            "price": 10.0,
            "category": "electronics"
        });
        
        let v1 = embedder.embed(&payload);
        let v2 = embedder.embed(&payload);
        
        // Same payload should produce identical vectors
        assert_eq!(v1.as_slice(), v2.as_slice());
    }

    #[test]
    fn test_similar_payloads_close_vectors() {
        let schema = create_test_schema();
        let embedder = StructuredEmbedder::new(schema);
        
        let payload1 = json!({
            "name": "Prosciutto cotto",
            "price": 1.99,
            "category": "salumi"
        });
        
        let payload2 = json!({
            "name": "Prosciutto crudo",
            "price": 2.49,
            "category": "salumi"
        });
        
        let v1 = embedder.embed(&payload1);
        let v2 = embedder.embed(&payload2);
        
        // Similar products should have high cosine similarity
        let similarity = v1.cosine_similarity(&v2);
        assert!(similarity > 0.5, "Expected similarity > 0.5, got {}", similarity);
    }

    #[test]
    fn test_different_payloads_different_vectors() {
        let schema = create_test_schema();
        let embedder = StructuredEmbedder::new(schema);
        
        let payload1 = json!({
            "name": "Apple iPhone",
            "price": 999.0,
            "category": "electronics"
        });
        
        let payload2 = json!({
            "name": "Organic Bananas",
            "price": 1.99,
            "category": "food"
        });
        
        let v1 = embedder.embed(&payload1);
        let v2 = embedder.embed(&payload2);
        
        // Very different products should have low similarity
        let similarity = v1.cosine_similarity(&v2);
        assert!(similarity < 0.5, "Expected similarity < 0.5, got {}", similarity);
    }

    #[test]
    fn test_builder_pattern() {
        let schema = create_test_schema();
        let embedder = EmbedderBuilder::new(schema)
            .text_dim(128)
            .categorical_dim(32)
            .build();
        
        // Vector dim = text(128) + number(1) + categorical(32) = 161
        assert_eq!(embedder.vector_dim(), 128 + 1 + 32);
    }
}
