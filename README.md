# DistX

[![Crates.io](https://img.shields.io/crates/v/distx.svg)](https://crates.io/crates/distx)
[![Documentation](https://docs.rs/distx/badge.svg)](https://docs.rs/distx)
[![Docker](https://img.shields.io/docker/v/distx/distx?label=docker)](https://hub.docker.com/r/distx/distx)
[![License](https://img.shields.io/crates/l/distx.svg)](https://github.com/antonellof/DistX#license)

A high-performance vector database with **Similarity Schema** — a structured reranking layer that adds explainability to vector search results.

---

## What Makes DistX Different

DistX combines traditional vector search with **structured similarity reranking**:

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Traditional Vector Database                                             │
│  ───────────────────────────                                             │
│  Query Vector → ANN Search → Results (score: 0.87)                      │
│                              (black box, no explanation)                 │
├──────────────────────────────────────────────────────────────────────────┤
│  DistX with Similarity Schema                                            │
│  ────────────────────────────                                            │
│  Query Vector → ANN Search → Structured Reranking → Explainable Results │
│                              (schema-driven)        (name: 0.25, price: 0.22)
└──────────────────────────────────────────────────────────────────────────┘
```

| DistX IS | DistX is NOT |
|----------|--------------|
| A vector database with structured reranking | An embedding generation service |
| Explainable per-field similarity scores | A black-box neural system |
| 100% Qdrant API compatible | A replacement for OpenAI/Cohere embeddings |
| Designed for structured/tabular data | Limited to unstructured text |

---

## How It Works

1. **You generate embeddings** using your preferred provider (OpenAI, Cohere, etc.)
2. **DistX stores vectors + payload** like any vector database
3. **Similarity Schema reranks results** with structured field comparisons
4. **You get explainable scores** showing why each result matched

```bash
# 1. Create collection with vectors AND similarity schema
curl -X PUT http://localhost:6333/collections/products -H "Content-Type: application/json" -d '{
  "vectors": {"size": 1536, "distance": "Cosine"},
  "similarity_schema": {
    "fields": {
      "name": {"type": "text", "weight": 0.4, "distance": "semantic"},
      "price": {"type": "number", "distance": "relative", "weight": 0.3},
      "category": {"type": "categorical", "weight": 0.2},
      "brand": {"type": "categorical", "weight": 0.1}
    }
  }
}'

# 2. Insert data WITH your embeddings
curl -X PUT http://localhost:6333/collections/products/points -H "Content-Type: application/json" -d '{
  "points": [
    {"id": 1, "vector": [0.1, 0.2, ...], "payload": {"name": "iPhone 15 Pro", "price": 1199, "category": "electronics", "brand": "Apple"}},
    {"id": 2, "vector": [0.15, 0.25, ...], "payload": {"name": "Galaxy S24", "price": 999, "category": "electronics", "brand": "Samsung"}}
  ]
}'

# 3. Query by example (uses schema for reranking)
curl -X POST http://localhost:6333/collections/products/similar -H "Content-Type: application/json" \
  -d '{"example": {"name": "smartphone", "price": 1000, "category": "electronics"}, "limit": 5}'
```

**Response includes per-field explainability:**

```json
{
  "result": [
    {
      "id": 1,
      "score": 0.82,
      "payload": {"name": "iPhone 15 Pro", "price": 1199, "category": "electronics"},
      "explain": {"name": 0.32, "price": 0.25, "category": 0.20, "brand": 0.05}
    }
  ]
}
```

---

## Quick Start

### 1. Start DistX with Docker

```bash
docker run -d --name distx \
  -p 6333:6333 -p 6334:6334 \
  -v distx_data:/qdrant/storage \
  distx/distx:latest
```

### 2. Try the Data Chatbot Demo

A full Next.js app demonstrating DistX capabilities:

```bash
cd examples/data-chatbot
cp .env.example .env.local
# Add your OPENAI_API_KEY and POSTGRES_URL
pnpm install && pnpm db:migrate && pnpm dev
```

Upload CSV files, ask natural language questions, get explainable similarity results.

### 3. Use Standard Vector Search

DistX is 100% Qdrant API compatible:

```bash
# Create collection
curl -X PUT http://localhost:6333/collections/embeddings \
  -d '{"vectors": {"size": 1536, "distance": "Cosine"}}'

# Insert with vectors
curl -X PUT http://localhost:6333/collections/embeddings/points \
  -d '{"points": [{"id": 1, "vector": [0.1, 0.2, ...], "payload": {"text": "example"}}]}'

# Search
curl -X POST http://localhost:6333/collections/embeddings/points/search \
  -d '{"vector": [0.1, 0.2, ...], "limit": 10}'
```

---

## Similarity Schema

The schema defines how payload fields are compared during reranking:

```json
{
  "similarity_schema": {
    "fields": {
      "name": {"type": "text", "distance": "semantic", "weight": 0.4},
      "price": {"type": "number", "distance": "relative", "weight": 0.3},
      "category": {"type": "categorical", "distance": "exact", "weight": 0.2},
      "in_stock": {"type": "boolean", "distance": "exact", "weight": 0.1}
    }
  }
}
```

### Field Types

| Type | Distance Options | Description |
|------|------------------|-------------|
| `text` | `semantic` | Trigram-based fuzzy matching |
| `number` | `relative`, `absolute` | Percentage or absolute difference |
| `categorical` | `exact`, `overlap` | Exact match or set overlap |
| `boolean` | `exact` | True/false match |

### Dynamic Weights at Query Time

Change what "similar" means without re-indexing:

```bash
# Find cheaper alternatives (boost price)
curl -X POST /collections/products/similar -d '{
  "example": {"name": "iPhone 15"},
  "weights": {"price": 0.7, "name": 0.2, "category": 0.1}
}'

# Find same brand products (boost brand)
curl -X POST /collections/products/similar -d '{
  "example": {"name": "iPhone 15"},
  "weights": {"brand": 0.6, "category": 0.3, "name": 0.1}
}'
```

---

## 100% Qdrant API Compatible

Use existing Qdrant client libraries:

```typescript
import { QdrantClient } from '@qdrant/js-client-rest';

const client = new QdrantClient({ url: 'http://localhost:6333' });

// All standard operations work
await client.createCollection('my_collection', { vectors: { size: 1536, distance: 'Cosine' } });
await client.upsert('my_collection', { points: [...] });
await client.search('my_collection', { vector: [...], limit: 10 });
```

The Similarity Schema is **additive** — all standard vector operations work exactly like Qdrant.

---

## Use Cases

### E-Commerce
```bash
# Find similar products with explainable scores
curl -X POST /collections/products/similar -d '{
  "example": {"name": "Nike Air Max", "price": 129, "category": "sneakers"}
}'
# Response: "matched because: name: 0.35, price: 0.25, category: 0.20"
```

### Data Quality / Deduplication
```bash
# Find potential duplicate records
curl -X POST /collections/contacts/similar -d '{
  "example": {"name": "John Smith", "email": "j.smith@acme.com", "company": "Acme Inc"}
}'
```

### CRM / Lead Scoring
```bash
# Find leads similar to closed-won deals
curl -X POST /collections/leads/similar -d '{
  "like_id": "won_deal_123",
  "weights": {"deal_size": 0.4, "industry": 0.3, "company_size": 0.3}
}'
```

---

## Performance

| Metric | Performance |
|--------|-------------|
| **Vector Insert** | ~8,000 ops/sec |
| **Vector Search** | ~400-500 ops/sec |
| **Search Latency (p50)** | ~2ms |
| **Schema Reranking** | <1ms overhead |

---

## Installation

```bash
# Docker (recommended)
docker run -p 6333:6333 distx/distx:latest

# From crates.io
cargo install distx

# From source
git clone https://github.com/antonellof/distx
cd distx && cargo build --release
```

---

## Documentation

| Guide | Description |
|-------|-------------|
| [**Similarity Engine**](documentation/SIMILARITY_ENGINE.md) | Schema-driven similarity for tabular data |
| [**Data Chatbot Demo**](examples/data-chatbot/README.md) | Interactive demo with CSV upload |
| [**Comparison**](documentation/COMPARISON.md) | DistX vs Qdrant, Pinecone, Elasticsearch |
| [Quick Start](documentation/QUICK_START.md) | Get started in 5 minutes |
| [API Reference](documentation/API.md) | REST and gRPC endpoints |

---

## Use as a Library

```toml
[dependencies]
distx = "0.2.7"
distx-schema = "0.2.7"  # Similarity Schema
distx-core = "0.2.7"    # Core data structures
```

```rust
use distx_schema::{SimilaritySchema, FieldConfig, DistanceType, Reranker};
use std::collections::HashMap;

// Define schema for reranking
let mut fields = HashMap::new();
fields.insert("name".to_string(), FieldConfig::text(0.5));
fields.insert("price".to_string(), FieldConfig::number(0.3, DistanceType::Relative));
fields.insert("category".to_string(), FieldConfig::categorical(0.2));

let schema = SimilaritySchema::new(fields);

// Use reranker with vector search results
let reranker = Reranker::new(schema);
let reranked = reranker.rerank(&query_payload, candidates, None);
```

---

## Links

- **Crates.io**: https://crates.io/crates/distx
- **Documentation**: https://docs.rs/distx
- **GitHub**: https://github.com/antonellof/distx

## License

Licensed under MIT OR Apache-2.0 at your option.
