// Performance benchmarks comparing DistX with Redis and HelixDB
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

fn benchmark_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("distx", size), size, |b, &size| {
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

fn benchmark_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    
    // Setup: Insert 10k points
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
    
    group.bench_function("distx_hnsw_search", |b| {
        b.iter(|| {
            let results = collection.search(black_box(&query), 10, None);
            black_box(results);
        });
    });
    
    group.finish();
}

fn benchmark_bm25_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("bm25_search");
    
    // Setup: Insert 10k points with text
    let config = CollectionConfig {
        name: "test".to_string(),
        vector_dim: 128,
        distance: Distance::Cosine,
        use_hnsw: false,
        enable_bm25: true,
    };
    let collection = Collection::new(config);
    
    for i in 0..10000 {
        let mut point = generate_random_point(i, 128);
        point.payload = Some(serde_json::json!({
            "text": format!("document about topic {} with some content", i)
        }));
        collection.upsert(point).unwrap();
    }
    
    group.bench_function("distx_bm25", |b| {
        b.iter(|| {
            let results = collection.search_text(black_box("topic content"), 10);
            black_box(results);
        });
    });
    
    group.finish();
}

fn benchmark_concurrent_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_reads");
    
    let config = CollectionConfig {
        name: "test".to_string(),
        vector_dim: 128,
        distance: Distance::Cosine,
        use_hnsw: true,
        enable_bm25: false,
    };
    let collection = Arc::new(Collection::new(config));
    
    // Insert data
    for i in 0..1000 {
        let point = generate_random_point(i, 128);
        collection.upsert(point).unwrap();
    }
    
    let query = generate_random_vector(128);
    
    group.bench_function("distx_concurrent", |b| {
        b.iter(|| {
            use std::thread;
            let handles: Vec<_> = (0..10).map(|_| {
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
    
    group.finish();
}

criterion_group!(benches, benchmark_insert, benchmark_search, benchmark_bm25_search, benchmark_concurrent_reads);
criterion_main!(benches);

