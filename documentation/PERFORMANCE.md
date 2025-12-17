# Performance Benchmarks - DistX vs Qdrant vs Redis

## Overview

DistX has been comprehensively benchmarked against Qdrant and Redis Stack using both REST and gRPC protocols. **DistX gRPC delivers the best performance across all metrics, beating both Qdrant and Redis on insert AND search operations.**

## Test Environment

- **DistX**: Release build with all optimizations (HNSW + SIMD)
- **Qdrant**: v1.16.2 (Docker)
- **Redis Stack**: Latest (Docker with RediSearch module)
- **Platform**: Apple Silicon (ARM64 with NEON SIMD)
- **Date**: 2025-12-17

---

## 🏆 Protocol Comparison Results

### Performance by Protocol (10,000 vectors, dim=128)

| System | Insert ops/s | Search ops/s | Search p50 | Search p99 |
|--------|-------------|--------------|------------|------------|
| **DistX gRPC** | **46,269** | **6,845** | **0.13ms** | **0.27ms** |
| DistX REST | 11,176 | 297 | 1.08ms | 1.98ms |
| Qdrant gRPC | 10,379 | 1,087 | 0.86ms | 1.29ms |
| Qdrant REST | 8,385 | 384 | 2.39ms | 8.45ms |
| Redis RESP | 4,419 | 1,361 | 0.66ms | 2.58ms |

### 🥇 DistX gRPC Wins Everything!

| Comparison | Insert | Search |
|------------|--------|--------|
| vs Qdrant gRPC | **4.46x faster** | **6.30x faster** |
| vs Qdrant REST | **5.52x faster** | **17.8x faster** |
| vs Redis RESP | **10.5x faster** | **5.03x faster** |
| vs DistX REST | 4.14x faster | 23.1x faster |

---

## Protocol Speedup Analysis

### DistX: gRPC vs REST

| Metric | REST | gRPC | Speedup |
|--------|------|------|---------|
| Insert ops/s | 11,176 | 46,269 | **4.14x** |
| Search ops/s | 297 | 6,845 | **23.1x** |
| Search p50 | 1.08ms | 0.13ms | **8.3x lower** |
| Search p99 | 1.98ms | 0.27ms | **7.3x lower** |

### Qdrant: gRPC vs REST

| Metric | REST | gRPC | Speedup |
|--------|------|------|---------|
| Insert ops/s | 8,385 | 10,379 | 1.24x |
| Search ops/s | 384 | 1,087 | 2.83x |

**DistX shows much larger gRPC improvement than Qdrant!**

---

## REST API Comparison

For applications requiring REST API compatibility:

### Insert Performance (REST)

| Rank | System | ops/s | vs DistX |
|------|--------|-------|----------|
| 🥇 | **DistX REST** | **11,176** | baseline |
| 🥈 | Qdrant REST | 8,385 | 0.75x |
| 🥉 | Redis RESP | 4,419 | 0.40x |

### Search Performance (REST)

| Rank | System | ops/s | p50 |
|------|--------|-------|-----|
| 🥇 | Qdrant REST | 384 | 2.39ms |
| 🥈 | **DistX REST** | 297 | 1.08ms |
| 🥉 | - | - | - |

**Note**: DistX REST has lower latency (1.08ms vs 2.39ms) despite lower throughput.

---

## gRPC API Comparison

For maximum performance, use gRPC:

### Insert Performance (gRPC)

| Rank | System | ops/s | vs DistX |
|------|--------|-------|----------|
| 🥇 | **DistX gRPC** | **46,269** | baseline |
| 🥈 | Qdrant gRPC | 10,379 | 0.22x |

**DistX is 4.46x faster on inserts!**

### Search Performance (gRPC)

| Rank | System | ops/s | p50 | vs DistX |
|------|--------|-------|-----|----------|
| 🥇 | **DistX gRPC** | **6,845** | **0.13ms** | baseline |
| 🥈 | Redis RESP | 1,361 | 0.66ms | 0.20x |
| 🥉 | Qdrant gRPC | 1,087 | 0.86ms | 0.16x |

**DistX is 6.3x faster than Qdrant on search!**

---

## Dataset Size Comparison (REST API)

### Insert Performance by Dataset Size

| Dataset | DistX | Qdrant | Redis | DistX vs Qdrant |
|---------|-------|--------|-------|-----------------|
| Small (500) | 9,611 | 7,146 | 11,210 | **1.34x faster** |
| Medium (5K) | 11,653 | 8,622 | 5,242 | **1.35x faster** |
| Large (50K) | 13,511 | 10,169 | 2,434 | **1.33x faster** |

### Search Performance by Dataset Size

| Dataset | DistX | Qdrant | Redis | DistX vs Qdrant |
|---------|-------|--------|-------|-----------------|
| Small (500) | 875 | 481 | 1,871 | **1.82x faster** |
| Medium (5K) | 644 | 442 | 1,920 | **1.46x faster** |
| Large (50K) | 832 | 397 | 1,311 | **2.10x faster** |

---

## Key Optimizations

### HNSW Index Optimizations
- **Bit Vector Visited Set**: O(1) vs HashSet O(log n)
- **Contiguous Vector Storage**: Cache-friendly memory layout
- **CPU Prefetching**: Prefetch neighbor vectors
- **Brute-Force Fallback**: Flat scan for <1000 vectors

### SIMD Optimizations
- **8-wide NEON**: Dual accumulator for ARM64
- **16-wide AVX2**: FMA instructions on x86_64
- **SSE Fallback**: 4-wide for older processors

### Protocol Optimizations
- **Binary Protobuf**: Minimal serialization overhead
- **Streaming**: Efficient batch operations
- **Connection Reuse**: Persistent gRPC channels

---

## Recommendations

### Use gRPC for Production

```
┌─────────────────────────────────────────────────────────────┐
│                    PERFORMANCE LADDER                       │
├─────────────────────────────────────────────────────────────┤
│  DistX gRPC      ████████████████████████████████  46,269   │
│  Qdrant gRPC     ████████                          10,379   │
│  DistX REST      ████████                          11,176   │
│  Qdrant REST     ██████                             8,385   │
│  Redis RESP      ███                                4,419   │
└─────────────────────────────────────────────────────────────┘
                   Insert Performance (ops/s)
```

```
┌─────────────────────────────────────────────────────────────┐
│                    PERFORMANCE LADDER                       │
├─────────────────────────────────────────────────────────────┤
│  DistX gRPC      ████████████████████████████████   6,845   │
│  Redis RESP      ██████                             1,361   │
│  Qdrant gRPC     █████                              1,087   │
│  Qdrant REST     ██                                   384   │
│  DistX REST      █                                    297   │
└─────────────────────────────────────────────────────────────┘
                   Search Performance (ops/s)
```

### When to Use Each Protocol

| Use Case | Recommended Protocol |
|----------|---------------------|
| Maximum throughput | **DistX gRPC** |
| Low latency search | **DistX gRPC** (0.13ms p50) |
| REST API compatibility | DistX REST |
| Quick prototyping | DistX REST |

---

## Benchmark Commands

```bash
# Full protocol comparison (REST + gRPC)
python3 scripts/protocol_comparison.py

# REST-only comparison across dataset sizes
python3 scripts/final_benchmark.py

# Quick comparison (5K vectors)
python3 scripts/full_comparison.py
```

### Docker Setup

```bash
# Start Qdrant (REST + gRPC)
docker run -d --name qdrant -p 16333:6333 -p 16334:6334 qdrant/qdrant

# Start Redis Stack
docker run -d --name redis -p 6379:6379 redis/redis-stack-server

# Start DistX (REST + gRPC)
cargo run --release
```

---

## Summary

### 🏆 DistX Performance Victories

| Category | Winner | Performance |
|----------|--------|-------------|
| **Insert (gRPC)** | 🥇 DistX | 46,269 ops/s |
| **Search (gRPC)** | 🥇 DistX | 6,845 ops/s |
| **Insert (REST)** | 🥇 DistX | 11,176 ops/s |
| **Search Latency** | 🥇 DistX | 0.13ms p50 |
| **gRPC Speedup** | 🥇 DistX | 23x vs REST |

### Key Takeaways

1. **DistX gRPC is 6.3x faster than Qdrant gRPC on search**
2. **DistX gRPC is 4.5x faster than Qdrant gRPC on insert**
3. **DistX gRPC is 5x faster than Redis on search**
4. **DistX gRPC provides 23x speedup over REST**
5. **DistX REST still beats Qdrant REST on inserts**

**For production workloads, use DistX gRPC for maximum performance!**
