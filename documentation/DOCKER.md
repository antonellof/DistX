# Docker Deployment Guide

This guide covers how to run vectX using Docker for quick deployment without compilation.

## Quick Start with Docker

### Pull and Run

```bash
# Pull the latest image
docker pull antonellofratepietro/vectx:latest

# Run vectX
docker run -p 6333:6333 -p 6334:6334 \
    -v "$(pwd)/vectx_storage:/qdrant/storage" \
    antonellofratepietro/vectx:latest
```

vectX is now accessible at:
- **REST API**: http://localhost:6333
- **Web UI**: http://localhost:6333/dashboard
- **gRPC API**: localhost:6334

### Build from Source

```bash
# Clone the repository
git clone https://github.com/antonellof/vectX.git
cd vectX

# Build the Docker image
docker build -t antonellofratepietro/vectx:latest .

# Run the container
docker run -p 6333:6333 -p 6334:6334 \
    -v "$(pwd)/vectx_storage:/qdrant/storage" \
    antonellofratepietro/vectx:latest
```

## Docker Compose

For production deployments, use Docker Compose:

```bash
# Start vectX
docker-compose up -d

# View logs
docker-compose logs -f

# Stop vectX
docker-compose down
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  vectx:
    image: antonellofratepietro/vectx:latest
    container_name: vectx
    ports:
      - "6333:6333"  # REST API
      - "6334:6334"  # gRPC API
    volumes:
      - vectx_storage:/qdrant/storage
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:6333/healthz"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  vectx_storage:
    driver: local
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (trace, debug, info, warn, error) | info |

### Command Line Options

You can pass command-line options to the container:

```bash
docker run -p 6333:6333 -p 6334:6334 \
    -v "$(pwd)/vectx_storage:/qdrant/storage" \
    antonellofratepietro/vectx:latest \
    vectx --data-dir /qdrant/storage --http-port 6333 --grpc-port 6334 --log-level debug
```

### Volume Mounts

| Container Path | Description |
|----------------|-------------|
| `/qdrant/storage` | Data persistence directory |
| `/qdrant/static` | Web dashboard static files |

## API Usage Examples

### REST API (Qdrant-compatible)

#### Check Health

```bash
curl http://localhost:6333/healthz
```

Response:
```json
{"title": "vectX", "version": "0.1.0"}
```

#### Create a Collection

```bash
curl -X PUT http://localhost:6333/collections/test_collection \
  -H "Content-Type: application/json" \
  -d '{
    "vectors": {
      "size": 4,
      "distance": "Cosine"
    }
  }'
```

Response:
```json
{"result": true}
```

#### Insert Points

```bash
curl -X PUT http://localhost:6333/collections/test_collection/points \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {"id": 1, "vector": [0.05, 0.61, 0.76, 0.74], "payload": {"city": "Berlin"}},
      {"id": 2, "vector": [0.19, 0.81, 0.75, 0.11], "payload": {"city": "London"}},
      {"id": 3, "vector": [0.36, 0.55, 0.47, 0.94], "payload": {"city": "Moscow"}}
    ]
  }'
```

Response:
```json
{"result": {"operation_id": 0, "status": "completed"}}
```

#### Search Vectors

```bash
curl -X POST http://localhost:6333/collections/test_collection/points/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.2, 0.1, 0.9, 0.7],
    "limit": 3
  }'
```

Response:
```json
{
  "result": [
    {"id": 3, "score": 1.208, "payload": {"city": "Moscow"}},
    {"id": 1, "score": 1.273, "payload": {"city": "Berlin"}},
    {"id": 2, "score": 0.871, "payload": {"city": "London"}}
  ]
}
```

#### List Collections

```bash
curl http://localhost:6333/collections
```

#### Get Collection Info

```bash
curl http://localhost:6333/collections/test_collection
```

#### Delete Collection

```bash
curl -X DELETE http://localhost:6333/collections/test_collection
```

### gRPC API

vectX provides a high-performance gRPC API on port 6334.

#### Using grpcurl

```bash
# List services
grpcurl -plaintext localhost:6334 list

# Create collection
grpcurl -plaintext -d '{
  "name": "test_collection",
  "vector_dim": 4,
  "distance": "Cosine"
}' localhost:6334 vectx.vectX/CreateCollection

# Upsert points
grpcurl -plaintext -d '{
  "collection_name": "test_collection",
  "points": [
    {"id_integer": 1, "vector": [0.1, 0.2, 0.3, 0.4], "payload": {"key": "value"}}
  ]
}' localhost:6334 vectx.vectX/UpsertPoints

# Search
grpcurl -plaintext -d '{
  "collection_name": "test_collection",
  "vector": [0.1, 0.2, 0.3, 0.4],
  "limit": 5
}' localhost:6334 vectx.vectX/SearchPoints

# List collections
grpcurl -plaintext localhost:6334 vectx.vectX/ListCollections
```

#### Python gRPC Client

```python
import grpc

# Generate Python code from proto file first:
# python -m grpc_tools.protoc -I. --python_out=. --grpc_python_out=. vectx.proto

import vectx_pb2
import vectx_pb2_grpc

# Connect to vectX
channel = grpc.insecure_channel('localhost:6334')
stub = vectx_pb2_grpc.vectXStub(channel)

# Create collection
response = stub.CreateCollection(vectx_pb2.CreateCollectionRequest(
    name="my_collection",
    vector_dim=128,
    distance="Cosine"
))
print(f"Created: {response.success}")

# Upsert points
points = [
    vectx_pb2.Point(
        id_integer=1,
        vector=[0.1] * 128,
        payload={"text": "example"}
    )
]
response = stub.UpsertPoints(vectx_pb2.UpsertPointsRequest(
    collection_name="my_collection",
    points=points
))
print(f"Upserted {response.points_count} points")

# Search
response = stub.SearchPoints(vectx_pb2.SearchPointsRequest(
    collection_name="my_collection",
    vector=[0.1] * 128,
    limit=10
))
for result in response.results:
    print(f"ID: {result.id}, Score: {result.score}")
```

## Web Dashboard

vectX includes a built-in web dashboard for visual management:

1. Open http://localhost:6333/dashboard in your browser
2. View collections, create new ones, and browse points
3. Use the API Console to execute REST requests

### Dashboard Features

- **Overview**: System statistics and quick start guide
- **Collections**: Create, view, and delete collections
- **Console**: Interactive API testing interface

## Production Considerations

### Resource Limits

```yaml
services:
  vectx:
    deploy:
      resources:
        limits:
          memory: 4G
          cpus: '2'
        reservations:
          memory: 512M
          cpus: '0.5'
```

### Persistence

Always mount a volume for data persistence:

```bash
docker run -v vectx_data:/qdrant/storage antonellofratepietro/vectx:latest
```

### Security

In production, consider:

1. Running behind a reverse proxy (nginx, traefik)
2. Adding TLS termination
3. Implementing authentication at the proxy level
4. Using Docker networks for isolation

### Health Checks

The container includes a health check that queries the `/healthz` endpoint every 30 seconds.

```bash
# Check container health
docker inspect --format='{{.State.Health.Status}}' vectx
```

## Troubleshooting

### Container won't start

```bash
# Check logs
docker logs vectx

# Run interactively for debugging
docker run -it --rm antonellofratepietro/vectx:latest /bin/bash
```

### Permission issues

```bash
# The container runs as user 'vectx' (UID 1000)
# Ensure mounted directories are accessible
chown -R 1000:1000 ./vectx_storage
```

### Port conflicts

```bash
# Use different ports
docker run -p 8080:6333 -p 8081:6334 antonellofratepietro/vectx:latest
```

## Next Steps

- Read the [API Reference](API.md) for complete endpoint documentation
- Check [Performance](PERFORMANCE.md) for optimization tips
- See [Quick Start](QUICK_START.md) for more usage examples
