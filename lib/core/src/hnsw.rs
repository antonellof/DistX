use crate::{Point, Vector};
use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone)]
struct HnswNode {
    point: Point,
    layers: Vec<Vec<usize>>,
}

/// HNSW index for approximate nearest neighbor search
pub struct HnswIndex {
    nodes: Vec<HnswNode>,
    point_id_to_index: Arc<RwLock<HashMap<String, usize>>>,
    max_connections: usize,
    max_layers: usize,
    ef_construction: usize,
}

impl HnswIndex {
    pub fn new(max_connections: usize, max_layers: usize) -> Self {
        Self {
            nodes: Vec::new(),
            point_id_to_index: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            max_layers,
            ef_construction: 200,
        }
    }

    /// Select layer using exponential decay
    fn select_layer(&self) -> usize {
        let mut layer = 0;
        while layer < self.max_layers - 1 && rand::random::<f32>() < 0.5 {
            layer += 1;
        }
        layer
    }

    /// Find nearest neighbors using greedy search
    fn search_layer(
        &self,
        query: &Vector,
        entry_point: usize,
        ef: usize,
        layer: usize,
    ) -> Vec<(usize, f32)> {
        #[derive(PartialEq)]
        struct Candidate {
            idx: usize,
            dist: f32,
        }
        
        impl Eq for Candidate {}
        
        impl Ord for Candidate {
            fn cmp(&self, other: &Self) -> Ordering {
                other.dist.partial_cmp(&self.dist).unwrap_or(Ordering::Equal)
            }
        }
        
        impl PartialOrd for Candidate {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut candidates = BinaryHeap::new();
        let mut visited = HashSet::new();
        let mut results: Vec<(usize, f32)> = Vec::new();

        let entry_dist = self.distance(query, entry_point);
        candidates.push(Candidate { idx: entry_point, dist: entry_dist });
        visited.insert(entry_point);

        while let Some(Candidate { idx: current_idx, dist: current_dist }) = candidates.pop() {
            if results.len() >= ef {
                if let Some(&(_, best_dist)) = results.last() {
                    if current_dist > best_dist {
                        break;
                    }
                }
            }

            let insert_pos = results.binary_search_by(|(_, d)| {
                d.partial_cmp(&current_dist).unwrap_or(Ordering::Equal)
            }).unwrap_or_else(|e| e);
            results.insert(insert_pos, (current_idx, current_dist));
            
            if results.len() > ef {
                results.pop();
            }

            if layer < self.nodes[current_idx].layers.len() {
                for &neighbor_idx in &self.nodes[current_idx].layers[layer] {
                    if !visited.contains(&neighbor_idx) {
                        visited.insert(neighbor_idx);
                        let dist = self.distance(query, neighbor_idx);
                        candidates.push(Candidate { idx: neighbor_idx, dist });
                    }
                }
            }
        }

        results
    }

    /// Distance between query vector and node
    fn distance(&self, query: &Vector, node_idx: usize) -> f32 {
        let node = &self.nodes[node_idx];
        let dot = crate::simd::dot_product_simd(query.as_slice(), node.point.vector.as_slice());
        1.0 - dot
    }

    /// Insert a new point into the HNSW graph
    pub fn insert(&mut self, point: Point) {
        let id_str = point.id.to_string();
        let layer = self.select_layer();

        let mut node = HnswNode {
            point: point.clone(),
            layers: vec![Vec::new(); layer + 1],
        };

        if self.nodes.is_empty() {
            self.nodes.push(node);
            let node_idx = self.nodes.len() - 1;
            self.point_id_to_index.write().insert(id_str, node_idx);
            return;
        }

        let entry_point = 0;
        let mut current_layer = self.max_layers - 1;

        while current_layer > layer {
            let neighbors = self.search_layer(&point.vector, entry_point, 1, current_layer);
            if neighbors.first().is_some() {
                current_layer -= 1;
            } else {
                break;
            }
        }

        let mut candidates = if !self.nodes.is_empty() {
            self.search_layer(&point.vector, entry_point, self.ef_construction, layer)
        } else {
            Vec::new()
        };
        
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let neighbors: Vec<usize> = candidates
            .iter()
            .take(self.max_connections)
            .map(|(idx, _)| *idx)
            .collect();

        node.layers[layer] = neighbors.clone();

        self.nodes.push(node);
        let node_idx = self.nodes.len() - 1;
        self.point_id_to_index.write().insert(id_str, node_idx);

        for &neighbor_idx in &neighbors {
            if neighbor_idx < self.nodes.len() && layer < self.nodes[neighbor_idx].layers.len() {
                self.nodes[neighbor_idx].layers[layer].push(node_idx);
                if self.nodes[neighbor_idx].layers[layer].len() > self.max_connections * 2 {
                    let neighbor_point = self.nodes[neighbor_idx].point.vector.clone();
                    let mut layer_connections = self.nodes[neighbor_idx].layers[layer].clone();
                    
                    layer_connections.sort_by(|&a, &b| {
                        if a < self.nodes.len() && b < self.nodes.len() {
                            let dist_a = neighbor_point.l2_distance(&self.nodes[a].point.vector);
                            let dist_b = neighbor_point.l2_distance(&self.nodes[b].point.vector);
                            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    });
                    layer_connections.truncate(self.max_connections * 2);
                    self.nodes[neighbor_idx].layers[layer] = layer_connections;
                }
            }
        }
    }

    /// Search for k nearest neighbors
    pub fn search(&self, query: &Vector, k: usize, ef: Option<usize>) -> Vec<(Point, f32)> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        let ef = ef.unwrap_or(k * 10).max(k);
        let entry_point = 0;
        let mut current_layer = self.max_layers - 1;

        while current_layer > 0 {
            let neighbors = self.search_layer(query, entry_point, 1, current_layer);
            if neighbors.first().is_some() {
                current_layer -= 1;
            } else {
                break;
            }
        }

        let results = self.search_layer(query, entry_point, ef, 0);
        
        let points: Vec<(Point, f32)> = results
            .iter()
            .take(k)
            .map(|(idx, dist)| {
                let node = &self.nodes[*idx];
                let similarity = 1.0 - dist;
                (node.point.clone(), similarity)
            })
            .collect();

        points
    }

    pub fn remove(&mut self, point_id: &str) -> bool {
        let mut index_map = self.point_id_to_index.write();
        if let Some(index) = index_map.remove(point_id) {
            self.nodes.remove(index);
            
            index_map.clear();
            for (i, node) in self.nodes.iter().enumerate() {
                index_map.insert(node.point.id.to_string(), i);
            }
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_insert_search() {
        let mut index = HnswIndex::new(16, 3);
        
        // Insert some test vectors
        for i in 0..10 {
            let vector = Vector::new(vec![i as f32, i as f32, i as f32]);
            let point = Point::new(crate::PointId::Integer(i), vector, None);
            index.insert(point);
        }

        // Search
        let query = Vector::new(vec![5.0, 5.0, 5.0]);
        let results = index.search(&query, 3, None);
        assert!(!results.is_empty());
    }
}
