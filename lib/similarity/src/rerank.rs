//! Reranker for structured similarity
//!
//! Performs multi-field scoring on ANN candidates to produce
//! accurate similarity scores with per-field explanations.

use crate::schema::{SimilaritySchema, FieldType, DistanceType};
use crate::distance::{text_similarity, number_similarity, categorical_similarity, boolean_similarity};
use distx_core::Point;
use serde_json::Value;
use std::collections::HashMap;

/// Result of reranking with per-field scores
#[derive(Debug, Clone)]
pub struct RankedResult {
    /// The original point
    pub point: Point,
    /// Overall weighted similarity score
    pub score: f32,
    /// Per-field similarity scores (already weighted)
    pub field_scores: HashMap<String, f32>,
}

impl RankedResult {
    /// Get the point ID as a string
    pub fn id_string(&self) -> String {
        self.point.id.to_string()
    }
}

/// Reranker that computes structured similarity scores
#[derive(Debug, Clone)]
pub struct Reranker {
    schema: SimilaritySchema,
}

impl Reranker {
    /// Create a new reranker with the given schema
    pub fn new(schema: SimilaritySchema) -> Self {
        Self { schema }
    }

    /// Get a reference to the schema
    pub fn schema(&self) -> &SimilaritySchema {
        &self.schema
    }

    /// Rerank candidates based on structured similarity to the query
    ///
    /// # Arguments
    /// * `query_payload` - The query payload (example record)
    /// * `candidates` - ANN search candidates with their vector scores
    ///
    /// # Returns
    /// Reranked results sorted by structured similarity score
    pub fn rerank(
        &self,
        query_payload: &Value,
        candidates: Vec<(Point, f32)>,
    ) -> Vec<RankedResult> {
        let mut results: Vec<RankedResult> = candidates
            .into_iter()
            .map(|(point, _ann_score)| {
                let (score, field_scores) = self.compute_structured_score(
                    query_payload,
                    &point.payload,
                );
                RankedResult { point, score, field_scores }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Compute structured similarity score between query and candidate
    ///
    /// Returns (total_score, field_scores) where:
    /// - total_score is the weighted sum of all field similarities
    /// - field_scores maps field names to their weighted contributions
    pub fn compute_structured_score(
        &self,
        query: &Value,
        candidate: &Option<Value>,
    ) -> (f32, HashMap<String, f32>) {
        let mut field_scores = HashMap::new();
        let mut total_score = 0.0f32;

        let candidate_payload = match candidate {
            Some(v) => v,
            None => {
                // No payload, all fields get zero score
                for field_name in self.schema.fields.keys() {
                    field_scores.insert(field_name.clone(), 0.0);
                }
                return (0.0, field_scores);
            }
        };

        for (field_name, config) in &self.schema.fields {
            let query_value = query.get(field_name);
            let candidate_value = candidate_payload.get(field_name);

            // Skip if query doesn't have this field (partial query)
            let similarity = if query_value.is_none() {
                // For missing query fields, assume neutral similarity
                0.5
            } else {
                self.compute_field_similarity(
                    query_value,
                    candidate_value,
                    config.field_type,
                    config.distance,
                )
            };

            // Apply weight to get contribution
            let weighted_score = similarity * config.weight;
            field_scores.insert(field_name.clone(), weighted_score);
            total_score += weighted_score;
        }

        (total_score, field_scores)
    }

    /// Compute similarity for a single field
    fn compute_field_similarity(
        &self,
        query_value: Option<&Value>,
        candidate_value: Option<&Value>,
        field_type: FieldType,
        distance: DistanceType,
    ) -> f32 {
        match (query_value, candidate_value) {
            (None, _) | (_, None) => 0.0,
            (Some(q), Some(c)) => {
                match field_type {
                    FieldType::Text => {
                        let q_str = q.as_str().unwrap_or("");
                        let c_str = c.as_str().unwrap_or("");
                        text_similarity(q_str, c_str, distance)
                    }
                    FieldType::Number => {
                        let q_num = q.as_f64().unwrap_or(0.0);
                        let c_num = c.as_f64().unwrap_or(0.0);
                        number_similarity(q_num, c_num, distance)
                    }
                    FieldType::Categorical => {
                        let q_str = q.as_str().unwrap_or("");
                        let c_str = c.as_str().unwrap_or("");
                        categorical_similarity(q_str, c_str, distance)
                    }
                    FieldType::Boolean => {
                        let q_bool = q.as_bool().unwrap_or(false);
                        let c_bool = c.as_bool().unwrap_or(false);
                        boolean_similarity(q_bool, c_bool)
                    }
                }
            }
        }
    }

    /// Create a new reranker with custom weight overrides
    ///
    /// Weight overrides replace schema weights for specific fields.
    /// Fields not in the overrides keep their original schema weights.
    /// After applying overrides, weights are re-normalized to sum to 1.0.
    ///
    /// # Example
    /// ```ignore
    /// let reranker = Reranker::new(schema);
    /// let custom = reranker.with_weights(&HashMap::from([
    ///     ("price".to_string(), 0.6),
    ///     ("name".to_string(), 0.2),
    /// ]));
    /// ```
    pub fn with_weights(&self, weight_overrides: &HashMap<String, f32>) -> Reranker {
        let mut modified_schema = self.schema.clone();
        
        // Apply weight overrides
        for (field_name, new_weight) in weight_overrides {
            if let Some(field) = modified_schema.fields.get_mut(field_name) {
                field.weight = new_weight.max(0.0);
            }
            // Silently ignore fields that don't exist in schema
        }
        
        // Re-normalize weights to sum to 1.0
        let _ = modified_schema.validate_and_normalize();
        
        Reranker::new(modified_schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FieldConfig;
    use distx_core::{PointId, Vector};
    use serde_json::json;

    fn create_test_schema() -> SimilaritySchema {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), FieldConfig::text(0.5));
        fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
        fields.insert("category".to_string(), FieldConfig::categorical(0.2));
        SimilaritySchema::new(fields)
    }

    fn create_test_point(id: &str, name: &str, price: f64, category: &str) -> Point {
        Point::new(
            PointId::String(id.to_string()),
            Vector::new(vec![0.0; 10]), // Dummy vector
            Some(json!({
                "name": name,
                "price": price,
                "category": category
            })),
        )
    }

    #[test]
    fn test_reranker_creation() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);
        assert_eq!(reranker.schema().fields.len(), 3);
    }

    #[test]
    fn test_identical_payloads_high_score() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);

        let query = json!({
            "name": "Prosciutto cotto",
            "price": 1.99,
            "category": "salumi"
        });

        let candidate = Some(json!({
            "name": "Prosciutto cotto",
            "price": 1.99,
            "category": "salumi"
        }));

        let (score, field_scores) = reranker.compute_structured_score(&query, &candidate);
        
        // Identical payloads should get perfect score
        assert!((score - 1.0).abs() < 0.01, "Expected ~1.0, got {}", score);
        assert_eq!(field_scores.len(), 3);
    }

    #[test]
    fn test_similar_payloads_good_score() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);

        let query = json!({
            "name": "Prosciutto cotto",
            "price": 1.99,
            "category": "salumi"
        });

        let candidate = Some(json!({
            "name": "Prosciutto crudo",
            "price": 2.49,
            "category": "salumi"
        }));

        let (score, _) = reranker.compute_structured_score(&query, &candidate);
        
        // Similar payloads should get good score
        assert!(score > 0.5, "Expected > 0.5, got {}", score);
    }

    #[test]
    fn test_rerank_sorting() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);

        let query = json!({
            "name": "Prosciutto cotto",
            "price": 1.99,
            "category": "salumi"
        });

        let candidates = vec![
            (create_test_point("1", "Banana", 0.99, "fruit"), 0.5),
            (create_test_point("2", "Prosciutto crudo", 2.49, "salumi"), 0.8),
            (create_test_point("3", "Prosciutto cotto", 1.99, "salumi"), 0.9),
        ];

        let results = reranker.rerank(&query, candidates);

        // Identical product should be first
        assert_eq!(results[0].id_string(), "3");
        // Similar product should be second
        assert_eq!(results[1].id_string(), "2");
        // Different product should be last
        assert_eq!(results[2].id_string(), "1");
    }

    #[test]
    fn test_partial_query() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);

        let query = json!({
            "name": "Prosciutto"
            // Missing price and category
        });

        let candidate = Some(json!({
            "name": "Prosciutto cotto",
            "price": 1.99,
            "category": "salumi"
        }));

        let (score, field_scores) = reranker.compute_structured_score(&query, &candidate);
        
        // Should still compute a score
        assert!(score > 0.0);
        assert_eq!(field_scores.len(), 3);
    }

    #[test]
    fn test_with_weights_override() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);
        
        // Override price to be much higher
        let overrides = HashMap::from([
            ("price".to_string(), 0.8),
            ("name".to_string(), 0.1),
        ]);
        let modified = reranker.with_weights(&overrides);
        
        // After normalization, price should have higher relative weight
        let original_price_weight = reranker.schema().fields.get("price").unwrap().weight;
        let modified_price_weight = modified.schema().fields.get("price").unwrap().weight;
        
        assert!(modified_price_weight > original_price_weight,
            "Expected modified price weight {} > original {}", 
            modified_price_weight, original_price_weight);
    }

    #[test]
    fn test_with_weights_unknown_field_ignored() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);
        
        // Override with unknown field - should be silently ignored
        let overrides = HashMap::from([
            ("unknown_field".to_string(), 0.5),
            ("price".to_string(), 0.6),
        ]);
        let modified = reranker.with_weights(&overrides);
        
        // Should still have 3 fields
        assert_eq!(modified.schema().fields.len(), 3);
        // Price should be updated
        assert!(modified.schema().fields.get("price").unwrap().weight > 0.0);
    }

    #[test]
    fn test_with_weights_normalization() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);
        
        // Set very high weights
        let overrides = HashMap::from([
            ("price".to_string(), 10.0),
            ("name".to_string(), 10.0),
            ("category".to_string(), 10.0),
        ]);
        let modified = reranker.with_weights(&overrides);
        
        // After normalization, weights should sum to ~1.0
        let total: f32 = modified.schema().fields.values()
            .map(|f| f.weight)
            .sum();
        
        assert!((total - 1.0).abs() < 0.01, 
            "Expected weights to sum to 1.0, got {}", total);
    }

    #[test]
    fn test_with_weights_affects_scoring() {
        let schema = create_test_schema();
        let reranker = Reranker::new(schema);

        // Query where price matches but name doesn't
        let query = json!({
            "name": "Apple",
            "price": 1.99,
            "category": "fruit"
        });

        let candidates = vec![
            // Same price, different name
            (create_test_point("1", "Orange", 1.99, "fruit"), 0.5),
            // Similar name, different price
            (create_test_point("2", "Apple Juice", 5.99, "beverages"), 0.5),
        ];

        // With default weights (name=0.5, price=0.3)
        let results_default = reranker.rerank(&query, candidates.clone());

        // With price-focused weights
        let overrides = HashMap::from([
            ("price".to_string(), 0.8),
            ("name".to_string(), 0.1),
        ]);
        let price_focused = reranker.with_weights(&overrides);
        let results_price = price_focused.rerank(&query, candidates);

        // With default weights, Apple Juice (similar name) might rank higher
        // With price weights, Orange (same price) should rank higher
        assert_eq!(results_price[0].id_string(), "1", 
            "Price-focused should prefer same-price item");
    }
}
