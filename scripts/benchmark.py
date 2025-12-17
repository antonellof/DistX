#!/usr/bin/env python3
"""
Performance benchmark script for DistX
Tests insert, search, and mixed workload performance via REST API

Inspired by Redis and Qdrant benchmarking patterns:
- Measures throughput (ops/sec)
- Measures latency (p50, p95, p99)
- Tests different vector dimensions
- Tests concurrent operations
"""
import requests
import time
import random
import statistics
import sys
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Tuple, Optional

DISTX_URL = "http://localhost:6333"


def generate_vector(dim: int) -> List[float]:
    """Generate random normalized vector"""
    vec = [random.uniform(-1.0, 1.0) for _ in range(dim)]
    # Normalize for cosine similarity
    norm = sum(x * x for x in vec) ** 0.5
    if norm > 0:
        vec = [x / norm for x in vec]
    return vec


def check_server() -> bool:
    """Check if DistX server is running"""
    try:
        response = requests.get(f"{DISTX_URL}/healthz", timeout=2)
        return response.status_code == 200
    except requests.exceptions.RequestException:
        return False


def delete_collection(name: str) -> bool:
    """Delete a collection if it exists"""
    try:
        response = requests.delete(f"{DISTX_URL}/collections/{name}", timeout=5)
        return response.status_code in [200, 404]
    except requests.exceptions.RequestException:
        return False


def create_collection(name: str, dim: int, use_hnsw: bool = True) -> bool:
    """Create a collection"""
    delete_collection(name)  # Clean up first
    try:
        response = requests.put(
            f"{DISTX_URL}/collections/{name}",
            json={
                "vectors": {"size": dim, "distance": "Cosine"},
                "use_hnsw": use_hnsw,
                "enable_bm25": False
            },
            timeout=5
        )
        return response.status_code in [200, 201]
    except requests.exceptions.RequestException as e:
        print(f"Error creating collection: {e}")
        return False


def benchmark_insert(collection: str, num_vectors: int, dim: int, batch_size: int = 100) -> dict:
    """Benchmark insert performance with latency tracking"""
    latencies = []
    total_inserted = 0
    
    overall_start = time.time()
    
    for i in range(0, num_vectors, batch_size):
        batch_end = min(i + batch_size, num_vectors)
        points = []
        for j in range(i, batch_end):
            points.append({
                "id": j,
                "vector": generate_vector(dim),
                "payload": {"id": j, "batch": i // batch_size}
            })
        
        batch_start = time.time()
        try:
            response = requests.put(
                f"{DISTX_URL}/collections/{collection}/points",
                json={"points": points},
                timeout=60
            )
            batch_latency = (time.time() - batch_start) * 1000  # ms
            
            if response.status_code == 200:
                total_inserted += len(points)
                latencies.append(batch_latency)
            else:
                print(f"Insert error: {response.status_code} - {response.text[:100]}")
                break
        except requests.exceptions.RequestException as e:
            print(f"Request error: {e}")
            break
    
    overall_elapsed = time.time() - overall_start
    
    return {
        "total": total_inserted,
        "elapsed_sec": overall_elapsed,
        "ops_per_sec": total_inserted / overall_elapsed if overall_elapsed > 0 else 0,
        "latency_p50_ms": statistics.median(latencies) if latencies else 0,
        "latency_p95_ms": statistics.quantiles(latencies, n=20)[18] if len(latencies) >= 20 else max(latencies) if latencies else 0,
        "latency_p99_ms": statistics.quantiles(latencies, n=100)[98] if len(latencies) >= 100 else max(latencies) if latencies else 0,
    }


def benchmark_search(collection: str, num_searches: int, dim: int) -> dict:
    """Benchmark search performance with latency tracking"""
    latencies = []
    successful = 0
    
    overall_start = time.time()
    
    for _ in range(num_searches):
        query_vector = generate_vector(dim)
        
        search_start = time.time()
        try:
            response = requests.post(
                f"{DISTX_URL}/collections/{collection}/points/search",
                json={"vector": query_vector, "limit": 10},
                timeout=10
            )
            search_latency = (time.time() - search_start) * 1000  # ms
            
            if response.status_code == 200:
                successful += 1
                latencies.append(search_latency)
            else:
                print(f"Search error: {response.status_code}")
        except requests.exceptions.RequestException as e:
            print(f"Request error: {e}")
    
    overall_elapsed = time.time() - overall_start
    
    return {
        "total": successful,
        "elapsed_sec": overall_elapsed,
        "ops_per_sec": successful / overall_elapsed if overall_elapsed > 0 else 0,
        "latency_p50_ms": statistics.median(latencies) if latencies else 0,
        "latency_p95_ms": statistics.quantiles(latencies, n=20)[18] if len(latencies) >= 20 else max(latencies) if latencies else 0,
        "latency_p99_ms": statistics.quantiles(latencies, n=100)[98] if len(latencies) >= 100 else max(latencies) if latencies else 0,
    }


def benchmark_concurrent_search(collection: str, num_searches: int, dim: int, num_threads: int) -> dict:
    """Benchmark concurrent search performance"""
    latencies = []
    successful = 0
    
    def do_search():
        query_vector = generate_vector(dim)
        start = time.time()
        try:
            response = requests.post(
                f"{DISTX_URL}/collections/{collection}/points/search",
                json={"vector": query_vector, "limit": 10},
                timeout=10
            )
            latency = (time.time() - start) * 1000
            return response.status_code == 200, latency
        except:
            return False, 0
    
    overall_start = time.time()
    
    with ThreadPoolExecutor(max_workers=num_threads) as executor:
        futures = [executor.submit(do_search) for _ in range(num_searches)]
        for future in as_completed(futures):
            success, latency = future.result()
            if success:
                successful += 1
                latencies.append(latency)
    
    overall_elapsed = time.time() - overall_start
    
    return {
        "total": successful,
        "threads": num_threads,
        "elapsed_sec": overall_elapsed,
        "ops_per_sec": successful / overall_elapsed if overall_elapsed > 0 else 0,
        "latency_p50_ms": statistics.median(latencies) if latencies else 0,
        "latency_p95_ms": statistics.quantiles(latencies, n=20)[18] if len(latencies) >= 20 else max(latencies) if latencies else 0,
    }


def print_results(name: str, results: dict):
    """Print benchmark results in a formatted way"""
    print(f"  {name}:")
    print(f"    Throughput: {results['ops_per_sec']:,.0f} ops/sec")
    print(f"    Total ops:  {results['total']:,}")
    print(f"    Time:       {results['elapsed_sec']:.2f}s")
    print(f"    Latency:    p50={results['latency_p50_ms']:.2f}ms, p95={results.get('latency_p95_ms', 0):.2f}ms")
    print()


def main():
    global DISTX_URL
    
    parser = argparse.ArgumentParser(description="DistX Performance Benchmark")
    parser.add_argument("--url", default=DISTX_URL, help="DistX server URL")
    parser.add_argument("--quick", action="store_true", help="Run quick benchmark only")
    parser.add_argument("--full", action="store_true", help="Run full benchmark suite")
    args = parser.parse_args()
    
    DISTX_URL = args.url
    
    print("=" * 70)
    print("DistX Performance Benchmark")
    print("=" * 70)
    print(f"Server: {DISTX_URL}")
    print()
    
    # Check server
    if not check_server():
        print("ERROR: DistX server is not running!")
        print(f"Please start it with: cargo run --release")
        print(f"Expected at: {DISTX_URL}")
        sys.exit(1)
    
    print("✓ Server is running")
    print()
    
    # Define test scenarios
    if args.quick:
        scenarios = [
            {"name": "Quick", "vectors": 1000, "dim": 128, "searches": 100},
        ]
    elif args.full:
        scenarios = [
            {"name": "Small-128d", "vectors": 1000, "dim": 128, "searches": 500},
            {"name": "Medium-128d", "vectors": 10000, "dim": 128, "searches": 1000},
            {"name": "Large-128d", "vectors": 50000, "dim": 128, "searches": 1000},
            {"name": "Small-512d", "vectors": 1000, "dim": 512, "searches": 500},
            {"name": "Medium-512d", "vectors": 10000, "dim": 512, "searches": 500},
        ]
    else:
        scenarios = [
            {"name": "Small-128d", "vectors": 1000, "dim": 128, "searches": 200},
            {"name": "Medium-128d", "vectors": 10000, "dim": 128, "searches": 500},
            {"name": "Small-512d", "vectors": 1000, "dim": 512, "searches": 200},
        ]
    
    all_results = []
    
    for scenario in scenarios:
        name = scenario["name"]
        num_vectors = scenario["vectors"]
        dim = scenario["dim"]
        num_searches = scenario["searches"]
        collection_name = f"bench_{name.lower().replace('-', '_')}"
        
        print("=" * 70)
        print(f"Scenario: {name}")
        print(f"  Vectors: {num_vectors:,}, Dimension: {dim}, Searches: {num_searches:,}")
        print("=" * 70)
        print()
        
        # Create collection
        if not create_collection(collection_name, dim):
            print(f"Failed to create collection for {name}")
            continue
        
        # Insert benchmark
        print("INSERT BENCHMARK")
        insert_results = benchmark_insert(collection_name, num_vectors, dim)
        print_results("Insert", insert_results)
        
        # Search benchmark
        print("SEARCH BENCHMARK (single-threaded)")
        search_results = benchmark_search(collection_name, num_searches, dim)
        print_results("Search", search_results)
        
        # Concurrent search benchmark
        print("CONCURRENT SEARCH BENCHMARK")
        for threads in [2, 4, 8]:
            concurrent_results = benchmark_concurrent_search(
                collection_name, num_searches // 2, dim, threads
            )
            print_results(f"Search ({threads} threads)", concurrent_results)
        
        # Cleanup
        delete_collection(collection_name)
        
        all_results.append({
            "scenario": name,
            "insert": insert_results,
            "search": search_results,
        })
    
    # Summary
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"{'Scenario':<15} {'Insert ops/s':<15} {'Search ops/s':<15} {'Search p50':<12}")
    print("-" * 70)
    for r in all_results:
        print(f"{r['scenario']:<15} {r['insert']['ops_per_sec']:<15,.0f} {r['search']['ops_per_sec']:<15,.0f} {r['search']['latency_p50_ms']:<12.2f}ms")
    print()
    print("Benchmark complete!")
    print()


if __name__ == "__main__":
    main()
