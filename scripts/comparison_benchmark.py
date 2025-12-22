#!/usr/bin/env python3
"""
Performance Comparison Benchmark: vectX vs Qdrant vs Redis Stack

Compares insert and search performance across three vector databases:
- vectX (our implementation)
- Qdrant (popular vector database)
- Redis Stack (Redis with vector search module)
"""
import requests
import redis
import time
import random
import statistics
import struct
import sys
from typing import List, Dict, Tuple
import argparse

# Configuration
VECTX_URL = "http://localhost:6333"
QDRANT_URL = "http://localhost:6333"  # Same port as vectX, we'll run separately
REDIS_HOST = "localhost"
REDIS_PORT = 6379


def generate_vector(dim: int) -> List[float]:
    """Generate random normalized vector"""
    vec = [random.uniform(-1.0, 1.0) for _ in range(dim)]
    norm = sum(x * x for x in vec) ** 0.5
    if norm > 0:
        vec = [x / norm for x in vec]
    return vec


def vector_to_bytes(vec: List[float]) -> bytes:
    """Convert vector to bytes for Redis"""
    return struct.pack(f'{len(vec)}f', *vec)


# ============================================================================
# vectX Benchmark
# ============================================================================
class vectXBenchmark:
    def __init__(self, url: str = "http://localhost:6333"):
        self.url = url
        self.name = "vectX"
    
    def is_available(self) -> bool:
        try:
            r = requests.get(f"{self.url}/healthz", timeout=2)
            return r.status_code == 200
        except:
            return False
    
    def setup(self, collection: str, dim: int):
        # Delete if exists
        requests.delete(f"{self.url}/collections/{collection}", timeout=5)
        # Create collection
        r = requests.put(f"{self.url}/collections/{collection}", json={
            "vectors": {"size": dim, "distance": "Cosine"},
            "use_hnsw": True
        }, timeout=5)
        return r.status_code in [200, 201]
    
    def cleanup(self, collection: str):
        requests.delete(f"{self.url}/collections/{collection}", timeout=5)
    
    def insert_batch(self, collection: str, points: List[Dict]) -> float:
        start = time.time()
        r = requests.put(f"{self.url}/collections/{collection}/points", 
                        json={"points": points}, timeout=60)
        elapsed = time.time() - start
        return elapsed if r.status_code == 200 else -1
    
    def search(self, collection: str, vector: List[float], limit: int = 10) -> float:
        start = time.time()
        r = requests.post(f"{self.url}/collections/{collection}/points/search",
                         json={"vector": vector, "limit": limit}, timeout=10)
        elapsed = time.time() - start
        return elapsed if r.status_code == 200 else -1


# ============================================================================
# Qdrant Benchmark
# ============================================================================
class QdrantBenchmark:
    def __init__(self, url: str = "http://localhost:6333"):
        self.url = url
        self.name = "Qdrant"
    
    def is_available(self) -> bool:
        try:
            r = requests.get(f"{self.url}/", timeout=2)
            data = r.json()
            return "title" in data and "qdrant" in data.get("title", "").lower()
        except:
            return False
    
    def setup(self, collection: str, dim: int):
        # Delete if exists
        requests.delete(f"{self.url}/collections/{collection}", timeout=5)
        # Create collection (Qdrant format)
        r = requests.put(f"{self.url}/collections/{collection}", json={
            "vectors": {
                "size": dim,
                "distance": "Cosine"
            }
        }, timeout=5)
        return r.status_code in [200, 201]
    
    def cleanup(self, collection: str):
        requests.delete(f"{self.url}/collections/{collection}", timeout=5)
    
    def insert_batch(self, collection: str, points: List[Dict]) -> float:
        # Convert to Qdrant format
        qdrant_points = [
            {"id": p["id"], "vector": p["vector"], "payload": p.get("payload", {})}
            for p in points
        ]
        start = time.time()
        r = requests.put(f"{self.url}/collections/{collection}/points",
                        json={"points": qdrant_points}, timeout=60)
        elapsed = time.time() - start
        return elapsed if r.status_code == 200 else -1
    
    def search(self, collection: str, vector: List[float], limit: int = 10) -> float:
        start = time.time()
        r = requests.post(f"{self.url}/collections/{collection}/points/search",
                         json={"vector": vector, "limit": limit}, timeout=10)
        elapsed = time.time() - start
        return elapsed if r.status_code == 200 else -1


# ============================================================================
# Redis Stack Benchmark
# ============================================================================
class RedisBenchmark:
    def __init__(self, host: str = "localhost", port: int = 6379):
        self.host = host
        self.port = port
        self.client = None
        self.name = "Redis Stack"
    
    def is_available(self) -> bool:
        try:
            self.client = redis.Redis(host=self.host, port=self.port, decode_responses=False)
            self.client.ping()
            # Check if search module is loaded
            modules = self.client.execute_command("MODULE", "LIST")
            # modules is a list of lists/tuples, check for 'search' in module names
            for m in modules:
                if isinstance(m, (list, tuple)) and len(m) > 1:
                    name = m[1] if isinstance(m[1], bytes) else str(m[1])
                    if b"search" in name.lower() if isinstance(name, bytes) else "search" in name.lower():
                        return True
            return True  # Assume it's available if we can ping
        except Exception as e:
            print(f"Redis error: {e}")
            return False
    
    def setup(self, collection: str, dim: int):
        try:
            # Drop existing index
            try:
                self.client.execute_command("FT.DROPINDEX", collection, "DD")
            except:
                pass
            
            # Create index with vector field
            self.client.execute_command(
                "FT.CREATE", collection,
                "ON", "HASH",
                "PREFIX", "1", f"{collection}:",
                "SCHEMA",
                "vector", "VECTOR", "HNSW", "6",
                "TYPE", "FLOAT32",
                "DIM", str(dim),
                "DISTANCE_METRIC", "COSINE"
            )
            return True
        except Exception as e:
            print(f"Redis setup error: {e}")
            return False
    
    def cleanup(self, collection: str):
        try:
            self.client.execute_command("FT.DROPINDEX", collection, "DD")
        except:
            pass
    
    def insert_batch(self, collection: str, points: List[Dict]) -> float:
        start = time.time()
        try:
            pipe = self.client.pipeline()
            for p in points:
                key = f"{collection}:{p['id']}"
                vec_bytes = vector_to_bytes(p["vector"])
                pipe.hset(key, mapping={"vector": vec_bytes, "id": str(p["id"])})
            pipe.execute()
            elapsed = time.time() - start
            return elapsed
        except Exception as e:
            print(f"Redis insert error: {e}")
            return -1
    
    def search(self, collection: str, vector: List[float], limit: int = 10) -> float:
        start = time.time()
        try:
            vec_bytes = vector_to_bytes(vector)
            query = f"*=>[KNN {limit} @vector $vec AS score]"
            result = self.client.execute_command(
                "FT.SEARCH", collection, query,
                "PARAMS", "2", "vec", vec_bytes,
                "SORTBY", "score",
                "DIALECT", "2"
            )
            elapsed = time.time() - start
            return elapsed
        except Exception as e:
            print(f"Redis search error: {e}")
            return -1


# ============================================================================
# Benchmark Runner
# ============================================================================
def run_benchmark(benchmark, collection: str, num_vectors: int, dim: int, 
                  num_searches: int, batch_size: int = 100) -> Dict:
    """Run benchmark for a single system"""
    results = {
        "name": benchmark.name,
        "available": False,
        "insert_ops_sec": 0,
        "insert_latency_ms": 0,
        "search_ops_sec": 0,
        "search_latency_p50_ms": 0,
        "search_latency_p99_ms": 0,
    }
    
    if not benchmark.is_available():
        print(f"  {benchmark.name}: NOT AVAILABLE")
        return results
    
    results["available"] = True
    print(f"  {benchmark.name}: Running...")
    
    # Setup
    if not benchmark.setup(collection, dim):
        print(f"  {benchmark.name}: Setup failed")
        return results
    
    # Insert benchmark
    insert_times = []
    total_inserted = 0
    for i in range(0, num_vectors, batch_size):
        batch_end = min(i + batch_size, num_vectors)
        points = [{"id": j, "vector": generate_vector(dim), "payload": {"id": j}} 
                  for j in range(i, batch_end)]
        elapsed = benchmark.insert_batch(collection, points)
        if elapsed > 0:
            insert_times.append(elapsed)
            total_inserted += len(points)
    
    total_insert_time = sum(insert_times)
    results["insert_ops_sec"] = total_inserted / total_insert_time if total_insert_time > 0 else 0
    results["insert_latency_ms"] = (total_insert_time / len(insert_times) * 1000) if insert_times else 0
    
    # Search benchmark
    search_latencies = []
    for _ in range(num_searches):
        query = generate_vector(dim)
        elapsed = benchmark.search(collection, query, 10)
        if elapsed > 0:
            search_latencies.append(elapsed * 1000)  # Convert to ms
    
    if search_latencies:
        total_search_time = sum(search_latencies) / 1000  # Back to seconds
        results["search_ops_sec"] = len(search_latencies) / total_search_time if total_search_time > 0 else 0
        results["search_latency_p50_ms"] = statistics.median(search_latencies)
        sorted_latencies = sorted(search_latencies)
        p99_idx = int(len(sorted_latencies) * 0.99)
        results["search_latency_p99_ms"] = sorted_latencies[min(p99_idx, len(sorted_latencies)-1)]
    
    # Cleanup
    benchmark.cleanup(collection)
    
    return results


def main():
    parser = argparse.ArgumentParser(description="Compare vectX vs Qdrant vs Redis Stack")
    parser.add_argument("--vectors", type=int, default=5000, help="Number of vectors to insert")
    parser.add_argument("--dim", type=int, default=128, help="Vector dimension")
    parser.add_argument("--searches", type=int, default=500, help="Number of searches")
    parser.add_argument("--vectx-port", type=int, default=6333, help="vectX port (when running separately)")
    parser.add_argument("--qdrant-port", type=int, default=6333, help="Qdrant port")
    parser.add_argument("--redis-port", type=int, default=6379, help="Redis port")
    args = parser.parse_args()
    
    print("=" * 80)
    print("Vector Database Performance Comparison")
    print("=" * 80)
    print(f"Configuration:")
    print(f"  Vectors: {args.vectors:,}")
    print(f"  Dimension: {args.dim}")
    print(f"  Searches: {args.searches:,}")
    print()
    
    # Note: vectX and Qdrant use the same default port (6333)
    # So we need to test them separately or use different ports
    
    benchmarks_to_run = []
    
    # Check what's running on port 6333
    try:
        r = requests.get("http://localhost:6333/", timeout=2)
        data = r.json()
        if "qdrant" in data.get("title", "").lower():
            print("Detected: Qdrant running on port 6333")
            benchmarks_to_run.append(("qdrant", QdrantBenchmark("http://localhost:6333")))
        elif "vectx" in data.get("title", "").lower():
            print("Detected: vectX running on port 6333")
            benchmarks_to_run.append(("vectx", vectXBenchmark("http://localhost:6333")))
    except:
        pass
    
    # Check Redis
    redis_bench = RedisBenchmark(port=args.redis_port)
    if redis_bench.is_available():
        print(f"Detected: Redis Stack running on port {args.redis_port}")
        benchmarks_to_run.append(("redis", redis_bench))
    
    if not benchmarks_to_run:
        print("\nNo vector databases detected!")
        print("\nTo run the comparison:")
        print("  1. Start vectX:  cargo run --release")
        print("  2. Or Qdrant:    docker run -p 6333:6333 qdrant/qdrant")
        print("  3. And Redis:    docker run -p 6379:6379 redis/redis-stack-server")
        sys.exit(1)
    
    print()
    print("=" * 80)
    print("Running Benchmarks...")
    print("=" * 80)
    
    all_results = []
    for name, benchmark in benchmarks_to_run:
        result = run_benchmark(
            benchmark,
            collection="bench_comparison",
            num_vectors=args.vectors,
            dim=args.dim,
            num_searches=args.searches
        )
        all_results.append(result)
        print(f"  {result['name']}: Done")
        print(f"    Insert: {result['insert_ops_sec']:,.0f} ops/sec")
        print(f"    Search: {result['search_ops_sec']:,.0f} ops/sec (p50: {result['search_latency_p50_ms']:.2f}ms)")
        print()
    
    # Print comparison table
    print("=" * 80)
    print("RESULTS COMPARISON")
    print("=" * 80)
    print()
    print(f"{'Database':<15} {'Insert ops/s':<15} {'Search ops/s':<15} {'Search p50':<12} {'Search p99':<12}")
    print("-" * 80)
    
    for r in all_results:
        if r["available"]:
            print(f"{r['name']:<15} {r['insert_ops_sec']:<15,.0f} {r['search_ops_sec']:<15,.0f} "
                  f"{r['search_latency_p50_ms']:<12.2f} {r['search_latency_p99_ms']:<12.2f}")
        else:
            print(f"{r['name']:<15} {'N/A':<15} {'N/A':<15} {'N/A':<12} {'N/A':<12}")
    
    print()
    print("=" * 80)
    print("Benchmark complete!")
    print()
    print("Note: To compare all three, run them on different ports:")
    print("  vectX:  cargo run --release (port 6333)")
    print("  Qdrant: docker run -p 16333:6333 qdrant/qdrant")
    print("  Redis:  docker run -p 6379:6379 redis/redis-stack-server")
    print()


if __name__ == "__main__":
    main()
