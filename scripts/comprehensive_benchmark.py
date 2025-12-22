#!/usr/bin/env python3
"""
Comprehensive Performance Comparison: vectX vs Qdrant
Tests all major API features including new multivector support

Features tested:
- Dense vector insert/search
- Multivector (ColBERT MaxSim) insert/search
- Query with grouping
- Batch operations
- Count/Scroll operations
- Point retrieval
"""
import requests
import time
import random
import statistics
import sys
from typing import List, Dict, Optional
from dataclasses import dataclass
import argparse
import json

# Configuration
VECTX_URL = "http://localhost:6333"
QDRANT_URL = "http://localhost:16333"


def generate_vector(dim: int) -> List[float]:
    """Generate random normalized vector"""
    vec = [random.uniform(-1.0, 1.0) for _ in range(dim)]
    norm = sum(x * x for x in vec) ** 0.5
    if norm > 0:
        vec = [x / norm for x in vec]
    return vec


def generate_multivector(num_vecs: int, dim: int) -> List[List[float]]:
    """Generate random multivector (ColBERT-style)"""
    return [generate_vector(dim) for _ in range(num_vecs)]


@dataclass
class BenchmarkResult:
    name: str
    operation: str
    ops_per_sec: float
    latency_p50_ms: float
    latency_p99_ms: float
    success_rate: float
    total_time_sec: float


class VectorDBBenchmark:
    def __init__(self, name: str, url: str):
        self.name = name
        self.url = url
        self.results: List[BenchmarkResult] = []
    
    def is_available(self) -> bool:
        try:
            r = requests.get(f"{self.url}/", timeout=2)
            return r.status_code == 200
        except:
            return False
    
    def get_version(self) -> str:
        try:
            r = requests.get(f"{self.url}/", timeout=2)
            data = r.json()
            return data.get("version", "unknown")
        except:
            return "unknown"
    
    def delete_collection(self, collection: str):
        try:
            requests.delete(f"{self.url}/collections/{collection}", timeout=5)
        except:
            pass
    
    def create_collection(self, collection: str, dim: int, enable_multivector: bool = False) -> bool:
        try:
            config = {"vectors": {"size": dim, "distance": "Cosine"}}
            if enable_multivector:
                config["vectors"]["multivector_config"] = {"comparator": "max_sim"}
            r = requests.put(f"{self.url}/collections/{collection}", json=config, timeout=5)
            return r.status_code in [200, 201]
        except:
            return False
    
    def upsert_points(self, collection: str, points: List[Dict]) -> float:
        """Insert points and return latency in ms"""
        start = time.time()
        try:
            r = requests.put(f"{self.url}/collections/{collection}/points", 
                           json={"points": points}, timeout=60)
            elapsed = (time.time() - start) * 1000
            return elapsed if r.status_code == 200 else -1
        except:
            return -1
    
    def search_vector(self, collection: str, vector: List[float], limit: int = 10) -> float:
        """Search with single vector, return latency in ms"""
        start = time.time()
        try:
            r = requests.post(f"{self.url}/collections/{collection}/points/search",
                            json={"vector": vector, "limit": limit}, timeout=10)
            elapsed = (time.time() - start) * 1000
            return elapsed if r.status_code == 200 else -1
        except:
            return -1
    
    def query_multivector(self, collection: str, multivector: List[List[float]], limit: int = 10) -> float:
        """Query with multivector (MaxSim), return latency in ms"""
        start = time.time()
        try:
            r = requests.post(f"{self.url}/collections/{collection}/points/query",
                            json={"query": multivector, "limit": limit, "with_payload": True}, timeout=10)
            elapsed = (time.time() - start) * 1000
            return elapsed if r.status_code == 200 else -1
        except:
            return -1
    
    def query_groups(self, collection: str, vector: List[float], group_by: str, 
                    limit: int = 5, group_size: int = 3) -> float:
        """Query with grouping, return latency in ms"""
        start = time.time()
        try:
            r = requests.post(f"{self.url}/collections/{collection}/points/query/groups",
                            json={
                                "query": vector, 
                                "group_by": group_by,
                                "limit": limit,
                                "group_size": group_size,
                                "with_payload": True
                            }, timeout=10)
            elapsed = (time.time() - start) * 1000
            return elapsed if r.status_code == 200 else -1
        except:
            return -1
    
    def scroll_points(self, collection: str, limit: int = 100) -> float:
        """Scroll through points, return latency in ms"""
        start = time.time()
        try:
            r = requests.post(f"{self.url}/collections/{collection}/points/scroll",
                            json={"limit": limit, "with_payload": True}, timeout=10)
            elapsed = (time.time() - start) * 1000
            return elapsed if r.status_code == 200 else -1
        except:
            return -1
    
    def count_points(self, collection: str) -> float:
        """Count points, return latency in ms"""
        start = time.time()
        try:
            r = requests.post(f"{self.url}/collections/{collection}/points/count",
                            json={}, timeout=10)
            elapsed = (time.time() - start) * 1000
            return elapsed if r.status_code == 200 else -1
        except:
            return -1
    
    def get_points_by_ids(self, collection: str, ids: List[int]) -> float:
        """Get points by IDs, return latency in ms"""
        start = time.time()
        try:
            r = requests.post(f"{self.url}/collections/{collection}/points",
                            json={"ids": ids, "with_payload": True, "with_vector": True}, timeout=10)
            elapsed = (time.time() - start) * 1000
            return elapsed if r.status_code == 200 else -1
        except:
            return -1
    
    def run_latency_test(self, operation: str, func, num_iterations: int) -> BenchmarkResult:
        """Run a latency test and collect statistics"""
        latencies = []
        failures = 0
        
        for _ in range(num_iterations):
            result = func()
            if result > 0:
                latencies.append(result)
            else:
                failures += 1
        
        if not latencies:
            return BenchmarkResult(
                name=self.name,
                operation=operation,
                ops_per_sec=0,
                latency_p50_ms=0,
                latency_p99_ms=0,
                success_rate=0,
                total_time_sec=0
            )
        
        total_time = sum(latencies) / 1000
        ops_per_sec = len(latencies) / total_time if total_time > 0 else 0
        p50 = statistics.median(latencies)
        sorted_lat = sorted(latencies)
        p99_idx = min(int(len(sorted_lat) * 0.99), len(sorted_lat) - 1)
        p99 = sorted_lat[p99_idx]
        
        return BenchmarkResult(
            name=self.name,
            operation=operation,
            ops_per_sec=ops_per_sec,
            latency_p50_ms=p50,
            latency_p99_ms=p99,
            success_rate=len(latencies) / (len(latencies) + failures) * 100,
            total_time_sec=total_time
        )


def run_comprehensive_benchmark(db: VectorDBBenchmark, config: Dict) -> List[BenchmarkResult]:
    """Run all benchmark tests on a database"""
    results = []
    collection = "comprehensive_bench"
    dim = config["dim"]
    num_vectors = config["num_vectors"]
    num_searches = config["num_searches"]
    batch_size = config["batch_size"]
    multivec_size = config["multivec_size"]
    
    print(f"\n{'='*60}")
    print(f"Benchmarking {db.name} (v{db.get_version()})")
    print(f"{'='*60}")
    
    # Cleanup
    db.delete_collection(collection)
    
    # Create collection
    if not db.create_collection(collection, dim, enable_multivector=True):
        print(f"  ❌ Failed to create collection")
        return results
    
    # ==== 1. Dense Vector Insert ====
    print(f"\n  📊 Dense Vector Insert ({num_vectors:,} vectors, batch={batch_size})...")
    insert_latencies = []
    insert_start = time.time()
    total_inserted = 0
    
    groups = ["group_a", "group_b", "group_c", "group_d", "group_e"]
    
    for i in range(0, num_vectors, batch_size):
        batch_end = min(i + batch_size, num_vectors)
        points = [
            {
                "id": j, 
                "vector": generate_vector(dim),
                "payload": {"group_id": random.choice(groups), "index": j}
            } 
            for j in range(i, batch_end)
        ]
        lat = db.upsert_points(collection, points)
        if lat > 0:
            insert_latencies.append(lat)
            total_inserted += len(points)
    
    insert_total_time = time.time() - insert_start
    insert_ops = total_inserted / insert_total_time if insert_total_time > 0 else 0
    
    results.append(BenchmarkResult(
        name=db.name,
        operation="Dense Insert",
        ops_per_sec=insert_ops,
        latency_p50_ms=statistics.median(insert_latencies) if insert_latencies else 0,
        latency_p99_ms=sorted(insert_latencies)[int(len(insert_latencies)*0.99)] if insert_latencies else 0,
        success_rate=100.0,
        total_time_sec=insert_total_time
    ))
    print(f"     ✅ {insert_ops:,.0f} ops/sec ({insert_total_time:.2f}s)")
    
    # ==== 2. Dense Vector Search ====
    print(f"\n  🔍 Dense Vector Search ({num_searches:,} queries)...")
    result = db.run_latency_test(
        "Dense Search",
        lambda: db.search_vector(collection, generate_vector(dim), 10),
        num_searches
    )
    results.append(result)
    print(f"     ✅ {result.ops_per_sec:,.0f} ops/sec (p50: {result.latency_p50_ms:.2f}ms, p99: {result.latency_p99_ms:.2f}ms)")
    
    # ==== 3. Multivector Insert ====
    mv_collection = "multivector_bench"
    db.delete_collection(mv_collection)
    db.create_collection(mv_collection, dim, enable_multivector=True)
    
    num_mv_vectors = num_vectors // 10  # Fewer multivectors (they're larger)
    print(f"\n  📊 Multivector Insert ({num_mv_vectors:,} multivectors, {multivec_size} sub-vectors each)...")
    
    mv_insert_start = time.time()
    mv_latencies = []
    for i in range(0, num_mv_vectors, batch_size):
        batch_end = min(i + batch_size, num_mv_vectors)
        points = [
            {
                "id": j, 
                "vector": generate_multivector(multivec_size, dim),
                "payload": {"group_id": random.choice(groups), "index": j}
            } 
            for j in range(i, batch_end)
        ]
        lat = db.upsert_points(mv_collection, points)
        if lat > 0:
            mv_latencies.append(lat)
    
    mv_insert_time = time.time() - mv_insert_start
    mv_insert_ops = num_mv_vectors / mv_insert_time if mv_insert_time > 0 else 0
    
    results.append(BenchmarkResult(
        name=db.name,
        operation="Multivector Insert",
        ops_per_sec=mv_insert_ops,
        latency_p50_ms=statistics.median(mv_latencies) if mv_latencies else 0,
        latency_p99_ms=sorted(mv_latencies)[int(len(mv_latencies)*0.99)] if len(mv_latencies) > 1 else (mv_latencies[0] if mv_latencies else 0),
        success_rate=100.0,
        total_time_sec=mv_insert_time
    ))
    print(f"     ✅ {mv_insert_ops:,.0f} ops/sec ({mv_insert_time:.2f}s)")
    
    # ==== 4. Multivector Query (MaxSim) ====
    print(f"\n  🔍 Multivector Query/MaxSim ({num_searches//2:,} queries)...")
    result = db.run_latency_test(
        "Multivector Query",
        lambda: db.query_multivector(mv_collection, generate_multivector(multivec_size, dim), 10),
        num_searches // 2
    )
    results.append(result)
    print(f"     ✅ {result.ops_per_sec:,.0f} ops/sec (p50: {result.latency_p50_ms:.2f}ms, p99: {result.latency_p99_ms:.2f}ms)")
    
    # ==== 5. Query Groups ====
    print(f"\n  📂 Query Groups ({num_searches//4:,} queries)...")
    result = db.run_latency_test(
        "Query Groups",
        lambda: db.query_groups(collection, generate_vector(dim), "group_id", 5, 3),
        num_searches // 4
    )
    results.append(result)
    print(f"     ✅ {result.ops_per_sec:,.0f} ops/sec (p50: {result.latency_p50_ms:.2f}ms, p99: {result.latency_p99_ms:.2f}ms)")
    
    # ==== 6. Count Points ====
    print(f"\n  📊 Count Points ({num_searches//4:,} queries)...")
    result = db.run_latency_test(
        "Count Points",
        lambda: db.count_points(collection),
        num_searches // 4
    )
    results.append(result)
    print(f"     ✅ {result.ops_per_sec:,.0f} ops/sec (p50: {result.latency_p50_ms:.2f}ms)")
    
    # ==== 7. Scroll Points ====
    print(f"\n  📜 Scroll Points ({num_searches//4:,} queries)...")
    result = db.run_latency_test(
        "Scroll Points",
        lambda: db.scroll_points(collection, 100),
        num_searches // 4
    )
    results.append(result)
    print(f"     ✅ {result.ops_per_sec:,.0f} ops/sec (p50: {result.latency_p50_ms:.2f}ms)")
    
    # ==== 8. Get Points by IDs ====
    print(f"\n  📥 Get Points by IDs ({num_searches//4:,} queries)...")
    result = db.run_latency_test(
        "Get Points",
        lambda: db.get_points_by_ids(collection, random.sample(range(num_vectors), min(10, num_vectors))),
        num_searches // 4
    )
    results.append(result)
    print(f"     ✅ {result.ops_per_sec:,.0f} ops/sec (p50: {result.latency_p50_ms:.2f}ms)")
    
    # Cleanup
    db.delete_collection(collection)
    db.delete_collection(mv_collection)
    
    return results


def print_comparison_table(all_results: Dict[str, List[BenchmarkResult]]):
    """Print a comparison table of results"""
    print("\n" + "=" * 100)
    print("COMPREHENSIVE BENCHMARK RESULTS")
    print("=" * 100)
    
    # Get all unique operations
    operations = []
    for results in all_results.values():
        for r in results:
            if r.operation not in operations:
                operations.append(r.operation)
    
    # Print header
    db_names = list(all_results.keys())
    header = f"{'Operation':<25}"
    for name in db_names:
        header += f" | {name:^30}"
    print(header)
    print("-" * 100)
    
    # Print each operation row
    for op in operations:
        row = f"{op:<25}"
        for name in db_names:
            result = next((r for r in all_results[name] if r.operation == op), None)
            if result and result.ops_per_sec > 0:
                cell = f"{result.ops_per_sec:>8,.0f} ops/s (p50:{result.latency_p50_ms:>5.1f}ms)"
            else:
                cell = "N/A"
            row += f" | {cell:^30}"
        print(row)
    
    print("-" * 100)
    
    # Print relative performance if both DBs available
    if len([r for results in all_results.values() for r in results if r.ops_per_sec > 0]) > 0:
        print("\nRelative Performance (vectX vs Qdrant):")
        print("-" * 60)
        
        for op in operations:
            vectx_result = next((r for r in all_results.get("vectX", []) if r.operation == op), None)
            qdrant_result = next((r for r in all_results.get("Qdrant", []) if r.operation == op), None)
            
            if vectx_result and qdrant_result and vectx_result.ops_per_sec > 0 and qdrant_result.ops_per_sec > 0:
                ratio = vectx_result.ops_per_sec / qdrant_result.ops_per_sec
                if ratio >= 1:
                    emoji = "🚀" if ratio >= 1.5 else "✅"
                    print(f"  {op:<25} {emoji} vectX is {ratio:.2f}x faster")
                else:
                    print(f"  {op:<25} ⚠️  Qdrant is {1/ratio:.2f}x faster")


def main():
    parser = argparse.ArgumentParser(description="Comprehensive vectX vs Qdrant benchmark")
    parser.add_argument("--vectors", type=int, default=5000, help="Number of vectors to insert")
    parser.add_argument("--dim", type=int, default=128, help="Vector dimension")
    parser.add_argument("--searches", type=int, default=500, help="Number of search operations")
    parser.add_argument("--batch-size", type=int, default=100, help="Batch size for inserts")
    parser.add_argument("--multivec-size", type=int, default=4, help="Number of sub-vectors in multivector")
    parser.add_argument("--vectx-url", type=str, default="http://localhost:6333", help="vectX URL")
    parser.add_argument("--qdrant-url", type=str, default="http://localhost:16333", help="Qdrant URL")
    parser.add_argument("--vectx-only", action="store_true", help="Only benchmark vectX")
    parser.add_argument("--qdrant-only", action="store_true", help="Only benchmark Qdrant")
    args = parser.parse_args()
    
    config = {
        "dim": args.dim,
        "num_vectors": args.vectors,
        "num_searches": args.searches,
        "batch_size": args.batch_size,
        "multivec_size": args.multivec_size
    }
    
    print("=" * 70)
    print("COMPREHENSIVE VECTOR DATABASE BENCHMARK")
    print("vectX vs Qdrant - Full API Comparison")
    print("=" * 70)
    print(f"\nConfiguration:")
    print(f"  Vectors:      {config['num_vectors']:,}")
    print(f"  Dimension:    {config['dim']}")
    print(f"  Searches:     {config['num_searches']:,}")
    print(f"  Batch Size:   {config['batch_size']}")
    print(f"  Multivec:     {config['multivec_size']} sub-vectors")
    
    all_results = {}
    
    # Benchmark vectX
    if not args.qdrant_only:
        vectx = VectorDBBenchmark("vectX", args.vectx_url)
        if vectx.is_available():
            all_results["vectX"] = run_comprehensive_benchmark(vectx, config)
        else:
            print(f"\n⚠️  vectX not available at {args.vectx_url}")
    
    # Benchmark Qdrant
    if not args.vectx_only:
        qdrant = VectorDBBenchmark("Qdrant", args.qdrant_url)
        if qdrant.is_available():
            all_results["Qdrant"] = run_comprehensive_benchmark(qdrant, config)
        else:
            print(f"\n⚠️  Qdrant not available at {args.qdrant_url}")
    
    if all_results:
        print_comparison_table(all_results)
    else:
        print("\n❌ No databases available to benchmark!")
        print("\nTo run the benchmark:")
        print("  1. Start vectX:  docker run -p 6333:6333 vectx:latest")
        print("  2. Start Qdrant: docker run -p 16333:6333 qdrant/qdrant")
        print("  3. Run:          python scripts/comprehensive_benchmark.py")
    
    print("\n" + "=" * 70)
    print("Benchmark complete!")
    print("=" * 70)


if __name__ == "__main__":
    main()
