use actix_web::{web, App, HttpServer, HttpResponse, Result as ActixResult};
use actix_cors::Cors;
use actix_files::Files;
use distx_core::{CollectionConfig, Distance, Point, Vector, PayloadFilter, FilterCondition, Filter, MultiVector};
use distx_storage::StorageManager;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use std::path::Path;
use std::collections::HashMap;

// Dashboard configuration
const STATIC_DIR: &str = "./static";
const DASHBOARD_PATH: &str = "/dashboard";

#[derive(Deserialize)]
struct CreateCollectionRequest {
    /// Dense vectors configuration (optional - can be omitted for sparse-only collections)
    #[serde(default, deserialize_with = "deserialize_vectors_optional")]
    vectors: Option<VectorConfig>,
    #[serde(default)]
    use_hnsw: bool,
    #[serde(default)]
    enable_bm25: bool,
    // Qdrant compatibility - sparse vectors (stored but not fully implemented)
    #[serde(default)]
    sparse_vectors: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone)]
struct VectorConfig {
    size: usize,
    distance: Option<String>,
    // Qdrant compatibility - ignored fields
    #[serde(default)]
    on_disk: Option<bool>,
    #[serde(default)]
    hnsw_config: Option<serde_json::Value>,
    #[serde(default)]
    quantization_config: Option<serde_json::Value>,
    #[serde(default)]
    multivector_config: Option<serde_json::Value>,
    #[serde(default)]
    datatype: Option<String>,
}

// Custom deserializer to handle both simple and named vector formats
fn deserialize_vectors_optional<'de, D>(deserializer: D) -> Result<Option<VectorConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    
    let Some(value) = value else {
        return Ok(None);
    };
    
    // Try simple format first: {"size": 1536, "distance": "Cosine"}
    if let Ok(config) = serde_json::from_value::<VectorConfig>(value.clone()) {
        return Ok(Some(config));
    }
    
    // Try named vectors format: {"": {"size": 1536, ...}} or {"vector_name": {"size": 1536, ...}}
    if let Ok(named) = serde_json::from_value::<HashMap<String, VectorConfig>>(value.clone()) {
        // Use the first vector config (default or named)
        if let Some(config) = named.into_values().next() {
            return Ok(Some(config));
        }
    }
    
    Err(serde::de::Error::custom("Invalid vectors configuration: expected either {\"size\": N, \"distance\": \"...\"} or {\"name\": {\"size\": N, ...}}"))
}

// Note: Using serde_json::json! for flexible responses instead of these structs
#[allow(dead_code)]
#[derive(Serialize)]
struct CollectionInfo {
    name: String,
    vectors: VectorConfigResponse,
    points_count: usize,
}

#[allow(dead_code)]
#[derive(Serialize)]
struct VectorConfigResponse {
    size: usize,
    distance: String,
}

#[derive(Deserialize)]
struct UpsertPointsRequest {
    points: Vec<PointRequest>,
}

/// Parsed vector data - can be single or multi
struct ParsedVector {
    /// First/primary vector (for backwards compatibility)
    primary: Vec<f32>,
    /// Full multivector data if this was a multivector input
    multivector: Option<Vec<Vec<f32>>>,
}

#[derive(Deserialize)]
struct PointRequest {
    id: serde_json::Value,
    #[serde(deserialize_with = "deserialize_vector")]
    vector: ParsedVector,
    payload: Option<serde_json::Value>,
}

// Custom deserializer to handle multiple vector formats (Qdrant compatibility)
// Simple: [0.1, 0.2, 0.3]
// Multivector: [[0.1, 0.2], [0.3, 0.4]] -> stores full multivector for MaxSim search
// Named vectors: {"vector_name": [0.1, 0.2]} -> extracts the vector
fn deserialize_vector<'de, D>(deserializer: D) -> Result<ParsedVector, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    
    fn parse_simple_vector(arr: &[serde_json::Value]) -> Result<Vec<f32>, String> {
        arr.iter()
            .map(|v| v.as_f64().map(|f| f as f32).ok_or_else(|| "expected f32".to_string()))
            .collect()
    }
    
    fn parse_multivector(arr: &[serde_json::Value]) -> Result<Vec<Vec<f32>>, String> {
        arr.iter()
            .map(|sub| {
                if let serde_json::Value::Array(sub_arr) = sub {
                    parse_simple_vector(sub_arr)
                } else {
                    Err("expected array of arrays for multivector".to_string())
                }
            })
            .collect()
    }
    
    match &value {
        // Array: could be simple vector or multivector
        serde_json::Value::Array(arr) if !arr.is_empty() => {
            match arr.first() {
                // Simple vector: [0.1, 0.2, 0.3]
                Some(serde_json::Value::Number(_)) => {
                    let primary = parse_simple_vector(arr).map_err(serde::de::Error::custom)?;
                    Ok(ParsedVector { primary, multivector: None })
                }
                // Multivector: [[0.1, 0.2], [0.3, 0.4]] - store full multivector
                Some(serde_json::Value::Array(_)) => {
                    let multivec = parse_multivector(arr).map_err(serde::de::Error::custom)?;
                    let primary = multivec.first().cloned().unwrap_or_default();
                    Ok(ParsedVector { primary, multivector: Some(multivec) })
                }
                _ => Err(serde::de::Error::custom("invalid vector format: expected number or array"))
            }
        }
        // Empty array
        serde_json::Value::Array(_) => {
            Err(serde::de::Error::custom("vector cannot be empty"))
        }
        // Named vector: {"vector_name": [0.1, 0.2]} or {"": [...]}
        serde_json::Value::Object(obj) => {
            if let Some((_, vec_value)) = obj.iter().next() {
                match vec_value {
                    serde_json::Value::Array(arr) if !arr.is_empty() => {
                        match arr.first() {
                            Some(serde_json::Value::Number(_)) => {
                                let primary = parse_simple_vector(arr).map_err(serde::de::Error::custom)?;
                                Ok(ParsedVector { primary, multivector: None })
                            }
                            Some(serde_json::Value::Array(_)) => {
                                // Named multivector
                                let multivec = parse_multivector(arr).map_err(serde::de::Error::custom)?;
                                let primary = multivec.first().cloned().unwrap_or_default();
                                Ok(ParsedVector { primary, multivector: Some(multivec) })
                            }
                            _ => Err(serde::de::Error::custom("invalid named vector format"))
                        }
                    }
                    _ => Err(serde::de::Error::custom("named vector value must be a non-empty array"))
                }
            } else {
                Err(serde::de::Error::custom("empty named vector object"))
            }
        }
        _ => Err(serde::de::Error::custom("vector must be an array or object")),
    }
}

#[derive(Deserialize)]
struct SearchRequest {
    vector: Option<Vec<f32>>,
    text: Option<String>,
    limit: Option<usize>,
    filter: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Serialize)]
struct SearchResult {
    id: serde_json::Value,
    score: f32,
    payload: Option<serde_json::Value>,
}

pub struct RestApi;

impl RestApi {
    pub async fn start(
        storage: Arc<StorageManager>,
        port: u16,
    ) -> std::io::Result<()> {
        Self::start_with_static_dir(storage, port, STATIC_DIR).await
    }
    
    pub async fn start_with_static_dir(
        storage: Arc<StorageManager>,
        port: u16,
        static_dir: &str,
    ) -> std::io::Result<()> {
        let static_folder = static_dir.to_string();
        
        HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);

            let mut app = App::new()
                .wrap(cors)
                .app_data(web::Data::new(storage.clone()))
                // Qdrant-compatible endpoints
                .route("/", web::get().to(root_info))
                .route("/healthz", web::get().to(health_check))
                .route("/collections", web::get().to(list_collections))
                .route("/collections/{name}", web::get().to(get_collection))
                .route("/collections/{name}", web::put().to(create_collection))
                .route("/collections/{name}", web::delete().to(delete_collection))
                .route("/collections/{name}/points", web::put().to(upsert_points))
                .route("/collections/{name}/points/scroll", web::post().to(scroll_points))
                .route("/collections/{name}/points/delete", web::post().to(delete_points_by_filter))
                .route("/collections/{name}/points/search", web::post().to(search_points))
                .route("/collections/{name}/points/query", web::post().to(query_points))
                .route("/collections/{name}/points/{id}", web::get().to(get_point))
                .route("/collections/{name}/points/{id}", web::delete().to(delete_point))
                .route("/collections/{name}/exists", web::get().to(collection_exists))
                // Qdrant compatibility - additional endpoints
                .route("/aliases", web::get().to(list_aliases))
                .route("/collections/aliases", web::post().to(update_aliases))
                .route("/collections/{name}/aliases", web::get().to(list_collection_aliases))
                .route("/cluster", web::get().to(cluster_info))
                .route("/collections/{name}/cluster", web::get().to(collection_cluster_info))
                .route("/telemetry", web::get().to(telemetry_info))
                // Points batch operations
                .route("/collections/{name}/points", web::post().to(get_points_by_ids))
                .route("/collections/{name}/points/count", web::post().to(count_points))
                .route("/collections/{name}/points/payload", web::post().to(set_payload))
                .route("/collections/{name}/points/payload", web::put().to(overwrite_payload))
                .route("/collections/{name}/points/payload/delete", web::post().to(delete_payload))
                .route("/collections/{name}/points/payload/clear", web::post().to(clear_payload))
                .route("/collections/{name}/points/vectors", web::put().to(update_vectors))
                .route("/collections/{name}/points/vectors/delete", web::post().to(delete_vectors))
                .route("/collections/{name}/points/batch", web::post().to(batch_update))
                .route("/collections/{name}/points/search/batch", web::post().to(batch_search))
                .route("/collections/{name}/points/query/batch", web::post().to(batch_query))
                .route("/collections/{name}/points/query/groups", web::post().to(query_groups))
                // Index endpoints
                .route("/collections/{name}/index", web::put().to(create_field_index))
                .route("/collections/{name}/index/{field_name}", web::delete().to(delete_field_index))
                // Recommend endpoint
                .route("/collections/{name}/points/recommend", web::post().to(recommend_points))
                // Snapshot endpoints (stubs for UI compatibility)
                .route("/collections/{name}/snapshots", web::get().to(list_snapshots))
                .route("/collections/{name}/snapshots", web::post().to(create_snapshot))
                .route("/collections/{name}/snapshots/recover", web::put().to(recover_snapshot))
                .route("/collections/{name}/snapshots/{snapshot_name}", web::get().to(get_snapshot))
                .route("/collections/{name}/snapshots/{snapshot_name}", web::delete().to(delete_snapshot))
                .route("/snapshots", web::get().to(list_all_snapshots));
            
            // Serve web UI dashboard if static folder exists
            let static_path = Path::new(&static_folder);
            if static_path.exists() && static_path.is_dir() {
                app = app.service(
                    Files::new(DASHBOARD_PATH, static_folder.clone())
                        .index_file("index.html")
                        .use_last_modified(true)
                );
            }
            
            app
        })
        .bind(("0.0.0.0", port))?
        .run()
        .await
    }
}

async fn root_info() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "title": "DistX - Fast Vector Database",
        "version": "0.2.1"
    })))
}

async fn health_check() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "title": "DistX",
        "version": "0.2.1"
    })))
}

async fn list_collections(
    storage: web::Data<Arc<StorageManager>>,
) -> ActixResult<HttpResponse> {
    let collection_names = storage.list_collections();
    
    // Format to match Qdrant's response structure
    let collections: Vec<serde_json::Value> = collection_names.into_iter().map(|name| {
        if let Some(collection) = storage.get_collection(&name) {
            serde_json::json!({
                "name": name,
                "config": {
                    "vectors": {
                        "size": collection.vector_dim(),
                        "distance": format!("{:?}", collection.distance())
                    }
                }
            })
        } else {
            serde_json::json!({
                "name": name
            })
        }
    }).collect();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "collections": collections
        }
    })))
}

async fn get_collection(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if let Some(collection) = storage.get_collection(&name) {
        let distance_str = format!("{:?}", collection.distance());
        let vector_dim = collection.vector_dim();
        let points_count = collection.count();
        
        // Format to match Qdrant's full response structure
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "result": {
                "status": "green",
                "optimizer_status": "ok",
                "vectors_count": points_count,
                "indexed_vectors_count": points_count,
                "points_count": points_count,
                "segments_count": 1,
                "config": {
                    "params": {
                        "vectors": {
                            "size": vector_dim,
                            "distance": distance_str
                        },
                        "shard_number": 1,
                        "replication_factor": 1
                    },
                    "hnsw_config": {
                        "m": 16,
                        "ef_construct": 100,
                        "full_scan_threshold": 10000
                    },
                    "optimizer_config": {
                        "deleted_threshold": 0.2,
                        "vacuum_min_vector_number": 1000,
                        "default_segment_number": 0,
                        "indexing_threshold": 20000,
                        "flush_interval_sec": 5,
                        "max_segment_size_mb": null,
                        "memmap_threshold_mb": null
                    },
                    "wal_config": {
                        "wal_capacity_mb": 32,
                        "wal_segments_ahead": 0
                    }
                },
                "payload_schema": {}
            }
        })))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": {
                "error": "Collection not found"
            }
        })))
    }
}

async fn create_collection(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<CreateCollectionRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    // Handle sparse-only collections (Qdrant compatibility)
    // For sparse-only collections, we create with a default vector dimension
    let (vector_dim, distance) = if let Some(ref vectors) = req.vectors {
        let dist = match vectors.distance.as_deref() {
            Some("Cosine") | Some("cosine") => Distance::Cosine,
            Some("Euclidean") | Some("euclidean") => Distance::Euclidean,
            Some("Dot") | Some("dot") => Distance::Dot,
            _ => Distance::Cosine,
        };
        (vectors.size, dist)
    } else if req.sparse_vectors.is_some() {
        // Sparse-only collection - use BM25 with default text dimension
        // Note: DistX uses BM25 for sparse text search, not sparse vectors
        (0, Distance::Cosine)
    } else {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "status": {
                "error": "Either 'vectors' or 'sparse_vectors' must be provided"
            }
        })));
    };

    let config = CollectionConfig {
        name: name.clone(),
        vector_dim,
        distance,
        use_hnsw: req.use_hnsw,
        // Enable BM25 for sparse collections
        enable_bm25: req.enable_bm25 || req.sparse_vectors.is_some(),
    };

    match storage.create_collection(config) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "result": true
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "status": {
                "error": e.to_string()
            }
        }))),
    }
}

async fn delete_collection(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    match storage.delete_collection(&name) {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "result": true
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": {
                "error": "Collection not found"
            }
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "status": {
                "error": e.to_string()
            }
        }))),
    }
}

async fn upsert_points(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<UpsertPointsRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    let collection = match storage.get_collection(&name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": {
                    "error": "Collection not found"
                }
            })));
        }
    };

    let points: Result<Vec<Point>, &str> = req.points.iter().map(|point_req| {
        let id = match &point_req.id {
            serde_json::Value::String(s) => distx_core::PointId::String(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(u) = n.as_u64() {
                    distx_core::PointId::Integer(u)
                } else {
                    return Err("Invalid point ID");
                }
            }
            _ => return Err("Invalid point ID"),
        };

        // Create point with multivector support
        let point = if let Some(ref multivec_data) = point_req.vector.multivector {
            // Create MultiVector and Point with multivector
            match MultiVector::new(multivec_data.clone()) {
                Ok(mv) => Point::new_multi(id, mv, point_req.payload.clone()),
                Err(_) => {
                    // Fallback to primary vector if multivector creation fails
                    let vector = Vector::new(point_req.vector.primary.clone());
                    Point::new(id, vector, point_req.payload.clone())
                }
            }
        } else {
            // Simple dense vector
            let vector = Vector::new(point_req.vector.primary.clone());
            Point::new(id, vector, point_req.payload.clone())
        };
        
        Ok(point)
    }).collect();

    match points {
        Ok(points_vec) => {
            if points_vec.len() > 1 {
                const PREWARM_THRESHOLD: usize = 1000;
                let should_prewarm = points_vec.len() >= PREWARM_THRESHOLD;
                
                let result = if should_prewarm {
                    collection.batch_upsert_with_prewarm(points_vec, true)
                } else {
                    collection.batch_upsert(points_vec)
                };
                
                if let Err(e) = result {
                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                        "status": {
                            "error": e.to_string()
                        }
                    })));
                }
            } else if let Some(point) = points_vec.first() {
                if let Err(e) = collection.upsert(point.clone()) {
                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                        "status": {
                            "error": e.to_string()
                        }
                    })));
                }
            }
        }
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "status": {
                    "error": e
                }
            })));
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

async fn search_points(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<SearchRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    let collection = match storage.get_collection(&name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": {
                    "error": "Collection not found"
                }
            })));
        }
    };

    let limit = req.limit.unwrap_or(10);

    if let Some(text) = &req.text {
        let results = collection.search_text(text, limit);
        let search_results: Vec<serde_json::Value> = results
            .into_iter()
            .filter_map(|(doc_id, score)| {
                collection.get(&doc_id).map(|point| {
                    serde_json::json!({
                        "id": match &point.id {
                            distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                            distx_core::PointId::Integer(i) => serde_json::Value::Number((*i).into()),
                            distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
                        },
                        "version": 0,
                        "score": score,
                        "payload": point.payload,
                    })
                })
            })
            .collect();

        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "result": search_results
        })));
    }

    if let Some(vector_data) = &req.vector {
        let query_vector = Vector::new(vector_data.clone());
        
        let filter: Option<Box<dyn Filter>> = req.filter.as_ref().and_then(|f| {
            parse_filter(f).map(|cond| Box::new(PayloadFilter::new(cond)) as Box<dyn Filter>)
        });

        let results = if let Some(f) = filter.as_deref() {
            collection.search(&query_vector, limit, Some(f))
        } else {
            collection.search(&query_vector, limit, None)
        };

        let search_results: Vec<serde_json::Value> = results
            .into_iter()
            .map(|(point, score)| {
                serde_json::json!({
                    "id": match &point.id {
                        distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                        distx_core::PointId::Integer(i) => serde_json::Value::Number((*i).into()),
                        distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
                    },
                    "version": 0,
                    "score": score,
                    "payload": point.payload,
                })
            })
            .collect();

        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "result": search_results
        })));
    }

    Ok(HttpResponse::BadRequest().json(serde_json::json!({
        "status": {
            "error": "Either 'vector' or 'text' must be provided"
        }
    })))
}

/// Query request for Qdrant's universal query API
/// Supports both single vectors and multivectors (ColBERT-style MaxSim)
#[derive(Deserialize)]
struct QueryRequest {
    /// Query vector - can be single [f32] or multi [[f32]]
    query: serde_json::Value,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    with_payload: Option<bool>,
    #[serde(default)]
    with_vector: Option<bool>,
    #[serde(default)]
    filter: Option<serde_json::Value>,
}

/// Query points using Qdrant's universal query API
/// Supports multivector queries with MaxSim scoring
async fn query_points(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<QueryRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    let collection = match storage.get_collection(&name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": {
                    "error": "Collection not found"
                }
            })));
        }
    };

    let limit = req.limit.unwrap_or(10);
    let with_payload = req.with_payload.unwrap_or(true);
    let with_vector = req.with_vector.unwrap_or(false);
    
    // Parse filter if provided
    let filter: Option<Box<dyn Filter>> = req.filter.as_ref().and_then(|f| {
        parse_filter(f).map(|cond| Box::new(PayloadFilter::new(cond)) as Box<dyn Filter>)
    });
    
    // Determine if query is multivector or single vector
    let results = match &req.query {
        serde_json::Value::Array(arr) if !arr.is_empty() => {
            match arr.first() {
                // Multivector: [[0.1, 0.2], [0.3, 0.4]]
                Some(serde_json::Value::Array(_)) => {
                    // Parse multivector
                    let multivec_data: Result<Vec<Vec<f32>>, _> = arr.iter()
                        .map(|sub| {
                            if let serde_json::Value::Array(sub_arr) = sub {
                                sub_arr.iter()
                                    .map(|v| v.as_f64().map(|f| f as f32).ok_or("expected f32"))
                                    .collect::<Result<Vec<f32>, _>>()
                            } else {
                                Err("expected array")
                            }
                        })
                        .collect();
                    
                    match multivec_data {
                        Ok(data) => {
                            match MultiVector::new(data) {
                                Ok(query_mv) => {
                                    // Use MaxSim search
                                    if let Some(f) = filter.as_deref() {
                                        collection.search_multivector(&query_mv, limit, Some(f))
                                    } else {
                                        collection.search_multivector(&query_mv, limit, None)
                                    }
                                }
                                Err(e) => {
                                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                        "status": { "error": format!("Invalid multivector: {}", e) }
                                    })));
                                }
                            }
                        }
                        Err(e) => {
                            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                "status": { "error": format!("Invalid multivector format: {}", e) }
                            })));
                        }
                    }
                }
                // Single vector: [0.1, 0.2, 0.3]
                Some(serde_json::Value::Number(_)) => {
                    let vector_data: Result<Vec<f32>, _> = arr.iter()
                        .map(|v| v.as_f64().map(|f| f as f32).ok_or("expected f32"))
                        .collect();
                    
                    match vector_data {
                        Ok(data) => {
                            let query_vector = Vector::new(data);
                            if let Some(f) = filter.as_deref() {
                                collection.search(&query_vector, limit, Some(f))
                            } else {
                                collection.search(&query_vector, limit, None)
                            }
                        }
                        Err(e) => {
                            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                "status": { "error": format!("Invalid vector: {}", e) }
                            })));
                        }
                    }
                }
                _ => {
                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                        "status": { "error": "Invalid query format" }
                    })));
                }
            }
        }
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "status": { "error": "Query must be a vector array" }
            })));
        }
    };
    
    // Format results
    let search_results: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(point, score)| {
            let mut result = serde_json::json!({
                "id": match &point.id {
                    distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                    distx_core::PointId::Integer(i) => serde_json::Value::Number((*i).into()),
                    distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
                },
                "version": 0,
                "score": score,
            });
            
            if with_payload {
                result["payload"] = point.payload.clone().unwrap_or(serde_json::Value::Null);
            }
            
            if with_vector {
                result["vector"] = serde_json::json!(point.vector.as_slice());
                if let Some(mv) = &point.multivector {
                    result["multivector"] = serde_json::json!(mv.vectors());
                }
            }
            
            result
        })
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "points": search_results
        }
    })))
}

fn parse_filter(filter_json: &serde_json::Value) -> Option<FilterCondition> {
    if let Some(obj) = filter_json.as_object() {
        if let Some(field) = obj.get("field").and_then(|v| v.as_str()) {
            if let Some(value) = obj.get("value") {
                if let Some(op) = obj.get("operator").and_then(|v| v.as_str()) {
                    match op {
                        "eq" => return Some(FilterCondition::Equals { field: field.to_string(), value: value.clone() }),
                        "ne" => return Some(FilterCondition::NotEquals { field: field.to_string(), value: value.clone() }),
                        "gt" => return value.as_f64().map(|v| FilterCondition::GreaterThan { field: field.to_string(), value: v }),
                        "lt" => return value.as_f64().map(|v| FilterCondition::LessThan { field: field.to_string(), value: v }),
                        "gte" => return value.as_f64().map(|v| FilterCondition::GreaterEqual { field: field.to_string(), value: v }),
                        "lte" => return value.as_f64().map(|v| FilterCondition::LessEqual { field: field.to_string(), value: v }),
                        _ => {}
                    }
                }
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct ScrollRequest {
    limit: Option<usize>,
    offset: Option<serde_json::Value>,
    with_payload: Option<bool>,
    with_vector: Option<bool>,
}

async fn scroll_points(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<ScrollRequest>,
) -> ActixResult<HttpResponse> {
    let collection_name = path.into_inner();
    
    let collection = match storage.get_collection(&collection_name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": {
                    "error": "Collection not found"
                }
            })));
        }
    };
    
    let limit = req.limit.unwrap_or(10);
    let with_payload = req.with_payload.unwrap_or(true);
    let with_vector = req.with_vector.unwrap_or(false);
    
    // Get offset as integer if provided
    let offset_id: Option<i64> = req.offset.as_ref().and_then(|v| {
        match v {
            serde_json::Value::Number(n) => n.as_i64(),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    });
    
    // Get all points and sort by ID for consistent pagination
    let all_points = collection.get_all_points();
    let mut points_with_ids: Vec<_> = all_points.iter()
        .map(|p| {
            let id_num: i64 = match &p.id {
                distx_core::PointId::Integer(i) => *i as i64,
                distx_core::PointId::String(s) => s.parse::<i64>().unwrap_or(0),
                distx_core::PointId::Uuid(_) => 0,
            };
            (id_num, p)
        })
        .collect();
    
    points_with_ids.sort_by_key(|(id, _)| *id);
    
    // Apply offset
    let start_idx = if let Some(offset) = offset_id {
        points_with_ids.iter().position(|(id, _)| *id > offset).unwrap_or(points_with_ids.len())
    } else {
        0
    };
    
    // Get page of results
    let page: Vec<_> = points_with_ids.iter()
        .skip(start_idx)
        .take(limit)
        .collect();
    
    // Determine next offset
    let next_offset = if page.len() == limit && start_idx + limit < points_with_ids.len() {
        page.last().map(|(id, _)| serde_json::json!(*id))
    } else {
        None
    };
    
    // Format results
    let results: Vec<serde_json::Value> = page.iter().map(|(_, point)| {
        let mut obj = serde_json::json!({
            "id": match &point.id {
                distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                distx_core::PointId::Integer(i) => serde_json::json!(*i),
                distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
            },
        });
        
        if with_payload {
            obj["payload"] = point.payload.clone().unwrap_or(serde_json::json!({}));
        }
        if with_vector {
            obj["vector"] = serde_json::json!(point.vector.as_slice());
        }
        
        obj
    }).collect();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "points": results,
            "next_page_offset": next_offset
        }
    })))
}

async fn get_point(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (collection_name, point_id) = path.into_inner();
    
    let collection = match storage.get_collection(&collection_name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": {
                    "error": "Collection not found"
                }
            })));
        }
    };

    match collection.get(&point_id) {
        Some(point) => {
            // Build response with optional multivector
            let mut result = serde_json::json!({
                "id": match &point.id {
                    distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                    distx_core::PointId::Integer(i) => serde_json::Value::Number((*i).into()),
                    distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
                },
                "vector": point.vector.as_slice(),
                "payload": point.payload,
            });
            
            // Add multivector if present
            if let Some(mv) = &point.multivector {
                result["multivector"] = serde_json::json!(mv.vectors());
            }
            
            Ok(HttpResponse::Ok().json(serde_json::json!({ "result": result })))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": {
                "error": "Point not found"
            }
        }))),
    }
}

async fn delete_point(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (collection_name, point_id) = path.into_inner();
    
    let collection = match storage.get_collection(&collection_name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": {
                    "error": "Collection not found"
                }
            })));
        }
    };

    match collection.delete(&point_id) {
        Ok(true) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "result": {
                "operation_id": 0,
                "status": "completed"
            }
        }))),
        Ok(false) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": {
                "error": "Point not found"
            }
        }))),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "status": {
                "error": e.to_string()
            }
        }))),
    }
}

#[derive(Deserialize)]
struct DeletePointsRequest {
    filter: Option<DeleteFilter>,
    points: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct DeleteFilter {
    must: Option<Vec<FilterMust>>,
}

#[derive(Deserialize)]
struct FilterMust {
    key: String,
    #[serde(rename = "match")]
    match_value: MatchValue,
}

#[derive(Deserialize)]
struct MatchValue {
    value: serde_json::Value,
}

async fn delete_points_by_filter(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<DeletePointsRequest>,
) -> ActixResult<HttpResponse> {
    let collection_name = path.into_inner();
    
    let collection = match storage.get_collection(&collection_name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": {
                    "error": "Collection not found"
                }
            })));
        }
    };
    
    let mut _deleted_count = 0;
    
    // Handle filter-based deletion
    if let Some(filter) = &req.filter {
        if let Some(must_conditions) = &filter.must {
            // Get the field and value to match
            if let Some(condition) = must_conditions.first() {
                let field_key = &condition.key;
                let match_value = &condition.match_value.value;
                
                // Get all points and filter by payload
                let all_points = collection.get_all_points();
                let mut points_to_delete = Vec::new();
                
                for point in all_points {
                    if let Some(payload) = &point.payload {
                        if let Some(field_value) = payload.get(field_key) {
                            if field_value == match_value {
                                points_to_delete.push(point.id.clone());
                            }
                        }
                    }
                }
                
                // Delete matching points
                for point_id in points_to_delete {
                    let id_str = match &point_id {
                        distx_core::PointId::String(s) => s.clone(),
                        distx_core::PointId::Integer(i) => i.to_string(),
                        distx_core::PointId::Uuid(u) => u.to_string(),
                    };
                    if collection.delete(&id_str).is_ok() {
                        _deleted_count += 1;
                    }
                }
            }
        }
    }
    
    // Handle point ID-based deletion
    if let Some(point_ids) = &req.points {
        for point_id in point_ids {
            let id_str = match point_id {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => continue,
            };
            if collection.delete(&id_str).is_ok() {
                _deleted_count += 1;
            }
        }
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

async fn collection_exists(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    let exists = storage.collection_exists(&name);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "exists": exists
        }
    })))
}

// Qdrant compatibility endpoints

async fn list_aliases() -> ActixResult<HttpResponse> {
    // DistX doesn't support aliases yet, return empty list
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "aliases": []
        }
    })))
}

async fn cluster_info() -> ActixResult<HttpResponse> {
    // DistX runs as single node, return minimal cluster info
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "status": "enabled",
            "peer_id": 1,
            "peers": {
                "1": {
                    "uri": "http://localhost:6335"
                }
            },
            "raft_info": {
                "term": 0,
                "commit": 0,
                "pending_operations": 0,
                "leader": 1,
                "role": "Leader"
            },
            "consensus_thread_status": {
                "consensus_thread_status": "working"
            },
            "message_send_failures": {}
        }
    })))
}

async fn telemetry_info() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "id": "distx-single-node",
            "app": {
                "name": "distx",
                "version": "0.2.1"
            }
        }
    })))
}

// Snapshot endpoints (stubs - feature not fully implemented)

async fn list_snapshots(
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let _collection_name = path.into_inner();
    // Return empty list - snapshots not implemented yet
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": []
    })))
}

async fn list_all_snapshots() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": []
    })))
}

async fn create_snapshot(
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let collection_name = path.into_inner();
    Ok(HttpResponse::NotImplemented().json(serde_json::json!({
        "status": {
            "error": format!("Snapshot creation not yet implemented for collection '{}'", collection_name)
        }
    })))
}

#[derive(Deserialize)]
struct RecoverSnapshotRequest {
    location: String,
    #[serde(default)]
    priority: Option<String>,
}

async fn recover_snapshot(
    path: web::Path<String>,
    req: web::Json<RecoverSnapshotRequest>,
) -> ActixResult<HttpResponse> {
    let collection_name = path.into_inner();
    Ok(HttpResponse::NotImplemented().json(serde_json::json!({
        "status": {
            "error": format!(
                "Remote snapshot recovery not yet implemented. Cannot recover '{}' for collection '{}'. Please use the REST API to create collections and upload points directly.",
                req.location,
                collection_name
            )
        }
    })))
}

async fn get_snapshot(
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (collection_name, snapshot_name) = path.into_inner();
    Ok(HttpResponse::NotFound().json(serde_json::json!({
        "status": {
            "error": format!("Snapshot '{}' not found in collection '{}'", snapshot_name, collection_name)
        }
    })))
}

async fn delete_snapshot(
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (collection_name, snapshot_name) = path.into_inner();
    Ok(HttpResponse::NotFound().json(serde_json::json!({
        "status": {
            "error": format!("Snapshot '{}' not found in collection '{}'", snapshot_name, collection_name)
        }
    })))
}

// ============ Additional Qdrant-compatible endpoints ============

/// Update aliases (stub - aliases not yet implemented)
async fn update_aliases() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": true
    })))
}

/// List collection aliases
async fn list_collection_aliases(
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let _collection_name = path.into_inner();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "aliases": []
        }
    })))
}

/// Collection cluster info
async fn collection_cluster_info(
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let collection_name = path.into_inner();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "peer_id": 0,
            "shard_count": 1,
            "local_shards": [{
                "shard_id": 0,
                "points_count": 0,
                "state": "Active"
            }],
            "remote_shards": [],
            "shard_transfers": [],
            "collection_name": collection_name
        }
    })))
}

/// Get multiple points by IDs
#[derive(Deserialize)]
struct GetPointsRequest {
    ids: Vec<serde_json::Value>,
    #[serde(default)]
    with_payload: Option<bool>,
    #[serde(default)]
    with_vector: Option<bool>,
}

async fn get_points_by_ids(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<GetPointsRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    let collection = match storage.get_collection(&name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": { "error": "Collection not found" }
            })));
        }
    };

    let with_payload = req.with_payload.unwrap_or(true);
    let with_vector = req.with_vector.unwrap_or(false);
    
    let mut points = Vec::new();
    for id_value in &req.ids {
        let id_str = match id_value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => continue,
        };
        
        if let Some(point) = collection.get(&id_str) {
            let mut result = serde_json::json!({
                "id": id_value
            });
            if with_payload {
                result["payload"] = point.payload.clone().unwrap_or(serde_json::Value::Null);
            }
            if with_vector {
                result["vector"] = serde_json::json!(point.vector.as_slice());
            }
            points.push(result);
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": points
    })))
}

/// Count points in collection
#[derive(Deserialize)]
struct CountRequest {
    #[serde(default)]
    filter: Option<serde_json::Value>,
    #[serde(default)]
    exact: Option<bool>,
}

async fn count_points(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<CountRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    let collection = match storage.get_collection(&name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": { "error": "Collection not found" }
            })));
        }
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "count": collection.count()
        }
    })))
}

/// Set payload on points
#[derive(Deserialize)]
struct SetPayloadRequest {
    payload: serde_json::Value,
    #[serde(default)]
    points: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    filter: Option<serde_json::Value>,
}

async fn set_payload(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<SetPayloadRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    // Note: payload update not fully implemented
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

/// Overwrite payload on points
async fn overwrite_payload(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<SetPayloadRequest>,
) -> ActixResult<HttpResponse> {
    set_payload(storage, path, req).await
}

/// Delete payload fields from points
#[derive(Deserialize)]
struct DeletePayloadRequest {
    keys: Vec<String>,
    #[serde(default)]
    points: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    filter: Option<serde_json::Value>,
}

async fn delete_payload(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<DeletePayloadRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

/// Clear all payload from points
#[derive(Deserialize)]
struct ClearPayloadRequest {
    #[serde(default)]
    points: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    filter: Option<serde_json::Value>,
}

async fn clear_payload(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<ClearPayloadRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

/// Update vectors on existing points
#[derive(Deserialize)]
struct UpdateVectorsRequest {
    points: Vec<serde_json::Value>,
}

async fn update_vectors(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<UpdateVectorsRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

/// Delete vectors from points
#[derive(Deserialize)]
struct DeleteVectorsRequest {
    #[serde(default)]
    points: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    vectors: Vec<String>,
    #[serde(default)]
    filter: Option<serde_json::Value>,
}

async fn delete_vectors(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<DeleteVectorsRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

/// Batch update operations
#[derive(Deserialize)]
struct BatchUpdateRequest {
    operations: Vec<serde_json::Value>,
}

async fn batch_update(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<BatchUpdateRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": []
    })))
}

/// Batch search
#[derive(Deserialize)]
struct BatchSearchRequest {
    searches: Vec<serde_json::Value>,
}

async fn batch_search(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<BatchSearchRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": []
    })))
}

/// Batch query
#[derive(Deserialize)]
struct BatchQueryRequest {
    searches: Vec<serde_json::Value>,
}

async fn batch_query(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<BatchQueryRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": []
    })))
}

/// Query points with grouping
#[derive(Deserialize)]
struct QueryGroupsRequest {
    query: serde_json::Value,
    group_by: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    group_size: Option<usize>,
    #[serde(default)]
    with_payload: Option<bool>,
    #[serde(default)]
    with_vector: Option<bool>,
    #[serde(default)]
    filter: Option<serde_json::Value>,
}

async fn query_groups(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<QueryGroupsRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    let collection = match storage.get_collection(&name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": { "error": "Collection not found" }
            })));
        }
    };

    let limit = req.limit.unwrap_or(5);
    let group_size = req.group_size.unwrap_or(3);
    let with_payload = req.with_payload.unwrap_or(true);
    let with_vector = req.with_vector.unwrap_or(false);
    let group_by = &req.group_by;
    
    // Parse query vector
    let query_vector = match &req.query {
        serde_json::Value::Array(arr) => {
            let vec: Result<Vec<f32>, _> = arr.iter()
                .map(|v| v.as_f64().map(|f| f as f32).ok_or("expected f32"))
                .collect();
            match vec {
                Ok(v) => Vector::new(v),
                Err(_) => {
                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                        "status": { "error": "Invalid query vector" }
                    })));
                }
            }
        }
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "status": { "error": "Query must be a vector array" }
            })));
        }
    };
    
    // Search for points
    let search_results = collection.search(&query_vector, limit * group_size * 2, None);
    
    // Group results by the group_by field
    let mut groups: std::collections::HashMap<String, Vec<serde_json::Value>> = std::collections::HashMap::new();
    
    for (point, score) in search_results {
        // Get group key from payload
        let group_key = point.payload
            .as_ref()
            .and_then(|p| p.get(group_by))
            .and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .unwrap_or_else(|| "unknown".to_string());
        
        let group = groups.entry(group_key).or_default();
        
        // Only add if group hasn't reached group_size
        if group.len() < group_size {
            let mut hit = serde_json::json!({
                "id": match &point.id {
                    distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                    distx_core::PointId::Integer(i) => serde_json::Value::Number((*i).into()),
                    distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
                },
                "score": score
            });
            
            if with_payload {
                hit["payload"] = point.payload.clone().unwrap_or(serde_json::Value::Null);
            }
            if with_vector {
                hit["vector"] = serde_json::json!(point.vector.as_slice());
            }
            
            group.push(hit);
        }
        
        // Stop if we have enough groups
        if groups.len() >= limit && groups.values().all(|g| g.len() >= group_size) {
            break;
        }
    }
    
    // Format response
    let group_results: Vec<serde_json::Value> = groups
        .into_iter()
        .take(limit)
        .map(|(key, hits)| {
            serde_json::json!({
                "id": key,
                "hits": hits
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "groups": group_results
        }
    })))
}

/// Create field index
#[derive(Deserialize)]
struct CreateIndexRequest {
    field_name: String,
    #[serde(default)]
    field_schema: Option<serde_json::Value>,
}

async fn create_field_index(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    _req: web::Json<CreateIndexRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

/// Delete field index
async fn delete_field_index(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (name, _field_name) = path.into_inner();
    
    if storage.get_collection(&name).is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "status": { "error": "Collection not found" }
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": {
            "operation_id": 0,
            "status": "completed"
        }
    })))
}

/// Recommend points based on positive/negative examples
/// Uses average vector strategy: query = 2*avg(positive) - avg(negative)
/// This creates a single query vector that moves toward positive examples
/// and away from negative examples, then performs one efficient search.
#[derive(Deserialize)]
struct RecommendRequest {
    #[serde(default)]
    positive: Vec<serde_json::Value>,
    #[serde(default)]
    negative: Vec<serde_json::Value>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    with_payload: Option<bool>,
    #[serde(default)]
    with_vector: Option<bool>,
    #[serde(default)]
    score_threshold: Option<f32>,
}

async fn recommend_points(
    storage: web::Data<Arc<StorageManager>>,
    path: web::Path<String>,
    req: web::Json<RecommendRequest>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    
    let collection = match storage.get_collection(&name) {
        Some(c) => c,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "status": { "error": "Collection not found" }
            })));
        }
    };

    let limit = req.limit.unwrap_or(10);
    let with_payload = req.with_payload.unwrap_or(true);
    let with_vector = req.with_vector.unwrap_or(false);
    let score_threshold = req.score_threshold;
    
    // Collect point IDs to exclude from results
    let mut exclude_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    // Helper to parse point ID from JSON
    let parse_id = |id: &serde_json::Value| -> Option<String> {
        match id {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    };
    
    // Collect positive vectors and compute average
    let mut positive_vectors: Vec<Vec<f32>> = Vec::new();
    for pos_id in &req.positive {
        if let Some(id_str) = parse_id(pos_id) {
            exclude_ids.insert(id_str.clone());
            if let Some(point) = collection.get(&id_str) {
                positive_vectors.push(point.vector.as_slice().to_vec());
            }
        }
    }
    
    if positive_vectors.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "status": { "error": "At least one valid positive example is required" }
        })));
    }
    
    // Collect negative vectors and compute average
    let mut negative_vectors: Vec<Vec<f32>> = Vec::new();
    for neg_id in &req.negative {
        if let Some(id_str) = parse_id(neg_id) {
            exclude_ids.insert(id_str.clone());
            if let Some(point) = collection.get(&id_str) {
                negative_vectors.push(point.vector.as_slice().to_vec());
            }
        }
    }
    
    // Compute average of positive vectors
    let dim = positive_vectors[0].len();
    let mut avg_positive = vec![0.0f32; dim];
    for vec in &positive_vectors {
        for (i, &val) in vec.iter().enumerate() {
            if i < dim {
                avg_positive[i] += val;
            }
        }
    }
    let pos_count = positive_vectors.len() as f32;
    for val in &mut avg_positive {
        *val /= pos_count;
    }
    
    // Create query vector: if negatives exist, use formula 2*avg_pos - avg_neg
    // This moves the query toward positives and away from negatives
    let query_vector = if !negative_vectors.is_empty() {
        let mut avg_negative = vec![0.0f32; dim];
        for vec in &negative_vectors {
            for (i, &val) in vec.iter().enumerate() {
                if i < dim {
                    avg_negative[i] += val;
                }
            }
        }
        let neg_count = negative_vectors.len() as f32;
        for val in &mut avg_negative {
            *val /= neg_count;
        }
        
        // Combined query: 2*avg_pos - avg_neg
        avg_positive.iter()
            .zip(avg_negative.iter())
            .map(|(&pos, &neg)| 2.0 * pos - neg)
            .collect::<Vec<f32>>()
    } else {
        avg_positive
    };
    
    // Perform single search with combined query vector
    let query = Vector::new(query_vector);
    
    // Request more results to account for excluded IDs
    let search_limit = limit + exclude_ids.len();
    let search_results = collection.search(&query, search_limit, None);
    
    // Build results, excluding input point IDs
    let mut results = Vec::with_capacity(limit);
    for (point, score) in search_results {
        // Skip excluded points (the positive/negative examples)
        let point_id_str = point.id.to_string();
        if exclude_ids.contains(&point_id_str) {
            continue;
        }
        
        // Apply score threshold if provided
        if let Some(threshold) = score_threshold {
            if score < threshold {
                continue;
            }
        }
        
        let mut result = serde_json::json!({
            "id": match &point.id {
                distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                distx_core::PointId::Integer(i) => serde_json::Value::Number((*i).into()),
                distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
            },
            "version": 0,
            "score": score
        });
        
        if with_payload {
            result["payload"] = point.payload.clone().unwrap_or(serde_json::Value::Null);
        }
        if with_vector {
            result["vector"] = serde_json::json!(point.vector.as_slice());
        }
        
        results.push(result);
        
        if results.len() >= limit {
            break;
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "result": results
    })))
}

