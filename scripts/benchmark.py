#!/usr/bin/env python3
"""
Basic benchmark script for DistX
Tests insert and search performance via REST API
"""
import requests
import time
import random
import sys

DISTX_URL = "http://localhost:6333"

def generate_vector(dim: int):
    """Generate random vector"""
    return [random.uniform(-1.0, 1.0) for _ in range(dim)]

def create_collection(name: str, dim: int):
    """Create a collection"""
    response = requests.put(
        f"{DISTX_URL}/collections/{name}",
        json={
            "vectors": {"size": dim, "distance": "Cosine"},
            "use_hnsw": True,
            "enable_bm25": False
        },
        timeout=5
    )
    return response.status_code in [200, 201]

def benchmark_insert(num_vectors: int, dim: int):
    """Benchmark insert performance"""
    if not create_collection("bench", dim):
        print("Failed to create collection")
        return 0, 0
    
    batch_size = 100
    start = time.time()
    total_inserted = 0
    
    for i in range(0, num_vectors, batch_size):
        batch_end = min(i + batch_size, num_vectors)
        points = []
        for j in range(i, batch_end):
            points.append({
                "id": f"point{j}",
                "vector": generate_vector(dim),
                "payload": {"id": j}
            })
        
        response = requests.put(
            f"{DISTX_URL}/collections/bench/points",
            json={"points": points},
            timeout=30
        )
        
        if response.status_code == 200:
            total_inserted += len(points)
        else:
            print(f"Insert error: {response.status_code}")
            break
    
    elapsed = time.time() - start
    return total_inserted / elapsed if elapsed > 0 else 0, elapsed

def benchmark_search(num_searches: int, dim: int):
    """Benchmark search performance"""
    query_vector = generate_vector(dim)
    
    start = time.time()
    searches = 0
    
    for _ in range(num_searches):
        response = requests.post(
            f"{DISTX_URL}/collections/bench/points/search",
            json={
                "vector": query_vector,
                "limit": 10
            },
            timeout=5
        )
        
        if response.status_code == 200:
            searches += 1
        else:
            print(f"Search error: {response.status_code}")
            break
    
    elapsed = time.time() - start
    return searches / elapsed if elapsed > 0 else 0, elapsed

def main():
    print("=" * 60)
    print("DistX Benchmark")
    print("=" * 60)
    print("")
    print("Make sure DistX is running on http://localhost:6333")
    print("")
    
    # Test scenarios
    scenarios = [
        (1000, 128, 100, "Small"),
        (10000, 128, 1000, "Medium"),
    ]
    
    print("=" * 60)
    print("INSERT PERFORMANCE")
    print("=" * 60)
    print(f"{'Scenario':<15} {'Ops/sec':<15} {'Time (s)':<15}")
    print("-" * 60)
    
    for num_vec, dim, num_search, name in scenarios:
        ops, elapsed = benchmark_insert(num_vec, dim)
        print(f"{name:<15} {ops:<15.0f} {elapsed:<15.2f}")
    
    print("")
    print("=" * 60)
    print("SEARCH PERFORMANCE")
    print("=" * 60)
    print(f"{'Scenario':<15} {'Ops/sec':<15} {'Time (s)':<15}")
    print("-" * 60)
    
    for num_vec, dim, num_search, name in scenarios:
        ops, elapsed = benchmark_search(num_search, dim)
        print(f"{name:<15} {ops:<15.0f} {elapsed:<15.2f}")
    
    print("")
    print("=" * 60)
    print("Benchmark complete")
    print("=" * 60)

if __name__ == "__main__":
    main()
