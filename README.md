# vectX Vector Database

[![Crates.io](https://img.shields.io/crates/v/vectx.svg)](https://crates.io/crates/vectx)
[![Docker](https://img.shields.io/docker/v/antonellofratepietro/vectx?label=docker)](https://hub.docker.com/r/antonellofratepietro/vectx)
[![License](https://img.shields.io/crates/l/vectx.svg)](https://github.com/antonellof/vectX#license)

A fast, in-memory vector database with **100% Qdrant API compatibility**.

vectX is a drop-in replacement for Qdrant, providing high-performance vector similarity search with full API compatibility.

---

## Quick Start

```bash
# Run vectX
docker run -p 6333:6333 antonellofratepietro/vectx:latest

# Create collection
curl -X PUT localhost:6333/collections/products -d '{
  "vectors": {"size": 1536, "distance": "Cosine"}
}'

# Insert points
curl -X PUT localhost:6333/collections/products/points -d '{
  "points": [{
    "id": 1,
    "vector": [0.1, 0.2, 0.3, ...],
    "payload": {"name": "iPhone", "price": 999}
  }]
}'

# Search
curl -X POST localhost:6333/collections/products/points/search -d '{
  "vector": [0.1, 0.2, 0.3, ...],
  "limit": 10
}'
```

---

## Key Features

| Feature | Description |
|---------|-------------|
| **100% Qdrant Compatible** | Use existing Qdrant clients, drop-in replacement |
| **High Performance** | Fast in-memory vector search with HNSW indexing |
| **Full Vector DB** | Search, filter, facets, scroll, recommendations |
| **Persistence** | WAL and snapshot support for data durability |
| **REST & gRPC APIs** | Full Qdrant API compatibility |
| **Multi-vector Support** | Named vectors and sparse vectors |

---

## REST API

Full Qdrant API compatibility:

| Endpoint | Description |
|----------|-------------|
| `PUT /collections/{name}` | Create collection |
| `POST /collections/{name}/points/search` | Vector similarity search |
| `POST /collections/{name}/points/scroll` | Filter and browse with pagination |
| `POST /collections/{name}/points/recommend` | Recommendations from examples |
| `POST /collections/{name}/facet` | Aggregated counts by field |
| `PUT /collections/{name}/points` | Upsert points |
| `GET /collections/{name}/points` | Get points by IDs |
| `DELETE /collections/{name}/points` | Delete points |

---

## Examples

Example applications demonstrating vectX usage:

- [Data Chatbot](examples/data-chatbot/README.md) — Next.js demo with CSV upload and vector search
- [Fastest RAG Stack](examples/fastest-rag-stack/README.md) — RAG application with Streamlit UI

---

## Installation

```bash
# Docker
docker run -p 6333:6333 antonellofratepietro/vectx:latest

# Cargo
cargo install vectx

# Source
git clone https://github.com/antonellof/vectX.git && cd vectX && cargo run --release
```

---

## Use with Qdrant Clients

```typescript
import { QdrantClient } from '@qdrant/js-client-rest';
const client = new QdrantClient({ url: 'http://localhost:6333' });

// All standard Qdrant operations work
await client.search('products', { vector: [...], limit: 10 });
```

---

## Documentation

- [Quick Start](documentation/QUICK_START.md) — Get running in 5 minutes
- [API Reference](documentation/API.md) — REST and gRPC endpoints
- [Architecture](documentation/ARCHITECTURE.md) — System design and internals
- [Docker Guide](documentation/DOCKER.md) — Container deployment
- [Persistence](documentation/PERSISTENCE.md) — WAL, snapshots, storage
- [Performance](documentation/PERFORMANCE.md) — Benchmarks and tuning

## License

MIT OR Apache-2.0
