use std::sync::Arc;
use tonic::{Request, Response, Status};
use distx_storage::StorageManager;
use distx_core::{Point, PointId, Vector, Distance};

pub mod distx {
    tonic::include_proto!("distx");
}

use distx::{
    dist_x_server::{DistX, DistXServer},
    UpsertPointsRequest, UpsertPointsResponse,
    SearchPointsRequest, SearchPointsResponse,
    GetPointRequest, GetPointResponse,
    DeletePointRequest, DeletePointResponse,
    CreateCollectionRequest, CreateCollectionResponse,
    ListCollectionsRequest, ListCollectionsResponse,
    Point as ProtoPoint, SearchResult as ProtoSearchResult,
};

pub struct GrpcApi {
    storage: Arc<StorageManager>,
}

#[tonic::async_trait]
impl DistX for GrpcApi {
    async fn upsert_points(
        &self,
        request: Request<UpsertPointsRequest>,
    ) -> Result<Response<UpsertPointsResponse>, Status> {
        let req = request.into_inner();
        let storage = &self.storage;
        
        let collection = storage.get_collection(&req.collection_name)
            .ok_or_else(|| Status::not_found("Collection not found"))?;

        let points: Result<Vec<Point>, Status> = req.points.into_iter().map(|p| {
            let id = match p.id {
                Some(distx::point::Id::IdString(s)) => PointId::String(s),
                Some(distx::point::Id::IdInteger(i)) => PointId::Integer(i),
                None => return Err(Status::invalid_argument("Point ID required")),
            };
            
            let vector = Vector::new(p.vector);
            let payload = if p.payload.is_empty() {
                None
            } else {
                Some(p.payload.into_iter().map(|(k, v)| (k, serde_json::Value::String(v))).collect())
            };
            
            Ok(Point::new(id, vector, payload))
        }).collect();

        let points = points?;
        
        const PREWARM_THRESHOLD: usize = 1000;
        let should_prewarm = points.len() >= PREWARM_THRESHOLD;
        
        if should_prewarm {
            collection.batch_upsert_with_prewarm(points, true)
                .map_err(|e| Status::internal(e.to_string()))?;
        } else {
            collection.batch_upsert(points)
                .map_err(|e| Status::internal(e.to_string()))?;
        }

        Ok(Response::new(UpsertPointsResponse {
            success: true,
            points_count: collection.count() as u32,
        }))
    }

    async fn search_points(
        &self,
        request: Request<SearchPointsRequest>,
    ) -> Result<Response<SearchPointsResponse>, Status> {
        let req = request.into_inner();
        let storage = &self.storage;
        
        let collection = storage.get_collection(&req.collection_name)
            .ok_or_else(|| Status::not_found("Collection not found"))?;

        let query = Vector::new(req.vector);
        let limit = req.limit as usize;
        
        let results = collection.search(&query, limit, None);
        
        let proto_results: Vec<ProtoSearchResult> = results.into_iter().map(|(point, score)| {
            let id = match point.id {
                PointId::String(s) => distx::PointId { id: Some(distx::point_id::Id::IdString(s)) },
                PointId::Integer(i) => distx::PointId { id: Some(distx::point_id::Id::IdInteger(i)) },
                PointId::Uuid(u) => distx::PointId { id: Some(distx::point_id::Id::IdString(u.to_string())) },
            };
            
            let payload = point.payload.as_ref()
                .and_then(|p| p.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            
            ProtoSearchResult {
                id: Some(id),
                score,
                payload,
            }
        }).collect();

        Ok(Response::new(SearchPointsResponse {
            results: proto_results,
        }))
    }

    async fn get_point(
        &self,
        request: Request<GetPointRequest>,
    ) -> Result<Response<GetPointResponse>, Status> {
        let req = request.into_inner();
        let storage = &self.storage;
        
        let collection = storage.get_collection(&req.collection_name)
            .ok_or_else(|| Status::not_found("Collection not found"))?;

        let id_str = match req.id {
            Some(distx::get_point_request::Id::IdString(s)) => s,
            Some(distx::get_point_request::Id::IdInteger(i)) => i.to_string(),
            None => return Err(Status::invalid_argument("Point ID required")),
        };

        if let Some(point) = collection.get(&id_str) {
            let point_id = match point.id {
                PointId::String(s) => distx::point::Id::IdString(s),
                PointId::Integer(i) => distx::point::Id::IdInteger(i),
                PointId::Uuid(u) => distx::point::Id::IdString(u.to_string()),
            };
            
            let proto_point = ProtoPoint {
                id: Some(point_id),
                vector: point.vector.as_slice().to_vec(),
                payload: point.payload.as_ref()
                    .and_then(|p| p.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default(),
            };
            
            Ok(Response::new(GetPointResponse {
                point: Some(proto_point),
                found: true,
            }))
        } else {
            Ok(Response::new(GetPointResponse {
                point: None,
                found: false,
            }))
        }
    }

    async fn delete_point(
        &self,
        request: Request<DeletePointRequest>,
    ) -> Result<Response<DeletePointResponse>, Status> {
        let req = request.into_inner();
        let storage = &self.storage;
        
        let collection = storage.get_collection(&req.collection_name)
            .ok_or_else(|| Status::not_found("Collection not found"))?;

        let id_str = match req.id {
            Some(distx::delete_point_request::Id::IdString(s)) => s,
            Some(distx::delete_point_request::Id::IdInteger(i)) => i.to_string(),
            None => return Err(Status::invalid_argument("Point ID required")),
        };

        collection.delete(&id_str)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DeletePointResponse { success: true }))
    }

    async fn create_collection(
        &self,
        request: Request<CreateCollectionRequest>,
    ) -> Result<Response<CreateCollectionResponse>, Status> {
        let req = request.into_inner();
        let storage = &self.storage;
        
        let distance = match req.distance.as_str() {
            "Cosine" => Distance::Cosine,
            "Euclidean" => Distance::Euclidean,
            "Dot" => Distance::Dot,
            _ => Distance::Cosine,
        };

        let config = distx_core::CollectionConfig {
            name: req.name,
            vector_dim: req.vector_dim as usize,
            distance,
            use_hnsw: true,
            enable_bm25: false,
        };

        storage.create_collection(config)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateCollectionResponse { success: true }))
    }

    async fn list_collections(
        &self,
        _request: Request<ListCollectionsRequest>,
    ) -> Result<Response<ListCollectionsResponse>, Status> {
        let storage = &self.storage;
        let collections = storage.list_collections();
        
        Ok(Response::new(ListCollectionsResponse {
            collections,
        }))
    }
}

impl GrpcApi {
    pub fn new(storage: Arc<StorageManager>) -> Self {
        Self { storage }
    }
    
    pub async fn start(
        storage: Arc<StorageManager>,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("0.0.0.0:{}", port).parse()?;
        
        let service = DistXServer::new(GrpcApi::new(storage));
        
        tonic::transport::Server::builder()
            .add_service(service)
            .serve(addr)
            .await?;
        
        Ok(())
    }
}
