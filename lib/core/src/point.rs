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

impl std::fmt::Display for PointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PointId::String(s) => write!(f, "{}", s),
            PointId::Uuid(u) => write!(f, "{}", u),
            PointId::Integer(i) => write!(f, "{}", i),
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
    #[inline]
    #[must_use]
    pub fn new(id: PointId, vector: Vector, payload: Option<serde_json::Value>) -> Self {
        Self {
            id,
            vector,
            payload,
        }
    }

    #[inline]
    #[must_use]
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }
}

