# DistX

[![Crates.io](https://img.shields.io/crates/v/distx.svg)](https://crates.io/crates/distx)
[![Documentation](https://docs.rs/distx/badge.svg)](https://docs.rs/distx)
[![Docker](https://img.shields.io/docker/v/distx/distx?label=docker)](https://hub.docker.com/r/distx/distx)
[![License](https://img.shields.io/crates/l/distx.svg)](https://github.com/antonellof/DistX#license)

A high-performance vector database with **schema-driven similarity search**.

DistX combines the speed of a Rust-native vector database with an innovative Similarity Engine that enables structured queries on tabular data — with full explainability and without external ML dependencies.

- **Qdrant API Compatible** — Drop-in replacement, use existing client libraries
- **Schema-Driven Similarity** — Define field types and weights declaratively
- **Explainable Results** — Per-field contribution breakdown for every match
- **Zero ML Dependencies** — No OpenAI, no embeddings pipeline, works offline

---

## How It Works

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Traditional Vector Database                                             │
│  ───────────────────────────                                             │
│  Your Data → External ML API → Embeddings → Vector DB → Score: 0.87     │
│              (cost per call)   (black box)              (unexplained)    │
├──────────────────────────────────────────────────────────────────────────┤
│  DistX Similarity Engine                                                 │
│  ───────────────────────────                                             │
│  Your Data → Schema (JSON) → Auto-Embedding → Explainable Results       │
│              (declarative)    (deterministic)  (name: 0.25, price: 0.22) │
└──────────────────────────────────────────────────────────────────────────┘
```

📖 [Detailed comparison with Qdrant, Pinecone, Elasticsearch →](documentation/COMPARISON.md)

---

## Similarity Engine

**The first schema-driven similarity engine with built-in explainability.**

Define what fields matter, insert your data, and query by example — vectors are generated automatically. No external ML services, no embedding pipelines, no black-box scores.

```bash
# 1. Define similarity schema
curl -X PUT http://localhost:6333/collections/products -H "Content-Type: application/json" -d '{
  "similarity_schema": {
    "fields": {
      "name": {"type": "text", "weight": 0.4},
      "price": {"type": "number", "distance": "relative", "weight": 0.3},
      "category": {"type": "categorical", "weight": 0.2},
      "brand": {"type": "categorical", "weight": 0.1}
    }
  }
}'

# 2. Insert data (vectors auto-generated)
curl -X PUT http://localhost:6333/collections/products/points -H "Content-Type: application/json" -d '{
  "points": [
    {"id": 1, "payload": {"name": "Prosciutto di Parma DOP", "price": 8.99, "category": "salumi", "brand": "Parma"}},
    {"id": 2, "payload": {"name": "Prosciutto cotto", "price": 4.99, "category": "salumi", "brand": "Negroni"}},
    {"id": 3, "payload": {"name": "iPhone 15 Pro", "price": 1199, "category": "electronics", "brand": "Apple"}}
  ]
}'

# 3. Query by example
curl -X POST http://localhost:6333/collections/products/similar -H "Content-Type: application/json" \
  -d '{"example": {"name": "prosciutto crudo", "price": 8.0, "category": "salumi"}, "limit": 3}'
```

**Response includes per-field explainability:**

```
┌──────┬─────────────────────────────┬───────┬──────────────────────────────────┐
│ Rank │ Product                     │ Score │ Contribution Breakdown           │
├──────┼─────────────────────────────┼───────┼──────────────────────────────────┤
│  1   │ Prosciutto di Parma DOP     │ 0.71  │ name: 0.22, price: 0.22          │
│  2   │ Prosciutto cotto            │ 0.68  │ name: 0.25, category: 0.20       │
│  3   │ Coppa di Parma              │ 0.53  │ category: 0.20, price: 0.25      │
└──────┴─────────────────────────────┴───────┴──────────────────────────────────┘
```

### Key Capabilities

| Capability | Description |
|------------|-------------|
| **Schema-Driven** | Declarative field definitions with typed similarity (text, number, categorical, boolean) |
| **Auto-Embedding** | Deterministic vector generation from structured payloads |
| **Query by Example** | Natural JSON queries instead of raw vectors |
| **Explainable Scoring** | Per-field contribution breakdown for every result |
| **Dynamic Weights** | Override field importance at query time without re-indexing |
| **Zero External Dependencies** | Fully self-contained, works offline and air-gapped |

```bash
# Override weights at query time
curl -X POST http://localhost:6333/collections/products/similar \
  -d '{"example": {"name": "iPhone"}, "weights": {"price": 0.7, "name": 0.1}}'
```

📖 [Documentation](documentation/SIMILARITY_ENGINE.md) · [Interactive Demo](documentation/SIMILARITY_DEMO.md) · [Comparison with Alternatives](documentation/COMPARISON.md)

---

## 100% Qdrant API Compatible

DistX maintains **full compatibility with the Qdrant API**, so you can:

- ✅ Use existing Qdrant client libraries (Python, JavaScript, Rust, Go)
- ✅ Drop-in replace Qdrant in your stack
- ✅ Use Qdrant's Web Dashboard UI
- ✅ Migrate with zero code changes

The Similarity Engine is **additive** — all standard vector operations work exactly like Qdrant:

```bash
# Standard Qdrant-compatible vector search still works!
curl -X POST http://localhost:6333/collections/my_collection/points/search \
  -d '{"vector": [0.1, 0.2, 0.3, ...], "limit": 10}'
```

---

## Quick Start

### 1. Start DistX with Docker

```bash
# Pull and run (with persistent storage)
docker run -d --name distx \
  -p 6333:6333 -p 6334:6334 \
  -v distx_data:/qdrant/storage \
  distx/distx:latest

# Or with docker-compose
docker-compose up -d
```

DistX is now running at:
- **REST API**: http://localhost:6333
- **Web Dashboard**: http://localhost:6333/dashboard
- **gRPC**: localhost:6334

### 2. Create a Collection with Similarity Schema

```bash
curl -X PUT http://localhost:6333/collections/products \
  -H "Content-Type: application/json" \
  -d '{
    "similarity_schema": {
      "fields": {
        "name": {"type": "text", "weight": 0.4},
        "price": {"type": "number", "distance": "relative", "weight": 0.3},
        "category": {"type": "categorical", "weight": 0.2},
        "in_stock": {"type": "boolean", "weight": 0.1}
      }
    }
  }'
```

### 3. Insert Data (No Vectors Needed!)

```bash
curl -X PUT http://localhost:6333/collections/products/points \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {"id": 1, "payload": {"name": "Prosciutto di Parma DOP", "price": 8.99, "category": "salumi", "in_stock": true}},
      {"id": 2, "payload": {"name": "Prosciutto cotto", "price": 4.99, "category": "salumi", "in_stock": true}},
      {"id": 3, "payload": {"name": "Mortadella Bologna", "price": 3.99, "category": "salumi", "in_stock": false}},
      {"id": 4, "payload": {"name": "Parmigiano Reggiano", "price": 18.99, "category": "cheese", "in_stock": true}},
      {"id": 5, "payload": {"name": "Grana Padano", "price": 14.99, "category": "cheese", "in_stock": true}}
    ]
  }'
```

### 4. Query by Example

```bash
# Find products similar to "prosciutto crudo around $8"
curl -X POST http://localhost:6333/collections/products/similar \
  -H "Content-Type: application/json" \
  -d '{
    "example": {
      "name": "prosciutto crudo",
      "price": 8.0,
      "category": "salumi"
    },
    "limit": 3
  }'
```

**Response with explainable scores:**

```json
{
  "result": [
    {
      "id": 1,
      "score": 0.71,
      "payload": {"name": "Prosciutto di Parma DOP", "price": 8.99, "category": "salumi"},
      "explain": {"name": 0.22, "price": 0.24, "category": 0.20, "in_stock": 0.05}
    },
    {
      "id": 2,
      "score": 0.65,
      "payload": {"name": "Prosciutto cotto", "price": 4.99, "category": "salumi"},
      "explain": {"name": 0.25, "price": 0.15, "category": 0.20, "in_stock": 0.05}
    }
  ]
}
```

### 5. Dynamic Weight Overrides

```bash
# Find cheaper alternatives (boost price importance)
curl -X POST http://localhost:6333/collections/products/similar \
  -H "Content-Type: application/json" \
  -d '{
    "example": {"name": "prosciutto", "category": "salumi"},
    "weights": {"price": 0.6, "name": 0.2, "category": 0.2},
    "limit": 3
  }'
```

### 6. Query by Existing Point ID

```bash
# Find products similar to ID 4 (Parmigiano Reggiano)
curl -X POST http://localhost:6333/collections/products/similar \
  -H "Content-Type: application/json" \
  -d '{"like_id": 4, "limit": 3}'
```

### 7. Run the Interactive Demo

```bash
# Full demo with sample data
python scripts/similarity_demo.py

# Or run specific demos
python scripts/similarity_demo.py --demo products
python scripts/similarity_demo.py --demo suppliers
```

### Alternative: Traditional Vector Search

DistX also supports standard Qdrant-compatible vector operations:

```bash
# Create collection with vectors
curl -X PUT http://localhost:6333/collections/embeddings \
  -H "Content-Type: application/json" \
  -d '{"vectors": {"size": 128, "distance": "Cosine"}}'

# Insert with vectors
curl -X PUT http://localhost:6333/collections/embeddings/points \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {"id": 1, "vector": [0.1, 0.2, ...], "payload": {"text": "example"}}
    ]
  }'

# Vector search
curl -X POST http://localhost:6333/collections/embeddings/points/search \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, ...], "limit": 10}'
```

### Installation Alternatives

```bash
# From crates.io
cargo install distx
distx --data-dir ./data

# From source
git clone https://github.com/antonellof/distx
cd distx && cargo build --release
./target/release/distx
```

---

## Performance

| Metric | Performance |
|--------|-------------|
| **Vector Insert** | ~8,000 ops/sec |
| **Vector Search** | ~400-500 ops/sec |
| **Search Latency (p50)** | ~2ms |
| **Search Latency (p99)** | ~5ms |
| **Similarity Query** | <1ms overhead |

*Benchmarks: 5,000 vectors, 128 dimensions, Cosine distance*

---

## All Features

### Similarity Engine (NEW)
- **Schema-driven similarity** — Define what fields matter
- **Auto-embedding** — Vectors generated from payload
- **Multi-type support** — Text, number, categorical, boolean
- **Explainable results** — Per-field score breakdown
- **Dynamic weights** — Override at query time

### Vector Database
- **HNSW Index** — Fast ANN with SIMD (AVX2, SSE, NEON)
- **BM25 Text Search** — Full-text ranking
- **Payload Filtering** — JSON metadata queries
- **Dual API** — REST + gRPC
- **Persistence** — WAL, snapshots, LMDB

### Operations
- **Single Binary** — ~6MB, no dependencies
- **Docker Ready** — Single command deployment
- **Web Dashboard** — Qdrant-compatible UI

---

## Documentation

| Guide | Description |
|-------|-------------|
| [**Similarity Engine**](documentation/SIMILARITY_ENGINE.md) | Schema-driven similarity for tabular data |
| [**Similarity Demo**](documentation/SIMILARITY_DEMO.md) | Interactive walkthrough with examples |
| [**Comparison**](documentation/COMPARISON.md) | DistX vs Qdrant, Pinecone, Elasticsearch |
| [Quick Start](documentation/QUICK_START.md) | Get started in 5 minutes |
| [Docker Guide](documentation/DOCKER.md) | Container deployment |
| [API Reference](documentation/API.md) | REST and gRPC endpoints |
| [Architecture](documentation/ARCHITECTURE.md) | System design |

---

## Use Cases

### 🛒 E-Commerce & Retail

**Problem:** "Show me products similar to this one" — but similarity means different things (style, price, brand).

```bash
# Similar products for "customers also viewed"
curl -X POST /collections/products/similar -d '{
  "example": {"name": "Nike Air Max 90", "price": 129, "category": "sneakers", "brand": "Nike"},
  "limit": 6
}'

# Budget alternatives (boost price importance)
curl -X POST /collections/products/similar -d '{
  "like_id": 123,
  "weights": {"price": 0.6, "category": 0.3, "brand": 0.1}
}'
```

**Use cases:**
- "Similar products" on product pages
- "You might also like" recommendations  
- Competitor price matching (find similar products, compare prices)
- Inventory substitution (out of stock → suggest alternatives)

---

### 🏭 ERP & Supply Chain

**Problem:** Find the best supplier match based on multiple criteria without building ML pipelines.

```bash
# Find suppliers similar to your top performer
curl -X POST /collections/suppliers/similar -d '{
  "example": {
    "industry": "manufacturing",
    "annual_revenue": 5000000,
    "employee_count": 150,
    "certified": true,
    "location": "Milan"
  },
  "limit": 10
}'
```

**Use cases:**
- Supplier discovery and matching
- Vendor risk assessment (find similar vendors to flagged ones)
- Partner recommendations
- RFQ (Request for Quote) matching

---

### 👥 CRM & Customer Data

**Problem:** Find similar customers for segmentation, lead scoring, or churn prediction.

```bash
# Find customers similar to your best ones
curl -X POST /collections/customers/similar -d '{
  "example": {
    "industry": "fintech",
    "company_size": "enterprise",
    "annual_spend": 50000,
    "engagement_score": 85
  }
}'

# Find leads similar to closed-won deals
curl -X POST /collections/leads/similar -d '{
  "like_id": "deal_12345",
  "weights": {"deal_size": 0.4, "industry": 0.3, "company_size": 0.3}
}'
```

**Use cases:**
- Lead scoring (similar to converted leads?)
- Customer segmentation
- Churn prediction (similar to churned customers?)
- Account-based marketing (find lookalike companies)

---

### 🔍 Data Quality & Deduplication

**Problem:** Find duplicate or near-duplicate records without exact matching.

```bash
# Find potential duplicates
curl -X POST /collections/contacts/similar -d '{
  "example": {"name": "John Smith", "email": "j.smith@acme.com", "company": "Acme Inc"},
  "limit": 5
}'

# Response shows WHY records might be duplicates
# → name: 0.35 (similar names)
# → company: 0.25 (same company) 
# → email: 0.15 (different email domain)
```

**Use cases:**
- Contact/account deduplication
- Data cleansing before migration
- Master data management
- Merge candidate identification

---

### 📊 Data Analysis & Exploration

**Problem:** Explore datasets by finding similar records without writing complex SQL.

```bash
# "Find transactions similar to this suspicious one"
curl -X POST /collections/transactions/similar -d '{
  "example": {"amount": 9999, "merchant_category": "travel", "country": "unusual"},
  "weights": {"amount": 0.5, "merchant_category": 0.3}
}'

# "Find properties similar to this sold one"
curl -X POST /collections/properties/similar -d '{
  "example": {"sqft": 2500, "bedrooms": 4, "neighborhood": "downtown", "year_built": 2010}
}'
```

**Use cases:**
- Fraud pattern detection
- Anomaly investigation
- Comparable analysis (real estate, finance)
- Research dataset exploration

---

### ⚖️ Regulated Industries (Finance, Healthcare, Legal)

**Problem:** Need similarity search with full auditability — can't use black-box ML.

**Why DistX:**
- **Explainable scores** — Per-field contribution breakdown
- **Deterministic** — Same query always returns same explanation
- **Auditable** — Schema defines what matters, weights are transparent
- **No external APIs** — Data never leaves your infrastructure

```bash
# Healthcare: Find similar patient cases
curl -X POST /collections/patients/similar -d '{
  "example": {"diagnosis_code": "E11.9", "age_group": "65+", "comorbidities": 3}
}'

# Response includes full explanation for audit trail:
# {
#   "score": 0.78,
#   "explain": {
#     "diagnosis_code": 0.35,  ← Same diagnosis
#     "age_group": 0.25,       ← Same age bracket
#     "comorbidities": 0.18   ← Similar complexity
#   }
# }
```

**Use cases:**
- Clinical trial patient matching
- Insurance claim similarity
- Legal case precedent search
- Compliance reporting

---

### 🏠 Real Estate & Property

```bash
# Find comparable properties for valuation
curl -X POST /collections/properties/similar -d '{
  "example": {
    "sqft": 2200,
    "bedrooms": 3,
    "bathrooms": 2,
    "year_built": 2015,
    "neighborhood": "downtown",
    "property_type": "condo"
  },
  "weights": {"sqft": 0.3, "neighborhood": 0.25, "property_type": 0.2}
}'
```

**Use cases:**
- Comparable property analysis (comps)
- Property valuation
- Investment opportunity matching
- Tenant-property matching

---

### 🎯 HR & Recruiting

```bash
# Find candidates similar to your top performers
curl -X POST /collections/employees/similar -d '{
  "example": {
    "department": "engineering",
    "years_experience": 5,
    "skills": "rust,python",
    "performance_rating": "exceeds"
  }
}'
```

**Use cases:**
- Candidate matching to job requirements
- Internal mobility (find similar roles)
- Team composition analysis
- Succession planning

---

## Use as a Library

```toml
[dependencies]
distx = "0.2.5"
distx-similarity = "0.2.5"  # Similarity Engine
distx-core = "0.2.5"        # Core data structures
```

```rust
use distx_similarity::{SimilaritySchema, FieldConfig, StructuredEmbedder, Reranker};
use std::collections::HashMap;

// Define schema
let mut fields = HashMap::new();
fields.insert("name".to_string(), FieldConfig::text(0.5));
fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
fields.insert("category".to_string(), FieldConfig::categorical(0.2));

let schema = SimilaritySchema::new(fields);
let embedder = StructuredEmbedder::new(schema.clone());

// Auto-generate vector from payload
let payload = json!({"name": "Prosciutto", "price": 8.99, "category": "salumi"});
let vector = embedder.embed(&payload);
```

---

## Links

- **Crates.io**: https://crates.io/crates/distx
- **Documentation**: https://docs.rs/distx
- **GitHub**: https://github.com/antonellof/distx

## License

Licensed under MIT OR Apache-2.0 at your option.
