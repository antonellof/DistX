//! Explainability for structured similarity results
//!
//! Provides output structures that explain how similarity scores were computed,
//! showing per-field contributions for transparency.

use crate::rerank::RankedResult;
use distx_core::PointId;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// An explained similarity result with per-field score breakdown
#[derive(Debug, Clone, Serialize)]
pub struct ExplainedResult {
    /// Point ID
    pub id: PointIdSer,
    /// Overall weighted similarity score
    pub score: f32,
    /// Point payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    /// Per-field score contributions (already weighted)
    pub explain: HashMap<String, f32>,
}

/// Serializable wrapper for PointId
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PointIdSer {
    String(String),
    Integer(u64),
}

impl From<&PointId> for PointIdSer {
    fn from(id: &PointId) -> Self {
        match id {
            PointId::String(s) => PointIdSer::String(s.clone()),
            PointId::Uuid(u) => PointIdSer::String(u.to_string()),
            PointId::Integer(i) => PointIdSer::Integer(*i),
        }
    }
}

impl ExplainedResult {
    /// Create an explained result from a ranked result
    pub fn from_ranked(ranked: RankedResult, include_payload: bool) -> Self {
        Self {
            id: PointIdSer::from(&ranked.point.id),
            score: ranked.score,
            payload: if include_payload { ranked.point.payload.clone() } else { None },
            explain: ranked.field_scores,
        }
    }

    /// Create a list of explained results from ranked results
    pub fn from_ranked_list(ranked_list: Vec<RankedResult>, include_payload: bool) -> Vec<Self> {
        ranked_list
            .into_iter()
            .map(|r| Self::from_ranked(r, include_payload))
            .collect()
    }
}

/// Response structure for the /similar endpoint
#[derive(Debug, Clone, Serialize)]
pub struct SimilarResponse {
    /// List of similar items with explanations
    pub result: Vec<ExplainedResult>,
}

impl SimilarResponse {
    /// Create a new similar response
    pub fn new(results: Vec<ExplainedResult>) -> Self {
        Self { result: results }
    }

    /// Create from ranked results
    pub fn from_ranked(ranked_list: Vec<RankedResult>, include_payload: bool) -> Self {
        Self {
            result: ExplainedResult::from_ranked_list(ranked_list, include_payload),
        }
    }
}

/// Summary statistics for a similarity query
#[derive(Debug, Clone, Serialize)]
pub struct SimilarityStats {
    /// Number of candidates considered
    pub candidates_count: usize,
    /// Number of results returned
    pub results_count: usize,
    /// Average score of results
    pub avg_score: f32,
    /// Score of best result
    pub best_score: f32,
    /// Field that contributed most to best result
    pub top_contributing_field: Option<String>,
}

impl SimilarityStats {
    /// Compute stats from ranked results
    pub fn compute(results: &[RankedResult], candidates_count: usize) -> Self {
        if results.is_empty() {
            return Self {
                candidates_count,
                results_count: 0,
                avg_score: 0.0,
                best_score: 0.0,
                top_contributing_field: None,
            };
        }

        let scores: Vec<f32> = results.iter().map(|r| r.score).collect();
        let avg_score = scores.iter().sum::<f32>() / scores.len() as f32;
        let best_score = scores[0]; // Results are sorted

        // Find top contributing field in best result
        let top_contributing_field = results[0]
            .field_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| name.clone());

        Self {
            candidates_count,
            results_count: results.len(),
            avg_score,
            best_score,
            top_contributing_field,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use distx_core::{Point, Vector};
    use serde_json::json;

    fn create_test_ranked_result(id: &str, score: f32) -> RankedResult {
        let mut field_scores = HashMap::new();
        field_scores.insert("name".to_string(), 0.4);
        field_scores.insert("price".to_string(), 0.25);
        field_scores.insert("category".to_string(), 0.15);

        RankedResult {
            point: Point::new(
                PointId::String(id.to_string()),
                Vector::new(vec![0.0; 10]),
                Some(json!({"name": "Test", "price": 1.99})),
            ),
            score,
            field_scores,
        }
    }

    #[test]
    fn test_explained_result_creation() {
        let ranked = create_test_ranked_result("1", 0.85);
        let explained = ExplainedResult::from_ranked(ranked, true);

        assert!(matches!(explained.id, PointIdSer::String(ref s) if s == "1"));
        assert_eq!(explained.score, 0.85);
        assert!(explained.payload.is_some());
        assert_eq!(explained.explain.len(), 3);
    }

    #[test]
    fn test_explained_result_without_payload() {
        let ranked = create_test_ranked_result("1", 0.85);
        let explained = ExplainedResult::from_ranked(ranked, false);

        assert!(explained.payload.is_none());
    }

    #[test]
    fn test_similar_response_serialization() {
        let ranked = vec![
            create_test_ranked_result("1", 0.95),
            create_test_ranked_result("2", 0.85),
        ];

        let response = SimilarResponse::from_ranked(ranked, true);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"result\""));
        assert!(json.contains("\"score\""));
        assert!(json.contains("\"explain\""));
    }

    #[test]
    fn test_similarity_stats() {
        let results = vec![
            create_test_ranked_result("1", 0.95),
            create_test_ranked_result("2", 0.85),
            create_test_ranked_result("3", 0.75),
        ];

        let stats = SimilarityStats::compute(&results, 10);

        assert_eq!(stats.candidates_count, 10);
        assert_eq!(stats.results_count, 3);
        assert_eq!(stats.best_score, 0.95);
        assert!((stats.avg_score - 0.85).abs() < 0.01);
        assert_eq!(stats.top_contributing_field, Some("name".to_string()));
    }

    #[test]
    fn test_empty_stats() {
        let stats = SimilarityStats::compute(&[], 5);

        assert_eq!(stats.candidates_count, 5);
        assert_eq!(stats.results_count, 0);
        assert_eq!(stats.best_score, 0.0);
    }

    #[test]
    fn test_point_id_ser_variants() {
        let string_id = PointIdSer::from(&PointId::String("test".to_string()));
        let int_id = PointIdSer::from(&PointId::Integer(42));

        let string_json = serde_json::to_string(&string_id).unwrap();
        let int_json = serde_json::to_string(&int_id).unwrap();

        assert_eq!(string_json, "\"test\"");
        assert_eq!(int_json, "42");
    }
}
