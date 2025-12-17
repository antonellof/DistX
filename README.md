# DistX

A simple, fast, in-memory vector database written in Rust. Designed with Redis-style simplicity and Qdrant API compatibility.

DistX combines the best of both worlds: **Redis's simple architecture and performance** with **Qdrant's easy-to-use API**. It's faster than Redis (26x faster inserts, 1.35x faster searches) while maintaining the same in-memory, fork-based persistence model. The Qdrant-compatible REST API makes it a drop-in replacement for existing applications.

## What is DistX?

DistX is an in-memory vector database that provides fast similarity search using HNSW indexing. It offers a simple API compatible with Qdrant, making it easy to migrate existing applications.

## Features

- **Fast Vector Search**: HNSW index for approximate nearest neighbor search
- **Text Search**: BM25 full-text search with ranking
- **Payload Filtering**: Filter results by JSON metadata
- **Graph Support**: Basic nodes and edges
- **Dual API**: REST (Qdrant-compatible) and gRPC APIs
- **Persistence**: Redis-style snapshots, WAL, and LMDB storage
- **In-Memory First**: Optimized for speed with optional persistence

## Getting Started

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **LMDB** (for persistence):
  - Linux: `sudo apt-get install liblmdb-dev`
  - macOS: `brew install lmdb`

### Build from Source

```bash
git clone https://github.com/antonellof/DistX.git
cd distx
cargo build --release
```

The binary will be at `target/release/distx`.

### Run the Server

Start DistX with default settings:

```bash
./target/release/distx
```

Or with custom options:

```bash
./target/release/distx --data-dir ./data --http-port 6333 --grpc-port 6334
```

The server will start and listen on:
- HTTP API: `http://localhost:6333`
- gRPC API: `localhost:6334`

### Quick Example

#### 1. Create a Collection

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

#### 2. Insert Vectors

```bash
curl -X PUT http://localhost:6333/collections/my_collection/points \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": "point1",
        "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
        "payload": {"text": "example document"}
      },
      {
        "id": "point2",
        "vector": [0.2, 0.3, 0.4, 0.5, 0.6],
        "payload": {"text": "another document"}
      }
    ]
  }'
```

#### 3. Search Vectors

```bash
curl -X POST http://localhost:6333/collections/my_collection/points/search \
  -H "Content-Type: application/json" \
  -d '{
    "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
    "limit": 10
  }'
```

#### 4. Get a Point

```bash
curl http://localhost:6333/collections/my_collection/points/point1
```

#### 5. Delete a Point

```bash
curl -X DELETE http://localhost:6333/collections/my_collection/points/point1
```

For more examples and detailed API documentation, see [Quick Start Guide](documentation/QUICK_START.md).

## Configuration

Command line options:

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

## Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration_test

# Run benchmarks
cargo bench
```

## Documentation

- [Quick Start Guide](documentation/QUICK_START.md) - Get started quickly with examples
- [Architecture](documentation/ARCHITECTURE.md) - System design and components
- [API Reference](documentation/API.md) - REST and gRPC API documentation
- [Performance](documentation/PERFORMANCE.md) - Benchmarks and optimization details

## License

DistX is licensed under the [GNU General Public License v3.0](https://github.com/antonellof/DistX/blob/main/LICENSE).
