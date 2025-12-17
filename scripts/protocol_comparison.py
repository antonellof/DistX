#!/usr/bin/env python3
"""
Protocol Comparison: REST vs gRPC
Compares DistX REST, DistX gRPC, Qdrant REST, Qdrant gRPC, and Redis
"""
import requests
import redis
import grpc
import time
import random
import statistics
import struct
import subprocess
import sys
import os
from typing import List, Dict
from concurrent.futures import ThreadPoolExecutor

# Add generated proto path
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROTO_DIR = os.path.join(SCRIPT_DIR, "..", "lib", "api", "proto")

# Configuration
DISTX_REST_URL = "http://localhost:6333"
DISTX_GRPC_HOST = "localhost:6334"
QDRANT_REST_URL = "http://localhost:16333"
QDRANT_GRPC_HOST = "localhost:16334"
REDIS_HOST = "localhost"
REDIS_PORT = 6379

# Test parameters
DIM = 128
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


# ============================================================================
# DistX REST Benchmark
# ============================================================================
def benchmark_distx_rest(num_vectors: int, num_searches: int) -> Dict:
    """Benchmark DistX REST API"""
    name = "DistX REST"
    url = DISTX_REST_URL
    collection = "bench_distx_rest"
    
    try:
        r = requests.get(f"{url}/healthz", timeout=2)
        if r.status_code != 200:
            return {"name": name, "available": False}
    except:
        return {"name": name, "available": False}
    
    # Setup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    requests.put(f"{url}/collections/{collection}", json={
        "vectors": {"size": DIM, "distance": "Cosine"},
        "use_hnsw": True
    }, timeout=5)
    
    # Insert
    start = time.time()
    for i in range(0, num_vectors, BATCH_SIZE):
        batch_end = min(i + BATCH_SIZE, num_vectors)
        points = [{"id": j, "vector": generate_vector(DIM)} for j in range(i, batch_end)]
        requests.put(f"{url}/collections/{collection}/points", json={"points": points}, timeout=60)
    insert_time = time.time() - start
    
    # Search
    latencies = []
    for _ in range(num_searches):
        query = generate_vector(DIM)
        start = time.time()
        requests.post(f"{url}/collections/{collection}/points/search",
                     json={"vector": query, "limit": 10}, timeout=10)
        latencies.append((time.time() - start) * 1000)
    
    # Cleanup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    
    return {
        "name": name,
        "available": True,
        "insert_ops": num_vectors / insert_time,
        "search_ops": num_searches / (sum(latencies) / 1000),
        "p50": statistics.median(latencies),
        "p99": sorted(latencies)[int(len(latencies) * 0.99)]
    }


# ============================================================================
# DistX gRPC Benchmark
# ============================================================================
def benchmark_distx_grpc(num_vectors: int, num_searches: int) -> Dict:
    """Benchmark DistX gRPC API"""
    name = "DistX gRPC"
    
    try:
        # Generate Python gRPC stubs if needed
        proto_file = os.path.join(PROTO_DIR, "distx.proto")
        if not os.path.exists(proto_file):
            return {"name": name, "available": False, "error": "Proto file not found"}
        
        # Compile proto to Python (in-memory)
        import grpc_tools.protoc
        grpc_tools.protoc.main([
            'grpc_tools.protoc',
            f'--proto_path={PROTO_DIR}',
            f'--python_out={SCRIPT_DIR}',
            f'--grpc_python_out={SCRIPT_DIR}',
            proto_file
        ])
        
        # Import generated modules
        sys.path.insert(0, SCRIPT_DIR)
        import distx_pb2
        import distx_pb2_grpc
        
        # Connect
        channel = grpc.insecure_channel(DISTX_GRPC_HOST)
        stub = distx_pb2_grpc.DistXStub(channel)
        
        collection = "bench_distx_grpc"
        
        # Create collection
        try:
            stub.CreateCollection(distx_pb2.CreateCollectionRequest(
                name=collection,
                vector_dim=DIM,
                distance="Cosine"
            ))
        except:
            pass
        
        # Insert
        start = time.time()
        for i in range(0, num_vectors, BATCH_SIZE):
            batch_end = min(i + BATCH_SIZE, num_vectors)
            points = []
            for j in range(i, batch_end):
                point = distx_pb2.Point(
                    id_integer=j,
                    vector=generate_vector(DIM)
                )
                points.append(point)
            
            stub.UpsertPoints(distx_pb2.UpsertPointsRequest(
                collection_name=collection,
                points=points
            ))
        insert_time = time.time() - start
        
        # Search
        latencies = []
        for _ in range(num_searches):
            query = generate_vector(DIM)
            start = time.time()
            stub.SearchPoints(distx_pb2.SearchPointsRequest(
                collection_name=collection,
                vector=query,
                limit=10
            ))
            latencies.append((time.time() - start) * 1000)
        
        channel.close()
        
        return {
            "name": name,
            "available": True,
            "insert_ops": num_vectors / insert_time,
            "search_ops": num_searches / (sum(latencies) / 1000),
            "p50": statistics.median(latencies),
            "p99": sorted(latencies)[int(len(latencies) * 0.99)]
        }
        
    except Exception as e:
        return {"name": name, "available": False, "error": str(e)}


# ============================================================================
# Qdrant REST Benchmark  
# ============================================================================
def benchmark_qdrant_rest(num_vectors: int, num_searches: int) -> Dict:
    """Benchmark Qdrant REST API"""
    name = "Qdrant REST"
    url = QDRANT_REST_URL
    collection = "bench_qdrant_rest"
    
    try:
        r = requests.get(f"{url}/", timeout=2)
        if "qdrant" not in r.json().get("title", "").lower():
            return {"name": name, "available": False}
    except:
        return {"name": name, "available": False}
    
    # Setup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    requests.put(f"{url}/collections/{collection}", json={
        "vectors": {"size": DIM, "distance": "Cosine"}
    }, timeout=5)
    
    # Insert
    start = time.time()
    for i in range(0, num_vectors, BATCH_SIZE):
        batch_end = min(i + BATCH_SIZE, num_vectors)
        points = [{"id": j, "vector": generate_vector(DIM)} for j in range(i, batch_end)]
        requests.put(f"{url}/collections/{collection}/points", json={"points": points}, timeout=60)
    insert_time = time.time() - start
    
    # Search
    latencies = []
    for _ in range(num_searches):
        query = generate_vector(DIM)
        start = time.time()
        requests.post(f"{url}/collections/{collection}/points/search",
                     json={"vector": query, "limit": 10}, timeout=10)
        latencies.append((time.time() - start) * 1000)
    
    # Cleanup
    requests.delete(f"{url}/collections/{collection}", timeout=5)
    
    return {
        "name": name,
        "available": True,
        "insert_ops": num_vectors / insert_time,
        "search_ops": num_searches / (sum(latencies) / 1000),
        "p50": statistics.median(latencies),
        "p99": sorted(latencies)[int(len(latencies) * 0.99)]
    }


# ============================================================================
# Qdrant gRPC Benchmark
# ============================================================================
def benchmark_qdrant_grpc(num_vectors: int, num_searches: int) -> Dict:
    """Benchmark Qdrant gRPC API"""
    name = "Qdrant gRPC"
    
    try:
        from qdrant_client import QdrantClient
        from qdrant_client.models import Distance, VectorParams, PointStruct
        
        client = QdrantClient(host="localhost", port=16333, grpc_port=16334, prefer_grpc=True)
        collection = "bench_qdrant_grpc"
        
        # Setup
        try:
            client.delete_collection(collection)
        except:
            pass
        
        client.create_collection(
            collection_name=collection,
            vectors_config=VectorParams(size=DIM, distance=Distance.COSINE)
        )
        
        # Insert
        start = time.time()
        for i in range(0, num_vectors, BATCH_SIZE):
            batch_end = min(i + BATCH_SIZE, num_vectors)
            points = [
                PointStruct(id=j, vector=generate_vector(DIM))
                for j in range(i, batch_end)
            ]
            client.upsert(collection_name=collection, points=points)
        insert_time = time.time() - start
        
        # Search
        latencies = []
        for _ in range(num_searches):
            query = generate_vector(DIM)
            start = time.time()
            client.query_points(collection_name=collection, query=query, limit=10)
            latencies.append((time.time() - start) * 1000)
        
        # Cleanup
        client.delete_collection(collection)
        
        return {
            "name": name,
            "available": True,
            "insert_ops": num_vectors / insert_time,
            "search_ops": num_searches / (sum(latencies) / 1000),
            "p50": statistics.median(latencies),
            "p99": sorted(latencies)[int(len(latencies) * 0.99)]
        }
        
    except ImportError:
        return {"name": name, "available": False, "error": "qdrant-client not installed"}
    except Exception as e:
        return {"name": name, "available": False, "error": str(e)}


# ============================================================================
# Redis Benchmark
# ============================================================================
def benchmark_redis(num_vectors: int, num_searches: int) -> Dict:
    """Benchmark Redis Stack"""
    name = "Redis RESP"
    collection = "bench_redis"
    
    try:
        client = redis.Redis(host=REDIS_HOST, port=REDIS_PORT, decode_responses=False)
        client.ping()
    except:
        return {"name": name, "available": False}
    
    # Setup
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
        return {"name": name, "available": False, "error": str(e)}
    
    # Insert
    start = time.time()
    for i in range(0, num_vectors, BATCH_SIZE):
        batch_end = min(i + BATCH_SIZE, num_vectors)
        pipe = client.pipeline()
        for j in range(i, batch_end):
            key = f"{collection}:{j}"
            vec = generate_vector(DIM)
            vec_bytes = vector_to_bytes(vec)
            pipe.hset(key, mapping={"vector": vec_bytes, "id": str(j)})
        pipe.execute()
    insert_time = time.time() - start
    
    # Search
    latencies = []
    for _ in range(num_searches):
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
        except:
            pass
    
    # Cleanup
    try:
        client.execute_command("FT.DROPINDEX", collection, "DD")
    except:
        pass
    
    if not latencies:
        return {"name": name, "available": False, "error": "Search failed"}
    
    return {
        "name": name,
        "available": True,
        "insert_ops": num_vectors / insert_time,
        "search_ops": len(latencies) / (sum(latencies) / 1000),
        "p50": statistics.median(latencies),
        "p99": sorted(latencies)[int(len(latencies) * 0.99)]
    }


def print_results(results: List[Dict], title: str):
    """Print formatted results table"""
    print(f"\n{'='*80}")
    print(f"{title}")
    print(f"{'='*80}")
    print(f"{'System':<20} {'Insert ops/s':<15} {'Search ops/s':<15} {'p50 (ms)':<12} {'p99 (ms)':<12}")
    print("-" * 80)
    
    for r in results:
        if r.get("available"):
            print(f"{r['name']:<20} {r['insert_ops']:<15,.0f} {r['search_ops']:<15,.0f} "
                  f"{r['p50']:<12.2f} {r['p99']:<12.2f}")
        else:
            error = r.get("error", "Not available")
            print(f"{r['name']:<20} {'N/A':<15} {'N/A':<15} {'N/A':<12} {error}")


def main():
    print("=" * 80)
    print("PROTOCOL COMPARISON: REST vs gRPC")
    print("DistX, Qdrant, and Redis")
    print("=" * 80)
    
    # Test with 10K vectors
    num_vectors = 10000
    num_searches = 500
    
    print(f"\nConfiguration:")
    print(f"  Vectors: {num_vectors:,}")
    print(f"  Dimension: {DIM}")
    print(f"  Searches: {num_searches}")
    
    results = []
    
    print("\nRunning benchmarks...")
    
    # DistX REST
    print("  Testing DistX REST...")
    results.append(benchmark_distx_rest(num_vectors, num_searches))
    
    # DistX gRPC
    print("  Testing DistX gRPC...")
    results.append(benchmark_distx_grpc(num_vectors, num_searches))
    
    # Qdrant REST
    print("  Testing Qdrant REST...")
    results.append(benchmark_qdrant_rest(num_vectors, num_searches))
    
    # Qdrant gRPC  
    print("  Testing Qdrant gRPC...")
    results.append(benchmark_qdrant_grpc(num_vectors, num_searches))
    
    # Redis
    print("  Testing Redis RESP...")
    results.append(benchmark_redis(num_vectors, num_searches))
    
    # Print results
    print_results(results, f"RESULTS ({num_vectors:,} vectors, {num_searches} searches)")
    
    # Protocol comparison
    print("\n" + "=" * 80)
    print("PROTOCOL SPEEDUP ANALYSIS")
    print("=" * 80)
    
    distx_rest = next((r for r in results if r["name"] == "DistX REST" and r.get("available")), None)
    distx_grpc = next((r for r in results if r["name"] == "DistX gRPC" and r.get("available")), None)
    qdrant_rest = next((r for r in results if r["name"] == "Qdrant REST" and r.get("available")), None)
    qdrant_grpc = next((r for r in results if r["name"] == "Qdrant gRPC" and r.get("available")), None)
    
    if distx_rest and distx_grpc:
        insert_speedup = distx_grpc["insert_ops"] / distx_rest["insert_ops"]
        search_speedup = distx_grpc["search_ops"] / distx_rest["search_ops"]
        print(f"\nDistX gRPC vs REST:")
        print(f"  Insert: {insert_speedup:.2f}x speedup")
        print(f"  Search: {search_speedup:.2f}x speedup")
    
    if qdrant_rest and qdrant_grpc:
        insert_speedup = qdrant_grpc["insert_ops"] / qdrant_rest["insert_ops"]
        search_speedup = qdrant_grpc["search_ops"] / qdrant_rest["search_ops"]
        print(f"\nQdrant gRPC vs REST:")
        print(f"  Insert: {insert_speedup:.2f}x speedup")
        print(f"  Search: {search_speedup:.2f}x speedup")
    
    if distx_grpc and qdrant_grpc:
        insert_ratio = distx_grpc["insert_ops"] / qdrant_grpc["insert_ops"]
        search_ratio = distx_grpc["search_ops"] / qdrant_grpc["search_ops"]
        print(f"\nDistX gRPC vs Qdrant gRPC:")
        print(f"  Insert: {insert_ratio:.2f}x {'faster' if insert_ratio > 1 else 'slower'}")
        print(f"  Search: {search_ratio:.2f}x {'faster' if search_ratio > 1 else 'slower'}")
    
    print("\n" + "=" * 80)
    print("Benchmark complete!")
    print("=" * 80)


if __name__ == "__main__":
    main()
