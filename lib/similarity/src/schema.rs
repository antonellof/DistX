//! Similarity Schema definitions
//!
//! Defines the declarative schema for structured similarity queries.
//! The schema specifies which fields matter for similarity, what type of
//! similarity to use per field, and the weight of each field.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Similarity schema version 1
/// 
/// A declarative schema that defines how to compute similarity for tabular rows.
/// Stored at collection level and used for both embedding and reranking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SimilaritySchema {
    /// Schema version for future compatibility
    #[serde(default = "default_version")]
    pub version: u32,
    
    /// Field configurations keyed by field name
    pub fields: HashMap<String, FieldConfig>,
}

fn default_version() -> u32 {
    1
}

impl SimilaritySchema {
    /// Create a new similarity schema with the given fields
    pub fn new(fields: HashMap<String, FieldConfig>) -> Self {
        Self {
            version: 1,
            fields,
        }
    }

    /// Validate the schema
    /// - Checks that weights are positive
    /// - Normalizes weights to sum to 1.0 if they don't
    pub fn validate_and_normalize(&mut self) -> Result<(), SchemaError> {
        if self.fields.is_empty() {
            return Err(SchemaError::EmptySchema);
        }

        // Check for negative weights
        for (name, config) in &self.fields {
            if config.weight < 0.0 {
                return Err(SchemaError::NegativeWeight(name.clone()));
            }
        }

        // Calculate sum of weights
        let weight_sum: f32 = self.fields.values().map(|f| f.weight).sum();
        
        if weight_sum <= 0.0 {
            return Err(SchemaError::ZeroTotalWeight);
        }

        // Normalize weights to sum to 1.0
        if (weight_sum - 1.0).abs() > 0.001 {
            for config in self.fields.values_mut() {
                config.weight /= weight_sum;
            }
        }

        Ok(())
    }

    /// Get the total number of dimensions needed for the composite vector
    /// Each field type contributes a fixed number of dimensions
    pub fn compute_vector_dim(&self, text_dim: usize) -> usize {
        self.fields.values().map(|config| {
            match config.field_type {
                FieldType::Text => text_dim,
                FieldType::Number => 1,
                FieldType::Categorical => 64, // Hash-based encoding
                FieldType::Boolean => 1,
            }
        }).sum()
    }

    /// Get field names in a deterministic order (sorted)
    pub fn sorted_field_names(&self) -> Vec<&String> {
        let mut names: Vec<_> = self.fields.keys().collect();
        names.sort();
        names
    }

    /// Get a field config by name
    pub fn get_field(&self, name: &str) -> Option<&FieldConfig> {
        self.fields.get(name)
    }
}

/// Configuration for a single field in the similarity schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldConfig {
    /// The type of the field (text, number, categorical, boolean)
    #[serde(rename = "type")]
    pub field_type: FieldType,
    
    /// The distance/similarity metric to use for this field
    #[serde(default)]
    pub distance: DistanceType,
    
    /// Weight of this field in the overall similarity score (0.0 to 1.0)
    #[serde(default = "default_weight")]
    pub weight: f32,
    
    /// Embedding type for text fields (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<EmbeddingType>,
}

fn default_weight() -> f32 {
    1.0
}

impl FieldConfig {
    /// Create a new text field configuration
    pub fn text(weight: f32) -> Self {
        Self {
            field_type: FieldType::Text,
            distance: DistanceType::Semantic,
            weight,
            embedding: Some(EmbeddingType::Semantic),
        }
    }

    /// Create a new number field configuration
    pub fn number(weight: f32, distance: DistanceType) -> Self {
        Self {
            field_type: FieldType::Number,
            distance,
            weight,
            embedding: None,
        }
    }

    /// Create a new categorical field configuration
    pub fn categorical(weight: f32) -> Self {
        Self {
            field_type: FieldType::Categorical,
            distance: DistanceType::Exact,
            weight,
            embedding: None,
        }
    }

    /// Create a new boolean field configuration
    pub fn boolean(weight: f32) -> Self {
        Self {
            field_type: FieldType::Boolean,
            distance: DistanceType::Exact,
            weight,
            embedding: None,
        }
    }
}

/// Field type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// Text field - uses semantic or exact matching
    Text,
    /// Numeric field - uses absolute or relative distance
    Number,
    /// Categorical field - uses exact or overlap matching
    Categorical,
    /// Boolean field - uses exact matching
    Boolean,
}

/// Distance/similarity type for fields
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DistanceType {
    /// Semantic similarity (for text) - uses embeddings
    #[default]
    Semantic,
    /// Absolute distance: 1 - |a - b| / max_range
    Absolute,
    /// Relative distance: 1 - |a - b| / max(|a|, |b|)
    Relative,
    /// Exact match: 1 if equal, 0 otherwise
    Exact,
    /// Overlap/Jaccard similarity for sets or tokens
    Overlap,
}

/// Embedding type for text fields
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingType {
    /// Semantic embeddings (hash-based in v1, can be extended to ML)
    Semantic,
    /// Exact string matching only
    Exact,
}

/// Errors that can occur during schema validation
#[derive(Debug, Clone, thiserror::Error)]
pub enum SchemaError {
    #[error("Schema cannot be empty")]
    EmptySchema,
    
    #[error("Field '{0}' has negative weight")]
    NegativeWeight(String),
    
    #[error("Total weight cannot be zero")]
    ZeroTotalWeight,
    
    #[error("Field '{0}' not found in schema")]
    FieldNotFound(String),
    
    #[error("Invalid field type for distance metric")]
    InvalidDistanceForType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), FieldConfig::text(0.5));
        fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
        fields.insert("category".to_string(), FieldConfig::categorical(0.2));

        let schema = SimilaritySchema::new(fields);
        assert_eq!(schema.version, 1);
        assert_eq!(schema.fields.len(), 3);
    }

    #[test]
    fn test_schema_normalization() {
        let mut fields = HashMap::new();
        fields.insert("a".to_string(), FieldConfig::text(2.0));
        fields.insert("b".to_string(), FieldConfig::number(2.0, DistanceType::Absolute));

        let mut schema = SimilaritySchema::new(fields);
        schema.validate_and_normalize().unwrap();

        let weight_sum: f32 = schema.fields.values().map(|f| f.weight).sum();
        assert!((weight_sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_empty_schema_error() {
        let mut schema = SimilaritySchema::new(HashMap::new());
        assert!(matches!(
            schema.validate_and_normalize(),
            Err(SchemaError::EmptySchema)
        ));
    }

    #[test]
    fn test_negative_weight_error() {
        let mut fields = HashMap::new();
        fields.insert("a".to_string(), FieldConfig {
            field_type: FieldType::Text,
            distance: DistanceType::Semantic,
            weight: -0.5,
            embedding: None,
        });

        let mut schema = SimilaritySchema::new(fields);
        assert!(matches!(
            schema.validate_and_normalize(),
            Err(SchemaError::NegativeWeight(_))
        ));
    }

    #[test]
    fn test_compute_vector_dim() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), FieldConfig::text(0.5));
        fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
        fields.insert("active".to_string(), FieldConfig::boolean(0.2));

        let schema = SimilaritySchema::new(fields);
        let dim = schema.compute_vector_dim(64); // 64-dim text embedding
        assert_eq!(dim, 64 + 1 + 1); // text + number + boolean
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), FieldConfig::text(0.5));
        fields.insert("price".to_string(), FieldConfig::number(0.5, DistanceType::Relative));

        let schema = SimilaritySchema::new(fields);
        let json = serde_json::to_string(&schema).unwrap();
        let parsed: SimilaritySchema = serde_json::from_str(&json).unwrap();
        
        assert_eq!(schema, parsed);
    }
}
