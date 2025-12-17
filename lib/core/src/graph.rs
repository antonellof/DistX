// Simple graph support - nodes and edges
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type NodeId = u128;
pub type EdgeId = u128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
}

impl Node {
    #[inline]
    #[must_use]
    pub fn new(id: NodeId, label: String) -> Self {
        Self {
            id,
            label,
            properties: HashMap::new(),
        }
    }

    #[inline]
    #[must_use]
    pub fn with_property(mut self, key: String, value: serde_json::Value) -> Self {
        self.properties.insert(key, value);
        self
    }
}

impl Edge {
    #[inline]
    #[must_use]
    pub fn new(id: EdgeId, from: NodeId, to: NodeId, label: String) -> Self {
        Self {
            id,
            from,
            to,
            label,
            properties: HashMap::new(),
        }
    }

    #[inline]
    #[must_use]
    pub fn with_property(mut self, key: String, value: serde_json::Value) -> Self {
        self.properties.insert(key, value);
        self
    }
}

