use actix_web::{web, App, HttpServer, HttpResponse, Result as ActixResult};
use actix_cors::Cors;
use distx_core::{CollectionConfig, Distance, Point, Vector, PayloadFilter, FilterCondition, Filter};
use distx_storage::StorageManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
struct CreateCollectionRequest {
    vectors: VectorConfig,
    #[serde(default)]
    use_hnsw: bool,
    #[serde(default)]
    enable_bm25: bool,
}

#[derive(Deserialize)]
struct VectorConfig {
    size: usize,
    distance: Option<String>,
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

#[derive(Deserialize)]
struct PointRequest {
    id: serde_json::Value,
    vector: Vec<f32>,
    payload: Option<serde_json::Value>,
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
        HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600);

            App::new()
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
                .route("/collections/{name}/points/search", web::post().to(search_points))
                .route("/collections/{name}/points/{id}", web::get().to(get_point))
                .route("/collections/{name}/points/{id}", web::delete().to(delete_point))
                .route("/collections/{name}/exists", web::get().to(collection_exists))
        })
        .bind(("0.0.0.0", port))?
        .run()
        .await
    }
}

async fn root_info() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "title": "DistX - Fast Vector Database",
        "version": "0.1.0"
    })))
}

async fn health_check() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "title": "DistX",
        "version": "0.1.0"
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
                        "indexing_threshold": 20000
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
    let distance = match req.vectors.distance.as_deref() {
        Some("Cosine") | Some("cosine") => Distance::Cosine,
        Some("Euclidean") | Some("euclidean") => Distance::Euclidean,
        Some("Dot") | Some("dot") => Distance::Dot,
        _ => Distance::Cosine,
    };

    let config = CollectionConfig {
        name: name.clone(),
        vector_dim: req.vectors.size,
        distance,
        use_hnsw: req.use_hnsw,
        enable_bm25: req.enable_bm25,
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

        let vector = Vector::new(point_req.vector.clone());
        Ok(Point::new(id, vector, point_req.payload.clone()))
    }).collect();

    match points {
        Ok(points_vec) => {
            if points_vec.len() > 1 {
                const PREWARM_THRESHOLD: usize = 1000;
                let should_prewarm = points_vec.len() >= PREWARM_THRESHOLD;
                
                if should_prewarm {
                    if let Err(e) = collection.batch_upsert_with_prewarm(points_vec, true) {
                        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                            "status": {
                                "error": e.to_string()
                            }
                        })));
                    }
                } else {
                    if let Err(e) = collection.batch_upsert(points_vec) {
                        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                            "status": {
                                "error": e.to_string()
                            }
                        })));
                    }
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
        Some(point) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "result": {
                "id": match &point.id {
                    distx_core::PointId::String(s) => serde_json::Value::String(s.clone()),
                    distx_core::PointId::Integer(i) => serde_json::Value::Number((*i).into()),
                    distx_core::PointId::Uuid(u) => serde_json::Value::String(u.to_string()),
                },
                "vector": point.vector.as_slice(),
                "payload": point.payload,
            }
        }))),
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

