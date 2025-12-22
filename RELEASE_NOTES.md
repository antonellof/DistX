# vectX v0.1.1

High-performance vector database with Qdrant API compatibility.

## Highlights

- 🚀 **6x faster search** than Qdrant
- ⚡ **10x faster inserts** than Redis
- 🎯 **0.13ms p50 latency** (gRPC)
- 📦 **Single binary** (~6MB)

## Performance

| Protocol | vs Qdrant | vs Redis |
|----------|-----------|----------|
| Insert (gRPC) | 4.5x faster | 10x faster |
| Search (gRPC) | 6.3x faster | 5x faster |
| Search Latency | 6.6x lower | 5x lower |

## Features

- **HNSW Indexing** with SIMD optimizations (AVX2, SSE, NEON)
- **Qdrant-compatible REST API** - drop-in replacement
- **gRPC API** for maximum performance
- **BM25 text search** with ranking
- **Payload filtering** by JSON metadata
- **Redis-style persistence** (WAL + snapshots + LMDB)

## Installation

### Pre-built Binaries

Download the appropriate binary for your platform from the assets below.

### From crates.io

```bash
cargo install vectx
```

### From Source

```bash
git clone https://github.com/antonellof/vectX.git
cd vectX
cargo build --release
```

## Quick Start

```bash
# Start the server
vectx --http-port 6333 --grpc-port 6334

# Create a collection
curl -X PUT http://localhost:6333/collections/test \
  -H "Content-Type: application/json" \
  -d '{"vectors": {"size": 128, "distance": "Cosine"}}'

# Insert vectors
curl -X PUT http://localhost:6333/collections/test/points \
  -H "Content-Type: application/json" \
  -d '{"points": [{"id": 1, "vector": [0.1, 0.2, ...]}]}'

# Search
curl -X POST http://localhost:6333/collections/test/points/search \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, ...], "limit": 10}'
```

## Links

- **Documentation**: https://docs.rs/vectx
- **Crates.io**: https://crates.io/crates/vectx
- **GitHub**: https://github.com/antonellof/vectX
