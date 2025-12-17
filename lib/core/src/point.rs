use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::vector::Vector;

/// A point in the vector space with optional payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub id: PointId,
    pub vector: Vector,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PointId {
    String(String),
    Uuid(Uuid),
    Integer(u64),
}

impl PointId {
    pub fn to_string(&self) -> String {
        match self {
            PointId::String(s) => s.clone(),
            PointId::Uuid(u) => u.to_string(),
            PointId::Integer(i) => i.to_string(),
        }
    }
}

impl From<String> for PointId {
    fn from(s: String) -> Self {
        PointId::String(s)
    }
}

impl From<u64> for PointId {
    fn from(i: u64) -> Self {
        PointId::Integer(i)
    }
}

impl From<Uuid> for PointId {
    fn from(u: Uuid) -> Self {
        PointId::Uuid(u)
    }
}

impl Point {
    pub fn new(id: PointId, vector: Vector, payload: Option<serde_json::Value>) -> Self {
        Self {
            id,
            vector,
            payload,
        }
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }
}

