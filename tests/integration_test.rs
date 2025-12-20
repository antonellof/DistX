// Integration tests for DistX
use distx_core::{Collection, CollectionConfig, Distance, Point, PointId, Vector};
use distx_storage::StorageManager;
use distx_schema::{SimilaritySchema, FieldConfig, DistanceType, Reranker};
use std::collections::HashMap;

#[test]
fn test_collection_creation() {
    let config = CollectionConfig {
        name: "test_collection".to_string(),
        vector_dim: 128,
        distance: Distance::Cosine,
        use_hnsw: true,
        enable_bm25: false,
    };
    
    let collection = Collection::new(config);
    assert_eq!(collection.name(), "test_collection");
    assert_eq!(collection.vector_dim(), 128);
    assert_eq!(collection.count(), 0);
}

#[test]
fn test_point_insertion() {
    let config = CollectionConfig {
        name: "test".to_string(),
        vector_dim: 3,
        distance: Distance::Cosine,
        use_hnsw: false,
        enable_bm25: false,
    };
    
    let collection = Collection::new(config);
    
    let point = Point::new(
        PointId::String("point1".to_string()),
        Vector::new(vec![1.0, 2.0, 3.0]),
        Some(serde_json::json!({"name": "test"})),
    );
    
    assert!(collection.upsert(point).is_ok());
    assert_eq!(collection.count(), 1);
    
    let retrieved = collection.get("point1");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().vector.dim(), 3);
}

#[test]
fn test_vector_search() {
    let config = CollectionConfig {
        name: "test".to_string(),
        vector_dim: 3,
        distance: Distance::Cosine,
        use_hnsw: false, // Use linear search for small dataset
        enable_bm25: false,
    };
    
    let collection = Collection::new(config);
    
    // Insert some points
    for i in 0..10 {
        let vector = Vector::new(vec![i as f32, i as f32, i as f32]);
        let point = Point::new(
            PointId::Integer(i),
            vector,
            None,
        );
        collection.upsert(point).unwrap();
    }
    
    // Search for similar vector
    let query = Vector::new(vec![5.0, 5.0, 5.0]);
    let results = collection.search(&query, 3, None);
    
    assert_eq!(results.len(), 3);
    // Should find points close to [5,5,5]
    assert!(results[0].1 > 0.9); // High similarity
}

#[test]
fn test_bm25_search() {
    let config = CollectionConfig {
        name: "test".to_string(),
        vector_dim: 3,
        distance: Distance::Cosine,
        use_hnsw: false,
        enable_bm25: true,
    };
    
    let collection = Collection::new(config);
    
    // Insert documents with text
    for i in 0..10 {
        let point = Point::new(
            PointId::String(format!("doc{}", i)),
            Vector::new(vec![0.0, 0.0, 0.0]),
            Some(serde_json::json!({
                "text": format!("document about topic {}", i)
            })),
        );
        collection.upsert(point).unwrap();
    }
    
    // Search
    let results = collection.search_text("topic", 5);
    assert!(!results.is_empty());
    assert!(results[0].1 >= 0.0); // Should have a score (can be 0.0)
}

#[test]
fn test_storage_manager() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageManager::new(temp_dir.path()).unwrap();
    
    let config = CollectionConfig {
        name: "test_collection".to_string(),
        vector_dim: 128,
        distance: Distance::Cosine,
        use_hnsw: true,
        enable_bm25: false,
    };
    
    let collection = storage.create_collection(config).unwrap();
    assert_eq!(collection.name(), "test_collection");
    
    let retrieved = storage.get_collection("test_collection");
    assert!(retrieved.is_some());
    
    let collections = storage.list_collections();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0], "test_collection");
}

#[test]
fn test_persistence_snapshot() {
    // Use unique temp directory for each test to avoid LMDB conflicts
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = StorageManager::new(temp_dir.path()).unwrap();
    
    // Create collection and insert data
    let config = CollectionConfig {
        name: "persistent".to_string(),
        vector_dim: 3,
        distance: Distance::Cosine,
        use_hnsw: false,
        enable_bm25: false,
    };
    
    let collection = storage.create_collection(config).unwrap();
    
    for i in 0..10 {
        let point = Point::new(
            PointId::Integer(i),
            Vector::new(vec![i as f32, i as f32, i as f32]),
            None,
        );
        collection.upsert(point).unwrap();
    }
    
    // Force save
    storage.save().unwrap();
    
    // Drop the first storage manager to close LMDB environment
    drop(storage);
    
    // Create new storage manager (simulates restart)
    let storage2 = StorageManager::new(temp_dir.path()).unwrap();
    let restored = storage2.get_collection("persistent");
    
    assert!(restored.is_some());
    assert_eq!(restored.unwrap().count(), 10);
}

#[test]
fn test_payload_filtering() {
    let config = CollectionConfig {
        name: "test".to_string(),
        vector_dim: 3,
        distance: Distance::Cosine,
        use_hnsw: false,
        enable_bm25: false,
    };
    
    let collection = Collection::new(config);
    
    // Insert points with different categories
    for i in 0..20 {
        let point = Point::new(
            PointId::Integer(i),
            Vector::new(vec![i as f32, i as f32, i as f32]),
            Some(serde_json::json!({
                "category": if i % 2 == 0 { "A" } else { "B" },
                "score": i
            })),
        );
        collection.upsert(point).unwrap();
    }
    
    // Search with filter
    use distx_core::{PayloadFilter, FilterCondition};
    let query = Vector::new(vec![10.0, 10.0, 10.0]);
    let filter = PayloadFilter::new(FilterCondition::Equals {
        field: "category".to_string(),
        value: serde_json::json!("A"),
    });
    
    let results = collection.search(&query, 10, Some(&filter));
    
    // All results should have category "A"
    for (point, _) in results {
        let category = point.payload
            .as_ref()
            .and_then(|p| p.get("category"))
            .and_then(|v| v.as_str());
        assert_eq!(category, Some("A"));
    }
}

// ==================== Similarity Engine Tests ====================

#[test]
fn test_similarity_schema_creation() {
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), FieldConfig::text(0.5));
    fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
    fields.insert("category".to_string(), FieldConfig::categorical(0.2));

    let mut schema = SimilaritySchema::new(fields);
    assert_eq!(schema.version, 1);
    assert_eq!(schema.fields.len(), 3);
    
    // Validate and normalize
    schema.validate_and_normalize().unwrap();
    
    // Weights should sum to 1.0
    let weight_sum: f32 = schema.fields.values().map(|f| f.weight).sum();
    assert!((weight_sum - 1.0).abs() < 0.01);
}

// Note: Embedding is now done client-side. These tests use mock vectors.

#[test]
fn test_similarity_with_vectors_and_collection() {
    // Create a schema for products
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), FieldConfig::text(0.5));
    fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
    fields.insert("category".to_string(), FieldConfig::categorical(0.2));

    let schema = SimilaritySchema::new(fields);
    
    // Create collection with a standard embedding dimension
    let config = CollectionConfig {
        name: "products".to_string(),
        vector_dim: 3,  // Simple mock vectors
        distance: Distance::Cosine,
        use_hnsw: false,
        enable_bm25: false,
    };
    
    let collection = Collection::new(config);
    
    // Insert products with mock vectors (in real use, these come from OpenAI etc.)
    let products = vec![
        (vec![1.0, 0.0, 0.0], serde_json::json!({"name": "Prosciutto cotto", "price": 1.99, "category": "salumi"})),
        (vec![0.9, 0.1, 0.0], serde_json::json!({"name": "Prosciutto crudo", "price": 2.49, "category": "salumi"})),
        (vec![0.8, 0.2, 0.0], serde_json::json!({"name": "Mortadella", "price": 1.79, "category": "salumi"})),
        (vec![0.0, 0.0, 1.0], serde_json::json!({"name": "iPhone 15", "price": 999.0, "category": "electronics"})),
        (vec![0.1, 0.0, 0.9], serde_json::json!({"name": "Samsung Galaxy", "price": 899.0, "category": "electronics"})),
    ];
    
    for (i, (vec, payload)) in products.iter().enumerate() {
        let point = Point::new(
            PointId::Integer(i as u64),
            Vector::new(vec.clone()),
            Some(payload.clone()),
        );
        collection.upsert(point).unwrap();
    }
    
    // Query with a vector similar to salumi products
    let query_vector = Vector::new(vec![0.95, 0.05, 0.0]);
    
    let results = collection.search(&query_vector, 3, None);
    assert_eq!(results.len(), 3);
    
    // Top results should be salumi products (similar vectors)
    let top_category = results[0].0.payload
        .as_ref()
        .and_then(|p| p.get("category"))
        .and_then(|v| v.as_str());
    assert_eq!(top_category, Some("salumi"));
}

#[test]
fn test_reranker_with_explanations() {
    // Create a schema
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), FieldConfig::text(0.5));
    fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
    fields.insert("category".to_string(), FieldConfig::categorical(0.2));

    let mut schema = SimilaritySchema::new(fields);
    schema.validate_and_normalize().unwrap();
    
    let reranker = Reranker::new(schema);
    
    // Create test candidates
    let candidates: Vec<(Point, f32)> = vec![
        (Point::new(
            PointId::Integer(1),
            Vector::new(vec![0.0; 10]),
            Some(serde_json::json!({"name": "Prosciutto cotto", "price": 1.99, "category": "salumi"})),
        ), 0.9),
        (Point::new(
            PointId::Integer(2),
            Vector::new(vec![0.0; 10]),
            Some(serde_json::json!({"name": "iPhone 15", "price": 999.0, "category": "electronics"})),
        ), 0.8),
        (Point::new(
            PointId::Integer(3),
            Vector::new(vec![0.0; 10]),
            Some(serde_json::json!({"name": "Prosciutto crudo", "price": 2.49, "category": "salumi"})),
        ), 0.7),
    ];
    
    // Query
    let query = serde_json::json!({
        "name": "Prosciutto",
        "price": 2.0,
        "category": "salumi"
    });
    
    let results = reranker.rerank(&query, candidates);
    
    // Should have 3 results
    assert_eq!(results.len(), 3);
    
    // Top 2 should be prosciutto products (salumi category matches)
    let top_id = results[0].point.id.to_string();
    assert!(top_id == "1" || top_id == "3", "Expected prosciutto product, got id={}", top_id);
    
    // Each result should have field_scores
    for result in &results {
        assert!(result.field_scores.contains_key("name"));
        assert!(result.field_scores.contains_key("price"));
        assert!(result.field_scores.contains_key("category"));
        
        // Score should be sum of field scores
        let sum: f32 = result.field_scores.values().sum();
        assert!((result.score - sum).abs() < 0.01, 
            "Score {} should equal sum of field scores {}", result.score, sum);
    }
}

#[test]
fn test_similarity_schema_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Create storage and schema
    {
        let storage = StorageManager::new(temp_dir.path()).unwrap();
        
        let config = CollectionConfig {
            name: "products".to_string(),
            vector_dim: 129, // Example dimension
            distance: Distance::Cosine,
            use_hnsw: false,
            enable_bm25: false,
        };
        storage.create_collection(config).unwrap();
        
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), FieldConfig::text(0.5));
        fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
        fields.insert("category".to_string(), FieldConfig::categorical(0.2));
        
        let schema = SimilaritySchema::new(fields);
        storage.set_similarity_schema("products", schema).unwrap();
        
        // Verify it's stored
        assert!(storage.has_similarity_schema("products"));
        let retrieved = storage.get_similarity_schema("products").unwrap();
        assert_eq!(retrieved.fields.len(), 3);
    }
    
    // Reopen storage and verify schema persisted
    {
        let storage = StorageManager::new(temp_dir.path()).unwrap();
        
        assert!(storage.has_similarity_schema("products"));
        let schema = storage.get_similarity_schema("products").unwrap();
        assert_eq!(schema.fields.len(), 3);
        assert!(schema.fields.contains_key("name"));
        assert!(schema.fields.contains_key("price"));
        assert!(schema.fields.contains_key("category"));
    }
}

#[test]
fn test_weight_overrides() {
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), FieldConfig::text(0.5));
    fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
    fields.insert("category".to_string(), FieldConfig::categorical(0.2));

    let mut schema = SimilaritySchema::new(fields);
    schema.validate_and_normalize().unwrap();
    
    let reranker = Reranker::new(schema.clone());
    let original_price_weight = reranker.schema().fields.get("price").unwrap().weight;
    
    // Apply custom weight overrides to boost price
    let weight_overrides = HashMap::from([
        ("price".to_string(), 0.8),
        ("name".to_string(), 0.1),
    ]);
    let modified = reranker.with_weights(&weight_overrides);
    let modified_price_weight = modified.schema().fields.get("price").unwrap().weight;
    
    // Price weight should be higher with the override
    assert!(modified_price_weight > original_price_weight,
        "Price weight should increase: {} > {}", modified_price_weight, original_price_weight);
}

