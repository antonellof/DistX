# Post for r/rust or users.rust-lang.org

---

## Title:
**DistX: High-performance vector database in Rust - 6x faster search than Qdrant**

---

## Post:

Hey Rustaceans! 👋

I'm excited to share **DistX**, a vector database I've been building in Rust. It's designed for AI/ML workloads like semantic search, RAG pipelines, and recommendation systems.

### Why Another Vector Database?

I wanted to combine:
- **Redis's simplicity** - single binary, straightforward persistence model
- **Qdrant's API compatibility** - drop-in replacement for existing code
- **Maximum performance** - leveraging Rust's zero-cost abstractions

### Performance Results

Benchmarked against Qdrant and Redis Stack on 5,000 128-dim vectors:

| Metric | DistX (gRPC) | Qdrant (gRPC) | Redis |
|--------|--------------|---------------|-------|
| Insert | 0.05ms | 0.22ms | 0.52ms |
| Search | 0.13ms | 0.82ms | 0.65ms |

**Key wins:**
- 🚀 6.3x faster search than Qdrant
- ⚡ 10x faster inserts than Redis
- 🎯 Sub-millisecond p99 latency

### Technical Highlights

**SIMD Optimizations:**
- AVX2 (x86_64) and NEON (ARM64) for vector distance calculations
- 8-wide parallel processing with loop unrolling
- Achieved near-theoretical throughput on M1/M2 Macs

```rust
// Example: NEON 8-wide dot product
unsafe fn dot_product_neon(a: &[f32], b: &[f32]) -> f32 {
    let mut sum1 = vdupq_n_f32(0.0);
    let mut sum2 = vdupq_n_f32(0.0);
    
    for i in (0..a.len()).step_by(8) {
        let a1 = vld1q_f32(a.as_ptr().add(i));
        let a2 = vld1q_f32(a.as_ptr().add(i + 4));
        let b1 = vld1q_f32(b.as_ptr().add(i));
        let b2 = vld1q_f32(b.as_ptr().add(i + 4));
        sum1 = vfmaq_f32(sum1, a1, b1);
        sum2 = vfmaq_f32(sum2, a2, b2);
    }
    // horizontal sum...
}
```

**HNSW Index Optimizations:**
- Custom `VisitedSet` using bit vectors instead of `HashSet` - O(1) with minimal allocations
- Contiguous vector storage for cache locality
- Pre-allocated buffers to avoid per-search allocations

**Concurrency:**
- `parking_lot::RwLock` for lock-free reads
- Lock-free metrics with `AtomicU64`
- Tokio for async I/O

### Stack

- **REST API**: `actix-web` 
- **gRPC**: `tonic` + `prost`
- **Storage**: `heed` (LMDB bindings)
- **Serialization**: `serde` + `bincode`

### Try It

```bash
# Install from crates.io
cargo install distx

# Or download pre-built binary
curl -LO https://github.com/antonellof/DistX/releases/latest/download/distx-linux-x86_64.tar.gz

# Run
distx --http-port 6333 --grpc-port 6334
```

### Links

- **GitHub**: https://github.com/antonellof/DistX
- **Crates.io**: https://crates.io/crates/distx
- **Docs**: https://docs.rs/distx

### Looking for Feedback

I'd love to hear your thoughts on:
1. The SIMD implementation approach
2. Any performance improvements I might have missed
3. API design suggestions

This is my first major Rust project, so any feedback from the community would be invaluable!

---

## Shorter Version (for r/rust with character limits):

**Title:** DistX: Vector database in Rust - 6x faster than Qdrant

I built a vector database optimized for AI/ML workloads. Key features:

- 🚀 6x faster search, 10x faster inserts vs competitors
- SIMD optimized (AVX2/NEON)
- Qdrant-compatible REST API
- gRPC for max performance
- Single ~6MB binary

Stack: actix-web, tonic, heed (LMDB), parking_lot

```bash
cargo install distx
```

GitHub: https://github.com/antonellof/DistX

Would love feedback on the SIMD implementation and any perf improvements I might have missed!

---

## For users.rust-lang.org (Show and Tell category):

**Title:** [Show] DistX - Vector database with SIMD-optimized HNSW search

Built my first production Rust project - a vector database for semantic search and RAG pipelines.

**What I learned:**
- `std::arch` intrinsics for SIMD (AVX2/NEON)
- Bit manipulation for efficient visited tracking in graph search
- `parking_lot` vs `std::sync` for high-contention scenarios
- `tonic` for gRPC services

**Challenges solved:**
- Cross-platform SIMD with runtime detection
- Cache-friendly memory layouts for vector data
- Balancing `unsafe` for performance vs safety

Would appreciate any code review or suggestions!

https://github.com/antonellof/DistX
