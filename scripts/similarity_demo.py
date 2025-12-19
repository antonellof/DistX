#!/usr/bin/env python3
"""
DistX Similarity Engine Demo

This script demonstrates the Similarity Engine features:
1. Creating collections with similarity schemas
2. Importing data from CSV (auto-embedding)
3. Querying by example with explainable results
4. Using weight overrides to modify query behavior

Usage:
    python similarity_demo.py [--url URL] [--csv FILE]

Requirements:
    pip install requests tabulate
"""

import argparse
import csv
import json
import sys
import time
from io import StringIO
from typing import Optional

try:
    import requests
except ImportError:
    print("Error: 'requests' package required. Install with: pip install requests")
    sys.exit(1)

try:
    from tabulate import tabulate
except ImportError:
    # Fallback if tabulate not installed
    def tabulate(data, headers, tablefmt=None):
        result = " | ".join(headers) + "\n"
        result += "-" * 60 + "\n"
        for row in data:
            result += " | ".join(str(x) for x in row) + "\n"
        return result

# Default DistX URL
DEFAULT_URL = "http://localhost:6333"

# Sample product data (embedded CSV)
SAMPLE_PRODUCTS_CSV = """id,name,price,category,brand,in_stock
1,Prosciutto di Parma DOP,8.99,salumi,Parma,true
2,Prosciutto cotto,4.99,salumi,Negroni,true
3,Prosciutto crudo San Daniele,9.49,salumi,San Daniele,true
4,Mortadella Bologna IGP,3.99,salumi,Felsineo,true
5,Salame Milano,5.49,salumi,Citterio,true
6,Bresaola della Valtellina,12.99,salumi,Rigamonti,false
7,Coppa di Parma,7.99,salumi,Parma,true
8,Guanciale,6.49,salumi,Norcia,true
9,Pancetta tesa,5.99,salumi,Negroni,true
10,Speck Alto Adige,8.49,salumi,Recla,true
11,iPhone 15 Pro Max,1199.00,electronics,Apple,true
12,iPhone 15,999.00,electronics,Apple,true
13,Samsung Galaxy S24 Ultra,1149.00,electronics,Samsung,true
14,Samsung Galaxy S24,849.00,electronics,Samsung,true
15,Google Pixel 8 Pro,999.00,electronics,Google,true
16,MacBook Pro 14,1999.00,electronics,Apple,false
17,Dell XPS 15,1799.00,electronics,Dell,true
18,Sony WH-1000XM5,349.00,electronics,Sony,true
19,AirPods Pro 2,249.00,electronics,Apple,true
20,Parmigiano Reggiano DOP,18.99,cheese,Parma,true
21,Grana Padano DOP,14.99,cheese,Latteria,true
22,Mozzarella di Bufala,6.99,cheese,Campana,true
23,Gorgonzola DOP,8.49,cheese,Igor,true
24,Pecorino Romano DOP,12.99,cheese,Brunelli,true
25,Taleggio DOP,9.99,cheese,Arrigoni,false
"""

# Sample suppliers data (embedded CSV)
SAMPLE_SUPPLIERS_CSV = """id,company_name,industry,annual_revenue,employee_count,certified,location
1,Acme Industrial Solutions,manufacturing,5000000,150,true,Milan
2,Global Tech Manufacturing,manufacturing,8000000,250,true,Turin
3,Small Parts Inc,manufacturing,1500000,45,false,Rome
4,Precision Components Ltd,manufacturing,3200000,90,true,Bologna
5,Quality First Industries,manufacturing,6500000,180,true,Florence
6,Eco Manufacturing Co,manufacturing,2800000,75,true,Naples
7,TechParts Global,manufacturing,12000000,350,true,Milan
8,Local Fabricators,manufacturing,800000,25,false,Venice
9,Innovation Works,manufacturing,4500000,120,true,Genoa
10,Premium Supplies SpA,manufacturing,9500000,280,true,Turin
"""


class SimilarityDemo:
    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip("/")
        
    def _request(self, method: str, endpoint: str, data: dict = None) -> dict:
        """Make HTTP request to DistX"""
        url = f"{self.base_url}{endpoint}"
        try:
            if method == "GET":
                resp = requests.get(url, timeout=30)
            elif method == "PUT":
                resp = requests.put(url, json=data, timeout=30)
            elif method == "POST":
                resp = requests.post(url, json=data, timeout=30)
            elif method == "DELETE":
                resp = requests.delete(url, timeout=30)
            else:
                raise ValueError(f"Unknown method: {method}")
            
            return resp.json()
        except requests.exceptions.ConnectionError:
            print(f"\n❌ Error: Cannot connect to DistX at {self.base_url}")
            print("   Make sure DistX is running: docker run -p 6333:6333 distx:similarity")
            sys.exit(1)
            
    def health_check(self) -> bool:
        """Check if DistX is running"""
        try:
            resp = self._request("GET", "/healthz")
            return "version" in resp
        except:
            return False
    
    def delete_collection(self, name: str):
        """Delete collection if exists"""
        self._request("DELETE", f"/collections/{name}")
        
    def create_collection_with_schema(self, name: str, schema: dict) -> dict:
        """Create collection with similarity schema"""
        return self._request("PUT", f"/collections/{name}", {
            "similarity_schema": schema
        })
    
    def get_schema(self, name: str) -> dict:
        """Get similarity schema for collection"""
        return self._request("GET", f"/collections/{name}/similarity-schema")
    
    def insert_points(self, name: str, points: list) -> dict:
        """Insert points (auto-embedding if schema exists)"""
        return self._request("PUT", f"/collections/{name}/points", {
            "points": points
        })
    
    def similar_query(self, name: str, example: dict = None, like_id: int = None,
                     weights: dict = None, limit: int = 5) -> dict:
        """Query for similar items"""
        query = {"limit": limit}
        if example:
            query["example"] = example
        if like_id is not None:
            query["like_id"] = like_id
        if weights:
            query["weights"] = weights
        return self._request("POST", f"/collections/{name}/similar", query)
    
    def get_collection_info(self, name: str) -> dict:
        """Get collection info"""
        return self._request("GET", f"/collections/{name}")


def parse_csv(csv_content: str) -> list:
    """Parse CSV content into list of dicts"""
    reader = csv.DictReader(StringIO(csv_content.strip()))
    rows = []
    for row in reader:
        # Convert types
        parsed = {}
        for key, value in row.items():
            if value.lower() == "true":
                parsed[key] = True
            elif value.lower() == "false":
                parsed[key] = False
            elif value.replace(".", "").replace("-", "").isdigit():
                parsed[key] = float(value) if "." in value else int(value)
            else:
                parsed[key] = value
        rows.append(parsed)
    return rows


def print_header(text: str):
    """Print section header"""
    print(f"\n{'='*70}")
    print(f"  {text}")
    print(f"{'='*70}\n")


def print_results(results: list, show_explain: bool = True):
    """Print similarity results in a nice table"""
    if not results:
        print("  No results found.")
        return
    
    headers = ["Rank", "ID", "Score", "Name/Company", "Price/Revenue", "Category/Industry"]
    if show_explain:
        headers.append("Top Contributing Field")
    
    rows = []
    for i, result in enumerate(results, 1):
        payload = result.get("payload", {})
        
        # Get name (works for products and suppliers)
        name = payload.get("name") or payload.get("company_name", "N/A")
        if len(name) > 25:
            name = name[:22] + "..."
            
        # Get price/revenue
        price = payload.get("price") or payload.get("annual_revenue", "N/A")
        if isinstance(price, (int, float)):
            if price > 10000:
                price = f"${price:,.0f}"
            else:
                price = f"${price:.2f}"
        
        # Get category/industry
        category = payload.get("category") or payload.get("industry", "N/A")
        
        row = [i, result.get("id"), f"{result.get('score', 0):.3f}", name, price, category]
        
        if show_explain:
            explain = result.get("explain", {})
            if explain:
                top_field = max(explain.items(), key=lambda x: x[1])
                row.append(f"{top_field[0]}: {top_field[1]:.3f}")
            else:
                row.append("-")
        
        rows.append(row)
    
    print(tabulate(rows, headers=headers, tablefmt="simple"))


def demo_products(demo: SimilarityDemo, csv_data: str = None):
    """Demo: E-commerce product similarity"""
    print_header("DEMO 1: E-Commerce Product Similarity")
    
    collection = "demo_products"
    
    # 1. Create collection with schema
    print("📦 Creating collection with similarity schema...")
    demo.delete_collection(collection)
    
    schema = {
        "version": 1,
        "fields": {
            "name": {
                "type": "text",
                "distance": "semantic",
                "weight": 0.4
            },
            "price": {
                "type": "number",
                "distance": "relative",
                "weight": 0.25
            },
            "category": {
                "type": "categorical",
                "distance": "exact",
                "weight": 0.2
            },
            "brand": {
                "type": "categorical",
                "distance": "exact",
                "weight": 0.1
            },
            "in_stock": {
                "type": "boolean",
                "weight": 0.05
            }
        }
    }
    
    result = demo.create_collection_with_schema(collection, schema)
    print(f"   ✅ Collection created: {result.get('status')}")
    
    # 2. Show schema
    print("\n📋 Similarity Schema:")
    schema_resp = demo.get_schema(collection)
    fields = schema_resp.get("result", {}).get("fields", {})
    schema_table = [[name, cfg.get("type"), cfg.get("distance", "-"), f"{cfg.get('weight', 0):.2f}"]
                    for name, cfg in sorted(fields.items())]
    print(tabulate(schema_table, headers=["Field", "Type", "Distance", "Weight"], tablefmt="simple"))
    
    # 3. Import data
    print("\n📥 Importing products from CSV (no vectors needed!)...")
    csv_content = csv_data or SAMPLE_PRODUCTS_CSV
    products = parse_csv(csv_content)
    
    points = [{"id": p.pop("id"), "payload": p} for p in products]
    result = demo.insert_points(collection, points)
    print(f"   ✅ Imported {len(points)} products")
    
    # 4. Show collection info
    info = demo.get_collection_info(collection)
    info_result = info.get("result", {})
    vector_size = info_result.get("config", {}).get("params", {}).get("vectors", {}).get("size", "N/A")
    print(f"   📊 Auto-generated vector dimension: {vector_size}")
    
    # 5. Query examples
    print("\n" + "-"*70)
    print("🔍 QUERY 1: Find products similar to 'prosciutto crudo' around $8")
    print("-"*70)
    
    result = demo.similar_query(collection, example={
        "name": "prosciutto crudo",
        "price": 8.0,
        "category": "salumi"
    }, limit=5)
    
    print_results(result.get("result", {}).get("result", []))
    
    print("\n" + "-"*70)
    print("🔍 QUERY 2: Find similar to iPhone but cheaper (boost price weight)")
    print("-"*70)
    
    result = demo.similar_query(collection, example={
        "name": "iPhone",
        "category": "electronics"
    }, weights={"price": 0.6, "name": 0.2}, limit=5)
    
    print_results(result.get("result", {}).get("result", []))
    
    print("\n" + "-"*70)
    print("🔍 QUERY 3: Find products similar to ID 20 (Parmigiano Reggiano)")
    print("-"*70)
    
    result = demo.similar_query(collection, like_id=20, limit=5)
    print_results(result.get("result", {}).get("result", []))


def demo_suppliers(demo: SimilarityDemo, csv_data: str = None):
    """Demo: ERP supplier matching"""
    print_header("DEMO 2: ERP Supplier Matching")
    
    collection = "demo_suppliers"
    
    # 1. Create collection
    print("🏭 Creating suppliers collection...")
    demo.delete_collection(collection)
    
    schema = {
        "version": 1,
        "fields": {
            "company_name": {
                "type": "text",
                "distance": "semantic",
                "weight": 0.25
            },
            "industry": {
                "type": "categorical",
                "distance": "exact",
                "weight": 0.2
            },
            "annual_revenue": {
                "type": "number",
                "distance": "relative",
                "weight": 0.2
            },
            "employee_count": {
                "type": "number",
                "distance": "relative",
                "weight": 0.15
            },
            "certified": {
                "type": "boolean",
                "weight": 0.1
            },
            "location": {
                "type": "categorical",
                "distance": "exact",
                "weight": 0.1
            }
        }
    }
    
    result = demo.create_collection_with_schema(collection, schema)
    print(f"   ✅ Collection created: {result.get('status')}")
    
    # 2. Import data
    print("\n📥 Importing suppliers...")
    csv_content = csv_data or SAMPLE_SUPPLIERS_CSV
    suppliers = parse_csv(csv_content)
    
    points = [{"id": s.pop("id"), "payload": s} for s in suppliers]
    result = demo.insert_points(collection, points)
    print(f"   ✅ Imported {len(points)} suppliers")
    
    # 3. Queries
    print("\n" + "-"*70)
    print("🔍 Find suppliers similar to: Manufacturing, ~$5M revenue, certified, Milan")
    print("-"*70)
    
    result = demo.similar_query(collection, example={
        "industry": "manufacturing",
        "annual_revenue": 5000000,
        "certified": True,
        "location": "Milan"
    }, limit=5)
    
    results = result.get("result", {}).get("result", [])
    if results:
        headers = ["Rank", "ID", "Score", "Company", "Revenue", "Employees", "Certified", "Location"]
        rows = []
        for i, r in enumerate(results, 1):
            p = r.get("payload", {})
            rows.append([
                i, r.get("id"), f"{r.get('score', 0):.3f}",
                p.get("company_name", "")[:25],
                f"${p.get('annual_revenue', 0):,.0f}",
                p.get("employee_count", 0),
                "✓" if p.get("certified") else "✗",
                p.get("location", "")
            ])
        print(tabulate(rows, headers=headers, tablefmt="simple"))


def demo_explain(demo: SimilarityDemo):
    """Demo: Explainability feature"""
    print_header("DEMO 3: Explainability Deep Dive")
    
    collection = "demo_products"
    
    print("🔬 Analyzing similarity breakdown for 'prosciutto' query...")
    print()
    
    result = demo.similar_query(collection, example={
        "name": "prosciutto",
        "price": 5.0,
        "category": "salumi",
        "brand": "Parma"
    }, limit=3)
    
    results = result.get("result", {}).get("result", [])
    
    for i, r in enumerate(results, 1):
        payload = r.get("payload", {})
        explain = r.get("explain", {})
        
        print(f"  Result #{i}: {payload.get('name', 'N/A')}")
        print(f"  {'─'*50}")
        print(f"  Total Score: {r.get('score', 0):.4f}")
        print()
        print("  Field Contributions:")
        
        # Sort by contribution
        sorted_explain = sorted(explain.items(), key=lambda x: x[1], reverse=True)
        max_contrib = max(explain.values()) if explain else 1
        
        for field, contrib in sorted_explain:
            bar_len = int(30 * contrib / max_contrib) if max_contrib > 0 else 0
            bar = "█" * bar_len + "░" * (30 - bar_len)
            print(f"    {field:12} [{bar}] {contrib:.4f}")
        
        print()


def main():
    parser = argparse.ArgumentParser(
        description="DistX Similarity Engine Demo",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python similarity_demo.py                    # Run with sample data
  python similarity_demo.py --csv products.csv # Use custom CSV
  python similarity_demo.py --url http://localhost:6333
        """
    )
    parser.add_argument("--url", default=DEFAULT_URL,
                       help=f"DistX server URL (default: {DEFAULT_URL})")
    parser.add_argument("--csv", type=str, default=None,
                       help="Path to CSV file for products demo")
    parser.add_argument("--demo", choices=["products", "suppliers", "explain", "all"],
                       default="all", help="Which demo to run")
    
    args = parser.parse_args()
    
    # Banner
    print("""
    ╔═══════════════════════════════════════════════════════════════════╗
    ║                                                                   ║
    ║      DistX Similarity Engine Demo                                 ║
    ║      Schema-driven similarity for tabular data                    ║
    ║                                                                   ║
    ╚═══════════════════════════════════════════════════════════════════╝
    """)
    
    # Initialize
    demo = SimilarityDemo(args.url)
    
    # Health check
    print(f"🔌 Connecting to DistX at {args.url}...")
    if not demo.health_check():
        print(f"\n❌ Cannot connect to DistX at {args.url}")
        print("   Start DistX with: docker run -p 6333:6333 distx:similarity")
        sys.exit(1)
    print("   ✅ Connected!")
    
    # Load custom CSV if provided
    csv_data = None
    if args.csv:
        try:
            with open(args.csv) as f:
                csv_data = f.read()
            print(f"   📄 Loaded CSV: {args.csv}")
        except Exception as e:
            print(f"   ⚠️  Could not load CSV: {e}")
            print("   Using sample data instead.")
    
    # Run demos
    start = time.time()
    
    if args.demo in ("products", "all"):
        demo_products(demo, csv_data)
    
    if args.demo in ("suppliers", "all"):
        demo_suppliers(demo)
    
    if args.demo in ("explain", "all"):
        demo_explain(demo)
    
    elapsed = time.time() - start
    
    # Summary
    print_header("Demo Complete")
    print(f"  ⏱️  Total time: {elapsed:.2f}s")
    print(f"  🌐 DistX URL: {args.url}")
    print(f"  📊 Web UI: {args.url}/dashboard")
    print()
    print("  ┌─────────────────────────────────────────────────────────────┐")
    print("  │  🎉 What you just saw:                                      │")
    print("  │                                                             │")
    print("  │  ✓ Imported CSV data with NO external embeddings           │")
    print("  │  ✓ Queried by example using natural JSON                   │")
    print("  │  ✓ Got explainable results with per-field contributions    │")
    print("  │  ✓ Used dynamic weight overrides at query time             │")
    print("  └─────────────────────────────────────────────────────────────┘")
    print()
    print("  📖 Documentation: documentation/SIMILARITY_ENGINE.md")
    print("  🚀 Full demo:     documentation/SIMILARITY_DEMO.md")
    print()
    print("  Try these curl commands:")
    print()
    print(f'  curl -s "{args.url}/collections/demo_products/similarity-schema" | jq .')
    print()
    print(f'  curl -s -X POST "{args.url}/collections/demo_products/similar" \\')
    print('       -H "Content-Type: application/json" \\')
    print('       -d \'{"example": {"name": "prosciutto", "price": 5}, "limit": 3}\' | jq .')
    print()


if __name__ == "__main__":
    main()
