# Performance Benchmarks

## Overview

vectX delivers **comparable performance to Qdrant** while providing full API compatibility. Both systems offer high-performance vector search with SIMD optimizations.

## Test Environment

- **vectX**: v0.2.3, Docker deployment
- **Qdrant**: v1.16.2, Docker deployment
- **Redis Stack**: Latest, Docker deployment
- **Platform**: Apple Silicon / x86_64
- **Date**: 2025-12-18

---

## Performance Results

### Typical Performance (5,000 vectors, 128 dimensions)

| System | Insert ops/s | Search ops/s | Search p50 | Search p99 |
|--------|-------------|--------------|------------|------------|
| **vectX** | ~8,000 | ~400-500 | ~2ms | ~5ms |
| **Qdrant** | ~8,000 | ~400-500 | ~2ms | ~5ms |
| **Redis Stack** | ~5,000 | ~1,500 | ~0.6ms | ~2ms |

*Performance varies based on hardware, Docker overhead, and workload characteristics.*

---

## Key Optimizations

### HNSW Index
- **Bit Vector Visited Set**: O(1) lookup vs HashSet O(log n)
- **Contiguous Vector Storage**: Cache-friendly memory layout
- **CPU Prefetching**: Prefetch neighbor vectors
- **Adaptive Search**: Brute-force for small datasets (<10K vectors)

### SIMD Optimizations
- **AVX2 + FMA**: 16-wide operations on x86_64
- **SSE Fallback**: 4-wide for older processors
- **NEON**: 8-wide for ARM64 (Apple Silicon)

### Memory Efficiency
- **Parallel Search**: Rayon for large datasets (10K+)
- **Lazy Cloning**: Only clone top-k results
- **Optimized Hot Path**: Minimal branching for common cases

---

## When to Use vectX

| Use Case | vectX Advantage |
|----------|-----------------|
| **Qdrant Compatibility** | Drop-in replacement with same API |
| **Lightweight Deployment** | Single ~6MB binary |
| **Embedded Use** | Can be used as a Rust library |
| **Resource Constrained** | Lower memory footprint |

---

## Benchmark Commands

```bash
# Run full comparison (vectX vs Qdrant vs Redis)
python3 scripts/full_comparison.py

# With custom parameters
python3 scripts/full_comparison.py --vectors 10000 --searches 1000
```

### Docker Setup

```bash
# Start vectX
docker run -d --name vectx -p 6333:6333 -p 6334:6334 antonellofratepietro/vectx

# Start Qdrant (on different port for comparison)
docker run -d --name qdrant -p 16333:6333 -p 16334:6334 qdrant/qdrant

# Start Redis Stack
docker run -d --name redis -p 6379:6379 redis/redis-stack-server
```

---

## Notes

- Performance is comparable between vectX and Qdrant
- Redis Stack shows higher search throughput due to in-memory architecture
- Results may vary based on:
  - Hardware (CPU, memory)
  - Docker overhead (especially on macOS)
  - Dataset size and vector dimensions
  - Query complexity (filters, payloads)

For production deployments, we recommend running your own benchmarks with representative workloads.
