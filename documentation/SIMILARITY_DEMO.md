# 🚀 DistX Similarity Engine Demo

> **Query tabular data by example. No embeddings. No ML. Just results.**

## What You'll See

The Similarity Engine lets you:
- ✅ **Import CSV data** without generating embeddings
- ✅ **Query by example** with natural JSON
- ✅ **Get explainable results** with per-field contribution breakdown
- ✅ **Override weights dynamically** at query time

---

## Run the Demo

```bash
# Start DistX
docker run -d -p 6333:6333 distx:similarity

# Run the demo
python scripts/similarity_demo.py
```

---

## Demo Output

```
╔═══════════════════════════════════════════════════════════════════╗
║                                                                   ║
║      DistX Similarity Engine Demo                                 ║
║      Schema-driven similarity for tabular data                    ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝

🔌 Connecting to DistX at http://localhost:6333...
   ✅ Connected!
```

---

## Demo 1: E-Commerce Product Similarity

### Step 1: Create Collection with Similarity Schema

```
📦 Creating collection with similarity schema...
   ✅ Collection created: ok

📋 Similarity Schema:
┌──────────┬─────────────┬──────────┬────────┐
│ Field    │ Type        │ Distance │ Weight │
├──────────┼─────────────┼──────────┼────────┤
│ name     │ text        │ semantic │ 0.40   │
│ price    │ number      │ relative │ 0.25   │
│ category │ categorical │ exact    │ 0.20   │
│ brand    │ categorical │ exact    │ 0.10   │
│ in_stock │ boolean     │ -        │ 0.05   │
└──────────┴─────────────┴──────────┴────────┘
```

### Step 2: Import Products (No Vectors Needed!)

```
📥 Importing products from CSV (no vectors needed!)...
   ✅ Imported 25 products
   📊 Auto-generated vector dimension: 194
```

The magic: **vectors are automatically generated** from your payload based on the schema!

### Step 3: Query by Example

**Query:** *"Find products similar to 'prosciutto crudo' around $8"*

```json
{
  "example": {
    "name": "prosciutto crudo",
    "price": 8.0,
    "category": "salumi"
  },
  "limit": 5
}
```

**Results with Explainability:**

```
┌──────┬────┬───────┬───────────────────────────┬───────┬─────────┬──────────────────────────┐
│ Rank │ ID │ Score │ Name                      │ Price │ Category│ Top Contributing Field   │
├──────┼────┼───────┼───────────────────────────┼───────┼─────────┼──────────────────────────┤
│  1   │ 3  │ 0.705 │ Prosciutto crudo San D... │ $9.49 │ salumi  │ name: 0.219 ✓            │
│  2   │ 2  │ 0.679 │ Prosciutto cotto          │ $4.99 │ salumi  │ name: 0.248 ✓            │
│  3   │ 1  │ 0.635 │ Prosciutto di Parma DOP   │ $8.99 │ salumi  │ price: 0.222 ✓           │
│  4   │ 7  │ 0.525 │ Coppa di Parma            │ $7.99 │ salumi  │ price: 0.250 ✓           │
│  5   │ 10 │ 0.522 │ Speck Alto Adige          │ $8.49 │ salumi  │ price: 0.236 ✓           │
└──────┴────┴───────┴───────────────────────────┴───────┴─────────┴──────────────────────────┘
```

### Step 4: Dynamic Weight Overrides

**Query:** *"Find similar to iPhone but cheaper (boost price weight)"*

```json
{
  "example": {"name": "iPhone", "category": "electronics"},
  "weights": {"price": 0.6, "name": 0.2},
  "limit": 5
}
```

**Results:**

```
┌──────┬────┬───────┬───────────────────────┬──────────┬─────────────┬────────────────────┐
│ Rank │ ID │ Score │ Name                  │ Price    │ Category    │ Top Field          │
├──────┼────┼───────┼───────────────────────┼──────────┼─────────────┼────────────────────┤
│  1   │ 12 │ 0.633 │ iPhone 15             │ $999.00  │ electronics │ name: 0.233        │
│  2   │ 11 │ 0.540 │ iPhone 15 Pro Max     │ $1199.00 │ electronics │ category: 0.200    │
│  3   │ 17 │ 0.400 │ Dell XPS 15           │ $1799.00 │ electronics │ category: 0.200    │
│  4   │ 18 │ 0.400 │ Sony WH-1000XM5       │ $349.00  │ electronics │ category: 0.200    │
│  5   │ 19 │ 0.400 │ AirPods Pro 2         │ $249.00  │ electronics │ category: 0.200    │
└──────┴────┴───────┴───────────────────────┴──────────┴─────────────┴────────────────────┘
```

### Step 5: Query by Existing Point ID

**Query:** *"Find products similar to ID 20 (Parmigiano Reggiano)"*

```json
{"like_id": 20, "limit": 5}
```

**Results:**

```
┌──────┬────┬───────┬─────────────────────────┬────────┬────────┬────────────────────┐
│ Rank │ ID │ Score │ Name                    │ Price  │ Category│ Top Field         │
├──────┼────┼───────┼─────────────────────────┼────────┼────────┼────────────────────┤
│  1   │ 20 │ 1.000 │ Parmigiano Reggiano DOP │ $18.99 │ cheese │ name: 0.400 ✓      │
│  2   │ 21 │ 0.551 │ Grana Padano DOP        │ $14.99 │ cheese │ category: 0.200    │
│  3   │ 24 │ 0.534 │ Pecorino Romano DOP     │ $12.99 │ cheese │ category: 0.200    │
│  4   │ 25 │ 0.432 │ Taleggio DOP            │ $9.99  │ cheese │ category: 0.200    │
│  5   │ 23 │ 0.410 │ Gorgonzola DOP          │ $8.49  │ cheese │ category: 0.200    │
└──────┴────┴───────┴─────────────────────────┴────────┴────────┴────────────────────┘
```

---

## Demo 2: ERP Supplier Matching

### Different Domain, Same Power

```
🏭 Creating suppliers collection...
   ✅ Collection created: ok

📥 Importing suppliers...
   ✅ Imported 10 suppliers
```

**Query:** *"Find suppliers similar to: Manufacturing, ~$5M revenue, certified, Milan"*

```json
{
  "example": {
    "industry": "manufacturing",
    "annual_revenue": 5000000,
    "certified": true,
    "location": "Milan"
  },
  "limit": 5
}
```

**Results:**

```
┌──────┬────┬───────┬───────────────────────────┬─────────────┬───────────┬───────────┬──────────┐
│ Rank │ ID │ Score │ Company                   │ Revenue     │ Employees │ Certified │ Location │
├──────┼────┼───────┼───────────────────────────┼─────────────┼───────────┼───────────┼──────────┤
│  1   │ 1  │ 0.800 │ Acme Industrial Solutions │ $5,000,000  │ 150       │ ✓         │ Milan    │
│  2   │ 7  │ 0.683 │ TechParts Global          │ $12,000,000 │ 350       │ ✓         │ Milan    │
│  3   │ 9  │ 0.680 │ Innovation Works          │ $4,500,000  │ 120       │ ✓         │ Genoa    │
│  4   │ 5  │ 0.654 │ Quality First Industries  │ $6,500,000  │ 180       │ ✓         │ Florence │
│  5   │ 4  │ 0.628 │ Precision Components Ltd  │ $3,200,000  │ 90        │ ✓         │ Bologna  │
└──────┴────┴───────┴───────────────────────────┴─────────────┴───────────┴───────────┴──────────┘
```

---

## Demo 3: Explainability Deep Dive

### See Exactly Why Each Result Matched

```
🔬 Analyzing similarity breakdown for 'prosciutto' query...

  Result #1: Prosciutto cotto
  ──────────────────────────────────────────────────
  Total Score: 0.7745

  Field Contributions:
    name         [██████████████████████████████] 0.3000
    price        [████████████████████████░░░░░░] 0.2495
    category     [███████████████████░░░░░░░░░░░] 0.2000
    in_stock     [██░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 0.0250
    brand        [░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 0.0000

  Result #2: Prosciutto di Parma DOP
  ──────────────────────────────────────────────────
  Total Score: 0.6333

  Field Contributions:
    category     [██████████████████████████████] 0.2000
    name         [█████████████████████████░░░░░] 0.1692
    price        [████████████████████░░░░░░░░░░] 0.1390
    brand        [███████████████░░░░░░░░░░░░░░░] 0.1000
    in_stock     [███░░░░░░░░░░░░░░░░░░░░░░░░░░░] 0.0250

  Result #3: Prosciutto crudo San Daniele
  ──────────────────────────────────────────────────
  Total Score: 0.4987

  Field Contributions:
    category     [██████████████████████████████] 0.2000
    name         [█████████████████████░░░░░░░░░] 0.1419
    price        [███████████████████░░░░░░░░░░░] 0.1317
    in_stock     [███░░░░░░░░░░░░░░░░░░░░░░░░░░░] 0.0250
    brand        [░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 0.0000
```

---

## Try It Yourself

```bash
# Quick curl commands to try:

# Get the schema
curl -s "http://localhost:6333/collections/demo_products/similarity-schema" | jq .

# Query by example
curl -s -X POST "http://localhost:6333/collections/demo_products/similar" \
     -H "Content-Type: application/json" \
     -d '{"example": {"name": "prosciutto", "price": 5}, "limit": 3}' | jq .

# Query with weight overrides
curl -s -X POST "http://localhost:6333/collections/demo_products/similar" \
     -H "Content-Type: application/json" \
     -d '{"example": {"name": "cheese"}, "weights": {"price": 0.7}, "limit": 3}' | jq .
```

---

## Key Takeaways

| Feature | Benefit |
|---------|---------|
| **No embeddings needed** | Skip OpenAI, skip ML pipelines - just define field weights |
| **Auto-generated vectors** | Payload → vector conversion happens automatically |
| **Query by example** | Natural JSON queries, not cryptic vector arrays |
| **Explainable results** | Know exactly *why* each result matched |
| **Dynamic weights** | Adjust what matters at query time |
| **Works with any data** | Products, suppliers, users, documents - any tabular data |

---

## Performance

```
⏱️  Total demo time: 0.04s
📊 Imported: 35 records across 2 collections
🔍 Queries: 5 similarity searches with reranking
```

---

## Next Steps

- 📖 [Full Documentation](SIMILARITY_ENGINE.md)
- 🔧 [API Reference](API.md)
- 🐳 [Docker Guide](DOCKER.md)
