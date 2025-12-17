// Intensive benchmark suite for DistX
// Tests multiple scenarios: different sizes, dimensions, and configurations
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use distx_core::{Collection, CollectionConfig, Distance, Point, PointId, Vector};
use std::sync::Arc;
use rand::prelude::*;

fn generate_random_vector(dim: usize) -> Vector {
    let mut rng = rand::thread_rng();
    let data: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect();
    Vector::new(data)
}

fn generate_random_point(id: usize, dim: usize) -> Point {
    Point::new(
        PointId::Integer(id as u64),
        generate_random_vector(dim),
        Some(serde_json::json!({
            "id": id,
            "text": format!("document number {}", id)
        })),
    )
}

// Test different dataset sizes
fn benchmark_insert_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_sizes");
    group.sample_size(10); // Fewer samples for large datasets
    
    for size in [100, 500, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("distx_128d", size), size, |b, &size| {
            let config = CollectionConfig {
                name: "test".to_string(),
                vector_dim: 128,
                distance: Distance::Cosine,
                use_hnsw: true,
                enable_bm25: false,
            };
            let collection = Collection::new(config);
            
            b.iter(|| {
                for i in 0..size {
                    let point = generate_random_point(i, 128);
                    collection.upsert(point).unwrap();
                }
            });
        });
    }
    
    group.finish();
}

// Test different vector dimensions
fn benchmark_insert_dimensions(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_dimensions");
    
    for dim in [64, 128, 256, 512, 1024].iter() {
        group.bench_with_input(BenchmarkId::new("distx_1k", dim), dim, |b, &dim| {
            let config = CollectionConfig {
                name: "test".to_string(),
                vector_dim: dim,
                distance: Distance::Cosine,
                use_hnsw: true,
                enable_bm25: false,
            };
            let collection = Collection::new(config);
            
            b.iter(|| {
                for i in 0..1000 {
                    let point = generate_random_point(i, dim);
                    collection.upsert(point).unwrap();
                }
            });
        });
    }
    
    group.finish();
}

// Test different distance metrics
fn benchmark_distance_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_metrics");
    
    let distances = [
        ("Cosine", Distance::Cosine),
        ("Euclidean", Distance::Euclidean),
        ("Dot", Distance::Dot),
    ];
    
    for (name, distance) in distances.iter() {
        group.bench_function(format!("distx_{}", name), |b| {
            let config = CollectionConfig {
                name: "test".to_string(),
                vector_dim: 128,
                distance: *distance,
                use_hnsw: true,
                enable_bm25: false,
            };
            let collection = Collection::new(config);
            
            // Insert 1K points
            for i in 0..1000 {
                let point = generate_random_point(i, 128);
                collection.upsert(point).unwrap();
            }
            
            let query = generate_random_vector(128);
            
            b.iter(|| {
                let results = collection.search(black_box(&query), 10, None);
                black_box(results);
            });
        });
    }
    
    group.finish();
}

// Test search with different dataset sizes
fn benchmark_search_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_sizes");
    
    for size in [100, 1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::new("distx_hnsw", size), size, |b, &size| {
            let config = CollectionConfig {
                name: "test".to_string(),
                vector_dim: 128,
                distance: Distance::Cosine,
                use_hnsw: true,
                enable_bm25: false,
            };
            let collection = Collection::new(config);
            
            // Pre-populate
            for i in 0..size {
                let point = generate_random_point(i, 128);
                collection.upsert(point).unwrap();
            }
            
            let query = generate_random_vector(128);
            
            b.iter(|| {
                let results = collection.search(black_box(&query), 10, None);
                black_box(results);
            });
        });
    }
    
    group.finish();
}

// Test HNSW vs Linear search
fn benchmark_hnsw_vs_linear(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_vs_linear");
    
    // HNSW search
    group.bench_function("distx_hnsw", |b| {
        let config = CollectionConfig {
            name: "test".to_string(),
            vector_dim: 128,
            distance: Distance::Cosine,
            use_hnsw: true,
            enable_bm25: false,
        };
        let collection = Collection::new(config);
        
        for i in 0..10000 {
            let point = generate_random_point(i, 128);
            collection.upsert(point).unwrap();
        }
        
        let query = generate_random_vector(128);
        
        b.iter(|| {
            let results = collection.search(black_box(&query), 10, None);
            black_box(results);
        });
    });
    
    // Linear search
    group.bench_function("distx_linear", |b| {
        let config = CollectionConfig {
            name: "test".to_string(),
            vector_dim: 128,
            distance: Distance::Cosine,
            use_hnsw: false, // Linear search
            enable_bm25: false,
        };
        let collection = Collection::new(config);
        
        for i in 0..10000 {
            let point = generate_random_point(i, 128);
            collection.upsert(point).unwrap();
        }
        
        let query = generate_random_vector(128);
        
        b.iter(|| {
            let results = collection.search(black_box(&query), 10, None);
            black_box(results);
        });
    });
    
    group.finish();
}

// Test concurrent operations
fn benchmark_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent");
    
    for num_threads in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(BenchmarkId::new("distx_reads", num_threads), num_threads, |b, &num_threads| {
            let config = CollectionConfig {
                name: "test".to_string(),
                vector_dim: 128,
                distance: Distance::Cosine,
                use_hnsw: true,
                enable_bm25: false,
            };
            let collection = Arc::new(Collection::new(config));
            
            // Pre-populate
            for i in 0..10000 {
                let point = generate_random_point(i, 128);
                collection.upsert(point).unwrap();
            }
            
            let query = generate_random_vector(128);
            
            b.iter(|| {
                use std::thread;
                let handles: Vec<_> = (0..num_threads).map(|_| {
                    let coll = collection.clone();
                    let q = query.clone();
                    thread::spawn(move || {
                        coll.search(&q, 10, None)
                    })
                }).collect();
                
                for handle in handles {
                    black_box(handle.join().unwrap());
                }
            });
        });
    }
    
    group.finish();
}

// Test BM25 with different document counts
fn benchmark_bm25_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25_sizes");
    
    for size in [100, 1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::new("distx_bm25", size), size, |b, &size| {
            let config = CollectionConfig {
                name: "test".to_string(),
                vector_dim: 128,
                distance: Distance::Cosine,
                use_hnsw: false,
                enable_bm25: true,
            };
            let collection = Collection::new(config);
            
            // Pre-populate with text
            for i in 0..size {
                let mut point = generate_random_point(i, 128);
                point.payload = Some(serde_json::json!({
                    "text": format!("document about topic {} with some content and keywords", i)
                }));
                collection.upsert(point).unwrap();
            }
            
            b.iter(|| {
                let results = collection.search_text(black_box("topic content keywords"), 10);
                black_box(results);
            });
        });
    }
    
    group.finish();
}

// Test mixed workload (inserts + searches)
fn benchmark_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    
    group.bench_function("distx_mixed", |b| {
        let config = CollectionConfig {
            name: "test".to_string(),
            vector_dim: 128,
            distance: Distance::Cosine,
            use_hnsw: true,
            enable_bm25: false,
        };
        let collection = Collection::new(config);
        
        // Initial data
        for i in 0..5000 {
            let point = generate_random_point(i, 128);
            collection.upsert(point).unwrap();
        }
        
        let query = generate_random_vector(128);
        
        b.iter(|| {
            // Mix of inserts and searches
            for i in 5000..5100 {
                let point = generate_random_point(i, 128);
                collection.upsert(point).unwrap();
            }
            for _ in 0..100 {
                let results = collection.search(&query, 10, None);
                black_box(results);
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    intensive_benches,
    benchmark_insert_sizes,
    benchmark_insert_dimensions,
    benchmark_distance_metrics,
    benchmark_search_sizes,
    benchmark_hnsw_vs_linear,
    benchmark_concurrent_operations,
    benchmark_bm25_sizes,
    benchmark_mixed_workload
);
criterion_main!(intensive_benches);

