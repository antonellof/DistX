#!/usr/bin/env python3
"""
Final Performance Comparison: vectX vs Qdrant vs Redis Stack
Tests both small and large datasets
"""
import requests
import redis
import time
import random
import statistics
import struct
from typing import List, Dict

# Configuration
VECTX_URL = "http://localhost:6333"
QDRANT_URL = "http://localhost:16333"
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
    return struct.pack(f'{len(vec)}f', *vec)

def run_benchmark(name: str, num_vectors: int, dim: int, num_searches: int, batch_size: int):
    print(f"\n{'='*70}")
    print(f"Benchmark: {name} ({num_vectors:,} vectors, dim={dim})")
    print(f"{'='*70}")
    
    results = {}
    
    # vectX
    try:
        collection = f"bench_{name}"
        requests.delete(f"{VECTX_URL}/collections/{collection}", timeout=5)
        requests.put(f"{VECTX_URL}/collections/{collection}", json={
            "vectors": {"size": dim, "distance": "Cosine"},
            "use_hnsw": True
        }, timeout=5)
        
        start = time.time()
        for i in range(0, num_vectors, batch_size):
            batch_end = min(i + batch_size, num_vectors)
            points = [{"id": j, "vector": generate_vector(dim)} for j in range(i, batch_end)]
            requests.put(f"{VECTX_URL}/collections/{collection}/points", json={"points": points}, timeout=60)
        insert_time = time.time() - start
        
        latencies = []
        for _ in range(num_searches):
            query = generate_vector(dim)
            start = time.time()
            requests.post(f"{VECTX_URL}/collections/{collection}/points/search",
                         json={"vector": query, "limit": 10}, timeout=10)
            latencies.append((time.time() - start) * 1000)
        
        results["vectx"] = {
            "insert_ops": num_vectors / insert_time,
            "search_ops": num_searches / (sum(latencies) / 1000),
            "p50": statistics.median(latencies),
            "p99": sorted(latencies)[int(len(latencies) * 0.99)]
        }
        requests.delete(f"{VECTX_URL}/collections/{collection}", timeout=5)
        print(f"  vectX:  Insert {results['vectx']['insert_ops']:,.0f} ops/s | Search {results['vectx']['search_ops']:,.0f} ops/s (p50: {results['vectx']['p50']:.2f}ms)")
    except Exception as e:
        print(f"  vectX: Error - {e}")
    
    # Qdrant
    try:
        collection = f"bench_{name}"
        requests.delete(f"{QDRANT_URL}/collections/{collection}", timeout=5)
        requests.put(f"{QDRANT_URL}/collections/{collection}", json={
            "vectors": {"size": dim, "distance": "Cosine"}
        }, timeout=5)
        
        start = time.time()
        for i in range(0, num_vectors, batch_size):
            batch_end = min(i + batch_size, num_vectors)
            points = [{"id": j, "vector": generate_vector(dim)} for j in range(i, batch_end)]
            requests.put(f"{QDRANT_URL}/collections/{collection}/points", json={"points": points}, timeout=60)
        insert_time = time.time() - start
        
        latencies = []
        for _ in range(num_searches):
            query = generate_vector(dim)
            start = time.time()
            requests.post(f"{QDRANT_URL}/collections/{collection}/points/search",
                         json={"vector": query, "limit": 10}, timeout=10)
            latencies.append((time.time() - start) * 1000)
        
        results["qdrant"] = {
            "insert_ops": num_vectors / insert_time,
            "search_ops": num_searches / (sum(latencies) / 1000),
            "p50": statistics.median(latencies),
            "p99": sorted(latencies)[int(len(latencies) * 0.99)]
        }
        requests.delete(f"{QDRANT_URL}/collections/{collection}", timeout=5)
        print(f"  Qdrant: Insert {results['qdrant']['insert_ops']:,.0f} ops/s | Search {results['qdrant']['search_ops']:,.0f} ops/s (p50: {results['qdrant']['p50']:.2f}ms)")
    except Exception as e:
        print(f"  Qdrant: Error - {e}")
    
    # Redis
    try:
        client = redis.Redis(host=REDIS_HOST, port=REDIS_PORT, decode_responses=False)
        collection = f"bench_{name}"
        
        try:
            client.execute_command("FT.DROPINDEX", collection, "DD")
        except:
            pass
        
        client.execute_command(
            "FT.CREATE", collection,
            "ON", "HASH",
            "PREFIX", "1", f"{collection}:",
            "SCHEMA",
            "vector", "VECTOR", "HNSW", "6",
            "TYPE", "FLOAT32",
            "DIM", str(dim),
            "DISTANCE_METRIC", "COSINE"
        )
        
        start = time.time()
        for i in range(0, num_vectors, batch_size):
            batch_end = min(i + batch_size, num_vectors)
            pipe = client.pipeline()
            for j in range(i, batch_end):
                key = f"{collection}:{j}"
                vec = generate_vector(dim)
                vec_bytes = vector_to_bytes(vec)
                pipe.hset(key, mapping={"vector": vec_bytes, "id": str(j)})
            pipe.execute()
        insert_time = time.time() - start
        
        latencies = []
        for _ in range(num_searches):
            query = generate_vector(dim)
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
            except:
                pass
        
        if latencies:
            results["redis"] = {
                "insert_ops": num_vectors / insert_time,
                "search_ops": len(latencies) / (sum(latencies) / 1000),
                "p50": statistics.median(latencies),
                "p99": sorted(latencies)[int(len(latencies) * 0.99)]
            }
            print(f"  Redis:  Insert {results['redis']['insert_ops']:,.0f} ops/s | Search {results['redis']['search_ops']:,.0f} ops/s (p50: {results['redis']['p50']:.2f}ms)")
        
        try:
            client.execute_command("FT.DROPINDEX", collection, "DD")
        except:
            pass
    except Exception as e:
        print(f"  Redis: Error - {e}")
    
    return results

def main():
    print("=" * 70)
    print("COMPREHENSIVE VECTOR DATABASE COMPARISON")
    print("vectX vs Qdrant vs Redis Stack")
    print("=" * 70)
    
    all_results = {}
    
    # Small dataset (brute-force optimal)
    all_results["small"] = run_benchmark("small", 500, 128, 200, 50)
    
    # Medium dataset
    all_results["medium"] = run_benchmark("medium", 5000, 128, 500, 100)
    
    # Large dataset (HNSW optimal)
    all_results["large"] = run_benchmark("large", 50000, 128, 1000, 500)
    
    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY - INSERT PERFORMANCE (ops/sec)")
    print("=" * 80)
    print(f"{'Dataset':<12} {'vectX':<15} {'Qdrant':<15} {'Redis':<15} {'vectX vs Qdrant':<15}")
    print("-" * 80)
    for name, res in all_results.items():
        vectx = res.get("vectx", {}).get("insert_ops", 0)
        qdrant = res.get("qdrant", {}).get("insert_ops", 0)
        redis_val = res.get("redis", {}).get("insert_ops", 0)
        ratio = f"{vectx/qdrant:.2f}x" if qdrant else "N/A"
        print(f"{name:<12} {vectx:<15,.0f} {qdrant:<15,.0f} {redis_val:<15,.0f} {ratio:<15}")
    
    print("\n" + "=" * 80)
    print("SUMMARY - SEARCH PERFORMANCE (ops/sec)")
    print("=" * 80)
    print(f"{'Dataset':<12} {'vectX':<15} {'Qdrant':<15} {'Redis':<15} {'vectX vs Qdrant':<15}")
    print("-" * 80)
    for name, res in all_results.items():
        vectx = res.get("vectx", {}).get("search_ops", 0)
        qdrant = res.get("qdrant", {}).get("search_ops", 0)
        redis_val = res.get("redis", {}).get("search_ops", 0)
        ratio = f"{vectx/qdrant:.2f}x" if qdrant else "N/A"
        print(f"{name:<12} {vectx:<15,.0f} {qdrant:<15,.0f} {redis_val:<15,.0f} {ratio:<15}")
    
    print("\n" + "=" * 80)
    print("Benchmark complete!")
    print("=" * 80)

if __name__ == "__main__":
    main()
