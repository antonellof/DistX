# Similarity Engine for Tabular Data

> 🚀 **[See it in action! Interactive Demo →](SIMILARITY_DEMO.md)**

DistX includes a **schema-driven similarity engine** that enables structured similarity queries over tabular data. This feature sits on top of the Qdrant-compatible vector database, adding intelligent multi-field similarity with explainability.

```bash
# Try it now:
python scripts/similarity_demo.py
```

## Overview

Traditional vector databases require you to:
1. Generate embeddings externally
2. Manage multiple vectors for different similarity aspects
3. Write complex hybrid queries

With DistX Similarity Engine:
1. Define a **Similarity Schema** once per collection
2. Insert data with just **payloads** (vectors are auto-generated)
3. Query by **example records** with explainable results

## Key Concepts

### Similarity Schema

A declarative schema that defines:
- **Which fields** matter for similarity
- **What type** of similarity to use per field
- **How much weight** each field has

```json
{
  "version": 1,
  "fields": {
    "name": {
      "type": "text",
      "distance": "semantic",
      "weight": 0.5
    },
    "price": {
      "type": "number",
      "distance": "relative",
      "weight": 0.3
    },
    "category": {
      "type": "categorical",
      "distance": "exact",
      "weight": 0.2
    }
  }
}
```

### Field Types

| Type | Description | Distance Options |
|------|-------------|------------------|
| `text` | Text/string fields | `semantic`, `exact`, `overlap` |
| `number` | Numeric fields | `absolute`, `relative`, `exact` |
| `categorical` | Category/enum fields | `exact`, `overlap` |
| `boolean` | True/false fields | `exact` |

### Distance Types

| Distance | Description |
|----------|-------------|
| `semantic` | Fuzzy text matching using trigrams |
| `absolute` | Numeric distance with exponential decay |
| `relative` | Numeric distance relative to magnitude |
| `exact` | Exact match (1.0 or 0.0) |
| `overlap` | Jaccard similarity for tokens/sets |

---

## How Auto-Embedding Works

When you insert points without vectors, DistX automatically generates vectors from the payload using the similarity schema. This is a **deterministic, hash-based approach** that requires no external ML models.

### Vector Composition

The final vector is a concatenation of per-field embeddings, each scaled by the square root of its weight:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         Final Vector (129 dims)                           │
├────────────────────────────────┬───────┬─────────────────────────────────┤
│    Text "name" (64 dims)       │ Num   │   Categorical "category" (64)   │
│    trigram + word hashing      │ (1)   │   multi-position hashing        │
└────────────────────────────────┴───────┴─────────────────────────────────┘
         × √0.5                    × √0.3              × √0.2
```

### Per-Field Embedding Methods

| Field Type | Dimensions | Embedding Method |
|------------|------------|------------------|
| **text** | 64 (default) | Trigram + word hashing |
| **number** | 1 | `tanh(value)` normalization |
| **categorical** | 64 (default) | Multi-position hash |
| **boolean** | 1 | `1.0` (true), `-1.0` (false), `0.0` (null) |

### Text Embedding (Trigram Hashing)

Text fields are embedded using character trigrams and word hashing:

```
Input: "Prosciutto cotto"

Step 1: Generate trigrams
  "  P", " Pr", "Pro", "ros", "osc", "sci", "ciu", "iut", "utt", "tto", "to ", "o  ",
  "  c", " co", "cot", "ott", "tto", "to ", "o  "

Step 2: Hash each trigram to a vector position (0-63)
  "Pro" → hash → position 23 → vector[23] += 1.0
  "ros" → hash → position 45 → vector[45] += 1.0
  ...

Step 3: Hash whole words (with 2x weight)
  "prosciutto" → hash → position 12 → vector[12] += 2.0
  "cotto"      → hash → position 37 → vector[37] += 2.0

Step 4: Normalize to unit length
  vector = vector / ||vector||

Result: [0.0, 0.0, 0.23, ..., 0.45, ..., 0.0]  (64 dimensions)
```

**Why this works**: Similar text shares trigrams, producing similar vectors:
- `"Prosciutto cotto"` and `"Prosciutto crudo"` share trigrams like "Pro", "ros", "osc", etc.
- The cosine similarity between their vectors will be high (~0.7+)

### Number Embedding

Numbers are normalized using `tanh()` which maps any value to the range [-1, 1]:

```
Input: 1.99   → tanh(1.99)  → [0.96]
Input: 2.49   → tanh(2.49)  → [0.99]
Input: 999.0  → tanh(999.0) → [1.0]   (saturates for large values)
Input: -5.0   → tanh(-5.0)  → [-1.0]
```

**Note**: For price comparisons, the reranking phase uses the actual values with relative distance, not just the embedded values.

### Categorical Embedding

Categories are hashed to multiple positions in a sparse vector:

```
Input: "salumi"

Step 1: Hash the category name
  hash("salumi") → 0x7A3B2C1D...

Step 2: Map to 4 positions using bit shifts
  position 1: (hash >> 0)  % 64 = 23
  position 2: (hash >> 16) % 64 = 45
  position 3: (hash >> 32) % 64 = 12
  position 4: (hash >> 48) % 64 = 56

Step 3: Set those positions to 1.0
  vector = [0, 0, ..., 1.0, ..., 1.0, ..., 1.0, ..., 1.0, ...]

Step 4: Normalize
  vector = vector / ||vector||  → [0, 0, ..., 0.5, ..., 0.5, ..., 0.5, ..., 0.5, ...]
```

**Result**: 
- Same category → identical vector → cosine similarity = 1.0
- Different category → different positions → cosine similarity ≈ 0.0

### Boolean Embedding

Booleans are simply mapped to scalar values:

```
true  → [1.0]
false → [-1.0]
null  → [0.0]
```

### Complete Example

Given this schema and payload:

```json
// Schema
{
  "fields": {
    "name": {"type": "text", "weight": 0.5},
    "price": {"type": "number", "weight": 0.3},
    "category": {"type": "categorical", "weight": 0.2}
  }
}

// Payload
{
  "name": "Prosciutto cotto",
  "price": 1.99,
  "category": "salumi"
}
```

The embedding process:

```
1. Embed "name" (text, 64 dims):
   trigram_hash("Prosciutto cotto") → [0.0, 0.12, 0.0, 0.34, ...]
   
2. Embed "price" (number, 1 dim):
   tanh(1.99) → [0.96]
   
3. Embed "category" (categorical, 64 dims):
   multi_hash("salumi") → [0.0, 0.0, 0.5, ..., 0.5, ...]

4. Apply weights (√weight):
   name_vec     × √0.5 = name_vec     × 0.707
   price_vec    × √0.3 = price_vec    × 0.548
   category_vec × √0.2 = category_vec × 0.447

5. Concatenate:
   final = [name_vec (64) | price_vec (1) | category_vec (64)]
   
6. Normalize:
   final = final / ||final||

Result: 129-dimensional unit vector
```

### Why Hash-Based Embedding?

| Approach | Pros | Cons |
|----------|------|------|
| **Hash-based (current)** | Fast, deterministic, no dependencies, works offline | Less semantic understanding |
| **ML embeddings (future)** | Better semantic similarity | Requires model, slower, external API |

The hash-based approach is ideal for structured/tabular data where:
- Exact and fuzzy text matching matter more than deep semantics
- Categories should match exactly
- Numbers need relative comparison
- Speed and simplicity are priorities

For use cases requiring deep semantic understanding (e.g., "cheap" ≈ "affordable"), you could extend the embedder to call external embedding APIs.

---

## API Reference

### Create Collection with Schema

Create a collection with a similarity schema. When a schema is provided, the vector dimension is automatically computed.

```
PUT /collections/{collection_name}
```

**Request Body:**
```json
{
  "similarity_schema": {
    "version": 1,
    "fields": {
      "field_name": {
        "type": "text|number|categorical|boolean",
        "distance": "semantic|absolute|relative|exact|overlap",
        "weight": 0.5
      }
    }
  }
}
```

### Get Similarity Schema

Retrieve the similarity schema for a collection.

```
GET /collections/{collection_name}/similarity-schema
```

### Set/Update Similarity Schema

Set or update the similarity schema for an existing collection.

```
PUT /collections/{collection_name}/similarity-schema
```

### Delete Similarity Schema

Remove the similarity schema from a collection.

```
DELETE /collections/{collection_name}/similarity-schema
```

### Structured Similarity Query

Find similar items using an example record.

```
POST /collections/{collection_name}/similar
```

**Request Body:**
```json
{
  "example": {
    "field1": "value1",
    "field2": 123
  },
  "limit": 10,
  "with_payload": true
}
```

Or query by existing point with custom weights:
```json
{
  "like_id": 42,
  "weights": {"price": 0.6, "name": 0.2},
  "limit": 10
}
```

**Response:**
```json
{
  "result": {
    "result": [
      {
        "id": 1,
        "score": 0.91,
        "payload": {
          "name": "Prosciutto cotto",
          "price": 1.99,
          "category": "salumi"
        },
        "explain": {
          "name": 0.46,
          "price": 0.28,
          "category": 0.17
        }
      }
    ]
  },
  "status": "ok",
  "time": 0.001
}
```

### Insert Points (Auto-Embedding)

When a collection has a similarity schema, you can insert points without vectors. The vector is automatically generated from the payload.

```
PUT /collections/{collection_name}/points
```

**Request Body (no vector needed):**
```json
{
  "points": [
    {
      "id": 1,
      "payload": {
        "name": "Prosciutto cotto",
        "price": 1.99,
        "category": "salumi"
      }
    }
  ]
}
```

---

## Weight Overrides

Override schema weights at query time for dynamic similarity behavior:

```json
{
  "example": {"name": "iPhone", "category": "electronics"},
  "weights": {"price": 0.6, "name": 0.1, "category": 0.3},
  "limit": 5
}
```

**How it works:**
- Weights not specified use schema defaults
- After applying overrides, weights are re-normalized to sum to 1.0
- Unknown fields are silently ignored

**Common patterns (define in your application):**
```python
# Python example - define reusable presets in your code
PRESETS = {
    "cheaper_alternative": {"price": 0.6, "name": 0.2},
    "quality_focus": {"rating": 0.5, "reviews": 0.3},
    "brand_loyal": {"brand": 0.6, "category": 0.3},
}

# Use in query
query(weights=PRESETS["cheaper_alternative"])
```

---

## Curl Examples

### Example 1: E-Commerce Products

```bash
# 1. Create collection with similarity schema for products
curl -X PUT "http://localhost:6333/collections/products" \
  -H "Content-Type: application/json" \
  -d '{
    "similarity_schema": {
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
          "weight": 0.3
        },
        "category": {
          "type": "categorical",
          "distance": "exact",
          "weight": 0.2
        },
        "in_stock": {
          "type": "boolean",
          "weight": 0.1
        }
      }
    }
  }'

# 2. Insert products (no vectors needed!)
curl -X PUT "http://localhost:6333/collections/products/points" \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": 1,
        "payload": {
          "name": "Prosciutto cotto",
          "price": 1.99,
          "category": "salumi",
          "in_stock": true
        }
      },
      {
        "id": 2,
        "payload": {
          "name": "Prosciutto crudo",
          "price": 2.49,
          "category": "salumi",
          "in_stock": true
        }
      },
      {
        "id": 3,
        "payload": {
          "name": "Mortadella Bologna",
          "price": 1.79,
          "category": "salumi",
          "in_stock": false
        }
      },
      {
        "id": 4,
        "payload": {
          "name": "iPhone 15 Pro",
          "price": 999.00,
          "category": "electronics",
          "in_stock": true
        }
      },
      {
        "id": 5,
        "payload": {
          "name": "Samsung Galaxy S24",
          "price": 899.00,
          "category": "electronics",
          "in_stock": true
        }
      }
    ]
  }'

# 3. Find similar products by example
curl -X POST "http://localhost:6333/collections/products/similar" \
  -H "Content-Type: application/json" \
  -d '{
    "example": {
      "name": "prosciutto",
      "price": 2.00,
      "category": "salumi"
    },
    "limit": 3
  }'

# 4. Find cheaper alternatives to a specific product (boost price weight)
curl -X POST "http://localhost:6333/collections/products/similar" \
  -H "Content-Type: application/json" \
  -d '{
    "like_id": 2,
    "weights": {"price": 0.6, "name": 0.2},
    "limit": 3
  }'

# 5. Get the similarity schema
curl -X GET "http://localhost:6333/collections/products/similarity-schema"
```

### Example 2: ERP Suppliers

```bash
# 1. Create collection for suppliers
curl -X PUT "http://localhost:6333/collections/suppliers" \
  -H "Content-Type: application/json" \
  -d '{
    "similarity_schema": {
      "version": 1,
      "fields": {
        "company_name": {
          "type": "text",
          "distance": "semantic",
          "weight": 0.3
        },
        "industry": {
          "type": "categorical",
          "distance": "exact",
          "weight": 0.25
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
        }
      }
    }
  }'

# 2. Insert suppliers
curl -X PUT "http://localhost:6333/collections/suppliers/points" \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": 1,
        "payload": {
          "company_name": "Acme Industrial Solutions",
          "industry": "manufacturing",
          "annual_revenue": 5000000,
          "employee_count": 150,
          "certified": true
        }
      },
      {
        "id": 2,
        "payload": {
          "company_name": "Global Tech Manufacturing",
          "industry": "manufacturing",
          "annual_revenue": 8000000,
          "employee_count": 250,
          "certified": true
        }
      },
      {
        "id": 3,
        "payload": {
          "company_name": "Small Parts Inc",
          "industry": "manufacturing",
          "annual_revenue": 1500000,
          "employee_count": 45,
          "certified": false
        }
      }
    ]
  }'

# 3. Find similar suppliers
curl -X POST "http://localhost:6333/collections/suppliers/similar" \
  -H "Content-Type: application/json" \
  -d '{
    "example": {
      "industry": "manufacturing",
      "annual_revenue": 4000000,
      "certified": true
    },
    "limit": 5
  }'
```

### Example 3: Job Candidates

```bash
# 1. Create collection for candidates
curl -X PUT "http://localhost:6333/collections/candidates" \
  -H "Content-Type: application/json" \
  -d '{
    "similarity_schema": {
      "version": 1,
      "fields": {
        "title": {
          "type": "text",
          "distance": "semantic",
          "weight": 0.35
        },
        "skills": {
          "type": "text",
          "distance": "overlap",
          "weight": 0.3
        },
        "years_experience": {
          "type": "number",
          "distance": "relative",
          "weight": 0.2
        },
        "location": {
          "type": "categorical",
          "distance": "exact",
          "weight": 0.15
        }
      }
    }
  }'

# 2. Insert candidates
curl -X PUT "http://localhost:6333/collections/candidates/points" \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": 1,
        "payload": {
          "title": "Senior Software Engineer",
          "skills": "rust python kubernetes docker",
          "years_experience": 8,
          "location": "remote"
        }
      },
      {
        "id": 2,
        "payload": {
          "title": "Backend Developer",
          "skills": "python django postgresql redis",
          "years_experience": 5,
          "location": "new_york"
        }
      },
      {
        "id": 3,
        "payload": {
          "title": "DevOps Engineer",
          "skills": "kubernetes docker terraform aws",
          "years_experience": 6,
          "location": "remote"
        }
      }
    ]
  }'

# 3. Find candidates similar to a job requirement
curl -X POST "http://localhost:6333/collections/candidates/similar" \
  -H "Content-Type: application/json" \
  -d '{
    "example": {
      "title": "Senior Backend Engineer",
      "skills": "python rust docker",
      "years_experience": 5,
      "location": "remote"
    },
    "limit": 5
  }'
```

### Example 4: With Constraints (Filters)

```bash
# Find similar products but only in stock
curl -X POST "http://localhost:6333/collections/products/similar" \
  -H "Content-Type: application/json" \
  -d '{
    "example": {
      "name": "prosciutto",
      "category": "salumi"
    },
    "constraints": {
      "must": [
        {
          "key": "in_stock",
          "match": {"value": true}
        }
      ]
    },
    "limit": 5
  }'

# Find similar products with price constraint
curl -X POST "http://localhost:6333/collections/products/similar" \
  -H "Content-Type: application/json" \
  -d '{
    "example": {
      "name": "prosciutto"
    },
    "constraints": {
      "must": [
        {
          "key": "price",
          "range": {"lte": 3.0}
        }
      ]
    },
    "limit": 5
  }'
```

---

## Comparison with Standard Qdrant Queries

### Without Similarity Engine (Standard Qdrant)

```bash
# You need to:
# 1. Generate embeddings externally
# 2. Send vectors with each point
# 3. Build complex hybrid queries for multi-field similarity

curl -X PUT "http://localhost:6333/collections/products/points" \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": 1,
        "vector": [0.1, 0.2, 0.3, ...],  # External embedding required!
        "payload": {"name": "Product", "price": 10}
      }
    ]
  }'
```

### With Similarity Engine (DistX)

```bash
# Just define schema once, insert payloads, and query by example!
# No external embeddings, no complex queries, full explainability

curl -X PUT "http://localhost:6333/collections/products/points" \
  -H "Content-Type: application/json" \
  -d '{
    "points": [
      {
        "id": 1,
        "payload": {"name": "Product", "price": 10}  # No vector needed!
      }
    ]
  }'
```

---

## Backwards Compatibility

The Similarity Engine is **100% additive** - all existing Qdrant APIs work unchanged:

- Collections without `similarity_schema` work exactly like standard Qdrant
- You can still provide vectors manually if needed
- Standard `/points/search` endpoint continues to work
- All existing client libraries remain compatible

---

## Best Practices

1. **Weight Normalization**: Weights are automatically normalized to sum to 1.0
2. **Field Selection**: Only include fields that matter for similarity
3. **Distance Choice**: 
   - Use `relative` for prices (handles different magnitudes)
   - Use `semantic` for free-text fields
   - Use `exact` for categories/enums
4. **Weight Overrides**: Define presets in your application for common query patterns (e.g., "cheaper_alternative", "quality_focus")
5. **Constraints**: Use constraints for hard filters, not for similarity

---

## Performance Considerations

- Initial ANN search retrieves `limit * 5` candidates for reranking
- Reranking is CPU-based (no GPU required)
- Schemas are cached in memory for fast access
- Auto-embedding uses deterministic hash-based vectors (no ML inference)

---

## See Also

- 🚀 [Interactive Demo](SIMILARITY_DEMO.md) - See the engine in action
- 📊 [Comparison](COMPARISON.md) - How DistX compares to Qdrant, Pinecone, Elasticsearch, and more
- 📖 [API Reference](API.md) - Full API documentation
