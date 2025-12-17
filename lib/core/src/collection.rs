use crate::{Error, Point, Result, Vector, HnswIndex, BM25Index, Filter};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Configuration for a collection
#[derive(Debug, Clone)]
pub struct CollectionConfig {
    pub name: String,
    pub vector_dim: usize,
    pub distance: Distance,
    pub use_hnsw: bool,
    pub enable_bm25: bool,
}

impl Default for CollectionConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            vector_dim: 128,
            distance: Distance::Cosine,
            use_hnsw: true,
            enable_bm25: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distance {
    Cosine,
    Euclidean,
    Dot,
}

/// A collection of vectors with metadata
pub struct Collection {
    config: CollectionConfig,
    points: Arc<RwLock<HashMap<String, Point>>>,
    hnsw: Option<Arc<RwLock<HnswIndex>>>,
    bm25: Option<Arc<RwLock<BM25Index>>>,
    hnsw_built: Arc<RwLock<bool>>,
    hnsw_rebuilding: Arc<AtomicBool>,
    batch_mode: Arc<RwLock<bool>>,
    pending_points: Arc<RwLock<Vec<Point>>>,
}

impl Collection {
    pub fn new(config: CollectionConfig) -> Self {
        let hnsw = if config.use_hnsw {
            Some(Arc::new(RwLock::new(HnswIndex::new(16, 3))))
        } else {
            None
        };

        let bm25 = if config.enable_bm25 {
            Some(Arc::new(RwLock::new(BM25Index::new())))
        } else {
            None
        };

        Self {
            config,
            points: Arc::new(RwLock::new(HashMap::new())),
            hnsw,
            bm25,
            hnsw_built: Arc::new(RwLock::new(false)),
            hnsw_rebuilding: Arc::new(AtomicBool::new(false)),
            batch_mode: Arc::new(RwLock::new(false)),
            pending_points: Arc::new(RwLock::new(Vec::new())),
        }
    }

    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.name
    }

    #[inline]
    #[must_use]
    pub fn vector_dim(&self) -> usize {
        self.config.vector_dim
    }

    #[inline]
    #[must_use]
    pub fn distance(&self) -> Distance {
        self.config.distance
    }

    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.points.read().len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.read().is_empty()
    }

    /// Get all points in the collection
    pub fn get_all_points(&self) -> Vec<Point> {
        self.points.read().values().cloned().collect()
    }

    /// Insert or update a point
    pub fn upsert(&self, point: Point) -> Result<()> {
        if point.vector.dim() != self.config.vector_dim {
            return Err(Error::InvalidDimension {
                expected: self.config.vector_dim,
                actual: point.vector.dim(),
            });
        }

        let id_str = point.id.to_string();
        
        let in_batch = *self.batch_mode.read();
        if in_batch {
            self.points.write().insert(id_str.clone(), point.clone());
            self.pending_points.write().push(point);
            return Ok(());
        }
        
        if let Some(hnsw) = &self.hnsw {
            let built = *self.hnsw_built.read();
            if built {
                let mut normalized_point = point.clone();
                normalized_point.vector.normalize();
                
                let mut index = hnsw.write();
                index.insert(normalized_point);
            }
        }

        if let Some(bm25) = &self.bm25 {
            if let Some(payload) = &point.payload {
                if let Some(text) = payload.get("text").and_then(|v| v.as_str()) {
                    let mut index = bm25.write();
                    index.insert_doc(&id_str, text);
                }
            }
        }

        self.points.write().insert(id_str, point);
        Ok(())
    }

    /// Start batch insert mode
    pub fn start_batch(&self) {
        *self.batch_mode.write() = true;
        self.pending_points.write().clear();
    }

    /// End batch insert mode
    pub fn end_batch(&self) -> Result<()> {
        *self.batch_mode.write() = false;
        
        if let Some(hnsw) = &self.hnsw {
            let points = self.points.read();
            let point_count = points.len();
            
            const HNSW_REBUILD_THRESHOLD: usize = 10_000;
            
            if point_count > HNSW_REBUILD_THRESHOLD && !self.hnsw_rebuilding.load(Ordering::Acquire) {
                self.hnsw_rebuilding.store(true, Ordering::Release);
                let points_clone: Vec<Point> = points.values().cloned().collect();
                let hnsw_clone = hnsw.clone();
                let built_flag = self.hnsw_built.clone();
                let rebuilding_flag = self.hnsw_rebuilding.clone();
                
                let job = crate::background::HnswRebuildJob::new(
                    points_clone,
                    hnsw_clone,
                    built_flag,
                    rebuilding_flag,
                );
                crate::background::get_background_system().submit(Box::new(job));
            }
        }
        
        self.pending_points.write().clear();
        Ok(())
    }

    /// Batch insert multiple points
    pub fn batch_upsert(&self, points: Vec<Point>) -> Result<()> {
        self.start_batch();
        for point in points {
            self.upsert(point)?;
        }
        self.end_batch()?;
        Ok(())
    }

    /// Batch insert with optional pre-warming
    pub fn batch_upsert_with_prewarm(&self, points: Vec<Point>, prewarm: bool) -> Result<()> {
        self.batch_upsert(points)?;
        if prewarm {
            self.prewarm_index()?;
        }
        Ok(())
    }

    /// Get a point by ID
    #[inline]
    pub fn get(&self, id: &str) -> Option<Point> {
        self.points.read().get(id).cloned()
    }

    /// Delete a point by ID
    pub fn delete(&self, id: &str) -> Result<bool> {
        if let Some(hnsw) = &self.hnsw {
            let mut index = hnsw.write();
            index.remove(id);
        }

        if let Some(bm25) = &self.bm25 {
            let mut index = bm25.write();
            index.delete_doc(id);
        }

        let mut points = self.points.write();
        Ok(points.remove(id).is_some())
    }

    /// Pre-warm HNSW index
    pub fn prewarm_index(&self) -> Result<()> {
        if let Some(hnsw) = &self.hnsw {
            let mut built = self.hnsw_built.write();
            if !*built {
                let points = self.points.read();
                if !points.is_empty() {
                    let mut index = hnsw.write();
                    *index = HnswIndex::new(16, 3);
                    for point in points.values() {
                        index.insert(point.clone());
                    }
                    *built = true;
                }
            }
        }
        Ok(())
    }

    /// Fast brute-force search using SIMD - optimal for small datasets
    fn brute_force_search(&self, query: &Vector, limit: usize, filter: Option<&dyn Filter>) -> Vec<(Point, f32)> {
        let points = self.points.read();
        let query_slice = query.as_slice();
        
        // Pre-allocate results with capacity
        let mut results: Vec<(Point, f32)> = Vec::with_capacity(points.len().min(limit * 2));
        
        for point in points.values() {
            if let Some(f) = filter {
                if !f.matches(point) {
                    continue;
                }
            }
            
            // Use SIMD-optimized dot product for cosine similarity (vectors are normalized)
            let score = match self.config.distance {
                Distance::Cosine => {
                    crate::simd::dot_product_simd(query_slice, point.vector.as_slice())
                }
                Distance::Euclidean => {
                    -crate::simd::l2_distance_simd(query_slice, point.vector.as_slice())
                }
                Distance::Dot => {
                    crate::simd::dot_product_simd(query_slice, point.vector.as_slice())
                }
            };
            
            results.push((point.clone(), score));
        }
        
        // Use partial sort for efficiency when limit << len
        if results.len() > limit {
            results.select_nth_unstable_by(limit, |a, b| {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(limit);
        }
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Search for similar vectors
    /// Uses brute-force for small datasets (<1000), HNSW for larger ones
    pub fn search(
        &self,
        query: &Vector,
        limit: usize,
        filter: Option<&dyn Filter>,
    ) -> Vec<(Point, f32)> {
        let normalized_query = query.normalized();
        let point_count = self.points.read().len();
        
        // Use brute-force for small datasets - faster than HNSW overhead
        const BRUTE_FORCE_THRESHOLD: usize = 1000;
        if point_count < BRUTE_FORCE_THRESHOLD {
            return self.brute_force_search(&normalized_query, limit, filter);
        }
        
        if let Some(hnsw) = &self.hnsw {
            // Check if we need to build the index first
            {
                let mut built = self.hnsw_built.write();
                if !*built {
                    let points = self.points.read();
                    if !points.is_empty() {
                        let mut index = hnsw.write();
                        *index = HnswIndex::new(16, 3);
                        for point in points.values() {
                            index.insert(point.clone());
                        }
                        *built = true;
                    }
                }
            }
            
            // Use write lock for search (HNSW search is now mutable for performance)
            let mut index = hnsw.write();
            let mut results = index.search(&normalized_query, limit, None);
            
            if let Some(f) = filter {
                results.retain(|(point, _)| f.matches(point));
            }
            
            results
        } else {
            let points = self.points.read();
            let results: Vec<(Point, f32)> = points
                .values()
                .filter(|point| {
                    filter.map(|f| f.matches(point)).unwrap_or(true)
                })
                .map(|point| {
                    let score = match self.config.distance {
                        Distance::Cosine => point.vector.cosine_similarity(query),
                        Distance::Euclidean => -point.vector.l2_distance(query),
                        Distance::Dot => {
                            point.vector.as_slice()
                                .iter()
                                .zip(query.as_slice().iter())
                                .map(|(a, b)| a * b)
                                .sum()
                        }
                    };
                    (point.clone(), score)
                })
                .collect();

            let mut sorted = results;
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            sorted.truncate(limit);
            sorted
        }
    }

    /// BM25 text search
    pub fn search_text(&self, query: &str, limit: usize) -> Vec<(String, f32)> {
        if let Some(bm25) = &self.bm25 {
            let index = bm25.read();
            index.search(query, limit)
        } else {
            Vec::new()
        }
    }

    /// Get all points
    pub fn iter(&self) -> Vec<Point> {
        self.points.read().values().cloned().collect()
    }
}

