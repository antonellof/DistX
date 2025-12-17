# DistX Architecture

## Overview

DistX is designed as a simple, fast, in-memory vector database with Redis-style simplicity and Qdrant API compatibility. The architecture follows these principles:

1. **Simplicity First**: Easy to understand and maintain
2. **Performance**: In-memory operations with optional persistence
3. **Modularity**: Clean separation of concerns
4. **Compatibility**: Qdrant API for easy migration

## Project Structure

```
distx/
├── Cargo.toml              # Workspace configuration
├── src/
│   └── main.rs             # Main entry point, server initialization
├── lib/
│   ├── core/               # Core data structures
│   │   ├── vector.rs      # Vector operations (cosine, L2, normalization)
│   │   ├── point.rs       # Point with ID, vector, and payload
│   │   ├── collection.rs  # Collection management
│   │   ├── hnsw.rs        # HNSW index (simple implementation)
│   │   └── error.rs       # Error types
│   ├── storage/            # Persistence layer
│   │   ├── manager.rs     # Collection storage manager
│   │   └── wal.rs         # Write-Ahead Log
│   └── api/                # API layer
│       ├── rest.rs         # REST API (Qdrant-compatible)
│       └── grpc.rs         # gRPC API (placeholder)
└── README.md
```

## Core Components

### 1. Core Library (`lib/core`)

**Vector (`vector.rs`)**
- Basic vector operations (cosine similarity, L2 distance)
- Normalization support
- Dimension validation

**Point (`point.rs`)**
- Point ID (String, UUID, or Integer)
- Vector data
- Optional JSON payload

**Collection (`collection.rs`)**
- Collection configuration (name, vector dimension, distance metric)
- Point insertion/update/deletion
- Similarity search (currently linear, will use HNSW)
- Thread-safe with `parking_lot::RwLock`

**HNSW Index (`hnsw.rs`)**
- Simple HNSW implementation
- Currently uses linear search (to be enhanced)
- Multi-layer graph structure

### 2. Storage Layer (`lib/storage`)

**StorageManager (`manager.rs`)**
- Manages collections
- Creates/deletes collections
- Provides access to collections

**Write-Ahead Log (`wal.rs`)**
- Redis-style WAL for durability
- Append-only log file
- Flush support

### 3. API Layer (`lib/api`)

**REST API (`rest.rs`)**
- Qdrant-compatible endpoints:
  - `GET /collections` - List collections
  - `GET /collections/{name}` - Get collection info
  - `PUT /collections/{name}` - Create collection
  - `DELETE /collections/{name}` - Delete collection
  - `PUT /collections/{name}/points` - Upsert points
  - `POST /collections/{name}/points/search` - Search points

**gRPC API (`grpc.rs`)**
- Placeholder for Qdrant-compatible gRPC
- Will use Qdrant's proto files

## Data Flow

### Insert Operation
1. Client sends PUT request to `/collections/{name}/points`
2. REST API validates request
3. StorageManager retrieves collection
4. Collection validates vector dimension
5. Point is inserted into collection
6. (Future: WAL entry written)

### Search Operation
1. Client sends POST request to `/collections/{name}/points/search`
2. REST API validates request
3. StorageManager retrieves collection
4. Collection performs similarity search
5. Results sorted by score and limited
6. Response sent to client

## Design Decisions

### Why In-Memory First?
- Maximum performance for read/write operations
- Simple implementation
- Can add persistence later without changing API

### Why Qdrant Compatibility?
- Easy migration from existing Qdrant deployments
- Familiar API for users
- Can leverage existing client libraries

### Why Modular Structure?
- Easy to test individual components
- Can swap implementations (e.g., different index types)
- Clear separation of concerns

## Future Enhancements

1. **HNSW Implementation**: Proper multi-layer graph with efficient search
2. **Persistence**: WAL replay, snapshots, RDB-style persistence
3. **Replication**: Master-replica setup like Redis
4. **Quantization**: Vector quantization for memory efficiency
5. **Filtering**: Payload-based filtering during search
6. **gRPC**: Full Qdrant gRPC protocol support

## Performance Considerations

- **Current**: Linear search O(N) - suitable for small collections
- **Future**: HNSW search O(log N) - suitable for large collections
- **Memory**: All vectors in memory for fast access
- **Concurrency**: Read-write locks for thread safety

## Comparison with Redis Vector Sets

Similarities:
- In-memory first
- Simple command-based API
- Fast operations
- Optional persistence

Differences:
- REST API instead of Redis protocol
- Qdrant-compatible endpoints
- Written in Rust (memory safety)

## Comparison with Qdrant

Similarities:
- REST API compatibility
- Collection-based organization
- Vector similarity search
- Payload support

Differences:
- Simpler implementation
- In-memory first (Qdrant has more persistence options)
- Smaller codebase
- Redis-style simplicity

