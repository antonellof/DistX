// Simple payload filter implementation
use serde_json::Value;
use crate::Point;

pub trait Filter {
    fn matches(&self, point: &Point) -> bool;
}

pub struct PayloadFilter {
    condition: FilterCondition,
}

#[derive(Debug, Clone)]
pub enum FilterCondition {
    Equals { field: String, value: Value },
    NotEquals { field: String, value: Value },
    GreaterThan { field: String, value: f64 },
    LessThan { field: String, value: f64 },
    GreaterEqual { field: String, value: f64 },
    LessEqual { field: String, value: f64 },
    Contains { field: String, value: String },
    And(Vec<FilterCondition>),
    Or(Vec<FilterCondition>),
    Not(Box<FilterCondition>),
}

impl PayloadFilter {
    pub fn new(condition: FilterCondition) -> Self {
        Self { condition }
    }

    fn get_field_value<'a>(point: &'a Point, field: &str) -> Option<&'a Value> {
        point.payload.as_ref().and_then(|p| {
            if field.starts_with('.') {
                let field = &field[1..];
                p.get(field)
            } else {
                p.get(field)
            }
        })
    }

    fn matches_condition(condition: &FilterCondition, point: &Point) -> bool {
        match condition {
            FilterCondition::Equals { field, value } => {
                Self::get_field_value(point, field)
                    .map(|v| v == value)
                    .unwrap_or(false)
            }
            FilterCondition::NotEquals { field, value } => {
                Self::get_field_value(point, field)
                    .map(|v| v != value)
                    .unwrap_or(true)
            }
            FilterCondition::GreaterThan { field, value } => {
                Self::get_field_value(point, field)
                    .and_then(|v: &serde_json::Value| v.as_f64())
                    .map(|v| v > *value)
                    .unwrap_or(false)
            }
            FilterCondition::LessThan { field, value } => {
                Self::get_field_value(point, field)
                    .and_then(|v: &serde_json::Value| v.as_f64())
                    .map(|v| v < *value)
                    .unwrap_or(false)
            }
            FilterCondition::GreaterEqual { field, value } => {
                Self::get_field_value(point, field)
                    .and_then(|v: &serde_json::Value| v.as_f64())
                    .map(|v| v >= *value)
                    .unwrap_or(false)
            }
            FilterCondition::LessEqual { field, value } => {
                Self::get_field_value(point, field)
                    .and_then(|v: &serde_json::Value| v.as_f64())
                    .map(|v| v <= *value)
                    .unwrap_or(false)
            }
            FilterCondition::Contains { field, value } => {
                Self::get_field_value(point, field)
                    .and_then(|v: &serde_json::Value| v.as_str())
                    .map(|v: &str| v.contains(value))
                    .unwrap_or(false)
            }
            FilterCondition::And(conditions) => {
                conditions.iter().all(|c| Self::matches_condition(c, point))
            }
            FilterCondition::Or(conditions) => {
                conditions.iter().any(|c| Self::matches_condition(c, point))
            }
            FilterCondition::Not(condition) => {
                !Self::matches_condition(condition, point)
            }
        }
    }
}

impl Filter for PayloadFilter {
    fn matches(&self, point: &Point) -> bool {
        Self::matches_condition(&self.condition, point)
    }
}

