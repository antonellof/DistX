#!/usr/bin/env python3
"""
Full Performance Comparison: DistX vs Qdrant vs Redis Stack
All three running on different ports
"""
import requests
import redis
import time
import random
import statistics
import struct
import sys
from typing import List, Dict

# Configuration
DISTX_URL = "http://localhost:6333"
QDRANT_URL = "http://localhost:16333"
REDIS_HOST = "localhost"
REDIS_PORT = 6379

# Test parameters
NUM_VECTORS = 5000  # Will test both small and large
DIM = 128
NUM_SEARCHES = 500
BATCH_SIZE = 100


def generate_vector(dim: int) -> List[float]:
    """Generate random normalized vector"""
    vec = [random.uniform(-1.0, 1.0) for _ in range(dim)]
    norm = sum(x * x for x in vec) ** 0.5
    if norm > 0:
        vec = [x / norm for x in vec]
    return vec


def vector_to_bytes(vec: List[float]) -> bytes:
    return struct.pack(f'{len(vec)}f', *vec)


def benchmark_distx():
    """Benchmark DistX"""
    name = "DistX"
    url = DISTX_URL
    collection = "bench_distx"
    
    print(f"\n{'='*60}")
    print(f"Benchmarking {name}")
    print(f"{'='*60}")
    
    # Check availability
    try:
        r = requests.get(f"{url}/healthz", timeout=2)
        if r.status_code != 200:
            return {"name": name, "available": False}
    except:
        return {"name": name, "available": False}
    
    # Setup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    r = requests.put(f"{url}/collections/{collection}", json={
        "vectors": {"size": DIM, "distance": "Cosine"},
        "use_hnsw": True
    }, timeout=5)
    
    # Insert benchmark
    print(f"  Inserting {NUM_VECTORS:,} vectors...")
    insert_start = time.time()
    for i in range(0, NUM_VECTORS, BATCH_SIZE):
        batch_end = min(i + BATCH_SIZE, NUM_VECTORS)
        points = [{"id": j, "vector": generate_vector(DIM)} for j in range(i, batch_end)]
        requests.put(f"{url}/collections/{collection}/points", json={"points": points}, timeout=60)
    insert_time = time.time() - insert_start
    insert_ops = NUM_VECTORS / insert_time
    print(f"  Insert: {insert_ops:,.0f} ops/sec ({insert_time:.2f}s)")
    
    # Search benchmark
    print(f"  Running {NUM_SEARCHES:,} searches...")
    latencies = []
    for _ in range(NUM_SEARCHES):
        query = generate_vector(DIM)
        start = time.time()
        requests.post(f"{url}/collections/{collection}/points/search",
                     json={"vector": query, "limit": 10}, timeout=10)
        latencies.append((time.time() - start) * 1000)
    
    search_time = sum(latencies) / 1000
    search_ops = NUM_SEARCHES / search_time
    p50 = statistics.median(latencies)
    p99 = sorted(latencies)[int(len(latencies) * 0.99)]
    print(f"  Search: {search_ops:,.0f} ops/sec (p50: {p50:.2f}ms, p99: {p99:.2f}ms)")
    
    # Cleanup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    
    return {
        "name": name,
        "available": True,
        "insert_ops_sec": insert_ops,
        "insert_time": insert_time,
        "search_ops_sec": search_ops,
        "search_p50_ms": p50,
        "search_p99_ms": p99
    }


def benchmark_qdrant():
    """Benchmark Qdrant"""
    name = "Qdrant"
    url = QDRANT_URL
    collection = "bench_qdrant"
    
    print(f"\n{'='*60}")
    print(f"Benchmarking {name}")
    print(f"{'='*60}")
    
    # Check availability
    try:
        r = requests.get(f"{url}/", timeout=2)
        if "qdrant" not in r.json().get("title", "").lower():
            return {"name": name, "available": False}
    except:
        return {"name": name, "available": False}
    
    # Setup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    r = requests.put(f"{url}/collections/{collection}", json={
        "vectors": {"size": DIM, "distance": "Cosine"}
    }, timeout=5)
    
    # Insert benchmark
    print(f"  Inserting {NUM_VECTORS:,} vectors...")
    insert_start = time.time()
    for i in range(0, NUM_VECTORS, BATCH_SIZE):
        batch_end = min(i + BATCH_SIZE, NUM_VECTORS)
        points = [{"id": j, "vector": generate_vector(DIM)} for j in range(i, batch_end)]
        requests.put(f"{url}/collections/{collection}/points", json={"points": points}, timeout=60)
    insert_time = time.time() - insert_start
    insert_ops = NUM_VECTORS / insert_time
    print(f"  Insert: {insert_ops:,.0f} ops/sec ({insert_time:.2f}s)")
    
    # Search benchmark
    print(f"  Running {NUM_SEARCHES:,} searches...")
    latencies = []
    for _ in range(NUM_SEARCHES):
        query = generate_vector(DIM)
        start = time.time()
        requests.post(f"{url}/collections/{collection}/points/search",
                     json={"vector": query, "limit": 10}, timeout=10)
        latencies.append((time.time() - start) * 1000)
    
    search_time = sum(latencies) / 1000
    search_ops = NUM_SEARCHES / search_time
    p50 = statistics.median(latencies)
    p99 = sorted(latencies)[int(len(latencies) * 0.99)]
    print(f"  Search: {search_ops:,.0f} ops/sec (p50: {p50:.2f}ms, p99: {p99:.2f}ms)")
    
    # Cleanup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    
    return {
        "name": name,
        "available": True,
        "insert_ops_sec": insert_ops,
        "insert_time": insert_time,
        "search_ops_sec": search_ops,
        "search_p50_ms": p50,
        "search_p99_ms": p99
    }


def benchmark_redis():
    """Benchmark Redis Stack"""
    name = "Redis Stack"
    collection = "bench_redis"
    
    print(f"\n{'='*60}")
    print(f"Benchmarking {name}")
    print(f"{'='*60}")
    
    # Check availability
    try:
        client = redis.Redis(host=REDIS_HOST, port=REDIS_PORT, decode_responses=False)
        client.ping()
    except Exception as e:
        print(f"  Redis not available: {e}")
        return {"name": name, "available": False}
    
    # Setup - create index
    try:
        client.execute_command("FT.DROPINDEX", collection, "DD")
    except:
        pass
    
    try:
        client.execute_command(
            "FT.CREATE", collection,
            "ON", "HASH",
            "PREFIX", "1", f"{collection}:",
            "SCHEMA",
            "vector", "VECTOR", "HNSW", "6",
            "TYPE", "FLOAT32",
            "DIM", str(DIM),
            "DISTANCE_METRIC", "COSINE"
        )
    except Exception as e:
        print(f"  Index creation failed: {e}")
        return {"name": name, "available": False}
    
    # Insert benchmark
    print(f"  Inserting {NUM_VECTORS:,} vectors...")
    insert_start = time.time()
    for i in range(0, NUM_VECTORS, BATCH_SIZE):
        batch_end = min(i + BATCH_SIZE, NUM_VECTORS)
        pipe = client.pipeline()
        for j in range(i, batch_end):
            key = f"{collection}:{j}"
            vec = generate_vector(DIM)
            vec_bytes = vector_to_bytes(vec)
            pipe.hset(key, mapping={"vector": vec_bytes, "id": str(j)})
        pipe.execute()
    insert_time = time.time() - insert_start
    insert_ops = NUM_VECTORS / insert_time
    print(f"  Insert: {insert_ops:,.0f} ops/sec ({insert_time:.2f}s)")
    
    # Search benchmark
    print(f"  Running {NUM_SEARCHES:,} searches...")
    latencies = []
    for _ in range(NUM_SEARCHES):
        query = generate_vector(DIM)
        vec_bytes = vector_to_bytes(query)
        start = time.time()
        try:
            client.execute_command(
                "FT.SEARCH", collection,
                f"*=>[KNN 10 @vector $vec AS score]",
                "PARAMS", "2", "vec", vec_bytes,
                "SORTBY", "score",
                "DIALECT", "2"
            )
            latencies.append((time.time() - start) * 1000)
        except Exception as e:
            pass
    
    if not latencies:
        return {"name": name, "available": True, "search_error": True}
    
    search_time = sum(latencies) / 1000
    search_ops = len(latencies) / search_time
    p50 = statistics.median(latencies)
    p99 = sorted(latencies)[int(len(latencies) * 0.99)]
    print(f"  Search: {search_ops:,.0f} ops/sec (p50: {p50:.2f}ms, p99: {p99:.2f}ms)")
    
    # Cleanup
    try:
        client.execute_command("FT.DROPINDEX", collection, "DD")
    except:
        pass
    
    return {
        "name": name,
        "available": True,
        "insert_ops_sec": insert_ops,
        "insert_time": insert_time,
        "search_ops_sec": search_ops,
        "search_p50_ms": p50,
        "search_p99_ms": p99
    }


def main():
    print("=" * 70)
    print("VECTOR DATABASE PERFORMANCE COMPARISON")
    print("DistX vs Qdrant vs Redis Stack")
    print("=" * 70)
    print(f"\nTest Configuration:")
    print(f"  Vectors:   {NUM_VECTORS:,}")
    print(f"  Dimension: {DIM}")
    print(f"  Searches:  {NUM_SEARCHES:,}")
    print(f"  Batch:     {BATCH_SIZE}")
    
    # Run benchmarks
    results = []
    results.append(benchmark_distx())
    results.append(benchmark_qdrant())
    results.append(benchmark_redis())
    
    # Print comparison table
    print("\n" + "=" * 80)
    print("FINAL RESULTS COMPARISON")
    print("=" * 80)
    print()
    print(f"{'Database':<15} {'Insert ops/s':<15} {'Search ops/s':<15} {'Search p50':<12} {'Search p99':<12}")
    print("-" * 80)
    
    for r in results:
        if not r.get("available", False):
            print(f"{r['name']:<15} {'N/A':<15} {'N/A':<15} {'N/A':<12} {'N/A':<12}")
        elif r.get("search_error"):
            print(f"{r['name']:<15} {r.get('insert_ops_sec', 0):<15,.0f} {'ERROR':<15} {'N/A':<12} {'N/A':<12}")
        else:
            print(f"{r['name']:<15} {r['insert_ops_sec']:<15,.0f} {r['search_ops_sec']:<15,.0f} "
                  f"{r['search_p50_ms']:<12.2f} {r['search_p99_ms']:<12.2f}")
    
    print()
    
    # Calculate relative performance
    available = [r for r in results if r.get("available") and not r.get("search_error")]
    if len(available) > 1:
        print("Relative Performance (higher is better):")
        print("-" * 50)
        baseline = available[0]
        for r in available:
            insert_ratio = r['insert_ops_sec'] / baseline['insert_ops_sec']
            search_ratio = r['search_ops_sec'] / baseline['search_ops_sec']
            print(f"  {r['name']}: Insert {insert_ratio:.2f}x, Search {search_ratio:.2f}x (vs {baseline['name']})")
    
    print()
    print("=" * 80)
    print("Benchmark complete!")
    print("=" * 80)


if __name__ == "__main__":
    main()
