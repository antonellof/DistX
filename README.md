# DistX

[![Crates.io](https://img.shields.io/crates/v/distx.svg)](https://crates.io/crates/distx)
[![Documentation](https://docs.rs/distx/badge.svg)](https://docs.rs/distx)
[![Docker](https://img.shields.io/docker/v/distx/distx?label=docker)](https://hub.docker.com/r/distx/distx)
[![License](https://img.shields.io/crates/l/distx.svg)](https://github.com/antonellof/DistX#license)

A fast, in-memory vector database written in Rust. Designed with Redis-style simplicity and full Qdrant API compatibility.

## Performance

DistX delivers **comparable performance to Qdrant** with the following characteristics:

| Metric | Typical Performance |
|--------|---------------------|
| **Insert** | ~8,000 ops/sec |
| **Search** | ~400-500 ops/sec |
| **Search Latency (p50)** | ~2ms |
| **Search Latency (p99)** | ~5ms |

*Benchmarks: 5,000 vectors, 128 dimensions, Cosine distance, Docker deployment*

See [Performance Benchmarks](documentation/PERFORMANCE.md) for detailed results.

## Quick Start with Docker

The fastest way to get started - no compilation required:

```bash
# Pull and run
docker pull distx/distx:latest
docker run -p 6333:6333 -p 6334:6334 \
    -v "$(pwd)/distx_storage:/qdrant/storage" \
    distx/distx:latest
```

Or with Docker Compose:

```bash
docker-compose up -d
```

DistX is now accessible at:
- **REST API**: http://localhost:6333 (Qdrant-compatible)
- **Web UI**: http://localhost:6333/dashboard
- **gRPC API**: localhost:6334

See [Docker Guide](documentation/DOCKER.md) for more options.

## Installation

### From crates.io

```bash
cargo install distx
```

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
distx = "0.1.0"

# Or individual components:
distx-core = "0.1.0"      # Core data structures (Vector, HNSW, BM25)
distx-storage = "0.1.0"   # Persistence layer (WAL, snapshots)
distx-api = "0.1.0"       # REST and gRPC APIs
```

### From Source

```bash
git clone https://github.com/antonellof/DistX.git
cd DistX
cargo build --release
```

### Prerequisites

- **Rust 1.75+**: Install from [rustup.rs](https://rustup.rs/)
- **LMDB** (for persistence):
  - Linux: `sudo apt-get install liblmdb-dev`
  - macOS: `brew install lmdb`

## Quick Start

### Run the Server

```bash
# Using cargo install
distx

# Or from source
./target/release/distx

# With custom options
distx --data-dir ./data --http-port 6333 --grpc-port 6334
```

The server will start and listen on:
- **REST API**: `http://localhost:6333` (Qdrant-compatible)
- **Web UI**: `http://localhost:6333/dashboard`
- **gRPC API**: `localhost:6334`

### Create a Collection

```bash
curl -X PUT http://localhost:6333/collections/my_collection \
  -H "Content-Type: application/json" \
  -d '{
    "vectors": {
      "size": 128,
      "distance": "Cosine"
    }
  }'
```

### Insert Vectors

```bash
curl -X PUT http://localhost:6333/collections/my_collection/points \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": "point1",
        "vector": [0.1, 0.2, 0.3, ...],
        "payload": {"text": "example document"}
      }
    ]
  }'
```

### Search Vectors

```bash
curl -X POST http://localhost:6333/collections/my_collection/points/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.1, 0.2, 0.3, ...],
    "limit": 10
  }'
```

## Features

- **Fast Vector Search**: HNSW index with SIMD optimizations (AVX2, SSE, NEON)
- **Text Search**: BM25 full-text search with ranking
- **Payload Filtering**: Filter results by JSON metadata
- **Dual API**: REST (Qdrant-compatible) and gRPC
- **Web Dashboard**: Uses Qdrant's fully compatible dashboard UI
- **Docker Ready**: Single command deployment
- **Persistence**: Redis-style snapshots, WAL, and LMDB storage
- **Lightweight**: Single ~6MB binary

## Configuration

```bash
distx [OPTIONS]

Options:
  --data-dir <PATH>      Data directory (default: ./data)
  --http-port <PORT>     HTTP API port (default: 6333)
  --grpc-port <PORT>     gRPC API port (default: 6334)
  --log-level <LEVEL>    Log level: trace, debug, info, warn, error (default: info)
```

## Architecture

```
distx/
├── lib/
│   ├── core/          # Core data structures (Vector, Point, Collection, HNSW)
│   ├── storage/       # Persistence layer (WAL, snapshots, LMDB)
│   └── api/           # REST and gRPC APIs
└── src/
    └── main.rs        # Main entry point
```

## Web Dashboard

DistX uses [Qdrant's Web UI](https://github.com/qdrant/qdrant-web-ui), which is fully compatible thanks to our Qdrant API implementation:

![Dashboard](documentation/dashboard-preview.png)

Access it at http://localhost:6333/dashboard

**Features:**
- View all collections and their statistics
- Create and delete collections
- Browse points with pagination
- Interactive API console for testing
- Full compatibility with Qdrant's dashboard features

## Documentation

- [Quick Start Guide](documentation/QUICK_START.md) - Get started quickly
- [Docker Deployment](documentation/DOCKER.md) - Docker and docker-compose guide
- [Architecture](documentation/ARCHITECTURE.md) - System design
- [API Reference](documentation/API.md) - REST and gRPC API docs
- [Performance](documentation/PERFORMANCE.md) - Benchmarks and optimizations

## Links

- **Crates.io**: https://crates.io/crates/distx
- **Documentation**: https://docs.rs/distx
- **GitHub**: https://github.com/antonellof/DistX

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
