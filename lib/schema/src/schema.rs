//! Similarity Schema definitions
//!
//! Defines the schema for structured similarity reranking.
//! The schema specifies field types, distance metrics, and weights
//! used to rerank vector search results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Similarity schema for structured reranking
/// 
/// Defines how payload fields should be compared when reranking
/// vector search results. Each field has a type, distance metric,
/// and weight in the final score.
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

    /// Validate the schema and normalize weights to sum to 1.0
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
}

fn default_weight() -> f32 {
    1.0
}

impl FieldConfig {
    /// Create a text field config
    /// 
    /// Text fields are compared using trigram similarity during reranking.
    /// For semantic search, use client-side embeddings + vector search.
    pub fn text(weight: f32) -> Self {
        Self {
            field_type: FieldType::Text,
            distance: DistanceType::Semantic,
            weight,
        }
    }

    /// Create a number field config
    /// 
    /// Use Relative for comparing values of different magnitudes (e.g., prices).
    /// Use Absolute for values in the same range (e.g., ratings 1-5).
    pub fn number(weight: f32, distance: DistanceType) -> Self {
        Self {
            field_type: FieldType::Number,
            distance,
            weight,
        }
    }

    /// Create a categorical field config
    /// 
    /// Categorical fields use exact match by default.
    /// Use Overlap for multi-value categories.
    pub fn categorical(weight: f32) -> Self {
        Self {
            field_type: FieldType::Categorical,
            distance: DistanceType::Exact,
            weight,
        }
    }

    /// Create a boolean field config
    pub fn boolean(weight: f32) -> Self {
        Self {
            field_type: FieldType::Boolean,
            distance: DistanceType::Exact,
            weight,
        }
    }
}

/// Field type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// Text field - uses trigram or exact matching for reranking
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
    /// Trigram similarity for text fields
    #[default]
    Semantic,
    /// Absolute distance: exp(-|a - b| / scale)
    Absolute,
    /// Relative distance: 1 - |a - b| / max(|a|, |b|)
    Relative,
    /// Exact match: 1 if equal, 0 otherwise
    Exact,
    /// Overlap/Jaccard similarity for token sets
    Overlap,
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
