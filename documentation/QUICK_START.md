# Quick Start Guide

This guide will help you get DistX up and running quickly.

## Prerequisites

### Install Rust

If you don't have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify installation:

```bash
rustc --version
cargo --version
```

### Install LMDB

DistX uses LMDB for fast persistence. Install it based on your platform:

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get update
sudo apt-get install liblmdb-dev
```

**macOS:**
```bash
brew install lmdb
```

**Other platforms:** See [LMDB documentation](https://symas.com/lmdb/) for installation instructions.

## Building DistX

### Clone and Build

```bash
git clone https://github.com/antonellof/DistX.git
cd distx
cargo build --release
```

The release binary will be at `target/release/distx`.

### Build Options

For faster builds during development:

```bash
cargo build
```

For optimized production builds:

```bash
cargo build --release
```

## Running DistX

### Start the Server

Basic usage (uses default ports and data directory):

```bash
./target/release/distx
```

With custom options:

```bash
./target/release/distx \
  --data-dir ./data \
  --http-port 6333 \
  --grpc-port 6334 \
  --log-level info
```

### Verify Server is Running

Check if the server is responding:

```bash
curl http://localhost:6333/collections
```

You should see an empty list: `[]`

## Basic Usage

### 1. Create a Collection

A collection is a named group of vectors with the same dimension and distance metric.

```bash
curl -X PUT http://localhost:6333/collections/products \
  -H "Content-Type: application/json" \
  -d '{
    "vectors": {
      "size": 128,
      "distance": "Cosine"
    }
  }'
```

**Response:**
```json
{"result": true}
```

**Distance Metrics:**
- `Cosine` - Cosine similarity (default, good for normalized vectors)
- `Euclidean` - L2 distance
- `Dot` - Dot product

### 2. Insert Points

Insert vectors with optional metadata (payload):

```bash
curl -X PUT http://localhost:6333/collections/products/points \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": "product1",
        "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
        "payload": {
          "name": "Laptop",
          "price": 999.99,
          "category": "electronics"
        }
      },
      {
        "id": "product2",
        "vector": [0.2, 0.3, 0.4, 0.5, 0.6],
        "payload": {
          "name": "Mouse",
          "price": 29.99,
          "category": "electronics"
        }
      }
    ]
  }'
```

**Batch Insert:** You can insert multiple points in a single request for better performance.

### 3. Search for Similar Vectors

Find the most similar vectors to a query vector:

```bash
curl -X POST http://localhost:6333/collections/products/points/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.15, 0.25, 0.35, 0.45, 0.55],
    "limit": 5
  }'
```

**Response:**
```json
{
  "result": [
    {
      "id": "product1",
      "score": 0.95,
      "payload": {
        "name": "Laptop",
        "price": 999.99,
        "category": "electronics"
      }
    },
    {
      "id": "product2",
      "score": 0.87,
      "payload": {
        "name": "Mouse",
        "price": 29.99,
        "category": "electronics"
      }
    }
  ]
}
```

### 4. Search with Filters

Filter results by payload conditions:

```bash
curl -X POST http://localhost:6333/collections/products/points/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.15, 0.25, 0.35, 0.45, 0.55],
    "limit": 5,
    "filter": {
      "field": "category",
      "operator": "eq",
      "value": "electronics"
    }
  }'
```

### 5. Get a Point by ID

Retrieve a specific point:

```bash
curl http://localhost:6333/collections/products/points/product1
```

**Response:**
```json
{
  "result": {
    "id": "product1",
    "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
    "payload": {
      "name": "Laptop",
      "price": 999.99,
      "category": "electronics"
    }
  }
}
```

### 6. Delete a Point

Remove a point from a collection:

```bash
curl -X DELETE http://localhost:6333/collections/products/points/product1
```

**Response:**
```json
{"result": true}
```

### 7. Get Collection Info

View collection details:

```bash
curl http://localhost:6333/collections/products
```

**Response:**
```json
{
  "name": "products",
  "vectors": {
    "size": 128,
    "distance": "Cosine"
  },
  "points_count": 2
}
```

### 8. Delete a Collection

Remove an entire collection:

```bash
curl -X DELETE http://localhost:6333/collections/products
```

**Response:**
```json
{"result": true}
```

## Using gRPC API

For better performance, use the gRPC API. See [API Reference](API.md) for gRPC examples.

## Python Example

```python
import requests

# Create collection
requests.put(
    "http://localhost:6333/collections/products",
    json={"vectors": {"size": 128, "distance": "Cosine"}}
)

# Insert points
requests.put(
    "http://localhost:6333/collections/products/points",
    json={
        "points": [
            {
                "id": "product1",
                "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
                "payload": {"name": "Laptop", "price": 999.99}
            }
        ]
    }
)

# Search
response = requests.post(
    "http://localhost:6333/collections/products/points/search",
    json={
        "vector": [0.15, 0.25, 0.35, 0.45, 0.55],
        "limit": 5
    }
)
results = response.json()["result"]
print(results)
```

## Next Steps

- Read the [API Reference](API.md) for complete API documentation
- Check [Architecture](ARCHITECTURE.md) to understand how DistX works
- See [Performance](PERFORMANCE.md) for optimization tips
- Review [Persistence](PERSISTENCE.md) for data durability options
