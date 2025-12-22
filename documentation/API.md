# API Reference

vectX provides two APIs: REST (Qdrant-compatible) and gRPC (binary protocol). The gRPC API is recommended for production workloads due to better performance.

> **New Feature**: vectX includes a [Similarity Engine](SIMILARITY_ENGINE.md) for schema-driven similarity queries over tabular data. See the dedicated documentation for details.

## REST API

The REST API is compatible with Qdrant's API, making it easy to migrate from Qdrant to vectX.

### Base URL

```
http://localhost:6333
```

### Collection Management

#### List Collections

```bash
GET /collections
```

**Response**:
```json
{
  "result": {
    "collections": [
      {
        "name": "my_collection",
        "config": {
          "vectors": {
            "size": 128,
            "distance": "Cosine"
          }
        }
      }
    ]
  }
}
```

#### Get Collection Info

```bash
GET /collections/{collection_name}
```

#### Create Collection

```bash
PUT /collections/{collection_name}
Content-Type: application/json

{
  "vectors": {
    "size": 128,
    "distance": "Cosine"
  }
}
```

**Distance Types**:
- `Cosine` - Cosine similarity (vectors are normalized)
- `Euclidean` - L2 distance

#### Delete Collection

```bash
DELETE /collections/{collection_name}
```

### Point Operations

#### Upsert Points

```bash
PUT /collections/{collection_name}/points
Content-Type: application/json

{
  "points": [
    {
      "id": "point1",
      "vector": [0.1, 0.2, 0.3, ...],
      "payload": {
        "text": "example",
        "category": "A"
      }
    }
  ]
}
```

**Batch Insert**: Provide multiple points in the `points` array for optimized batch insertion.

#### Get Point

```bash
GET /collections/{collection_name}/points/{point_id}
```

#### Delete Point

```bash
DELETE /collections/{collection_name}/points/{point_id}
```

### Search

#### Vector Search

```bash
POST /collections/{collection_name}/points/search
Content-Type: application/json

{
  "vector": [0.1, 0.2, 0.3, ...],
  "limit": 10,
  "filter": {
    "must": [
      {
        "key": "category",
        "match": {
          "value": "A"
        }
      }
    ]
  }
}
```

**Response**:
```json
{
  "result": [
    {
      "id": "point1",
      "score": 0.95,
      "payload": {
        "text": "example"
      }
    }
  ]
}
```

#### Text Search (BM25)

```bash
POST /collections/{collection_name}/points/text_search
Content-Type: application/json

{
  "query": "example text",
  "limit": 10
}
```

## gRPC API

The gRPC API uses a binary protocol for better performance. It's recommended for production workloads.

### Connection

```python
import grpc
from vectx_pb2_grpc import vectXStub

channel = grpc.insecure_channel('localhost:6334')
stub = vectXStub(channel)
```

### Create Collection

```python
from vectx_pb2 import CreateCollectionRequest, VectorConfig

request = CreateCollectionRequest(
    name="my_collection",
    vector_config=VectorConfig(
        size=128,
        distance="Cosine"
    )
)
stub.CreateCollection(request)
```

### Upsert Points

```python
from vectx_pb2 import UpsertPointsRequest, Point, PointId, Vector

request = UpsertPointsRequest(
    collection="my_collection",
    points=[
        Point(
            id=PointId(string="point1"),
            vector=Vector(values=[0.1, 0.2, 0.3, ...]),
            payload={"text": "example"}
        )
    ]
)
stub.UpsertPoints(request)
```

### Search Points

```python
from vectx_pb2 import SearchPointsRequest, Vector

request = SearchPointsRequest(
    collection="my_collection",
    query_vector=Vector(values=[0.1, 0.2, 0.3, ...]),
    limit=10
)
response = stub.SearchPoints(request)
for result in response.results:
    print(f"ID: {result.id}, Score: {result.score}")
```

### Get Point

```python
from vectx_pb2 import GetPointRequest, PointId

request = GetPointRequest(
    collection="my_collection",
    id=PointId(string="point1")
)
response = stub.GetPoint(request)
```

### Delete Point

```python
from vectx_pb2 import DeletePointRequest, PointId

request = DeletePointRequest(
    collection="my_collection",
    id=PointId(string="point1")
)
stub.DeletePoint(request)
```

### List Collections

```python
from vectx_pb2 import ListCollectionsRequest

request = ListCollectionsRequest()
response = stub.ListCollections(request)
for collection in response.collections:
    print(f"Collection: {collection.name}")
```

## Protocol Buffer Definitions

The gRPC API uses Protocol Buffers. See `lib/api/proto/vectx.proto` for the complete schema.

## Performance Comparison

| Operation | REST API | gRPC API | Recommendation |
|-----------|----------|----------|----------------|
| Insert | 12,100 ops/s | 61,980 ops/s | Use gRPC |
| Search | 850 ops/s | 2,850 ops/s | Use gRPC |

**Use gRPC API for production workloads to achieve best performance!**

